#!/usr/bin/env python3
"""**性能回归比较器**：读 `docs/perf/ledger.jsonl`，说清"这一版哪一条退化了"。

为什么需要它：`docs/PERF.md` 一直只有**人肉规则**（"优先比 `best_ms`、±25% 内算同档"），
没有任何脚本读台账判退化。而大计划是 100+ 个小环节推进的，"修 A 弄慢 B"必然会发生
——**只有机械判据能拦住它**。

用法：

    python3 scripts/perf-compare.py                 # 最后一条 vs 上一条
    python3 scripts/perf-compare.py --since <sha|版本|-N>
    python3 scripts/perf-compare.py --threshold 25  # 退化多少算红（默认 25%）
    python3 scripts/perf-compare.py --json
    python3 scripts/perf-compare.py --self-test     # 自检判定规则本身

判定规则（四条）：

1. 某个 `*_ms` 指标退化 **> threshold%** ⇒ **红**（"快了没有"的反面）；
2. **新增**的 `(scope, case)` ⇒ 提示"无基线"，**不算红**；
3. **消失**的 `(scope, case)` ⇒ **红**——悄悄删掉一个哨兵是最隐蔽的回归；
4. 两条记录的**宿主或 cli_profile 不同** ⇒ 只提示"不可比"，**不算红**
   （`docs/PERF.md` 的纪律：不同口径的数字不能直接比）。

夹具不同（`modules`/`decls_per_module` 等描述字段不一致）时该 case 标"夹具不同"、
不算红——那说明哨兵本身被改了，值得人看一眼。

退出码：0 = 无退化；1 = 有退化或哨兵消失；2 = 用法/环境错误。
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
LEDGER = ROOT / "docs" / "perf" / "ledger.jsonl"

# 夹具描述字段：用来判"可比不可比"，不参与退化判定。
DESCRIPTORS = {
    "schema",
    "modules",
    "decls_per_module",
    "matches",
    "publishes",
    "publishes_per_keystroke",
    "dependent_diagnostics",
    "ratio_4x",
}


def is_metric(name: str) -> bool:
    """参与退化判定的字段：时间量（`ms` / `*_ms`）。"""
    return name == "ms" or name.endswith("_ms")


def flatten(record: dict) -> dict:
    """把一条 record 展平成 `{指标名: 数值}`（列表按下标展开）。"""
    out: dict[str, float] = {}
    for key, value in record.items():
        if not is_metric(key):
            continue
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            out[key] = float(value)
        elif isinstance(value, list):
            for i, item in enumerate(value):
                if isinstance(item, (int, float)) and not isinstance(item, bool):
                    out[f"{key}[{i}]"] = float(item)
    return out


def descriptors(record: dict) -> dict:
    return {k: v for k, v in record.items() if k in DESCRIPTORS}


def index(entry: dict) -> dict[tuple[str, str], dict]:
    out: dict[tuple[str, str], dict] = {}
    for record in entry.get("records", []):
        out[(record.get("scope", "?"), record.get("case", "?"))] = record
    return out


def comparable(before: dict, after: dict) -> tuple[bool, str]:
    """宿主 / cli_profile / 夹具是否一致。"""
    for key in ("system", "machine"):
        a = (before.get("host") or {}).get(key)
        b = (after.get("host") or {}).get(key)
        if a != b:
            return False, f"宿主不同（{key}: {a} vs {b}）"
    if before.get("cli_profile") != after.get("cli_profile"):
        return False, f"cli_profile 不同（{before.get('cli_profile')} vs {after.get('cli_profile')}）"
    return True, ""


def compare(
    before: dict, after: dict, threshold: float, floor_ms: float = 5.0
) -> tuple[list, list, list]:
    """返回 (rows, regressions, notes)。rows 每项 = dict(scope, case, metric, before, after, pct, note)。"""
    b, a = index(before), index(after)
    rows: list[dict] = []
    regressions: list[dict] = []
    notes: list[str] = []

    host_ok, host_reason = comparable(before, after)
    if not host_ok:
        notes.append(f"两条记录**不可比**：{host_reason}（只提示，不判红）")

    for key in sorted(set(b) | set(a)):
        scope, case = key
        if key not in b:
            rows.append({"scope": scope, "case": case, "metric": "-", "before": None,
                         "after": None, "pct": None, "note": "新增（无基线）"})
            continue
        if key not in a:
            # 宿主不同时连"消失"也不判红：两条记录本来就不是同一口径，
            # 可能只是这一版用 `--no-build` 少跑了套件（计划 §0.6 的规则 ④）。
            if host_ok:
                rows.append({"scope": scope, "case": case, "metric": "-", "before": None,
                             "after": None, "pct": None, "note": "**消失**"})
                regressions.append({"scope": scope, "case": case, "metric": "-",
                                    "before": None, "after": None, "pct": None,
                                    "reason": "哨兵消失（这一版不再记录这个 case）"})
            else:
                rows.append({"scope": scope, "case": case, "metric": "-", "before": None,
                             "after": None, "pct": None, "note": "消失（不可比，不判红）"})
            continue
        if descriptors(b[key]) != descriptors(a[key]):
            rows.append({"scope": scope, "case": case, "metric": "-", "before": None,
                         "after": None, "pct": None, "note": "夹具不同（哨兵被改了？）"})
            continue
        fb, fa = flatten(b[key]), flatten(a[key])
        for metric in sorted(set(fb) & set(fa)):
            base, now = fb[metric], fa[metric]
            if base <= 0:
                continue
            pct = (now - base) / base * 100.0
            row = {"scope": scope, "case": case, "metric": metric,
                   "before": base, "after": now, "pct": pct, "note": ""}
            if not host_ok:
                row["note"] = "不可比"
            elif now < floor_ms:
                # 低于下限的量级：噪声带（真台账上实测过 1.0ms → 3.0ms = "+200%"）。
                row["note"] = f"（<{floor_ms:g}ms，噪声带）"
            elif pct > threshold:
                row["note"] = "**退化**"
                regressions.append({**row, "reason": f"退化 {pct:+.1f}%（阈值 {threshold:.0f}%）"})
            rows.append(row)
    return rows, regressions, notes


def render(rows: list[dict], regressions: list[dict], notes: list[str], before: dict, after: dict) -> None:
    print(
        f"基准：v{before.get('version')} {str(before.get('commit'))[:7]} "
        f"({before.get('date')}, cli={before.get('cli_profile')})"
    )
    print(
        f"对比：v{after.get('version')} {str(after.get('commit'))[:7]} "
        f"({after.get('date')}, cli={after.get('cli_profile')})"
    )
    for note in notes:
        print(f"⚠ {note}")
    print()
    print(f"{'scope':14s} {'case':32s} {'指标':18s} {'基准':>10s} {'这次':>10s} {'变化':>9s}  说明")
    print("-" * 108)
    for row in rows:
        base = f"{row['before']:.1f}" if row["before"] is not None else "-"
        now = f"{row['after']:.1f}" if row["after"] is not None else "-"
        pct = f"{row['pct']:+.1f}%" if row["pct"] is not None else "-"
        print(
            f"{row['scope']:14s} {row['case']:32s} {row['metric']:18s} "
            f"{base:>10s} {now:>10s} {pct:>9s}  {row['note']}"
        )
    print()
    if regressions:
        print(f"判红 {len(regressions)} 条：", file=sys.stderr)
        for r in regressions:
            print(f"  {r['scope']}/{r['case']} · {r['metric']}：{r['reason']}", file=sys.stderr)
        print(
            "先复测一次（性能是噪声敏感的量）；仍然退化就查这一版改了什么"
            "（docs/PERF.md 的判读纪律：先采样口径、再二进制对拍，两步都排除再谈放宽阈值）。",
            file=sys.stderr,
        )
    else:
        print("没有退化，也没有哨兵消失。")


def pick(entries: list[dict], since: str | None) -> tuple[dict, dict]:
    if len(entries) < 2:
        raise SystemExit("error: 台账里少于两条记录，无法对比（先 scripts/perf-ledger.sh）")
    after = entries[-1]
    if since is None:
        return entries[-2], after
    # `-N`：往回数 N 条
    try:
        back = int(since)
    except ValueError:
        back = None
    if back is not None and back < 0:
        idx = len(entries) + back
        if idx < 0:
            raise SystemExit(f"error: --since {since} 超出台账范围（共 {len(entries)} 条）")
        return entries[idx], after
    for entry in reversed(entries[:-1]):
        commit = str(entry.get("commit") or "")
        if commit.startswith(since) or str(entry.get("version")) == since:
            return entry, after
    raise SystemExit(f"error: --since {since} 在台账里找不到（用 sha 前缀、版本号，或 -N）")


def self_test() -> int:
    """自检四条判定规则（用造的台账，不碰真文件）。"""
    base_host = {"system": "Darwin", "machine": "arm64"}
    before = {
        "version": "1.0.0", "commit": "a" * 40, "date": "2026-01-01", "cli_profile": "release",
        "host": base_host,
        "records": [
            {"scope": "s", "case": "grows", "best_ms": 100.0},
            {"scope": "s", "case": "gone", "ms": 50.0},
            {"scope": "s", "case": "stable", "ms": 10.0},
        ],
    }
    after = {
        "version": "1.0.1", "commit": "b" * 40, "date": "2026-01-02", "cli_profile": "release",
        "host": base_host,
        "records": [
            {"scope": "s", "case": "grows", "best_ms": 130.0},   # +30% ⇒ 红
            {"scope": "s", "case": "stable", "ms": 10.0},        # 不变
            {"scope": "s", "case": "fresh", "ms": 7.0},          # 新增 ⇒ 不红
        ],
    }
    cases: list[tuple[str, bool, bool]] = []

    rows, regressions, _ = compare(before, after, 25.0)
    red = {(r["scope"], r["case"], r["metric"]) for r in regressions}
    cases.append(("① 退化 >25% 判红", ("s", "grows", "best_ms") in red, True))
    cases.append(("② 新增 case 不判红", not any(r["case"] == "fresh" for r in regressions), True))
    cases.append(("③ 消失 case 判红", any(r["case"] == "gone" for r in regressions), True))

    other_host = json.loads(json.dumps(after))
    other_host["host"] = {"system": "Linux", "machine": "x86_64"}
    _, regressions2, notes2 = compare(before, other_host, 25.0)
    cases.append(("④ 宿主不同不判红", not regressions2 and bool(notes2), True))

    # ⑤⑥ 用一份"case 集合与基线完全一致"的对照，否则测的是规则③（哨兵消失）
    same_set = json.loads(json.dumps(after))
    same_set["records"] = [
        {"scope": "s", "case": "grows", "best_ms": 125.0},
        {"scope": "s", "case": "gone", "ms": 50.0},
        {"scope": "s", "case": "stable", "ms": 10.0},
    ]
    _, regressions5, _ = compare(before, same_set, 25.0)
    cases.append(("⑤ 恰好等于阈值不算红", not regressions5, True))

    changed_fixture = json.loads(json.dumps(same_set))
    changed_fixture["records"][0]["modules"] = 99
    changed_fixture["records"][0]["best_ms"] = 300.0
    rows6, regressions6, _ = compare(before, changed_fixture, 25.0)
    fixture_note = any(r["case"] == "grows" and "夹具不同" in r["note"] for r in rows6)
    cases.append(("⑥ 夹具不同不判红但标出", fixture_note and not regressions6, True))

    # ⑦ 噪声带：低于 floor_ms 的抖动不判红
    noisy = json.loads(json.dumps(same_set))
    noisy["records"][2]["ms"] = 3.0  # 10ms → 3ms 是变快，换一个更小的基线
    tiny_before = json.loads(json.dumps(before))
    tiny_before["records"][2]["ms"] = 1.0
    _, regressions7, _ = compare(tiny_before, noisy, 25.0, floor_ms=5.0)
    cases.append(("⑦ 低于 5ms 的抖动不判红", not regressions7, True))

    failed = 0
    for name, got, want in cases:
        mark = "ok  " if got == want else "FAIL"
        if got != want:
            failed += 1
        print(f"  {mark} {name}（got={got}）")
    print(f"perf-compare self-test: {len(cases) - failed}/{len(cases)}")
    return 1 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="性能回归比较器（docs/perf/ledger.jsonl）")
    parser.add_argument("--since", default=None, help="基准：sha 前缀 / 版本号 / -N（默认上一条）")
    parser.add_argument("--threshold", type=float, default=25.0, help="退化多少算红（默认 25%%）")
    parser.add_argument("--floor-ms", type=float, default=5.0,
                        help="低于这个毫秒数不判红（噪声带，默认 5ms）")
    parser.add_argument("--json", action="store_true", help="机器可读输出")
    parser.add_argument("--self-test", action="store_true", help="自检判定规则")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    if not LEDGER.exists():
        print(f"error: 找不到台账 {LEDGER}（先跑 scripts/perf-ledger.sh）", file=sys.stderr)
        return 2
    entries = [json.loads(line) for line in LEDGER.read_text(encoding="utf-8").splitlines() if line.strip()]
    before, after = pick(entries, args.since)
    rows, regressions, notes = compare(before, after, args.threshold, args.floor_ms)

    if args.json:
        print(json.dumps(
            {
                "before": {"version": before.get("version"), "commit": before.get("commit")},
                "after": {"version": after.get("version"), "commit": after.get("commit")},
                "threshold": args.threshold,
                "rows": rows,
                "regressions": regressions,
                "notes": notes,
            },
            ensure_ascii=False,
            indent=2,
        ))
    else:
        render(rows, regressions, notes, before, after)
    return 1 if regressions else 0


if __name__ == "__main__":
    raise SystemExit(main())
