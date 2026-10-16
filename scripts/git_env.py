#!/usr/bin/env python3
"""**干净 git 环境**：仓库发现**只许由 `cwd` 决定** ⇒ 一律摘掉 `GIT_*` 环境变量。

## 为什么有这个文件（洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`，2026-10-16）

⭐⭐ **夹具的 `git` 命令会打到调用者的仓上** ✗ —— 实测（干净 clone + 链接工作树，
`GIT_DIR=<主>/.git/worktrees/<wt> python3 scripts/check-attention-budget.py --selftest`）：

    core.bare              false → **true**                      ✗
    .git/config sha256     3388889a… → **b3398260…**             ✗
    主 checkout            **fatal: this operation must be run in a work tree** ✗
    夹具提交               2 个 `feat(front): 第 N 轮产品` 落到**工作树分支** ✗
    工作树 dirty           **238 行**                             ✗
    .git/config 新增       `user.email=fixture@example.invalid` · `user.name=fixture` ✗

⇒ 后果不是"读数不可归属"（那是轻的一档），是**夹具改掉了真仓的状态** ✗：
`git revert` / `git mv` / `git add` 全部失败，且**提交身份被顶成 `fixture`**
（本地 `user.*` 压过全局 ⇒ `git log --format='%an'` 记成夹具身份 ✗）。

⚠ **机制**（最小探针）：`GIT_DIR=<主>/.git/worktrees/<wt> git init -q -b main <tmp>/repo`
（cwd 在 tmp）⇒ `git init` **不认 cwd、认 `GIT_DIR`** ⇒ 直接把 `core.bare=true`
写进**共享的** `.git/config` ✓。`GIT_DIR` 优先于 cwd 决定仓库发现 ⇒ 之后每一句 `git`
（`add` / `commit` / `config`）都落在调用者的仓上 ✗。

⚠ **`GIT_DIR` 从哪来**（实测 ✓，别猜 ✗）：普通 `git push` **不**导出它；
**从链接工作树里 `git push`** 才导出 `GIT_DIR=<主>/.git/worktrees/<wt>` ⇒
`scripts/githooks/pre-push` → `scripts/ci-local.sh:268-271` 的
`check-attention-budget.py --selftest` 就在那个环境里 ⇒ **只在"从 hook 里跑 selftest"时中招** ✗
（CI 不跑 pre-push ⇒ CI 一直是绿的 ✓，所以这条活了一个月才被抓到）。

## 口径（**一处定义**，`AGENTS.md` 硬规则 0(b)）

本函数是**全仓唯一一处**：所有会跑 `git` 的脚本都从这里取环境 ✓，别在每个
`subprocess.run` 边上各写一遍 ✗（先例 `scripts/check-queue-rewrite-trigger.py` 的
`clean_git_env()` 已改为**委托本模块** —— 它的实测读数留在那边的 docstring 里 ✓）。

⚠ **摘的是前缀 `GIT_` 全族**（不是点名的几个）：`GIT_DIR` / `GIT_WORK_TREE` /
`GIT_INDEX_FILE` / `GIT_COMMON_DIR` / `GIT_OBJECT_DIRECTORY` /
`GIT_ALTERNATE_OBJECT_DIRECTORIES` / `GIT_PREFIX` … 一次覆盖 ✓ —— 点名的写法下次
git 新增一个 `GIT_*` 就又漏一个 ✗。
⚠ `GIT_AUTHOR_*` / `GIT_COMMITTER_*` **一并摘掉也更好** ✓：提交身份应当来自仓库配置，
不该被钩子的调用方注入（与先例同口径 ✓）。**要显式给身份的**（夹具 / 可复现提交）⇒
摘完再 `{**clean_git_env(), "GIT_AUTHOR_DATE": …}` 自己加回去 ✓
（`scripts/check-guard-bite.py` 的 C5 夹具就是这么用的 ✓）。

## 用法

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import git_env  # noqa: E402  （同目录库，照 `threshold_ratchet.py` 的先例 ✓）

    subprocess.run(["git", …], cwd=repo, env=git_env.clean_git_env())
    # 要额外变量：env={**git_env.clean_git_env(), "GIT_TERMINAL_PROMPT": "0"}
"""
from __future__ import annotations

import os


def clean_git_env(base: dict | None = None) -> dict:
    """`base`（缺省 `os.environ`）去掉全部 `GIT_` 前缀变量后的副本 ✓。

    ⚠ **返回新 dict**（不改 `os.environ` ✓）：调用方可能只想给**子进程**一个干净环境，
    而不想污染本进程后续的 git 调用（例如 `docs-lint.py` 还要用 cwd 语义读真仓 ✓）。
    """
    src = os.environ if base is None else base
    return {k: v for k, v in src.items() if not k.startswith("GIT_")}
