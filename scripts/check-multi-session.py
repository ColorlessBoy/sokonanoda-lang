#!/usr/bin/env python3
# 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
"""**多会话 / 多分支守卫**（X2，2026-09-28）：两条**只有 main 出得起**的红线，写成可判红的东西。

用户要求（`docs/PLAN-0.74-0.79.md` X2）：

> 「**加锁 = 让它们排队；worktree = 让它们物理上不相干** —— 后者根本得多。」
> 「**共享账本**（`docs/gaps/ledger.jsonl`、`docs/e2e/ledger.jsonl`）多分支同改必冲突 ⇒
> 约定"**只有 main 能改台账**"，并且**写成守卫**，别只写进文档
> （写在文档里几轮就失效 —— E18 那条已经证明过一次）。」

## 两条判据

**① 台账只在 main 上改**（`git rev-parse --git-dir` 含 `/worktrees/` ⇒ 这是链接工作树）。
在**链接工作树**里，`docs/gaps/ledger.jsonl` 与 `docs/e2e/ledger.jsonl` **必须与 `main` 逐字节一致** ✗
—— 一次都不许改。理由：这两份台账是**共享**的，任何分支改动都会在合回时与 main 冲突
（而 `e2e ledger` job **会往 main 回提交** ⇒ main 一定在动 ⇒ 分支**必然**落后 ⇒ 冲突是**确定**的，不是概率）。

**② 分支落后 main 太多要 rebase**（"要把这条也变成可判的（落后 N 个提交就红）"）。
判据是**台账落后**：`main` 上的台账比本分支新 ⇒ 合回时必冲突 ⇒ **判红**。
⚠ 这里**不看**总提交数 —— `e2e ledger` 每轮往 main 回提交一次，
分支的**总落后数**会一直涨，用它判红会**每轮都红**（假红 ⇒ 守卫失效 ✗）。
**只有"台账落后"才是真的会冲突**，所以只判它 ✓。

**③ 本树里未提交的路径不属于本树的主人**（洞 `WORKTREE-ISOLATION-NOT-ENFORCED`，high，
2026-10-17 ✗→✓）。⭐ **这一条是 `kh1` 给的判据的机制化**：它的原话是"**提交前看 `git status`
里有没有不属于自己的路径**" —— 那次实测 `.worktrees/kh1` 里躺着 **20 处 `crates/front/**`**
（14 改 + 6 删，**"拆文件 + 新模块"的形状** ✗），而 kh1 自己的活全在 `crates/kernel/**`。

**病根（形状）**：`--claim` 管**队列条目**（同模块互斥有了 ✓）、"每条会话一个 worktree"管
**目录**（`scripts/agent-worktree.sh` ✓），但**没有任何东西强制它** ✗ ⇒ **"约定不是机制"** ✗。
危险动作是 `git add -A`（**别人的活会被卷进它的提交** ✗）。

**判据**：只在**链接工作树**里判（主 checkout 是编排者的场地 ✓，与 ① 同口径）。先把"**这棵树
主人自己的范围**"算出来（**证据 4 源**，全是机械可得的），再逐条判未提交路径：

  1. **声明**：`<树根>/.owner` 标记（`scripts/agent-worktree.sh new --scope …` 写，`own` 改）；
  2. **认领**：队列锁里**主人自己**的活跃认领的 `resources`（`--claim` 已经写了 ✓，不用新约定）；
  3. **收口提交**：队列事件里**主人自己**收口过的 `commit` 动过的路径（`--advance --to done`
     的证据①✓ —— 这一源**在 land rebase 之后仍然有效**，是 kh1 今天还能被咬住的原因 ✓）；
  4. **本分支自己的提交**：`merge-base <base> HEAD..HEAD` 动过的路径。

  ∪ **常允**（`ALWAYS_ALLOWED`：收尾义务的 `STATUS.md` / `docs/CI-FAILURES.md` ✓）。
  **不在**这个并集里的未提交路径 ⇒ **判红**（逐条点名 + 两条出路）。
  **证据为空**（没声明、没认领、没收口提交、分支也没提交过）⇒ **ⓘ 判不了**（印出路径、
  **不判红**：那时"不属于自己"没有可判据的定义，判红会是一条**没有出路的红** ✗）。

⚠ **如实划的边界**（不是密封 ✗）：① 这一条判的是**未提交**的路径 —— 一旦别人的活被提交进
本分支，它就变成了"本分支自己的证据"（源 3/4）⇒ **它必须跑在提交之前**，`scripts/githooks/pre-commit`
（`--staged`）就是那一半 ✓；② 判据只认**路径层**，不认"谁的手写的"（那需要 OS 级隔离 ✗）。

## 用法

    python3 scripts/check-multi-session.py            # 判当前工作树 / 当前分支（①②③）
    python3 scripts/check-multi-session.py --selftest # 自检：反例必须判红
    python3 scripts/check-multi-session.py --ownership  # 只判 ③（未提交路径的归属；land 用）
    python3 scripts/check-multi-session.py --staged     # 只判 ③，但看**暂存区**（pre-commit 钩子用）
    python3 scripts/check-multi-session.py --scope 'a/**,b'  # 覆盖"声明"那一源（复现/取证用）

退出码：0 = 不冲突 · 1 = 会冲突（判红）· 2 = 用法/环境错。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）

# **共享台账**：只有 main 能改。加台账**必须同时加进这里**（守卫与文档同一个真相源 ✓）。
SHARED_LEDGERS = (
    "docs/gaps/ledger.jsonl",
    "docs/e2e/ledger.jsonl",
)

# 链接工作树的判定：`git rev-parse --git-dir` 形如
#   <主仓库>/.git/worktrees/<名字>
# 而主工作树恒为 `.git`（相对）或 `<主仓库>/.git` ✓。
WORKTREE_MARK = "/worktrees/"


def git(*args: str, cwd: str | None = None) -> tuple[int, str]:
    # ⚠ `env=`（洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`）：`GIT_DIR` 优先于 `cwd` ⇒
    #   不清就会量到**调用者的仓**（本文件整条判据都建在这些读数上 ✗）。
    p = subprocess.run(["git", *args], capture_output=True, text=True, cwd=cwd,
                       env=git_env.clean_git_env())
    return p.returncode, (p.stdout if p.returncode == 0 else p.stderr).strip()


def in_linked_worktree() -> bool:
    rc, out = git("rev-parse", "--git-dir")
    return rc == 0 and WORKTREE_MARK in out


def blob(ref: str, path: str) -> bytes | None:
    """`ref:path` 的**原始字节**；不存在返回 None。

    ⚠ 用 `subprocess` 的 **bytes** 通道直接拿（**不用**上面那个 `git()` —— 它会 `.strip()`
    ⇒ **尾部换行被吃掉** ⇒ 拿它比"是否逐字节一致"会**永远判不等** ✗✓。
    "逐字节"就必须真的逐字节 ✓。
    """
    p = subprocess.run(["git", "show", f"{ref}:{path}"], capture_output=True,
                       env=git_env.clean_git_env())
    return p.stdout if p.returncode == 0 else None


def file_matches(ref: str, path: str) -> bool:
    """工作区文件与 `ref:path` 是否**逐字节一致**（少了文件 ⇒ False ✓）。"""
    want = blob(ref, path)
    if want is None:
        return True  # 该 ref 里没有这个文件 ⇒ 无从比较，不算分支的错
    try:
        with open(path, "rb") as fh:
            return fh.read() == want
    except FileNotFoundError:
        return False


def worktree_residue(base: str = "main") -> list[str]:
    """**残留工作树**（= 已经合回 `base` 的 `.worktrees/<id>`）⇒ 建议清理。

    ⚠ 只**报告**、不判红（2026-10-12 接口 agent 提交 `SUB-worktree-cleanup-drift` 的处置）：
    清理是**编排者**该做的事（`scripts/agent-worktree.sh rm <id>`），而"正在干活"的树**不该**
    被判红 —— 判红的代价是每个会话都被打断。但**沉默**的代价更大：实测一次收口后
    `git worktree list` 里躺着 **7 棵**（`h1`/`g110`/`n2`/`r07`/`iface`/`langA`/`lspA`），
    下一个接手者会以为**有 7 条在飞** ✗（本仓自己的规则："worktree 残留是已知坑"）。
    ⇒ 判据：`main..agent/<id>` 的 commit 数为 **0**（全合回/零 commit）且工作区干净 ⇒ 点名建议删。
    """
    rc, out = git("worktree", "list", "--porcelain")
    if rc != 0:
        return []
    trees: list[tuple[str, str]] = []
    path = ""
    for line in out.splitlines():
        if line.startswith("worktree "):
            path = line.split(" ", 1)[1]
        elif line.startswith("branch ") and path:
            trees.append((path, line.split(" ", 1)[1].removeprefix("refs/heads/")))
    stuck: list[str] = []
    for path, branch in trees:
        if ".worktrees/" not in path or not branch.startswith("agent/"):
            continue
        rc_n, n = git("rev-list", "--count", f"{base}..{branch}")
        rc_d, dirty = git("-C", path, "status", "--porcelain")
        if rc_n == 0 and int(n or 0) == 0 and rc_d == 0 and not dirty.strip():
            stuck.append(f"{path}（{branch}）")
    return stuck


# ══ 判据 ③ 的**证据面**（`WORKTREE-ISOLATION-NOT-ENFORCED`；唯一一处 ✓）══════════
# 规格在 docstring 的「判据 ③」；这里只有**常量**与**纯谓词**，取数在下面几个函数里。
OWNER_MARKER = ".owner"

# **常允**（每个 agent 都该能改、与"我的模块"无关的路径 = 仓库的**收尾义务**）：
#   · `STATUS.md`           = `AGENTS.md` 收尾义务（落 commit 前更新它 ✓）
#   · `docs/CI-FAILURES.md` = 同文件的「CI 失败记录」（CI 红了追加一条 ✓）
# ⚠ 这份名单**只作用于判据 ③**（新判据）—— 一条**存量**判据都没碰 ✓。
# ⚠ **只许减**：多一条 = 少一次假红，也少一次"这是别人的吗"的追问 ⇒ 增删都要写进 commit message ✓。
ALWAYS_ALLOWED = ("STATUS.md", "docs/CI-FAILURES.md")

# 判红时最多点名几条（防 20+ 条把日志刷爆；超出只印计数 ✓ —— 计数仍是完整的 ✓）。
MAX_NAMED = 25


def _norm_ident(text: str) -> str:
    """身份归一：`KH1` / `kh1` / `CHAIN-V` / `chainV` ⇒ `kh1` / `chainv` ✓。

    ⚠ 本仓的**实测约定**（不是新造的 ✗）：工作树 id 是小写（`.worktrees/kh1`），
    而 `--claim --owner` 里同一个会话写成大写（事件表里 `KH1` ✓；同族的
    `chainV` ↔ `CHAIN-V`、`rpY` ↔ `RP-Y` 说明连 `-` 都不稳定）⇒ 归一化是**必须的**，
    否则"主人自己的认领"一条都匹配不上 ⇒ 判据静默失效 ✗（**咬不住的守卫等于没有** ✗）。
    """
    return re.sub(r"[-_\s]", "", (text or "").strip().lower())


def glob_matches(path: str, globs: tuple[str, ...] | list[str]) -> bool:
    """`path` 是否落在这些范围里。**纯函数**（自检直接喂它 ✓）。

    ⚠ 三种写法都要认（队列的 `resources` 实测三种都出现过 ✗）：
      · `crates/front/**`（`RESOURCE_OF_OWNER` 的形状）⇒ **前缀**；
      · `crates/front` / `crates/front/`（`--touch` 手写的形状）⇒ **前缀**；
      · `docs/*.md`（带通配）⇒ 逐段通配 ✓。
    判"前缀"时必须**按目录边界**比（`crates/front` 不许匹配 `crates/frontend/x` ✗）。

    ⚠ **不用 `fnmatch`**（实测踩到 ✓）：它的 `*` **跨 `/`** ⇒ `docs/*.md` 会匹配
    `docs/design/x.md` ✗（假绿：范围被悄悄放大）。这里把 glob 翻成**路径感知**的正则：
    `**` ⇒ `.*` · `*` ⇒ `[^/]*` · `?` ⇒ `[^/]` ✓。
    """
    for g in globs:
        g = (g or "").strip()
        if not g:
            continue
        if g.endswith("/**"):
            g = g[:-3]
        g = g.rstrip("/")
        if not g:
            continue
        if g == path:
            return True
        if not any(ch in g for ch in "*?["):
            if path.startswith(g + "/"):
                return True
            continue
        if re.fullmatch(_glob_to_re(g), path):
            return True
    return False


def _glob_to_re(g: str) -> str:
    """把**路径感知**的 glob 翻成正则（`**` 跨目录、`*`/`?` 不跨 ✓）。**唯一一处** ✓。"""
    out: list[str] = []
    i = 0
    while i < len(g):
        ch = g[i]
        if ch == "*":
            if g.startswith("**", i):
                out.append(".*")
                i += 2
                continue
            out.append("[^/]*")
        elif ch == "?":
            out.append("[^/]")
        else:
            out.append(re.escape(ch))
        i += 1
    return "".join(out)


def ownership_split(dirty: list[str], evidence: list[str]) -> tuple[list[str], list[str]]:
    """**纯谓词**：把未提交路径分成 `(不属于主人的, 属于主人的)` ✓（自检直接喂反例 ✓）。

    ⚠ 这一条**只有一份**（`--ownership` / `--staged` / `check()` 都调它 ✓）——
    三处各写一遍就是三个真相 ✗（本仓的"一处定义"硬规则 0(b) ✓）。
    """
    foreign = [p for p in dirty if not glob_matches(p, evidence)]
    mine = [p for p in dirty if glob_matches(p, evidence)]
    return foreign, mine


def tree_root() -> Path | None:
    """当前工作树的**根**（`git rev-parse --show-toplevel`）。判不了 ⇒ None ✓。"""
    rc, out = git("rev-parse", "--show-toplevel")
    return Path(out) if rc == 0 and out else None


def worktree_id() -> str:
    """链接工作树的 id（`git rev-parse --git-dir` = `<主仓>/.git/worktrees/<id>` ✓）。

    ⚠ 用 **git 自己的**读数、不猜目录名：`SOKO_WORKTREE_DIR` 可以改父目录（`agent-worktree.sh:32`），
    而 git-dir 那个名字**恒等于** `agent-worktree.sh new <id>` 的 `<id>` ✓。
    """
    rc, out = git("rev-parse", "--git-dir")
    if rc != 0 or WORKTREE_MARK not in out:
        return ""
    return Path(out).name


def read_owner_marker(tree: Path) -> dict:
    """读 `<树根>/.owner`（`agent-worktree.sh new/own` 写 ✓）。缺失/坏了 ⇒ `{}` ✓。

    ⚠ 坏了**不判红**（那是"没声明"，不是"声明了却违反"）：`--ownership` 的 ⓘ 会如实印出来 ✓。
    """
    try:
        data = json.loads((tree / OWNER_MARKER).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return data if isinstance(data, dict) else {}


def queue_root(tree: Path) -> Path:
    """队列状态单点（`$SOKO_QUEUE_ROOT` → 本树 ✓，与 `ux-queue.py:250` **同一口径** ✓）。"""
    env = (os.environ.get("SOKO_QUEUE_ROOT") or "").strip()
    return Path(env) if env else tree


def live_claim_scopes(owner: str, tree: Path) -> tuple[list[str], list[str], str]:
    """队列锁里**主人自己的** / **别人的**活跃认领资源 ⇒ `(我的, 别人的, ⓘ)` ✓。

    ⚠ 锁文件在**主仓**的 git common dir（`ux-queue.py:1445` `lock_file()` ✓）——
    链接工作树里 `--git-common-dir` 指回主仓 ⇒ 与 `ux-queue.py` **同一份真相** ✓。
    ⚠ 只**读**，从不写（本文件是判据，不是工具 ✗）。锁坏了/不在 ⇒ 空（**不判红** ✓）。
    """
    rc, common = git("rev-parse", "--git-common-dir")
    if rc != 0 or not common:
        return [], [], ""
    path = Path(common)
    if not path.is_absolute():
        path = tree / path
    try:
        data = json.loads((path / "soko-ux-queue.lock").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return [], [], ""
    claims = data.get("claims") if isinstance(data, dict) else None
    if not isinstance(claims, list):
        return [], [], ""
    mine: list[str] = []
    others: list[str] = []
    for c in claims:
        if not isinstance(c, dict):
            continue
        res = [str(r) for r in (c.get("resources") or []) if str(r).strip()]
        if not res:
            continue
        if _norm_ident(str(c.get("owner") or "")) == owner:
            mine.extend(res)
        else:
            others.extend(res)
    note = ""
    if others:
        note = (f"锁里另有 {len(others)} 条**别人的**活跃认领资源（{', '.join(sorted(set(others))[:3])}…）"
                f"—— 落在那些范围里的未提交路径**点名**时会指出来源 ✓")
    return mine, others, note


def owner_commit_paths(owner: str, tree: Path) -> tuple[list[str], str]:
    """队列事件里**主人自己**收口过的 `commit` 动过的路径（**证据源 3** ✓）。

    ⚠ 为什么需要这一源（**kh1 实测** ✗→✓）：`land` 走 rebase + 快进 ⇒ 分支提交被重写成
    `main` 上的**孪生** ⇒ "本分支自己的提交"（源 4）在**已 land** 的树上**是空的** ✗ ——
    而 `.worktrees/kh1` 今天正是这个状态（`git cherry main agent/kh1` = `-`），
    只有**队列事件里那条收口 commit**（`--advance --to done` 的证据①✓）还留着
    "这棵树的主人动过 `crates/kernel/**`"这个事实 ✓。
    ⚠ 事件表读**队列单点**那一份（`$SOKO_QUEUE_ROOT` → 本树 ✓）；读不到/坏了 ⇒ 空（不判红 ✓）。
    ⚠ `git show` 解析不了的 sha（rebase 前的旧指针 / 只在别的分支上的对象）⇒ **跳过并如实印出来**
    （`ⓘ`），**不**当判红（那是"量不出来"，不是"违反" ✓）。
    """
    path = queue_root(tree) / "docs" / "ux-queue-events.jsonl"
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return [], ""
    shas: list[str] = []
    for raw in lines:
        raw = raw.strip()
        if not raw or raw.startswith("#"):
            continue
        try:
            ev = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if not isinstance(ev, dict) or _norm_ident(str(ev.get("owner") or "")) != owner:
            continue
        sha = str(ev.get("commit") or "").strip()
        if sha and sha != "None":
            shas.append(sha)
    out: list[str] = []
    skipped = 0
    for sha in dict.fromkeys(shas):          # 去重、保序
        rc, files = git("show", "--pretty=format:", "--name-only", sha)
        if rc != 0:
            skipped += 1
            continue
        out.extend(f for f in files.splitlines() if f.strip())
    note = ""
    if skipped:
        note = (f"ⓘ 队列里主人自己的 {skipped} 个收口 sha 在本树解析不了"
                f"（rebase 前的旧指针 / 对象不在这份历史里）⇒ 这一源少算了一部分，**不是判红** ✓")
    return out, note


def branch_own_paths(base: str, tree: Path) -> tuple[list[str], str]:
    """**证据源 4**：本分支自己的提交（`merge-base <base> HEAD`..`HEAD`）动过的路径 ✓。

    ⚠ 已 land（分支 tip 成了 `base` 的祖先）⇒ merge-base = `HEAD` ⇒ **空**（如实：那时这一源
    没有信息，靠源 3 兜 ✓）。量不出来 ⇒ 空 + `ⓘ`（**判不了 ≠ 违反** ✓）。
    """
    rc, mb = git("merge-base", base, "HEAD")
    if rc != 0 or not mb:
        return [], f"ⓘ 量不出与 `{base}` 的合并基 ⇒ 证据源 4（本分支自己的提交）**本跑为空** ✓"
    rc, out = git("diff", "--name-only", f"{mb}..HEAD")
    if rc != 0:
        return [], f"ⓘ `git diff {mb[:12]}..HEAD` 失败 ⇒ 证据源 4 **本跑为空** ✓"
    return [p for p in out.splitlines() if p.strip()], ""


def uncommitted_paths(tree: Path, staged: bool = False) -> list[str]:
    """未提交（`--staged` ⇒ 暂存区）的路径，**相对树根**、已去掉改名记录的第二半 ✓。

    ⚠ `-z` + `--porcelain` 是**机器可读**的那一种（`--porcelain=v1 -z`：条目以 NUL 分隔，
    改名/复制条目**多一个** NUL 分隔的旧路径 ✗）⇒ 不处理就会把旧路径也当"未提交路径"判 ✗。
    ⚠ 从**树根**跑（`git -C <树根>`）：从子目录跑的话 `--porcelain` 的路径是相对 cwd 的 ✗
    ⇒ 与"范围 glob"（仓库相对）根本对不上。
    """
    args = (["-C", str(tree), "diff", "--cached", "--name-only", "-z"] if staged
            else ["-C", str(tree), "status", "--porcelain", "-z"])
    p = subprocess.run(["git", *args], capture_output=True, text=True,
                       env=git_env.clean_git_env())
    if p.returncode != 0:
        return []
    if staged:
        return [t for t in p.stdout.split("\0") if t.strip()]
    out: list[str] = []
    fields = p.stdout.split("\0")
    i = 0
    while i < len(fields):
        f = fields[i]
        i += 1
        if len(f) < 4:
            continue
        status, path = f[:2], f[3:]
        out.append(path)
        if "R" in status or "C" in status:   # 改名/复制：下一个字段是**旧路径** ⇒ 跳过 ✓
            i += 1
    return [p_ for p_ in out if p_.strip()]


def ownership_problems(base: str = "main", staged: bool = False,
                       scope_override: str = "") -> tuple[list[str], list[str], str]:
    """**判据 ③**：本树里未提交（`--staged` ⇒ 暂存）的路径不属于本树的主人 ⇒ 判红。

    ⇒ `(红行, ⓘ行, 一行摘要)`。规格与边界见 docstring「判据 ③」✓。
    """
    tree = tree_root()
    if tree is None:
        return [], [], "归属校验：判不出工作树根 ⇒ 本跑不判（**不假装判过** ✗）"
    if not in_linked_worktree():
        return [], [], "归属校验：主工作树（编排者的场地）⇒ 判据 ③ **本跑不适用** ✓"

    wid = worktree_id()
    marker = read_owner_marker(tree)
    owner = _norm_ident(str(marker.get("id") or wid))
    if not owner:
        return [], [], "归属校验：判不出本树主人 ⇒ 本跑不判（**不假装判过** ✗）"

    info: list[str] = []
    declared = [str(s) for s in (marker.get("scope") or []) if str(s).strip()]
    if scope_override:
        declared = [s.strip() for s in scope_override.split(",") if s.strip()]
        info.append(f"ⓘ `--scope` 覆盖了声明那一源（{len(declared)} 条）—— 复现/取证用 ✓")
    elif not declared:
        info.append(f"ⓘ 这棵树**没有声明范围**（`<树根>/{OWNER_MARKER}` 不在或 `scope` 为空）"
                    f"⇒ 补：`scripts/agent-worktree.sh own {wid} --scope 'crates/xxx/**,…'` ✓")
    claims_mine, claims_others, claim_note = live_claim_scopes(owner, tree)
    if claim_note:
        info.append(claim_note)
    commits_mine, commit_note = owner_commit_paths(owner, tree)
    if commit_note:
        info.append(commit_note)
    branch_mine, branch_note = branch_own_paths(base, tree)
    if branch_note:
        info.append(branch_note)

    evidence = list(declared) + claims_mine + commits_mine + branch_mine + list(ALWAYS_ALLOWED)
    dirty = uncommitted_paths(tree, staged=staged)
    # ⚠ **判据自己的输入文件不算活**（`scripts/bite-worktree-foreign-paths.sh` 实测抓到 ✗→✓）：
    #   `<树根>/.owner` 是本判据**读的声明**，不是谁的工作产物。真仓里 `.worktrees/` 被
    #   `.gitignore:53` 忽略 ⇒ 它根本不进 `git status` ✓；但**夹具 / 没忽略它的仓 /
    #   `SOKO_WORKTREE_DIR` 指到别处**时它会以 `??` 出现 ⇒ 不排掉就**每次都自己判自己红** ✗
    #   （恒红 = 守卫失效 ✗，不是"咬得住"）。
    dirty = [p for p in dirty if p != OWNER_MARKER]
    where = "暂存区" if staged else "未提交"
    what = "暂存" if staged else "未提交"
    head = (f"归属校验（判据 ③ · {where}）：树 `{tree.name}`（主人 `{owner}`）· "
            f"证据 {len(set(evidence))} 条（声明 {len(declared)} · 认领 {len(claims_mine)} · "
            f"收口提交 {len(set(commits_mine))} · 本分支提交 {len(set(branch_mine))} · "
            f"常允 {len(ALWAYS_ALLOWED)}）· {where} {len(dirty)} 处")
    if not dirty:
        return [], info, head + " ⇒ 没有要判的路径 ✓"

    # ⚠ **"判不了 ≠ 判红"**：证据（除常允外）全空 ⇒ "不属于自己"没有可判据的定义 ⇒
    #   只印路径、**不判红**（判红会是一条**没有出路的红** ✗ —— 照 C5「0 个 rev 不判红」的先例 ✓）。
    #   ⚠ 但有一条**仍然确定**的子情形：某条未提交路径落在**别人活跃认领**的范围里
    #   —— 那是**别人已经声明过**的活 ⇒ 不需要"我的范围"就能判 ⇒ 这一档**照样判红** ✓。
    if not (declared or claims_mine or commits_mine or branch_mine):
        others_hit = [p for p in dirty if claims_others and glob_matches(p, claims_others)]
        if others_hit:
            red = [
                f"③ 本树主人 `{owner}` 的**{what}路径**里有一条落在**别人的活跃认领**范围里 ✗："
                f"`{p}`（别人已经声明过它 ⇒ 不需要「我的范围」就能判 ✓）"
                for p in others_hit[:MAX_NAMED]
            ]
            red.append(
                f"③ **共 {len(others_hit)} 处{what}路径落在别人的活跃认领范围里**"
                f"（树 `{tree.name}` · 主人 `{owner}`）⇒ 危险动作是 `git add -A`"
                f"（**别人的活会被卷进你的提交** ✗）⇒ **原样留着 + 报告编排者**"
                f"（**别 add、别 stash、别 revert、别删** ✗）")
            return red, info, head + f" ⇒ **{len(others_hit)} 处在别人的认领范围里** ✗"
        info.append(f"ⓘ 证据面为空（没声明 / 没认领 / 没收口提交 / 分支也没提交过）⇒ "
                    f"这 {len(dirty)} 处**归属判不了**，**不判红** ✓ —— 补一条声明即可让它可判："
                    f"`scripts/agent-worktree.sh own {wid} --scope '…'`")
        info.append("ⓘ 判不了的路径（如实印出，便于人眼过一遍）："
                    + " · ".join(dirty[:MAX_NAMED])
                    + (f" …（共 {len(dirty)} 处）" if len(dirty) > MAX_NAMED else ""))
        return [], info, head + " ⇒ 归属判不了（证据面为空）"

    foreign, mine = ownership_split(dirty, evidence)
    if not foreign:
        return [], info, head + f" ⇒ 全部落在主人的范围里 ✓（{len(mine)} 处）"

    others_hit = [p for p in foreign if claims_others and glob_matches(p, claims_others)]
    red: list[str] = []
    for p in foreign[:MAX_NAMED]:
        extra = "（⚠ 它落在**别人的活跃认领**范围里 ✗）" if p in others_hit else ""
        red.append(f"③ 本树主人 `{owner}` 的**{what}路径**里有一条不属于它：`{p}`{extra}")
    if len(foreign) > MAX_NAMED:
        red.append(f"③ ……另有 {len(foreign) - MAX_NAMED} 条同类（共 {len(foreign)} 处未归属；"
                   f"上面只点名了前 {MAX_NAMED} 条 ✓）")
    red.append(
        f"③ **共 {len(foreign)} 处{what}路径不在本树主人的证据范围里**（树 `{tree.name}` · 主人 `{owner}`）"
        f"⇒ ⭐ 这正是 `WORKTREE-ISOLATION-NOT-ENFORCED` 的形状（别人的活被写进这棵树 ✗；"
        f"危险动作是 `git add -A` ⇒ **别人的活会被卷进你的提交** ✗）。两条出路："
        f"① **是你的**（本分支还没提交过它们）⇒ 把范围声明出来："
        f"`scripts/agent-worktree.sh own {wid} --scope '<你的 glob,…>'` ✓；"
        f"② **不是你的** ⇒ **原样留着 + 报告编排者**（**别 add、别 stash、别 revert、别删** ✗）"
        f"，并让真正的作者在自己的树里落它 ✓")
    return red, info, head + f" ⇒ **{len(foreign)} 处未归属** ✗"


def check(base: str = "main", scope: str = "") -> int:
    bad: list[str] = []

    # ── 判据 ⓪：残留工作树（只报告不判红，见 `worktree_residue` 的说明）─────────
    for tree in worktree_residue(base):
        print(f"⏳ 残留工作树（已全合回 `{base}` 且干净）⇒ 建议 "
              f"`scripts/agent-worktree.sh rm <id>` 清掉：{tree}", file=sys.stderr)

    # ── 判据 ①：链接工作树里不许改共享台账 ──────────────────────────────
    if in_linked_worktree():
        rc, branch = git("branch", "--show-current")
        branch = branch or "(detached)"
        for led in SHARED_LEDGERS:
            if not file_matches(base, led):
                # ⚠ **文案必须分清三种情形**（2026-09-28 两轮实测才修对 ✓）：
                #   ① 工作区改了（未提交）· ② 分支提交里改了 · ③ **只是落后**（用户什么都没改 ✗✓）。
                # 判据对三者**都一样判红** ✓（都会冲突），但"怎么修"完全不同 ⇒ 说出来 ✓。
                # 第一版一律说"你改了" ✗ —— 在**只是落后**的分支上那句话是错的 ✓；
                # 第二版拿"工作区 vs HEAD"分辨 ✗ —— 落后时两者**相同** ⇒ 仍被误判成"提交里改了" ✓。
                # **正解**：拿**合并基**分辨 —— 分支相对合并基有没有动过台账 ✓。
                # ⚠ **顺序有讲究**（第三轮实测 ✓）：先判**工作区**（相对 HEAD），
                # 再判**合并基**（分支提交相对"分叉点"动没动）。
                # 反过来写会漏：在"从旧提交开出来的分支"上，工作区改了台账，
                # 但**合并基上那份台账本来就没动** ⇒ 落到"落后"分支 ⇒ 说错话 ✗✓。
                if not file_matches("HEAD", led):
                    where, kind = "工作区里", "工作区"
                else:
                    rc_mb, mb = git("merge-base", "HEAD", base)
                    touched = rc_mb != 0 or blob(mb, led) != blob("HEAD", led)
                    where, kind = ("提交里", "提交") if touched else ("", "落后")
                if kind == "落后":
                    why = (f"分支 `{branch}` 的 `{led}` **落后于 `{base}`**（本分支没动过它，"
                           f"是 `{base}` 往前走了 —— `e2e ledger` job 每轮往 `{base}` 回提交）")
                    fix = f"`git rebase {base}`（落后不是你的错，但不 rebase 就合不回去 ✓）"
                else:
                    why = (f"分支 `{branch}` **在{where}改了** `{led}` ✗ —— 它只在 `{base}` 上改"
                           f"（合回时必与 `{base}` 冲突）")
                    fix = (f"把该改动**撤出本分支**，改到 `{base}` 上做："
                           f"`git checkout {base} -- {led}`；确需保留 ⇒ rebase 后只留 `{base}` 版 ✓")
                bad.append(f"① 链接工作树：{why}；修法：{fix}")
    else:
        pass  # 主工作树（= main 的场地）改台账是**正常**的 ✓

    # ── 判据 ②：台账落后 main ⇒ 合回必冲突 ─────────────────────────────
    rc, cur = git("branch", "--show-current")
    if rc == 0 and cur and cur != base:
        for led in SHARED_LEDGERS:
            mine, theirs = blob("HEAD", led), blob(base, led)
            if mine is None or theirs is None:
                continue
            if mine != theirs:
                bad.append(
                    f"② `{led}` 与 `{base}` 不一致 ⇒ 合回时**必冲突** ✗ ⇒ 先 rebase："
                    f"`git rebase {base}`（台账冲突一律**取 `{base}` 的版本**，然后在 `{base}` 上重做你的台账改动）"
                )

    # ── 判据 ③：本树里未提交的路径不属于本树的主人（洞 WORKTREE-ISOLATION-NOT-ENFORCED）──
    #   ⚠ 与 ①② **同一个谓词**（`ownership_problems()` ⇒ `--ownership` / `--staged` / 钩子
    #   都调它 ✓）—— 三处各写一遍就是三个真相 ✗。
    own_red, own_info, own_head = ownership_problems(base, scope_override=scope)
    for line in own_info:
        print(line, file=sys.stderr)
    if own_red:
        bad.extend(own_red)
    else:
        print(f"  ✓ {own_head}", file=sys.stderr)

    if bad:
        print("multi-session：会冲突 ✗\n", file=sys.stderr)
        for b in bad:
            print(f"  ✗ {b}", file=sys.stderr)
        print(
            "\n**正解不是加锁，是 worktree 隔离**（用户原话：「加锁 = 让它们排队；"
            "worktree = 让它们物理上不相干 —— 后者根本得多」）✓：\n"
            "  git worktree add -b <分支> ../soko-<分支> main     # 每条会话一个工作树\n"
            "  # 独立 target（**别共享** —— 会被 cargo 加锁串行，抵消并行收益 ✗）：\n"
            "  CARGO_TARGET_DIR=$PWD/target-<分支> scripts/soko gate\n"
            "台账（`docs/gaps/ledger.jsonl` / `docs/e2e/ledger.jsonl`）**只在 main 上改** ✓。",
            file=sys.stderr,
        )
        return 1

    where = "链接工作树" if in_linked_worktree() else "主工作树"
    print(f"multi-session: ✓ {where} · 分支 `{cur or '(detached)'}` —— 共享台账未偏离 `{base}` ✓")
    return 0


# ── 自检：**反例必须判红**（守卫咬不住就等于没有 ✓）────────────────────
def selftest() -> int:
    """纯函数级自检：把两条判据的**判定逻辑**单独喂反例。

    真起一个 worktree 太重（且会污染仓库 ✗）⇒ 这里验的是**判据本身**
    （`file_matches` / 落后判定），worktree 探测由 `check()` 的实跑覆盖 ✓。
    """
    import tempfile
    import os

    fails = 0
    with tempfile.TemporaryDirectory() as td:
        a = os.path.join(td, "same.jsonl")
        b = os.path.join(td, "diff.jsonl")
        with open(a, "w", encoding="utf-8") as fh:
            fh.write('{"g":"G-01"}\n')
        with open(b, "w", encoding="utf-8") as fh:
            fh.write('{"g":"G-99"}\n')

        # `HEAD:docs/gaps/ledger.jsonl` 的真实内容拿出来当"基准"
        ref = blob("HEAD", SHARED_LEDGERS[0])
        if ref is None:
            print("  ✗ selftest：拿不到 HEAD 的台账 ⇒ 环境错", file=sys.stderr)
            return 2
        same = os.path.join(td, "ledger-same.jsonl")
        diff = os.path.join(td, "ledger-diff.jsonl")
        with open(same, "wb") as fh:
            fh.write(ref)  # **原样字节** ⇒ 必须判"一致" ✓
        with open(diff, "wb") as fh:
            fh.write(ref + b'{"g":"G-XX","status":"open"}\n')

        # 直接验**判据谓词**（`工作区字节 == 基准字节`）—— 不绕 `file_matches` 的 ref 解析，
        # 因为临时文件**不在 git 里** ⇒ 走 ref 解析会得到"该 ref 无此文件 ⇒ True" ✗✓。
        def consistent(path: str) -> bool:
            try:
                with open(path, "rb") as fh:
                    return fh.read() == ref
            except FileNotFoundError:
                return False

        cases = [
            ("与基准逐字节相同 ⇒ 不判红", same, True),
            ("多了一行 ⇒ 必须判红", diff, False),
            ("文件不存在 ⇒ 必须判红", os.path.join(td, "nope.jsonl"), False),
        ]
        for label, path, want_ok in cases:
            got_ok = consistent(path)
            if got_ok != want_ok:
                fails += 1
                print(f"  ✗ selftest 反例失败：{label} ⇒ got_ok={got_ok} 期望 {want_ok}", file=sys.stderr)
        print(f"  ✓ 台账一致性判据：{len(cases)} 个反例全部符合预期")

        # `file_matches` 本身也要验一次**真路径**（它在 `check()` 里就是这么被调的 ✓）
        if not file_matches("HEAD", same.replace("ledger-same.jsonl", "x.jsonl")):
            pass  # 走不到也没关系：下面用真台账文件验 ✓
        if not file_matches("HEAD", SHARED_LEDGERS[0]):
            fails += 1
            print(
                f"  ✗ selftest 反例失败：工作区的 `{SHARED_LEDGERS[0]}` 与 HEAD **逐字节不一致** "
                f"⇒ 要么你改了台账（那就该判红 ✓），要么 `file_matches` 坏了 ✗",
                file=sys.stderr,
            )

    # 落后判据：两份不同的内容必须被认成"不一致" ✓
    if blob("HEAD", SHARED_LEDGERS[0]) == blob("HEAD", SHARED_LEDGERS[1]):
        print("  ⚠ 两份台账内容相同 ⇒ 落后判据在本次自检里区分不出（不算失败，但覆盖变弱）", file=sys.stderr)

    fails += _selftest_ownership()

    if fails:
        print(f"multi-session --selftest：{fails} 个反例不达预期 ✗", file=sys.stderr)
        return 1
    print("multi-session --selftest：✓ 判据咬得住（含 2 个「必须判红」+ 判据 ③ 的正反两向）")
    return 0


def _selftest_ownership() -> int:
    """**判据 ③ 的纯函数级自检**（正反两向 ⇒ 证明它**不恒真也不恒假** ✓）。

    ⚠ 为什么验**谓词**而不是端到端（照本文件既有那条的先例 ✓）：真起一棵链接工作树
    会污染仓库 ✗（而且本判据的端到端读数由 `--ownership` 的实跑 + `docs/guard-bite.jsonl`
    那条 bite 覆盖 ✓）。这里钉的是**最容易写错的两处**：
      · `glob_matches` 的**目录边界**（`crates/front` 不许匹配 `crates/frontend/x` ✗）；
      · `ownership_split` 的**方向**（落在证据里的**不许**被报成"不属于主人" ✗ —— 那是假红）。
    """
    fails = 0

    def expect(label: str, got: object, want: object) -> None:
        nonlocal fails
        if got != want:
            fails += 1
            print(f"  ✗ selftest 反例失败（判据 ③）：{label} ⇒ got={got!r} 期望 {want!r}",
                  file=sys.stderr)

    # ── `glob_matches`：三种写法 + **目录边界**（kh1 的真形状：声明 kernel，混进 front ✗）──
    ev = ["crates/kernel/**", "STATUS.md", "docs/*.md"]
    for label, path, want in [
        ("`crates/kernel/**` 覆盖它自己", "crates/kernel/src/util.rs", True),
        ("**目录边界**：`crates/kernel` 不许匹配 `crates/kernelx/a.rs` ✗", "crates/kernelx/a.rs", False),
        ("`STATUS.md` 精确匹配", "STATUS.md", True),
        ("`docs/*.md` 通配", "docs/CI-FAILURES.md", True),
        ("`docs/*.md` 不许匹配子目录 ✗", "docs/design/x.md", False),
        ("⭐ kh1 的真形状：`crates/front/**` **不在**声明里 ⇒ 必须判「不属于主人」✗",
         "crates/front/src/ast.rs", False),
        ("无通配的目录写法也要认（`--touch` 的形状）", "crates/front/src/ast.rs", None),
    ]:
        if want is None:
            expect(label, glob_matches(path, ["crates/front"]), True)
            expect(label + "（`crates/front` 不许匹配 `crates/frontend` ✗）",
                   glob_matches("crates/frontend/a.rs", ["crates/front"]), False)
            continue
        expect(label, glob_matches(path, ev), want)

    # ── `ownership_split`：**方向**必须对（落在证据里的**不许**被点名 ✗）──
    dirty = ["crates/kernel/src/util.rs", "STATUS.md", "crates/front/src/ast.rs",
             "crates/front/src/expr.rs"]
    foreign, mine = ownership_split(dirty, ["crates/kernel/**", "STATUS.md"])
    expect("混进别人的 2 条必须被点名", foreign, ["crates/front/src/ast.rs", "crates/front/src/expr.rs"])
    expect("自己的 2 条不许被点名（假红 ✗）", mine, ["crates/kernel/src/util.rs", "STATUS.md"])
    expect("空未提交 ⇒ 两边都空（不恒真 ✗）", ownership_split([], ["crates/kernel/**"]), ([], []))
    expect("常允面也要算数", ownership_split(["docs/CI-FAILURES.md"], list(ALWAYS_ALLOWED)), ([], ["docs/CI-FAILURES.md"]))

    # ── `_norm_ident`：本仓**实测**的四种拼法（工作树 id ↔ `--claim --owner`）──
    for a, b in [("KH1", "kh1"), ("CHAIN-V", "chainV"), ("RP-Y", "rpY"), ("cw2", "CW2")]:
        expect(f"身份归一 `{a}` ≈ `{b}`", _norm_ident(a), _norm_ident(b))

    if not fails:
        print("  ✓ 判据 ③（未提交路径的归属）：纯谓词正反两向全部符合预期 "
              "（含「混进别人的路径必须被点名」+「自己的不许被点名」）")
    return fails


def main() -> int:
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--base", default="main", help="基线分支（默认 main）")
    ap.add_argument("--selftest", action="store_true", help="自检：反例必须判红")
    ap.add_argument("--ownership", action="store_true",
                    help="只判 ③（未提交路径的归属）；`land` 用它点名")
    ap.add_argument("--staged", action="store_true",
                    help="只判 ③，但看**暂存区**（`scripts/githooks/pre-commit` 用它拦提交）")
    ap.add_argument("--scope", default="",
                    help="覆盖 ③ 的「声明」那一源（逗号分隔；复现 / 取证用 ✓）")
    ap.add_argument("--print-live-scope", action="store_true",
                    help="印出本树主人的**活跃认领资源**（`agent-worktree.sh own --from-claim` 用 ✓）")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.print_live_scope:
        # ⚠ **一处定义**：`agent-worktree.sh own --from-claim` 走这里取"认领"那一源
        #   —— 归一化规则（`_norm_ident`）只在本文件写一遍 ✓（不在这里重写一份 ✗）。
        tree = tree_root()
        if tree is None or not in_linked_worktree():
            print("live-scope: 不是链接工作树 ⇒ 没有「本树主人」可谈 ✗", file=sys.stderr)
            return 3
        marker = read_owner_marker(tree)
        owner = _norm_ident(str(marker.get("id") or worktree_id()))
        mine, _, _ = live_claim_scopes(owner, tree)
        if not mine:
            print(f"live-scope: 主人 `{owner}` 没有活跃认领 ⇒ **量不出来**（≠「范围是空的」✗）",
                  file=sys.stderr)
            return 3
        for r in dict.fromkeys(mine):
            print(r)
        return 0
    if args.ownership or args.staged:
        # ⚠ 这两条**只**判 ③（①② 要基线分支、且与"这次提交了什么"无关 ✗）——
        #   所以它们**先自带前提**（不要求 `main` 在 ✗：钩子可能在任何分支上跑 ✓）。
        red, info, head = ownership_problems(args.base, staged=args.staged,
                                             scope_override=args.scope)
        for line in info:
            print(line, file=sys.stderr)
        if red:
            # ⚠ **读数必须可归属**（本仓的既定纪律 ✓）：判红时也把那一行**证据计数**印出来
            #   —— 否则"20 处不属于主人"这句话的**依据**（哪几源、各几条）看不见 ✗。
            print(f"  ✗ {head}", file=sys.stderr)
            print("归属校验：**有路径不属于本树的主人** ✗\n", file=sys.stderr)
            for r in red:
                print(f"  ✗ {r}", file=sys.stderr)
            return 1
        print(f"  ✓ {head}", file=sys.stderr)
        return 0
    rc, _ = git("rev-parse", "--verify", args.base)
    if rc != 0:
        print(f"multi-session: 找不到基线分支 `{args.base}` ✗", file=sys.stderr)
        return 2
    return check(args.base, scope=args.scope)


if __name__ == "__main__":
    sys.exit(main())
