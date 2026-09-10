// Stage the built sokonanoda-lsp binary into `editor/vscode/bin/<target>/` so
// `vsce package --target <target>` produces a platform-specific VSIX with a
// bundled server (docs/design-bundled-lsp.md §3.3).
//
// Usage:
//   node scripts/stage-lsp.js                              # host release build
//   node scripts/stage-lsp.js --profile debug              # host debug build
//   node scripts/stage-lsp.js --rust-target <triple> --binary <path>
//   node scripts/stage-lsp.js --package [--out sokonanoda.vsix]
//
// Staging keeps exactly one target in `bin/` (the packaging job iterates
// targets by re-running this script), so a single VSIX never carries another
// platform's payload. Exit code 1 on unsupported host or missing binary.

const { execFileSync, spawnSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const HOST_TO_VSCE = {
  "x86_64-unknown-linux-gnu": "linux-x64",
  "aarch64-unknown-linux-gnu": "linux-arm64",
  "x86_64-unknown-linux-musl": "alpine-x64",
  "aarch64-unknown-linux-musl": "alpine-arm64",
  "aarch64-apple-darwin": "darwin-arm64",
  "x86_64-apple-darwin": "darwin-x64",
  "x86_64-pc-windows-msvc": "win32-x64",
  "aarch64-pc-windows-msvc": "win32-arm64",
};

const EXTENSION_DIR = path.join(__dirname, "..");
const REPO_ROOT = path.join(EXTENSION_DIR, "..", "..");

function hostRustTarget() {
  const out = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  const match = out.match(/^host:\s*(\S+)/m);
  if (!match) throw new Error("cannot determine the rust host triple (rustc -vV)");
  return match[1];
}

function vsceTargetForRust(triple) {
  return HOST_TO_VSCE[triple];
}

function parseArgs(argv) {
  const args = { profile: "release", package: false, out: "sokonanoda.vsix" };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--profile") args.profile = argv[++i];
    else if (arg === "--rust-target") args.rustTarget = argv[++i];
    else if (arg === "--binary") args.binary = argv[++i];
    else if (arg === "--package") args.package = true;
    else if (arg === "--out") args.out = argv[++i];
    else throw new Error(`unknown argument: ${arg}`);
  }
  return args;
}

/// Locate the built server binary for the given rust target: cargo puts it
/// under `target/<triple>/<profile>/` when `--target` was passed and under
/// `target/<profile>/` for a host build.
function findBinary(rustTarget, profile) {
  const exe = rustTarget.includes("windows") ? ".exe" : "";
  const name = `sokonanoda-lsp${exe}`;
  const candidates = [
    path.join(REPO_ROOT, "target", rustTarget, profile, name),
    path.join(REPO_ROOT, "target", profile, name),
  ];
  const found = candidates.find((candidate) => fs.existsSync(candidate));
  if (!found) {
    throw new Error(
      `built server not found; run \`cargo build ${profile === "release" ? "--release " : ""}` +
        `-p sokonanoda-lsp\` first (looked in ${candidates.join(", ")})`,
    );
  }
  return found;
}

/// Clear `bin/` and stage exactly one platform's binary. Returns
/// `{target, rustTarget, source, dest}`.
function stage(args, log = console.log) {
  const rustTarget = args.rustTarget ?? hostRustTarget();
  const target = args.target ?? vsceTargetForRust(rustTarget);
  if (!target) {
    throw new Error(
      `no VSCE target known for rust triple \`${rustTarget}\` — ` +
        "add it to scripts/stage-lsp.js and to the release matrix first",
    );
  }
  const source = args.binary ?? findBinary(rustTarget, args.profile);
  const binDir = path.join(EXTENSION_DIR, "bin");
  fs.rmSync(binDir, { recursive: true, force: true });
  const dest = path.join(
    binDir,
    target,
    rustTarget.includes("windows") ? "sokonanoda-lsp.exe" : "sokonanoda-lsp",
  );
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.copyFileSync(source, dest);
  // POSIX executable bit must be set before packaging on Linux/macOS: VS Code
  // restores the zip entry's mode on install (docs/design-bundled-lsp.md §0).
  if (!rustTarget.includes("windows")) fs.chmodSync(dest, 0o755);
  log(`staged ${source} -> ${path.relative(EXTENSION_DIR, dest)} (target ${target})`);
  return { target, rustTarget, source, dest };
}

function packageVsix(target, out, log = console.log) {
  log(`packaging ${out} (target ${target})`);
  const result = spawnSync(
    "npx",
    ["--yes", "@vscode/vsce", "package", "--target", target, "--out", out],
    { cwd: EXTENSION_DIR, stdio: "inherit", shell: process.platform === "win32" },
  );
  if (result.status !== 0) throw new Error(`vsce package failed (exit ${result.status})`);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const staged = stage(args);
  if (args.package) packageVsix(staged.target, args.out);
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    console.error(`stage-lsp: ${error.message}`);
    process.exit(1);
  }
}

module.exports = { HOST_TO_VSCE, hostRustTarget, vsceTargetForRust, parseArgs, stage, findBinary };
