#!/usr/bin/env bash
# 发版节点的**本地预检**（2026-10-08 立 · 把这一轮踩过的坑固化成一条命令 ✓）。
#
# ## 为什么需要它（本轮实测踩过的四个坑，每个都让门禁红过一次 ✗）
#
# 1. **启动器拒绝过期缓存**：`scripts/soko gate` 会用缓存里的 CLI，而它常是旧版
#    （实测 `cache(STALE: expected 0.86.0, found 0.83.0)` ⇒ **exit 3** ✗）⇒
#    必须显式给 `SOKONANODA_BIN`（用**仓库构建** ✓）。
# 2. **锚点版本必须与仓库一致**：仓库 bump 之后旧二进制当锚点 ⇒ 门禁**exit 3**
#    （"gate 用的是 v0.85.2 二进制，而仓库是 v0.86.0" ✗）⇒ 先重建。
# 3. **插件自带 CLI 版本 = 插件版本**（`cargo test -p sokonanoda-cli --test extension` ✗）：
#    `editor/vscode/bin/**` 是构建产物（`.gitignore` ✓），`stage-lsp.js` 从
#    `target/**release**/` 取件 ⇒ **必须先 `cargo build --release`** 再 stage ✓，
#    否则自带 CLI 停在旧版本 ⇒ 两条 extension 测试红 ✗。
# 4. **`docs-lint` 的层/必读上限**：收尾编辑（STATUS/REQUIREMENTS 增长）会顶红 ⑦ ✗
#    ⇒ 先跑它（它比整套 gate 快得多 ✓）。
#
# ## 用法
#
#     scripts/release-preflight.sh              # 全套（约 10–15 分钟）
#     scripts/release-preflight.sh --fast       # 只跑 ④ docs-lint + ① 版本钉一致性
#     scripts/release-preflight.sh --sweep      # 额外跑全课程 --json 逐字节对拍（要基线）
#
# ⚠ **不要**用 `--no-verify` 绕过任何一步：仓规要求把理由写进 `STATUS.md` ✓。
set -u
cd "$(dirname "$0")/.." || exit 3
FAST=0; SWEEP=0
for arg in "$@"; do
  case "$arg" in
    --fast) FAST=1 ;;
    --sweep) SWEEP=1 ;;
    *) echo "未知参数：$arg" >&2; exit 2 ;;
  esac
done
step() { echo; echo "── $* ─────────────────────────────"; }

# ① 版本钉一致性（两处版本 + 课程 requires）
step "① 版本钉一致性"
VER=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
EXT=$(grep -m1 '"version"' editor/vscode/package.json | sed 's/.*"\(.*\)".*/\1/')
echo "Cargo.toml=$VER · package.json=$EXT"
[ "$VER" = "$EXT" ] || { echo "✗ 两处版本不一致（CI 强制相等）" >&2; exit 1; }
for f in course/shared/sokonanoda.toml courses/set-theory/sokonanoda.toml; do
  [ -f "$f" ] || continue
  req=$(grep -m1 'requires' "$f" | sed 's/.*"\(.*\)".*/\1/')
  echo "$f requires=$req"
  [ "$req" = "$VER" ] || { echo "✗ $f 的 requires 与版本不一致" >&2; exit 1; }
done

# ② docs-lint（层上限/必读合计/过期登记 —— 收尾编辑最容易顶红的一关）
step "② docs-lint"
python3 scripts/docs-lint.py || { echo "✗ docs-lint 判红（先清文档再发版）" >&2; exit 1; }

if [ "$FAST" = 1 ]; then
  echo; echo "✓ --fast 预检通过（版本钉 + docs-lint）"; exit 0
fi

# ③ release 构建 + 插件暂存（坑 3：extension 测试盯"自带 CLI = 插件版本"）
step "③ release 构建 + 暂存插件自带二进制"
cargo build --release -p sokonanoda-cli -p sokonanoda-lsp || exit 1
node editor/vscode/scripts/stage-lsp.js || exit 1
BIN="$PWD/target/release/sokonanoda"
echo "锚点二进制：$("$BIN" version --json | sed 's/.*"version":"\([^"]*\)".*/\1/')"

# ④ 完整门禁（坑 1+2：显式给锚点、且它必须是刚建的那份）
step "④ scripts/soko gate（锚点 = 仓库 release 构建）"
SOKONANODA_BIN="$BIN" scripts/soko gate || { echo "✗ gate 判红" >&2; exit 1; }

# ⑤ 可选：全课程 --json 逐字节（要一份"已发布 tag 的构建"当基线）
if [ "$SWEEP" = 1 ]; then
  step "⑤ 全课程 --json 逐字节对拍"
  BASE=${SOKO_BASELINE_BIN:-/tmp/target-v852/debug/sokonanoda}
  [ -x "$BASE" ] || { echo "✗ 基线不可执行：$BASE（先按 check-json-identity.py 文件头建）" >&2; exit 3; }
  python3 scripts/check-json-identity.py --baseline "$BASE" --new "$BIN" || exit 1
fi

echo; echo "✓ 发版预检全部通过 —— 可以 commit + **一次 push**（auto-tag → release）"
