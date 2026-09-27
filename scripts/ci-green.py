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
ALWAYS = ["changes", "lint-fmt", "lint-clippy", "docs-lint", "status-lint"]

# 按设计可合法 skip
CONDITIONAL = [
    "auto-tag",    # 只在 main 的 push 上跑（发版动作，不是验证）
    "fast-fail",   # 只在**已经有 failure** 时才跑 ⇒ 真绿时它必然 skipped ✓
]

CLASSIFIED = set(HEAVY) | set(HEAVY_MAIN_ONLY) | set(ALWAYS) | set(CONDITIONAL)

# 非绿结论（GitHub job conclusion 全集里"不是绿"的那些）
BAD = {"failure", "cancelled", "timed_out", "action_required", "stale", "startup_failure"}

EXIT_TRUE_GREEN, EXIT_RED, EXIT_FAKE_GREEN, EXIT_NOT_JUDGED = 0, 1, 2, 3


def parse_ci_jobs(path: Path = CI_YML) -> dict[str, str | None]:
    """从 ci.yml 取出 {job_id: 显式 name 或 None}。

    轻量解析 ✓（不引 pyyaml —— 托管版 python3 缺它会让门禁**假红** ✗）：
    只认 `jobs:` 段里 **2 空格缩进**的 `  <id>:`，以及其下 **4 空格缩进**的 `name:`
    （step 的 name 在 8 空格、`if: >-` 的续行在 6 空格 ⇒ 都不会误命中 ✓）。
    """
    if not path.is_file():
        sys.exit(
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


def judge(
    jobs: list[tuple[str, str]],
    ci_jobs: dict[str, str | None],
    is_main_push: bool,
) -> dict:
    """核心判定 —— **纯函数** ✓（`--selftest` 直接喂合成 job 表，不需要 gh ✓）。"""
    exact, prefix = build_matchers(ci_jobs)

    by_id: dict[str, list[str]] = {}
    unclassified: list[str] = []
    for name, concl in jobs:
        jid = classify(name, exact, prefix)
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
    }


# ── 取真轮次（唯一需要 gh / 网络的地方 ✓）────────────────────────────────────


def gh_json(args: list[str]) -> object:
    try:
        out = subprocess.run(
            ["gh", *args], capture_output=True, text=True, check=True
        ).stdout
    except FileNotFoundError:
        sys.exit("✗ 找不到 `gh`（GitHub CLI）—— 判定真绿需要它 ✗（exit 3）")
    except subprocess.CalledProcessError as e:
        sys.exit(f"✗ `gh {' '.join(args)}` 失败 ✗：{e.stderr.strip()[:400]}（exit 3）")
    return json.loads(out)


def fetch_run(run_id: str | None) -> dict:
    if run_id is None:
        runs = gh_json(
            ["run", "list", "--workflow", "ci", "--limit", "1",
             "--json", "databaseId"]
        )
        if not runs:
            sys.exit("✗ 取不到任何 ci 轮次 ✗（exit 3）")
        run_id = str(runs[0]["databaseId"])
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


# ── 判据通道自检（离线 ✓，不需要 gh / 网络 ✓）──────────────────────────────


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
        ("perf-gate（性能 smoke，第一轮只报不拦）", "success"),
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
        ("perf-gate（性能 smoke，第一轮只报不拦）", "skipped"),
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

    cases: list[tuple[str, list, bool, str, int]] = [
        ("真绿（main push，重活全跑）", true_green, True, "true-green", 0),
        ("**假绿**（纯 docs，重活全 skip）", fake_green, True, "fake-green", 2),
        ("红（重活里一个 failure）", red, True, "red", 1),
        ("红优先于假绿（failure 与 skip 并存）", fake_green + [("test (x, lib)", "failure")], True, "red", 1),
        ("真绿（PR：e2e-ledger 按设计 skip）", pr, False, "true-green", 0),
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

    print()
    if bad:
        print(f"✗ selftest 判红：{bad}/{len(cases) + 1} 个用例不符 ✗")
        return 1
    print(f"✓ selftest 全过：{len(cases) + 1}/{len(cases) + 1} 个用例符合 ✓")
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
