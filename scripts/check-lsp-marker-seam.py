#!/usr/bin/env python3
"""LSP 槽位 marker 的**跨接缝**守卫（洞 `LSP-MARKER-NO-COMMITTED-GUARD`，2026-10-15 CMT-G）。

## 为什么需要它

同一个缓存 marker 有**一个规范 + 一个写端 + 三个读点**，分居两门语言、三份文件：

| 角色 | 位置 |
|---|---|
| 规范（唯一词汇） | `crates/cli/src/env/target.rs` `expected_marker()` = `<version> <vsce-target>` |
| 写端（LSP 槽位） | `editor/vscode/server.js` `serverMarkerValue` → `writeServerMarker`（`downloadLspBinary` 调它） |
| 读点 ① | `editor/vscode/server.js` `cachedServerIsCurrent` |
| 读点 ② | `scripts/soko` `markerExpected`（`markerMatches` 调它） |
| 读点 ③ | `editor/vscode/extension.js` doctor 5/6 |

2026-10-15 的 lspF 轮（`1b6b7eb5`）把 ①③ 与写端**一次改齐**到规范两字段（此前写端写裸
`<version>`、① 也按裸 `<version>` 比 ⇒ 扩展自己下进缓存的 LSP 被启动器判 `cache(STALE: …)`
并**拒绝 exec** ✗），但**跨接缝探针只活在 `.cache/ux/fix-lspF/probe-lsp-cross-seam.mjs`**
（`.cache/**` 不 commit）⇒ 对齐做完了、**没有守卫钉住它**：三处读点随时可以再漂回去 ✗
（`AGENTS.md`：**咬不住的守卫等于没有**）。本文件把那个探针收进 `scripts/`（`gate` 里跑），
并补 `--selftest`（把旧写法放回去，每一处都必须判红并**点名** ✓）。

## 判据（六条；C1–C5 是「用户动作 ⇒ 可见结果」，C6 是读点③的同源断言）

| # | 夹具 | 期望 | 咬住 |
|---|---|---|---|
| C1 | 真写端（`downloadLspBinary` 打**本机 http**，临时 HOME/缓存）写出 marker ⇒ **启动器**读它 | `version --json` 的 `lsp.source == "cache"` **且** `scripts/soko lsp` rc=0、哨兵真跑 | 写端 ↔ 读点② |
| C2 | 同一份缓存 ⇒ **扩展自己**的读点 | `cachedServerIsCurrent` = `true` **且**再下一次 http 请求数 **0**（没有「每次启动重下」） | 读点① |
| C3 | 负对照：**版本不匹配**（`<别的版本> <本机 target>`） | 两端都拒：启动器 rc=3 且哨兵没跑 · 扩展读点 `false` | 「版本那一半不许松」 |
| C4 | 负对照：**同版本、错平台** | 两端都拒（同上） | 「`fields[1] !== target` 那一半不许松」= markC 明确**不选**的「收单字段」那条 |
| C5 | 反向：缓存按**启动器 / `sokonanoda setup`** 的写法预置（规范两字段），扩展没下过 | 扩展认得出 ⇒ 再下一次 **0** 请求（不重下） | 读点① 的反方向 |
| C6 | 读点③（doctor 5/6）**同源**：期望值来自 `server.serverMarkerValue(...)`、按**整串**比 | 不拿裸 `extVersion` 比、不拿 `marker === extVersion` | 读点③（`extension.js:1931`） |
| C7 | **横向**（硬规则 0(b)）：同一份契约的**第二个槽位**（CLI）—— 扩展「Install CLI」写端 `installBundledCli` 写出 marker ⇒ 启动器读它 | `version --json` 的 `cli.source == "cache"` **且** `scripts/soko grade` rc=0、哨兵真跑 | CLI 槽位写端（markC `a5feb8b5`） |

⚠ **C7 是横向排查收进来的**（硬规则 0(b)「同类问题横向排查，不许修单点」）：同一个洞族
`INSTALL-BUNDLED-CLI-MARKER-DRIFT` 的证据也只活在 `.cache/ux/fix-markC/fixture-marker.py`
⇒ 一并收进常驻守卫 ✓。今天**没有任何 committed 判据**盖住这一格：`test-server.js` 只盖
LSP 槽位（`cachedServerIsCurrent` / `serverMarkerValue` / `writeServerMarker`），
`test-extension-host.js` 把 `installBundledCli` 整个 stub 掉 ⇒ 都咬不到它 ✗。

⚠ **C6 是结构性断言，不是行为读数**（如实标注 ✗）：跑 `runDoctor` 需要**完整 VS Code 宿主
stub**，而 `editor/vscode/test-extension-host.js` 把 `./server` 整个换成了 stub
（`Module._load` 那条 `request === "./server"`）⇒ 现有宿主层**咬不到**真
`serverMarkerValue`，而本轮**不许动 `editor/vscode/**`** ✗。C6 因此断言"期望值来自哪个
函数、按什么比"，并靠 `--selftest` 的旧文反例证明它真的会红 ✓。

⚠ 夹具自带 `HOME` + `SOKONANODA_CACHE_DIR` + 空 `CARGO_TARGET_DIR` + `SOKONANODA_OFFLINE=1`
+ 两个 `NO_*ARTIFACTS` ⇒ 一个字节都不碰真环境 / 真缓存 / 真网络（Release 基址指向本机
`http://127.0.0.1:<随机端口>`）✓。

## 用法

    python3 scripts/check-lsp-marker-seam.py             # 判据（gate）
    python3 scripts/check-lsp-marker-seam.py --json      # 机器可读
    python3 scripts/check-lsp-marker-seam.py --selftest  # 反向验证：四处旧写法各自必须判红

退出码：0 = 跨接缝一致 · 1 = **判红**（漂移 / 语义松了）· 2 = 形状异常（夹具或反例造不出来）
        · 3 = 拿不到 `node`/`tar` 或仓库形状不对（与 gate 其它步同一条纪律：探不到就 exit 3，
        **绝不静默通过** ✗）。
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve()
REPO = HERE.parent.parent
LAUNCHER = REPO / "scripts" / "soko"
SERVER_JS = REPO / "editor" / "vscode" / "server.js"
EXTENSION_JS = REPO / "editor" / "vscode" / "extension.js"

# 夹具自己的版本钉（夹具仓没有 `Cargo.toml` ⇒ 走 `sokonanoda-version.txt` ✓）。
# **有意不读仓库版本**：判据与"今天是哪个版本"无关 ⇒ 不会每次 bump 就假红 ✓。
PIN = "0.90.0"
MISMATCH = "0.89.0"

WINDOWS = sys.platform == "win32"
SENTINEL = "SENTINEL-CACHED-LSP-EXECUTED"
CLI_SENTINEL = "SENTINEL-CACHED-CLI-EXECUTED"

# ── 跨接缝夹具驱动（临时目录里落盘后由 `node` 跑）─────────────────────────────
# 它只做一件事：用**真写端**往夹具 HOME 的缓存里写 marker，并把两端各自的读数打成 JSON。
# 判据（哪条算红）全在下面的 Python 里 ⇒ 这个驱动**不是**第二个真相，只是夹具 ✓。
DRIVER = r'''
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { execFileSync } from "node:child_process";
import module from "node:module";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  options: {
    server: { type: "string" },
    home: { type: "string" },
    case: { type: "string" },
    version: { type: "string" },
    mismatch: { type: "string" },
    out: { type: "string" },
  },
});

const SENTINEL = "SENTINEL-CACHED-LSP-EXECUTED";
const home = path.resolve(values.home);
const cache = path.join(home, ".local", "share", "sokonanoda", "bin");
const binName = process.platform === "win32" ? "sokonanoda-lsp.exe" : "sokonanoda-lsp";
const dest = path.join(cache, binName);
const markerPath = dest + ".version";
fs.mkdirSync(cache, { recursive: true });

// 夹具 payload：tar.gz 里一个哨兵脚本（真下载路的解包对象）。
const stage = path.join(home, "stage");
fs.mkdirSync(stage, { recursive: true });
const staged = path.join(stage, binName);
fs.writeFileSync(staged, `#!/bin/sh\necho ${SENTINEL}\n`);
fs.chmodSync(staged, 0o755);
const tarball = path.join(home, "fixture.tar.gz");
execFileSync("tar", ["czf", tarball, "-C", stage, binName]);

let requests = 0;
const payload = fs.readFileSync(tarball);
const srv = http.createServer((_req, res) => {
  requests += 1;
  res.writeHead(200, { "content-type": "application/gzip" });
  res.end(payload);
});
await new Promise((resolve) => srv.listen(0, "127.0.0.1", resolve));
const base = `http://127.0.0.1:${srv.address().port}/releases`;

// 只改 Release 基址、其余一字节不动地加载**真** `server.js`。
const serverPath = path.resolve(values.server);
const src = fs
  .readFileSync(serverPath, "utf8")
  .replace(/const RELEASES_BASE = "[^"]*"/, `const RELEASES_BASE = ${JSON.stringify(base)}`);
const mod = new module.Module(serverPath);
mod.filename = serverPath;
mod.paths = module.Module._nodeModulePaths(path.dirname(serverPath));
mod._compile(src, serverPath);
const server = mod.exports;

// `serverCacheDir()` 读 `process.env.HOME` ⇒ 必须在调用前就指到夹具 HOME。
process.env.HOME = home;

const version = values.version;
const platform = process.platform;
const arch = process.arch;
const alpine = server.isAlpineLinux(platform);
const opts = { version, platform, arch };
const target = server.platformTarget(platform, arch, alpine);
const readMarker = () => (fs.existsSync(markerPath) ? fs.readFileSync(markerPath, "utf8") : null);
const extRead = () => server.cachedServerIsCurrent({ version, platform, arch, alpine });
const resolveSource = () =>
  server.resolveServerCommand({
    setting: undefined,
    envBin: undefined,
    extensionPath: path.join(home, "no-such-extension"),
    roots: [],
    platform,
    arch,
    version,
  }).source;

let wroteMarker = null;
let reDownloads = null;
let cliMarker = null;
let cliDest = null;

if (values.case === "cli-install") {
  // 同一份 marker 契约的**第二个槽位**：扩展的「Install CLI」写端 `installBundledCli`
  // （洞 `INSTALL-BUNDLED-CLI-MARKER-DRIFT` 修在 markC `a5feb8b5`）。它的守卫同样
  // 只活在 `.cache/ux/fix-markC/` ⇒ 一并收进来 ✓。
  const extPath = path.join(
    home, ".vscode", "extensions", `sokonanoda-lang.sokonanoda-${version}-${target}`,
  );
  const binDir = path.join(extPath, "bin", target);
  fs.mkdirSync(binDir, { recursive: true });
  const cliName = process.platform === "win32" ? "sokonanoda.exe" : "sokonanoda";
  const cliSrc = path.join(binDir, cliName);
  fs.writeFileSync(cliSrc, `#!/bin/sh\necho SENTINEL-CACHED-CLI-EXECUTED\n`);
  fs.chmodSync(cliSrc, 0o755);
  const installed = server.installBundledCli({ extensionPath: extPath, platform, arch, version });
  cliDest = installed.dest ?? null;
  cliMarker = cliDest ? fs.readFileSync(cliDest + ".version", "utf8") : null;
} else if (values.case === "launcher-written") {
  // 反向：缓存按**启动器 / `sokonanoda setup`** 的写法预置（规范两字段），扩展没下过。
  fs.copyFileSync(staged, dest);
  fs.chmodSync(dest, 0o755);
  fs.writeFileSync(markerPath, `${version} ${target}\n`);
  wroteMarker = readMarker();
  const before = requests;
  await server.downloadLspBinary(opts); // 认得出 ⇒ 提前返回：0 请求、不重写 marker
  reDownloads = requests - before;
} else {
  await server.downloadLspBinary(opts);
  wroteMarker = readMarker();
  if (values.case === "version-mismatch") {
    fs.writeFileSync(markerPath, `${values.mismatch} ${target}\n`);
  } else if (values.case === "wrong-target") {
    const other = target === "linux-x64" ? "darwin-arm64" : "linux-x64";
    fs.writeFileSync(markerPath, `${version} ${other}\n`);
  } else {
    // 「会不会每次重下」**最后**量：这一步可能把 marker 写回规范值 ⇒ 必须先测完上面那些 ✓。
    const before = requests;
    await server.downloadLspBinary(opts);
    reDownloads = requests - before;
  }
}

const isCli = values.case === "cli-install";
fs.writeFileSync(
  values.out,
  JSON.stringify(
    {
      sentinel: SENTINEL,
      cache,
      dest,
      markerPath,
      target,
      wroteMarker,
      markerOnDisk: readMarker(),
      extReads: isCli ? null : extRead(),
      resolvedSource: isCli ? null : resolveSource(),
      reDownloads,
      cliDest,
      cliMarker,
    },
    null,
    2,
  ),
);
srv.close();
'''


def _env(home: Path, cache: Path, target_dir: Path) -> dict[str, str]:
    """最小环境：真环境里的 `SOKONANODA_*` / 真 `HOME` 一律不带进去 ✓。"""
    env = {
        "PATH": os.environ.get("PATH", ""),
        "HOME": str(home),
        "USERPROFILE": str(home),  # Windows 上启动器读的是它
        "SOKONANODA_CACHE_DIR": str(cache),
        "CARGO_TARGET_DIR": str(target_dir),
        "SOKONANODA_OFFLINE": "1",
        # 夹具纪律：两处产物一律关掉（不许往任何模块根写 `.sokonanoda/`）。
        "SOKONANODA_NO_PROJECT_ARTIFACTS": "1",
        "SOKONANODA_NO_MODULE_ARTIFACTS": "1",
    }
    if os.environ.get("SystemRoot"):
        env["SystemRoot"] = os.environ["SystemRoot"]  # node 在 Windows 上要它
    return env


class Fixture:
    """一个自足的假课程仓：`repo/scripts/soko` + 版本钉 + 假 HOME / 缓存 / 空 target。"""

    def __init__(self, root: Path, launcher: Path):
        self.root = root
        self.home = root / "home"
        self.cache = self.home / ".local" / "share" / "sokonanoda" / "bin"
        self.empty_target = root / "empty-target"
        self.repo = root / "repo"
        for path in (self.home, self.cache, self.empty_target, self.repo / "scripts"):
            path.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(launcher, self.repo / "scripts" / "soko")
        (self.repo / "sokonanoda-version.txt").write_text(f"{PIN}\n", encoding="utf-8")

    def run(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["node", str(self.repo / "scripts" / "soko"), *args],
            capture_output=True,
            text=True,
            env=_env(self.home, self.cache, self.empty_target),
            timeout=120,
        )

    def seam(self, driver: Path, server: Path, case: str) -> dict:
        """跑一次真写端夹具，拿回跨接缝读数（`--out` 的 JSON）。"""
        out = self.root / f"seam-{case}.json"
        proc = subprocess.run(
            [
                "node", str(driver),
                "--server", str(server),
                "--home", str(self.home),
                "--case", case,
                "--version", PIN,
                "--mismatch", MISMATCH,
                "--out", str(out),
            ],
            capture_output=True,
            text=True,
            env=_env(self.home, self.cache, self.empty_target),
            timeout=180,
        )
        if not out.is_file():
            raise RuntimeError(
                f"跨接缝夹具（case={case}）没给出读数：rc={proc.returncode} "
                f"stderr={proc.stderr.strip()[:300]}"
            )
        return json.loads(out.read_text(encoding="utf-8"))


def _launcher_readings(fx: Fixture) -> tuple[str | None, int, bool]:
    """启动器读端的读数：`version --json` 的 `lsp.source` · `lsp` 的 rc · 哨兵跑没跑。"""
    out = fx.run("version", "--json")
    source = None
    if out.returncode == 0 and out.stdout.strip():
        try:
            source = json.loads(out.stdout).get("lsp", {}).get("source")
        except (ValueError, AttributeError):
            source = None
    if WINDOWS:
        # 夹具哨兵是 `sh` 脚本（跑不了）⇒ 退化成 `doctor --json` 的 `lsp.ready`
        # （**仍然**是「exec 闸门开了」的读数），并如实标注这个降级 ✓。
        doc = fx.run("doctor", "--json")
        try:
            ready = bool(json.loads(doc.stdout).get("lsp", {}).get("ready"))
        except (ValueError, AttributeError):
            ready = False
        return source, 0 if ready else 3, ready
    proc = fx.run("lsp")
    return source, proc.returncode, SENTINEL in proc.stdout


def check_all(launcher: Path, server: Path, root: Path, driver: Path) -> tuple[list[dict], str | None]:
    """六条判据。返回 (results, fatal)；`fatal` 非空 = 形状异常（exit 2）。"""
    results: list[dict] = []

    def add(cid: str, what: str, ok: bool, detail: str) -> None:
        results.append({"id": cid, "what": what, "ok": bool(ok), "detail": detail})

    # C1/C2 —— 洞本身：真写端写出规范 marker ⇒ 启动器与扩展**都**认
    fx = Fixture(root / "c1", launcher)
    seam = fx.seam(driver, server, "download")
    source, rc, ran = _launcher_readings(fx)
    target = seam.get("target")
    canonical = f"{PIN} {target}\n"
    add(
        "C1",
        f"真写端写出的 marker ⇒ 启动器认（`lsp.source=cache` + `lsp` rc=0 且哨兵真跑）",
        seam.get("wroteMarker") == canonical and source == "cache" and rc == 0 and ran,
        f"写出的 marker={seam.get('wroteMarker')!r} 期望={canonical!r} · 启动器 source={source!r} "
        f"rc={rc} 哨兵={'跑了' if ran else '没跑'}",
    )
    add(
        "C2",
        "同一份缓存 ⇒ 扩展自己的读点认（`cachedServerIsCurrent` true + 再下一次 0 请求）",
        seam.get("extReads") is True and seam.get("reDownloads") == 0,
        f"cachedServerIsCurrent={seam.get('extReads')!r} · resolveServerCommand.source="
        f"{seam.get('resolvedSource')!r} · 再下一次 http 请求数={seam.get('reDownloads')!r}",
    )

    # C3 —— 不许松：版本那一半
    fx = Fixture(root / "c3", launcher)
    seam = fx.seam(driver, server, "version-mismatch")
    source, rc, ran = _launcher_readings(fx)
    add(
        "C3",
        f"负对照：marker 版本={MISMATCH}（≠ 钉 v{PIN}）⇒ 两端都拒",
        seam.get("extReads") is False and (source or "").startswith("cache(STALE") and rc == 3 and not ran,
        f"cachedServerIsCurrent={seam.get('extReads')!r} · 启动器 source={source!r} rc={rc} "
        f"哨兵={'跑了（✗ 不该跑）' if ran else '没跑'}",
    )

    # C4 —— 不许松：target 那一半（= markC 明确**不选**的「启动器收单字段」）
    fx = Fixture(root / "c4", launcher)
    seam = fx.seam(driver, server, "wrong-target")
    source, rc, ran = _launcher_readings(fx)
    add(
        "C4",
        "负对照：**同版本、错平台** ⇒ 两端都拒（target 那一半不许松）",
        seam.get("extReads") is False and (source or "").startswith("cache(STALE") and rc == 3 and not ran,
        f"盘上 marker={seam.get('markerOnDisk')!r} · cachedServerIsCurrent={seam.get('extReads')!r} · "
        f"启动器 source={source!r} rc={rc} 哨兵={'跑了（✗ 不该跑）' if ran else '没跑'}",
    )

    # C5 —— 反方向：启动器/`setup` 写的那份，扩展认不认
    fx = Fixture(root / "c5", launcher)
    seam = fx.seam(driver, server, "launcher-written")
    add(
        "C5",
        "反向：缓存按启动器/`sokonanoda setup` 的写法预置 ⇒ 扩展认得出（0 重下）",
        seam.get("extReads") is True and seam.get("reDownloads") == 0,
        f"预置 marker={seam.get('wroteMarker')!r} · cachedServerIsCurrent={seam.get('extReads')!r} · "
        f"再下一次 http 请求数={seam.get('reDownloads')!r}",
    )

    # C6 —— 读点③（doctor 5/6）**同源**：期望值来自 `server.serverMarkerValue`、按整串比
    # ⚠ 结构性断言（见模块抬头「C6 是结构性断言」那段：跑 runDoctor 要完整宿主 stub，
    #    而本轮不许动 `editor/vscode/**` ⇒ 宿主层那条路走不通）。
    text = EXTENSION_JS.read_text(encoding="utf-8")
    start = text.find("// 5. download cache marker")
    end = text.find("// 6. old extension versions", start if start >= 0 else 0)
    block = text[start:end] if start >= 0 and end > start else ""
    same_source = "server.serverMarkerValue(" in block
    whole_string = "markerVersion === markerExpected" in block
    not_bare = "markerVersion === extVersion" not in block
    add(
        "C6",
        "读点③ doctor 5/6 同源：期望值来自 `server.serverMarkerValue`、按整串比（不拿裸 extVersion）",
        bool(block) and same_source and whole_string and not_bare,
        f"锚点段={len(block)} 字节 · serverMarkerValue={'yes' if same_source else 'NO'} · "
        f"markerVersion===markerExpected={'yes' if whole_string else 'NO'} · "
        f"裸 extVersion 比对={'NO（对）' if not_bare else 'yes（✗）'}",
    )

    # C7 —— **横向**（硬规则 0(b)）：同一份 marker 契约的**第二个槽位**（CLI）。
    # 扩展的「Install CLI」写端 `installBundledCli`（洞 `INSTALL-BUNDLED-CLI-MARKER-DRIFT`，
    # markC `a5feb8b5`）当时也**只把证据留在 `.cache/ux/fix-markC/`** ⇒ 同一个洞族、
    # 同一条守卫缺口，一并收进来 ✓（`test-server.js` 只盖 LSP 那一格，
    # `test-extension-host.js` 把 `installBundledCli` 整个 stub 掉 ⇒ 都咬不到它）。
    fx = Fixture(root / "c7", launcher)
    seam = fx.seam(driver, server, "cli-install")
    out = fx.run("version", "--json")
    try:
        cli_source = json.loads(out.stdout).get("cli", {}).get("source") if out.returncode == 0 else None
    except (ValueError, AttributeError):
        cli_source = None
    grade = fx.run("grade", "u.sokonanoda")
    grade_ran = CLI_SENTINEL in grade.stdout
    if WINDOWS:
        # 夹具哨兵是 `sh` 脚本（跑不了）⇒ 「真的跑了」退化成"闸门开了"（`cli.source=cache`），
        # 并**如实打印**这个降级 ✓（不是静默跳过 ✗）。
        cli_ran = cli_source == "cache"
        cli_reading = f"Windows：sh 哨兵跑不了 ⇒ 只读闸门 cli.source={cli_source!r} ✓"
    else:
        cli_ran = grade.returncode == 0 and grade_ran
        cli_reading = f"`grade` rc={grade.returncode} 哨兵={'跑了' if grade_ran else '没跑'}"
    add(
        "C7",
        "横向：扩展「Install CLI」写端（CLI 槽位）写出的 marker ⇒ 启动器认且真的跑它",
        seam.get("cliMarker") == canonical and cli_source == "cache" and cli_ran,
        f"installBundledCli 写出的 marker={seam.get('cliMarker')!r} 期望={canonical!r} · "
        f"启动器 cli.source={cli_source!r} · {cli_reading}",
    )

    return results, None


def _print(results: list[dict], as_json: bool) -> None:
    if as_json:
        print(json.dumps({"results": results, "ok": all(r["ok"] for r in results)}, ensure_ascii=False, indent=2))
        return
    print("LSP 槽位 marker 跨接缝守卫（scripts/check-lsp-marker-seam.py）")
    for r in results:
        print(f"  {'✓' if r['ok'] else '✗'} {r['id']} {r['what']}")
        print(f"      {r['detail']}")
    bad = [r["id"] for r in results if not r["ok"]]
    print("  ⇒ " + ("跨接缝一致 ✓" if not bad else f"判红：{', '.join(bad)} ✗"))


# ── `--selftest`：把四处旧写法放回去（`git show 1b6b7eb5^` / markC 明确**不选**的那条）──
# 每处给出（反例名 · 造文本的函数 · **必须**判红的 id）✓。**只作夹具**，不是第二个真相。
def _revert_server_read(server_text: str) -> str | None:
    """R1：读点① 还原成 lspF 之前的样子（单字段 + 旧签名 + 旧调用点）。"""
    pairs = [
        (
            "function cachedServerIsCurrent({ version, platform, arch, alpine = false }, fsImpl = fs) {",
            "function cachedServerIsCurrent(version, fsImpl = fs) {",
        ),
        (
            "  const expected = serverMarkerValue(version, platform, arch, alpine);\n"
            "  if (expected === undefined) return false;",
            "  const expected = String(version);",
        ),
        (
            "cachedServerIsCurrent({ version, platform, arch, alpine }, fsImpl)",
            "cachedServerIsCurrent(version, fsImpl)",
        ),
        (
            "cachedServerIsCurrent({ version, platform, arch, alpine })",
            "cachedServerIsCurrent(version)",
        ),
    ]
    out = server_text
    for new, old in pairs:
        if new not in out:
            return None
        out = out.replace(new, old, 1)
    return out


def _revert_doctor(extension_text: str) -> str | None:
    """R2：读点③ 还原成 lspF 之前的样子（拿裸 `extVersion` 比单字段）。"""
    new = "    if (markerVersion !== undefined && markerVersion === markerExpected) {"
    old = "    if (markerVersion === extVersion) {"
    if new not in extension_text:
        return None
    return extension_text.replace(new, old, 1)


def _revert_soko_target(launcher_text: str) -> str | None:
    """R3：读点② 还原成「启动器收单字段」（= markC 明确**不选**的那条）。"""
    line = "  if (fields[1] !== target) return false\n"
    if line not in launcher_text:
        return None
    return launcher_text.replace(line, "", 1)


def _revert_cli_writer(server_text: str) -> str | None:
    """R4：CLI 槽位写端还原成 markC 之前的样子（只写裸 `<version>`）。"""
    new = (
        "  fsImpl.writeFileSync(\n"
        "    dest + \".version\",\n"
        "    `${version} ${platformTarget(platform, arch, alpine)}\\n`,\n"
        "  );"
    )
    old = "  fsImpl.writeFileSync(dest + \".version\", `${version}\\n`);"
    if new not in server_text:
        return None
    return server_text.replace(new, old, 1)


def _selftest_cases(root: Path) -> list[tuple[str, Path, Path, tuple[str, ...]]]:
    """(反例名 · 该轮用的 launcher · 该轮用的 server.js · **必须**判红的 id)。"""
    cases: list[tuple[str, Path, Path, tuple[str, ...]]] = []

    server_text = SERVER_JS.read_text(encoding="utf-8")
    reverted = _revert_server_read(server_text)
    if reverted is not None:
        bad_server = root / "server-old-read.js"
        bad_server.write_text(reverted, encoding="utf-8")
        cases.append(("R1 `server.js` 读点① 旧写法（单字段 `=== String(version)`）",
                      LAUNCHER, bad_server, ("C2", "C5")))

    ext_text = EXTENSION_JS.read_text(encoding="utf-8")
    reverted_ext = _revert_doctor(ext_text)
    if reverted_ext is not None:
        # 读点③ 是结构性断言 ⇒ 喂反例文本（见 `check_all` 的 C6 与模块抬头）。
        cases.append(("R2 `extension.js` 读点③ 旧写法（`markerVersion === extVersion`）",
                      None, None, ("C6",)))
        (root / "extension-old-doctor.js").write_text(reverted_ext, encoding="utf-8")

    launcher_text = LAUNCHER.read_text(encoding="utf-8")
    reverted_soko = _revert_soko_target(launcher_text)
    if reverted_soko is not None:
        bad_launcher = root / "soko-single-field"
        bad_launcher.write_text(reverted_soko, encoding="utf-8")
        cases.append(("R3 `scripts/soko` 读点② 收单字段（丢掉 `fields[1] !== target`）",
                      bad_launcher, SERVER_JS, ("C4",)))

    reverted_cli = _revert_cli_writer(server_text)
    if reverted_cli is not None:
        bad_cli = root / "server-old-cli-writer.js"
        bad_cli.write_text(reverted_cli, encoding="utf-8")
        cases.append(("R4 `server.js` CLI 槽位写端旧写法（只写裸 `<version>`，markC `a5feb8b5` 前）",
                      LAUNCHER, bad_cli, ("C7",)))

    return cases


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--json", action="store_true", help="机器可读")
    ap.add_argument("--selftest", action="store_true", help="反向验证：四处旧写法必须判红并点名")
    args = ap.parse_args(argv)

    if shutil.which("node") is None:
        print("check-lsp-marker-seam: 找不到 node —— 跨接缝夹具跑不了（exit 3）✗", file=sys.stderr)
        return 3
    if shutil.which("tar") is None:
        print("check-lsp-marker-seam: 找不到 tar —— 真下载路的解包夹具跑不了（exit 3）✗", file=sys.stderr)
        return 3
    missing = [str(p) for p in (LAUNCHER, SERVER_JS, EXTENSION_JS) if not p.is_file()]
    if missing:
        print(f"check-lsp-marker-seam: 缺文件 {', '.join(missing)} ⇒ exit 3 ✗", file=sys.stderr)
        return 3

    with tempfile.TemporaryDirectory(prefix="soko-lsp-seam-") as tmp:
        root = Path(tmp)
        driver = root / "seam-driver.mjs"
        driver.write_text(DRIVER, encoding="utf-8")

        if args.selftest:
            cases = _selftest_cases(root)
            if len(cases) < 4:
                print(f"check-lsp-marker-seam --selftest: 只造出 {len(cases)}/4 个反例"
                      f"（找不到要还原的那几行）⇒ exit 2 ✗", file=sys.stderr)
                return 2
            bad = 0
            for idx, (name, launcher, server, must_be_red) in enumerate(cases):
                try:
                    if server is None:
                        # R2：读点③ 只吃文本 ⇒ 直接对着反例文件跑 C6 那一格。
                        text = (root / "extension-old-doctor.js").read_text(encoding="utf-8")
                        start = text.find("// 5. download cache marker")
                        end = text.find("// 6. old extension versions", start if start >= 0 else 0)
                        block = text[start:end] if start >= 0 and end > start else ""
                        red = set()
                        if not ("server.serverMarkerValue(" in block
                                and "markerVersion === markerExpected" in block
                                and "markerVersion === extVersion" not in block):
                            red.add("C6")
                        detail = f"判红 {sorted(red) or '（无！）'}"
                    else:
                        results, fatal = check_all(launcher, server, root / f"r{idx}", driver)
                        if fatal:
                            print(f"check-lsp-marker-seam --selftest: {fatal} ⇒ exit 2 ✗", file=sys.stderr)
                            return 2
                        red = {r["id"] for r in results if not r["ok"]}
                        detail = f"判红 {sorted(red) or '（无！）'}"
                except RuntimeError as err:
                    print(f"check-lsp-marker-seam --selftest: {err} ⇒ exit 2 ✗", file=sys.stderr)
                    return 2
                hit = [cid for cid in must_be_red if cid in red]
                ok = len(hit) == len(must_be_red)
                bad += 0 if ok else 1
                print(f"  {'✓' if ok else '✗'} 反例：{name} ⇒ {detail}"
                      f"（必须含 {', '.join(must_be_red)}）")
            if bad:
                print(f"  ⇒ **咬不住**：{bad}/4 个反例没被判红 ✗", file=sys.stderr)
                return 1
            print("  ⇒ 反向验证成立：四处旧写法各自判红并点名 ✓（守卫真的在咬）")
            return 0

        try:
            results, fatal = check_all(LAUNCHER, SERVER_JS, root / "live", driver)
        except RuntimeError as err:
            print(f"check-lsp-marker-seam: {err} ⇒ exit 2 ✗", file=sys.stderr)
            return 2
        if fatal:
            print(f"check-lsp-marker-seam: {fatal} ⇒ exit 2 ✗", file=sys.stderr)
            return 2
        _print(results, args.json)
        return 0 if all(r["ok"] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
