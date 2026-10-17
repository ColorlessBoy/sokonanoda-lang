#!/usr/bin/env bash
# **咬住证明**：`TOP-LEVEL-TABLE-GOES-BLIND` —— `check-ci-wiring.py` **层①**（原
# `check-ci-gate-coverage.py`，2026-10-17 第 5 轮 `gs5` 并进来 ✓）的 `TOP_LEVEL` 曾是
# **硬编码的一级条目名表**（28 条）⇒ 新加一个顶层目录，它下面的路径
# token **认不出来** ⇒ 判据 R1（"按改动范围决定跑不跑"）**静默变瞎** ✗（2026-10-16 ✗→✓）。
#
# ⚠ **改名之后的形状**（`gs5` ✓）：**坏输入**仍然取自**旧路径**那份文件
# （`${REV}:scripts/check-ci-gate-coverage.py` —— 那一版里它**就是**那张硬编码表 ✓，
# **不是**新造的 ✓）；**改后读数**打在**合并后**的 `check-ci-wiring.py --layer coverage` 上 ✓。
#
# 用法：`scripts/bite-tl1-top-level-blind.sh <pinned-rev>` —— **必须**是 `HEAD` 的祖先
# （C5 可达性：CI 只检出**一个 ref** 的历史 ⇒ 别的分支上的 rev `git show` rc=128 ✗）。
# 台账钉的是 `32a9f6f9`（= 修前 main 的 tip ✓，**不写 `HEAD`** ✗ —— 写 `HEAD` 的话
# 提交之后它就成了"修好的那一版" ⇒ 证明自己失效 ✗）。
#
# ## 已存在的坏输入（**不是**本证明新造的 ✓）
#
# `git show <rev>:scripts/check-ci-gate-coverage.py` 拿到的**就是**那张硬编码表
# （实测：28 条，而仓里 29 个一级条目 —— 缺 `.gitignore` ✓）。
#
# ## 这条命令量什么（两半都要成立才算咬住 ✓）
#
# * **改前读数**：旧守卫在**同一份夹具**上必须 **rc=0** —— 那就是"静默变瞎"本身
#   （它照印「判据 6 条，违反 0 条」，**没有任何读数**告诉你它瞎了 ✗）；
# * **改后读数**：现行守卫在**同一份夹具**上必须 **rc=1** 且**点名** `newtop/thing.js`
#   —— 即"新加一级目录 ⇒ 它下面的路径**进面**" ✓。
#
# 夹具 = **真 git 仓**（照真仓的形状：地板那几个一级条目各有一个文件，**外加**一个新顶层
# 目录 `newtop/`）+ **真 `ci.yml` 的副本** + 一个**只被 `rust` 门控**却消费
# `newtop/thing.js` 的 job ⇒ 同一份夹具上"旧瞎 / 新看见"是可对照的 ✓。
#
# ## 闸（缺一条就不许算咬住 ✓）
#
# * **空转闸 ①**：`git show` rc≠0 ⇒ `exit 9`；旧版里**没有**那张硬编码表 ⇒ `exit 3`。
# * **空转闸 ②**：旧版与现行**逐字节相同**（= 修被撤回了）⇒ `exit 3`
#   （**绝不冒充咬住** ✗）。
# * **归属闸 ①**：旧守卫在夹具上若**不是** rc=0 ⇒ 夹具本身不干净 ⇒ `exit 0`（不冒充 ✓）。
# * **归属闸 ②**：新守卫的红必须**点名** `newtop/thing.js`（"有红就算"不算 ✗）⇒ 否则 `exit 0`。
#
# ## ⚠ 夹具跑 git 一律 `unset GIT_*`（洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`）
#
# `GIT_DIR` **优先于 cwd** 决定仓库发现 ⇒ 从链接工作树的 `pre-push` 里跑时，夹具的
# `git init` 会把 `core.bare=true` 写进**调用者的** `.git/config` ✗。本脚本开头
# `unset` 一次，之后每一句 `git` 都按 cwd 找仓 ✓。
#
# 退出码：1 = 咬住了（本证明**就是要非 0** ✓）· 0 = 没咬住 / 归属不成立（C2 判红 ✓）
#        3 = 空转（那条坏输入不在了）· 9 = 夹具/环境错。
set -u
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_PREFIX

REV=${1:?用法: scripts/bite-tl1-top-level-blind.sh <pinned-rev>（HEAD 的祖先）}
GUARD=scripts/check-ci-wiring.py              # 现行守卫（合并后 ✓）
OLD_GUARD=scripts/check-ci-gate-coverage.py   # 旧路径 —— **坏输入**在那一版里 ✓
[ -f "${GUARD}" ] || { echo "✗ 不在仓库根跑（缺 ${GUARD}）"; exit 9; }

T=$(mktemp -d) || exit 9
trap 'rm -rf "${T}"' EXIT
R="${T}/repo"
OLD="${T}/old"
mkdir -p "${OLD}" "${R}/.github/workflows" "${R}/newtop" || exit 9

# ① 旧守卫 = **已存在的坏输入**（钉死的旧 rev 上那份硬编码表 ✓）
git show "${REV}:${OLD_GUARD}" > "${OLD}/check-ci-gate-coverage.py" || {
    echo "✗ 取不到 ${REV}:${OLD_GUARD}（C5：钉的必须是 HEAD 的祖先）"; exit 9; }
# 旧守卫也要它的**同目录库**（`_load_push_range()` 读 `HERE/ci-push-range.py` ✓）——
# 少了它旧守卫会 rc=1（`FileNotFoundError`），那读数**不可归属** ✗。
cp scripts/ci-push-range.py "${OLD}/ci-push-range.py" || exit 9
# 空转闸 ①：钉的那个 rev 上必须**真的**是硬编码表（否则这条证明量不到东西 ✗）
grep -q 'TOP_LEVEL = (' "${OLD}/check-ci-gate-coverage.py" || {
    echo "空转闸：${REV} 上那份守卫里没有硬编码的 \`TOP_LEVEL = (\` ⇒ 坏输入不在了 ⇒ 不许算咬住"; exit 3; }
# 空转闸 ②（**改名后换了个量法** ✗→✓）：现行守卫必须**带着那个修**
# （`discover_top_level` = 动态枚举 ✓）—— 修被撤回（改回硬编码表）⇒ 空转 ✓
grep -q 'def discover_top_level' "${GUARD}" || {
    echo "空转闸：现行 ${GUARD} 里没有 \`discover_top_level\` ⇒ 动态枚举那个修被撤回了 ⇒ 不许算咬住"; exit 3; }

# ② 夹具仓（照真仓的形状 ✓）：地板那几个一级条目各有一个文件 —— **空目录不进**
#    `git ls-files` ⇒ 枚举看不见它 ✓；再加一个**新顶层目录** `newtop/` ✓
for f in Cargo.toml courses/.keep crates/.keep docs/.keep scripts/.keep; do
    mkdir -p "$(dirname "${R}/${f}")" || exit 9
    : > "${R}/${f}" || exit 9
done
printf '%s\n' '// 新顶层目录里的文件（探针 ✓）' > "${R}/newtop/thing.js" || exit 9
cp .github/workflows/ci.yml "${R}/.github/workflows/ci.yml" || exit 9
# 夹具 ci.yml = 真 ci.yml 的**副本** + 一个只被 `rust` 门控却消费 `newtop/thing.js` 的 job
python3 - "${R}/.github/workflows/ci.yml" <<'PY' || exit 9
import pathlib, sys
p = pathlib.Path(sys.argv[1])
p.write_text(p.read_text(encoding="utf-8").rstrip("\n") + "\n" + (
    "\n  tl1-probe:\n"
    "    needs: changes\n"
    "    if: needs.changes.outputs.rust == 'true'\n"
    "    runs-on: ubuntu-24.04\n"
    "    steps:\n"
    "      - run: node newtop/thing.js\n"), encoding="utf-8")
PY
( cd "${R}" && git init -q && git add -A ) || exit 9
git -C "${R}" ls-files | grep -q '^newtop/thing\.js$' || {
    echo "✗ 夹具错：新顶层目录里的文件没进夹具仓的索引"; exit 9; }

# ③ 改前读数：旧守卫在**同一份夹具**上必须 rc=0（= 静默变瞎 ✗）
python3 "${OLD}/check-ci-gate-coverage.py" --check --ci "${R}/.github/workflows/ci.yml" \
    > "${T}/old.out" 2>&1
old_rc=$?
[ "${old_rc}" = 0 ] || {
    echo "归属不成立：旧守卫在夹具上 rc=${old_rc}（期望 0 ⇒ 它该是瞎的）⇒ 不冒充咬住"
    tail -n 2 "${T}/old.out"; exit 0; }

# ④ 改后读数：现行守卫（层①）必须 rc=1 且**点名**那条路径 ✓
python3 "${GUARD}" --layer coverage --root "${R}" > "${T}/new.out" 2>&1
new_rc=$?
printf '%s\n' "旧守卫（${REV} 那份硬编码表）在夹具上 rc=${old_rc} ⇒ **静默变瞎** ✗"
grep -m1 'R1 .*newtop/thing\.js' "${T}/new.out" || true
printf '%s\n' "现行守卫 rc=${new_rc}（期望 1）· 顶层条目读数：$(grep -m1 '顶层条目' "${T}/new.out")"
[ "${new_rc}" = 1 ] || exit 0
# 归属闸 ②：红必须**点名** `newtop/thing.js`（"有红就算"不算 ✗）
grep -q 'R1 .*newtop/thing\.js' "${T}/new.out" || exit 0
exit 1
