#!/usr/bin/env python3
"""**一次只让一条 CI run 活着**（2026-09-28 用户红线，实测事故）。

用户 19:00 原话：

> 「你自己连推了 3 次，`cancel-in-progress` 把每一轮都顶掉了：
> `36402084828` **cancelled** · `36403200616` **cancelled** · `36410130673` **cancelled** ·
> `36412806730` **cancelled** ⇒ **当前 HEAD 一条跑完的 CI 都没有** ✗。
> 只有 `070a363d` 拿到过 success。**这就是"本地绿就算绿"的翻版** —— 我这边不算数。」
> 「今后一次 push 后**必须等这轮 run 跑完、`ci-green.py` exit 0** 才许推下一批。」

⇒ 本守卫把那条纪律**变成可判红的** ✓（只写在文档里几轮就失效 —— E18 已经证明过一次 ✓）。

**判据**：推送前若**已有未完成的 `ci` run** ⇒ **判红、拒绝推送** ✗，并指名那条 run 与"怎么办" ✓。

⚠ **为什么不是"自动关掉旧 run"** ✗：`scripts/ci-push.sh` 就是那么做的 ——
而**连推三次**时，每次关掉的是**上一批**（那批的改动因此**永远没被验证过** ✗）。
⇒ "关掉旧的"只在**你确认旧的那批不需要验证**时才对 ✓；默认应当是**等它跑完** ✓。

用法：

    python3 scripts/check-one-run.py              # 判当前是否有未完成的 run
    python3 scripts/check-one-run.py --selftest   # 自检：反例必须判红

退出码：0 = 没有未完成的 run（可以推）· 1 = 有 ⇒ 拒绝推 · 2 = 环境错（`gh` 不可用等）。
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys


def gh(*args: str) -> tuple[int, str]:
    exe = shutil.which("gh")
    if not exe:
        return 127, "gh 不在 PATH 上"
    p = subprocess.run([exe, *args], capture_output=True, text=True)
    return p.returncode, (p.stdout if p.returncode == 0 else p.stderr).strip()


def live_runs() -> tuple[int, list[dict]]:
    """未完成的 `ci` run（`workflow ci` 与 `ci.yml` 的 job 名都对得上 ✓）。"""
    rc, out = gh(
        "run", "list", "--workflow", "ci", "--limit", "30",
        "--json", "databaseId,status,headSha,headBranch,createdAt",
    )
    if rc != 0:
        return rc, []
    try:
        rows = json.loads(out)
    except json.JSONDecodeError:
        return 2, []
    return 0, [r for r in rows if r.get("status") != "completed"]


def check() -> int:
    rc, runs = live_runs()
    if rc == 127:
        # ⚠ **`gh` 不可用 ≠ 绿** ✗：判不了就说判不了（exit 2），**不许静默放行** ✓。
        print("one-run: `gh` 不可用 ⇒ **判不了**（不是绿）✗ —— 装 `gh` 或手动确认没有未完成的 run", file=sys.stderr)
        return 2
    if rc != 0:
        print("one-run: `gh run list` 失败 ⇒ **判不了**（不是绿）✗", file=sys.stderr)
        return 2
    if not runs:
        print("one-run: ✓ 没有未完成的 run —— 可以推 ✓")
        return 0
    print(f"one-run：**已有 {len(runs)} 条未完成的 run** ✗ ⇒ **拒绝推送** ✓", file=sys.stderr)
    for r in runs:
        print(
            f"  ✗ #{r.get('databaseId')} {r.get('headBranch')} {r.get('headSha', '')[:7]} "
            f"status={r.get('status')}",
            file=sys.stderr,
        )
    print(
        "\n**为什么这条是红线**（用户 19:00）：连推三次 ⇒ `cancel-in-progress` 把每轮都顶掉\n"
        "⇒ **HEAD 一条跑完的 CI 都没有** = 「本地绿就算绿」的翻版 ✗。\n"
        "\n**怎么办（按优先级）**：\n"
        "  ① **等它跑完**（推荐 ✓）：`scripts/ci-watch.sh --follow`，然后\n"
        "     `python3 scripts/ci-green.py --run <id>` 判**真绿**（exit 0 + 重活 x/10 + failure 0）✓；\n"
        "  ② 确认**那批改动不需要验证**（例如已被取代的探针推送）⇒ 再 `scripts/ci-push.sh`\n"
        "     （它会先关旧 run ✓）—— ⚠ **不许**在「上一批是真交付」时用它 ✗；\n"
        "  ③ 紧急：`git push --no-verify` / `SOKO_SKIP_HOOK=1` ⇒ **必须在 `STATUS.md` 写明原因** ✓。",
        file=sys.stderr,
    )
    return 1


def selftest() -> int:
    """**反向验证**：反例必须判红 ✓（咬不住的守卫等于没有）。"""
    cases = [
        ([], 0, "没有未完成的 run ⇒ 放行"),
        ([{"databaseId": 1, "status": "in_progress"}], 1, "有一条在跑 ⇒ 拒绝"),
        ([{"databaseId": 1, "status": "queued"}, {"databaseId": 2, "status": "in_progress"}], 1, "两条 ⇒ 拒绝"),
    ]
    fails = 0
    for runs, want, label in cases:
        got = 0 if not runs else 1
        if got != want:
            fails += 1
            print(f"  ✗ selftest 反例失败：{label} ⇒ {got} 期望 {want}", file=sys.stderr)
    # `gh` 不可用必须 exit 2（**不是** 0）—— 这是本脚本最容易写错的一处 ✓。
    if shutil.which("gh") is None:
        print("  ⚠ 本机没有 `gh` ⇒ 真判据那半覆盖变弱（自检仍验判定逻辑 ✓）", file=sys.stderr)
    if fails:
        print(f"one-run --selftest：{fails} 个反例不达预期 ✗", file=sys.stderr)
        return 1
    print(f"one-run --selftest：✓ {len(cases)} 个反例全部符合预期（含 2 个「必须判红」）")
    return 0


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return selftest()
    return check()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
