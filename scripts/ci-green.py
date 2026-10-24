#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""ci-green.py —— 「真绿」判定：**skipped 不算绿**（2026-09-27 前置 2 ✓）

为什么需要它 ✗✓（2026-09-27 复盘第 1 条，**实测翻车** ✓）：
  所谓「CI 连续 6 次绿」**是假的** ✗ —— 那几次是**纯 docs 推送** ✓ ⇒ `changes` 过滤器
  判定 rust/courses/editor 都没变 ✓ ⇒ `test` / `gates-course` / `ledger` / `contract` /
  `e2e` **全部 skipped** ✗，只有 5 个 lint 类 job 真跑 ✓ ⇒ **整轮 conclusion 仍是
  `success`** ✗ ⇒ 拿它当「CI 绿」= **拿一个什么都没验证的轮次当证据** ✗。

  ⚠ 已有的 `scripts/ci-watch.sh` **只判 failure** ✗：它对这种轮次照样说
  「✓ 最新一轮暂无失败 ✓」✗ ⇒ **咬不住这个已知的历史 bug** ✗。
  **咬不住的守卫等于没有** ✓ ⇒ 本脚本是它的**强度层** ✓：
  逐 job 分三类，**skipped 单独列出、绝不折进绿** ✓。

判据（**可执行** ✓，两条都用**真轮次**当夹具 ✓）：
  scripts/ci-green.py --run 36291907128   # 假绿夹具（纯 docs 推送）→ 必须 exit 2
  scripts/ci-green.py --run 36285752786   # 真绿夹具（0.73.0 发版 bump）→ 必须 exit 0
  scripts/ci-green.py --selftest          # 离线判据通道自检（不需要 gh / 网络）

  **反向验证**（撤掉修复必须判红 ✓）：把 `judge()` 里的强度层去掉、只留
  `if failures: red`（= 退回 `ci-watch.sh` 的弱判据 ✓）⇒ 假绿夹具会变成 exit 0 ✗
  ⇒ `--selftest` 的第 2 个用例立刻判红 ✓（见 `--selftest` 的实现 ✓）。

退出码：
  0 = 真绿（**无失败** 且 **重活真跑且 success**）
  1 = 红（有 job failure / cancelled / timed_out / …）
  2 = 假绿（无失败，但重活被 skipped ⇒ **不构成证据**）
  3 = 还不能判（整轮没跑完 / gh 缺失 / run 不存在 / ci.yml 有新 job **未分类**）

⚠ 为什么"未分类的 job"要 exit 3 ✗✓：`ci.yml` 将来加一个重活 job 时，
  本脚本若不认识它 ⇒ 它被 skip 也不会判红 ✗ ⇒ **守卫静默腐烂** ✗。
  所以分类表是**棘轮** ✓：`ci.yml` 里出现任何未分类的 job id ⇒ **直接拒绝判定** ✓。

## 2026-10-16 修的三处腐烂（洞 `CI-GREEN-READOUT-ROTTED`，medium ✓）

**改前实测读数**（main `4adae13e` 那一刻，逐条都能复跑 ✓）：

  ①  `python3 scripts/ci-green.py --run 38884242478`（最新 completed run）
      ⇒ **rc=3**：`ci.yml` 有 **4 个未分类 job**（`tactic-docs-lint` · `wiring-lint` ·
      `board` · `mark-verified`）+ **1 个认不出的显示名**（`wiring-lint（…`）✗
      ⇒ **量具自己烂了**：没人能自动核"这一轮 CI 到底绿没绿" ✗。
  ②  `python3 scripts/ci-green.py --selftest` ⇒ **rc=1**（**3/6 用例不符** ✗）——
      同一个腐烂在**它自己的离线判据通道**里就红了 ✓，而**没有任何 job / gate 跑它** ✗
      （`grep -rn ci-green .github scripts/ci-local.sh` 只命中一条注释）
      ⇒ **守卫空转**：它红了多久都没人知道 ✗。
  ③  `python3 scripts/ci-green.py --run 4adae13e`（**一个 sha**，不是 run id）
      ⇒ `gh` 404 ⇒ 脚本印 `…（exit 3）` 而**实退 `1`** ✗ —— 因为
      `sys.exit("<字符串>")` 的退出码恒为 **1** ✓，而 **`1` 在本脚本里 = 红** ✗
      ⇒ **"前提缺失"被印成"CI 红"** ✗（读 rc 的人会照抄成"这一轮红了" ✗）。

**改法**（三处，一处都没放宽判据 ✗ —— `skipped 不算绿` 那条强度层逐字未动 ✓）：

  * **分类**：`tactic-docs-lint` / `wiring-lint` / `board` **无 `if:` ⇒ 每推必跑**
    ⇒ 进 `ALWAYS`（**被 skip 属异常** ⇒ 照旧判假绿 ✓，**比"不分类"更严** ✓）；
    `mark-verified` 有 `if:`（`always() && main push && 无 failure && rust|courses|editor`）
    ⇒ 它是**写标记**动作、按设计可合法 skip ⇒ 进 `CONDITIONAL` ✓。
  * **显示名截断**：GitHub API 的 `jobs[].name` **按字节截断到 100**（实测
    `wiring-lint（…）` 118 字节 ⇒ 回来是 `…跨接缝 + ...`，62 字符 / 100 字节）✗
    ⇒ 精确匹配**永远**认不回它 ✗ ⇒ 新增 `match_truncated()`：**只对以 `...` 结尾的名字**、
    且**必须唯一**前缀命中某个 `ci.yml` 名字 ⇒ 才认回（**歧义 / 认不回 ⇒ 照旧 exit 3** ✓，
    认回时**印出来** ⇒ 不是静默容忍 ✓）。
  * **退出码**：新增 `bail()`（印 stderr + `sys.exit(EXIT_NOT_JUDGED)`）替换 4 处
    `sys.exit("<字符串>")` ⇒ **前提缺失 = 3**，与 `1=红` 分开 ✓。
  * **`--run` 收 sha**：`gh run list --commit` 只认**全 sha**（实测短 sha 回 `[]` ✗）
    ⇒ 先用 `git rev-parse` 展开（**过 `git_env.clean_git_env()`** ✓，同
    `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO` 的修法）再取该 commit 的**最新** ci 轮次，
    并把解析结果印到 **stderr**（`--json` 的 stdout 保持干净 ✓）。

⚠ **已知边界（如实 ✗）**：`always_skipped` 只抓"**在轮次里但被 skip**"的 `ALWAYS` job；
**整个缺席**（`by_id` 里没有）不判红 —— 两条历史夹具（`36285752786` / `36291907128`）
早于那三个 lint job ⇒ 加"缺席即假绿"会把**真历史形状**判红 ✗。实测今天每个 `ALWAYS`
job 都在（33/33 条齐全 ✓）⇒ 本条不改变任何现行读数 ✓。
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CI_YML = REPO_ROOT / ".github" / "workflows" / "ci.yml"

# ── 分类表（**棘轮** ✓：ci.yml 里的每个 job id 都必须落在下面四组之一，否则 exit 3）──

# 验证承载 job：**被 skip ⇒ 这一轮不构成证据** ✗（假绿 ✓）
HEAVY = [
    "test",          # 4 crate 矩阵 + 3 种 kind
    "perf-gate",     # ⚠ 第一轮 continue-on-error（只报不拦）⇒ 它 success 也只算"跑过了"
    "gates-fast",
    "gates-course",
    "ledger",
    "contract",
    "editor",
    "e2e",           # ubuntu 矩阵（含最低支持版本 1.106.0）
    "e2e-macos",
]
# 只在 `refs/heads/main` 的 push 上跑 ⇒ PR 上被 skip 是**设计如此** ✓，不算弱
HEAVY_MAIN_ONLY = ["e2e-ledger"]

# 无 `if:` ⇒ 每轮都该跑；被 skip 属异常
ALWAYS = [
    "changes",
    "lint-fmt",
    "lint-clippy",
    "docs-lint",
    "status-lint",
    # ⭐ 2026-10-16（洞 `CI-GREEN-READOUT-ROTTED`）：下面三个**都是无 `if:` 的独立
    # lint job** —— `ci.yml` 自己的注释就写着它们"照 `status-lint`/`docs-lint` 的形状：
    # 独立 job、不设 `if:`、不加过滤器" ✓（`wiring-lint` 那段注释还给了实测：
    # "17 轮里 0 次 skipped" ✓）。**按定义**它们被 skip 就是异常 ⇒ 进 ALWAYS
    # （⇒ 被 skip 判**假绿** ✓）—— ⚠ 这比"留在未分类里"**更严**，不是放宽 ✗。
    "tactic-docs-lint",   # tactic 清单三处一致 + selftest（ci.yml:102）
    "wiring-lint",        # harness 接线漂移 + 启动器链接 + LSP marker + 推送范围（ci.yml:154）
    "board", "guard-bite", "budgets", "site", "hygiene",
]

# 按设计可合法 skip
CONDITIONAL = [
    "auto-tag",       # 只在 main 的 push 上跑（发版动作，不是验证）
    "fast-fail",      # 只在**已经有 failure** 时才跑 ⇒ 真绿时它必然 skipped ✓
    # ⭐ 2026-10-16（同洞）：它是**写标记**动作、不是验证承载 job ——
    # `if: always() && github.event_name == 'push' && github.ref == 'refs/heads/main'
    #      && !contains(needs.*.result, 'failure') && !contains(needs.*.result, 'cancelled')
    #      && (rust || courses || editor)`（ci.yml:591-596）⇒ **PR 上 / 纯 docs 推送上
    # 按设计必然 skip** ✓。⚠ 真出错时它不会静默：`needs` 里有 failure/cancelled ⇒
    # 那一项本身已经落在 `BAD` 里 ⇒ 先判红 ✓。
    "mark-verified",
]

CLASSIFIED = set(HEAVY) | set(HEAVY_MAIN_ONLY) | set(ALWAYS) | set(CONDITIONAL)

# 非绿结论（GitHub job conclusion 全集里"不是绿"的那些）
BAD = {"failure", "cancelled", "timed_out", "action_required", "stale", "startup_failure"}

EXIT_TRUE_GREEN, EXIT_RED, EXIT_FAKE_GREEN, EXIT_NOT_JUDGED = 0, 1, 2, 3

sys.path.insert(0, str(Path(__file__).resolve().parent))
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）


def bail(msg: str) -> None:
    """**前提缺失 ⇒ exit 3**（⚠ 绝不 `sys.exit("<字符串>")` —— 那恒退 **1** ✗）。

    ⚠ 本仓约定：`exit 3` = "前提缺失 ⇒ 判不了" ✓；而 **`1` 在本脚本里 = 红** ✗
    ⇒ 用 `sys.exit(str)` 会把"gh 缺失 / run 不存在 / ci.yml 找不到"印成 **CI 红** ✗。
    **实测**（2026-10-16，洞 `CI-GREEN-READOUT-ROTTED`）：`--run 4adae13e`（一个 sha，
    不是 run id）⇒ `gh` 404 ⇒ 脚本印 `…（exit 3）`而**实退 1** ✗ ⇒ 读 rc 的人会
    照抄成"这一轮红了" ✗。⇒ 前提缺失**一律**走这里 ✓。
    """
    print(msg, file=sys.stderr)
    sys.exit(EXIT_NOT_JUDGED)


def parse_ci_jobs(path: Path = CI_YML) -> dict[str, str | None]:
    """从 ci.yml 取出 {job_id: 显式 name 或 None}。

    轻量解析 ✓（不引 pyyaml —— 托管版 python3 缺它会让门禁**假红** ✗）：
    只认 `jobs:` 段里 **2 空格缩进**的 `  <id>:`，以及其下 **4 空格缩进**的 `name:`
    （step 的 name 在 8 空格、`if: >-` 的续行在 6 空格 ⇒ 都不会误命中 ✓）。
    """
    if not path.is_file():
        bail(
            f"✗ 找不到 {path} ✗ —— 本脚本按 `__file__` 定位仓库根，必须从仓库内运行\n"
            f"   （**不要**把它拷到仓库外再跑：那时仓库根会解析错 ⇒ 判定无意义 ✗）（exit 3）"
        )
    jobs: dict[str, str | None] = {}
    in_jobs = False
    cur: str | None = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not in_jobs:
            if raw.startswith("jobs:"):
                in_jobs = True
            continue
        if raw and not raw[0].isspace():  # 回到顶层键 ⇒ jobs 段结束
            break
        m = re.match(r"^ {2}([A-Za-z0-9_-]+):\s*$", raw)
        if m:
            cur = m.group(1)
            jobs[cur] = None
            continue
        m = re.match(r"^ {4}name:\s*(\S.*?)\s*$", raw)
        if m and cur is not None and jobs[cur] is None:
            jobs[cur] = m.group(1)
    return jobs


def build_matchers(ci_jobs: dict[str, str | None]) -> tuple[dict[str, str], list[tuple[str, str]]]:
    """把 run 里的 job **显示名**映射回 ci.yml 的 job **id**。

    run 的 `name` 是显示名 ✓：有 `name:` 的用 `name:`，没有的用 id ✓；
    matrix job 会在后面接 ` (...)` ✓（如 `test (sokonanoda-front, lib)`、`ledger (2)`）。
    ⚠ **必须先精确匹配再前缀匹配** ✗✓：`e2e` 的前缀 `e2e (` 会**吞掉**
    `e2e (macos-latest · VS Code 1.138.0)` ✗（那其实是 `e2e-macos` ✓，它有显式 name ✓）。
    """
    exact: dict[str, str] = {}
    for jid, nm in ci_jobs.items():
        if nm:
            exact[nm] = jid
    for jid in ci_jobs:
        exact.setdefault(jid, jid)
    prefix: list[tuple[str, str]] = []
    for jid, nm in ci_jobs.items():
        prefix.append((jid + " (", jid))
        if nm:
            prefix.append((nm + " (", jid))
    return exact, prefix


def classify(name: str, exact: dict[str, str], prefix: list[tuple[str, str]]) -> str | None:
    if name in exact:
        return exact[name]
    for pat, jid in prefix:
        if name.startswith(pat):
            return jid
    return None


# ── 显示名被 API 截断（2026-10-16，洞 `CI-GREEN-READOUT-ROTTED`）──────────────
#
# **实测**（run `38884242478`，`gh api …/runs/<id>/jobs`，**不是** gh 自己截的 ✗）：
#   ci.yml 里 `wiring-lint` 的名字 = 66 字符 / **118 字节**
#   回来的是 `wiring-lint（harness 接线文档漂移 + 启动器扩展链接 + LSP 槽位 marker 跨接缝 + ...`
#     = 62 字符 / **100 字节** ⇒ **API 按字节截断到 100**（97 字节正文 + `...`）✗
# ⇒ `classify()` 的精确/前缀两条路**都**认不回它 ✗ ⇒ `unclassified` 非空 ⇒ **exit 3** ✗
# ⇒ 量具**永远**判不了（就算把 `wiring-lint` 加进分类表也没用 ✗）。
#
# ⚠ **这不是放宽** ✗：只对**以 `...` 结尾**的名字生效 ✓、且必须**唯一**命中某个
# `ci.yml` 名字的前缀 ✓ —— **歧义 / 认不回 ⇒ 照旧 `unclassified` ⇒ exit 3** ✓
# （`ci.yml` 里改个名字就再也认不回 ⇒ 守卫照样咬得住 ✓）。
TRUNCATION_MARK = "..."


def match_truncated(name: str, ci_jobs: dict[str, str | None]) -> str | None:
    """API 截断过的显示名 ⇒ 按**唯一前缀**认回 job id（否则 `None` ⇒ 上层判 exit 3 ✓）。"""
    if not name.endswith(TRUNCATION_MARK):
        return None
    stem = name[: -len(TRUNCATION_MARK)].rstrip()
    if not stem:
        return None
    hits = sorted(
        {jid for jid, nm in ci_jobs.items() if (nm or "").startswith(stem) or jid.startswith(stem)}
    )
    return hits[0] if len(hits) == 1 else None


def judge(
    jobs: list[tuple[str, str]],
    ci_jobs: dict[str, str | None],
    is_main_push: bool,
) -> dict:
    """核心判定 —— **纯函数** ✓（`--selftest` 直接喂合成 job 表，不需要 gh ✓）。"""
    exact, prefix = build_matchers(ci_jobs)

    by_id: dict[str, list[str]] = {}
    unclassified: list[str] = []
    name_truncations: dict[str, str] = {}   # 被截断但**认回**了 ⇒ 印出来（不静默 ✓）
    ambiguous_truncations: list[str] = []   # 被截断且**歧义** ⇒ 拒绝判定 ✓
    for name, concl in jobs:
        jid = classify(name, exact, prefix)
        if jid is None and name.endswith(TRUNCATION_MARK):
            jid = match_truncated(name, ci_jobs)
            if jid is not None:
                name_truncations[name] = jid
            else:
                ambiguous_truncations.append(name)
        if jid is None:
            unclassified.append(name)
            continue
        by_id.setdefault(jid, []).append(concl)

    unknown_jobs = sorted(set(ci_jobs) - CLASSIFIED)

    failures = sorted(n for n, c in jobs if c in BAD)
    skipped = sorted(n for n, c in jobs if c == "skipped")
    succeeded = sorted(n for n, c in jobs if c == "success")

    heavy_required = list(HEAVY) + (list(HEAVY_MAIN_ONLY) if is_main_push else [])

    def ran_ok(jid: str) -> bool:
        cs = by_id.get(jid)
        return bool(cs) and all(c == "success" for c in cs)

    heavy_ok = [j for j in heavy_required if ran_ok(j)]
    heavy_missing = [
        j for j in heavy_required if j not in heavy_ok and j not in by_id
    ]
    heavy_skipped = [
        j
        for j in heavy_required
        if j not in heavy_ok and j in by_id and all(c == "skipped" for c in by_id[j])
    ]
    heavy_other = [
        j for j in heavy_required if j not in heavy_ok and j not in heavy_missing and j not in heavy_skipped
    ]
    always_skipped = [j for j in ALWAYS if j in by_id and all(c == "skipped" for c in by_id[j])]

    if failures:
        verdict, code = "red", EXIT_RED
    elif unknown_jobs or unclassified:
        verdict, code = "not-judged", EXIT_NOT_JUDGED
    elif heavy_skipped or heavy_missing or heavy_other or always_skipped:
        verdict, code = "fake-green", EXIT_FAKE_GREEN
    else:
        verdict, code = "true-green", EXIT_TRUE_GREEN

    return {
        "verdict": verdict,
        "exit_code": code,
        "counts": {
            "success": len(succeeded),
            "skipped": len(skipped),
            "failure": len(failures),
            "total": len(jobs),
        },
        "succeeded": succeeded,
        "skipped": skipped,
        "failures": failures,
        "heavy_required": heavy_required,
        "heavy_ok": heavy_ok,
        "heavy_skipped": heavy_skipped,
        "heavy_missing": heavy_missing,
        "heavy_other": heavy_other,
        "always_skipped": always_skipped,
        "unknown_jobs": unknown_jobs,
        "unclassified_job_names": sorted(unclassified),
        "name_truncations": dict(sorted(name_truncations.items())),
        "ambiguous_truncations": sorted(ambiguous_truncations),
    }


# ── 取真轮次（唯一需要 gh / 网络的地方 ✓）────────────────────────────────────


def gh_json(args: list[str]) -> object:
    try:
        out = subprocess.run(
            ["gh", *args], capture_output=True, text=True, check=True
        ).stdout
    except FileNotFoundError:
        bail("✗ 找不到 `gh`（GitHub CLI）—— 判定真绿需要它 ✗（exit 3）")
    except subprocess.CalledProcessError as e:
        bail(f"✗ `gh {' '.join(args)}` 失败 ✗：{e.stderr.strip()[:400]}（exit 3）")
    return json.loads(out)


RUN_ID_RE = re.compile(r"^\d{6,}$")


def looks_like_run_id(arg: str) -> bool:
    """run id 是**纯数字**（11 位左右）；sha 含 `a`–`f` 或有别的形状 ✓。"""
    return bool(RUN_ID_RE.match(arg))


def resolve_run_arg(arg: str) -> str:
    """`--run` 收 **run id** 或 **commit sha**（2026-10-16 新增 ✓）。

    ⚠ **为什么**（洞 `CI-GREEN-READOUT-ROTTED`）：编排者手上是**推送的那个 sha**、
    不是 run id ⇒ 实测 `--run 4adae13e` 得到 `gh` **404** ✗（而 404 曾因
    `sys.exit("<字符串>")` 退 **1** ⇒ 看起来像"CI 红" ✗✗）。
    ⚠ `gh run list --commit` **只认全 sha**（实测短 sha 回 `[]` ✗）⇒ 先用
    `git rev-parse` 展开 ✓（**过 `git_env.clean_git_env()`** ✓ —— 同
    `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO` 的修法：`GIT_DIR` 优先于 `cwd` ✗）。
    解析结果印到 **stderr** ⇒ `--json` 的 stdout 保持干净 ✓。
    """
    if looks_like_run_id(arg):
        return arg
    full = subprocess.run(
        ["git", "rev-parse", "--verify", f"{arg}^{{commit}}"],
        capture_output=True, text=True, env=git_env.clean_git_env(),
    )
    if full.returncode != 0:
        bail(
            f"✗ `--run {arg}` 既不是 run id（纯数字）也不是本仓能解析的 commit ✗："
            f"{full.stderr.strip()[:200]}（exit 3）"
        )
    sha = full.stdout.strip()
    runs = gh_json(
        ["run", "list", "--workflow", "ci", "--commit", sha, "--limit", "5",
         "--json", "databaseId,createdAt,status,conclusion"]
    )
    if not runs:
        bail(f"✗ commit {arg}（{sha[:7]}）在 ci workflow 下**没有任何轮次** ✗（exit 3）")
    runs.sort(key=lambda r: str(r.get("createdAt") or ""), reverse=True)
    print(f"（--run {arg} ⇒ commit {sha[:7]} ⇒ run {runs[0]['databaseId']}）", file=sys.stderr)
    return str(runs[0]["databaseId"])


def fetch_run(run_id: str | None) -> dict:
    if run_id is None:
        runs = gh_json(
            ["run", "list", "--workflow", "ci", "--limit", "1",
             "--json", "databaseId"]
        )
        if not runs:
            bail("✗ 取不到任何 ci 轮次 ✗（exit 3）")
        run_id = str(runs[0]["databaseId"])
    else:
        run_id = resolve_run_arg(str(run_id))
    return gh_json(
        ["run", "view", str(run_id),
         "--json", "databaseId,headSha,status,conclusion,event,headBranch,displayTitle,jobs"]
    )


def render(run: dict, res: dict) -> None:
    jobs = run.get("jobs", [])
    print(
        f"run {run['databaseId']}  sha={str(run.get('headSha'))[:7]}  "
        f"event={run.get('event')}  branch={run.get('headBranch')}  "
        f"整轮={run.get('status')}/{run.get('conclusion') or '-'}"
    )
    print(f"  {str(run.get('displayTitle') or '')[:70]}")
    print()
    c = res["counts"]
    print("  逐 job 结论（⚠ skipped **不算绿** ✗ —— 它不构成证据 ✓）：")
    print(f"    ✅ success  {c['success']:>2} 个")
    for n in res["succeeded"]:
        print(f"         · {n}")
    print(f"    ⏭  skipped  {c['skipped']:>2} 个  ← **不是绿** ✗")
    for n in res["skipped"]:
        print(f"         · {n}")
    print(f"    ❌ failure  {c['failure']:>2} 个")
    for n in res["failures"]:
        print(f"         · {n}")
    print()
    req, ok = res["heavy_required"], res["heavy_ok"]
    print(f"  重活（验证承载 job）：应有 {len(req)} 个，实跑且 success {len(ok)} 个")
    if res["heavy_skipped"]:
        print(f"    ⏭ 被 skip：{' · '.join(res['heavy_skipped'])}")
    if res["heavy_missing"]:
        print(f"    ✗ 轮次里根本没有：{' · '.join(res['heavy_missing'])}")
    if res["heavy_other"]:
        print(f"    ✗ 非 success 也非全 skip：{' · '.join(res['heavy_other'])}")
    if res["always_skipped"]:
        print(f"    ⚠ 本该每轮都跑却被 skip：{' · '.join(res['always_skipped'])}")
    if res["name_truncations"]:
        # ⚠ **印出来**（不是静默容忍 ✓）—— API 按字节截断到 100 ⇒ 按唯一前缀认回 ✓
        print(f"    ⚠ 显示名被 API 截断、按前缀认回 {len(res['name_truncations'])} 条：")
        for nm, jid in res["name_truncations"].items():
            print(f"         · {nm}  ⇒ {jid}")
    print()

    v = res["verdict"]
    if v == "true-green":
        print(f"  ⇒ ✓ 真绿（exit 0）：重活 {len(ok)}/{len(req)} 实跑且 success，无失败 ✓")
    elif v == "red":
        print(f"  ⇒ ❌ 红（exit 1）：{len(res['failures'])} 个 job 非绿 ✗")
    elif v == "fake-green":
        print(
            "  ⇒ ⚠ **假绿**（exit 2）：**无失败**，但重活被 skipped ✗\n"
            "     ⇒ 这一轮**没有验证任何东西** ✗，**不能**当作「CI 绿」的证据 ✗。\n"
            "     成因通常是纯 docs 推送（`changes` 判定 rust/courses/editor 都没变 ✓）；\n"
            "     发版 bump 的推送会同时动 rust + editor ⇒ 那时才会有重活 ✓。"
        )
    else:
        if res["unknown_jobs"]:
            print(
                f"  ⇒ ✗ 拒绝判定（exit 3）：ci.yml 里有 **未分类** 的 job ✗ "
                f"{res['unknown_jobs']}\n"
                "     ⇒ 守卫会**静默漏掉**它们 ✗。请更新本脚本的分类表"
                "（HEAVY / HEAVY_MAIN_ONLY / ALWAYS / CONDITIONAL）✓。"
            )
        if res["unclassified_job_names"]:
            print(
                f"  ⇒ ✗ 拒绝判定（exit 3）：轮次里有**认不出**的 job 显示名 ✗ "
                f"{res['unclassified_job_names']}"
            )
        if res["ambiguous_truncations"]:
            print(
                f"  ⇒ ✗ 拒绝判定（exit 3）：这些名字被 API 截断**且前缀有歧义** ⇒ "
                f"不猜 ✗ {res['ambiguous_truncations']}\n"
                "     ⇒ 请把 ci.yml 里那条 job 的 `name:` 缩短到 100 字节以内 ✓。"
            )


# ── 判据通道自检（离线 ✓，不需要 gh / 网络 ✓）──────────────────────────────


def _bail_exit_code() -> object:
    """前提缺失时 `gh_json` 的**退出码**（离线 ✓：把 `subprocess.run` 打成 `FileNotFoundError`）。

    ⚠ **为什么值得一条断言**：`sys.exit("<字符串>")` 的退出码**恒为 1** ✗，而 `1` 在本
    脚本里 = **红** ✗ ⇒ 一个手滑就又把"gh 缺失 / run 不存在"印成"CI 红" ✗
    （2026-10-16 实测踩到过：`--run 4adae13e` ⇒ 404 ⇒ rc=1 ✗）。
    """
    import unittest.mock as mock

    with mock.patch.object(subprocess, "run", side_effect=FileNotFoundError):
        try:
            gh_json(["run", "list"])
        except SystemExit as e:  # noqa: PERF203
            return e.code
    return None


def selftest() -> int:
    """**故意喂已知形状** ✓：判据通道自己坏了必须在这里判红 ✓。"""
    ci = parse_ci_jobs()
    if not ci:
        print("✗ selftest：解析不出 ci.yml 的 job ✗")
        return 1

    # 真绿夹具的形状：0.73.0 发版轮（run 36285752786）
    true_green = [
        ("changes", "success"),
        ("lint-fmt（格式，不编译）", "success"),
        ("lint-clippy（clippy 全量 check）", "success"),
        ("docs-lint（文档预算 + 归档索引）", "success"),
        ("status-lint（STATUS.md 瘦身）", "success"),
        ("test (sokonanoda-front, lib)", "success"),
        ("test (sokonanoda-lsp, tests)", "success"),
        ("perf-gate（性能 smoke，只报不拦；计数守卫在 gates-fast 判红）", "success"),
        ("gates-fast（课程语料 + 契约，debug 路径）", "success"),
        ("gates-course（release 构建 + 课程门禁）", "success"),
        ("ledger (1)", "success"),
        ("contract", "success"),
        ("editor", "success"),
        ("e2e (ubuntu-24.04 · VS Code 1.106.0)", "success"),
        ("e2e (macos-latest · VS Code 1.138.0)", "success"),
        ("e2e ledger (commit back on main)", "success"),
        ("auto-tag", "success"),
        ("fast-fail", "skipped"),
    ]
    # 假绿夹具的形状：纯 docs 推送（run 36291907128）
    fake_green = [
        ("changes", "success"),
        ("lint-fmt（格式，不编译）", "success"),
        ("lint-clippy（clippy 全量 check）", "success"),
        ("docs-lint（文档预算 + 归档索引）", "success"),
        ("status-lint（STATUS.md 瘦身）", "success"),
        ("test (sokonanoda-front, lib)", "skipped"),
        ("perf-gate（性能 smoke，只报不拦；计数守卫在 gates-fast 判红）", "skipped"),
        ("gates-fast（课程语料 + 契约，debug 路径）", "skipped"),
        ("gates-course（release 构建 + 课程门禁）", "skipped"),
        ("ledger (1)", "skipped"),
        ("contract", "skipped"),
        ("editor", "skipped"),
        ("e2e (ubuntu-24.04 · VS Code 1.106.0)", "skipped"),
        ("e2e (macos-latest · VS Code 1.138.0)", "skipped"),
        ("e2e ledger (commit back on main)", "skipped"),
        ("auto-tag", "skipped"),
        ("fast-fail", "skipped"),
    ]
    red = [("changes", "success"), ("test (sokonanoda-front, lib)", "failure")] + true_green[1:]
    # PR 上 e2e-ledger 被 skip 是**设计如此** ✓ ⇒ 仍应判真绿 ✓
    pr = [j for j in true_green if j[0] != "e2e ledger (commit back on main)"] + [
        ("e2e ledger (commit back on main)", "skipped")
    ]

    # ── 2026-10-16 新增（洞 `CI-GREEN-READOUT-ROTTED`）──────────────────────
    # **已存在的坏输入** = `4adae13e`（main）那一刻的 `ci.yml` + 那份分类表：
    # **4 个 job 未分类** + **1 个显示名被 API 截断** ⇒ 本 `--selftest` **在 main 上
    # 本来就是红的**（实测 rc=1 · 3/6 用例不符 ✗），而**没有任何 job 跑它** ✗。
    # ⇒ 下面这些用例把三处腐烂**钉死** ✓（⚠ 一条判据都没放宽 ✗：`skipped 不算绿`
    # 那条强度层逐字未动，新增的 ALWAYS 分类只会让它**更严** ✓）。
    wiring_full = ci.get("wiring-lint") or ""
    wiring_trunc = wiring_full.encode("utf-8")[:97].decode("utf-8", errors="ignore").rstrip() + "..."
    # 只有"名字真的长到会被 API 截断"时，截断那两条用例才有前提 ✓（否则如实说明、不假装 ✓）
    trunc_shape = (
        wiring_trunc.endswith("...")
        and len(wiring_trunc) < len(wiring_full)
        and wiring_full.startswith(wiring_trunc[: -len(TRUNCATION_MARK)].rstrip())
    )

    def with_concl(jobs: list, name: str, concl: str) -> list:
        return [(n, concl if n == name else c) for n, c in jobs]

    lint_now = [
        ("tactic-docs-lint（tactic 清单三处一致 + selftest）", "success"),
        (wiring_trunc if trunc_shape else wiring_full, "success"),
        ("board（队列 + 多会话 + 计划一致性；不设 if ⇒ 每推必跑）", "success"),
    ]
    queue_name = lint_now[2][0]
    # **当前形状**：真历史夹具 + 今天新增的三个"每推必跑" lint job + `mark-verified`
    # 按设计 skip（main push 上 rust/courses/editor 没变时它就是 skip ✓ ⇒ 不误报 ✓）
    now = true_green + lint_now + [
        ("mark-verified（重活真跑过且全绿 ⇒ 推进「已验证点」标记）", "skipped"),
    ]

    cases: list[tuple[str, list, bool, str, int]] = [
        ("真绿（main push，重活全跑）", true_green, True, "true-green", 0),
        ("**假绿**（纯 docs，重活全 skip）", fake_green, True, "fake-green", 2),
        ("红（重活里一个 failure）", red, True, "red", 1),
        ("红优先于假绿（failure 与 skip 并存）", fake_green + [("test (x, lib)", "failure")], True, "red", 1),
        ("真绿（PR：e2e-ledger 按设计 skip）", pr, False, "true-green", 0),
        # ⭐ 当前形状：分类表补全 + 截断名认回 + `mark-verified` 按设计 skip
        ("真绿（当前形状：三个 lint job + 截断名 + mark-verified skip）",
         now, True, "true-green", 0),
        # ⭐⭐ **反向验证（用户写死的形状）**：ALWAYS 类 job 被 skip ⇒ **必须不判绿** ✗
        ("**ALWAYS 被 skip（board）⇒ 不判绿**",
         with_concl(now, queue_name, "skipped"), True, "fake-green", 2),
        # ⭐ failure ⇒ 必须判红（用户写死的形状）
        ("**failure（board）⇒ 判红**",
         with_concl(now, queue_name, "failure"), True, "red", 1),
    ]
    if trunc_shape:
        cases += [
            # ⭐ **截断不是逃生门**：认回的那个 job 被 skip ⇒ 照样判假绿 ✓
            ("**截断名被 skip ⇒ 照样不判绿（非逃生门）**",
             with_concl(now, wiring_trunc, "skipped"), True, "fake-green", 2),
        ]

    bad = 0
    for label, jobs, is_main, want_v, want_code in cases:
        got = judge(jobs, ci, is_main)
        okv = got["verdict"] == want_v and got["exit_code"] == want_code
        print(
            f"  {'✓' if okv else '✗'} {label:<34} → {got['verdict']:<12}"
            f"(exit {got['exit_code']})  期望 {want_v} (exit {want_code})"
        )
        if not okv:
            bad += 1

    # 棘轮自检：ci.yml 里塞一个未分类的 job id ⇒ 必须拒绝判定（exit 3）
    ci2 = dict(ci)
    ci2["brand-new-heavy-job"] = None
    got = judge(true_green, ci2, True)
    okv = got["exit_code"] == EXIT_NOT_JUDGED
    print(
        f"  {'✓' if okv else '✗'} {'棘轮：ci.yml 新增未分类 job':<34} → "
        f"{got['verdict']} (exit {got['exit_code']})  期望 not-judged (exit 3)"
    )
    if not okv:
        bad += 1

    # ── 纯函数单元断言（不依赖 `ci.yml` 形状 ⇒ **永不空转** ✓）──────────────
    unit: list[tuple[str, bool]] = [
        # 截断**歧义** ⇒ **不许猜** ⇒ None（上层 ⇒ unclassified ⇒ exit 3）✓
        ("截断歧义 ⇒ 认不回（不猜）",
         match_truncated("job-x（前缀 + ...",
                         {"a": "job-x（前缀 + 尾巴甲）", "b": "job-x（前缀 + 尾巴乙）"}) is None),
        # 截断**唯一** ⇒ 认回 ✓
        ("截断唯一 ⇒ 认回",
         match_truncated("job-x（前缀 + ...", {"a": "job-x（前缀 + 尾巴甲）"}) == "a"),
        # ⚠ **不以 `...` 结尾就绝不走前缀路** —— 否则就是**放宽** ✗
        ("非截断名不走前缀路（不放宽）",
         match_truncated("job-x（前缀", {"a": "job-x（前缀 + 尾巴）"}) is None),
        # `--run` 两形（sha 不是 run id ⇒ 走解析；否则会 404 成"红" ✗）
        ("--run 认 run id / sha 两形",
         looks_like_run_id("38884242478") and not looks_like_run_id("4adae13e")),
        # ⚠ 前提缺失 ⇒ **exit 3**（不是 `1`=红 ✗）—— 退化成 `sys.exit("<字符串>")` 就恒退 1 ✗
        ("前提缺失 ⇒ exit 3（不是 1=红）", _bail_exit_code() == EXIT_NOT_JUDGED),
    ]
    for label, okv in unit:
        print(f"  {'✓' if okv else '✗'} {label:<34} → {'ok' if okv else '不符'}")
        if not okv:
            bad += 1

    total = len(cases) + 1 + len(unit)
    print()
    if bad:
        print(f"✗ selftest 判红：{bad}/{total} 个用例不符 ✗")
        return 1
    print(f"✓ selftest 全过：{total}/{total} 个用例符合 ✓")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(
        description="「真绿」判定：skipped 不算绿，逐 job 检查结论"
    )
    ap.add_argument("--run", help="指定 CI run id（默认：最新一轮 ci）")
    ap.add_argument("--json", action="store_true", help="输出单个 JSON 对象（给 agent 消费）")
    ap.add_argument("--selftest", action="store_true", help="离线判据通道自检")
    a = ap.parse_args()

    if a.selftest:
        return selftest()

    run = fetch_run(a.run)
    status = run.get("status")
    jobs = [(j.get("name", ""), j.get("conclusion") or j.get("status") or "") for j in run.get("jobs", [])]

    if status != "completed":
        payload = {
            "run_id": run["databaseId"],
            "verdict": "not-judged",
            "exit_code": EXIT_NOT_JUDGED,
            "reason": f"整轮还没跑完（status={status}）⇒ 不能判定",
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2) if a.json
              else f"⏳ run {run['databaseId']} 还没跑完（status={status}）⇒ 不能判定（exit 3）")
        return EXIT_NOT_JUDGED

    is_main_push = run.get("event") == "push" and run.get("headBranch") == "main"
    res = judge(jobs, parse_ci_jobs(), is_main_push)

    if a.json:
        payload = {
            "run_id": run["databaseId"],
            "sha": run.get("headSha"),
            "event": run.get("event"),
            "branch": run.get("headBranch"),
            "conclusion": run.get("conclusion"),
            "is_main_push": is_main_push,
            **res,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        render(run, res)
    return res["exit_code"]


if __name__ == "__main__":
    sys.exit(main())
