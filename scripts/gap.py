#!/usr/bin/env python3
"""gap.py —— 课程驱动缺口台账（`docs/gaps/ledger.jsonl`）的读写与交接工具。

设计：`docs/design/teaching-project.md` §6（台账协议、WO 模板、排序规则、「缺口即测试」）。

用法：
  python3 scripts/gap.py list [--severity blocker] [--kind library] [--status open] [--json]
  python3 scripts/gap.py show <id>
  python3 scripts/gap.py next [--json]        # 挑下一条要修的缺口，并打印**可直接粘贴给另一个 agent** 的 prompt
  python3 scripts/gap.py check                # 跑全部 repro，报告「台账 vs 现实」的偏差（CI/收尾用）
  python3 scripts/gap.py selftest             # 自检 judge() 的判定规则（不跑判卷，秒级）
  python3 scripts/gap.py close <id> --version 0.59.0 [--note "..."]   # 写 fixed_in（重跑 repro 确认已绿）

退出码：0 = 正常 / 1 = 有偏差（`check`）或用法错误 / 2 = 台账读不出来。
零依赖（python3 标准库）；repro 脚本的约定见 `docs/gaps/README.md`
（exit 0 = 缺口仍在且与台账一致；exit 1 = 行为变了）。

复现件的**期望**默认由 `status` 推出（`fixed` ⇒ 应当转绿 / `.sh` 应当 exit≠0），
但有些缺口的「修好」恰恰是**判红**（例如 G-01 钉的是"签名写错必须被拒」）。这类条目
在台账里写一个显式的 `repro_expect`（`clean` / `rejected` / `exit0` / `nonzero`）覆盖推导。
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / "docs" / "gaps" / "ledger.jsonl"
SEVERITY_ORDER = {"blocker": 0, "painful": 1, "nice": 2}
OPEN_STATUSES = {"open", "workaround", "wo-filed"}


def load() -> list[dict]:
    try:
        text = LEDGER.read_text(encoding="utf-8")
    except FileNotFoundError:
        print(f"error: 找不到台账 {LEDGER}", file=sys.stderr)
        sys.exit(2)
    entries = []
    for lineno, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            entries.append(json.loads(line))
        except json.JSONDecodeError as err:
            print(f"error: {LEDGER}:{lineno} 不是合法 JSON：{err}", file=sys.stderr)
            sys.exit(2)
    return entries


def save(entries: list[dict]) -> None:
    lines = [json.dumps(e, ensure_ascii=False, separators=(",", ":")) for e in entries]
    LEDGER.write_text("\n".join(lines) + "\n", encoding="utf-8")


def sort_key(entry: dict) -> tuple:
    wo = entry.get("wo_planned") or "WO-999"
    return (SEVERITY_ORDER.get(entry.get("severity", "nice"), 9), wo, entry["id"])


def clean_env() -> dict:
    """复现的运行环境：剔除会**短路夹具**的二进制覆盖。

    `SOKONANODA_BIN` / `SOKONANODA_LSP_BIN` 是"显式指定二进制"的逃生门，启动器会
    无条件采信它；而这批复现里有一半**测的就是启动器自己的解析链**（G-11/G-16 的
    夹具故意放一个陈旧缓存，看它会不会被拒绝）。一旦环境里带着覆盖，夹具就被绕过，
    复现会假报「缺口仍在」——`scripts/soko gate` 一开始正是这么把台账门禁带红的。
    所以这里一律剔除：要测覆盖的脚本自己 inline 赋值（G-11 ③ 就是这么写的）。
    """
    env = dict(os.environ)
    for key in ("SOKONANODA_BIN", "SOKONANODA_LSP_BIN"):
        env.pop(key, None)
    return env


def run_repro(entry: dict) -> tuple[str, int, str]:
    """跑一条缺口的复现；返回 (kind, exit, 摘要)。"""
    repro = entry.get("repro")
    if not repro:
        return ("none", -1, "没有复现文件")
    path = ROOT / repro
    if not path.exists():
        return ("missing", -1, f"复现文件不存在：{repro}")
    if path.suffix == ".sh":
        proc = subprocess.run(["bash", str(path)], cwd=ROOT, capture_output=True, text=True,
                              env=clean_env())
        return ("script", proc.returncode, proc.stderr.strip().splitlines()[-1] if proc.stderr.strip() else "")
    if path.suffix == ".sokonanoda":
        proc = subprocess.run(
            [str(ROOT / "scripts" / "soko"), "grade", str(path)],
            cwd=ROOT, capture_output=True, text=True, env=clean_env(),
        )
        text = proc.stdout + proc.stderr
        checked = '"type":"decl.checked"' in text
        diagnostic = '"type":"diagnostic"' in text
        clean = proc.returncode == 0 and checked and not diagnostic
        return ("sokonanoda", 0 if clean else 1, "clean" if clean else "仍有失败/无 checked")
    if path.is_dir():
        return ("dir", -1, "目录型复现，请用它下面的 .sh（见 ledger 的 repro 字段）")
    return ("unknown", -1, f"不认识的复现类型：{repro}")


def judge(entry: dict, kind: str, code: int) -> tuple[str, str, bool]:
    """把一次复现结果判成 (observed, expected, ok)。

    期望默认由 `status` 推出；`repro_expect` 显式覆盖（见模块 docstring）。
    取值与复现类型不匹配、或取值非法时 ok=False，且 expected 位置直接给错误原因
    ——台账是契约，写错的字段必须报出来，而不是被默认推导悄悄盖过。
    """
    raw = entry.get("repro_expect")
    fixed = entry.get("status") == "fixed"
    if kind == "sokonanoda":
        clean = code == 0
        observed = "已判卷通过" if clean else "仍有失败"
        if raw is None:
            want, expected = fixed, ("已判卷通过" if fixed else "仍有失败")
        elif raw == "clean":
            want, expected = True, "已判卷通过（repro_expect=clean）"
        elif raw == "rejected":
            want, expected = False, "应判红（repro_expect=rejected）"
        else:
            return observed, f"非法 repro_expect={raw!r}：.sokonanoda 只吃 clean/rejected", False
        return observed, expected, clean == want
    if kind == "script":
        changed = code != 0
        observed = "行为已变" if changed else "缺口仍在"
        if raw is None:
            want, expected = fixed, ("行为已变" if fixed else "缺口仍在")
        elif raw == "nonzero":
            want, expected = True, "行为已变（repro_expect=nonzero）"
        elif raw == "exit0":
            want, expected = False, "缺口仍在（repro_expect=exit0）"
        else:
            return observed, f"非法 repro_expect={raw!r}：脚本只吃 exit0/nonzero", False
        return observed, expected, changed == want
    return "", "", False


def cmd_list(args: argparse.Namespace) -> int:
    entries = load()
    if args.severity:
        entries = [e for e in entries if e.get("severity") == args.severity]
    if args.status:
        entries = [e for e in entries if e.get("status") == args.status]
    if getattr(args, "kind", None):
        entries = [e for e in entries if e.get("kind") == args.kind]
    entries.sort(key=sort_key)
    if args.json:
        print(json.dumps(entries, ensure_ascii=False, indent=2))
        return 0
    print(f"{'ID':<6}{'级别':<10}{'状态':<12}{'WO':<20}标题")
    for e in entries:
        print(f"{e['id']:<6}{e.get('severity', '?'):<10}{e.get('status', '?'):<12}"
              f"{(e.get('wo') or e.get('wo_planned') or '-').split('/')[-1][:18]:<20}{e.get('title', '')}")
    by_kind = {}
    for e in entries:
        by_kind[e.get("kind", "?")] = by_kind.get(e.get("kind", "?"), 0) + 1
    print(f"\n共 {len(entries)} 条（" + "、".join(f"{k} {v}" for k, v in sorted(by_kind.items())) + "）；"
          f"blocker {sum(1 for e in entries if e.get('severity') == 'blocker')} 条，"
          f"未关账 {sum(1 for e in entries if e.get('status') in OPEN_STATUSES)} 条。")
    return 0


def cmd_show(args: argparse.Namespace) -> int:
    for e in load():
        if e["id"] == args.id:
            print(json.dumps(e, ensure_ascii=False, indent=2))
            return 0
    print(f"error: 没有 {args.id}", file=sys.stderr)
    return 1


def wo_prompt(entry: dict) -> str:
    repro = entry.get("repro") or "（缺）"
    cmd = f"bash {repro}" if str(repro).endswith(".sh") else f"scripts/soko grade {repro}"
    return "\n".join([
        f"# 工作单 {entry.get('wo_planned') or '（未编号）'} —— {entry['id']} {entry['title']}",
        "",
        f"仓库：sokonanoda-lang（本仓库）。设计：docs/design/teaching-project.md §6。",
        "",
        "## 用户可见症状 / 最小复现",
        f"- 复现：`{cmd}`",
        f"- 期望（官方 Lean 4 语义）：{entry.get('expected_lean', '（未填）')}",
        f"- 今天：{entry.get('today', '（未填）')}",
        f"- 位置线索：{(entry.get('where') or {}).get('area', '（未填）')}",
        f"  {(entry.get('where') or {}).get('note', '')}".rstrip(),
        f"- 现有绕行（与代价）：{entry.get('workaround', '（无）')}",
        f"- 被它挡住的东西：{'、'.join(entry.get('blocks') or []) or '（未标）'}",
        "",
        "## 交付要求（本仓库硬规则）",
        "1. **设计先行**：先在 docs/design/ 补/改一篇设计（含取舍与 as-built），再动手；",
        "2. **TDD 三层**：front 单测 + CLI e2e + 课程/复现用例；改前先让复现失败、改后转绿；",
        "3. **判据走内核**，禁止文本比对；**内核是冻结快照**（若必须动内核，先读 docs/architecture.md §6）；",
        "4. **语法增量三件套**：课程 + 测试 + 白名单（REQUIREMENTS §2 第 3 条）；",
        "5. **同一轮同步**：editor/vscode/ + skills/ + AGENTS.md + docs/HANDOVER.md + STATUS.md + REQUIREMENTS §9；",
        "6. **验收命令**：`scripts/soko gate`（fmt+clippy+test+playground 锚点）+ 全量 `cargo test --workspace --locked`；",
        "7. **关账**：`python3 scripts/gap.py close " + entry["id"] + " --version <新版本>`（复现脚本会翻成 exit 1 提醒你）。",
        "",
        "## 完成后请回填",
        f"- docs/gaps/ledger.jsonl 的 {entry['id']}：status=fixed、fixed_in=<版本>；",
        "- 课程侧（独立课程仓）升级版本钉并复跑它的本地门禁。",
    ])


def cmd_next(args: argparse.Namespace) -> int:
    entries = [e for e in load() if e.get("status") in OPEN_STATUSES]
    if not entries:
        print("没有未关账的缺口。")
        return 0
    entries.sort(key=sort_key)
    entry = entries[0]
    if args.json:
        print(json.dumps(entry, ensure_ascii=False, indent=2))
    else:
        print(wo_prompt(entry))
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    entries = load()
    bad = 0
    print(f"{'ID':<6}{'状态':<12}{'复现':<12}判定")
    for e in entries:
        status = e.get("status", "?")
        if not e.get("repro"):
            print(f"{e['id']:<6}{status:<12}{'-':<12}跳过（没有复现文件）")
            continue
        kind, code, note = run_repro(e)
        if kind in {"missing", "dir", "none", "unknown"}:
            print(f"{e['id']:<6}{status:<12}{kind:<12}跳过（{note}）")
            continue
        observed, expected, ok = judge(e, kind, code)
        bad += not ok
        print(f"{e['id']:<6}{status:<12}{kind:<12}{observed}"
              f"{'' if ok else f'  ← 台账写的是「{expected}」，请更新'}")
    if bad:
        print(f"\n{bad} 条与台账不一致 —— 台账是契约：要么修好了（写 fixed_in），要么行为回退了。",
              file=sys.stderr)
        return 1
    print("\n全部与台账一致。")
    return 0


def cmd_close(args: argparse.Namespace) -> int:
    entries = load()
    for e in entries:
        if e["id"] == args.id:
            kind, code, note = run_repro(e) if e.get("repro") else ("none", -1, "")
            if kind in {"script", "sokonanoda"}:
                # 用「如果现在写 fixed，复现该是什么样」来预检：显式 repro_expect 也算数。
                _, expected, ok = judge(dict(e, status="fixed"), kind, code)
                if not ok:
                    print(f"error: {args.id} 的复现与「已修」不符"
                          f"（实测 {kind}/{code} {note}，期望{expected}），先确认再关。",
                          file=sys.stderr)
                    return 1
            e["status"] = "fixed"
            e["fixed_in"] = args.version
            if args.note:
                e["notes"] = (e.get("notes", "") + "｜" + args.note).strip("｜")
            save(entries)
            print(f"{args.id} 已关账：fixed_in={args.version}（复现判定：{kind}/{code} {note}）")
            return 0
    print(f"error: 没有 {args.id}", file=sys.stderr)
    return 1


def cmd_selftest(args: argparse.Namespace) -> int:
    """判据通道自检：`judge()` 的期望推导 + 显式覆盖 + 非法值都要被钉住。

    它不碰台账文件、不跑判卷——只验证"台账 vs 现实"的判定规则本身。
    """
    cases = [
        # (说明, 条目, kind, code, 期望 ok, 期望 expected 里含「非法」)
        ("缺省 fixed+sokonanoda 干净 = 一致", {"status": "fixed"}, "sokonanoda", 0, True, False),
        ("缺省 fixed+sokonanoda 判红 = 不一致", {"status": "fixed"}, "sokonanoda", 1, False, False),
        ("缺省 open+sokonanoda 判红 = 一致", {"status": "open"}, "sokonanoda", 1, True, False),
        ("缺省 open+sokonanoda 干净 = 不一致", {"status": "open"}, "sokonanoda", 0, False, False),
        ("显式 rejected+判红 = 一致（G-01 形态）",
         {"status": "fixed", "repro_expect": "rejected"}, "sokonanoda", 1, True, False),
        ("显式 rejected+干净 = 不一致（假绿会被抓）",
         {"status": "fixed", "repro_expect": "rejected"}, "sokonanoda", 0, False, False),
        ("显式 clean+判红 = 不一致",
         {"status": "open", "repro_expect": "clean"}, "sokonanoda", 1, False, False),
        ("sokonanoda 写脚本取值 = 非法", {"repro_expect": "nonzero"}, "sokonanoda", 1, False, True),
        ("缺省 fixed+script 行为已变 = 一致", {"status": "fixed"}, "script", 1, True, False),
        ("缺省 fixed+script 缺口仍在 = 不一致", {"status": "fixed"}, "script", 0, False, False),
        ("缺省 open+script 缺口仍在 = 一致", {"status": "open"}, "script", 0, True, False),
        ("显式 exit0+缺口仍在 = 一致（修好前也允许显式）",
         {"status": "fixed", "repro_expect": "exit0"}, "script", 0, True, False),
        ("显式 nonzero+缺口仍在 = 不一致",
         {"status": "open", "repro_expect": "nonzero"}, "script", 0, False, False),
        ("script 写判卷取值 = 非法", {"repro_expect": "clean"}, "script", 0, False, True),
    ]
    bad = 0
    saved = {k: os.environ.get(k) for k in ("SOKONANODA_BIN", "SOKONANODA_LSP_BIN")}
    os.environ["SOKONANODA_BIN"] = "/nonexistent/sokonanoda"
    os.environ["SOKONANODA_LSP_BIN"] = "/nonexistent/sokonanoda-lsp"
    env = clean_env()
    for key in saved:
        if key in env:
            bad += 1
            print(f"FAIL clean_env 没有剔除 {key}（复现夹具会被环境短路）", file=sys.stderr)
    for key, value in saved.items():
        if value is None:
            os.environ.pop(key, None)
        else:
            os.environ[key] = value
    for desc, entry, kind, code, want, illegal in cases:
        observed, expected, ok = judge(entry, kind, code)
        if ok != want:
            bad += 1
            print(f"FAIL {desc}: judge -> ({observed}, {expected}, {ok})，期望 ok={want}",
                  file=sys.stderr)
        elif ("非法" in expected) != illegal:
            bad += 1
            print(f"FAIL {desc}: 非法取值的报错形态不对（expected={expected}，期望非法={illegal}）",
                  file=sys.stderr)
    if bad:
        print(f"gap.py selftest: {bad}/{len(cases)} 条判据不成立。", file=sys.stderr)
        return 1
    print(f"gap.py selftest: {len(cases)} 条判据 + clean_env 剔除覆盖 全部成立。")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="课程驱动缺口台账工具（docs/gaps/ledger.jsonl）")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("list", help="列表（按级别/WO 排序）")
    p.add_argument("--severity", choices=list(SEVERITY_ORDER))
    p.add_argument("--kind", help="language / tooling / infra / library")
    p.add_argument("--status")
    p.add_argument("--json", action="store_true")
    p.set_defaults(func=cmd_list)

    p = sub.add_parser("show", help="看一条的完整 JSON")
    p.add_argument("id")
    p.set_defaults(func=cmd_show)

    p = sub.add_parser("next", help="下一条要修的缺口 + 可粘贴给另一个 agent 的 prompt")
    p.add_argument("--json", action="store_true")
    p.set_defaults(func=cmd_next)

    p = sub.add_parser("check", help="跑全部 repro，报告台账与现实的偏差")
    p.set_defaults(func=cmd_check)

    p = sub.add_parser("close", help="关账（写 fixed_in）")
    p.add_argument("id")
    p.add_argument("--version", required=True, help="修复落地的版本，例如 0.59.0")
    p.add_argument("--note")
    p.set_defaults(func=cmd_close)

    p = sub.add_parser("selftest", help="自检 judge() 的判定规则（不碰台账、不判卷）")
    p.set_defaults(func=cmd_selftest)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
