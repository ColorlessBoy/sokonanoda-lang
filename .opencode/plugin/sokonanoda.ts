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
    if (existsSync(path.join(dir, "Cargo.toml")) && existsSync(path.join(dir, "scripts", "soko.sh")))
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

/// Newest server bundled by an installed VS Code extension (zero network).
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
  let best: { path: string; mtime: number } | undefined
  for (const root of roots) {
    if (!existsSync(root)) continue
    let entries: string[]
    try {
      entries = readdirSync(root)
    } catch {
      continue
    }
    for (const entry of entries) {
      if (!entry.startsWith("sokonanoda-lang.sokonanoda-")) continue
      const candidate = path.join(root, entry, "bin", target, server)
      try {
        const mtime = statSync(candidate).mtimeMs
        if (!best || mtime > best.mtime) best = { path: candidate, mtime }
      } catch {
        // no binary for this target
      }
    }
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
  if (existsSync(dest)) return dest
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
  if (existsSync(cached)) return cached
  if (!repo) return undefined
  const version = readVersion(repo)
  if (!version) return undefined
  return downloadBinary(version, "sokonanoda-lsp", "sokonanoda-lsp")
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
