#!/usr/bin/env bash
# sokonanoda-lsp shim for harnesses that need a command entrypoint.
# (opencode itself wires the native binary through `.opencode/plugins/sokonanoda.ts`;
# `opencode.json` deliberately carries no `lsp` block.)
#
# ⚠ **本 shim 不自己解析二进制**（2026-10-15 CHAIN-Z，洞 `OPENCODE-LSP-SHIM-OWN-CHAIN`）。
# 它以前**自带第二条解析链**：`SOKONANODA_BIN` → `<root>/target/{release,debug}` → PATH
# —— **不查版本、不认 `CARGO_TARGET_DIR`、不看缓存 marker、没有 stale 概念** ⇒ 可能选到
# **版本不匹配或陈旧**的语言服务器，而且**零信号** ✗（实测：版本钉 0.90.0、仓库构建
# 0.80.0 时它照跑 0.80.0 那份）。而 `AGENTS.md` / `skills/README.md` 一直写着它走
# `scripts/soko lsp` ⇒ **文档/实现漂移** ✗。
#
# 现在**只有一条链**：原样交给 `scripts/soko lsp`（版本**匹配**的仓库构建 → 缓存
# （marker 对版本钉）→ 扩展自带 → 按版本钉锁定下载；release 优先 · 认
# `CARGO_TARGET_DIR`/`SOKONANODA_CACHE_DIR` · 陈旧时在 **stderr** 出声 · 解析不出
# 期望版本就绝不 exec）。这与 `dsh/cordis.patch.yml` 的接线**同源** ✓ —— 别在这里
# 再发明第二套 ✗（同族的 `.opencode/plugins/sokonanoda.ts` 也已照 `scripts/soko` 对齐）。
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
launcher="$root/scripts/soko"

# 显式 override = **逃生门**，原样保留（与 `scripts/soko` 的 `SOKONANODA_LSP_BIN`、插件的
# `SOKONANODA_LSP_BIN` **同类**：人自己指的二进制，版本是使用者的事 ⇒ 不查版本、不出声；
# `lspBuildIdentity` 对 override 也**有意**回 `undefined`）。
# ⚠ **不能改成转发**：参数口径不同 —— 启动器把 override 当**服务器二进制**（不传参数），
# 而本 shim 历来把它当**能接 `lsp` 子命令的二进制** ⇒ 转发会让 `$*` 变成空 ✗
# （`crates/cli/tests/opencode.rs::launcher_resolves_the_binary_and_runs_lsp` 断言 `ARGS:lsp`）。
if [ -n "${SOKONANODA_BIN:-}" ] && [ -x "${SOKONANODA_BIN}" ]; then
  exec "$SOKONANODA_BIN" lsp
fi

# 启动器不在 = 这台机器上确实没有可解析的 sokonanoda ⇒ 干净报错（exit 3 + 人话），
# 不要丢一句 `exec: No such file or directory`（rc=127）就完事 ✗。
if [ ! -x "$launcher" ]; then
  echo "sokonanoda-lsp: 找不到 sokonanoda 二进制（先跑 \`scripts/soko setup\` 或 cargo build）" >&2
  echo "  本 shim 只做转发，而启动器不可执行：$launcher" >&2
  exit 3
fi

exec "$launcher" lsp
