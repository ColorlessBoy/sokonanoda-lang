#!/usr/bin/env node
// scripts/avatar_proto/shots.mjs — deterministic evidence screenshots.
//
//   node scripts/avatar_proto/shots.mjs
//
// Produces the required /tmp evidence plus three committed copies:
//   /tmp/avatar-sheet.png      -> screenshots/character-sheet.png
//   /tmp/avatar-peer.png       -> screenshots/stage-peer-1440.png
//   /tmp/avatar-mentor.png     (local only, same scene as peer)
//   /tmp/avatar-mobile.png     -> screenshots/stage-mobile-390.png

import { writeFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  openProbe,
  capture,
  fullPageCapture,
  clickSelector,
  setViewport,
  sleep,
  ensureDir,
} from "./lib/cdp.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "screenshots");
const HTML = join(HERE, "index.html");
ensureDir(OUT);

const written = [];
function save(name, buffer) {
  writeFileSync(name, buffer);
  written.push({ file: name, bytes: buffer.length });
}

async function settle(client, { scrollTop = false } = {}) {
  await client.evaluate(`(() => {
    if (${scrollTop}) window.scrollTo(0, 0);
    document.activeElement?.blur?.();
    return true;
  })()`);
  await client.call("Input.dispatchMouseEvent", { type: "mouseMoved", x: 2, y: 2 });
  // Let the one-shot greeting wave finish and avoid catching a blink.
  await sleep(2700);
  const deadline = Date.now() + 2500;
  while (Date.now() < deadline) {
    const state = await client.evaluate(`(() => {
      const av = document.querySelector('#xiao-xi');
      return { wave: av.classList.contains('is-waving'), blink: av.dataset.blink };
    })()`);
    if (!state.wave && state.blink !== "1") {
      await sleep(120);
      return;
    }
    await sleep(80);
  }
  throw new Error("character never settled (wave/blink)");
}

const peer = await openProbe({
  htmlPath: HTML,
  width: 1440,
  height: 1100,
  dpr: 1,
  readyExpression: "document.body && document.body.dataset.ready === '1'",
});
try {
  const { client } = peer;
  await sleep(300);
  await clickSelector(client, '[data-persona-btn="peer"]');
  await clickSelector(client, '[data-line-btn="1"]');
  await clickSelector(client, '[data-expr-btn="curious"]');
  await settle(client, { scrollTop: true });
  save("/tmp/avatar-peer.png", await capture(client, { format: "png" }));
  save(join(OUT, "stage-peer-1440.png"), await capture(client, { format: "png" }));

  await clickSelector(client, '[data-persona-btn="mentor"]');
  await clickSelector(client, '[data-line-btn="1"]');
  await clickSelector(client, '[data-expr-btn="thinking"]');
  await settle(client, { scrollTop: true });
  save("/tmp/avatar-mentor.png", await capture(client, { format: "png" }));

  // Mobile: 390 CSS px, 2x for legibility. Clip the top 1500 CSS px so the
  // evidence includes header/intro, the whole stage, the line bar and the
  // beginning of the control dock without a 4000px-tall page image.
  await setViewport(client, { width: 390, height: 844, dpr: 2, mobile: true });
  await clickSelector(client, '[data-persona-btn="peer"]');
  await clickSelector(client, '[data-line-btn="1"]');
  await clickSelector(client, '[data-expr-btn="curious"]');
  await settle(client, { scrollTop: true });
  save(
    "/tmp/avatar-mobile.png",
    await capture(client, {
      format: "png",
      clip: { x: 0, y: 0, width: 390, height: 1500, scale: 1 },
      captureBeyondViewport: true,
    })
  );
  save(
    join(OUT, "stage-mobile-390.png"),
    await capture(client, {
      format: "png",
      clip: { x: 0, y: 0, width: 390, height: 1500, scale: 1 },
      captureBeyondViewport: true,
    })
  );
} finally {
  await peer.close();
}

const sheet = await openProbe({
  htmlPath: HTML,
  width: 1440,
  height: 900,
  dpr: 1.5,
  query: "?view=sheet",
  readyExpression: "document.body && document.body.dataset.ready === '1'",
});
try {
  await sleep(350);
  const buffer = await fullPageCapture(sheet.client, { maxHeight: 3000 });
  save("/tmp/avatar-sheet.png", buffer);
  save(join(OUT, "character-sheet.png"), buffer);
} finally {
  await sheet.close();
}

for (const item of written) {
  console.log(`${item.file}  ${(item.bytes / 1024).toFixed(1)} KiB`);
}
const sizes = Object.fromEntries(written.map((item) => [item.file, statSync(item.file).size]));
if (!sizes["/tmp/avatar-sheet.png"] || !sizes["/tmp/avatar-peer.png"] || !sizes["/tmp/avatar-mobile.png"]) {
  throw new Error("one of the required /tmp screenshots is missing");
}
