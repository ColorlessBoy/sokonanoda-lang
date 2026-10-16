#!/usr/bin/env bash
# **冻结证明的输入**：把一条咬住证明钉到**钉死 rev 的队列夹具**上跑。
#
# 用法：`scripts/queue-frozen-proof.sh <rev> -- <cmd> [args…]`
#
# ## 为什么要有（洞 `BITE-PROOF-READS-LIVE-SHARED-LEDGER`，medium，2026-10-16 实测 ✗）
#
# `docs/guard-bite.jsonl` 里 `scripts/ux-queue.py` 那两行的 `pass` 原先直接读**活的**共享队列
# 台账（`SOKO_QUEUE_ROOT=<主 checkout>`）⇒ 它的 rc 是**别人正在写的状态**的函数：
#   · `--selftest` 的「④ 真来源」那一臂（`scripts/ux-queue.py:9594-9603`）判**活数据**
#     （`undecided == []` / 每行处置与严重度合法）—— 任何一个 agent 把一条发现**先登记进
#     索引**（`docs/ux-findings-2026-10-12.md`）、处置稍后补进 extras，`undecided` 就非空
#     ⇒ `--selftest` **rc=1**；
#   · `--check` 同理：它判的就是活台账（活台账此刻红 ⇒ rc=1）。
# ⇒ `check-guard-bite.py` 把这种红印成「**守卫自己坏了**」✗ —— 而真相是**输入动了**，
#   不是守卫坏了（实测形状：`✗ 正对照：真台账 --all 逐条真跑 ⇒ rc=0：rc=1`）。
#
# 口径（与 `check-attention-budget.py --source-rev` / `check-threshold-ratchet.py
# --source-rev` / `check-history-depth-wiring.py --ci-yml-rev` **同一族** ✓）：
# **证明的输入必须钉死在一个 rev 上** ⇒ 同一份台账在任何时刻、任何机器上跑出同一个读数 ✓。
#
# ## 夹具的形状（**两条都是刻意的** ✓）
#
# * **数据 = 钉死的 rev**：`docs/ux-queue-extras.jsonl` / `docs/gaps/ledger.jsonl` /
#   `docs/ux-findings-2026-10-12.md` / `docs/ONBOARDING.md` / `docs/ux-queue-events.jsonl` /
#   `Cargo.toml` 全部用 `git show <rev>:<path>` 取 ⇒ 之后**谁再写活台账都与本证明无关** ✓。
# * **代码 = 工作区那一份** `scripts/ux-queue.py` ⇒ 证明咬的仍是**当前**守卫，不是旧版本 ✗
#   （这一条与 `--source-rev` 那族**相反**，理由：那些是"把坏版本换进来"，这里是
#   "把**输入**钉死、守卫照旧" ✓）。
# * **git 历史 = 真历史**（`git clone --shared --no-checkout` ⇒ objects 走 alternates、秒级 ✓）：
#   `--check` 的祖先性断言要能解析事件表里那些真 commit（拿 `git init` 的空仓当夹具 ⇒
#   实测 **347 条红**，全是"查不到对象" ✗）。⚠ **一个字节都不写来源仓** ✓（`--shared` 只读
#   它的 objects；夹具自己新造的对象落在夹具的 object 目录里 ✓）。
#
# ## 空转闸（四条，任一不成立 ⇒ **exit 3**，绝不冒充"冻结过了" ✗）
#
#   ① `<rev>` 在本仓里**解析不了**；② `<rev>` **不是 `HEAD` 的祖先**（与 `check-guard-bite.py`
#   的 **C5 可达性**同一口径 —— CI 只检出**一个 ref** 的历史，钉别的分支上的 rev 会量不到）；
#   ③ 那个 rev 里**没有**队列根的必需文件（那不是一份队列根）；④ 夹具的 `HEAD` 与来源仓
#   不是同一个 rev、或夹具就是来源仓本身（⇒ "冻结"是假的）。
#
# ⚠ **本脚本只改一件事：证明读哪份数据** ✗ —— 它不碰任何判据、不碰 `ux-queue.py`、
#   不写活台账（跑完 `mktemp` 目录整个删掉 ✓）。
#
# 退出码：`<cmd>` 的退出码原样传出；**3 = 前提错**（造不出夹具 / rev 不合格 ⇒ **不假装跑过** ✗）；
# 2 = 用法错。
set -u

if [ "$#" -lt 3 ] || [ "$2" != "--" ]; then
    echo "用法：scripts/queue-frozen-proof.sh <rev> -- <cmd> [args…]" >&2
    echo "  ⇒ 把 <cmd> 钉到 <rev> 那份**冻结的队列夹具**上跑（见本文件抬头）" >&2
    exit 2
fi
rev="$1"; shift 2

repo="$(git rev-parse --show-toplevel 2>/dev/null)" || {
    echo "✗ 冻结不了：当前目录不在 git 仓库里（`git rev-parse --show-toplevel` rc≠0）⇒ 不假装跑过 ✗" >&2
    exit 3
}

# 空转闸 ①/②：rev 必须存在、且是 HEAD 的祖先（C5 同口径）。
git -C "$repo" cat-file -e "${rev}^{commit}" 2>/dev/null || {
    echo "✗ 冻结不了：rev \`$rev\` 在**本仓里不存在**（`git cat-file -e ${rev}^{commit}` rc≠0）" >&2
    echo "  ⇒ CI 只检出**一个 ref** 的历史 ⇒ 钉别的分支上的 rev 会量不到 ⇒ 换 \`HEAD\` 的祖先 ✓" >&2
    exit 3
}
git -C "$repo" merge-base --is-ancestor "$rev" HEAD 2>/dev/null || {
    echo "✗ 冻结不了：rev \`$rev\` **不是 HEAD 的祖先**（`git merge-base --is-ancestor` rc≠0）" >&2
    echo "  ⇒ 同 ①：换 \`HEAD\` 的祖先（`git merge-base --is-ancestor <rev> HEAD` 核 ✓）" >&2
    exit 3
}

# 队列根的必需文件（口径 = `ux-queue.py` 的 `REQUIRED_ROOT_FILES` + 两份**数据**台账）。
FILES="Cargo.toml docs/gaps/ledger.jsonl docs/ux-findings-2026-10-12.md docs/ONBOARDING.md docs/ux-queue-extras.jsonl docs/ux-queue-events.jsonl"
missing=""
for f in $FILES; do
    git -C "$repo" cat-file -e "$rev:$f" 2>/dev/null || missing="$missing $f"
done
if [ -n "$missing" ]; then
    # 空转闸 ③：那个 rev 里没有队列根 ⇒ 冻结出来的是个**假夹具**（跑什么都会红，读数不可归属 ✗）。
    echo "✗ 冻结不了：rev \`$rev\` 里没有这些文件：$missing" >&2
    echo "  ⇒ 那不是一份**队列根**（队列台账 2026-10-12 才落地 ⇒ 更早的 rev 天然缺它们）" >&2
    echo "  ⇒ 钉一个**队列落地之后**的 rev ✓（别拿更早的当夹具：跑出来的是"根不齐"、不是判据 ✗）" >&2
    exit 3
fi

tmp="$(mktemp -d)" || {
    echo "✗ 冻结不了：\`mktemp -d\` rc≠0（临时目录都建不出来）⇒ 不假装跑过 ✗" >&2
    exit 3
}
trap 'rm -rf "$tmp"' EXIT
root="$tmp/root"

# 真历史的独立仓（objects 走 alternates ⇒ 秒级；**不写来源仓** ✓）。
git clone -q --shared --no-checkout "$repo" "$root" 2>/dev/null || {
    echo "✗ 冻结不了：\`git clone --shared --no-checkout\` rc≠0（造不出夹具仓）" >&2
    exit 3
}
# HEAD 指到钉死的 rev（**不做 checkout** ⇒ 不铺 26 万个文件；工作区只放我们要的那几份 ✓）。
git -C "$root" update-ref refs/heads/frozen-proof "$rev" 2>/dev/null || {
    echo "✗ 冻结不了：夹具仓里 \`update-ref refs/heads/frozen-proof $rev\` rc≠0" >&2
    exit 3
}
git -C "$root" symbolic-ref HEAD refs/heads/frozen-proof 2>/dev/null || {
    echo "✗ 冻结不了：夹具仓里 \`symbolic-ref HEAD\` rc≠0" >&2
    exit 3
}

# **数据 = 钉死的 rev**（一份一份从 rev 取 ⇒ 活的那些之后怎么变都与本跑无关 ✓）。
# ⚠ `git clone --no-checkout` **不铺目录** ⇒ 每一份先 `mkdir -p` 它的父级（否则重定向就失败 ✗）。
for f in $FILES; do
    mkdir -p "$root/$(dirname "$f")" || exit 3
    git -C "$repo" show "$rev:$f" > "$root/$f" || {
        echo "✗ 冻结不了：\`git show $rev:$f\` rc≠0" >&2
        exit 3
    }
done
# **代码 = 工作区那一份**（被测的守卫是**当前**的，不是那个 rev 的旧版 ✗）。
mkdir -p "$root/scripts" || exit 3
cp "$repo/scripts/ux-queue.py" "$root/scripts/ux-queue.py" || {
    echo "✗ 冻结不了：拷不进 \`scripts/ux-queue.py\`" >&2
    exit 3
}

# 空转闸 ④：夹具必须**真是**夹具（HEAD 就是那个 rev；根不是来源仓）。
got="$(git -C "$root" rev-parse HEAD 2>/dev/null)"
want="$(git -C "$repo" rev-parse "${rev}^{commit}" 2>/dev/null)"
if [ "$got" != "$want" ] || [ "$root" = "$repo" ]; then
    echo "✗ 冻结不了：夹具的 HEAD=$got 与 rev=$want 不一致（或夹具就是来源仓）⇒ \"冻结\"是假的 ✗" >&2
    exit 3
fi

cd "$root" || exit 3
# ⚠ 连 `SOKO_QUEUE_EVENTS` 一起摘掉：证明只许读/写**夹具自己**那份事件表 ✓
#   （否则调用方环境里一个重定向就把读数带出夹具 ⇒ 不可归属 ✗）。
env -u SOKO_QUEUE_EVENTS SOKO_QUEUE_ROOT="$root" "$@"
