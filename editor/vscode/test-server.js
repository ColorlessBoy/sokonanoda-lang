// Unit tests for server.js (bundled-binary resolution + version-pinned
// download URL). Run: node editor/vscode/test-server.js
// No VS Code, no network, no Rust needed — pure Node with an injected fs.

const fs = require("fs");
const path = require("path");
const assert = require("assert");

const server = require("./server");

// ── A tiny injectable fs ────────────────────────────────────────────────
function fakeFs({ existing = [], executable = [], contents = {} } = {}) {
  const files = new Set(existing);
  const exec = new Set(executable);
  const calls = { chmod: [] };
  return {
    constants: fs.constants,
    calls,
    existsSync: (p) => files.has(p),
    accessSync: (p) => {
      if (!exec.has(p)) throw Object.assign(new Error("EACCES"), { code: "EACCES" });
    },
    chmodSync: (p, mode) => {
      calls.chmod.push({ path: p, mode });
      exec.add(p);
    },
    readFileSync: (p) => {
      if (Object.prototype.hasOwnProperty.call(contents, p)) return contents[p];
      throw new Error(`ENOENT: ${p}`);
    },
  };
}

// ── Harness ─────────────────────────────────────────────────────────────
let passed = 0;
let failed = 0;

async function test(name, fn) {
  try {
    await fn();
    passed++;
    console.log(`  ✓ ${name}`);
  } catch (e) {
    failed++;
    console.error(`  ✗ ${name}`);
    console.error(`    ${e.stack ?? e.message}`);
  }
}

async function run() {
  console.log("server.js unit tests\n");

  await test("platformTarget maps supported platform/arch pairs", () => {
    assert.strictEqual(server.platformTarget("darwin", "arm64"), "darwin-arm64");
    assert.strictEqual(server.platformTarget("darwin", "x64"), "darwin-x64");
    assert.strictEqual(server.platformTarget("linux", "x64"), "linux-x64");
    assert.strictEqual(server.platformTarget("win32", "x64"), "win32-x64");
  });

  await test("platformTarget rejects unsupported pairs", () => {
    for (const [platform, arch] of [
      ["linux", "arm64"],
      ["win32", "arm64"],
      ["darwin", "ia32"],
      ["freebsd", "x64"],
    ]) {
      assert.strictEqual(
        server.platformTarget(platform, arch),
        undefined,
        `${platform}-${arch} must have no bundled target`,
      );
    }
  });

  await test("rustTarget mirrors platformTarget (release asset naming)", () => {
    assert.strictEqual(server.rustTarget("darwin", "arm64"), "aarch64-apple-darwin");
    assert.strictEqual(server.rustTarget("darwin", "x64"), "x86_64-apple-darwin");
    assert.strictEqual(server.rustTarget("linux", "x64"), "x86_64-unknown-linux-gnu");
    assert.strictEqual(server.rustTarget("win32", "x64"), "x86_64-pc-windows-msvc");
    assert.strictEqual(server.rustTarget("linux", "arm64"), undefined);
  });

  await test("bundledServerPath points at bin/<target>/sokonanoda-lsp[.exe]", () => {
    assert.strictEqual(
      server.bundledServerPath("/ext", "darwin", "arm64"),
      path.join("/ext", "bin", "darwin-arm64", "sokonanoda-lsp"),
    );
    assert.strictEqual(
      server.bundledServerPath("/ext", "win32", "x64"),
      path.join("/ext", "bin", "win32-x64", "sokonanoda-lsp.exe"),
    );
    assert.strictEqual(server.bundledServerPath("/ext", "linux", "arm64"), undefined);
  });

  await test("resolveBundledServer returns the executable binary as-is", () => {
    const bundled = path.join("/ext", "bin", "darwin-arm64", "sokonanoda-lsp");
    const fake = fakeFs({ existing: [bundled], executable: [bundled] });
    const found = server.resolveBundledServer({
      extensionPath: "/ext",
      platform: "darwin",
      arch: "arm64",
      fs: fake,
    });
    assert.strictEqual(found, bundled);
    assert.deepStrictEqual(fake.calls.chmod, [], "no chmod when already executable");
  });

  await test("resolveBundledServer repairs a lost executable bit", () => {
    const bundled = path.join("/ext", "bin", "linux-x64", "sokonanoda-lsp");
    const fake = fakeFs({ existing: [bundled] });
    const found = server.resolveBundledServer({
      extensionPath: "/ext",
      platform: "linux",
      arch: "x64",
      fs: fake,
    });
    assert.strictEqual(found, bundled);
    assert.strictEqual(fake.calls.chmod.length, 1);
    assert.strictEqual(fake.calls.chmod[0].mode, 0o755);
  });

  await test("resolveBundledServer falls through when chmod is impossible (read-only)", () => {
    const bundled = path.join("/ext", "bin", "linux-x64", "sokonanoda-lsp");
    const fake = fakeFs({ existing: [bundled] });
    fake.chmodSync = () => {
      throw new Error("EROFS: read-only file system");
    };
    const logs = [];
    const found = server.resolveBundledServer({
      extensionPath: "/ext",
      platform: "linux",
      arch: "x64",
      fs: fake,
      log: (message) => logs.push(message),
    });
    assert.strictEqual(found, undefined, "must fall through to the next path");
    assert.ok(logs.some((line) => line.includes("chmod failed")), logs.join("\n"));
  });

  await test("resolveBundledServer ignores missing binaries and unsupported platforms", () => {
    const fake = fakeFs();
    assert.strictEqual(
      server.resolveBundledServer({ extensionPath: "/ext", platform: "linux", arch: "x64", fs: fake }),
      undefined,
    );
    assert.strictEqual(
      server.resolveBundledServer({ extensionPath: "/ext", platform: "linux", arch: "arm64", fs: fake }),
      undefined,
    );
  });

  const ARGS = {
    extensionPath: "/ext",
    roots: [],
    platform: "darwin",
    arch: "arm64",
    version: "0.7.0",
  };
  const BUNDLED = path.join("/ext", "bin", "darwin-arm64", "sokonanoda-lsp");

  await test("resolveServerCommand: setting wins over everything", () => {
    const fake = fakeFs({ existing: [BUNDLED], executable: [BUNDLED] });
    const found = server.resolveServerCommand({
      ...ARGS,
      setting: "  /custom/sokonanoda-lsp  ",
      envBin: "/env/sokonanoda-lsp",
      fs: fake,
    });
    assert.strictEqual(found, "/custom/sokonanoda-lsp");
  });

  await test("resolveServerCommand: env var wins over bundled", () => {
    const fake = fakeFs({ existing: [BUNDLED], executable: [BUNDLED] });
    const found = server.resolveServerCommand({
      ...ARGS,
      envBin: "/env/sokonanoda-lsp",
      fs: fake,
    });
    assert.strictEqual(found, "/env/sokonanoda-lsp");
  });

  await test("resolveServerCommand: bundled binary beats workspace builds", () => {
    const workspace = path.join("/ws", "target", "release", "sokonanoda-lsp");
    const fake = fakeFs({ existing: [BUNDLED, workspace], executable: [BUNDLED, workspace] });
    const found = server.resolveServerCommand({ ...ARGS, roots: ["/ws"], fs: fake });
    assert.strictEqual(found, BUNDLED);
  });

  await test("resolveServerCommand: workspace build used when no bundled binary", () => {
    const workspace = path.join("/ws", "target", "release", "sokonanoda-lsp");
    const fake = fakeFs({ existing: [workspace], executable: [workspace] });
    const found = server.resolveServerCommand({ ...ARGS, roots: ["/ws"], fs: fake });
    assert.strictEqual(found, workspace);
  });

  await test("resolveServerCommand: version-current cache is the last resort", () => {
    const previousHome = process.env.HOME;
    process.env.HOME = path.join("/home", "test");
    try {
      const dest = server.serverDest();
      const marker = server.serverVersionMarker();
      const fake = fakeFs({
        existing: [dest, marker],
        executable: [dest],
        contents: { [marker]: "0.7.0\n" },
      });
      const found = server.resolveServerCommand({ ...ARGS, fs: fake });
      assert.strictEqual(found, dest);
      // A stale marker must NOT be reused.
      const stale = fakeFs({
        existing: [dest, marker],
        executable: [dest],
        contents: { [marker]: "0.6.0\n" },
      });
      assert.strictEqual(server.resolveServerCommand({ ...ARGS, fs: stale }), undefined);
    } finally {
      process.env.HOME = previousHome;
    }
  });

  await test("downloadUrl is pinned to v<version>, never /latest/", () => {
    const url = server.downloadUrl("0.7.0", "darwin", "arm64");
    assert.strictEqual(
      url,
      "https://github.com/ColorlessBoy/sokonanoda-lang/releases/download/v0.7.0/sokonanoda-lsp-aarch64-apple-darwin.tar.gz",
    );
    assert.ok(!url.includes("/latest/"), "the download must never resolve to latest");
    assert.strictEqual(server.downloadUrl("0.7.0", "linux", "arm64"), undefined);
  });

  await test("downloadLspBinary rejects unsupported platforms with guidance", async () => {
    await assert.rejects(
      server.downloadLspBinary({ version: "0.7.0", platform: "linux", arch: "arm64" }),
      /没有内置语言服务器/,
    );
  });

  console.log(`\n${passed + failed} tests, ${passed} passed, ${failed} failed\n`);
  process.exit(failed > 0 ? 1 : 0);
}

run();
