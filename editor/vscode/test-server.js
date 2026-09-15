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
    assert.strictEqual(server.platformTarget("linux", "arm64"), "linux-arm64");
    assert.strictEqual(server.platformTarget("win32", "x64"), "win32-x64");
    assert.strictEqual(server.platformTarget("win32", "arm64"), "win32-arm64");
    assert.strictEqual(server.platformTarget("linux", "x64", true), "alpine-x64");
    assert.strictEqual(server.platformTarget("linux", "arm64", true), "alpine-arm64");
  });

  await test("platformTarget rejects unsupported pairs", () => {
    for (const [platform, arch] of [
      ["linux", "ia32"],
      ["linux", "arm"],
      ["win32", "ia32"],
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
    assert.strictEqual(server.rustTarget("linux", "arm64"), "aarch64-unknown-linux-gnu");
    assert.strictEqual(server.rustTarget("win32", "x64"), "x86_64-pc-windows-msvc");
    assert.strictEqual(server.rustTarget("win32", "arm64"), "aarch64-pc-windows-msvc");
    assert.strictEqual(server.rustTarget("linux", "x64", true), "x86_64-unknown-linux-musl");
    assert.strictEqual(server.rustTarget("linux", "arm64", true), "aarch64-unknown-linux-musl");
    assert.strictEqual(server.rustTarget("linux", "ia32"), undefined);
  });

  await test("isAlpineLinux detects /etc/alpine-release (linux only)", () => {
    const alpine = fakeFs({ existing: ["/etc/alpine-release"] });
    assert.strictEqual(server.isAlpineLinux("linux", alpine), true);
    assert.strictEqual(server.isAlpineLinux("darwin", alpine), false);
    assert.strictEqual(server.isAlpineLinux("linux", fakeFs()), false);
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
    assert.strictEqual(
      server.bundledServerPath("/ext", "win32", "arm64"),
      path.join("/ext", "bin", "win32-arm64", "sokonanoda-lsp.exe"),
    );
    assert.strictEqual(
      server.bundledServerPath("/ext", "linux", "arm64"),
      path.join("/ext", "bin", "linux-arm64", "sokonanoda-lsp"),
    );
    assert.strictEqual(
      server.bundledServerPath("/ext", "linux", "x64", true),
      path.join("/ext", "bin", "alpine-x64", "sokonanoda-lsp"),
    );
    assert.strictEqual(server.bundledServerPath("/ext", "linux", "ia32"), undefined);
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
      server.resolveBundledServer({ extensionPath: "/ext", platform: "linux", arch: "ia32", fs: fake }),
      undefined,
    );
  });

  await test("resolveCliCommand: bundled CLI, then workspace build, then PATH", () => {
    const bundled = path.join("/ext", "bin", "linux-x64", "sokonanoda");
    const withBundled = fakeFs({ existing: [bundled], executable: [bundled] });
    assert.strictEqual(
      server.resolveCliCommand({
        extensionPath: "/ext",
        roots: [],
        platform: "linux",
        arch: "x64",
        fs: withBundled,
      }),
      bundled,
    );
    const workspace = path.join("/ws", "target", "release", "sokonanoda");
    const withWorkspace = fakeFs({ existing: [workspace], executable: [workspace] });
    assert.strictEqual(
      server.resolveCliCommand({
        extensionPath: "/ext",
        roots: ["/ws"],
        platform: "linux",
        arch: "x64",
        fs: withWorkspace,
      }),
      workspace,
    );
    assert.strictEqual(
      server.resolveCliCommand({
        extensionPath: "/ext",
        roots: [],
        platform: "linux",
        arch: "x64",
        fs: fakeFs(),
      }),
      "sokonanoda",
    );
  });

  await test("resolveCliCommand uses the Windows .exe name", () => {
    const bundled = path.join("/ext", "bin", "win32-x64", "sokonanoda.exe");
    const fake = fakeFs({ existing: [bundled] });
    assert.strictEqual(
      server.resolveCliCommand({
        extensionPath: "/ext",
        roots: [],
        platform: "win32",
        arch: "x64",
        fs: fake,
      }),
      bundled,
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

  // Bundled-first policy (docs/design/extension-server-policy.md §2, §5):
  // default `override === false` ignores setting/env/workspace builds; only an
  // explicit opt-in restores them. `resolveServerCommand` returns
  // `{command, source}`.
  await test("resolveServerCommand: default (override=false) returns the bundled server", () => {
    const fake = fakeFs({ existing: [BUNDLED], executable: [BUNDLED] });
    const found = server.resolveServerCommand({
      ...ARGS,
      setting: "/custom/sokonanoda-lsp",
      envBin: "/env/sokonanoda-lsp",
      roots: ["/ws"],
      fs: fake,
    });
    assert.deepStrictEqual(found, { command: BUNDLED, source: "bundled" });
  });

  await test("resolveServerCommand: override=false keeps a stale serverPath from overriding bundled", () => {
    // Even a serverPath that does not exist must not change the result (and
    // must not turn into an error at this layer).
    const fake = fakeFs({ existing: [BUNDLED], executable: [BUNDLED] });
    const found = server.resolveServerCommand({
      ...ARGS,
      setting: "/custom/missing-sokonanoda-lsp",
      fs: fake,
    });
    assert.strictEqual(found.command, BUNDLED);
    assert.strictEqual(found.source, "bundled");
  });

  await test("resolveServerCommand: override=false ignores a workspace build in favor of bundled", () => {
    const workspace = path.join("/ws", "target", "release", "sokonanoda-lsp");
    const fake = fakeFs({ existing: [BUNDLED, workspace], executable: [BUNDLED, workspace] });
    const found = server.resolveServerCommand({
      ...ARGS,
      roots: ["/ws"],
      override: false,
      fs: fake,
    });
    assert.deepStrictEqual(found, { command: BUNDLED, source: "bundled" });
  });

  await test("resolveServerCommand: override=true restores setting → env → bundled → workspace", () => {
    const fake = fakeFs({ existing: [BUNDLED], executable: [BUNDLED] });
    assert.deepStrictEqual(
      server.resolveServerCommand({
        ...ARGS,
        override: true,
        setting: "  /custom/sokonanoda-lsp  ",
        envBin: "/env/sokonanoda-lsp",
        fs: fake,
      }),
      { command: "/custom/sokonanoda-lsp", source: "setting" },
    );
    assert.deepStrictEqual(
      server.resolveServerCommand({
        ...ARGS,
        override: true,
        envBin: "/env/sokonanoda-lsp",
        fs: fake,
      }),
      { command: "/env/sokonanoda-lsp", source: "env" },
    );
    const workspace = path.join("/ws", "target", "release", "sokonanoda-lsp");
    const withBoth = fakeFs({
      existing: [BUNDLED, workspace],
      executable: [BUNDLED, workspace],
    });
    assert.deepStrictEqual(
      server.resolveServerCommand({ ...ARGS, override: true, roots: ["/ws"], fs: withBoth }),
      { command: BUNDLED, source: "bundled" },
    );
    const onlyWorkspace = fakeFs({ existing: [workspace], executable: [workspace] });
    assert.deepStrictEqual(
      server.resolveServerCommand({
        ...ARGS,
        override: true,
        roots: ["/ws"],
        fs: onlyWorkspace,
      }),
      { command: workspace, source: "workspace" },
    );
  });

  await test("resolveServerCommand: no bundled falls back to cache only when current", () => {
    const previousHome = process.env.HOME;
    process.env.HOME = path.join("/home", "test");
    try {
      const dest = server.serverDest();
      const marker = server.serverVersionMarker();
      const current = fakeFs({
        existing: [dest, marker],
        executable: [dest],
        contents: { [marker]: "0.7.0\n" },
      });
      assert.deepStrictEqual(server.resolveServerCommand({ ...ARGS, fs: current }), {
        command: dest,
        source: "cache",
      });
      // A stale marker must NOT be reused (caller then downloads).
      const stale = fakeFs({
        existing: [dest, marker],
        executable: [dest],
        contents: { [marker]: "0.6.0\n" },
      });
      assert.strictEqual(server.resolveServerCommand({ ...ARGS, fs: stale }).command, undefined);
      // No bundled, no cache -> undefined (caller then downloads).
      assert.strictEqual(server.resolveServerCommand({ ...ARGS, fs: fakeFs() }).command, undefined);
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
    assert.strictEqual(
      server.downloadUrl("0.7.0", "linux", "arm64", true),
      "https://github.com/ColorlessBoy/sokonanoda-lang/releases/download/v0.7.0/sokonanoda-lsp-aarch64-unknown-linux-musl.tar.gz",
    );
    assert.strictEqual(server.downloadUrl("0.7.0", "linux", "ia32"), undefined);
  });

  await test("downloadLspBinary rejects unsupported platforms with guidance", async () => {
    await assert.rejects(
      server.downloadLspBinary({ version: "0.7.0", platform: "linux", arch: "ia32" }),
      /没有内置语言服务器/,
    );
  });

  console.log(`\n${passed + failed} tests, ${passed} passed, ${failed} failed\n`);
  process.exit(failed > 0 ? 1 : 0);
}

run();
