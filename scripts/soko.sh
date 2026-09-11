#!/usr/bin/env bash
# The single environment entrypoint for sokonanoda-lang.
# Design + research: docs/design/onboarding.md.
#
#   scripts/soko.sh setup            version-pinned CLI + LSP into the cache
#   scripts/soko.sh update           force-refresh the cache to the repo version
#   scripts/soko.sh version [--json] print the repo/target/cached versions
#   scripts/soko.sh doctor [--json]  read-only readiness report
#   scripts/soko.sh grade <file...>  kernel-grade a canvas (CLI --json)
#   scripts/soko.sh gate             contributor CI gate (requires cargo)
#   scripts/soko.sh lsp              resolve + exec the language server (internal:
#                                    used by .opencode/lsp/sokonanoda-lsp.sh)
#
# Paths/files can be overridden for tests and installs:
#   SOKONANODA_CACHE_DIR  (default ~/.local/share/sokonanoda/bin)
#   SOKONANODA_OFFLINE=1  forbid the network step
#   SOKONANODA_LSP_BIN    explicit server binary
# Exit codes: 0 ok · 3 environment not ready · 2 misuse · 1 internal/task failure.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" 2>/dev/null | head -1)"
cache="${SOKONANODA_CACHE_DIR:-$HOME/.local/share/sokonanoda/bin}"
offline="${SOKONANODA_OFFLINE:-${SOKONANODA_LSP_OFFLINE:-}}"

die() { # exit-code message...
  local code="$1"
  shift
  echo "soko: $*" >&2
  exit "$code"
}

usage() {
  sed -n '2,18p' "$0" | sed 's/^# \{0,1\}//'
}

# VS Code / vsce target (extension bundle layout, doctor field).
platform_target() {
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) echo "darwin-arm64" ;;
    Darwin-x86_64) echo "darwin-x64" ;;
    Linux-x86_64)
      if [ -f /etc/alpine-release ]; then echo "alpine-x64"; else echo "linux-x64"; fi
      ;;
    Linux-aarch64)
      if [ -f /etc/alpine-release ]; then echo "alpine-arm64"; else echo "linux-arm64"; fi
      ;;
    *) echo "win32-x64" ;;
  esac
}

# Rust target triple (GitHub Release asset naming).
rust_target() {
  case "$(platform_target)" in
    darwin-arm64) echo "aarch64-apple-darwin" ;;
    darwin-x64) echo "x86_64-apple-darwin" ;;
    linux-x64) echo "x86_64-unknown-linux-gnu" ;;
    linux-arm64) echo "aarch64-unknown-linux-gnu" ;;
    alpine-x64) echo "x86_64-unknown-linux-musl" ;;
    alpine-arm64) echo "aarch64-unknown-linux-musl" ;;
    win32-x64) echo "x86_64-pc-windows-msvc" ;;
    win32-arm64) echo "aarch64-pc-windows-msvc" ;;
  esac
}

binary_path() { echo "$cache/$1"; }
marker_path() { echo "$cache/$1.version"; }

binary_ready() { # name — present + executable + version marker matches
  local path
  path="$(binary_path "$1")"
  [ -x "$path" ] || return 1
  [ -f "$(marker_path "$1")" ] || return 1
  [ "$(cat "$(marker_path "$1")")" = "$version $target" ] || return 1
}

download_one() { # pkg-name binary-name rust-triple
  local pkg="$1" name="$2" triple="$3"
  local url="https://github.com/ColorlessBoy/sokonanoda-lang/releases/download/v${version}/${pkg}-${triple}.tar.gz"
  mkdir -p "$cache"
  if ! curl --proto '=https' --tlsv1.2 --retry 2 --max-time 300 -fsSL "$url" | tar xz -C "$cache"; then
    echo "soko: 下载失败：$url" >&2
    return 1
  fi
  if [ ! -f "$(binary_path "$name")" ]; then
    echo "soko: 解压后未找到二进制文件：$name" >&2
    return 1
  fi
  # Release tarballs may carry 0644 (GitHub artifact round-trips strip the
  # exec bit); restore it before the readiness check.
  chmod +x "$(binary_path "$name")" 2>/dev/null || true
  if [ ! -x "$(binary_path "$name")" ]; then
    echo "soko: 二进制不可执行（只读文件系统？）：$name" >&2
    return 1
  fi
  printf '%s\n' "$version $target" >"$(marker_path "$name")"
}

cmd_setup() {
  [ -n "$version" ] || die 1 "读不到 $root/Cargo.toml 的版本"
  target="$(platform_target)"
  triple="$(rust_target)"
  local force=""
  [ "${1:-}" = "--force" ] && force=1
  mkdir -p "$cache"
  for entry in "sokonanoda-cli:sokonanoda" "sokonanoda-lsp:sokonanoda-lsp"; do
    local pkg="${entry%%:*}" name="${entry##*:}"
    if [ -z "$force" ] && binary_ready "$name"; then
      continue
    fi
    if [ -n "$offline" ]; then
      die 3 "离线模式且缓存缺失 ${name}（需要 v${version} ${triple}）；先联网跑 scripts/soko.sh setup"
    fi
    download_one "$pkg" "$name" "$triple" || die 3 "无法获取 ${name}（v${version} ${triple}）"
  done
  "$(binary_path sokonanoda)" --version >/dev/null 2>&1 || die 3 "下载的 CLI 无法执行"
  echo "soko: ready — $(binary_path sokonanoda) (v$version $target, $triple)"
}

bool() { if "$@" >/dev/null 2>&1; then echo true; else echo false; fi; }

cmd_doctor() {
  local as_json=""
  [ "${1:-}" = "--json" ] && as_json=1
  target="$(platform_target)"
  triple="$(rust_target)"
  local cli_present lsp_present cli_ok lsp_ok launcher launcher_exec cargo_present plugin ready
  cli_present="$(bool test -x "$(binary_path sokonanoda)")"
  lsp_present="$(bool test -x "$(binary_path sokonanoda-lsp)")"
  cli_ok="$(bool binary_ready sokonanoda)"
  lsp_ok="$(bool binary_ready sokonanoda-lsp)"
  launcher="$root/.opencode/lsp/sokonanoda-lsp.sh"
  launcher_exec="$(bool test -x "$launcher")"
  plugin="$root/.opencode/plugin/sokonanoda.ts"
  cargo_present="$(bool command -v cargo)"
  if [ "$cli_ok" = true ] && [ "$lsp_ok" = true ]; then ready=true; else ready=false; fi

  if [ -n "$as_json" ]; then
    printf '{"ready":%s,"version":"%s","target":"%s","rust_target":"%s","cache":"%s","offline":%s,' \
      "$ready" "$version" "$target" "$triple" "$cache" "$(bool test -n "$offline")"
    printf '"cli":{"path":"%s","present":%s,"version_match":%s},' "$(binary_path sokonanoda)" "$cli_present" "$cli_ok"
    printf '"lsp":{"path":"%s","present":%s,"version_match":%s},' "$(binary_path sokonanoda-lsp)" "$lsp_present" "$lsp_ok"
    printf '"launcher":{"path":"%s","executable":%s},"plugin":{"path":"%s","present":%s},"cargo":%s}\n' \
      "$launcher" "$launcher_exec" "$plugin" "$(bool test -f "$plugin")" "$cargo_present"
  else
    echo "soko doctor"
    echo "  version:  v${version:-?}"
    echo "  platform: ${target:-?} (rust: ${triple:-?})"
    echo "  cache:    $cache${offline:+ (offline)}"
    echo "  cli:      $(binary_path sokonanoda) present=$cli_present version_match=$cli_ok"
    echo "  lsp:      $(binary_path sokonanoda-lsp) present=$lsp_present version_match=$lsp_ok"
    echo "  launcher: $launcher executable=$launcher_exec"
    echo "  plugin:   $plugin present=$(bool test -f "$plugin")"
    echo "  cargo:    $cargo_present (contributors only)"
    if [ "$ready" = true ]; then
      echo "  status:   READY"
    else
      echo "  status:   NOT READY — run: scripts/soko.sh setup"
    fi
  fi
  [ "$ready" = true ] || exit 3
}

# Version marker text ("<version> <target>") for a cached binary, or empty.
cached_marker() {
  local marker
  marker="$(marker_path "$1")"
  if [ -f "$marker" ]; then cat "$marker"; fi
}

cmd_version() {
  local as_json=""
  [ "${1:-}" = "--json" ] && as_json=1
  target="$(platform_target)"
  triple="$(rust_target)"
  local cli_marker lsp_marker cli_present lsp_present cli_match lsp_match
  cli_marker="$(cached_marker sokonanoda)"
  lsp_marker="$(cached_marker sokonanoda-lsp)"
  cli_present="$(bool test -x "$(binary_path sokonanoda)")"
  lsp_present="$(bool test -x "$(binary_path sokonanoda-lsp)")"
  cli_match="$(bool test "$cli_marker" = "$version $target")"
  lsp_match="$(bool test "$lsp_marker" = "$version $target")"
  if [ -n "$as_json" ]; then
    printf '{"version":"%s","target":"%s","rust_target":"%s","cache":"%s",' \
      "$version" "$target" "$triple" "$cache"
    printf '"cli":{"present":%s,"marker":"%s","match":%s},' \
      "$cli_present" "$cli_marker" "$cli_match"
    printf '"lsp":{"present":%s,"marker":"%s","match":%s}}\n' \
      "$lsp_present" "$lsp_marker" "$lsp_match"
  else
    echo "soko version"
    echo "  repo:  v${version:-?} (${target:-?}, ${triple:-?})"
    echo "  cache: $cache"
    echo "  cli:   ${cli_marker:-<missing>} ($([ "$cli_match" = true ] && echo match || echo mismatch))"
    echo "  lsp:   ${lsp_marker:-<missing>} ($([ "$lsp_match" = true ] && echo match || echo mismatch))"
  fi
}

# Force-refresh the cache to the repo's pinned version (setup --force).
cmd_update() {
  echo "soko: updating cache to v${version:-?}…" >&2
  cmd_setup --force
}

cmd_grade() {
  [ "$#" -gt 0 ] || die 2 "用法: scripts/soko.sh grade <file...>"
  target="$(platform_target)"
  if ! binary_ready sokonanoda; then
    cmd_setup >&2
  fi
  exec "$(binary_path sokonanoda)" --json "$@"
}

cmd_gate() {
  command -v cargo >/dev/null 2>&1 || die 3 "贡献者门禁需要 Rust/cargo；用户/agent 只需 scripts/soko.sh setup"
  run() {
    echo "+ $*" >&2
    "$@" || die 1 "gate failed: $*"
  }
  run cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
  run cargo clippy --workspace --all-targets
  run cargo test --workspace --locked
  run cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json "$root/playground.sokonanoda"
  echo "soko: gate PASS" >&2
}

# Editor path: resolve the language server (stdout stays pure LSP; status to stderr).
cmd_lsp() {
  if [ -n "${SOKONANODA_LSP_BIN:-}" ] && [ -x "$SOKONANODA_LSP_BIN" ]; then
    exec "$SOKONANODA_LSP_BIN"
  fi
  target="$(platform_target)"

  best=""
  for candidate in "$root/target/release/sokonanoda-lsp" "$root/target/debug/sokonanoda-lsp"; do
    [ -x "$candidate" ] || continue
    if [ -z "$best" ] || [ "$candidate" -nt "$best" ]; then best="$candidate"; fi
  done
  if [ -n "$best" ]; then exec "$best"; fi

  for ext_root in \
    "$HOME/.vscode/extensions" \
    "$HOME/.vscode-insiders/extensions" \
    "$HOME/.vscode-oss/extensions" \
    "$HOME/.cursor/extensions" \
    "$HOME/.windsurf/extensions" \
    "$HOME/.vscode-server/extensions"
  do
    [ -d "$ext_root" ] || continue
    for candidate in "$ext_root"/sokonanoda-lang.sokonanoda-*/bin/"$target"/sokonanoda-lsp; do
      [ -x "$candidate" ] || continue
      if [ -z "$best" ] || [ "$candidate" -nt "$best" ]; then best="$candidate"; fi
    done
  done
  if [ -n "$best" ]; then exec "$best"; fi

  if [ -x "$(binary_path sokonanoda-lsp)" ]; then exec "$(binary_path sokonanoda-lsp)"; fi

  if [ -n "$version" ] && [ -z "$offline" ] && command -v curl >/dev/null 2>&1; then
    if download_one sokonanoda-lsp sokonanoda-lsp "$(rust_target)" 2>/dev/null; then
      exec "$(binary_path sokonanoda-lsp)"
    fi
    echo "soko: 下载语言服务器失败，尝试本地构建。" >&2
  fi

  if command -v cargo >/dev/null 2>&1; then
    echo "soko: cargo build -p sokonanoda-lsp（首次较慢）…" >&2
    cargo build --quiet --manifest-path "$root/Cargo.toml" -p sokonanoda-lsp >&2
    exec "$root/target/debug/sokonanoda-lsp"
  fi

  die 3 "找不到语言服务器二进制，也无法下载（离线或网络失败）"
}

case "${1:-}" in
  setup)
    shift
    cmd_setup "$@"
    ;;
  update)
    shift
    cmd_update "$@"
    ;;
  version)
    shift
    cmd_version "$@"
    ;;
  doctor)
    shift
    cmd_doctor "$@"
    ;;
  grade)
    shift
    cmd_grade "$@"
    ;;
  gate)
    cmd_gate
    ;;
  lsp)
    cmd_lsp
    ;;
  "" | -h | --help)
    usage
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
