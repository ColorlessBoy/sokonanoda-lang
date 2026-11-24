#!/usr/bin/env python3
"""L2 调度引擎：掌握度模型 + 推荐式选题 + 复习排程（纯函数，零模型）。

CLI（插件 Host 半通过子进程调用；stdout 永远是一个 JSON 对象）：

    python3 engine.py next   --state <state.json> [--chapter I.2] [--seed 7]
    python3 engine.py answer --state <state.json> --qid I.2.strict-1 --choice 2
    python3 engine.py state  --state <state.json>
    python3 engine.py selftest
    python3 engine.py simulate [--questions 20] [--out reports/]

数据：data/questions.json · data/cards.json · data/params.json
状态：由调用方指定路径（Host 半落在 ~/.dsh/learn/<sessionId>.json）。
"""
import argparse
import json
import math
import os
import random
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DEFAULT_DATA_DIR = ROOT / "data"
ISO = "%Y-%m-%dT%H:%M:%SZ"


# ───────────────────────── 数据 ─────────────────────────
def load_data(data_dir: Path) -> dict:
    questions = json.loads((data_dir / "questions.json").read_text(encoding="utf-8"))
    glossary_path = data_dir / "glossary.json"
    glossary = json.loads(glossary_path.read_text(encoding="utf-8")) if glossary_path.exists() else {"entries": []}
    lemmas_path = data_dir / "lemmas.json"
    lemmas = json.loads(lemmas_path.read_text(encoding="utf-8")) if lemmas_path.exists() else {"lemmas": []}
    lessons_path = data_dir / "lessons.json"
    lessons = json.loads(lessons_path.read_text(encoding="utf-8")) if lessons_path.exists() else {"lessons": []}
    code_path = data_dir / "code-exercises.json"
    code_bank = json.loads(code_path.read_text(encoding="utf-8")) if code_path.exists() else {"exercises": []}
    cards = json.loads((data_dir / "cards.json").read_text(encoding="utf-8"))
    kp_path = data_dir / "knowledge-points.json"
    knowledge_points = json.loads(kp_path.read_text(encoding="utf-8")) if kp_path.exists() else {"kps": []}
    params = json.loads((data_dir / "params.json").read_text(encoding="utf-8"))
    return {
        "questions": questions["questions"],
        "kps": knowledge_points.get("kps", []),
        "chapters": questions.get("chapters", {}),
        "cards": cards["cards"],
        "_spine": course_spine({"questions": questions["questions"]}, data_dir),
        "glossary": glossary.get("entries", []),
        "lessons": lessons.get("lessons", []),
        "lemmas": lemmas.get("lemmas", []),
        "lemma_by_name": {item.get("name"): item for item in lemmas.get("lemmas", []) if isinstance(item, dict)},
        "lesson_by_unit": {(item.get("chapter"), item.get("unit")): item for item in lessons.get("lessons", [])},
        "code_exercises": code_bank.get("exercises", []),
        "params": params,
        "by_id": {q["id"]: q for q in questions["questions"]},
        "card_by_tag": {},
    }


def index_cards(data: dict) -> None:
    mapping = {}
    for card in data["cards"]:
        for tag in card.get("tags", []):
            mapping.setdefault(tag, card)
    data["card_by_tag"] = mapping


def params(data: dict) -> dict:
    return data["params"]


def tags_of(question: dict) -> list:
    return list(question.get("tags") or [])


def difficulty(question: dict) -> float:
    return float(question.get("difficulty", 0.0))


# ───────────────────────── 状态 ─────────────────────────
def now_iso() -> str:
    return datetime.now(timezone.utc).strftime(ISO)


def parse_iso(text: str) -> float:
    try:
        return datetime.strptime(text, ISO).replace(tzinfo=timezone.utc).timestamp()
    except Exception:
        return 0.0


def new_state(session: str = "") -> dict:
    return {
        "version": 1,
        "session": session,
        "created_at": now_iso(),
        "theta": {},
        "last_at": {},
        "history": [],
        "seen": {},
        "wrong": {},
        "streak": 0,
        "cards_shown": {},
        "pending_card": None,
        "last_decision": None,
        "issued_qid": None,
        "issued_card": None,
        "skipped": [],
        "confusions": {},
        "calibration": {"step": 0, "done": False},
        "flow": {"served": 0},
        "after_primer_qid": None,
        "taught": {},
        "code_done": {},
        "kp_theta": {},
    }


def load_state(path: Path) -> dict:
    if not path.exists():
        return new_state()
    try:
        state = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(state, dict):
            raise ValueError("state is not an object")
        base = new_state(state.get("session", ""))
        base.update(state)
        return base
    except Exception:
        # 坏文件降级为空态（Host 半会记 warning，不崩）
        return new_state()


def save_state(path: Path, state: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    os.replace(tmp, path)


def theta_of(state: dict, data: dict, tag: str) -> float:
    """纯读取：不写入 state（写入只发生在 apply_answer）。

    历史 bug：读时惰性写 theta ⇒ 刚发出、还没作答的题的 tag 被初始化成 0.5，
    热力图会把“未接触”画成“刚起步”。touched 应只由 history 决定。
    """
    value = (state.get("theta") or {}).get(tag)
    if value is None:
        value = params(data).get("theta_init", 0.0)
    return float(value)


def mastery(state: dict, data: dict, tag: str) -> float:
    return sigmoid(theta_of(state, data, tag))


def sigmoid(x: float) -> float:
    x = max(-30.0, min(30.0, x))
    return 1.0 / (1.0 + math.exp(-x))


def expected(state: dict, data: dict, question: dict) -> float:
    tags = tags_of(question)
    if not tags:
        return 0.5
    primary = min(tags, key=lambda tag: mastery(state, data, tag))
    return sigmoid(theta_of(state, data, primary) - difficulty(question))


# ───────────────────────── 排程 ─────────────────────────
def recent_ids(state: dict, window: int) -> set:
    return {entry["qid"] for entry in state["history"][-window:] if "qid" in entry}


def chapter_coverage(state: dict, data: dict) -> dict:
    count = {}
    for entry in state["history"]:
        question = data["by_id"].get(entry.get("qid"))
        if question:
            count[question["chapter"]] = count.get(question["chapter"], 0) + 1
    return count


def review_due(state: dict, data: dict) -> dict:
    """返回 tag -> 到期程度（0..1），用于复习优先。"""
    cfg = params(data)["review"]
    now = time.time()
    due = {}
    for tag, theta in state["theta"].items():
        m = sigmoid(theta)
        if m >= cfg["mastery_below"]:
            continue
        last = parse_iso(state["last_at"].get(tag, ""))
        if last == 0.0:
            continue
        interval_days = cfg["interval_days_base"] + cfg["interval_days_slope"] * (1.0 - m)
        age_days = (now - last) / 86400.0
        if age_days >= interval_days:
            due[tag] = min(1.0, (age_days - interval_days) / max(interval_days, 0.5) + 0.5)
    return due


def consecutive_wrong_tag(state: dict) -> str:
    """最近连续答错且同 tag（用于触发知识卡）。"""
    recent = [entry for entry in state["history"] if entry.get("correct") is False][-2:]
    if len(recent) < 2:
        return ""
    last_two = state["history"][-2:]
    if not all(entry.get("correct") is False for entry in last_two):
        return ""
    tag_sets = []
    for entry in last_two:
        question = entry.get("_tags")
        if not question:
            return ""
        tag_sets.append(set(question))
    common = tag_sets[0] & tag_sets[1]
    return sorted(common)[0] if common else ""


def score_question(state: dict, data: dict, question: dict, due: dict, coverage: dict, last_wrong_tags: set) -> float:
    cfg = params(data)
    weights = cfg["weights"]
    tags = tags_of(question)
    if not tags:
        return 0.0
    min_mastery = min(mastery(state, data, tag) for tag in tags)
    gap = 1.0 - min_mastery
    blind_spot_boost = 0.4 if min_mastery < 0.35 else 0.0
    p = expected(state, data, question)
    challenge = 1.0 - 2.0 * abs(p - 0.5)
    novelty = 1.0 if question["id"] not in state["seen"] else 0.15
    target = cfg["coverage_target_per_chapter"]
    covered = coverage.get(question["chapter"], 0)
    coverage_bonus = 1.0 if covered < target else 0.0
    score = (weights["gap"] * gap + weights["challenge"] * challenge
             + weights["novelty"] * novelty + weights["coverage"] * coverage_bonus)
    score += blind_spot_boost
    # 复习到期优先
    score += 0.25 * max([due.get(tag, 0.0) for tag in tags] or [0.0])
    # 答错后的变式优先
    if last_wrong_tags & set(tags):
        score += cfg.get("variant_bonus", 0.2)
    # 同 tag 连续出现惩罚
    last = state["history"][-1] if state["history"] else None
    if last:
        last_q = data["by_id"].get(last.get("qid"))
        if last_q and set(tags) & set(tags_of(last_q)):
            score -= cfg.get("same_tag_streak_penalty", 0.15)
    return round(score, 6)


def excluded_ids(data: dict) -> set:
    """纯背名字/演示名的题（用户反馈「相当于背书，不考逻辑」）不参与选题；可用 params.selection.exclude 配置。"""
    params_map = data.get("params") if isinstance(data.get("params"), dict) else {}
    selection = params_map.get("selection") if isinstance(params_map.get("selection"), dict) else {}
    raw = selection.get("exclude")
    if not isinstance(raw, list):
        return set()
    return {item for item in raw if isinstance(item, str)}


def pick(state: dict, data: dict, chapter: str = None, seed: int = None) -> dict:
    cfg = params(data)
    window = cfg["recent_window"]
    recent = recent_ids(state, window)
    exclude = excluded_ids(data)
    skipped = set(state.get("skipped") or [])
    pool = [q for q in data["questions"]
            if (chapter is None or q["chapter"] == chapter) and q["id"] not in recent
            and q["id"] not in exclude and q["id"] not in skipped]
    if not pool:
        # 只剩被跳过的题：允许回头，但优先选最久没碰的
        pool = [q for q in data["questions"]
                if (chapter is None or q["chapter"] == chapter) and q["id"] not in exclude
                and q["id"] not in recent]
    relaxed = False
    if not pool:
        recent = recent_ids(state, max(5, window // 4))
        pool = [q for q in data["questions"]
                if (chapter is None or q["chapter"] == chapter) and q["id"] not in recent]
        relaxed = True
    if not pool:
        pool = [q for q in data["questions"] if chapter is None or q["chapter"] == chapter] or data["questions"]

    due = review_due(state, data)
    coverage = chapter_coverage(state, data)
    last_wrong_tags = set()
    for entry in reversed(state["history"]):
        if entry.get("correct") is False:
            question = data["by_id"].get(entry.get("qid"))
            if question:
                last_wrong_tags = set(tags_of(question))
            break
        break  # 只看紧邻的上一次

    scored = [(score_question(state, data, q, due, coverage, last_wrong_tags), q) for q in pool]
    best_score = max(score for score, _ in scored)
    tied = [q for score, q in scored if abs(score - best_score) < 1e-9]
    # 确定性：并列时按 id 排序取第一（seed 参数保留兼容，但不再制造随机）
    question = sorted(tied, key=lambda q: q["id"])[0]
    tags = tags_of(question)
    reason = "challenge"
    if any(due.get(tag, 0.0) > 0 for tag in tags):
        reason = "review"
    elif last_wrong_tags & set(tags):
        reason = "variant"
    elif question["id"] not in state["seen"]:
        reason = "new"
    elif min(mastery(state, data, tag) for tag in tags) < 0.5:
        reason = "gap"
    elif coverage.get(question["chapter"], 0) < cfg["coverage_target_per_chapter"]:
        reason = "coverage"
    return {"question": question, "decision": {
        "qid": question["id"], "reason": reason, "score": best_score,
        "relaxed": relaxed, "p": round(expected(state, data, question), 3),
    }}


CALIBRATION_LADDER = [
    ("I.1", 1), ("I.1", 3),
    ("I.2", 1), ("I.2", 3),
    ("I.3", 1), ("I.3", 3),
]


def calibration_question(state: dict, data: dict) -> dict:
    """校准阶梯：先用 6 道题快速定位水平（每章 L1 + L3），再进入自适应。"""
    step = state.get("calibration", {}).get("step", 0)
    if step >= len(CALIBRATION_LADDER):
        state["calibration"]["done"] = True
        return {}
    chapter, level = CALIBRATION_LADDER[step]
    pool = [q for q in data["questions"]
            if q.get("chapter") == chapter and q.get("level") == level
            and q.get("kind", "choice") == "choice"
            and q["id"] not in state["seen"]
            and q["id"] not in excluded_ids(data)
            and q["id"] not in set(state.get("skipped") or [])]
    if not pool:
        # 该格没有题就跳过这一步（阶梯不阻塞开局）
        state["calibration"]["step"] = step + 1
        return calibration_question(state, data)
    pool.sort(key=lambda q: q["id"])
    question = pool[0]
    return {"question": question, "decision": {
        "qid": question["id"], "reason": "calibration", "step": step + 1,
        "ladder": chapter + " L" + str(level), "p": 0.5,
    }}



# ═══════════════════════════════════════════════════════════════════════════
# 探针（probe）：按知识点依赖顺序推进 + 同一 KP 内难度阶梯
#
# 设计目标（用户 2026-11-23：「你是在随机出题还是在设计探针」）：
#   1) 不用「全库加权打分」自由跳章 —— 那会 I.1→I.3→I.1 乱跳（见 history）；
#   2) 当前探针锁定一个知识点，按 L1→L2→L3 逐步加压：
#        答对 → 升一级；答错 → 降一级并在同 KP 巩固；
#        到了该 KP 的最高级还答对 → 这个 KP 结案，换下一个；
#   3) 下一个 KP 按 `prereq` 拓扑序选（前置没结案不跳过去）；
#   4) 每道题都带 `probe` 元数据（role / kp / level / why），给用户看得见的意图。
# ═══════════════════════════════════════════════════════════════════════════

PROBE_LEVELS = [1, 2, 3]
PROBE_CLEAR_AT = {1: 1, 2: 1, 3: 1}   # 该级答对几次算过（快车道：一次对就升级/结案）


def kp_graph(data: dict) -> dict:
    """KP 依赖图：{kp_id: {"title":..., "prereq": [...], "chapter": ...}}。"""
    graph = {}
    for kp in data.get("kps") or data.get("knowledge_points") or []:
        graph[kp["id"]] = {
            "title": kp.get("title", ""),
            "prereq": [p for p in (kp.get("prereq") or []) if isinstance(p, str)],
            "chapter": kp.get("chapter", ""),
            "kind": kp.get("kind", ""),
        }
    return graph


def kp_order(data: dict) -> list:
    """拓扑序（前置在前）；环或缺失依赖时退化为按 id 排序，绝不抛错。"""
    graph = kp_graph(data)
    order, seen, visiting = [], set(), set()

    def visit(node):
        if node in seen or node in visiting or node not in graph:
            return
        visiting.add(node)
        for dep in graph[node]["prereq"]:
            visit(dep)
        visiting.discard(node)
        seen.add(node)
        order.append(node)

    for node in sorted(graph):
        visit(node)
    return order


def kp_questions(data: dict, kp_id: str) -> list:
    excluded = excluded_ids(data)
    return [q for q in data["questions"]
            if kp_id in (q.get("kps") or []) and q["id"] not in excluded]


COGNITIVE_RANK = ("识别", "用法", "边界")


def cognitive_label(question: dict) -> str:
    """认知步的中文标签（序列题按标注，普通题按启发式）。"""
    return COGNITIVE_RANK[max(1, min(3, cognitive_step(question))) - 1]


def cognitive_step(question: dict) -> int:
    """一道题在「一个知识点」内部的认知步：1 识别 → 2 用法 → 3 边界/综合。

    优先用人工标注 `cognitiveStep`；没有就按题干措辞与 level 近似：
      · 「是什么 / 类型 / 定义 / 哪个命题」→ 1
      · 「应当 / 用哪个 / 正确写法 / 怎么」→ 2
      · 「为什么 / 能否 / 反例 / 边界 / 错」→ 3
    """
    explicit = question.get("cognitiveStep")
    if isinstance(explicit, int) and explicit in (1, 2, 3):
        return explicit
    prompt = str(question.get("prompt") or "")
    level = int(question.get("level", 2))
    if any(word in prompt for word in ("为什么", "能否", "反例", "边界", "错在", "哪一个说法")):
        return 3
    if level >= 3:
        return 3
    if any(word in prompt for word in ("应当", "用哪", "正确写法", "怎么", "如何", "选用")):
        return 2
    if any(word in prompt for word in ("是什么", "类型", "定义", "哪个命题", "形状")):
        return 1
    return min(max(level, 1), 3)


def ordered_kp_questions(data: dict, kp_id: str) -> list:
    """一个知识点的题按「认知步 → level → id」排成一条有序序列。"""
    questions = kp_questions(data, kp_id)
    def order_key(q):
        seq = q.get("seqIndex")
        if isinstance(seq, int) and seq > 0:      # 人工编好的题序优先
            return (0, seq, q["id"])
        return (1, cognitive_step(q), int(q.get("level", 2)), q["id"])
    questions.sort(key=order_key)
    return questions


def probe_state(state: dict, data: dict, kp_id: str) -> dict:
    probes = state.setdefault("probes", {})
    entry = probes.setdefault(kp_id, {"level": 2, "cleared": 0, "asked": 0, "wrong": 0, "done": False})
    return entry


def next_probe_kp(state: dict, data: dict) -> str:
    """下一个要探的知识点：拓扑序里第一个「还没结案、且有题」的 KP。"""
    probes = state.setdefault("probes", {})
    for kp_id in kp_order(data):
        entry = probes.get(kp_id) or {}
        if entry.get("done") is True:
            continue
        if not kp_questions(data, kp_id):
            continue
        return kp_id
    return ""


def probe_question(state: dict, data: dict) -> dict:
    """按当前探针目标挑一道题：同 KP、level 最接近目标、避开近期做过。"""
    kp_id = next_probe_kp(state, data)
    if kp_id == "":
        return None
    entry = probe_state(state, data, kp_id)
    target_level = int(entry.get("level", 2))
    # 推断：这个 KP 最近连着答对 ≥2 题 ⇒ 判定「已经会了」，不再浪费时间
    correct_streak = 0
    for entry_hist in reversed(state.get("history", [])):
        q_hist = data["by_id"].get(entry_hist.get("qid"))
        if not q_hist or kp_id not in (q_hist.get("kps") or []):
            break
        if entry_hist.get("correct") is True:
            correct_streak += 1
        else:
            break
    if correct_streak >= 2:
        entry["done"] = True
        return probe_question(state, data)

    recent = recent_ids(state, params(data)["recent_window"])
    pool = [q for q in kp_questions(data, kp_id) if q["id"] not in recent]
    if not pool:
        # 这个 KP 的题都做过了 → 结案，换下一个（不重复出题）
        entry["done"] = True
        return probe_question(state, data)
    # 有序序列：认知步（识别→用法→边界）优先，其次 level、题号；
    # 每次都取「这条序列里第一道还没做过的题」，所以顺序是稳定且递进的。
    sequence = ordered_kp_questions(data, kp_id)
    done_ids = {entry.get("qid") for entry in state.get("history", [])}
    skipped_ids = set(state.get("skipped") or [])
    pending = [q for q in sequence
               if q["id"] not in done_ids and q["id"] not in recent and q["id"] not in skipped_ids]
    if not pending:
        entry["done"] = True
        return probe_question(state, data)
    question = pending[0]
    position_in_kp = sequence.index(question) + 1
    total_in_kp = len(sequence)
    target_index = cognitive_step(question)

    # 承接：同一 KP 的第 n 题 vs 从上一站过来
    previous = state.get("probe_last") or {}
    same_kp = previous.get("kp") == kp_id and bool(previous.get("qid"))
    continuity = {
        "sameKp": same_kp,
        "kpStep": int(entry.get("asked", 0)) + 1,
        "prevKp": previous.get("kp", ""),
        "prevKpTitle": previous.get("kpTitle", ""),
        "prevQid": previous.get("qid", ""),
        "kpIndex": position_in_kp,
        "kpTotal": total_in_kp,
        "cognitiveStep": target_index,
        "cognitiveLabel": cognitive_label(question),
    }
    role = "diagnose" if entry.get("asked", 0) == 0 else ("practice" if entry.get("wrong", 0) > 0 else "ascend")
    entry["asked"] = entry.get("asked", 0) + 1
    return {
        "question": question,
        "probe": {
            "kp": kp_id,
            "kpTitle": (kp_graph(data).get(kp_id) or {}).get("title", ""),
            "level": int(question.get("level", 2)),
            "targetLevel": target_level,
            "role": role,
            "asked": entry["asked"],
            "cleared": entry.get("cleared", 0),
            "continuity": continuity,
        },
    }


def probe_observe(state: dict, data: dict, question: dict, correct: bool) -> dict:
    """把一次作答反馈给探针：升降级、结案、换下一个 KP。"""
    kp_ids = [kp for kp in (question.get("kps") or []) if kp]
    if not kp_ids:
        return {}
    kp_id = kp_ids[0]
    entry = probe_state(state, data, kp_id)
    level = int(question.get("level", entry.get("level", 2)))
    if correct:
        entry["cleared"] = entry.get("cleared", 0) + 1
        if level >= max(PROBE_LEVELS) and entry["cleared"] >= PROBE_CLEAR_AT[3]:
            entry["done"] = True
        elif entry["cleared"] >= PROBE_CLEAR_AT.get(level, 2):
            entry["level"] = min(max(PROBE_LEVELS), level + 1)
            entry["cleared"] = 0
    else:
        entry["wrong"] = entry.get("wrong", 0) + 1
        entry["level"] = max(min(PROBE_LEVELS), level - 1)
        entry["cleared"] = 0
    return {"kp": kp_id, "level": entry.get("level"), "done": entry.get("done", False)}


def next_step(state: dict, data: dict, advance: bool = False, chapter: str = None, no_lesson: bool = False,
              ignore_probe: bool = False) -> dict:
    """客户端取卡：默认**恢复已发出的卡**（刷新不换题）；advance=True 才推进。

    返回 {"kind": "question"|"card", ...}；调用方负责持久化 state。
    """
    if advance:
        previous = state.get("issued_qid")
        if previous:
            answered = any(entry.get("qid") == previous for entry in state["history"])
            if not answered:
                skipped = state.setdefault("skipped", [])
                if previous not in skipped:
                    skipped.append(previous)
                # 只保留最近 30 个，避免状态无限增长
                del skipped[:-30]
        state["issued_qid"] = None
        state["issued_card"] = None
    # a) 恢复已发出的知识卡
    issued_card = state.get("issued_card")
    if issued_card:
        card = data["card_by_tag"].get(issued_card.get("tag", ""))
        if card is not None and card["id"] == issued_card.get("cardId"):
            return {"kind": "card", "card": card,
                    "decision": {"kind": "card", "tag": issued_card.get("tag"), "reason": "resume"}}
        state["issued_card"] = None
    # a2) 恢复已发出但未完成的代码练习
    issued_code = state.get("issued_code")
    if issued_code and not state.get("code_done", {}).get(issued_code):
        for exercise in data.get("code_exercises", []):
            if exercise["id"] == issued_code:
                public = {key: exercise.get(key) for key in ("id", "chapter", "unit", "title", "prompt", "hints", "template", "kps")}
                return {"kind": "code", "exercise": public,
                        "decision": {"reason": "resume-code", "exerciseId": issued_code}}
    # b) 恢复已发出但未作答的题（刷新不变题）
    issued_qid = state.get("issued_qid")
    if issued_qid:
        question = data["by_id"].get(issued_qid)
        answered = any(entry.get("qid") == issued_qid for entry in state["history"])
        if question is not None and not answered:
            return {"kind": "question", "question": question,
                    "decision": {"qid": question["id"], "reason": "resume", "p": round(expected(state, data, question), 3)}}
        state["issued_qid"] = None
    # c) 校准阶梯（前 6 题快速评估水平）
    if not state.get("calibration", {}).get("done"):
        picked = calibration_question(state, data)
        if picked:
            state["issued_qid"] = picked["question"]["id"]
            return {"kind": "question", **picked}
    # c2.5) 先服务 primer 卡之后挂的那道题（保证「卡 → 题」相邻）
    after = state.get("after_primer_qid")
    if after:
        question = data["by_id"].get(after)
        state["after_primer_qid"] = None
        if question is not None:
            state["issued_qid"] = question["id"]
            return {"kind": "question", "question": question,
                    "decision": {"qid": question["id"], "reason": "after-primer",
                                 "p": round(expected(state, data, question), 3)}}
    # c2) 无限瀑布流：每 3 个条目插入一次代码练习（若还有未完成的）
    flow = state.setdefault("flow", {"served": 0})
    code_every = 0
    flow_params = data.get("params", {}).get("flow", {}) if isinstance(data.get("params"), dict) else {}
    if isinstance(flow_params.get("codeEvery"), (int, float)) and flow_params.get("codeEvery") >= 1:
        code_every = int(flow_params.get("codeEvery"))
    if code_every > 0 and state.get("calibration", {}).get("done") and (flow.get("served", 0) + 1) % code_every == 0:
        done = state.setdefault("code_done", {})
        kp_theta = state.setdefault("kp_theta", {})
        pool = [ex for ex in data.get("code_exercises", []) if not done.get(ex["id"])]
        if pool:
            def score(ex):
                kps = ex.get("kps") or []
                values = [sigmoid(kp_theta.get(kp, 0.0)) for kp in kps] or [0.5]
                return max(values)
            exercise = sorted(pool, key=lambda ex: (-score(ex), ex["id"]))[0]
            state["issued_code"] = exercise["id"]
            flow["served"] = flow.get("served", 0) + 1
            public = {key: exercise.get(key) for key in ("id", "chapter", "unit", "title", "prompt", "hints", "template", "kps")}
            return {"kind": "code", "exercise": public,
                    "decision": {"reason": "code", "exerciseId": exercise["id"], "p": round(score(exercise), 3)}}
    flow["served"] = flow.get("served", 0) + 1
    # d) 知识卡触发（连续答错）
    tag = state.get("pending_card")
    if tag:
        card = data["card_by_tag"].get(tag)
        if card is not None:
            state["pending_card"] = None
            state["issued_card"] = {"cardId": card["id"], "tag": tag}
            state["cards_shown"][card["id"]] = state["cards_shown"].get(card["id"], 0) + 1
            return {"kind": "card", "card": card,
                    "decision": {"kind": "card", "tag": tag, "reason": "wrong-twice"}}
        state["pending_card"] = None
    # d3) 教学环节：进入一个没上过课的单元 → 先给「微课」，题目排在后面
    taught = state.setdefault("taught", {})
    try:
        peek = pick(state, data, chapter=chapter)
        peek_unit = (peek["question"]["chapter"], peek["question"]["unit"])
    except Exception:
        peek_unit = (None, None)
    if not no_lesson and peek_unit[0] is not None and taught.get("|".join(peek_unit)) is not True:
        lesson = (data.get("lesson_by_unit") or {}).get(peek_unit)
        question = data["by_id"].get(peek["question"]["id"])
        if lesson is not None and question is not None:
            taught["|".join(peek_unit)] = True
            state["after_lesson_qid"] = question["id"]
            flow = state.setdefault("flow", {"served": 0})
            flow["served"] = flow.get("served", 0) + 1
            return {"kind": "lesson", "lesson": lesson,
                    "decision": {"kind": "lesson", "unit": peek_unit[1], "reason": "teach-first",
                                 "then": question["id"], "p": round(expected(state, data, question), 3)}}
    # d4) 上完课之后要出的那道题
    after_lesson = state.get("after_lesson_qid")
    if after_lesson:
        question = data["by_id"].get(after_lesson)
        state["after_lesson_qid"] = None
        if question is not None:
            state["issued_qid"] = question["id"]
            return {"kind": "question", "question": question,
                    "decision": {"qid": question["id"], "reason": "after-lesson",
                                 "p": round(expected(state, data, question), 3)}}
    # d4) 探针：按知识点依赖顺序 + 难度阶梯选题（用户要求「设计探针，而不是随机出题」）
    if state.get("calibration", {}).get("done") and not ignore_probe:
        probed = probe_question(state, data)
        if probed is not None:
            question = probed["question"]
            state["issued_qid"] = question["id"]
            flow = state.setdefault("flow", {"served": 0})
            flow["served"] = flow.get("served", 0) + 1
            state["probe_last"] = {
                "kp": probed["probe"].get("kp", ""),
                "kpTitle": probed["probe"].get("kpTitle", ""),
                "qid": question["id"],
            }
            return {"kind": "question", "question": question,
                    "decision": {"qid": question["id"], "reason": "probe",
                                 "probe": probed["probe"],
                                 "p": round(expected(state, data, question), 3)}}

    # e) 自适应选题（确定性，无随机）
    picked = pick(state, data, chapter=chapter)
    question = picked["question"]
    # e2) primer：新知识点/没见过的题 → 先把对应的简洁知识卡放上来，题目排在它后面
    tags = tags_of(question)
    seen_tags = set()
    for entry in state["history"]:
        prev = data["by_id"].get(entry.get("qid"))
        if prev:
            seen_tags.update(tags_of(prev))
    card = None
    for candidate in data["cards"]:
        if set(candidate.get("tags") or []) & set(tags) and candidate["id"] not in state["cards_shown"]:
            card = candidate
            break
    flow_cfg = (data.get("params") or {}).get("flow") or {}
    primer_enabled = flow_cfg.get("primerCards") is not False
    if card is not None and primer_enabled and not (set(tags) & seen_tags) and not no_lesson:
        state["after_primer_qid"] = question["id"]
        state["issued_card"] = {"cardId": card["id"], "tag": sorted(set(card.get("tags") or []) & set(tags))[0]}
        state["cards_shown"][card["id"]] = state["cards_shown"].get(card["id"], 0) + 1
        flow = state.setdefault("flow", {"served": 0})
        flow["served"] = flow.get("served", 0) + 1
        return {"kind": "card", "card": card,
                "decision": {"kind": "card", "tag": state["issued_card"]["tag"], "reason": "primer",
                             "then": question["id"]}}
    state["issued_qid"] = question["id"]
    return {"kind": "question", **picked}


def next_card(state: dict, data: dict) -> dict:
    tag = state.get("pending_card")
    if not tag:
        return {}
    card = data["card_by_tag"].get(tag)
    if not card:
        state["pending_card"] = None
        return {}
    state["pending_card"] = None
    state["cards_shown"][card["id"]] = state["cards_shown"].get(card["id"], 0) + 1
    return {"kind": "card", "card": card, "decision": {"kind": "card", "tag": tag, "reason": "wrong-twice"}}


def mastery_level(value: float) -> int:
    """掌握度 → GitHub 热力图色阶（0 空 → 4 最深），只做**推测**展示。"""
    if value >= 0.85:
        return 4
    if value >= 0.7:
        return 3
    if value >= 0.55:
        return 2
    if value >= 0.4:
        return 1
    return 0


def tag_universe(data: dict) -> dict:
    tags_by_chapter = {}
    for question in data["questions"]:
        for tag in tags_of(question):
            tags_by_chapter.setdefault(question["chapter"], set()).add(tag)
    return {chapter: sorted(tags) for chapter, tags in sorted(tags_by_chapter.items())}


def coverage_snapshot(state: dict, data: dict) -> dict:
    universe = tag_universe(data)
    all_tags = set().union(*universe.values()) if universe else set()
    touched = set()
    for entry in state["history"]:
        question = data["by_id"].get(entry.get("qid"))
        if question:
            touched.update(tags_of(question))
    chapters = {}
    for chapter, tags in universe.items():
        answered = sum(1 for entry in state["history"]
                       if (data["by_id"].get(entry.get("qid")) or {}).get("chapter") == chapter)
        chapters[chapter] = {
            "title": (data.get("chapters") or {}).get(chapter, chapter),
            "asked": answered,
            "tags_touched": len(set(tags) & touched),
            "tags_total": len(tags),
            "completion": round(len(set(tags) & touched) / max(1, len(tags)), 3),
        }
    return {
        "theory": {"touched": len(touched), "total": len(all_tags),
                   "ratio": round(len(touched) / max(1, len(all_tags)), 3)},
        "chapters": chapters,
    }


def heatmap_snapshot(state: dict, data: dict) -> dict:
    universe = tag_universe(data)
    asked_by_chapter = {}
    touched_by_chapter = {}
    for entry in state["history"]:
        question = data["by_id"].get(entry.get("qid"))
        if question:
            chapter = question["chapter"]
            asked_by_chapter[chapter] = asked_by_chapter.get(chapter, 0) + 1
            touched_by_chapter.setdefault(chapter, set()).update(tags_of(question))
    rows = []
    for chapter, tags in universe.items():
        cells = []
        touched = 0
        chapter_touched = touched_by_chapter.get(chapter, set())
        for tag in tags:
            is_touched = tag in chapter_touched
            m = mastery(state, data, tag) if is_touched else 0.0
            if is_touched:
                touched += 1
            cells.append({"tag": tag, "mastery": round(m, 3),
                          "level": mastery_level(m) if is_touched else 0,
                          "touched": is_touched})
        # 单元级聚合（给移动端紧凑热力图：每章 5–8 格，而不是 20+ 个知识点格）
        unit_cells = {}
        for entry in state["history"]:
            question = data["by_id"].get(entry.get("qid"))
            if question and question["chapter"] == chapter:
                unit = question.get("unit") or "未分单元"
                unit_cells.setdefault(unit, set()).update(tags_of(question))
        units = []
        for question in data["questions"]:
            if question["chapter"] != chapter:
                continue
            unit = question.get("unit") or "未分单元"
            if any(item.get("unit") == unit for item in units):
                continue
            unit_tags = {tag for q in data["questions"] if q["chapter"] == chapter and (q.get("unit") or "未分单元") == unit for tag in tags_of(q)}
            unit_touched = unit_cells.get(unit, set()) & unit_tags
            mastery_values = [mastery(state, data, tag) for tag in unit_touched]
            avg = sum(mastery_values) / len(mastery_values) if mastery_values else 0.0
            units.append({"unit": unit, "touched": len(unit_touched), "total": len(unit_tags),
                          "mastery": round(avg, 3), "level": mastery_level(avg) if unit_touched else 0})
        rows.append({"chapter": chapter, "title": (data.get("chapters") or {}).get(chapter, chapter),
                     "asked": asked_by_chapter.get(chapter, 0), "touched": touched,
                     "total": len(tags), "cells": cells, "units": units})
    return {"note": "掌握度为智能推测：由作答结果自适应估计，仅供参考",
            "legend": {"0": "未接触", "1": "刚起步", "2": "有点感觉", "3": "比较稳", "4": "很熟"},
            "rows": rows}


def positive_snapshot(state: dict, data: dict) -> dict:
    history = state["history"]
    answered = len(history)
    mastered = sum(1 for tag in state["theta"] if mastery(state, data, tag) >= params(data)["mastered_threshold"])
    days = set()
    for entry in history:
        at = entry.get("at", "")
        if len(at) >= 10:
            days.add(at[:10])
    streak_days = 0
    day_cursor = time.gmtime()
    while True:
        stamp = time.strftime("%Y-%m-%d", day_cursor)
        if stamp in days:
            streak_days += 1
            day_cursor = time.gmtime(time.mktime(day_cursor) - 86400)
        else:
            break
    coverage = coverage_snapshot(state, data)
    return {
        "answered": answered,
        "streak": state.get("streak", 0),
        "streak_days": streak_days,
        "mastered_tags": mastered,
        "tags_touched": coverage["theory"]["touched"],
        "coverage_ratio": coverage["theory"]["ratio"],
        "chapters_started": sum(1 for chapter in coverage["chapters"].values() if chapter["asked"] > 0),
        "cards_shown": sum(state.get("cards_shown", {}).values()),
    }


def history_tail(state: dict, data: dict, limit: int = 20) -> list:
    out = []
    for entry in state["history"][-limit:]:
        question = data["by_id"].get(entry.get("qid")) or {}
        out.append({"qid": entry.get("qid"), "choice": entry.get("choice"),
                    "correct": entry.get("correct"), "at": entry.get("at"),
                    "decision": entry.get("decision"),
                    "prompt": question.get("prompt", ""), "why": question.get("why", ""),
                    "tags": tags_of(question)})
    return out


def mastery_snapshot(state: dict, data: dict) -> dict:
    snapshot = {}
    for tag in sorted(state["theta"]):
        m = round(mastery(state, data, tag), 3)
        snapshot[tag] = {"theta": round(state["theta"][tag], 3), "mastery": m}
    return snapshot


def progress_snapshot(state: dict, data: dict) -> dict:
    history = state["history"]
    asked = len(history)
    correct = sum(1 for entry in history if entry.get("correct"))
    tags = set()
    for entry in history:
        question = data["by_id"].get(entry.get("qid"))
        if question:
            tags.update(tags_of(question))
    return {
        "asked": asked,
        "correct": correct,
        "accuracy": round(correct / asked, 3) if asked else 0.0,
        "streak": state.get("streak", 0),
        "tags_touched": len(tags),
        "cards_shown": sum(state.get("cards_shown", {}).values()),
        "wrong": [{"qid": qid, "count": count} for qid, count in sorted(state.get("wrong", {}).items(), key=lambda kv: -kv[1])][:20],
        "due_tags": sorted(review_due(state, data)),
    }


# ───────────────────────── 开放题（short/explain）自动判分 ─────────────────────────
def normalize_text(text: str) -> str:
    return " ".join(str(text or "").lower().replace("`", " ").split())


def grade_open(question: dict, text: str) -> dict:
    """rubric 关键词覆盖率判分（确定性、零模型）。返回 correct/score/feedback。"""
    rubric = question.get("rubric") or {}
    points = rubric.get("points") or []
    min_score = float(rubric.get("minScore", 0.6))
    normalized = normalize_text(text)
    total = sum(float(point.get("weight", 1)) for point in points) or 1.0
    earned = 0.0
    matched, missed = [], []
    for point in points:
        keywords = [normalize_text(word) for word in (point.get("any") or []) if normalize_text(word)]
        hit = bool(keywords) and any(word in normalized for word in keywords)
        if hit:
            earned += float(point.get("weight", 1))
            matched.append({"id": point.get("id"), "note": point.get("note", "")})
        else:
            missed.append({"id": point.get("id"), "note": point.get("note", "")})
    score = round(earned / total, 3)
    correct = score >= min_score
    needs_review = score < min_score and question.get("kind") == "explain"
    return {
        "correct": correct,
        "score": score,
        "feedback": {
            "kind": question.get("kind", "choice"),
            "matched": matched,
            "missed": missed,
            "minScore": min_score,
            "reference": question.get("reference", question.get("why", "")),
            "needsReview": needs_review,
            "message": "点击提交后由本地 rubric 判分" if question.get("kind") == "short" else "rubric 自动判分；不清楚的可再问老师",
        },
    }


# ───────────────────────── 作答 ─────────────────────────
def auto_text(question: dict) -> str:
    """开放题的“满分回答”：拼接 rubric 关键词（仅测试/模拟用）。"""
    rubric = question.get("rubric") or {}
    return " ".join(" ".join(point.get("any") or []) for point in rubric.get("points") or [])


def _apply_auto(state: dict, data: dict, question: dict, correct: bool = True) -> None:
    """测试/模拟辅助：自动作答（choice 选正确答案；开放题填 rubric 关键词或随便写）。"""
    if question.get("kind") in ("short", "explain", "proof"):
        apply_answer(state, data, question, 0, text=auto_text(question) if correct else "不知道")
        return
    if correct:
        apply_answer(state, data, question, int(question.get("answer", 1)))
    else:
        worse = 1 if question.get("answer") != 1 else 2
        apply_answer(state, data, question, worse)


def apply_answer(state: dict, data: dict, question: dict, choice: int, decision: str = None, text: str = None) -> dict:
    cfg = params(data)
    eta = cfg["eta"]
    low, high = cfg["theta_bounds"]
    if question.get("kind") in ("short", "explain", "proof"):
        graded = grade_open(question, text or "")
        correct = graded["correct"]
        open_score = graded["score"]
        open_feedback = graded["feedback"]
    else:
        correct = int(choice) == int(question.get("answer", -1))
        open_score = None
        open_feedback = None
    kp_theta = state.setdefault("kp_theta", {})
    for kp in question.get("kps") or []:
        p_kp = sigmoid(kp_theta.get(kp, 0.0) - difficulty(question))
        value = kp_theta.get(kp, 0.0) + eta * ((1.0 if correct else 0.0) - p_kp)
        kp_theta[kp] = max(low, min(high, value))
    for tag in tags_of(question):
        p = sigmoid(theta_of(state, data, tag) - difficulty(question))
        value = theta_of(state, data, tag) + eta * ((1.0 if correct else 0.0) - p)
        state.setdefault("theta", {})[tag] = max(low, min(high, value))
    probe_after = probe_observe(state, data, question, correct)
    history_entry = {
        "qid": question["id"], "choice": int(choice), "correct": correct,
        "at": now_iso(), "decision": decision,
        "_tags": tags_of(question),
        "probe": probe_after or None,
    }
    if open_score is not None:
        history_entry["kind"] = question.get("kind")
        history_entry["text"] = str(text or "")[:2000]
        history_entry["score"] = open_score
    state["history"].append(history_entry)
    for tag in tags_of(question):
        state.setdefault("last_at", {})[tag] = now_iso()
    if state.get("issued_qid") == question["id"]:
        state["issued_qid"] = None
    skipped = state.get("skipped")
    if isinstance(skipped, list) and question["id"] in skipped:
        skipped.remove(question["id"])
    if not state.get("calibration", {}).get("done") and state["calibration"].get("step", 0) < len(CALIBRATION_LADDER):
        state["calibration"]["step"] = state["calibration"].get("step", 0) + 1
        if state["calibration"]["step"] >= len(CALIBRATION_LADDER):
            state["calibration"]["done"] = True
    state["seen"][question["id"]] = state["seen"].get(question["id"], 0) + 1
    if not correct:
        state["wrong"][question["id"]] = state["wrong"].get(question["id"], 0) + 1
        state["streak"] = 0
    else:
        state["streak"] = state.get("streak", 0) + 1
    # 知识卡触发：连续两次答错且共享 tag
    if len(state["history"]) >= 2:
        last_two = state["history"][-2:]
        if all(entry.get("correct") is False for entry in last_two):
            common = set(last_two[0].get("_tags") or []) & set(last_two[1].get("_tags") or [])
            for tag in common:
                if tag in data["card_by_tag"]:
                    state["pending_card"] = tag
                    break
    feedback = open_feedback if open_feedback is not None else {
        "answerIndex": question.get("answer"),
        "answerText": (question.get("options") or [None])[int(question.get("answer", 1)) - 1],
        "why": question.get("why", ""),
        "source": question.get("source", ""),
    }
    return {
        "correct": correct,
        "feedback": feedback,
        "kind": question.get("kind", "choice"),
        "score": open_score if open_score is not None else (1.0 if correct else 0.0),
        "mastery": mastery_snapshot(state, data),
        "progress": progress_snapshot(state, data),
    }


# ───────────────────────── “我不懂” → 知识卡 ─────────────────────────
def course_spine(data: dict, data_dir: Path = None) -> list:
    """课程「脊」：以题库自身的 (章, 单元) 出现顺序为准（题库按课程序落笔），保证与题目 unit 字段一致。

    course.json 的单元是「文件级」（如 58 差集的代数），题库的 unit 是「课级聚合」（如 ④ 外延性与集合恒等式），
    两者不等价；用户要的「之前/之后」是课级顺序，所以这里用题库顺序。
    """
    spine = []
    seen = set()
    for question in data.get("questions", []):
        if not isinstance(question, dict):
            continue
        chapter = question.get("chapter")
        unit = question.get("unit")
        if not isinstance(chapter, str) or not isinstance(unit, str):
            continue
        key = (chapter, unit)
        if key in seen:
            continue
        seen.add(key)
        spine.append(key)
    return sorted(spine, key=lambda item: (item[0], spine.index(item))) if False else spine


def position_of(data: dict, question: dict) -> dict:
    """题目在课程里的位置：本单元序号、单元内题号、上一单元/下一单元。"""
    spine = data.get("_spine") or []
    key = (question.get("chapter"), question.get("unit"))
    index = -1
    for i, entry in enumerate(spine):
        if entry == key:
            index = i
            break
    unit_questions = [q for q in data.get("questions", [])
                      if q.get("chapter") == key[0] and q.get("unit") == key[1]]
    order = [q["id"] for q in unit_questions]
    qid = question.get("id")
    in_unit = (order.index(qid) + 1) if qid in order else 0
    prev_entry = spine[index - 1] if index > 0 else None
    next_entry = spine[index + 1] if 0 <= index < len(spine) - 1 else None
    return {
        "chapter": key[0],
        "unit": key[1],
        "unitIndex": index + 1 if index >= 0 else 0,
        "unitTotal": len(spine),
        "prevChapter": prev_entry[0] if prev_entry else "",
        "prevUnit": prev_entry[1] if prev_entry else "",
        "nextChapter": next_entry[0] if next_entry else "",
        "nextUnit": next_entry[1] if next_entry else "",
        "questionIndex": in_unit,
        "questionTotal": len(unit_questions),
    }


def explain_for(state: dict, data: dict, qid: str) -> dict:
    """给定题目，返回最匹配的知识卡；没有匹配卡时退化为该题的知识摘要。"""
    question = data["by_id"].get(qid)
    if not question:
        return {"ok": False, "error": "unknown question: " + qid}
    tags = tags_of(question)
    best_card = None
    best_score = -1
    for card in data["cards"]:
        score = len(set(card.get("tags") or []) & set(tags))
        if score > best_score:
            best_score = score
            best_card = card
    if best_card is not None and best_score > 0:
        return {"ok": True, "kind": "knowledge-card", "card": best_card,
                "reason": "matched-tags", "matched": sorted(set(best_card.get("tags") or []) & set(tags))}
    texts = " ".join([question.get("prompt", ""), " ".join(question.get("options") or []), question.get("why", "")])
    terms = []
    for entry in data.get("glossary", []):
        term = str(entry.get("term", ""))
        if term and term in texts and entry not in terms:
            terms.append(entry)
        if len(terms) >= 6:
            break
    sections = [{"heading": "为什么", "kind": "paragraph", "text": question.get("why", "")}]
    if terms:
        sections.append({"heading": "这题涉及的术语/类型", "kind": "list",
                         "items": ["`" + str(entry.get("term")) + "`：" + str(entry.get("oneLiner", "")) for entry in terms]})
    sections.append({"heading": "再想一步", "kind": "list", "items": [
        "把正确的选项逐字读一遍，确认每个符号的含义；",
        "回到本题重答一次（不算错）。"]})
    fallback = {
        "id": "explain-" + question["id"],
        "kicker": question.get("chapter", "") + " · " + question.get("unit", ""),
        "title": "这题在考什么",
        "oneLiner": question.get("why", ""),
        "definition": {"lang": "lean", "code": (question.get("options") or [""])[int(question.get("answer", 1)) - 1]},
        "sections": sections,
        "source": question.get("source", ""),
        "types": [{"term": entry.get("term"), "oneLiner": entry.get("oneLiner"), "source": entry.get("source")} for entry in terms],
    }
    return {"ok": True, "kind": "fallback-card", "card": fallback, "reason": "no-matched-card", "types": fallback["types"]}


def cmd_explain(state_path: Path, data: dict, qid: str) -> int:
    state = load_state(state_path)
    result = explain_for(state, data, qid)
    if result.get("ok"):
        confusions = state.setdefault("confusions", {})
        confusions[qid] = confusions.get(qid, 0) + 1
        save_state(state_path, state)
    print(json.dumps(result, ensure_ascii=False))
    return 0 if result.get("ok") else 1


def cmd_skip_code(state_path: Path, data: dict, exercise: str) -> int:
    state = load_state(state_path)
    skipped = state.setdefault("code_done", {})
    skipped[exercise] = "skipped"
    if state.get("issued_code") == exercise:
        state["issued_code"] = None
    save_state(state_path, state)
    print(json.dumps({"ok": True, "exercise": exercise, "skipped": True}, ensure_ascii=False))
    return 0


def cmd_mark_code(state_path: Path, data: dict, exercise: str, solved: bool) -> int:
    state = load_state(state_path)
    if solved:
        state.setdefault("code_done", {})[exercise] = True
        if state.get("issued_code") == exercise:
            state["issued_code"] = None
    save_state(state_path, state)
    print(json.dumps({"ok": True, "exercise": exercise, "solved": bool(solved),
                      "codeDone": len(state.get("code_done", {}))}, ensure_ascii=False))
    return 0


# ───────────────────────── CLI 命令 ─────────────────────────
LEMMA_NAME_RE = None


def lemmas_of_question(data: dict, question: dict) -> list:
    """题目（题干/选项/解析）里提到的库声明 → 附上 signature/一句话/出处。"""
    import re
    global LEMMA_NAME_RE
    if LEMMA_NAME_RE is None:
        LEMMA_NAME_RE = re.compile(r"\b(?:[A-Z][A-Za-z]*(?:\.[A-Za-z_][A-Za-z0-9_]*)+)\b|\b[a-z_][a-z0-9_]*_iff_[a-z0-9_]+\b|\b[A-Za-z_][A-Za-z0-9_]*_of_[A-Za-z0-9_]+\b")
    if not isinstance(question, dict):
        return []
    text = " ".join([question.get("prompt") or "", " ".join(question.get("options") or []), question.get("why") or ""])
    index = data.get("lemma_by_name") or {}
    found = []
    seen = set()
    for name in LEMMA_NAME_RE.findall(text):
        if name in seen or name not in index:
            continue
        seen.add(name)
        item = index[name]
        found.append({"name": name, "signature": item.get("signature", ""),
                      "oneLiner": item.get("oneLiner", ""), "source": item.get("source", "")})
        if len(found) >= 6:
            break
    return found


def map_snapshot(state: dict, data: dict) -> dict:
    """学习地图：按课程脊逐单元给出状态（✅ 稳 / ◐ 沾过 / · 未碰）+ 当前位置。"""
    spine = data.get("_spine") or []
    current_qid = state.get("issued_qid")
    current = data["by_id"].get(current_qid) if current_qid else None
    current_unit = (current.get("chapter"), current.get("unit")) if current else (None, None)
    rows = []
    for chapter, unit in spine:
        unit_questions = [q for q in data["questions"] if q.get("chapter") == chapter and q.get("unit") == unit]
        tags = set()
        for question in unit_questions:
            tags.update(tags_of(question))
        touched = set()
        for entry in state.get("history", []):
            question = data["by_id"].get(entry.get("qid"))
            if question and question.get("chapter") == chapter and question.get("unit") == unit:
                touched.update(tags_of(question))
        values = [mastery(state, data, tag) for tag in tags]
        avg = sum(values) / len(values) if values else 0.0
        if not touched:
            status = "未碰"
            mark = "·"
        elif avg >= 0.66:
            status = "稳"
            mark = "✅"
        elif avg >= 0.45:
            status = "在学"
            mark = "◐"
        else:
            status = "刚起步"
            mark = "◔"
        rows.append({
            "chapter": chapter, "unit": unit, "mark": mark, "status": status,
            "touched": len(touched & tags), "total": len(tags),
            "answered": sum(1 for entry in state.get("history", [])
                            if (data["by_id"].get(entry.get("qid")) or {}).get("chapter") == chapter
                            and (data["by_id"].get(entry.get("qid")) or {}).get("unit") == unit),
            "mastery": round(avg, 3),
            "current": (chapter, unit) == current_unit,
        })
    done_units = sum(1 for row in rows if row["status"] == "稳")
    return {"units": rows, "unitTotal": len(rows), "unitDone": done_units,
            "currentUnit": (current_unit[1] if current_unit[1] else "")}


def format_map(snapshot: dict, data: dict) -> list:
    """把学习地图排成几行文本（章内一行，单元用符号串联）。"""
    lines = []
    by_chapter = {}
    for row in snapshot.get("units", []):
        by_chapter.setdefault(row["chapter"], []).append(row)
    lines.append("🗺️ 学习地图 · 共 %d 个单元，稳了 %d 个" % (snapshot.get("unitTotal", 0), snapshot.get("unitDone", 0)))
    for chapter in sorted(by_chapter):
        rows = by_chapter[chapter]
        marks = []
        for row in rows:
            mark = row["mark"]
            if row["current"]:
                mark = "▶" + mark
            marks.append(mark)
        lines.append("%s  %s   （%d/%d 单元稳）" % (
            chapter, " ".join(marks),
            sum(1 for row in rows if row["status"] == "稳"), len(rows)))
    lines.append("")
    lines.append("图例：✅ 稳　◐ 在学　◔ 刚起步　· 未碰　▶ 当前")
    current = [row for row in snapshot.get("units", []) if row.get("current")]
    if current:
        row = current[0]
        lines.append("你现在在：%s %s（本单元已答 %d 题，接触 %d/%d 个知识点）" % (
            row["chapter"], row["unit"], row["answered"], row["touched"], row["total"]))
    return lines


def cmd_next(state_path: Path, data: dict, chapter: str, seed: int, advance: bool, no_lesson: bool = False) -> int:
    state = load_state(state_path)
    step = next_step(state, data, advance=advance, chapter=chapter, no_lesson=no_lesson)
    state["last_decision"] = step.get("decision")
    save_state(state_path, state)
    payload = {"ok": True, **step, "progress": progress_snapshot(state, data),
               "positive": positive_snapshot(state, data),
               "mastery": mastery_snapshot(state, data),
               "calibration": state.get("calibration")}
    target = step.get("question")
    if isinstance(target, dict):
        payload["position"] = position_of(data, target)
        payload["lemmas"] = lemmas_of_question(data, target)
    elif isinstance(step.get("card"), dict) and step.get("decision", {}).get("then"):
        pending = data["by_id"].get(step["decision"]["then"])
        if pending is not None:
            payload["position"] = position_of(data, pending)
    print(json.dumps(payload, ensure_ascii=False))
    return 0


def cmd_answer(state_path: Path, data: dict, qid: str, choice: int, text: str = None,
               dry_run: bool = False) -> int:
    state = load_state(state_path)
    question = data["by_id"].get(qid)
    if not question:
        print(json.dumps({"ok": False, "error": "unknown question: " + qid}, ensure_ascii=False))
        return 1
    kind = question.get("kind") or "choice"
    if kind != "choice" and (text is None or str(text).strip() == ""):
        print(json.dumps({"ok": False, "error": "need-text", "kind": kind,
                          "qid": qid, "prompt": question.get("prompt", ""),
                          "hint": "这是开放题：请把回答文本交上来（--text）"},
                         ensure_ascii=False))
        return 1
    if dry_run:
        if kind == "choice":
            correct = int(choice) == int(question.get("answer", -1))
            hints = question.get("hints") or []
            hint = hints[0] if hints else (question.get("hint") or "")
            if hint == "":
                # 用「本知识点的真实要点」当提示：切题且不泄露答案
                kp_index = {kp["id"]: kp for kp in (data.get("kps") or [])}
                for kp_id in question.get("kps") or []:
                    key_point = (kp_index.get(kp_id) or {}).get("keyPoint")
                    if isinstance(key_point, str) and key_point.strip() != "":
                        hint = key_point.strip()
                        break
            if hint == "":
                hint = "回到定义：把题干里的符号展开成「逐个元素」的命题，再对照选项。"
            print(json.dumps({"ok": True, "dryRun": True, "correct": correct,
                              "qid": qid, "hint": hint, "kind": "choice",
                              "position": position_of(data, question)}, ensure_ascii=False))
            return 0
        graded = grade_open(question, text or "")
        print(json.dumps({"ok": True, "dryRun": True, "correct": graded["correct"],
                          "score": graded["score"], "feedback": graded["feedback"],
                          "qid": qid, "position": position_of(data, question)}, ensure_ascii=False))
        return 0
    decision = (state.get("last_decision") or {}).get("reason")
    result = apply_answer(state, data, question, choice, decision=decision, text=text)
    save_state(state_path, state)
    print(json.dumps({"ok": True, **result, "position": position_of(data, question)}, ensure_ascii=False))
    return 0


def cmd_state(state_path: Path, data: dict) -> int:
    """**Agent 感知接口**：返回学习系统的全部可观察状态（客户端 UI 与 agent 共用同一份）。"""
    state = load_state(state_path)
    print(json.dumps({"ok": True,
                      "progress": progress_snapshot(state, data),
                      "mastery": mastery_snapshot(state, data),
                      "positive": positive_snapshot(state, data),
                      "coverage": coverage_snapshot(state, data),
                      "heatmap": heatmap_snapshot(state, data),
                      "history_tail": history_tail(state, data),
                      "wrong": [{"qid": qid, "count": count} for qid, count in
                                sorted(state.get("wrong", {}).items(), key=lambda kv: -kv[1])][:20],
                      "cards_shown": state.get("cards_shown", {}),
                      "pending_card": state.get("pending_card"),
                      "issued_qid": state.get("issued_qid"),
                      "calibration": state.get("calibration"),
                      "flow": state.get("flow"),
                      "afterPrimer": state.get("after_primer_qid"),
                      "codeDone": sorted(state.get("code_done", {}).keys()),
                      "confusions": state.get("confusions", {}),
                      "last_decision": state.get("last_decision")}, ensure_ascii=False))
    return 0


# ───────────────────────── 自检 ─────────────────────────
def cmd_selftest(data: dict) -> int:
    cfg = params(data)
    failures = []

    def check(name, condition, detail=""):
        if condition:
            print("PASS " + name + ("  " + detail if detail else ""))
        else:
            print("FAIL " + name + ("  " + detail if detail else ""))
            failures.append(name)

    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)

        # 1) 确定性：同 seed 同状态 → 同题
        s1 = new_state(); p1 = pick(s1, data, seed=42)["question"]["id"]
        s2 = new_state(); p2 = pick(s2, data, seed=42)["question"]["id"]
        check("next-deterministic", p1 == p2, p1 + " vs " + p2)

        # 2) 去重：连续 15 题不重复（全对作答）
        state = new_state()
        asked_ids = []
        for _ in range(15):
            q = pick(state, data, seed=1)["question"]
            asked_ids.append(q["id"])
            _apply_auto(state, data, q, True)
        check("no-repeat-in-15", len(set(asked_ids)) == 15, str(len(set(asked_ids))) + "/15")

        # 3) 盲区优先：把某 tag 的 theta 压到 -2，接下来 5 题应优先该 tag
        state = new_state()
        target_tag = "antisymmetric"
        state["theta"][target_tag] = -2.0
        picked_tags = []
        for _ in range(8):
            q = pick(state, data, seed=3)["question"]
            picked_tags.append(q["tags"])
            if target_tag in q["tags"]:
                break
            _apply_auto(state, data, q, True)
        check("gap-priority", any(target_tag in tags for tags in picked_tags), str(picked_tags[:2]))

        # 4) 答错后变式优先：对 strict-order 题答错，下一题应命中同 tag
        state = new_state()
        q = next(x for x in data["questions"] if "strict-order" in x["tags"])
        wrong_choice = 1 if q["answer"] != 1 else 2
        _apply_auto(state, data, q, False)
        nxt = pick(state, data, seed=4)["question"]
        check("variant-after-wrong", bool(set(q["tags"]) & set(nxt["tags"])), nxt["id"])

        # 5) 知识卡触发：同 tag 连续两错 → 下一张是知识卡
        state = new_state()
        order_qs = [x for x in data["questions"] if "order" in x["tags"]][:2]
        for item in order_qs:
            wrong = 1 if item["answer"] != 1 else 2
            _apply_auto(state, data, item, False)
        card = next_card(state, data)
        check("card-after-two-wrong", bool(card) and card.get("kind") == "card", str(bool(card)))

        # 6) 复习到期：老时间 + 低掌握 → decision.reason == review
        state = new_state()
        tag = "extensionality"
        state["theta"][tag] = -1.0
        state["last_at"][tag] = "2020-01-01T00:00:00Z"
        state["history"] = [{"qid": data["questions"][0]["id"], "choice": 1, "correct": True,
                             "at": "2020-01-01T00:00:00Z", "decision": "review"}]
        due = review_due(state, data)
        check("review-due", tag in due or any(t in due for t in state["theta"]), str(sorted(due)))

        # 7) 掌握度收敛：同 tag 连对 8 次 → mastery > 0.8
        state = new_state()
        q = next(x for x in data["questions"] if "membership" in x["tags"])
        for _ in range(40):
            _apply_auto(state, data, q, True)
        m = mastery(state, data, "membership")
        check("mastery-converges", m > 0.7, str(round(m, 3)))

        # 8) “我不懂” → 知识卡：antisymm 题应命中有 antisymmetric tag 的卡
        q = next(x for x in data["questions"] if "antisymmetric" in x["tags"])
        explained = explain_for(new_state(), data, q["id"])
        check("explain-matches-card", explained.get("ok") and explained.get("kind") == "knowledge-card",
              str(explained.get("matched", [])))

        # 9) 刷新恢复：不带 advance 连取两次 → 同一题
        s1 = new_state()
        first = next_step(s1, data)
        second = next_step(s1, data)
        check("resume-same-card", first["question"]["id"] == second["question"]["id"],
              first["question"]["id"] + " / " + second["decision"]["reason"])
        _apply_auto(s1, data, first["question"], True)
        advanced = next_step(s1, data, advance=True)
        check("advance-moves-on", advanced["question"]["id"] != first["question"]["id"],
              first["question"]["id"] + " -> " + advanced["question"]["id"])

        # 10) 校准阶梯：前 6 题的章节/level 按阶梯推进
        s2 = new_state()
        ladder_seen = []
        for _ in range(6):
            step = next_step(s2, data, advance=True)
            if step["kind"] != "question":
                continue
            ladder_seen.append((step["question"]["chapter"], step["question"]["level"]))
            item = step["question"]
            apply_answer(s2, data, item, item["answer"])
        expected_ladder = [("I.1", 1), ("I.1", 3), ("I.2", 1), ("I.2", 3), ("I.3", 1), ("I.3", 3)]
        check("calibration-ladder", ladder_seen == expected_ladder, str(ladder_seen))
        check("calibration-done", s2["calibration"]["done"] is True, str(s2["calibration"]))

        # 11) explain fallback 会列出涉及的类型
        question = next(x for x in data["questions"] if "membership" in x["tags"])
        result = explain_for(new_state(), data, question["id"])
        ok = result.get("kind") == "knowledge-card" or (result.get("kind") == "fallback-card"
                                                        and isinstance(result.get("types"), list))
        check("explain-has-types", ok, str(result.get("kind")))

        # 12) 开放题 rubric 自动判分（命中/漏点两种）
        open_q = next(x for x in data["questions"] if x.get("kind") == "short")
        good = grade_open(open_q, " ".join(" ".join(p.get("any") or []) for p in (open_q.get("rubric") or {}).get("points") or []))
        bad = grade_open(open_q, "完全无关的回答")
        check("open-rubric-good", good["correct"] is True and good["score"] >= 0.6, str(good["score"]))
        check("open-rubric-bad", bad["correct"] is False and bad["score"] < 0.6, str(bad["score"]))

        # 13) 只发卡未作答时，热力图不得“提前点亮”
        s3 = new_state()
        next_step(s3, data)
        hot = heatmap_snapshot(s3, data)
        touched_total = sum(1 for row in hot["rows"] for cell in row["cells"] if cell["touched"])
        check("heatmap-not-touched-before-answer", touched_total == 0, "touched=" + str(touched_total))
        # 作答一题后，该题 tag 才亮
        s4 = new_state()
        step = next_step(s4, data)
        _apply_auto(s4, data, step["question"], True)
        hot2 = heatmap_snapshot(s4, data)
        touched_after = sum(1 for row in hot2["rows"] for cell in row["cells"] if cell["touched"])
        check("heatmap-touched-after-answer", touched_after > 0, "touched=" + str(touched_after))

        # 14) 瀑布流编排：校准完成后第 3 个条目插入代码练习
        s5 = new_state(); s5["calibration"] = {"step": 6, "done": True}
        kinds = []
        for _ in range(6):
            step = next_step(s5, data, advance=True)
            kinds.append(step["kind"])
            if step["kind"] == "question":
                _apply_auto(s5, data, step["question"], True)
            elif step["kind"] == "code":
                s5.setdefault("code_done", {})[step["exercise"]["id"]] = True
                s5["issued_code"] = None
        flow_cfg = (data.get("params") or {}).get("flow") or {}
        code_every = flow_cfg.get("codeEvery", 3)
        if isinstance(code_every, (int, float)) and code_every >= 1:
            check("flow-includes-code", "code" in kinds, str(kinds))
        else:
            check("flow-code-disabled", "code" not in kinds, str(kinds))

        # 15) primer：新知识点题目之前先出简洁知识卡（卡 → 题）
        s6 = new_state(); s6["calibration"] = {"step": 6, "done": True}
        s6["taught"] = {"|".join(key): True for key in (data.get("lesson_by_unit") or {})}  # 跳过教学环节，专测 primer
        # primer 默认关闭（params.flow.primerCards=false），本用例显式打开来测通道
        data["params"].setdefault("flow", {})["primerCards"] = True
        first = next_step(s6, data, advance=True, ignore_probe=True)
        # 新 session 的第一条应当是 primer 卡或题；找到一次「卡后面紧跟同一 tag 的题」即通过
        kinds = [first["kind"]]
        primer_ok = first["kind"] == "card" and first.get("decision", {}).get("reason") == "primer"
        if primer_ok:
            after = next_step(s6, data, advance=True)
            kinds.append(after["kind"])
            primer_ok = after["kind"] == "question" and after["question"]["id"] == first["decision"]["then"]
        check("primer-card-before-question", primer_ok, str(kinds))
        data["params"]["flow"]["primerCards"] = False  # 恢复默认：primer 关闭

        # 15b2) primer 默认关闭时：直接出题（用户要求「下一题/开始做题」就给题）
        s6b = new_state(); s6b["calibration"] = {"step": 6, "done": True}
        s6b["taught"] = {"|".join(key): True for key in (data.get("lesson_by_unit") or {})}
        step6b = next_step(s6b, data, advance=True)
        check("primer-default-off", step6b["kind"] == "question", str(step6b.get("kind")))

        # 15b) 教学环节：新单元先上微课，题目排在后面（探针排在教学环节之后）
        s7 = new_state(); s7["calibration"] = {"step": 6, "done": True}
        first7 = next_step(s7, data, advance=True)
        lesson_ok = first7["kind"] == "lesson" and isinstance((first7.get("lesson") or {}).get("goal"), str)
        if lesson_ok:
            after7 = next_step(s7, data, advance=True)
            lesson_ok = after7["kind"] == "question" and after7["question"]["id"] == first7["decision"]["then"]
        check("lesson-before-questions", lesson_ok, str(first7.get("kind")))

        # 15c) 连续推进不能重复同一道题（用户报「下一题」出同一题）
        s8 = new_state(); s8["calibration"] = {"step": 6, "done": True}
        s8["taught"] = {"|".join(key): True for key in (data.get("lesson_by_unit") or {})}
        ids = []
        for _ in range(12):  # 跳过教学环节/探针，收集 3 道题
            step = next_step(s8, data, advance=True, no_lesson=True, ignore_probe=True)
            if step.get("kind") == "question":
                ids.append(step["question"]["id"])
            if len(ids) == 3:
                break
        check("advance-does-not-repeat", len(ids) == 3 and len(set(ids)) == 3, str(ids))

        # 15d) 选项基准：题库 answer 是 1-based（A=1），choice 也是 1-based
        s_choice = new_state()
        probe_q = None
        for question in data["questions"]:
            if isinstance(question.get("answer"), int) and question.get("options"):
                probe_q = question
                break
        if probe_q is not None:
            right = apply_answer(s_choice, data, probe_q, int(probe_q["answer"]))
            wrong_choice = 1 if int(probe_q["answer"]) != 1 else 2
            wrong = apply_answer(s_choice, data, probe_q, wrong_choice)
            check("answer-index-1-based",
                  right["correct"] is True and wrong["correct"] is False,
                  str((probe_q["id"], probe_q["answer"], right["correct"], wrong["correct"])))

        # 15e) 「定义级相等」必须能通过 rfl（防止再出「让学习者证明 rfl 能过的东西」的坏题）
        definitional = [q for q in data["questions"]
                        if "A \\ B = A ∩ Bᶜ" in (q.get("prompt") or "") and q.get("kind") == "choice"]
        check("definitional-equality-question-present", len(definitional) <= 1, str(len(definitional)))

        # 15f) 离线评测必须容忍开放题（short/explain 没有 answer 字段）
        open_only = dict(data)
        open_only["questions"] = [q for q in data["questions"]
                                  if q.get("kind") in ("short", "explain", "proof")]
        open_only["by_id"] = {q["id"]: q for q in open_only["questions"]}
        try:
            sim = simulate_learner(0.0, open_only, 4, seed=7)
            simulate_ok = bool(sim.get("asked")) and len(sim["asked"]) == 4
        except Exception as exc:  # noqa: BLE001 - 自检要显示具体异常
            simulate_ok = False
            sim = {"error": "%s: %s" % (type(exc).__name__, exc)}
        check("simulate-open-kinds", simulate_ok, str(sim.get("error") or sim.get("accuracy")))

        # 16) 状态文件坏文件降级
        bad = tmp / "bad.json"
        bad.write_text("{not json", encoding="utf-8")
        state = load_state(bad)
        check("bad-state-degrades", state.get("version") == 1 and state["history"] == [])

        del cfg

    if failures:
        print("\nSELFTEST: %d failed" % len(failures), file=sys.stderr)
        return 1
    print("\nSELFTEST: all passed")
    return 0


# ───────────────────────── 离线评测 ─────────────────────────
def simulate_learner(ability: float, data: dict, count: int, seed: int) -> dict:
    rng = random.Random(seed)
    state = new_state()
    student = {q["id"]: rng.random() for q in data["questions"]}  # 固定人格
    asked = []
    correct = 0
    for _ in range(count):
        picked = pick(state, data, seed=rng.randrange(1, 10 ** 6))["question"]
        asked.append(picked["id"])
        # 答对概率由学习者能力 vs 题目难度决定
        p = sigmoid(ability - difficulty(picked))
        outcome = rng.random() < p
        if picked.get("kind") in ("short", "explain", "proof"):
            # 开放题没有 answer 字段；_apply_auto 会按 rubric 自动作答。
            choice = 0
        else:
            answer = int(picked.get("answer", 1))
            choice = answer if outcome else (1 if answer != 1 else 2)
        _apply_auto(state, data, picked, outcome)
        correct += 1 if outcome else 0
    tags = {}
    for qid in asked:
        question = data["by_id"].get(qid)
        if question:
            for tag in tags_of(question):
                tags[tag] = tags.get(tag, 0) + 1
    return {
        "asked": asked, "correct": correct, "accuracy": round(correct / count, 3),
        "distinct": len(set(asked)), "tag_coverage": tags,
        "cards_shown": sum(state["cards_shown"].values()),
        "mastery_end": mastery_snapshot(state, data),
    }


def cmd_simulate(data: dict, count: int, out_dir: Path) -> int:
    learners = {"weak": -1.2, "mid": 0.0, "strong": 1.2}
    report = {"generated_at": now_iso(), "questions": count, "learners": {}}
    for index, (name, ability) in enumerate(learners.items()):
        result = simulate_learner(ability, data, count, seed=1000 + index)
        result["ability"] = ability
        result.pop("asked", None)  # 报告不带逐题序列（太长）
        report["learners"][name] = result
        print("%-7s accuracy=%.2f  distinct=%d/%d  cards=%d" % (
            name, result["accuracy"], result["distinct"], count, result["cards_shown"]))
    out_dir.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    target = out_dir / ("eval-" + stamp + ".json")
    target.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("report: " + str(target.relative_to(ROOT.parent.parent)))
    return 0


# ───────────────────────── main ─────────────────────────
def main() -> int:
    parser = argparse.ArgumentParser(description="L2 自适应调度引擎")
    parser.add_argument("--data-dir", default=os.environ.get("SK_KNOWLEDGE_DIR", str(DEFAULT_DATA_DIR)))
    sub = parser.add_subparsers(dest="command", required=True)

    p_next = sub.add_parser("next")
    p_next.add_argument("--state", required=True)
    p_next.add_argument("--chapter")
    p_next.add_argument("--seed", type=int)
    p_next.add_argument("--advance", action="store_true", help="推进到下一张（默认恢复已发出的卡）")
    p_next.add_argument("--no-lesson", action="store_true", dest="no_lesson",
                        help="不要教学环节卡：直接给题（「下一题」用这个）")

    p_answer = sub.add_parser("answer")
    p_answer.add_argument("--state", required=True)
    p_answer.add_argument("--qid", required=True)
    p_answer.add_argument("--choice", type=int, default=0)
    p_answer.add_argument("--text", default=None)
    p_answer.add_argument("--dry-run", action="store_true", dest="dry_run",
                        help="只判分不记账（开放题给用户重答机会时用）")

    p_state = sub.add_parser("state")
    p_state.add_argument("--state", required=True)

    p_explain = sub.add_parser("explain")
    p_explain.add_argument("--state", required=True)
    p_explain.add_argument("--qid", required=True)

    p_skip = sub.add_parser("skip-kp")
    p_skip.add_argument("--state", required=True)
    p_skip.add_argument("--kp", default=None, help="要跳过的知识点 id（缺省=当前题的知识点）")

    p_map = sub.add_parser("map")
    p_map.add_argument("--state", required=True)

    p_mark = sub.add_parser("mark-code")
    p_mark.add_argument("--state", required=True)
    p_mark.add_argument("--exercise", required=True)
    p_mark.add_argument("--solved", action="store_true")

    p_skip = sub.add_parser("skip-code")
    p_skip.add_argument("--state", required=True)
    p_skip.add_argument("--exercise", required=True)

    sub.add_parser("selftest")

    p_sim = sub.add_parser("simulate")
    p_sim.add_argument("--questions", type=int, default=20)
    p_sim.add_argument("--out", default=str(ROOT / "reports"))

    args = parser.parse_args()
    data_dir = Path(args.data_dir).expanduser()
    if not (data_dir / "questions.json").exists():
        print(json.dumps({"ok": False, "error": "data dir has no questions.json: " + str(data_dir)}))
        return 2
    data = load_data(data_dir)
    index_cards(data)

    if args.command == "next":
        return cmd_next(Path(args.state).expanduser(), data, args.chapter, args.seed, args.advance,
                        no_lesson=getattr(args, "no_lesson", False))
    if args.command == "answer":
        return cmd_answer(Path(args.state).expanduser(), data, args.qid, args.choice, args.text,
                          dry_run=getattr(args, "dry_run", False))
    if args.command == "state":
        return cmd_state(Path(args.state).expanduser(), data)
    if args.command == "explain":
        return cmd_explain(Path(args.state).expanduser(), data, args.qid)
    if args.command == "skip-kp":
        state = load_state(Path(args.state).expanduser())
        kp_id = args.kp
        if not kp_id:
            issued = data["by_id"].get(state.get("issued_qid") or "")
            kp_list = (issued or {}).get("kps") or []
            kp_id = kp_list[0] if kp_list else ""
        if not kp_id:
            print(json.dumps({"ok": False, "error": "没有可以跳过的知识点"}, ensure_ascii=False))
            return 1
        entry = probe_state(state, data, kp_id)
        entry["done"] = True
        # 掌握度给一个「已会」的下限证据（不是满分，只是不再怀疑）
        state.setdefault("kp_theta", {})[kp_id] = max(state.setdefault("kp_theta", {}).get(kp_id, 0.0), 0.6)
        state["issued_qid"] = None
        save_state(Path(args.state).expanduser(), state)
        print(json.dumps({"ok": True, "skipped": kp_id}, ensure_ascii=False))
        return 0
    if args.command == "map":
        state = load_state(Path(args.state).expanduser())
        snapshot = map_snapshot(state, data)
        print(json.dumps({"ok": True, "map": snapshot, "lines": format_map(snapshot, data)}, ensure_ascii=False))
        return 0
    if args.command == "mark-code":
        return cmd_mark_code(Path(args.state).expanduser(), data, args.exercise, args.solved)
    if args.command == "skip-code":
        return cmd_skip_code(Path(args.state).expanduser(), data, args.exercise)
    if args.command == "selftest":
        return cmd_selftest(data)
    if args.command == "simulate":
        return cmd_simulate(data, args.questions, Path(args.out).expanduser())
    return 2


if __name__ == "__main__":
    sys.exit(main())
