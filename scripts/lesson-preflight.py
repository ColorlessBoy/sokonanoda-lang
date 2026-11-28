#!/usr/bin/env python3
"""会话预检：开始做题前，把「即将发给学习者的题」先过一遍内容门禁。

父任务 #147 的「出题前预检」环节（issue #151）。fast path 直接由
`engine.py next` 出题，出题前不查任何内容质量 ⇒ 坏题会直接发给学习者。
本脚本在**不发题、不改 state** 的前提下，算出学习者接下来会遇到的题并逐题检查：

1. **文本规则**（复用 `text_lint.py` 的 question 模式，issue #149）：
   X11 自造术语 = **blocker**；X6 段首代词 / X7 空洞评价 / X12 隐喻 /
   X13 选项中文过长 = warning。
2. **卡片覆盖**（复用 `scripts/card-lint.py`，issue #146）：选项里点名的引理
   没出现在**实际推送卡**里 = blocker。脚本存在时以它为准（子进程
   `--json-out`）；未合并/不存在时退回本脚本内置的**同口径兼容检查**，
   并在输出里标明 `card_lint: "compat"`（不静默跳过）。
3. **无卡兜底**：`engine.explain_for` 返回 fallback-card 时，要求它包含这道题的
   `why` 与选项点名的引理名 = 缺一项就是 blocker。

两种模式：

    python3 scripts/lesson-preflight.py                    # learner path（默认）
    python3 scripts/lesson-preflight.py --all              # content 模式：全库扫描
    python3 scripts/lesson-preflight.py --selftest         # 红/绿对照 fixtures
    python3 scripts/lesson-preflight.py --json             # 机读输出（stdout）
    python3 scripts/lesson-preflight.py --limit 22 --json  # 看学习者接下来 22 题
    python3 scripts/lesson-preflight.py --state S.json --chapter I.9
    python3 scripts/lesson-preflight.py --all --baseline B.json --update-baseline
    python3 scripts/lesson-preflight.py --all --no-baseline   # 列出全部 blocker
    python3 scripts/lesson-preflight.py --card-lint PATH   # 指定 card-lint.py

learner 模式：默认 state 解析顺序 = 显式 `--state` / `$LESSON_PREFLIGHT_STATE`
/ `~/.dsh/state/sokonanoda-dsh/quiz/default.json`（DSH fast path 的真实 state）
/ `<repo>/.cache/quiz-proxy/state/default.json`。**读到的 state 一律 deepcopy 后再走
`engine.next_step`**，绝不写回学习者的 state。找不到 state ⇒ 自动转 content 模式。

content 模式：扫描题库全部题目（不依赖 state），blocker 数按存量基线
`scripts/lesson-preflight-baseline.json` **双向棘轮**：基线外新增判红，
已修条目没同步收紧也判红（与 card-lint 同口径）。`--update-baseline` 收紧。

退出码：0 = 0 blocker；1 = 有 blocker（或基线漂移）；2 = 用法错误；
3 = 前提缺失（题库/引擎/text_lint 读不出，或显式给的 --state / --card-lint
不存在）——**绝不静默跳过**。
"""
from __future__ import annotations

import argparse
import copy
import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# 仓库根：scripts/lesson-preflight.py → parents[1] = 仓库根。
HERE = Path(__file__).resolve().parent
REPO = HERE.parent
KNOWLEDGE = REPO / "courses" / "set-theory" / "knowledge"
DATA = KNOWLEDGE / "data"
COURSE = REPO / "courses" / "set-theory"
TEXT_LINT_PATH = KNOWLEDGE / "text_lint.py"
CARD_LINT_PATH = HERE / "card-lint.py"
TERMS_PATH = KNOWLEDGE / "text-lint-terms.json"
BASELINE = HERE / "lesson-preflight-baseline.json"
FIXTURES = HERE / "lesson-preflight-fixtures.json"

#: learner 模式默认 state 候选（按优先级；第一个存在的即采用）。
DEFAULT_STATE_CANDIDATES = (
    Path(os.path.expanduser("~")) / ".dsh" / "state" / "sokonanoda-dsh" / "quiz" / "default.json",
    REPO / ".cache" / "quiz-proxy" / "state" / "default.json",
)

#: 预检只关心这五条题目规则（#151 定义）；其余段落公理由 text_lint 自己的门禁管。
QUESTION_AXIOMS = frozenset({"X6", "X7", "X11", "X12", "X13"})
#: 文本规则里判 blocker 的只有 X11（自造术语）；其余为 warning。
BLOCKER_AXIOMS = frozenset({"X11"})

#: X11/X12 词表（`text-lint-terms.json` 缺失/坏掉时的兜底，与 text_lint 默认一致）。
DEFAULT_SELF_INVENTED = ("落点", "原序对", "全称命题")
DEFAULT_METAPHORS = ("搬运", "拆开", "装回", "接起来", "外壳")
X13_MAX_CJK = 24

CJK = re.compile(r"[\u4e00-\u9fff]")
CODE_SPAN = re.compile(r"`[^`\n]*`")
#: 选项里的反引号片段 / 标识符 / 「点名」的合法后接字符（与 card-lint 同口径）。
SPAN_RE = re.compile(r"`([^`\n]+)`")
IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_.']*")
NAMED_TAIL = frozenset(" \t(（ \u3000：:")
DECL_RE = re.compile(r"^[ \t]*(theorem|def)[ \t]+([A-Za-z_][A-Za-z0-9_.']*)", re.M)
SIGNATURE_CAP = 1200
#: card-lint 兼容检查扫描的课程正文子目录（引理声明的唯一来源）。
COURSE_SUBDIRS = ("lib", "units")


class PreconditionError(Exception):
    """前提缺失（题库/引擎/text_lint/显式路径）⇒ exit 3，不假装判过。"""


# ── 依赖加载 ──────────────────────────────────────────────────────────────────


def _load_module(path: Path, name: str):
    """按文件路径加载模块（text_lint.py 是可导入模块，但不保证在 sys.path 上）。"""
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise PreconditionError("无法加载 %s" % path)
    module = importlib.util.module_from_spec(spec)
    # Python 3.12 的 dataclasses 解析 `from __future__ annotations` 时会读
    # `sys.modules[cls.__module__].__dict__`；模块不先登记就直接 exec ⇒
    # AttributeError: 'NoneType' object has no attribute '__dict__'（CI 3.12 实测）。
    sys.modules[name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        sys.modules.pop(name, None)
        raise
    return module


def load_engine():
    """课程引擎（只读复用 `new_state`/`load_data`/`next_step`/`explain_for`）。"""
    if str(KNOWLEDGE) not in sys.path:
        sys.path.insert(0, str(KNOWLEDGE))
    try:
        import engine  # noqa: PLC0415  （课程引擎；唯一数据来源）
    except Exception as exc:  # noqa: BLE001
        raise PreconditionError("课程引擎 engine.py 加载失败：%s" % exc) from exc
    return engine


def load_text_lint():
    if not TEXT_LINT_PATH.exists():
        raise PreconditionError("找不到文本门禁 %s（#144）" % TEXT_LINT_PATH)
    try:
        return _load_module(TEXT_LINT_PATH, "lesson_preflight_text_lint")
    except Exception as exc:  # noqa: BLE001
        raise PreconditionError("文本门禁 text_lint.py 加载失败：%s" % exc) from exc


def load_data(engine, data_dir: Path) -> dict:
    if not (data_dir / "questions.json").exists() or not (data_dir / "cards.json").exists():
        raise PreconditionError("读不到题库数据（%s 下缺 questions.json / cards.json）" % data_dir)
    try:
        data = engine.load_data(data_dir)
        engine.index_cards(data)
    except Exception as exc:  # noqa: BLE001
        raise PreconditionError("加载课程数据失败：%s" % exc) from exc
    return data


# ── X11/X12 词表（兼容兜底：text_lint 没有 question 模式时用） ────────────────


def load_term_lists() -> tuple[tuple[str, ...], tuple[str, ...], str]:
    """读 X11/X12 词表；返回（自造术语, 隐喻, 来源说明）。"""
    try:
        data = json.loads(TERMS_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        data = {}
    if not isinstance(data, dict):
        data = {}

    def pick(key: str, default: tuple[str, ...]) -> tuple[str, ...]:
        words = data.get(key)
        if not isinstance(words, list):
            return default
        return tuple(str(word) for word in words if str(word))

    if data:
        return pick("selfInvented", DEFAULT_SELF_INVENTED), pick(
            "metaphors", DEFAULT_METAPHORS), str(TERMS_PATH)
    return DEFAULT_SELF_INVENTED, DEFAULT_METAPHORS, "builtin-default"


# ── 文本规则：X11 blocker + X6/X7/X12/X13 warning ────────────────────────────


def question_units(text_lint, question: dict, qid: str):
    """题目的用户可见文本单元（prompt/why/options[i]）；优先用 #149 的接口。"""
    if hasattr(text_lint, "question_text_units"):
        try:
            return text_lint.question_text_units(question, "questions.json", qid)
        except Exception:  # noqa: BLE001  （接口漂移就退回自建单元）
            pass
    units = [
        text_lint.Unit(file="questions.json", id=qid, field="prompt",
                       text=str(question.get("prompt") or ""), is_question=True),
        text_lint.Unit(file="questions.json", id=qid, field="why",
                       text=str(question.get("why") or ""), is_question=True),
    ]
    for index, option in enumerate(question.get("options") or []):
        units.append(text_lint.Unit(file="questions.json", id=qid, field=f"options[{index}]",
                                    text=str(option), is_question=True, is_option=True))
    return units


def compat_expression_findings(text_lint, unit, terms) -> list[dict]:
    """text_lint 没有 question 模式时，按 #149 的口径补跑 X11/X12/X13。

    只在 `check_question_expression` 缺席时启用，避免与 text_lint 重复计数。
    """
    invented, metaphors = terms
    out: list[dict] = []
    raw = str(unit.text or "")
    if not raw.strip():
        return out
    hits = [word for word in invented if word in raw]
    if hits:
        out.append({"axiom": "X11", "code": "X11-invented-term", "level": "error",
                    "field": unit.field, "note": "自造术语：" + "、".join(hits)})
    hits = [word for word in metaphors if word in raw]
    if hits:
        out.append({"axiom": "X12", "code": "X12-metaphor", "level": "warning",
                    "field": unit.field, "note": "用隐喻代替代码/命名：" + "、".join(hits)})
    if getattr(unit, "is_option", False):
        cjk = len(CJK.findall(text_lint.normalize(raw)))
        if cjk > X13_MAX_CJK:
            out.append({"axiom": "X13", "code": "X13-option-long", "level": "warning",
                        "field": unit.field,
                        "note": f"选项中文 {cjk} 字超过 {X13_MAX_CJK} 字（代码/公式不计）"})
    return out


def check_question_text(text_lint, question: dict, terms, has_question_mode: bool) -> list[dict]:
    """题目文本规则 → 归一化 findings（level = blocker / warning）。"""
    qid = str(question.get("id") or "?")
    out: list[dict] = []
    for unit in question_units(text_lint, question, qid):
        try:
            raw = text_lint.check_unit(unit)
        except Exception as exc:  # noqa: BLE001
            raise PreconditionError("text_lint.check_unit 失败（%s）：%s" % (qid, exc)) from exc
        if not has_question_mode:
            raw = raw + compat_expression_findings(text_lint, unit, terms)
        for item in raw:
            axiom = str(item.get("axiom") or item.get("code") or "?")
            if axiom not in QUESTION_AXIOMS:
                continue
            out.append({
                "qid": qid,
                "code": axiom,
                "level": "blocker" if axiom in BLOCKER_AXIOMS else "warning",
                "field": str(item.get("field") or unit.field),
                "name": "",
                "note": str(item.get("note") or ""),
            })
    return out


# ── 卡片覆盖：选项点名的引理必须出现在实际推送卡里（#146 / card-lint 同口径） ──


def _result_head(signature: str) -> str | None:
    """取 def 签名里结果类型的第一个标识符（忽略参数括号内的冒号）。"""
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


def collect_declarations(course_root: Path) -> tuple[set[str], set[str]]:
    """扫课程正文，返回（可点名引理集合, 命名空间集合）——与 card-lint 同口径。"""
    declared: dict[str, set[str]] = {}
    for subdir in COURSE_SUBDIRS:
        root = course_root / subdir
        if not root.is_dir():
            continue
        for path in sorted(root.rglob("*.sokonanoda")):
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
                kinds.add("def-prop" if _result_head(signature) == "Prop" else "def-other")
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
    """该题选项里点名、且在课程正文有声明的引理（按选项顺序去重）。"""
    names: list[str] = []
    for option in question.get("options") or []:
        if not isinstance(option, str):
            continue
        for span in SPAN_RE.findall(option):
            token = leading_token(span)
            if token is None or "." in token or token not in candidates:
                continue
            if token not in names:
                names.append(token)
    return names


def card_text_units(card) -> str:
    """把卡片里**所有用户可见字符串**拼起来（不经过 json.dumps）。

    `json.dumps` 会转义 `\\` / `"`，直接用序列化 JSON 做子串匹配会把「卡片里明明
    有的 why」误判成缺失（2026-11-29 lead 反馈的 I.1.sdiff-4 假红）。这里按
    definition.code / oneLiner / sections[].text/items/rows / types 收集原始文本。
    """
    parts: list[str] = []

    def walk(value) -> None:
        if isinstance(value, str):
            parts.append(value)
        elif isinstance(value, dict):
            for item in value.values():
                walk(item)
        elif isinstance(value, (list, tuple)):
            for item in value:
                walk(item)

    walk(card)
    return "\n".join(parts)


def compat_coverage(engine, data: dict, candidates: set[str]) -> dict[str, list[dict]]:
    """内置卡片覆盖检查（card-lint.py 不存在时的同口径兼容路径）→ qid → findings。"""
    by_qid: dict[str, list[dict]] = {}
    for question in data.get("questions") or []:
        names = named_lemmas(question, candidates)
        if not names:
            continue
        result = engine.explain_for(engine.new_state(), data, question["id"])
        card = result.get("card") if result.get("kind") == "knowledge-card" else None
        text = card_text_units(card) if card else ""
        for name in names:
            if name in text:
                continue
            by_qid.setdefault(question["id"], []).append({
                "qid": question["id"], "code": "card", "level": "blocker",
                "field": "", "name": name,
                "note": "选项点名的引理没出现在推送卡里（card=%s）"
                        % (card.get("id") if isinstance(card, dict) else "fallback/no-card"),
            })
    return by_qid


def run_card_lint(script: Path, data_dir: Path) -> tuple[dict[str, list[dict]] | None, str]:
    """跑真实 `card-lint.py --json-out --no-baseline`；失败返回 (None, 原因) 由调用方降级。"""
    command = [sys.executable, str(script), "--json-out", "--no-baseline",
               "--data-dir", str(data_dir)]
    try:
        proc = subprocess.run(command, cwd=str(REPO), capture_output=True, text=True, timeout=180)
    except (OSError, subprocess.SubprocessError) as exc:
        return None, "card-lint 启动失败：%s" % exc
    if proc.returncode not in (0, 1):
        return None, "card-lint exit %d：%s" % (proc.returncode, (proc.stderr or "").strip()[:200])
    try:
        payload = json.loads(proc.stdout)
        findings = payload.get("findings") or []
    except (ValueError, AttributeError) as exc:
        return None, "card-lint 输出不是 JSON：%s" % exc
    by_qid: dict[str, list[dict]] = {}
    for item in findings:
        if not isinstance(item, dict):
            continue
        qid = str(item.get("qid") or "")
        name = str(item.get("name") or "")
        if qid == "" or name == "":
            continue
        if str(item.get("rule") or "coverage") == "content":
            card = item.get("card") if item.get("card") else "?"
            if item.get("code") == "first-paragraph":
                note = "推送卡 %s 首段先讲怎么用，没先答「它是什么」（`%s`）" % (card, name)
            else:
                note = "推送卡只报了 `%s`，没给结论/签名（card=%s）" % (name, card)
            by_qid.setdefault(qid, []).append({
                "qid": qid, "code": "card-content", "level": "blocker", "field": "",
                "name": name, "note": note,
            })
            continue
        by_qid.setdefault(qid, []).append({
            "qid": qid, "code": "card", "level": "blocker", "field": "", "name": name,
            "note": "选项点名的引理没出现在推送卡里（card=%s）"
                    % (item.get("card") if item.get("card") else "fallback/no-card"),
        })
    return by_qid, ""


# ── 无卡兜底：fallback-card 必须含 why + 选项点名的引理名 ─────────────────────


def check_fallback(engine, data: dict, question: dict, candidates: set[str]) -> list[dict]:
    qid = str(question.get("id") or "?")
    result = engine.explain_for(engine.new_state(), data, qid)
    if result.get("kind") != "fallback-card":
        return []
    card = result.get("card") or {}
    text = card_text_units(card)  # 原始文本，不经过 json.dumps 转义（见上）
    out: list[dict] = []
    why = str(question.get("why") or "").strip()
    if why == "" or why not in text:
        out.append({"qid": qid, "code": "fallback", "level": "blocker", "field": "why",
                    "name": "", "note": "无卡兜底缺少题目 why"})
    for name in named_lemmas(question, candidates):
        if name not in text:
            out.append({"qid": qid, "code": "fallback", "level": "blocker", "field": "",
                        "name": name, "note": "无卡兜底缺少选项点名的引理 `%s`" % name})
    return out


# ── 单题判断 ──────────────────────────────────────────────────────────────────


def check_question(engine, data: dict, text_lint, terms, card_by_qid: dict[str, list[dict]],
                   candidates: set[str], question: dict) -> dict:
    qid = str(question.get("id") or "?")
    findings = check_question_text(text_lint, question, terms,
                                   has_question_mode=hasattr(text_lint, "check_question_expression"))
    findings.extend(card_by_qid.get(qid, []))
    findings.extend(check_fallback(engine, data, question, candidates))
    # 同一 (code,field,name) 去重（兼容兜底与真实 card-lint 可能命中同一条）。
    deduped: dict[tuple, dict] = {}
    for item in findings:
        key = (item["code"], item["field"], item["name"])
        deduped.setdefault(key, item)
    ordered = sorted(deduped.values(),
                     key=lambda item: (0 if item["level"] == "blocker" else 1,
                                       item["code"], item["field"], item["name"]))
    return {
        "qid": qid,
        "chapter": question.get("chapter"),
        "unit": question.get("unit"),
        "findings": ordered,
        "blockers": sum(1 for item in ordered if item["level"] == "blocker"),
        "warnings": sum(1 for item in ordered if item["level"] == "warning"),
    }


def finding_key(item: dict) -> str:
    """基线键：qid + code + 定位（文本=field / 卡片与兜底=name）。"""
    where = item.get("field") or item.get("name") or "-"
    return "%s:%s:%s" % (item["qid"], item["code"], where)


# ── learner 模式：算出接下来要发的 N 道题（deepcopy，绝不写回 state） ─────────


def load_state_json(path: Path) -> dict:
    try:
        state = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise PreconditionError("state 读不出（%s）：%s" % (path, exc)) from exc
    if not isinstance(state, dict):
        raise PreconditionError("state 不是 JSON 对象：%s" % path)
    return state


def resolve_state_path(explicit: str | None) -> Path | None:
    if explicit:
        path = Path(explicit).expanduser()
        if not path.exists():
            raise PreconditionError("显式 --state 不存在：%s" % path)
        return path
    env = os.environ.get("LESSON_PREFLIGHT_STATE")
    if env:
        path = Path(env).expanduser()
        if not path.exists():
            raise PreconditionError("$LESSON_PREFLIGHT_STATE 不存在：%s" % path)
        return path
    for candidate in DEFAULT_STATE_CANDIDATES:
        if candidate.exists():
            return candidate
    return None


def pending_questions(engine, data: dict, state: dict, limit: int,
                      chapter: str | None) -> list[tuple[dict, dict]]:
    """模拟 fast path 的取题：resume 优先，遇到卡/课/代码题就继续推进。

    `engine.next_step` 会改 state；这里传入 deepcopy，学习者的真实 state 不受影响。
    """
    work = copy.deepcopy(state)
    advance = False
    picked: list[tuple[dict, dict]] = []
    for _guard in range(max(8, limit * 6 + 20)):
        step = engine.next_step(work, data, advance=advance, chapter=chapter)
        kind = step.get("kind")
        if kind == "question":
            picked.append((step["question"], step.get("decision") or {}))
            if len(picked) >= limit:
                break
            advance = True
        elif kind in ("card", "lesson", "code"):
            advance = True
        else:
            break
    return picked


# ── content 模式：全库扫描 ────────────────────────────────────────────────────


def content_questions(data: dict, chapter: str | None) -> list[dict]:
    questions = data.get("questions") or []
    if chapter:
        questions = [question for question in questions if question.get("chapter") == chapter]
    return questions


# ── 基线（双向棘轮） ─────────────────────────────────────────────────────────


def load_baseline(path: Path) -> dict[str, dict]:
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    entries = payload.get("entries") or []
    return {str(entry["key"]): entry for entry in entries
            if isinstance(entry, dict) and isinstance(entry.get("key"), str)}


def check_baseline(blockers: list[dict], baseline: dict[str, dict]) -> tuple[list[dict], dict[str, dict]]:
    """（基线外新增, 已修未收紧的过期条目）——两者都判红。"""
    current = {finding_key(item): item for item in blockers}
    new = [item for key, item in current.items() if key not in baseline]
    stale = {key: entry for key, entry in baseline.items() if key not in current}
    return new, stale


def baseline_payload(blockers: list[dict]) -> dict:
    entries = sorted(
        ({"key": finding_key(item), "qid": item["qid"], "code": item["code"],
          "field": item.get("field") or "", "name": item.get("name") or ""}
         for item in blockers),
        key=lambda entry: entry["key"],
    )
    return {
        "schema": "soko.lesson-preflight-baseline/1",
        "note": (
            "会话预检 content 模式的存量 blocker 基线（键 = qid:code:定位）。"
            "只许减不许增：基线外新增判红，已修条目没同步收紧也判红。"
            "收紧用 `python3 scripts/lesson-preflight.py --all --update-baseline`。"
        ),
        "updated": "2026-11-29",
        "blockers": len(entries),
        "entries": entries,
    }


# ── 自检（红/绿 fixtures） ───────────────────────────────────────────────────


def _fixture_data(case: dict) -> dict:
    questions = case.get("questions") or []
    cards = case.get("cards") or []
    return {
        "questions": questions,
        "cards": cards,
        "by_id": {question["id"]: question for question in questions},
        "glossary": case.get("glossary") or [],
    }


def _fixture_findings(engine, text_lint, terms, candidates, data: dict, question: dict) -> list[str]:
    card_by_qid = compat_coverage(engine, data, candidates)
    result = check_question(engine, data, text_lint, terms, card_by_qid, candidates, question)
    return sorted(finding_key(item) for item in result["findings"])


def run_selftest() -> int:
    if not FIXTURES.exists():
        print("SELFTEST FAIL：找不到 %s" % FIXTURES, file=sys.stderr)
        return 3
    try:
        engine = load_engine()
        text_lint = load_text_lint()
    except PreconditionError as exc:
        print("SELFTEST FAIL：%s" % exc, file=sys.stderr)
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
        for name in fixtures.get("expected_candidates") or []:
            if name in candidates:
                passed += 1
            else:
                failures.append("声明解析：`%s` 应当是可点名引理，却没被收" % name)
        for name in fixtures.get("expected_not_candidates") or []:
            if name not in candidates:
                passed += 1
            else:
                failures.append("声明解析：`%s` 不该是可点名引理，却被收了" % name)
        for name in fixtures.get("expected_namespaces") or []:
            if name in namespaces:
                passed += 1
            else:
                failures.append("命名空间识别：`%s` 应当是命名空间" % name)

        terms_fixture = fixtures.get("terms") or {}
        terms = (
            tuple(terms_fixture.get("selfInvented") or DEFAULT_SELF_INVENTED),
            tuple(terms_fixture.get("metaphors") or DEFAULT_METAPHORS),
        )
        # 让 text_lint 的题目模式也读夹具词表（真数据词表变化不影响红/绿对照）。
        original_loader = getattr(text_lint, "load_expression_terms", None)
        if original_loader is not None:
            text_lint.load_expression_terms = lambda path=None: terms

        def token_checkable() -> None:
            nonlocal passed
            # 词表解析：兼容兜底的 X11/X12 必须真的咬住（否则红例不红）。
            unit = text_lint.Unit(file="fixture", id="fixture", field="prompt",
                                  text="落点 搬运", is_question=True)
            hits = compat_expression_findings(text_lint, unit, terms)
            codes = sorted(item["axiom"] for item in hits)
            if codes == ["X11", "X12"]:
                passed += 1
            else:
                failures.append("词表兜底：X11/X12 应当各咬一条，实际 %r" % codes)

        try:
            token_checkable()
            for case in fixtures.get("cases") or []:
                name = case.get("name") or "case"
                data = _fixture_data(case)
                got: list[str] = []
                for question in data["questions"]:
                    got.extend(_fixture_findings(engine, text_lint, terms, candidates, data, question))
                expected = sorted(case.get("expect") or [])
                if sorted(got) == expected:
                    passed += 1
                else:
                    failures.append("%s：期望 %r，实际 %r" % (name, expected, sorted(got)))
        finally:
            if original_loader is not None:
                text_lint.load_expression_terms = original_loader

    # 基线棘轮：新增判红 · 已修未收紧也判红（双向）。
    sample = [{"qid": "q1", "code": "card", "level": "blocker", "field": "", "name": "foo"}]
    baseline = {finding_key(sample[0]): sample[0]}
    new, stale = check_baseline(sample, baseline)
    if not new and not stale:
        passed += 1
    else:
        failures.append("基线棘轮：完全一致的 findings 不该有新增/过期")
    new, stale = check_baseline(
        sample + [{"qid": "q2", "code": "X11", "level": "blocker",
                   "field": "prompt", "name": ""}], baseline)
    if len(new) == 1 and not stale:
        passed += 1
    else:
        failures.append("基线棘轮：基线外新增必须判红")
    new, stale = check_baseline([], baseline)
    if not new and finding_key(sample[0]) in stale:
        passed += 1
    else:
        failures.append("基线棘轮：已修条目没收紧必须判红")

    if failures:
        print("SELFTEST FAIL")
        for failure in failures:
            print("  -", failure)
        print("（%d 条对照通过，%d 条失败）" % (passed, len(failures)))
        return 1
    print("SELFTEST OK（%d 条对照；红/绿 fixtures + 声明解析 + 词表 + 基线棘轮）" % passed)
    return 0


# ── 主流程 ───────────────────────────────────────────────────────────────────


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="会话预检：开始做题前检查即将发出的题（文本规则 + 卡片覆盖 + 无卡兜底）",
        formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--selftest", action="store_true", help="红/绿对照 fixtures")
    parser.add_argument("--json", action="store_true", help="机读输出（stdout）")
    parser.add_argument("--all", action="store_true", help="content 模式：扫全库（不看 state）")
    parser.add_argument("--state", default=None, metavar="PATH",
                        help="学习者 state 路径（默认自动找 fast path 的 state）")
    parser.add_argument("--data-dir", default=str(DATA), metavar="DIR",
                        help="题库数据目录（默认课程 knowledge/data）")
    parser.add_argument("--limit", type=int, default=3, metavar="N",
                        help="learner 模式预检接下来多少题（默认 3）")
    parser.add_argument("--chapter", default=None, metavar="CH",
                        help="只看这一章（如 I.9）")
    parser.add_argument("--card-lint", default=None, metavar="PATH",
                        help="card-lint.py 路径（默认自动发现 scripts/card-lint.py）")
    parser.add_argument("--baseline", default=None, metavar="PATH",
                        help="content 模式存量基线（默认 scripts/lesson-preflight-baseline.json）")
    parser.add_argument("--no-baseline", action="store_true", help="不按基线判，列出全部 blocker")
    parser.add_argument("--update-baseline", action="store_true", help="把当前 blocker 收紧成新基线")
    return parser


def _resolve_card_source(args) -> tuple[Path | None, str]:
    """返回（card-lint 脚本路径, 来源）：显式给的必须存在，否则 exit 3。"""
    if args.card_lint:
        path = Path(args.card_lint).expanduser()
        if not path.exists():
            raise PreconditionError("显式 --card-lint 不存在：%s" % path)
        return path, "script"
    if CARD_LINT_PATH.exists():
        return CARD_LINT_PATH, "script"
    return None, "compat"


def _print_finding(item: dict, indent: str = "  ") -> None:
    mark = "✗" if item["level"] == "blocker" else "!"
    where = item.get("field") or item.get("name") or ""
    print("%s%s %s %s%s" % (indent, mark, item["code"],
                            ("[%s] " % where) if where else "", item.get("note") or ""))


def _emit(payload: dict, args, human_lines: list[str]) -> None:
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return
    for line in human_lines:
        print(line)


def run(args) -> int:
    engine = load_engine()
    text_lint = load_text_lint()
    data_dir = Path(args.data_dir).expanduser()
    data = load_data(engine, data_dir)
    terms = load_term_lists()
    candidates, _namespaces = collect_declarations(COURSE)
    card_lint_path, card_source = _resolve_card_source(args)

    state_path = None if args.all else resolve_state_path(args.state)
    mode = "content" if state_path is None else "learner"
    degraded: list[str] = []
    card_by_qid: dict[str, list[dict]] | None = None

    if card_source == "script":
        card_by_qid, reason = run_card_lint(card_lint_path, data_dir)
        if card_by_qid is None:
            card_source = "compat"
            degraded.append("card-lint 降级为内置兼容检查：%s" % reason)
    if card_source == "compat":
        card_by_qid = compat_coverage(engine, data, candidates)
        if card_lint_path is None:
            degraded.append("card-lint.py 不存在（#146 未合并）：用内置兼容覆盖检查（同口径）")

    if not hasattr(text_lint, "check_question_expression"):
        degraded.append("text_lint 没有 question 模式（#149 未合并）：用内置兼容 X11/X12/X13")
    if not hasattr(text_lint, "question_text_units"):
        degraded.append("text_lint 缺少 question_text_units：自建 prompt/why/options 单元")

    if mode == "content":
        questions = content_questions(data, args.chapter)
        checked = [check_question(engine, data, text_lint, terms, card_by_qid,
                                  candidates, question) for question in questions]
    else:
        state = load_state_json(state_path)
        picked = pending_questions(engine, data, state, max(1, args.limit), args.chapter)
        checked = [check_question(engine, data, text_lint, terms, card_by_qid,
                                  candidates, question) for question, _decision in picked]
        if len(checked) < max(1, args.limit):
            degraded.append("state 里只推得出 %d 道后续题（请求 limit=%d）"
                            % (len(checked), args.limit))

    blockers = [item for result in checked for item in result["findings"]
                if item["level"] == "blocker"]
    warnings = [item for result in checked for item in result["findings"]
                if item["level"] == "warning"]

    baseline_path = Path(args.baseline).expanduser() if args.baseline else BASELINE
    use_baseline = mode == "content" and not args.no_baseline
    new: list[dict] = []
    stale: dict[str, dict] = {}
    if use_baseline:
        if args.update_baseline:
            payload = baseline_payload(blockers)
            baseline_path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                                     encoding="utf-8")
            print("lesson-preflight: 基线已写入 %s（%d 条存量）" % (baseline_path, len(blockers)))
            return 0
        baseline = load_baseline(baseline_path)
        new, stale = check_baseline(blockers, baseline)

    payload = {
        "schema": "soko.lesson-preflight/1",
        "mode": mode,
        "state": str(state_path) if state_path else None,
        "data_dir": str(data_dir),
        "limit": None if mode == "content" else max(1, args.limit),
        "chapter": args.chapter,
        "checked": len(checked),
        "blockers": len(blockers),
        "warnings": len(warnings),
        "card_lint": card_source,
        "terms": terms[2],
        "degraded": degraded,
        "questions": checked,
    }
    if use_baseline:
        payload["baseline"] = {
            "path": str(baseline_path),
            "new": new,
            "stale": list(stale.values()),
        }
    elif args.no_baseline:
        payload["baseline"] = None

    # 有基线时按「基线外新增 / 过期」判红（存量欠账冻结）；无基线时 blocker 本身就是红。
    ok = (not new and not stale) if use_baseline else (len(blockers) == 0)
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return 0 if ok else 1

    header = ("lesson-preflight: %s · %s · 检查 %d 题"
              % (mode, ("state=" + str(state_path)) if state_path else ("题库 " + str(len(checked)) + " 题"),
                 len(checked)))
    if args.chapter:
        header += " · chapter=" + args.chapter
    header += " · card-lint=" + card_source
    print(header)
    for message in degraded:
        print("  ! 降级：%s" % message)
    for result in checked:
        if not result["findings"]:
            continue
        print("  %s（blocker %d / warning %d）"
              % (result["qid"], result["blockers"], result["warnings"]))
        for item in result["findings"]:
            _print_finding(item, indent="    ")
    if use_baseline:
        if new:
            print("基线外新增（必须先修内容，或说明后收紧基线）：")
            for item in new:
                _print_finding(item)
        if stale:
            print("基线已过期（这些 blocker 已修，跑 --update-baseline 收紧）：")
            for entry in stale.values():
                print("  %s" % entry.get("key"))
    print("lesson-preflight: blockers %d / warnings %d%s"
          % (len(blockers), len(warnings),
             "（基线外新增 %d、过期 %d）" % (len(new), len(stale)) if use_baseline else ""))
    if ok:
        if use_baseline and blockers:
            print("⇒ 绿：0 条基线外新增（存量欠账已冻结，见 %s）" % baseline_path)
        else:
            print("⇒ 绿：0 blocker，可以出题")
    else:
        print("⇒ 红：有 blocker ⇒ 先修内容，别把坏题发给学习者")
    return 0 if ok else 1


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.limit < 1:
        print("lesson-preflight: --limit 必须是正整数", file=sys.stderr)
        return 2
    try:
        if args.selftest:
            return run_selftest()
        return run(args)
    except PreconditionError as exc:
        print("lesson-preflight: 前提缺失：%s ⇒ 不假装判过 ✗" % exc, file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
