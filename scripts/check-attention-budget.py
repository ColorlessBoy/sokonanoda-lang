#!/usr/bin/env python3
"""**簿记预算**：别让"关于工作的账"吃掉注意力（ratchet，照 `docs-budget.json` 的先例 ✓）。

## 为什么要有这个文件（2026-10-15 · `SUB-orchestrator-attention-structure` 【5】/【5'】）

用户实测（本会话，2026-10-14 起 246 笔提交）：

    ① **队列/CI/基建类提交 = 137/246 = 56%** ✗
    ② 改动行数：**生成物/台账 76,280** · 工具 13,690 · **产品 4,572** · 其它文档 3,942
       ⇒ **生成物/台账 : 产品 ≈ 17 : 1** ✗
    ③ 改动最频繁的前 4 个文件**全是队列簿记**（合计 259 次触达）✗

⇒ 病根不是"不努力"，是**注意力结构**：账本身不断出问题（`ux-queue.py` 54 次）
⇒ 自指的循环 ✗。⇒ 【5'】要的就是"**给簿记设预算并判红**"✓。

## 判据（两条，都只许**减**不许增 ✓）

  B1 **生成物/台账 : 产品** ≤ `RATIO_MAX`
  B2 **簿记类提交占比** ≤ `BOOKKEEPING_SHARE_MAX`

⚠ **窗口是"固定锚点 → HEAD"**（不是滑动窗口 ✗）：滑窗会在"一笔大产品提交老化出窗"时
把比值**推高** ⇒ **假红** ✗。固定锚点下三边都只增 ⇒ 比值只因**新提交**变化 ✓
（锚点见 `WINDOW_SINCE`；批次边界上**手改那一行**往前推，评审可见 ✓）。

⚠ **浅克隆 ⇒ 算不了**：`exit 3` + 人话（**绝不静默判过** ✗）—— 与仓库其它门禁同口径 ✓
（CI 里挂在 `queue-lint`，它本来就 `fetch-depth: 0` ✓）。

## 退出码

  0 = 在预算内 / 1 = 超预算（判红） / 2 = 用法错 / 3 = 环境错（浅克隆之类，不假装判过）

## 用法

  python3 scripts/check-attention-budget.py             # 判红/绿 + 读数
  python3 scripts/check-attention-budget.py --json
  python3 scripts/check-attention-budget.py --recent 50 # 另给一个近窗读数（**只报不判**）
  python3 scripts/check-attention-budget.py --selftest
"""
from __future__ import annotations

import argparse
import collections
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# ── 冻结上限（**只许减不许增** ✗→✓；确需放宽 ⇒ 手改这一行 + 写清为什么，评审可见 ✓）──
# 实测 2026-10-15（窗口 = 2026-10-14 → HEAD，255 笔）：比值 **16.98** · 簿记提交 **56.1%**。
# 上限贴着实测值取 ⇒ **下一笔生成物提交就会顶破**（一笔 `perf(e2e)` 约 +660 行 ⇒
# (77628+660)/4572 = **17.12 > 17.0** ⇒ 当场判红 ✓）—— 这正是要的"咬得住" ✓。
RATIO_MAX = 17.0
BOOKKEEPING_SHARE_MAX = 0.57
# 窗口锚点：用户给读数的那一天（**固定** ⇒ 三边只增 ⇒ 没有"老化出窗"的假红 ✓）。
WINDOW_SINCE = "2026-10-14T00:00:00+08:00"

# ── 分桶（**一处定义** ✓；与用户的读数口径逐条对齐）────────────────────────
# 生成物/台账 = 可**机器重生成**或**纯簿记**的东西：e2e 产物（含每轮日志）· 各本
# `ledger.jsonl` · 队列三件 · perf 台账。
# ⚠ 口径与用户 2026-10-15 的实测对齐（他数出 gen=76,280 · tools=13,690 · product=4,572
#   · docs=3,942 —— 本文件逐桶复算，差 0 行 ✓；`docs/gaps/ledger.jsonl` 算 gen ✓）。
_GEN_PREFIXES = ("docs/e2e/", "docs/perf/", "docs/ux-queue-")
_GEN_SUFFIXES = ("ledger.jsonl",)
_TOOL_PREFIXES = ("scripts/", ".github/", "editor/")
_PRODUCT_PREFIXES = ("crates/", "courses/")

# 簿记类提交（标题命中）—— 与用户 ① 的判据**同一条正则** ✓（他把 137/246 数成 56%）。
_BOOKKEEPING_SUBJECT = re.compile(
    r"queue|ci|workflow|ledger|.*-lint|checker|guard|栅栏|复核")


def bucket(path: str) -> str:
    """改动路径 → 区域。**纯函数** ✓（selftest 直接喂合成路径）。"""
    if path.startswith(_GEN_PREFIXES) or path.endswith(_GEN_SUFFIXES):
        return "gen"
    if path.startswith(_TOOL_PREFIXES):
        return "tools"
    if path.startswith(_PRODUCT_PREFIXES):
        return "product"
    return "docs"


def collect(since: str, until: str | None = None, cwd: Path = ROOT) -> tuple[list, str | None]:
    """`git log --numstat` ⇒ [(added, deleted, path, subject), …]。**只读** ✓。

    返回 (行, 环境错误)。浅克隆 ⇒ 环境错误（**不静默** ✗）。
    """
    shallow = subprocess.run(["git", "rev-parse", "--is-shallow-repository"],
                             cwd=cwd, capture_output=True, text=True)
    if shallow.returncode != 0:
        return [], f"`git rev-parse --is-shallow-repository` rc={shallow.returncode}"
    if shallow.stdout.strip() == "true":
        return [], ("仓库是浅克隆 ⇒ 这个窗口**算不了**（不是判过 ✗）："
                    "`git fetch --unshallow`，或 CI 里给这个 job `fetch-depth: 0` ✓")
    # ⚠ **键必须是 sha，不是标题** ✗：本窗里有 **7 对同标题提交**（实测 2026-10-15：
    # 255 笔提交只有 248 个不同标题）⇒ 拿标题当键会把它们**合并**、分母少 7 ✗
    # （用户 ① 数的是**提交数** 143/255 ✓）。这条 bug 是自检咬出来的形状之一 ✓。
    argv = ["git", "log", "--numstat", f"--since={since}",
            "--format=@@%H|%s"]
    if until:
        argv.insert(3, until)
    out = subprocess.run(argv, cwd=cwd, capture_output=True, text=True)
    if out.returncode != 0:
        return [], f"`git log` rc={out.returncode}：{out.stderr.strip()[:200]}"
    rows: list = []
    subject = None
    seen_subject = False
    for line in out.stdout.splitlines():
        if line.startswith("@@"):
            # ⭐ **每笔提交都要进分母**（哪怕它只有二进制改动 ⇒ 一行 numstat 都没有 ✗）：
            # 用户 ① 数的是**提交分类**（143/255 = 56.1%），不是"有文本改动的提交" ✗。
            # ⇒ 没行数的提交发一条**哨兵行**（`path is None`）⇒ 进 `subjects`、不进任何桶 ✓。
            if subject is not None and not seen_subject:
                rows.append((0, 0, None, subject))
            subject, seen_subject = line[2:], False        # `sha|标题` ✓
            continue
        if not line.strip():
            continue
        parts = line.split("\t", 2)
        if len(parts) != 3:
            continue
        added, deleted, path = parts
        if added == "-":                      # 二进制：不计行数，但**这笔提交仍算数** ✓
            continue
        rows.append((int(added), int(deleted), path, subject))
        seen_subject = True
    if subject is not None and not seen_subject:
        rows.append((0, 0, None, subject))
    return rows, None


def aggregate(rows: list) -> dict:
    """行 → 读数。**纯函数** ✓。"""
    lines = collections.Counter()
    touches = collections.Counter()
    subjects: dict[str, str] = {}
    for added, deleted, path, subject in rows:
        subjects[subject] = subject           # `sha|标题` ⇒ 同标题不合并 ✓
        if path is None:                      # 哨兵行：只贡献分母 ✓
            continue
        b = bucket(path)
        lines[b] += added + deleted
        touches[b] += 1
    total = sum(lines.values())
    book = [k for k, title in subjects.items() if _BOOKKEEPING_SUBJECT.search(title)]
    product = lines["product"]
    return {
        "lines": dict(lines),
        "touches": dict(touches),
        "total_lines": total,
        "commits": len(subjects),
        "bookkeeping_commits": len(book),
        "bookkeeping_share": (len(book) / len(subjects)) if subjects else 0.0,
        "ratio": (lines["gen"] / product) if product else float("inf"),
        "top_files": _top_files(rows),
    }


def _top_files(rows: list, n: int = 4) -> list:
    per = collections.Counter()
    for added, deleted, path, _ in rows:
        if path is None:
            continue
        per[path] += added + deleted
    return [[p, v] for p, v in per.most_common(n)]


def judge(agg: dict, ratio_max: float = RATIO_MAX,
          share_max: float = BOOKKEEPING_SHARE_MAX) -> list[str]:
    """**纯函数**：读数 → 判红列表 ✓（selftest 直接喂合成读数）。"""
    red = []
    if agg["ratio"] > ratio_max:
        red.append(
            f"B1 **生成物/台账 : 产品 = {agg['ratio']:.2f} : 1 > 上限 {ratio_max}** ⇒ "
            f"这一窗的改动又在往「账」上堆 ✗（生成物 {agg['lines'].get('gen', 0)} 行 vs "
            f"产品 {agg['lines'].get('product', 0)} 行）。出路：**做产品**，或让生成物别进 git ✓")
    if agg["bookkeeping_share"] > share_max:
        red.append(
            f"B2 **簿记类提交占比 = {agg['bookkeeping_share'] * 100:.1f}% > 上限 "
            f"{share_max * 100:.0f}%** ⇒ {agg['bookkeeping_commits']}/{agg['commits']} 笔"
            f"在维护「关于工作的账」✗")
    return red


def report(agg: dict) -> None:
    lines = agg["lines"]
    print(f"簿记预算（窗口 = {WINDOW_SINCE} → HEAD）：{agg['commits']} 笔提交 · "
          f"{agg['total_lines']} 行改动")
    for key, label in (("gen", "生成物/台账"), ("tools", "工具"),
                       ("product", "产品"), ("docs", "其它文档")):
        n = lines.get(key, 0)
        pct = (100 * n / agg["total_lines"]) if agg["total_lines"] else 0.0
        print(f"  {label:10s} {n:7d} 行  {pct:5.1f}%")
    print(f"  ⇒ **生成物/台账 : 产品 = {agg['ratio']:.2f} : 1**（上限 {RATIO_MAX}）")
    print(f"  ⇒ 簿记类提交 {agg['bookkeeping_commits']}/{agg['commits']} = "
          f"{agg['bookkeeping_share'] * 100:.1f}%（上限 {BOOKKEEPING_SHARE_MAX * 100:.0f}%）")
    print("  触达最多的 4 个文件：" + " · ".join(f"{p} {v}" for p, v in agg["top_files"]))


def recent(n: int) -> tuple[dict, str | None]:
    """近窗读数（**只报不判** —— 滑窗会老化 ⇒ 拿它判红就是假红 ✗）✓。"""
    out = subprocess.run(["git", "log", "--numstat", f"-{n}",
                          "--format=@@%H|%s"], cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        return {}, f"`git log -{n}` rc={out.returncode}"
    rows, subject = [], ""
    for line in out.stdout.splitlines():
        if line.startswith("@@"):
            subject = line[2:]
            continue
        parts = line.split("\t", 2)
        if len(parts) == 3 and parts[0] != "-":
            rows.append((int(parts[0]), int(parts[1]), parts[2], subject))
    return aggregate(rows), None


# ── 判据通道自检（反向验证：故意造出该现象 ⇒ 必须判红）──────────────────────
def selftest() -> int:
    bad = 0

    def case(name: str, rows: list, want_red: bool, expect: str = "",
             ratio_max: float = RATIO_MAX, share_max: float = BOOKKEEPING_SHARE_MAX) -> None:
        nonlocal bad
        agg = aggregate(rows)
        red = judge(agg, ratio_max, share_max)
        hit = bool(red) == want_red
        if not hit:
            bad += 1
        print(f"  {'✓' if hit else '✗'} {name}：判红 {len(red)} 条"
              + (f"（{red[0][:64]}）" if red else "")
              + ("" if hit else f"  ← 期望 {'红' if want_red else '绿'}"))
        if expect and not any(expect in r for r in red):
            bad += 1
            print(f"      ✗ 没有期望的那条判红（找 {expect!r}）")

    _seq = [0]

    def row(path, n, subject="feat(x): 做产品"):
        _seq[0] += 1
        return (n, 0, path, f"sha{_seq[0]:04d}|{subject}")

    print("判据通道自检（合成读数；每组都带**正对照** ✓）：")
    # 正对照：产品为主 ⇒ 必须绿
    case("正对照（产品为主）", [row("crates/front/src/a.rs", 5000),
                          row("docs/e2e/logs/x.log", 500)],
         want_red=False, ratio_max=17.0)
    # B1：生成物压倒产品（**17:1 那个形状**）
    case("B1 生成物:产品 超上限", [row("crates/front/src/a.rs", 1000),
                            row("docs/e2e/logs/x.log", 20000)],
         want_red=True, expect="B1", ratio_max=17.0)
    # B1 边界：正好等于上限 ⇒ **不判红**（≤ 是闭区间 ✓）
    case("B1 正对照：正好等于上限", [row("crates/front/src/a.rs", 1000),
                              row("docs/e2e/logs/x.log", 17000)],
         want_red=False, ratio_max=17.0)
    # B1：产品为 0 ⇒ inf（不许除零崩、必须判红 ✓）
    case("B1 正对照：产品为 0 行", [row("docs/e2e/logs/x.log", 10)],
         want_red=True, expect="B1")
    # B2：簿记提交占比超上限
    case("B2 簿记提交占比超上限",
         [row("scripts/ux-queue.py", 10, "chore(queue): 收口 X"),
          row("crates/front/src/a.rs", 10, "feat(front): 真干活")],
         want_red=True, expect="B2", ratio_max=1e9, share_max=0.4)
    # B2 正对照：簿记占比在限内 ⇒ 绿
    case("B2 正对照：簿记占比在限内",
         [row("scripts/ux-queue.py", 10, "chore(queue): 收口 X"),
          row("crates/front/src/a.rs", 10, "feat(front): 真干活"),
          row("crates/front/src/b.rs", 10, "feat(front): 再干活")],
         want_red=False, ratio_max=1e9, share_max=0.4)
    # 分桶本身：每个桶各一格（口径回归 —— 改口径就会在这里红 ✓）
    cases = [("crates/x.rs", "product"), ("courses/x.md", "product"),
             ("scripts/gap.py", "tools"), (".github/workflows/ci.yml", "tools"),
             ("editor/vscode/server.js", "tools"), ("docs/e2e/logs/a.log", "gen"),
             ("docs/e2e/ledger.jsonl", "gen"), ("docs/gaps/ledger.jsonl", "gen"),
             ("docs/perf/ledger.jsonl", "gen"), ("docs/ux-queue-events.jsonl", "gen"),
             ("docs/ONBOARDING.md", "docs"), ("AGENTS.md", "docs")]
    wrong = [(p, bucket(p), want) for p, want in cases if bucket(p) != want]
    if wrong:
        bad += 1
        print(f"  ✗ 分桶口径漂了：{wrong}")
    else:
        print(f"  ✓ 分桶口径：{len(cases)} 格全对（gen/tools/product/docs）")
    print(f"\n自检：{'✓ 全过' if bad == 0 else f'✗ {bad} 组不对'}"
          f"（B1×3 + 正对照×2 · B2×1 + 正对照×1 · 分桶 {len(cases)} 格）")
    return 0 if bad == 0 else 1


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="簿记预算（ratchet）")
    ap.add_argument("--since", default=WINDOW_SINCE)
    ap.add_argument("--until", default=None, help="窗口右端（默认 HEAD；反向验证用）")
    ap.add_argument("--ratio-max", type=float, default=RATIO_MAX)
    ap.add_argument("--share-max", type=float, default=BOOKKEEPING_SHARE_MAX)
    ap.add_argument("--recent", type=int, default=0, metavar="N",
                    help="另给一个近 N 笔的读数（**只报不判** ✓）")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args(argv)

    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, ValueError, OSError):
            pass

    if args.selftest:
        return selftest()

    rows, err = collect(args.since, args.until)
    if err:
        print(f"error: {err}", file=sys.stderr)
        return 3
    agg = aggregate(rows)
    red = judge(agg, args.ratio_max, args.share_max)

    payload = {"reading": agg, "red": red, "since": args.since, "until": args.until,
               "caps": {"ratio_max": args.ratio_max, "share_max": args.share_max}}
    if args.recent:
        r, rerr = recent(args.recent)
        payload["recent"] = r if not rerr else {"error": rerr}

    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        report(agg)
        if args.recent and "error" not in payload.get("recent", {}):
            r = payload["recent"]
            print(f"  近 {args.recent} 笔（**只报不判** ✓）：比值 {r['ratio']:.2f} : 1 · "
                  f"簿记提交 {r['bookkeeping_share'] * 100:.1f}%")
        if red:
            print(f"\n✗ 簿记预算 **{len(red)} 条红**：", file=sys.stderr)
            for m in red:
                print(f"  · {m}", file=sys.stderr)
            return 1
        print("✓ 簿记预算在限内 ✓")
    return 0 if not red else 1


if __name__ == "__main__":
    sys.exit(main())
