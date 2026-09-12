// Provision the version-pinned sokonanoda binaries and wire them into opencode
// without any shell script:
//
//   - at startup (best effort) resolve the language server — env override →
//     repo build → VS Code extension bundle → cache → version-pinned download;
//   - rewrite `lsp.sokonanoda.command` to the native binary via `config`;
//   - expose the cache directory to all shells via `shell.env`.
//
// Cross-platform (macOS/Linux/Windows): uses fetch + `tar` (present on
// Windows 10+, macOS, Linux), never bash. Failures never block the session.
// Offline opt-out: SOKONANODA_OFFLINE=1. Design: docs/design/onboarding.md.
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
  const home = process.env.HOME ?? process.env.USERPROFILE ?? ""
  return path.join(home, ".local", "share", "sokonanoda", "bin")
}

function binaryName(base: string): string {
  return process.platform === "win32" ? `${base}.exe` : base
}

/// opencode may be opened in a repo subdirectory; walk up to the root.
function findRepoRoot(start: string): string | undefined {
  let dir = start
  for (;;) {
    if (
      existsSync(path.join(dir, "Cargo.toml")) &&
      existsSync(path.join(dir, ".opencode", "plugins", "sokonanoda.ts"))
    )
      return dir
    const parent = path.dirname(dir)
    if (parent === dir) return undefined
    dir = parent
  }
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

function readVersion(repo: string): string | undefined {
  try {
    return readFileSync(path.join(repo, "Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)?.[1]
  } catch {
    return undefined
  }
}

function markerPath(dir: string, base: string): string {
  return path.join(dir, `${base}.version`)
}

/// True when the cached binary's version marker matches `<version> <target>`
/// (the same marker the CLI's `setup`/`update` writes). A missing/stale marker
/// means the cached binary must be refreshed from the version-pinned release.
function markerMatches(dir: string, base: string, version: string): boolean {
  const target = platformTarget()
  if (!target) return false
  try {
    return readFileSync(markerPath(dir, base), "utf8").trim() === `${version} ${target}`
  } catch {
    return false
  }
}

/// Newest of the contributor builds (`target/release|debug`).
function repoBuild(repo: string, base: string): string | undefined {
  const name = binaryName(base)
  let best: { path: string; mtime: number } | undefined
  for (const profile of ["release", "debug"]) {
    const candidate = path.join(repo, "target", profile, name)
    try {
      const mtime = statSync(candidate).mtimeMs
      if (!best || mtime > best.mtime) best = { path: candidate, mtime }
    } catch {
      // not built
    }
  }
  return best?.path
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
function extensionServer(target: string): string | undefined {
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
  candidates.sort((a, b) => {
    if (a.version && b.version) return compareVersion(b.version, a.version) || b.mtime - a.mtime
    if (a.version) return -1
    if (b.version) return 1
    return b.mtime - a.mtime
  })
  return candidates[0]?.path
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
  if (repo) {
    const local = repoBuild(repo, "sokonanoda-lsp")
    if (local) return local
  }
  const target = platformTarget()
  if (target) {
    const bundled = extensionServer(target)
    if (bundled) return bundled
  }
  const cached = path.join(cacheDir(), binaryName("sokonanoda-lsp"))
  const version = repo ? readVersion(repo) : undefined
  // Use the cache when we cannot determine the expected version, or when its
  // marker matches. A stale cache is refreshed from the pinned release.
  if (
    existsSync(cached) &&
    (!version || markerMatches(cacheDir(), "sokonanoda-lsp", version))
  )
    return cached
  if (!version) return existsSync(cached) ? cached : undefined
  const downloaded = await downloadBinary(version, "sokonanoda-lsp", "sokonanoda-lsp")
  // Offline / download failure: fall back to the (possibly stale) cache.
  return downloaded ?? (existsSync(cached) ? cached : undefined)
}

export const Sokonanoda: Plugin = async ({ directory }) => {
  const repo = findRepoRoot(directory)
  const server = await resolveServer(repo).catch(() => undefined)
  // Best effort: fetch the matching CLI too, so `sokonanoda` works in shells.
  if (repo && !process.env.SOKONANODA_OFFLINE) {
    const version = readVersion(repo)
    if (version) await downloadBinary(version, "sokonanoda-cli", "sokonanoda").catch(() => undefined)
  }
  const cache = cacheDir()
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
      if (!current.split(path.delimiter).includes(cache)) {
        output.env.PATH = `${cache}${path.delimiter}${current}`
      }
    },
  }
}
