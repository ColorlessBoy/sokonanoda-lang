// Provision the version-pinned sokonanoda binaries and wire them into opencode
// without any shell script:
//
//   - at startup (best effort) resolve the language server — env override →
//     **版本匹配**的仓库构建（release 优先）→ **缓存（marker 对上版本钉）** →
//     VS Code 扩展自带 → 版本锁定下载;
//   - rewrite `lsp.sokonanoda.command` to the native binary via `config`;
//   - expose the same resolution's CLI directory to all shells via `shell.env`.
//
// Cross-platform (macOS/Linux/Windows): uses fetch + `tar` (present on
// Windows 10+, macOS, Linux), never bash. Failures never block the session.
// Offline opt-out: SOKONANODA_OFFLINE=1. The repo root is marked by
// `Cargo.toml` (language repo) or by the version pin / project manifest of a
// course repo; the version chain is
// `SOKONANODA_VERSION` → `sokonanoda-version.txt` → `requires` → `Cargo.toml`
// and mirrors `scripts/soko` exactly (WO-001). Design: docs/design/onboarding.md.
//
// ⚠ 2026-10-15 CHAIN-X（洞 `OPENCODE-PLUGIN-OWN-RESOLVER`）：这条链以前是**平行**的第二套
// —— 仓库构建跨 profile 只按 mtime、不查版本、没有 stale 概念，且**扩展排在缓存之前**
// ⇒ 编辑器可能选到版本不匹配或陈旧的二进制、**零信号** ✗。现在照 `scripts/soko` 的形状对齐
// （release 优先 · 版本校验 · 陈旧出声 · 缓存先于扩展 · 认 `CARGO_TARGET_DIR`/`SOKONANODA_CACHE_DIR`）。
import { spawnSync } from "node:child_process"
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs"
import { tmpdir } from "node:os"
import path from "node:path"
import type { Plugin } from "@opencode-ai/plugin"

const RELEASES = "https://github.com/ColorlessBoy/sokonanoda-lang/releases/download"

function cacheDir(): string {
  // ⚠ `SOKONANODA_CACHE_DIR` 必须认（2026-10-15 CHAIN-X）：`scripts/soko` 与 CLI
  // （`crates/cli/src/env/target.rs`）都认 ⇒ 不认就是"CLI 写到 A、插件去 B 找" ✗。
  if (process.env.SOKONANODA_CACHE_DIR) return process.env.SOKONANODA_CACHE_DIR
  const home = process.env.HOME ?? process.env.USERPROFILE ?? ""
  return path.join(home, ".local", "share", "sokonanoda", "bin")
}

function binaryName(base: string): string {
  return process.platform === "win32" ? `${base}.exe` : base
}

/// opencode may be opened in a repo subdirectory; walk up to the root. The
/// language repository is marked by `Cargo.toml`; a course repository (a
/// separate checkout that only consumes released binaries) is marked by its
/// version pin or project manifest — see `readVersion` below.
function findRepoRoot(start: string): string | undefined {
  let dir = start
  for (;;) {
    if (
      existsSync(path.join(dir, "Cargo.toml")) ||
      existsSync(path.join(dir, "sokonanoda-version.txt")) ||
      existsSync(path.join(dir, "sokonanoda.toml"))
    )
      return dir
    const parent = path.dirname(dir)
    if (parent === dir) return undefined
    dir = parent
  }
}

/// Version pin/"constraint" of the repository — the same chain `scripts/soko`
/// implements (WO-001), so the two entry points never disagree:
/// `$SOKONANODA_VERSION` → `<repo>/sokonanoda-version.txt` →
/// `<repo>/sokonanoda.toml`'s `requires` → `<repo>/Cargo.toml`.
/// `version` is only set for a *complete* x.y.z (the value a release tag can be
/// built from); `constraint` may be a major.minor requirement.
function readVersionSource(repo: string): { version?: string; constraint?: string } {
  const full = (text: string | undefined): string | undefined => {
    const match = /^v?(\d+)\.(\d+)\.(\d+)$/.exec(String(text ?? "").trim())
    return match ? `${match[1]}.${match[2]}.${match[3]}` : undefined
  }
  const read = (name: string): string | undefined => {
    try {
      return readFileSync(path.join(repo, name), "utf8")
    } catch {
      return undefined
    }
  }
  const env = full(process.env.SOKONANODA_VERSION)
  if (env) return { version: env }
  const pinned = (read("sokonanoda-version.txt") ?? "").split(/\r?\n/).map((l) => l.trim()).find((l) => l && !l.startsWith("#"))
  const fromTxt = full(pinned)
  if (fromTxt) return { version: fromTxt }
  const requires = /^\s*requires\s*=\s*["']([^"']+)["']/m.exec(read("sokonanoda.toml") ?? "")?.[1]?.trim()
  const fromManifest = full(requires)
  if (fromManifest) return { version: fromManifest }
  const cargo = /^version = "([^"]+)"/m.exec(read("Cargo.toml") ?? "")?.[1]
  const fromCargo = full(cargo)
  if (fromCargo) return { version: fromCargo }
  const majorMinor = /^v?(\d+)\.(\d+)$/.exec(String(requires ?? "").trim())
  return { constraint: majorMinor ? `${majorMinor[1]}.${majorMinor[2]}` : undefined }
}

function readVersion(repo: string): string | undefined {
  return readVersionSource(repo).version
}

/// A cached marker matches only against the pinned version: a constraint
/// (`0.58`) accepts any patch of that release line, an anchor (`0.58.0`)
/// requires exactly the marker the download wrote.
function versionSatisfied(version: string | undefined, marker: string): boolean {
  if (!version) return false
  const found = /^(\d+)\.(\d+)(?:\.(\d+))?/.exec(marker.trim())
  if (!found) return false
  const pinned = /^(\d+)\.(\d+)(?:\.(\d+))?$/.exec(version)
  if (!pinned) return false
  if (found[1] !== pinned[1] || found[2] !== pinned[2]) return false
  return pinned[3] === undefined || found[3] === pinned[3]
}
/// VS Code / vsce target for this host.
function platformTarget(): string | undefined {
  const { platform, arch } = process
  if (platform === "darwin") return arch === "arm64" ? "darwin-arm64" : "darwin-x64"
  if (platform === "linux") {
    const alpine = existsSync("/etc/alpine-release")
    if (arch === "arm64") return alpine ? "alpine-arm64" : "linux-arm64"
    return alpine ? "alpine-x64" : "linux-x64"
  }
  if (platform === "win32") return arch === "arm64" ? "win32-arm64" : "win32-x64"
  return undefined
}

/// Rust target triple used in release asset names.
function rustTriple(): string | undefined {
  switch (platformTarget()) {
    case "darwin-arm64": return "aarch64-apple-darwin"
    case "darwin-x64": return "x86_64-apple-darwin"
    case "linux-x64": return "x86_64-unknown-linux-gnu"
    case "linux-arm64": return "aarch64-unknown-linux-gnu"
    case "alpine-x64": return "x86_64-unknown-linux-musl"
    case "alpine-arm64": return "aarch64-unknown-linux-musl"
    case "win32-x64": return "x86_64-pc-windows-msvc"
    case "win32-arm64": return "aarch64-pc-windows-msvc"
    default: return undefined
  }
}

function markerPath(dir: string, base: string): string {
  return path.join(dir, `${base}.version`)
}

/// True when the cached binary's version marker matches `<version> <target>`
/// (the same marker the CLI's `setup`/`update` writes). A missing/stale marker
/// means the cached binary must be refreshed from the version-pinned release.
function markerMatches(dir: string, base: string, version: string | undefined): boolean {
  const target = platformTarget()
  if (!target) return false
  try {
    const marker = readFileSync(markerPath(dir, base), "utf8").trim()
    const [found, markerTarget] = marker.split(/\s+/)
    return markerTarget === target && versionSatisfied(version, found ?? "")
  } catch {
    return false
  }
}

/// Contributor builds on disk: `<CARGO_TARGET_DIR>|<repo>/target` × `release|debug`.
///
/// ⚠ **照 `scripts/soko` 的 `repoBuildCandidates` 对齐**（2026-10-15 CHAIN-X）：旧实现四处与启动器不同，每一处都**零信号**：
/// **跨 profile 只按 mtime**（刚编的 debug 会遮蔽稍旧的 release：同一份源码、**同一个
/// 版本号**，`--version` 分辨不出来，而判卷实测从约 1–2 分钟变成 >20 分钟）·
/// **不认 `CARGO_TARGET_DIR`**（worktree 里 `<repo>/target` 根本不存在 ⇒ 刚编好的看不见）·
/// **不查版本**（上一轮留下的 `target/` 被静默使用）· **没有 stale 概念**。
/// 口径与启动器一致：**release 先**、同 profile 内按 mtime 新者优先。
function repoBuildCandidates(
  repo: string,
  base: string,
): { path: string; profile: string; mtime: number }[] {
  const name = binaryName(base)
  const roots: string[] = []
  const custom = process.env.CARGO_TARGET_DIR
  // `CARGO_TARGET_DIR` 允许相对路径（相对当前工作目录，cargo 的口径）⇒ 先 resolve。
  if (custom) roots.push(path.resolve(custom))
  const own = path.join(repo, "target")
  if (!roots.includes(own)) roots.push(own)
  const found: { path: string; profile: string; mtime: number }[] = []
  for (const root of roots) {
    for (const profile of ["release", "debug"]) {
      const candidate = path.join(root, profile, name)
      try {
        found.push({ path: candidate, profile, mtime: statSync(candidate).mtimeMs })
      } catch {
        // not built
      }
    }
  }
  const rank = (c: { profile: string }) => (c.profile === "release" ? 0 : 1)
  return found.sort((a, b) => rank(a) - rank(b) || b.mtime - a.mtime)
}

/// The version a CLI build reports about itself (`sokonanoda --version`). The
/// language server has **no** `--version`, so — exactly like `scripts/soko`'s
/// `repoBuildForVersion` — a server build is trusted only when the CLI of the
/// **same tree** proves the version (a build that cannot answer is not trusted).
function binaryVersion(binary: string | undefined): string | undefined {
  if (!binary) return undefined
  let probe
  try {
    probe = spawnSync(binary, ["--version"], { encoding: "utf8", timeout: 15_000 })
  } catch {
    return undefined
  }
  if (probe.status !== 0) return undefined
  const match = /(\d+\.\d+\.\d+)/.exec(`${probe.stdout ?? ""}${probe.stderr ?? ""}`)
  return match?.[1]
}

/// The contributor build that reports the pinned version — **release first**.
/// A `target/` left over from an earlier round must never be used silently.
function repoBuildForVersion(
  repo: string,
  base: string,
  version: string | undefined,
): string | undefined {
  if (!version) return undefined
  const candidates = repoBuildCandidates(repo, base)
  if (!candidates.length) return undefined
  if (base !== "sokonanoda") {
    // The server binary cannot speak for itself: trust a build only when the
    // CLI build of the same tree already proved the version.
    const cli = repoBuildCandidates(repo, "sokonanoda")[0]?.path
    return versionSatisfied(version, binaryVersion(cli) ?? "") ? candidates[0].path : undefined
  }
  for (const candidate of candidates) {
    if (versionSatisfied(version, binaryVersion(candidate.path) ?? "")) return candidate.path
  }
  return undefined
}

/// The newest engine source in the checkout (`crates/**` `*.rs` except `tests/`
/// and `tests.rs`, plus `Cargo.{toml,lock}`) — the same walk `scripts/soko` uses.
/// `0` = **no** engine source (a course repository) ⇒ nothing to compare against,
/// and that is **not** "very old" ✗.
function newestEngineSource(repo: string): number {
  let newest = 0
  const consider = (file: string) => {
    try {
      const mtime = statSync(file).mtimeMs
      if (mtime > newest) newest = mtime
    } catch {
      // gone
    }
  }
  for (const name of ["Cargo.toml", "Cargo.lock"]) consider(path.join(repo, name))
  const walk = (dir: string) => {
    let entries
    try {
      entries = readdirSync(dir, { withFileTypes: true })
    } catch {
      return
    }
    for (const entry of entries) {
      if (entry.name === "target" || entry.name === ".git" || entry.name === ".worktrees") continue
      const full = path.join(dir, entry.name)
      if (entry.isDirectory()) {
        if (entry.name === "tests") continue
        walk(full)
      } else if (entry.name.endsWith(".rs") && entry.name !== "tests.rs") consider(full)
    }
  }
  walk(path.join(repo, "crates"))
  return newest
}

/// ⚠ **出声**（照 `scripts/soko` 的 stale 族对齐）：源码比二进制新 ⇒ 编辑器跑的是**不属于
/// 当前 commit** 的引擎。⚠ **只喊不拦**（编辑器**必须**能启动），要消灭的是**静默** ✗。
function staleWarning(repo: string, binary: string): void {
  let mtime
  try {
    mtime = statSync(binary).mtimeMs
  } catch {
    return
  }
  const newest = newestEngineSource(repo)
  if (newest > mtime) {
    console.warn(
      `[sokonanoda] ⚠ 语言服务器不是当前 commit 的引擎：${binary}（${new Date(mtime).toISOString()}）` +
        `早于本检出最新的引擎源（${new Date(newest).toISOString()}）⇒ 编辑器里的判定可能不属于这棵树 ✗` +
        `（正解：cargo build --release -p sokonanoda-lsp --locked）`,
    )
  }
}

/// Semver-ish tuple parsed from an extension folder name
/// (`sokonanoda-lang.sokonanoda-0.13.0-darwin-arm64` -> `[0, 13, 0]`).
function entryVersion(entry: string): [number, number, number] | undefined {
  const match = /^sokonanoda-lang\.sokonanoda-(\d+)\.(\d+)\.(\d+)/.exec(entry)
  if (!match) return undefined
  return [Number(match[1]), Number(match[2]), Number(match[3])]
}

/// Folder names VS Code marked for deletion (it removes them on its next
/// start). Stale installs from other profiles and uninstalls land here; they
/// must never be resolved, even when their folder happens to be newer.
function obsoleteEntries(root: string): Set<string> {
  try {
    const marked = JSON.parse(readFileSync(path.join(root, ".obsolete"), "utf8")) as Record<string, boolean>
    return new Set(Object.keys(marked).filter((name) => marked[name]))
  } catch {
    return new Set()
  }
}

function compareVersion(a: [number, number, number], b: [number, number, number]): number {
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return a[i] - b[i]
  }
  return 0
}

/// Server bundled by an installed VS Code extension (zero network). Picks the
/// highest version instead of the newest mtime: VS Code keeps several versions
/// on disk (profiles, lazy deletion), and mtime does not track version order.
///
/// ⚠ **先挑与版本钉匹配的那份**（2026-10-15 CHAIN-X）：旧实现只按"目录名里版本最大"
/// 挑 ⇒ 仓库钉 0.90.0、盘上只有 0.83.0 的扩展时会**静默**拿它当语言服务器 ✗（与
/// `scripts/soko` 相反：那边**缓存先于扩展**，而缓存是**按版本钉校验过**的）。
/// 一份都不匹配时仍返回最高的（编辑器**必须**能启动），但要**出声** ✓。
function extensionServer(target: string, version: string | undefined): string | undefined {
  const home = process.env.HOME ?? process.env.USERPROFILE ?? ""
  const roots = [
    ".vscode/extensions",
    ".vscode-insiders/extensions",
    ".vscode-oss/extensions",
    ".cursor/extensions",
    ".windsurf/extensions",
    ".vscode-server/extensions",
  ].map((rel) => path.join(home, rel))
  const server = binaryName("sokonanoda-lsp")
  const candidates: { path: string; version?: [number, number, number]; mtime: number }[] = []
  for (const root of roots) {
    if (!existsSync(root)) continue
    let entries: string[]
    try {
      entries = readdirSync(root)
    } catch {
      continue
    }
    const obsolete = obsoleteEntries(root)
    for (const entry of entries) {
      if (!entry.startsWith("sokonanoda-lang.sokonanoda-")) continue
      if (obsolete.has(entry)) continue
      const candidate = path.join(root, entry, "bin", target, server)
      try {
        candidates.push({
          path: candidate,
          version: entryVersion(entry),
          mtime: statSync(candidate).mtimeMs,
        })
      } catch {
        // no binary for this target
      }
    }
  }
  const order = (a: typeof candidates[number], b: typeof candidates[number]) => {
    if (a.version && b.version) return compareVersion(b.version, a.version) || b.mtime - a.mtime
    if (a.version) return -1
    if (b.version) return 1
    return b.mtime - a.mtime
  }
  const matching = candidates.filter(
    (c) => c.version && versionSatisfied(version, c.version.join(".")),
  )
  const pool = matching.length ? matching : candidates
  const best = pool.sort(order)[0]
  if (best && !matching.length && version) {
    console.warn(
      `[sokonanoda] ⚠ VS Code 扩展自带的语言服务器与仓库版本钉不一致：${best.path}` +
        `（v${(best.version ?? []).join(".") || "?"} ≠ v${version}）⇒ 缓存里也没有版本匹配的条目，` +
        `只能先用它（编辑器里的判定可能不属于这棵树 ✗）`,
    )
  }
  return best?.path
}

/// Version-pinned download + extract (fetch + `tar`; no shell).
async function downloadBinary(
  version: string,
  pkg: string,
  base: string,
): Promise<string | undefined> {
  const triple = rustTriple()
  if (!triple || process.env.SOKONANODA_OFFLINE) return undefined
  const dir = cacheDir()
  const dest = path.join(dir, binaryName(base))
  // Re-use the cached binary only when its marker matches the repo version;
  // otherwise re-fetch from the version-pinned release (never `latest`).
  if (existsSync(dest) && markerMatches(dir, base, version)) return dest
  let tmp: string | undefined
  try {
    const res = await fetch(`${RELEASES}/v${version}/${pkg}-${triple}.tar.gz`)
    if (!res.ok) return undefined
    tmp = mkdtempSync(path.join(tmpdir(), "sokonanoda-"))
    const tarball = path.join(tmp, "asset.tar.gz")
    writeFileSync(tarball, Buffer.from(await res.arrayBuffer()))
    const untar = spawnSync("tar", ["xzf", tarball, "-C", tmp], { stdio: "ignore" })
    if (untar.status !== 0) return undefined
    mkdirSync(dir, { recursive: true })
    const extracted = path.join(tmp, binaryName(base))
    if (!existsSync(extracted)) return undefined
    writeFileSync(dest, readFileSync(extracted))
    if (process.platform !== "win32") {
      try {
        chmodSync(dest, 0o755)
      } catch {
        // read-only cache dir
      }
    }
    writeFileSync(markerPath(dir, base), `${version} ${platformTarget()}\n`)
    return existsSync(dest) ? dest : undefined
  } catch {
    return undefined
  } finally {
    if (tmp) rmSync(tmp, { recursive: true, force: true })
  }
}

async function resolveServer(repo: string | undefined): Promise<string | undefined> {
  const explicit = process.env.SOKONANODA_LSP_BIN
  if (explicit && existsSync(explicit)) return explicit
  const version = repo ? readVersion(repo) : undefined
  if (repo) {
    // ⚠ 版本匹配 + release 优先 + 认 `CARGO_TARGET_DIR`（照 `scripts/soko` 对齐）✓；
    // 选到之后**必须**看它陈不陈旧（`staleWarning`，只喊不拦 ✓）。
    const local = repoBuildForVersion(repo, "sokonanoda-lsp", version)
    if (local) {
      staleWarning(repo, local)
      return local
    }
  }
  const dir = cacheDir()
  const cached = path.join(dir, binaryName("sokonanoda-lsp"))
  // ⚠ **缓存排在扩展之前**（2026-10-15 CHAIN-X）：缓存条目是**按版本钉校验过**的
  // （`markerMatches`），而扩展那份只按"目录名里版本最大"挑 ⇒ 旧顺序会拿一份**与仓库
  // 版本不匹配**的服务器压住匹配的缓存 ✗。与 `scripts/soko` 的顺序一致 ✓。
  // G-16：没有版本钉时缓存**不可用**（没有东西可比）——上面那条 `markerMatches` 会拒。
  if (existsSync(cached) && markerMatches(dir, "sokonanoda-lsp", version)) return cached
  const target = platformTarget()
  if (target) {
    const bundled = extensionServer(target, version)
    if (bundled) return bundled
  }
  if (!version) return undefined
  const downloaded = await downloadBinary(version, "sokonanoda-lsp", "sokonanoda-lsp")
  // Offline / download failure: the matching cache entry (if any) was already
  // returned above, so there is nothing safe left to fall back to.
  return downloaded
}

export const Sokonanoda: Plugin = async ({ directory }) => {
  const repo = findRepoRoot(directory)
  const server = await resolveServer(repo).catch(() => undefined)
  const version = repo ? readVersion(repo) : undefined
  // ⚠ **PATH 也走同一条解析链**（2026-10-15 CHAIN-X）：旧实现无条件把**缓存目录**塞到
  // PATH 最前 ⇒ 就算这棵树刚编好一份**版本匹配**的 CLI，shell 里的 `sokonanoda` 仍是
  // 缓存里那份（**同版本号、可能更旧的引擎**，零信号 ✗）。现在：有版本匹配的仓库构建
  // 就暴露它的目录，否则才回落到缓存 —— 与 `scripts/soko` 的顺序一致 ✓。
  const cli = repo ? repoBuildForVersion(repo, "sokonanoda", version) : undefined
  const cliDir = cli ? path.dirname(cli) : cacheDir()
  // Best effort: fetch the matching CLI too, so `sokonanoda` works in shells.
  if (repo && !process.env.SOKONANODA_OFFLINE) {
    if (version) await downloadBinary(version, "sokonanoda-cli", "sokonanoda").catch(() => undefined)
  }
  return {
    config: async (cfg) => {
      if (!server) return
      const existing = cfg.lsp?.sokonanoda as { command?: string[] } | undefined
      // Respect a user's own lsp.sokonanoda configuration; only take over the
      // committed shim (or an empty slot).
      const ours =
        !existing ||
        !existing.command ||
        existing.command.some((part) => part.includes("sokonanoda-lsp.sh"))
      if (!ours) return
      cfg.lsp ??= {}
      cfg.lsp.sokonanoda = {
        command: [server],
        extensions: [".sokonanoda"],
      }
    },
    "shell.env": async (_input, output) => {
      output.env ??= {}
      const current = output.env.PATH ?? process.env.PATH ?? ""
      if (!current.split(path.delimiter).includes(cliDir)) {
        output.env.PATH = `${cliDir}${path.delimiter}${current}`
      }
    },
  }
}
