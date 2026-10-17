#!/usr/bin/env bash
# **咬住证明**：判据 ③「本树里未提交的路径不属于本树的主人」（洞
# `WORKTREE-ISOLATION-NOT-ENFORCED`，high，2026-10-17 · **kh1 实测** ✗）。
#
# 咬什么（**kh1 的真形状**，一字不改地搬进来 ✓）：`.worktrees/kh1` 里躺着 **20 处
# `crates/front/**`**（14 改 + 6 删 —— "拆文件 + 新模块"的形状 ✗），而 kh1 自己的活
# 全在 `crates/kernel/**`（它的收口提交 `fix(kernel): 静音窗口改成按线程计数` 只动
# `crates/kernel/src/{lib,quiet,util}.rs` + `crates/kernel/tests/memory_api.rs` + `STATUS.md` ✓）。
# ⇒ 判据必须**点名**那些不属于主人的路径 ✓。
#
# ⚠ **为什么自建一棵合成树、而不是去打真 `.worktrees/kh1`**（**刻意** ✓）：
#   · 真树里躺着**别人的活** ⇒ 本脚本**一个字节都不许动它**（别 add / stash / revert / 删 ✗）；
#   · 真树会被 `agent-worktree.sh rm` 清掉 ⇒ 拿它当夹具 = 一条**会自己失效**的证明 ✗
#     （本仓的既定口径：证明的前提要么自带、要么判红 ✓）。
#   ⇒ 这里**复现那个形状**（照 `check-attention-budget.py:_fixture_repo` 的"照真仓的形状、
#     不打桩"先例 ✓），真树那份读数留在台账行的 `bug` 字段里（评审可见 ✓）。
#
# 三跑（缺一条这条证明就不成立 ✓）：
#   ⓪ **空转闸**：干净树上先跑一次 ⇒ 必须 rc=0（判据不是恒红 ✗）
#   ① **正对照**：只改**自己范围**里的文件 ⇒ 必须 rc=0（**不恒真** ✓ —— 判据不是"有未提交就红" ✗）
#   ② **咬住**：再混进**别人的**路径 ⇒ 必须 rc=1，且输出**点名**那一条、
#      且**不许**点名主人自己那一条（**归属**要准 ✓）
#   ③ **前提闸**：判据的接口不在（`--ownership` 没了）⇒ **exit 3**（"前提不在" ≠ "咬不住" ✗）
#
# ⚠ **夹具的 `git` 一律摘掉 `GIT_*`**（洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO` ✗：
#   `GIT_DIR` **优先于 `cwd`** ⇒ 从链接工作树的钩子里跑会把夹具提交打到**真仓**上 ✗）。
#   口径 = `scripts/git_env.py` 的 `clean_git_env()`（**一处定义** ✓），这里用 shell 复刻同一件事 ✓。
set -uo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
CHECK="${ROOT}/scripts/check-multi-session.py"

# ── 摘掉 `GIT_*` 全族（与 `git_env.clean_git_env()` 同一口径 ✓）──────────────
while IFS= read -r v; do unset "${v}"; done < <(env | sed -n 's/^\(GIT_[A-Za-z_]*\)=.*/\1/p')

[ -f "${CHECK}" ] || { echo "✗ 前提不在：缺 ${CHECK}"; exit 3; }
command -v python3 >/dev/null 2>&1 || { echo "✗ 前提不在：没有 python3"; exit 3; }

T=$(mktemp -d) || exit 3
trap 'rm -rf "${T}"' EXIT
V="${T}/v"
WT="${V}/.worktrees/kh1"          # ⚠ 路径里必须有 `/worktrees/`（判据靠它认链接工作树 ✓）

git init -q -b main "${V}" || exit 3
mkdir -p "${V}/crates/kernel/src" "${V}/crates/front/src"
printf 'kernel\n' > "${V}/crates/kernel/src/util.rs"
printf 'front\n'  > "${V}/crates/front/src/ast.rs"
printf 'status\n' > "${V}/STATUS.md"
git -C "${V}" add -A || exit 3
git -C "${V}" -c user.name=fixture -c user.email=fixture@example.invalid \
    commit -qm init || exit 3
git -C "${V}" worktree add -q "${WT}" -b agent/kh1 main || exit 3

# 主人**声明**自己的范围（`agent-worktree.sh new --scope` 写的那个文件 ✓）。
# ⚠ 只有 `crates/kernel/**`：`STATUS.md` 走 `ALWAYS_ALLOWED`（收尾义务）✓。
printf '%s\n' '{"id": "kh1", "branch": "agent/kh1", "scope": ["crates/kernel/**"]}' \
    > "${WT}/.owner"

run() { ( cd "${WT}" && python3 "${CHECK}" --ownership 2>&1 ); }

# ⓪ 空转闸：干净树 ⇒ 判据必须能跑（接口在 ✓）且**不红** ✓。
out=$(run); rc=$?
if [ "${rc}" != 0 ]; then
    echo "✗ 前提不在：干净树上判据 ③ 就 rc=${rc}（接口没了 / 判据恒红）⇒ 这条证明量不到坏输入"
    printf '%s\n' "${out}"
    exit 3
fi

# ① 正对照：只改**自己范围**里的文件 ⇒ **不许**红（否则判据是"有未提交就红"的假守卫 ✗）。
printf 'kernel v2\n' > "${WT}/crates/kernel/src/util.rs"
out=$(run); rc=$?
if [ "${rc}" != 0 ]; then
    echo "✗ 判据**假红**：只改主人自己范围里的文件却 rc=${rc} ⇒ 它不是归属判据"
    printf '%s\n' "${out}"
    exit 3
fi

# ② 咬住：混进**别人的**路径（kh1 实测的那一条 ✓）⇒ 必须红 + 点名它 + 不点名自己的。
printf 'front v2\n' > "${WT}/crates/front/src/ast.rs"
out=$(run); rc=$?
printf '%s\n' "${out}"
printf '%s\n' "${out}" | grep -q 'crates/front/src/ast.rs' || {
    echo "✗ 咬住证明空转：判红输出里**没有点名**那条不属于主人的路径 ⇒ 读数不可归属"; exit 3; }
printf '%s\n' "${out}" | grep -q 'crates/kernel/src/util.rs' && {
    echo "✗ 归属判错：把**主人自己**的路径也点名了（假红 ✗）"; exit 3; }
if [ "${rc}" != 1 ]; then
    echo "✗ 咬不住：混进别人的路径，判据却 rc=${rc}（期望 1）"; exit 0
fi
exit 1
