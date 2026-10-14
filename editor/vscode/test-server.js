// 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
// Unit tests for server.js (bundled-binary resolution + version-pinned
// download URL). Run: node editor/vscode/test-server.js
// No VS Code, no network, no Rust needed — pure Node with an injected fs.

const fs = require("fs");
const os = require("os");
const path = require("path");
const assert = require("assert");

const server = require("./server");

// ── A tiny injectable fs ────────────────────────────────────────────────
// `mtimes` / `dirs` are only needed by the workspace-build judgement
// (stale + version); every other test keeps the original three maps.
function fakeFs({ existing = [], executable = [], contents = {}, mtimes = {}, dirs = {} } = {}) {
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
    statSync: (p) => {
      if (!Object.prototype.hasOwnProperty.call(mtimes, p)) throw new Error(`ENOENT: ${p}`);
      return { mtimeMs: mtimes[p] };
    },
    readdirSync: (p) => {
      if (!Object.prototype.hasOwnProperty.call(dirs, p)) throw new Error(`ENOTDIR: ${p}`);
      return dirs[p].map((entry) => ({
        name: entry.name,
        isDirectory: () => entry.directory === true,
      }));
    },
  };
}

/// Directory tree for `newestEngineSource`'s `crates/**` walk, given a list of
/// `crates/<crate>/<dir>/<file>` leaves.
function engineDirs(root, leaves) {
  const dirs = {};
  const add = (dir, entry) => {
    dirs[dir] ??= [];
    dirs[dir].push(entry);
  };
  add(root, { name: "crates", directory: true });
  for (const leaf of leaves) {
    const parts = leaf.split("/");
    let dir = path.join(root, "crates");
    for (const part of parts.slice(0, -1)) {
      if (!(dirs[dir] ?? []).some((e) => e.name === part)) {
        add(dir, { name: part, directory: true });
      }
      dir = path.join(dir, part);
    }
    add(dir, { name: parts[parts.length - 1], directory: false });
  }
  return dirs;
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

  // ── VSCODE-OVERRIDE-NO-STALE (2026-10-15 CHAIN-Z) ──────────────────────
  // The `workspace` link is the only one in either chain that is picked by
  // **existence alone**, so it is judged before it is handed back: stale, or a
  // version that disagrees with this extension. Readings are the **number of
  // `log` lines** (the user-visible signal), and the pick must not move.
  const WS_LSP = path.join("/ws", "target", "release", "sokonanoda-lsp");
  const WS_CLI = path.join("/ws", "target", "release", "sokonanoda");
  const BUNDLED_SERVER = path.join("/ext", "bin", "darwin-arm64", "sokonanoda-lsp");

  const overrideWith = (extra) => {
    const logs = [];
    const found = server.resolveServerCommand({
      ...ARGS,
      override: true,
      roots: ["/ws"],
      log: (message) => logs.push(message),
      ...extra,
    });
    return { found, logs };
  };

  await test("resolveServerCommand: override reports a stale workspace build", () => {
    const { found, logs } = overrideWith({
      fs: fakeFs({
        existing: [WS_LSP],
        executable: [WS_LSP],
        mtimes: { [WS_LSP]: 1000, "/ws/Cargo.toml": 2000 },
      }),
    });
    assert.deepStrictEqual(found, { command: WS_LSP, source: "workspace" });
    assert.strictEqual(logs.length, 1, logs.join("\n"));
    assert.ok(logs[0].includes("早于本检出最新的引擎源"), logs[0]);
    assert.ok(logs[0].includes("Cargo.toml"), logs[0]);
    assert.ok(logs[0].includes(WS_LSP), logs[0]);
  });

  await test("resolveServerCommand: override reports a version-mismatched workspace build", () => {
    const { found, logs } = overrideWith({
      fs: fakeFs({ existing: [WS_LSP, WS_CLI], executable: [WS_LSP, WS_CLI] }),
      probeVersion: () => "0.6.0",
    });
    assert.deepStrictEqual(found, { command: WS_LSP, source: "workspace" });
    assert.strictEqual(logs.length, 1, logs.join("\n"));
    assert.ok(logs[0].includes("版本不一致"), logs[0]);
    assert.ok(logs[0].includes("v0.6.0") && logs[0].includes("v0.7.0"), logs[0]);
  });

  await test("resolveServerCommand: matching + fresh workspace build stays silent (positive control)", () => {
    const { found, logs } = overrideWith({
      fs: fakeFs({
        existing: [WS_LSP, WS_CLI],
        executable: [WS_LSP, WS_CLI],
        mtimes: { [WS_LSP]: 5000, "/ws/Cargo.toml": 1000 },
      }),
      probeVersion: () => "0.7.0",
    });
    assert.deepStrictEqual(found, { command: WS_LSP, source: "workspace" });
    assert.deepStrictEqual(logs, []);
  });

  await test("resolveServerCommand: a test-only source edit is not stale (no false alarm)", () => {
    // `crates/*/tests/**` is not compiled into the binary ⇒ a newer test file
    // must not invent a stale build (`scripts/soko::newestEngineSource` rule).
    const { logs } = overrideWith({
      fs: fakeFs({
        existing: [WS_LSP],
        executable: [WS_LSP],
        mtimes: {
          [WS_LSP]: 2000,
          "/ws/crates/front/src/lib.rs": 1000,
          "/ws/crates/front/tests/foo.rs": 3000,
        },
        dirs: engineDirs("/ws", ["front/src/lib.rs", "front/tests/foo.rs"]),
      }),
    });
    assert.deepStrictEqual(logs, []);
  });

  await test("resolveServerCommand: the crates walk does bite when real source is newer", () => {
    // Reverse direction of the test above: same fixture, `lib.rs` moved past the
    // binary ⇒ exactly one warning naming the file. Without this the guard
    // above could pass by never looking at anything.
    const { logs } = overrideWith({
      fs: fakeFs({
        existing: [WS_LSP],
        executable: [WS_LSP],
        mtimes: { [WS_LSP]: 2000, "/ws/crates/front/src/lib.rs": 3000 },
        dirs: engineDirs("/ws", ["front/src/lib.rs"]),
      }),
    });
    assert.strictEqual(logs.length, 1, logs.join("\n"));
    assert.ok(logs[0].includes(path.join("crates", "front", "src", "lib.rs")), logs[0]);
  });

  await test("resolveServerCommand: default path ignores workspace builds entirely (policy unchanged)", () => {
    // The very same stale + mismatched workspace build, with `override` off: the
    // bundled server wins and **nothing** is reported (the default policy must
    // not change, and users must not be scared by a contributor's tree).
    const logs = [];
    const found = server.resolveServerCommand({
      ...ARGS,
      roots: ["/ws"],
      log: (message) => logs.push(message),
      probeVersion: () => "0.6.0",
      fs: fakeFs({
        existing: [BUNDLED_SERVER, WS_LSP, WS_CLI],
        executable: [BUNDLED_SERVER, WS_LSP, WS_CLI],
        mtimes: { [WS_LSP]: 1000, "/ws/Cargo.toml": 2000 },
      }),
    });
    assert.deepStrictEqual(found, { command: BUNDLED_SERVER, source: "bundled" });
    assert.deepStrictEqual(logs, []);
  });

  // ── VSCODE-CLI-RESOLVER-NO-VERSION (2026-10-15 CLI-A) ──────────────────
  // `resolveCliCommand`'s chain is `bundled → workspace → PATH`, and the **last
  // two links are picked by existence alone** — the same shape the override
  // branch's `workspace` link had before CHAIN-Z. Both are now judged before
  // they are handed back. Readings are the **number of `log` lines** (the
  // user-visible signal), and the pick must not move: the reference for "who
  // runs" is the pre-fix `firstExisting(builtBinaryCandidates(...))` pair.
  const CLI_ARGS = { extensionPath: "/ext", platform: "darwin", arch: "arm64", version: "0.7.0" };

  const cliWith = (extra) => {
    const logs = [];
    const command = server.resolveCliCommand({
      ...CLI_ARGS,
      roots: ["/ws"],
      log: (message) => logs.push(message),
      ...extra,
    });
    return { command, logs };
  };

  await test("resolveCliCommand: a version-mismatched workspace CLI is reported", () => {
    const { command, logs } = cliWith({
      fs: fakeFs({ existing: [WS_CLI], executable: [WS_CLI] }),
      probeVersion: () => "0.6.0",
    });
    assert.strictEqual(command, WS_CLI, "the pick must not move");
    assert.strictEqual(logs.length, 1, logs.join("\n"));
    assert.ok(logs[0].includes("版本不一致"), logs[0]);
    assert.ok(logs[0].includes("v0.6.0") && logs[0].includes("v0.7.0"), logs[0]);
    assert.ok(logs[0].includes("sokonanoda-cli"), logs[0]);
  });

  await test("resolveCliCommand: a workspace CLI older than the engine source is reported", () => {
    const { command, logs } = cliWith({
      fs: fakeFs({
        existing: [WS_CLI],
        executable: [WS_CLI],
        mtimes: { [WS_CLI]: 1000, "/ws/Cargo.toml": 2000 },
      }),
      probeVersion: () => "0.7.0",
    });
    assert.strictEqual(command, WS_CLI);
    assert.strictEqual(logs.length, 1, logs.join("\n"));
    assert.ok(logs[0].includes("早于本检出最新的引擎源"), logs[0]);
    assert.ok(logs[0].includes("Cargo.toml"), logs[0]);
  });

  await test("resolveCliCommand: matching + fresh workspace CLI stays silent (positive control)", () => {
    const { command, logs } = cliWith({
      fs: fakeFs({
        existing: [WS_CLI],
        executable: [WS_CLI],
        mtimes: { [WS_CLI]: 5000, "/ws/Cargo.toml": 1000 },
      }),
      probeVersion: () => "0.7.0",
    });
    assert.strictEqual(command, WS_CLI);
    assert.deepStrictEqual(logs, []);
  });

  await test("resolveCliCommand: the probe asks the CLI, never the server (no 15s hang)", () => {
    // `binaryName()` returns the **server** name; asking *it* for `--version`
    // starts an LSP on stdio and blocks `spawnSync` for its whole 15s timeout
    // (CHAIN-Z's first attempt). Both names exist in the tree here, so a wrong
    // implementation would visibly probe the LSP.
    const seen = [];
    const { logs } = cliWith({
      fs: fakeFs({ existing: [WS_CLI, WS_LSP], executable: [WS_CLI, WS_LSP] }),
      probeVersion: (binary) => {
        seen.push(binary);
        return "0.6.0";
      },
    });
    assert.deepStrictEqual(seen, [WS_CLI]);
    assert.ok(!seen[0].endsWith("sokonanoda-lsp"), seen[0]);
    assert.strictEqual(logs.length, 1, logs.join("\n"));
  });

  await test("resolveCliCommand: a CLI that cannot answer --version stays silent (cannot tell)", () => {
    // `versionMatches` returning `undefined` is "cannot tell", never "mismatch"
    // — warning on it would be noise, not a signal.
    const { logs } = cliWith({
      fs: fakeFs({ existing: [WS_CLI], executable: [WS_CLI] }),
      probeVersion: () => undefined,
    });
    assert.deepStrictEqual(logs, []);
  });

  await test("resolveCliCommand: bundled CLI wins and nothing is reported (policy unchanged)", () => {
    // The same stale + mismatched workspace CLI as above, with a bundled CLI
    // present: bundled wins and **nothing** is reported (the bundled CLI is
    // built from the same tag as the extension — no judgement needed, and a dev
    // checkout being newer must not scare anyone about the released engine).
    const bundledCli = path.join("/ext", "bin", "darwin-arm64", "sokonanoda");
    const { command, logs } = cliWith({
      fs: fakeFs({
        existing: [bundledCli, WS_CLI],
        executable: [bundledCli, WS_CLI],
        mtimes: { [WS_CLI]: 1000, "/ws/Cargo.toml": 2000 },
      }),
      probeVersion: () => "0.6.0",
    });
    assert.strictEqual(command, bundledCli);
    assert.deepStrictEqual(logs, []);
  });

  await test("resolveCliCommand: a mismatched CLI on PATH is reported (version only)", () => {
    const logs = [];
    const command = server.resolveCliCommand({
      ...CLI_ARGS,
      roots: [],
      log: (message) => logs.push(message),
      probeVersion: () => "0.6.0",
      fs: fakeFs(),
    });
    assert.strictEqual(command, "sokonanoda", "the PATH fallback must stay the fallback");
    assert.strictEqual(logs.length, 1, logs.join("\n"));
    assert.ok(logs[0].includes("PATH 上的"), logs[0]);
    assert.ok(logs[0].includes("版本不一致"), logs[0]);
  });

  await test("resolveCliCommand: a matching CLI on PATH stays silent (positive control)", () => {
    const logs = [];
    const command = server.resolveCliCommand({
      ...CLI_ARGS,
      roots: [],
      log: (message) => logs.push(message),
      probeVersion: () => "0.7.0",
      fs: fakeFs(),
    });
    assert.strictEqual(command, "sokonanoda");
    assert.deepStrictEqual(logs, []);
  });

  await test("resolveCliCommand: no log callback stays silent and never throws", () => {
    let command;
    assert.doesNotThrow(() => {
      command = server.resolveCliCommand({
        ...CLI_ARGS,
        roots: ["/ws"],
        probeVersion: () => "0.6.0",
        fs: fakeFs({
          existing: [WS_CLI],
          executable: [WS_CLI],
          mtimes: { [WS_CLI]: 1000, "/ws/Cargo.toml": 2000 },
        }),
      });
    });
    assert.strictEqual(command, WS_CLI);
  });

  await test("resolveCliCommand: the pick is byte-for-byte the old firstExisting() pair", () => {
    // The reference is the pre-fix expression, evaluated from the same exported
    // helpers ⇒ a future edit that re-orders the chain or drops a fallback fails
    // here, not in a user's course map.
    const fsImpl = fakeFs({ existing: [WS_CLI], executable: [WS_CLI] });
    const old = server.firstExisting(server.builtBinaryCandidates(["/ws"], "sokonanoda"), fsImpl);
    const logs = [];
    const command = server.resolveCliCommand({
      ...CLI_ARGS,
      roots: ["/ws"],
      log: (message) => logs.push(message),
      probeVersion: () => "0.6.0",
      fs: fsImpl,
    });
    assert.strictEqual(old, WS_CLI);
    assert.strictEqual(command, old);
  });

  await test("resolveCliCommand: the workspace CLI lookup uses the Windows .exe name", () => {
    const winCli = path.join("/ws", "target", "release", "sokonanoda.exe");
    const seen = [];
    const logs = [];
    const command = server.resolveCliCommand({
      extensionPath: "/ext",
      roots: ["/ws"],
      platform: "win32",
      arch: "x64",
      version: "0.7.0",
      log: (message) => logs.push(message),
      probeVersion: (binary) => {
        seen.push(binary);
        return "0.6.0";
      },
      fs: fakeFs({ existing: [winCli], executable: [winCli] }),
    });
    assert.strictEqual(command, winCli);
    assert.deepStrictEqual(seen, [winCli]);
    assert.strictEqual(logs.length, 1, logs.join("\n"));
  });

  await test("resolveServerCommand: no bundled falls back to cache only when current", () => {
    const previousHome = process.env.HOME;
    process.env.HOME = path.join("/home", "test");
    try {
      const dest = server.serverDest();
      const marker = server.serverVersionMarker();
      // ⚠ marker 是**规范两字段** `<version> <vsce-target>`（ARGS = darwin/arm64）
      // —— 与写端 `writeServerMarker` / 启动器 `markerExpected` 同一份词汇
      // （洞 `LSP-SLOT-CROSS-SEAM-DRIFT`：旧断言钉的是单字段 `"0.7.0\n"`，
      // 正是"写端只写单字段"这个漂移被冻结下来的那一处 ⇒ 跟着改 ✓）。
      const current = fakeFs({
        existing: [dest, marker],
        executable: [dest],
        contents: { [marker]: "0.7.0 darwin-arm64\n" },
      });
      assert.deepStrictEqual(server.resolveServerCommand({ ...ARGS, fs: current }), {
        command: dest,
        source: "cache",
      });
      // A stale marker must NOT be reused (caller then downloads).
      const stale = fakeFs({
        existing: [dest, marker],
        executable: [dest],
        contents: { [marker]: "0.6.0 darwin-arm64\n" },
      });
      assert.strictEqual(server.resolveServerCommand({ ...ARGS, fs: stale }).command, undefined);
      // 同版本、**错平台**同样不许复用 —— target 正是防这一半（不许放宽 ✓）。
      const wrongTarget = fakeFs({
        existing: [dest, marker],
        executable: [dest],
        contents: { [marker]: "0.7.0 linux-x64\n" },
      });
      assert.strictEqual(server.resolveServerCommand({ ...ARGS, fs: wrongTarget }).command, undefined);
      // 单字段（旧写端的产物）也不许复用：两端词汇必须一致，否则启动器会拒它 ✗。
      const singleField = fakeFs({
        existing: [dest, marker],
        executable: [dest],
        contents: { [marker]: "0.7.0\n" },
      });
      assert.strictEqual(
        server.resolveServerCommand({ ...ARGS, fs: singleField }).command,
        undefined,
      );
      // No bundled, no cache -> undefined (caller then downloads).
      assert.strictEqual(server.resolveServerCommand({ ...ARGS, fs: fakeFs() }).command, undefined);
    } finally {
      process.env.HOME = previousHome;
    }
  });

  await test("LSP marker vocabulary: one source for the write and the read end", () => {
    // 规范 = `crates/cli/src/env/target.rs` 的 `expected_marker`（`<version> <vsce-target>`）。
    assert.strictEqual(server.serverMarkerValue("0.7.0", "darwin", "arm64"), "0.7.0 darwin-arm64");
    assert.strictEqual(server.serverMarkerValue("0.7.0", "darwin", "x64"), "0.7.0 darwin-x64");
    assert.strictEqual(server.serverMarkerValue("0.7.0", "linux", "x64", true), "0.7.0 alpine-x64");
    assert.strictEqual(server.serverMarkerValue("0.7.0", "win32", "arm64"), "0.7.0 win32-arm64");
    // 平台不支持 ⇒ 没有词汇（调用方按"没有可复用条目"处理）。
    assert.strictEqual(server.serverMarkerValue("0.7.0", "linux", "ia32"), undefined);

    // **跨端往返**：写端写什么 ⇒ 读端必须认什么（洞 `LSP-SLOT-CROSS-SEAM-DRIFT`
    // 的接缝守卫 —— 旧实现两端各自自洽，这条往返才会红 ✓）。用真 fs + 临时 HOME。
    const previousHome = process.env.HOME;
    const home = fs.mkdtempSync(path.join(os.tmpdir(), "soko-marker-"));
    process.env.HOME = home;
    try {
      const dest = server.serverDest();
      fs.mkdirSync(path.dirname(dest), { recursive: true });
      fs.writeFileSync(dest, "#!/bin/sh\n");
      const written = server.writeServerMarker("0.7.0", "darwin", "arm64");
      assert.strictEqual(written, "0.7.0 darwin-arm64");
      assert.strictEqual(fs.readFileSync(server.serverVersionMarker(), "utf8"), "0.7.0 darwin-arm64\n");
      assert.strictEqual(
        server.cachedServerIsCurrent({ version: "0.7.0", platform: "darwin", arch: "arm64" }),
        true,
      );
      // 版本不符 / 平台不符 / 单字段（旧写端）⇒ 都不认。
      assert.strictEqual(
        server.cachedServerIsCurrent({ version: "0.6.0", platform: "darwin", arch: "arm64" }),
        false,
      );
      assert.strictEqual(
        server.cachedServerIsCurrent({ version: "0.7.0", platform: "linux", arch: "x64" }),
        false,
      );
      fs.writeFileSync(server.serverVersionMarker(), "0.7.0\n");
      assert.strictEqual(
        server.cachedServerIsCurrent({ version: "0.7.0", platform: "darwin", arch: "arm64" }),
        false,
      );
    } finally {
      process.env.HOME = previousHome;
      fs.rmSync(home, { recursive: true, force: true });
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

  // ── 版本切换检测（2026-10-11 用户要求：最小编译策略在版本切换时自动变 rebuild）──
  //
  // 判定依据**只有条目文件名**（`<编译器>+<构建戳>+<键>.json`）——`meta.json.compiler`
  // 会被任何一次写入刷成当前版本（G-106 的谎）⇒ 照它判会漏判"247 条旧 + 1 条新"。
  await test("entryWrittenByOtherVersion: 只认条目文件名里的编译器版本", () => {
    const current = "0.88.1";
    assert.strictEqual(
      server.entryWrittenByOtherVersion("0.88.0+209c01a9b479e460+abc.json", current),
      true,
      "旧版本写的条目必须被认出来（触发全量预热）",
    );
    assert.strictEqual(
      server.entryWrittenByOtherVersion("0.88.1+209c01a9b479e460+abc.json", current),
      false,
      "当前版本写的条目不是证据",
    );
    // ⚠ **同版本、不同构建戳 ⇒ 不算**（与面板 `artifacts.stale` 的有意差异）：
    // G-112 是"同版本换构建"，不是"版本切换"（用户 2026-10-11 明确只要后者）。
    assert.strictEqual(
      server.entryWrittenByOtherVersion("0.88.1+751385453009d556+abc.json", current),
      false,
      "同版本不同构建戳不算版本切换（G-112 那条另算）",
    );
    // 形状不认识 ⇒ 不猜（旧命名 / 写了一半的 tmp / 别的文件）。
    assert.strictEqual(server.entryWrittenByOtherVersion("abc.json", current), false);
    assert.strictEqual(server.entryWrittenByOtherVersion("0.88.0+abc.json", current), false);
    assert.strictEqual(
      server.entryWrittenByOtherVersion("0.88.0+abc+def.json.tmp-1", current),
      false,
    );
    assert.strictEqual(server.entryWrittenByOtherVersion("0.88.0+abc+def.json", ""), false);
  });

  await test("projectArtifactsWrittenByOtherVersion: 只读目录、缺失即 false", () => {
    const root = "/proj";
    const dir = path.join(root, ".sokonanoda", "compiled");
    const fsWith = (names) => ({
      readdirSync: (p) => {
        if (p !== dir) throw Object.assign(new Error("ENOENT"), { code: "ENOENT" });
        return names;
      },
    });
    assert.strictEqual(
      server.projectArtifactsWrittenByOtherVersion(
        root,
        "0.88.1",
        fsWith(["0.88.1+aaaa+1.json", "0.88.0+bbbb+2.json"]),
      ),
      true,
      "混合目录（升级后只重编了一部分）也是旧版本 ⇒ 必须预热",
    );
    assert.strictEqual(
      server.projectArtifactsWrittenByOtherVersion(
        root,
        "0.88.1",
        fsWith(["0.88.1+aaaa+1.json", "0.88.1+bbbb+2.json"]),
      ),
      false,
      "全是当前版本 ⇒ 不动",
    );
    assert.strictEqual(
      server.projectArtifactsWrittenByOtherVersion(root, "0.88.1", fsWith([])),
      false,
      "空目录 ⇒ 不动",
    );
    // 目录不存在（没编过）⇒ false，且**绝不创建**（只读探针）。
    assert.strictEqual(
      server.projectArtifactsWrittenByOtherVersion(root, "0.88.1", {
        readdirSync: () => {
          throw new Error("ENOENT");
        },
      }),
      false,
      "没有产物目录 ⇒ 谈不上旧版本",
    );
    assert.strictEqual(
      server.projectArtifactsWrittenByOtherVersion(undefined, "0.88.1", fsWith(["x"])),
      false,
      "没有模块根 ⇒ 不动",
    );
  });

  console.log(`\n${passed + failed} tests, ${passed} passed, ${failed} failed\n`);
  process.exit(failed > 0 ? 1 : 0);
}

run();
