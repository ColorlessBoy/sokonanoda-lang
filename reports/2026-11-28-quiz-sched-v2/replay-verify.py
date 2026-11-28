#!/usr/bin/env python3
"""B 状态回放判据（issue #138 验证方案 B）：真实 state 副本上连续 next + 答对。

判据：
  1) 从进入一个新单元的那一步起，连续 3 次 next 落在同一 unit，第 4 次才换；
  2) map.kps.solved 不含 asked < 3 的 KP，且 probing 计数出现。
"""
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(sys.argv[1]).resolve()
STATE = Path(sys.argv[2]).resolve()
sys.path.insert(0, str(REPO / "courses/set-theory/knowledge"))
import engine  # noqa: E402

print("repo:", REPO)
print("state:", STATE)
print("state_sha256:", hashlib.sha256(STATE.read_bytes()).hexdigest())
print("engine.py_sha256:", hashlib.sha256(
    (REPO / "courses/set-theory/knowledge/engine.py").read_bytes()).hexdigest())

data = engine.load_data(REPO / "courses/set-theory/knowledge/data")
engine.index_cards(data)
state = engine.load_state(STATE)

steps = []
for index in range(10):
    step = engine.next_step(state, data, advance=True, no_lesson=True)
    if step.get("kind") != "question":
        steps.append({"step": index + 1, "kind": step.get("kind"),
                      "reason": (step.get("decision") or {}).get("reason")})
        continue
    question = step["question"]
    decision = step.get("decision") or {}
    probe = decision.get("probe") or {}
    steps.append({
        "step": index + 1, "kind": "question", "qid": question["id"],
        "chapter": question.get("chapter"), "unit": question.get("unit"),
        "reason": decision.get("reason"), "kp": probe.get("kp"),
        "probe_asked": probe.get("asked"), "role": probe.get("role"),
    })
    engine.apply_answer(state, data, question, int(question.get("answer", 2)))

for row in steps:
    print(json.dumps(row, ensure_ascii=False))

units = [row.get("unit") for row in steps if row.get("kind") == "question"]
print("units:", json.dumps(units, ensure_ascii=False))

runs = []
for index in range(0, len(units) - 3):
    if units[index] == units[index + 1] == units[index + 2] != units[index + 3]:
        runs.append({"start": index + 1, "unit": units[index],
                     "next_unit": units[index + 3]})
print("three_same_then_switch:", json.dumps(runs, ensure_ascii=False))

snapshot = engine.map_snapshot(state, data)
kps = snapshot["kps"]
min_probes = snapshot["thresholds"]["min_probes"]
rows = engine.kp_status(state, data)["kps"]
# 边界：可服务题不足 min_probes 的 KP 做满现有题即判（有效门槛 = min(min_probes, total)）
bad_solved = [row["id"] for row in rows
              if row["solved"] and row["asked"] < min(min_probes, max(1, row["total"]))]
small_kp_solved = [{"id": row["id"], "asked": row["asked"], "total": row["total"]}
                   for row in rows if row["solved"] and row["asked"] < min_probes]
bad_probing = [row["id"] for row in rows
               if row["probing"] and not (0 < row["asked"] < min(min_probes, row["total"]))]
print("map_kps:", json.dumps(kps, ensure_ascii=False))
print("min_probes:", min_probes)
print("solved_with_asked_below_min:", json.dumps(bad_solved))
print("small_servable_kp_solved_boundary:", json.dumps(small_kp_solved, ensure_ascii=False))
print("probing_outside_0_asked_min:", json.dumps(bad_probing))
print("B_CRITERIA_PASS:", bool(runs) and not bad_solved and not bad_probing and kps["probing"] > 0)
