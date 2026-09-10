// Auto-provision the version-pinned CLI/LSP at opencode startup (best effort)
// by running `scripts/soko.sh setup`, and expose the cache directory to shell
// tools via `shell.env`.
//
// Design: docs/design-onboarding.md. The provisioning step is idempotent and
// fast when the cache is current; failures never block the session (opencode
// logs plugin errors and continues). Set SOKONANODA_OFFLINE=1 to stay offline.
import { spawnSync } from "node:child_process"
import { existsSync } from "node:fs"
import path from "node:path"
import type { Plugin } from "@opencode-ai/plugin"

export const Sokonanoda: Plugin = async ({ directory }) => {
  const script = path.join(directory, "scripts", "soko.sh")
  if (existsSync(script) && !process.env.SOKONANODA_OFFLINE) {
    spawnSync("bash", [script, "setup"], { stdio: "ignore", timeout: 120000 })
  }
  return {
    "shell.env": async (_input, output) => {
      const cache = path.join(
        process.env.HOME ?? "",
        ".local",
        "share",
        "sokonanoda",
        "bin",
      )
      output.env ??= {}
      const current = output.env.PATH ?? process.env.PATH ?? ""
      if (!current.split(path.delimiter).includes(cache)) {
        output.env.PATH = `${cache}${path.delimiter}${current}`
      }
    },
  }
}
