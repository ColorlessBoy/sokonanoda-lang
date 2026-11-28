#!/usr/bin/env python3
"""知识卡覆盖门禁：选项里点名的引理必须出现在实际推送的知识卡里。

问题（#146）：学习者点「我不知道」时，推送的知识卡应当是**选项里那几个引理的定义**，
而不是一张无关的通用卡。本脚本把这件事变成可执行判据：

1. 对每道有 `options` 的题（choice），从选项的**反引号片段**里取「点名的引理」——
   片段的第一个标识符，且它后面是结尾 / 空白 / `(` / `（` / `：` / `:` ⇒ 视为
   在点名这个引理（`le_empty`：… / `subset_trans α A B C h1 h2`）；
   表达式片段（`∃ f, …` / `a ∈ univ` / `apply Set.ext`）不算点名。
2. 只统计在课程正文（`courses/set-theory/lib` + `courses/set-theory/units`）里
   **有声明**的标识符：`theorem`，或结果类型是 `Prop` 的 `def`（谓词/关系定义）。
   泛型缩写（`h`）因此天然不计；命名空间名（`Set.Le` / `Set.Equiv` /
   `Type.Equiv`：自己还是别的声明的点号前缀）不计；类型/数据名（`Set` / `Rel` /
   `univ` / `Prod`：不是命题声明）不计。
3. 用 `engine.explain_for` 求**实际推送卡**；点名的引理必须出现在**卡片 JSON 文本**
   里（`definition.code`、`oneLiner`、`sections[].items` / `text` / 表格行都算——
   术语可能写在解释里）。没有卡（fallback）⇒ 这些引理全部算未覆盖。
4. errors 即红；存量欠账冻结在 `scripts/card-lint-baseline.json`（键 = `qid:name`，
   **双向棘轮**：基线外新增判红，已修条目没同步收紧也判红）。

判据正文、误报治理、已知限制与补卡流程见
`skills/sokonanoda-prep/references/card-coverage.md`。

用法：

    python3 scripts/card-lint.py                    # 门禁：按存量基线判红
    python3 scripts/card-lint.py --selftest         # 红/绿对照 fixtures
    python3 scripts/card-lint.py --json-out         # 机读 findings（stdout）
    python3 scripts/card-lint.py --no-baseline      # 不按基线判（全部列出）
    python3 scripts/card-lint.py --update-baseline  # 把当前 findings 收紧成新基线
    python3 scripts/card-lint.py --data-dir DIR     # 换一份 data/（复现件/夹具用）

退出码：0 = 通过；1 = 有判红 / 基线漂移；2 = 用法错误；3 = 前提缺失（引擎或数据读不出）。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import tempfile
from pathlib import Path

# 仓库根：scripts/card-lint.py → parents[1] = 仓库根。
HERE = Path(__file__).resolve().parent
REPO = HERE.parent
KNOWLEDGE = REPO / "courses" / "set-theory" / "knowledge"
DATA = KNOWLEDGE / "data"
COURSE = REPO / "courses" / "set-theory"
BASELINE = HERE / "card-lint-baseline.json"
FIXTURES = HERE / "card-lint-fixtures.json"

#: 课程正文（引理声明的唯一来源）。题目 `source` 指向的单元与 `lib/` 都在其中。
COURSE_SUBDIRS = ("lib", "units")
#: 解析一条声明时最多往后看多少字符找 `:=`（本课程所有 def 都远小于这个数）。
SIGNATURE_CAP = 1200

#: 行首声明：`theorem name …` / `def name …`（只认行首，注释 `--` 开头的行不匹配）。
DECL_RE = re.compile(r"^[ \t]*(theorem|def)[ \t]+([A-Za-z_][A-Za-z0-9_.']*)", re.M)
#: 选项里的反引号片段（不跨行）。
SPAN_RE = re.compile(r"`([^`\n]+)`")
#: 标识符（允许点号限定名与撇号，与课程正文的命名风格一致）。
IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_.']*")
#: 「点名」的合法后接字符：片段结束 / 空白 / 应用括号 / 中文冒号 / 英文冒号。
NAMED_TAIL = frozenset(" \t(（ \u3000：:")


# ── 声明解析（课程正文 → 可点名的引理集合）──────────────────────────────────


def _result_head(signature: str) -> str | None:
    """取 def 签名里**结果类型**的第一个标识符（忽略参数括号内的冒号）。"""
    depth = 0
    last = None
    for index, char in enumerate(signature):
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth = max(0, depth - 1)
        elif char == ":" and depth == 0:
            last = index
    if last is None:
        return None
    match = IDENT_RE.match(signature[last + 1:].strip())
    return match.group(0) if match else None


def collect_declarations(course_root: Path = COURSE) -> tuple[set[str], set[str]]:
    """扫描课程正文里的 `theorem` / `def`，返回（可点名引理集合, 命名空间集合）。

    * `theorem` 一律收（本内核要求签名是 Prop，见 AGENTS G-01）；
    * `def` 只在**结果类型是 `Prop`** 时收（谓词/关系定义，如 `Injective` /
      `Set.LeftInvOn` / `IsZero`）；结果类型是 `Type` / `Set α` / `Rel α α` /
      类型变量的是类型/数据（`Set` / `Rel` / `univ` / `EmptyRelation`），不收；
    * **命名空间名**不收：自己还是别的声明的点号前缀的名字（`Set.Le` 是
      `Set.Le.refl` 的前缀、`Set.Equiv` 是 `Set.Equiv.mk` 的前缀）。
    """
    declared: dict[str, set[str]] = {}
    for subdir in COURSE_SUBDIRS:
        root = course_root / subdir
        if not root.is_dir():
            continue
        for path in sorted(root.rglob("*.sokonanoda")):
            if not path.is_file():
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except OSError:
                continue
            for match in DECL_RE.finditer(text):
                kind, name = match.group(1), match.group(2)
                kinds = declared.setdefault(name, set())
                if kind == "theorem":
                    kinds.add("theorem")
                    continue
                signature = text[match.end():match.end() + SIGNATURE_CAP]
                body = signature.find(":=")
                if body >= 0:
                    signature = signature[:body]
                if _result_head(signature) == "Prop":
                    kinds.add("def-prop")
                else:
                    kinds.add("def-other")
    all_names = set(declared)
    namespaces = {
        name for name in all_names
        if any(other != name and other.startswith(name + ".") for other in all_names)
    }
    candidates = {
        name for name, kinds in declared.items()
        if (kinds & {"theorem", "def-prop"}) and name not in namespaces
    }
    return candidates, namespaces


# ── 选项 → 点名的引理 ─────────────────────────────────────────────────────────


def leading_token(span: str) -> str | None:
    """反引号片段的第一个标识符；片段不是「点名」（如表达式）时返回 None。"""
    text = span.strip()
    match = IDENT_RE.match(text)
    if not match:
        return None
    rest = text[match.end():]
    if rest and rest[0] not in NAMED_TAIL:
        return None
    return match.group(0)


def named_lemmas(question: dict, candidates: set[str]) -> list[str]:
    """该题选项里点名、且在课程正文有声明的引理（按选项顺序去重）。

    v1 只检查**不带命名空间的引理名**：`le_empty` / `subset_trans` 会被检查，
    `Set.Le.refl` 这类限定名留给后续批次（见 card-coverage.md §已知限制）。
    """
    names: list[str] = []
    for option in question.get("options") or []:
        if not isinstance(option, str):
            continue
        for span in SPAN_RE.findall(option):
            token = leading_token(span)
            if token is None or "." in token:
                continue
            if token in candidates and token not in names:
                names.append(token)
    return names


# ── 判据：实际推送卡必须覆盖点名的引理 ───────────────────────────────────────


def find_violations(data: dict, candidates: set[str]) -> list[dict]:
    """返回未覆盖清单：[{qid, name, card}]（按题目、选项顺序；确定且可复跑）。"""
    # 延迟导入：只有真跑判据时才依赖课程引擎；--selftest 也用它做匹配。
    if str(KNOWLEDGE) not in sys.path:
        sys.path.insert(0, str(KNOWLEDGE))
    import engine  # noqa: PLC0415  （课程引擎；CLI 侧唯一依赖）

    violations: list[dict] = []
    for question in data.get("questions") or []:
        names = named_lemmas(question, candidates)
        if not names:
            continue
        result = engine.explain_for(engine.new_state(), data, question["id"])
        card = result.get("card") if result.get("kind") == "knowledge-card" else None
        text = json.dumps(card, ensure_ascii=False) if card else ""
        for name in names:
            if name in text:
                continue
            violations.append({
                "qid": question["id"],
                "name": name,
                "card": card.get("id") if isinstance(card, dict) else None,
            })
    return violations


# ── 存量基线（双向棘轮：只许收紧，不许新增）──────────────────────────────────


def baseline_key(entry: dict) -> str:
    return "%s:%s" % (entry.get("qid"), entry.get("name"))


def load_baseline(path: Path) -> dict[str, dict]:
    if not path.exists():
        return {}
    payload = json.loads(path.read_text(encoding="utf-8"))
    entries = payload.get("entries") or []
    return {baseline_key(entry): entry for entry in entries if isinstance(entry, dict)}


def check_baseline(violations: list[dict], baseline: dict[str, dict]) -> tuple[list[dict], dict[str, dict]]:
    """（基线外新增, 已修未收紧的过期条目）——两者都判红。"""
    current = {baseline_key(item): item for item in violations}
    new = [item for key, item in current.items() if key not in baseline]
    stale = {key: entry for key, entry in baseline.items() if key not in current}
    return new, stale


def baseline_payload(violations: list[dict]) -> dict:
    entries = [
        {"qid": item["qid"], "name": item["name"], "card": item["card"]}
        for item in violations
    ]
    return {
        "schema": "soko.card-lint-baseline/1",
        "note": (
            "知识卡覆盖门禁的存量欠账基线：键 = qid:name（选项点名的引理没出现在实际"
            "推送卡里）；只许减不许增（新条目判红，已修条目没同步收紧也判红）。"
            "收紧用 `python3 scripts/card-lint.py --update-baseline`。"
        ),
        "updated": "2026-11-28",
        "violations": len(entries),
        "entries": entries,
    }


# ── 输出 ──────────────────────────────────────────────────────────────────────


def format_violation(item: dict) -> str:
    card = item.get("card")
    where = card if card else "fallback（无卡）"
    return "  %s: %s —— 推送卡 %s 里没有它" % (item["qid"], item["name"], where)


def print_violations(items: list[dict]) -> None:
    for item in items:
        print(format_violation(item))


# ── 自检（红/绿对照 fixtures）────────────────────────────────────────────────


def _fixture_data(case: dict) -> dict:
    questions = case.get("questions") or []
    cards = case.get("cards") or []
    return {
        "questions": questions,
        "cards": cards,
        "by_id": {question["id"]: question for question in questions},
        "glossary": [],
    }


def run_selftest() -> int:
    if not FIXTURES.exists():
        print("SELFTEST FAIL：找不到 %s" % FIXTURES, file=sys.stderr)
        return 3
    fixtures = json.loads(FIXTURES.read_text(encoding="utf-8"))
    passed = 0
    failures: list[str] = []

    declarations = fixtures.get("declarations") or {}
    if not declarations:
        print("SELFTEST FAIL：fixtures 里没有 declarations（夹具必须真走声明解析）", file=sys.stderr)
        return 3
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        for rel, content in declarations.items():
            path = root / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        candidates, namespaces = collect_declarations(root)

        # 夹具自证：声明解析真的分出了 theorem / Prop-def / 类型 / 命名空间。
        for name in fixtures.get("expected_candidates") or []:
            if name in candidates:
                passed += 1
            else:
                failures.append("声明解析：`%s` 应当是可点名引理，却没被收" % name)
        for name in fixtures.get("expected_not_candidates") or []:
            if name not in candidates:
                passed += 1
            else:
                failures.append("声明解析：`%s` 不该是可点名引理（命名空间/类型/数据），却被收了" % name)
        for name in fixtures.get("expected_namespaces") or []:
            if name in namespaces:
                passed += 1
            else:
                failures.append("命名空间识别：`%s` 应当是命名空间" % name)

        for case in fixtures.get("cases") or []:
            name = case.get("name") or "case"
            data = _fixture_data(case)
            violations = find_violations(data, candidates)
            got = [(item["qid"], item["name"]) for item in violations]
            expected = [tuple(pair) for pair in (case.get("expect") or [])]
            if got == expected:
                passed += 1
            else:
                failures.append("%s：期望 %r，实际 %r" % (name, expected, got))

    # 基线棘轮：新增判红 · 已修未收紧也判红（双向）。
    sample = [{"qid": "q1", "name": "foo", "card": "fixture-card"}]
    baseline = {"q1:foo": {"qid": "q1", "name": "foo"}}
    new, stale = check_baseline(sample, baseline)
    if not new and not stale:
        passed += 1
    else:
        failures.append("基线棘轮：完全一致的 findings 不该有新增/过期")
    new, stale = check_baseline(sample + [{"qid": "q2", "name": "bar", "card": None}], baseline)
    if len(new) == 1 and not stale:
        passed += 1
    else:
        failures.append("基线棘轮：基线外新增必须判红")
    new, stale = check_baseline([], baseline)
    if not new and "q1:foo" in stale:
        passed += 1
    else:
        failures.append("基线棘轮：已修条目没收紧必须判红")

    if failures:
        print("SELFTEST FAIL")
        for failure in failures:
            print("  -", failure)
        print("（%d 条对照通过，%d 条失败）" % (passed, len(failures)))
        return 1
    print("SELFTEST OK（%d 条对照；红/绿 fixtures + 声明解析 + 基线棘轮）" % passed)
    return 0


# ── 主流程 ───────────────────────────────────────────────────────────────────


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="知识卡覆盖门禁：选项里点名的引理必须出现在实际推送卡里",
        formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--selftest", action="store_true", help="红/绿对照 fixtures")
    parser.add_argument("--json-out", action="store_true", help="机读 findings（stdout）")
    parser.add_argument("--data-dir", default=str(DATA), metavar="DIR",
                        help="题库数据目录（默认为课程的 knowledge/data）")
    parser.add_argument("--baseline", default=str(BASELINE), metavar="PATH", help="存量基线路径")
    parser.add_argument("--no-baseline", action="store_true", help="不按基线判（全部列出）")
    parser.add_argument("--update-baseline", action="store_true", help="把当前 findings 收紧成新基线")
    args = parser.parse_args(argv)

    if args.selftest:
        return run_selftest()

    data_dir = Path(args.data_dir)
    if not (data_dir / "questions.json").exists() or not (data_dir / "cards.json").exists():
        print("card-lint: 读不到题库数据（%s 下缺 questions.json / cards.json）⇒ 不假装判过 ✗"
              % data_dir, file=sys.stderr)
        return 3
    try:
        if str(KNOWLEDGE) not in sys.path:
            sys.path.insert(0, str(KNOWLEDGE))
        import engine  # noqa: PLC0415
        data = engine.load_data(data_dir)
    except Exception as exc:  # 引擎或数据坏了：3 = 前提缺失，不是「通过」
        print("card-lint: 加载课程引擎/数据失败：%s ⇒ 不假装判过 ✗" % exc, file=sys.stderr)
        return 3

    candidates, _namespaces = collect_declarations(COURSE)
    violations = find_violations(data, candidates)

    errors = len(violations)
    baseline_path = Path(args.baseline)
    new: list[dict] = []
    stale: dict[str, dict] = {}
    if not args.no_baseline:
        if args.update_baseline:
            payload = baseline_payload(violations)
            baseline_path.write_text(
                json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            print("card-lint: 基线已写入 %s（%d 条存量）" % (baseline_path, errors))
            return 0
        baseline = load_baseline(baseline_path)
        new, stale = check_baseline(violations, baseline)

    if args.json_out:
        print(json.dumps({
            "violations": errors,
            "findings": violations,
            "new": new,
            "stale": list(stale.values()),
            "baseline": None if args.no_baseline else str(baseline_path),
        }, ensure_ascii=False, indent=2))
    else:
        if args.no_baseline:
            print_violations(violations)
        elif new:
            print("基线外新增（必须补卡/扩卡，或说明后收紧基线）：")
            print_violations(new)
        if stale:
            print("基线已过期（这些引理已被覆盖，跑 --update-baseline 收紧）：")
            for entry in stale.values():
                print("  %s:%s" % (entry.get("qid"), entry.get("name")))
        if not args.no_baseline and not new and not stale and violations:
            print("存量基线（只减不增）：%d 条（见 %s）"
                  % (errors, baseline_path.relative_to(REPO) if baseline_path.is_relative_to(REPO) else baseline_path))
        print("card-lint: %d 处未覆盖%s"
              % (errors, "" if args.no_baseline else
                 "（基线外新增 %d 条、过期 %d 条）" % (len(new), len(stale))))

    if args.no_baseline:
        # 不按基线判时，未覆盖本身就是红（诊断/复现件用法）。
        return 1 if violations else 0
    if new or stale:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
