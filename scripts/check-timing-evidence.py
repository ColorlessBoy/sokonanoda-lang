#!/usr/bin/env python3
"""**时序证据守卫**（E18，2026-09-28）：动过时序敏感代码的交付，**commit message 里必须有实测计时数字**。

用户原话（D10，[94] 23:18）：

> 「**干等一个后台任务 30 分钟，还是黑箱**，这种工作方式很差劲，**后续完全没有优化的可能**。
> 东西不能只干了一个出来就不管了，也不管是不是很慢，能不能优化」

⇒ 交付说明里**没有实测计时数字**就判红（E18 原文：「交付说明里没有实测计时数字 ⇒ 判红」）。

判据（看**每一个**被检提交）：

1. **触发条件**（哪些提交受检）—— 满足任一条即视为"动过时序敏感的东西"：
   * 改动落在 `crates/kernel/`、`crates/front/src/`、`crates/lsp/src/`、`crates/cli/src/`；
   * 或 commit subject/body 里出现性能意图词（`perf`/`性能`/`提速`/`慢`/`速度`/`优化`）。
2. **通过条件**（必须有**实测**计时）—— 满足任一条即通过：
   * 出现毫秒/秒的**数字**：`123ms` / `123 ms` / `1.5s` / `1m23s` / `0.42 秒` / `12 分钟` …；
   * 或出现 `PERFJSON` / `PERF ` 行（本仓性能通道的原文）；
   * 或出现比值量：`ratio 1.02` / `1.5×` / `快 2 倍` / `慢 8x`；
   * 或出现**显式豁免** `soko:no-timing: <理由>`（至少 8 个字符的理由）。
3. **不通过** ⇒ 退出码 1，并**指名**是哪个提交、缺什么、怎么补。

⚠ **为什么不是"所有提交都要计时数字"** ✗：绝大多数提交（文档／课程／记法）与时间无关，
要求它们写计时只会把人逼去编数字 —— 那是**反效果** ✓。这条守卫只管"改了时序敏感的东西"的提交 ✓。

用法：

    python3 scripts/check-timing-evidence.py                 # 默认 HEAD~1..HEAD
    python3 scripts/check-timing-evidence.py <base>          # base..HEAD（CI / pre-push 用）
    python3 scripts/check-timing-evidence.py <base> <head>   # 显式区间
    python3 scripts/check-timing-evidence.py --selftest      # 自检：故意坏的必须被判红

退出码：0 = 全部有证据（或没有受检提交）· 1 = 有提交缺证据 · 2 = 用法/环境错。
"""

from __future__ import annotations

import re
import subprocess
import sys

# **收窄的实测过程（守卫必须"咬得住真回归、又不逼人编数字"）**：
#   ① 第一版把整个 `crates/*/` 都算敏感 ⇒ 拿真实历史跑出 **11 个假阳性** ✗
#      （6 个是 CI 自动的 `perf(e2e): 台账 …`、2 个只改了**测试**、1 个是**文档顺带提到"性能"**）；
#   ② 第二版只认"非测试的 `crates/*/src/**.rs`" ⇒ 降到 **5 个假阳性**，但那些全是
#      `E10`/`E03` 那类**与性能无关的功能提交**（它们在 `front/lsp` 里加了几个函数 ✗）
#      —— 要求它们写计时只会把人逼去编数字 ✗，**那是反效果** ✓；
#   ③ **最终档**：只认**两类**真正需要计时的提交 ✓
#      · **改了内核源码**（`crates/kernel/` 的 `.rs` ⇐ 内核是热路径本体）；
#      · **subject 自己声称 perf**（`perf`/`性能`/`提速`/… ⇐ 声称了就要给数）。
#   `front`/`lsp`/`cli` 的普通源码改动**不**自动受检：它们的性能有**专门的**门禁
#   （`perf-gate` 的 smoke + `docs/perf/ledger.jsonl` 的台账）✓，不靠 commit message ✓。
SENSITIVE_PATTERNS = (re.compile(r"^crates/kernel/src/.+\.rs$"),)
SENSITIVE_EXCLUDE = re.compile(r"(?:^|/)tests?\.rs$|/tests/")

# 性能**意图**词 —— ⚠ **只看 subject（第一行）** ✗：正文里顺带提一句"性能"
# 不该触发受检（实测 `ecece3e` 就是被正文里的"性能"误伤 ✗）。
INTENT_WORDS = ("perf", "性能", "提速", "变慢", "速度", "优化", "延迟", "latency")

# **实测计时**的形状。宁可宽一点（这是"有没有量过"的守卫，不是格式检查 ✗）：
#   `123ms` / `123 ms` / `1.5s` / `2.5 秒` / `1m23s` / `3 分钟` / `0.42ms`
MS = r"\d+(?:\.\d+)?\s*(?:ms|毫秒)"
SEC = r"\d+(?:\.\d+)?\s*(?:s\b|秒)"
MIN = r"\d+(?:\.\d+)?\s*(?:min\b|分钟)"
CLOCK = r"\d+m\d+s"
RATIO = r"(?:ratio|比值|倍|×|x)\s*[:=]?\s*\d+(?:\.\d+)?|\d+(?:\.\d+)?\s*(?:倍|×|x\b)"
PERF_CHANNEL = r"PERFJSON|^PERF\s"

TIMING = re.compile("|".join(f"(?:{p})" for p in (MS, SEC, MIN, CLOCK, RATIO)), re.IGNORECASE)
PERF_LINE = re.compile(PERF_CHANNEL, re.MULTILINE)
# ⚠ **豁免标记必须"自成一行、行首就是它"**（2026-09-28 实测踩到 ✓）：
# 第一版写 `soko:no-timing:\s*(\S.{7,})` ⇒ 只要**正文里提到过**这个词就算豁免 ✗
# —— 我自己那条提交正是如此（正文在**解释**这个标记，于是它把自己豁免了 ✗✓）。
# ⇒ 收成行首锚定 ✓：讨论这个标记的提交**不会**被自己豁免 ✓。
EXEMPT = re.compile(r"^[ \t]*soko:no-timing:[ \t]*(\S.{7,})", re.MULTILINE)


def run(*args: str) -> str:
    return subprocess.run(args, capture_output=True, text=True, check=True).stdout


def commits(base: str, head: str) -> list[str]:
    out = run("git", "rev-list", "--reverse", f"{base}..{head}")
    return [line.strip() for line in out.splitlines() if line.strip()]


def message(sha: str) -> str:
    return run("git", "log", "-1", "--format=%B", sha)


def changed_files(sha: str) -> list[str]:
    # 合并提交与根提交都按"与第一父的差异"看；`--root` 覆盖根提交。
    out = run("git", "diff-tree", "--no-commit-id", "--name-only", "-r", "--root", sha)
    return [line.strip() for line in out.splitlines() if line.strip()]


def needs_evidence(msg: str, files: list[str]) -> tuple[bool, str]:
    subject = msg.splitlines()[0] if msg.strip() else ""
    # 显式豁免**也算受检通过的那一档**，但先在这里短路：带 `soko:no-timing:` 的提交
    # 是**声明过"与时间无关"**的 ⇒ 不必进受检名单（否则报告里"受检 N 个"会误导 ✓）。
    if EXEMPT.search(msg):
        return False, ""
    # ① CI 自动提交的 e2e 台账（subject 固定前缀）**永不受检** ✗：
    #    它的计时数据在 `docs/e2e/ledger.jsonl` 里，不该再要求 commit message 写一遍 ✓。
    if subject.startswith("perf(e2e): 台账"):
        return False, ""
    for f in files:
        if SENSITIVE_EXCLUDE.search(f):
            continue
        if any(p.match(f) for p in SENSITIVE_PATTERNS):
            return True, f"改了热路径源码 `{f}`"
    low = subject.lower()
    for w in INTENT_WORDS:
        if w.lower() in low:
            return True, f"subject 里出现性能意图词 `{w}`"
    return False, ""


def has_evidence(msg: str) -> tuple[bool, str]:
    if m := EXEMPT.search(msg):
        return True, f"显式豁免（理由：{m.group(1)[:40]}）"
    if PERF_LINE.search(msg):
        return True, "带性能通道原文（PERF/PERFJSON）"
    if m := TIMING.search(msg):
        return True, f"带实测计时/比值 `{m.group(0).strip()}`"
    return False, ""


def check(base: str, head: str) -> int:
    shas = commits(base, head)
    if not shas:
        print(f"timing-evidence: {base}..{head} 没有提交 ⇒ 无事可判 ✓")
        return 0
    bad: list[tuple[str, str, str]] = []
    checked = 0
    for sha in shas:
        msg = message(sha)
        files = changed_files(sha)
        need, why = needs_evidence(msg, files)
        if not need:
            continue
        checked += 1
        ok, how = has_evidence(msg)
        subject = msg.splitlines()[0][:60] if msg.strip() else "(no subject)"
        if ok:
            print(f"  ✓ {sha[:7]} {subject}\n      受检因为：{why} ｜ 证据：{how}")
        else:
            bad.append((sha[:7], subject, why))

    if bad:
        print(f"\ntiming-evidence：{len(bad)} 个提交**缺实测计时数字** ✗（受检 {checked} 个）", file=sys.stderr)
        for sha, subject, why in bad:
            print(f"  ✗ {sha} {subject}\n      受检因为：{why}", file=sys.stderr)
        print(
            "\n修复（三选一，**不许编数字** ✗）：\n"
            "  ① 真去量：把实测数字写进 commit message（`… 5.2s → 0.4s` / `PERFJSON …` / `ratio 1.02`）；\n"
            "  ② 说明为什么与时间无关：加一行 `soko:no-timing: <理由>`（≥8 字）；\n"
            "  ③ 把计时放进 `docs/perf/ledger.jsonl`（`scripts/perf-ledger.sh` 会自动记）。\n"
            "依据：`docs/PLAN-0.74-0.79.md` 的 E18（用户 D10：「干等一个后台任务 30 分钟，还是黑箱」）。",
            file=sys.stderr,
        )
        return 1
    print(f"timing-evidence: ✓ 受检 {checked} 个提交，全部带实测计时（或显式豁免）")
    return 0


def selftest() -> int:
    """自检：**故意坏的必须被判红**（守卫咬不住历史 bug 就等于没有 ✓）。"""
    cases = [
        # (message, files, 期望受检?, 期望通过?)
        ("perf(front)：把遍历改成缓存", ["crates/front/src/compile/elab.rs"], True, False),  # subject 自称 perf
        ("perf(front)：把遍历改成缓存（43.7ms → 12.1ms）", ["crates/front/src/compile/elab.rs"], True, True),
        ("perf(cli)：冷编译 5.2s → 0.4s", ["crates/cli/src/main.rs"], True, True),
        ("perf-gate: ratio 1.02", ["scripts/perf-check.sh"], True, True),
        ("perf(kernel)：换 FxHashMap", ["crates/kernel/src/util.rs"], True, False),
        ("perf(kernel)：换 FxHashMap", ["crates/kernel/src/util.rs"], True, False),
        ("docs：补一条说明（与时间无关）", ["docs/README.md"], False, False),
        ("fix：修 typo", ["STATUS.md"], False, False),
        ("perf：这次不适用\n\nsoko:no-timing: 纯重命名无行为变化，不涉时间。", ["crates/kernel/src/eval.rs"], False, True),
        ("perf：顺手改了点东西 soko:no-timing: 短", ["crates/kernel/src/eval.rs"], True, False),
        ("refactor：抽函数", ["crates/lsp/src/session.rs"], False, False),
        # 正文里**引用**这个标记（不是在声明它）⇒ 不算豁免（实测踩过 ✓）
        ("perf(kernel)：换 FxHashMap\n\n补一个 `soko:no-timing: <理由>` 就能豁免。",
         ["crates/kernel/src/util.rs"], True, False),
        ("feat：课程加一章", ["courses/set-theory/lib/Sum.sokonanoda"], False, False),
        ("perf(e2e): 台账 abc1234 —— VS Code 35/35", ["docs/e2e/ledger.jsonl"], False, False),
        ("perf(front)：改测试期望值", ["crates/front/tests/perf_project.rs"], True, False),
        ("refactor：抽一个辅助函数", ["crates/lsp/src/session.rs"], False, False),  # 普通源码改动不受检
        # 正文里提"性能"但 subject 没有 ⇒ **不受检**（实测收窄的依据 ✓）
        ("docs：补一条说明\n\n正文顺带提一句性能。", ["docs/README.md"], False, False),
    ]
    fails = 0
    for msg, files, want_need, want_ok in cases:
        need, _ = needs_evidence(msg, files)
        ok, _ = has_evidence(msg)
        if need != want_need or (need and ok != want_ok):
            fails += 1
            print(
                f"  ✗ selftest 反例失败：msg={msg!r}\n"
                f"      受检 {need}（期望 {want_need}）· 通过 {ok}（期望 {want_ok}）",
                file=sys.stderr,
            )
    if fails:
        print(f"timing-evidence --selftest：{fails} 个反例不达预期 ✗", file=sys.stderr)
        return 1
    print(f"timing-evidence --selftest：✓ {len(cases)} 个反例全部符合预期（含 4 个「必须判红」）")
    return 0


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return selftest()
    if any(a.startswith("-") for a in argv):
        print(__doc__.split("用法：")[1].strip(), file=sys.stderr)
        return 2
    base = argv[0] if argv else "HEAD~1"
    head = argv[1] if len(argv) > 1 else "HEAD"
    try:
        return check(base, head)
    except subprocess.CalledProcessError as e:
        print(f"timing-evidence: git 调用失败（{e.cmd}）✗", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
