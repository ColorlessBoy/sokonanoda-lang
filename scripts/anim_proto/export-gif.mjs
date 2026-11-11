#!/usr/bin/env node
// scripts/anim_proto/export-gif.mjs —— 确定性地截帧并编码 GIF（无 npm 依赖）。
//
// 流程：
//   1. 文件层 + 浏览器层验收（与 check.mjs 同一份判据）；
//   2. Chrome CDP 打开 file://…/index.html，调 window.SokoAnim.seek(t) 逐帧截图；
//   3. ffmpeg palettegen/paletteuse 编码；ffprobe 复核宽高/fps/帧数/体积；
//   4. 同时写 /tmp/anim-first.png、anim-band.png、anim-converged.png。
//
// 用法：
//   node scripts/anim_proto/export-gif.mjs
//   node scripts/anim_proto/export-gif.mjs --check        # 只跑验收，不截图/编码
//   node scripts/anim_proto/export-gif.mjs --stills-only  # 只截三张证据图
//   node scripts/anim_proto/export-gif.mjs --fps 20 --out scripts/anim_proto/epsilon-n.gif
//   node scripts/anim_proto/export-gif.mjs --keep-frames  # 保留临时 PNG 帧便于排错
//
// 依赖：本机 Chrome/Chromium、ffmpeg、ffprobe；Node 24 自带 fetch/WebSocket/net。
// 不安装任何 npm 包，不调用 Manim/Python。

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  openProbe,
  setViewport,
  setReducedMotion,
  capture,
  sleep,
} from "./lib/cdp.mjs";
import { staticChecks, browserChecks } from "./lib/verify.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const SOURCE = join(HERE, "index.html");
const PREVIEW = join(HERE, "preview", "index.html");
const MAX_GIF_BYTES = 2_000_000;
const DEFAULT_FPS = 20;
const DEFAULT_OUT = join(HERE, "epsilon-n.gif");
const log = (...args) => console.log("[export]", ...args);

function parseArgs(argv) {
  const args = {
    out: DEFAULT_OUT,
    fps: DEFAULT_FPS,
    width: 880,
    framesDir: "",
    keepFrames: false,
    stillsOnly: false,
    checkOnly: false,
    chrome: "",
    port: null,
    keep: false,
    json: false,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--out") args.out = resolve(argv[++i]);
    else if (arg === "--fps") args.fps = Number(argv[++i]);
    else if (arg === "--width") args.width = Number(argv[++i]);
    else if (arg === "--frames-dir") args.framesDir = resolve(argv[++i]);
    else if (arg === "--keep-frames") args.keepFrames = true;
    else if (arg === "--stills-only") args.stillsOnly = true;
    else if (arg === "--check") args.checkOnly = true;
    else if (arg === "--chrome") args.chrome = resolve(argv[++i]);
    else if (arg === "--port") args.port = Number(argv[++i]);
    else if (arg === "--keep") args.keep = true;
    else if (arg === "--json") args.json = true;
    else if (arg === "-h" || arg === "--help") {
      console.log(readFileSync(fileURLToPath(import.meta.url), "utf8").split("\n").slice(0, 30).join("\n"));
      process.exit(0);
    } else {
      console.error(`[export] unknown argument: ${arg} (see --help)`);
      process.exit(2);
    }
  }
  if (!Number.isFinite(args.fps) || args.fps < 20 || args.fps > 30) {
    console.error("[export] --fps must be between 20 and 30 (GIF review target)");
    process.exit(2);
  }
  return args;
}

const args = parseArgs(process.argv.slice(2));
const failures = [];

function have(binary) {
  const result = spawnSync(binary, ["-version"], { encoding: "utf8" });
  return result.status === 0;
}

function run(binary, argv, options = {}) {
  return spawnSync(binary, argv, {
    encoding: "utf8",
    maxBuffer: 32 * 1024 * 1024,
    ...options,
  });
}

function pngSize(buffer) {
  if (buffer.length < 24 || buffer.readUInt32BE(0) !== 0x89504e47) return { width: 0, height: 0 };
  return { width: buffer.readUInt32BE(16), height: buffer.readUInt32BE(20) };
}

function parseRate(text) {
  if (!text) return 0;
  const [num, den] = String(text).split("/").map(Number);
  return den ? num / den : num;
}

function ffprobe(file) {
  const result = run("ffprobe", [
    "-v", "error",
    "-select_streams", "v:0",
    "-count_frames",
    "-show_entries", "stream=width,height,r_frame_rate,avg_frame_rate,nb_read_frames",
    "-of", "json",
    file,
  ]);
  if (result.status !== 0) throw new Error(`ffprobe failed for ${file}: ${result.stderr.slice(0, 500)}`);
  const stream = JSON.parse(result.stdout).streams?.[0];
  if (!stream) throw new Error(`ffprobe returned no video stream for ${file}`);
  return {
    width: Number(stream.width),
    height: Number(stream.height),
    fps: parseRate(stream.r_frame_rate || stream.avg_frame_rate),
    frames: Number(stream.nb_read_frames),
  };
}

async function doubleRaf(client) {
  await client.evaluate(
    "new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve(true))))"
  );
}

async function seekAndSettle(client, t) {
  await client.evaluate(`window.SokoAnim.seek(${Number(t)})`);
  await doubleRaf(client);
}

async function captureStill(client, { path, t, width, height, dpr, fullPage = true }) {
  await setViewport(client, { width, height, dpr, mobile: width < 700 });
  await client.evaluate("window.scrollTo(0, 0)");
  await seekAndSettle(client, t);
  let clip;
  if (fullPage) {
    const pageHeight = await client.evaluate(
      "Math.ceil(Math.max(document.documentElement.scrollHeight, document.body.scrollHeight))"
    );
    clip = { x: 0, y: 0, width, height: pageHeight, scale: 1 };
  } else {
    clip = { x: 0, y: 0, width, height, scale: 1 };
  }
  const png = await capture(client, { format: "png", clip, captureBeyondViewport: true });
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, png);
  const size = pngSize(png);
  log(`png · ${path} · ${size.width}×${size.height} · ${(png.length / 1024).toFixed(0)} KiB`);
  return { path, bytes: png.length, width: size.width, height: size.height, t, cssWidth: width };
}

async function captureGifFrames(client, { framesDir, fps, width }) {
  if (!existsSync(framesDir)) mkdirSync(framesDir, { recursive: true });
  await setReducedMotion(client, false);
  await setViewport(client, { width, height: 1100, dpr: 2, mobile: false });
  await client.evaluate("window.scrollTo(0, 0)");
  await client.evaluate("window.SokoAnim.beginCapture()");
  await doubleRaf(client);
  const rect = await client.evaluateJSON(`(() => {
    const card = document.getElementById('stageCard');
    const r = card.getBoundingClientRect();
    return { x: r.x, y: r.y, width: r.width, height: r.height, pageHeight: document.documentElement.scrollHeight };
  })()`);
  const margin = 12;
  const clip = {
    x: Math.max(0, rect.x - margin),
    y: Math.max(0, rect.y - margin),
    width: rect.width + margin * 2,
    height: rect.height + margin * 2,
    scale: 1,
  };
  log(`frame clip · ${clip.x.toFixed(1)},${clip.y.toFixed(1)} ${clip.width.toFixed(1)}×${clip.height.toFixed(1)} css · dpr 2`);
  const frameCount = Math.round((await client.evaluate("window.SokoAnim.duration")) * fps);
  const files = [];
  for (let i = 0; i < frameCount; i += 1) {
    const t = i / fps;
    await seekAndSettle(client, t);
    const png = await capture(client, { format: "png", clip, captureBeyondViewport: true });
    const file = join(framesDir, `frame-${String(i).padStart(4, "0")}.png`);
    writeFileSync(file, png);
    files.push(file);
    if (i === 0 || (i + 1) % 20 === 0 || i === frameCount - 1) {
      log(`frame ${i + 1}/${frameCount} · t=${t.toFixed(2)}s · ${(png.length / 1024).toFixed(0)} KiB`);
    }
  }
  await client.evaluate("window.SokoAnim.endCapture()");
  return { files, frameCount, clip, cssWidth: clip.width, cssHeight: clip.height };
}

function encodeGif(framesDir, fps, width, outPath) {
  const filter = `[0:v]scale=${width}:-2:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128:stats_mode=diff[p];[s1][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle`;
  const result = run("ffmpeg", [
    "-hide_banner", "-loglevel", "error", "-y",
    "-framerate", String(fps),
    "-start_number", "0",
    "-i", join(framesDir, "frame-%04d.png"),
    "-filter_complex", filter,
    "-loop", "0",
    outPath,
  ]);
  if (result.status !== 0) throw new Error(`ffmpeg failed: ${result.stderr.slice(0, 1200)}`);
  return statSync(outPath).size;
}

function encodeUnderBudget(framesDir, fps, preferredWidth, outPath) {
  const widths = [preferredWidth, 800, 720, 640, 580, 520, 460].filter(
    (value, index, list) => value > 0 && list.indexOf(value) === index
  );
  const attempts = [];
  for (const width of widths) {
    const temp = outPath.replace(/\.gif$/i, `-${width}.gif`);
    const bytes = encodeGif(framesDir, fps, width, temp);
    attempts.push({ width, bytes, ok: bytes < MAX_GIF_BYTES });
    log(`gif attempt · ${width}px · ${(bytes / 1024 / 1024).toFixed(2)} MiB · ${bytes < MAX_GIF_BYTES ? "under budget" : "over budget"}`);
    if (bytes < MAX_GIF_BYTES) {
      if (existsSync(outPath)) rmSync(outPath, { force: true });
      renameSync(temp, outPath);
      for (const other of widths) {
        const path = outPath.replace(/\.gif$/i, `-${other}.gif`);
        if (existsSync(path)) rmSync(path, { force: true });
      }
      return { width, bytes, attempts };
    }
    rmSync(temp, { force: true });
  }
  return { width: widths[widths.length - 1], bytes: Infinity, attempts };
}

async function main() {
  const staticResult = staticChecks({ htmlPath: SOURCE, previewPath: PREVIEW });
  if (!args.json) {
    for (const failure of staticResult.failures) log("FAIL ·", failure);
    log(`static: ${staticResult.ok ? "ok" : "failed"} · ${staticResult.info.bytes ?? 0} bytes · sha ${staticResult.info.sha256 ?? "?"}`);
  }
  if (!staticResult.ok) {
    failures.push(...staticResult.failures.map((message) => `static: ${message}`));
    return finish({ static: staticResult, browser: null });
  }

  if (!args.stillsOnly && !args.checkOnly) {
    if (!have("ffmpeg")) throw new Error("ffmpeg not found on PATH; install it or use --stills-only");
    if (!have("ffprobe")) throw new Error("ffprobe not found on PATH; install it or use --stills-only");
  }

  let probe = null;
  let browserResult = null;
  let framesDir = args.framesDir;
  let frames = null;
  const stills = [];
  let gif = null;
  try {
    probe = await openProbe({
      htmlPath: SOURCE,
      chrome: args.chrome,
      width: 1440,
      height: 1100,
      dpr: 1,
      port: args.port,
      keep: args.keep,
    });
    browserResult = await browserChecks({ client: probe.client });
    if (!args.json) {
      for (const failure of browserResult.failures) log("FAIL ·", failure);
      log(`browser: ${browserResult.ok ? "ok" : "failed"} · ${browserResult.checks} checks`);
    }
    if (!browserResult.ok) {
      failures.push(...browserResult.failures.map((message) => `browser: ${message}`));
      return finish({ static: staticResult, browser: browserResult, stills, gif });
    }
    if (args.checkOnly) return finish({ static: staticResult, browser: browserResult, stills, gif });

    await setReducedMotion(probe.client, false);
    await probe.client.evaluate("window.SokoAnim.beginCapture()");
    await probe.client.evaluate("window.scrollTo(0, 0)");
    await setViewport(probe.client, { width: 1440, height: 1100, dpr: 1, mobile: false });
    await sleep(80);

    stills.push(await captureStill(probe.client, {
      path: "/tmp/anim-first.png", t: 0, width: 1440, height: 1100, dpr: 2, fullPage: true,
    }));
    stills.push(await captureStill(probe.client, {
      path: "/tmp/anim-band.png", t: 3.0, width: 1440, height: 1100, dpr: 2, fullPage: true,
    }));
    stills.push(await captureStill(probe.client, {
      path: "/tmp/anim-converged.png", t: 6.0, width: 1440, height: 1100, dpr: 2, fullPage: true,
    }));
    stills.push(await captureStill(probe.client, {
      path: "/tmp/anim-mobile.png", t: 3.0, width: 390, height: 844, dpr: 2, fullPage: true,
    }));
    await probe.client.evaluate("window.SokoAnim.endCapture()");

    if (!args.stillsOnly) {
      if (!framesDir) {
        framesDir = mkdtempSync(join(tmpdir(), "soko-anim-frames-"));
      } else {
        mkdirSync(framesDir, { recursive: true });
      }
      log(`frames dir · ${framesDir}`);
      frames = await captureGifFrames(probe.client, {
        framesDir,
        fps: args.fps,
        width: 1440,
      });
      log(`captured ${frames.files.length} frames`);

      const chosen = encodeUnderBudget(framesDir, args.fps, args.width, args.out);
      if (!Number.isFinite(chosen.bytes)) {
        throw new Error(
          `could not encode a GIF under ${MAX_GIF_BYTES} bytes; attempts: ${JSON.stringify(chosen.attempts)}`
        );
      }
      if (!existsSync(args.out)) throw new Error(`GIF was not written to ${args.out}`);
      const meta = ffprobe(args.out);
      const bytes = statSync(args.out).size;
      const expectedFrames = Math.round((await probe.client.evaluate("window.SokoAnim.duration")) * args.fps);
      const fitsBudget = bytes < MAX_GIF_BYTES;
      const fpsOk = Math.abs(meta.fps - args.fps) < 0.01;
      const framesOk = meta.frames === expectedFrames;
      gif = {
        path: args.out,
        bytes,
        mib: Number((bytes / 1024 / 1024).toFixed(2)),
        width: meta.width,
        height: meta.height,
        fps: meta.fps,
        frames: meta.frames,
        expectedFrames,
        underBudget: fitsBudget,
        attempts: chosen.attempts,
        source: "Chrome CDP window.SokoAnim.seek(t)",
      };
      log(`gif · ${args.out} · ${meta.width}×${meta.height} · ${meta.fps}fps · ${meta.frames} frames · ${(bytes / 1024 / 1024).toFixed(2)} MiB`);
      if (!fitsBudget) failures.push(`gif is ${bytes} bytes (>= ${MAX_GIF_BYTES})`);
      if (!fpsOk) failures.push(`gif fps is ${meta.fps} (expected ${args.fps})`);
      if (!framesOk) failures.push(`gif has ${meta.frames} frames (expected ${expectedFrames})`);
      if (meta.width <= 0 || meta.height <= 0) failures.push("gif has invalid dimensions");
      if (meta.height % 2 !== 0) failures.push("gif height is not even");
    }
    return finish({ static: staticResult, browser: browserResult, stills, gif, frames });
  } finally {
    if (probe) await probe.close();
    if (!args.keepFrames && framesDir && existsSync(framesDir) && !args.framesDir) {
      try { rmSync(framesDir, { recursive: true, force: true }); } catch { /* ignore */ }
    }
    if (args.keepFrames && framesDir) log(`kept frames · ${framesDir}`);
  }
}

function finish({ static: staticResult, browser: browserResult, stills = [], gif = null, frames = null }) {
  const summary = {
    ok: failures.length === 0,
    static: { ok: staticResult.ok, bytes: staticResult.info.bytes, sha256: staticResult.info.sha256 },
    browser: browserResult
      ? {
          ok: browserResult.ok,
          checks: browserResult.checks,
          externalRequests: browserResult.details.externalRequests,
          pageErrors: browserResult.details.pageErrors,
          consoleErrors: browserResult.details.consoleErrors,
          formulaCases: browserResult.details.formulaCases,
          formulaPasses: browserResult.details.formulaPasses,
          layout: Object.fromEntries(
            Object.entries(browserResult.details.layout || {}).map(([name, value]) => [
              name,
              { overflow: value.overflow, labels: value.labels, offenders: value.offenders.length },
            ])
          ),
        }
      : null,
    stills,
    gif,
    frames: frames ? { count: frames.files.length, dir: args.keepFrames ? frames.files[0].replace(/\/frame-.*$/, "") : "(temp removed)" } : null,
    failures,
  };
  console.log(JSON.stringify(summary, null, 2));
  process.exitCode = failures.length === 0 ? 0 : 1;
  return summary;
}

try {
  await main();
} catch (error) {
  failures.push(error.message);
  console.error("[export] fatal:", error.stack || error.message);
  process.exitCode = 1;
}
