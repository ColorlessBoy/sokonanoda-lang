#!/usr/bin/env python3
"""**真课程分阶段耗时剖面**（2026-09-29，用户 09:12 要的"基础剖面链路"）。

为什么必须入库：`218.8s` 这种聚合数**答不了"花在哪一步"**，也验不了"单条成本是否
随序号线性增长"（O(N²) 前缀重跑）⇒ 一次性分析没用，**每一刀优化都要用同一把尺子前后对比**。

口径（硬要求）：真课程（**不许合成夹具**）· **release** · 冷缓存 · 同一输入跑两次给方差。
实测方差 **±2%** ⇒ **<5% 的差不许当结论**。

出处（业界做法，已核实）：
  * 阈值过滤仿 Lean `trace.profiler.threshold`（默认 100ms，只报超阈值的）
    https://vca-epfl.github.io/wiki/lean-profiling/
  * 逐文件单独跑避免并行污染（Sebastian Ullrich）
    https://leanprover-community.github.io/archive/stream/270676-lean4/topic/profiling.20a.20project.html#500637969

用法：
    python3 scripts/profile-course.py [课程目录] [--runs N] [--threshold-ms MS]
产物：
    docs/perf/course-profile-<课程名>-<日期>.json   （schema `soko.course-profile/1`）

schema：version / commit / host{system,machine,cpus} / **profile** / course /
runs[] / stages / amplification / hot_top10 / hypotheses{A,B,C}
（`profile` 必须写进输出 —— debug/release 那个坑靠它堵 ✓）
"""

import argparse
import collections
import datetime
import glob
import json
import os
import platform
import re
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DECL_RE = re.compile(r'^\s*(theorem|def|axiom|inductive|example|opaque|instance)\s')


def parse_args():
    ap = argparse.ArgumentParser()
    ap.add_argument("course", nargs="?", default=os.path.join(ROOT, "courses/set-theory"))
    ap.add_argument("--runs", type=int, default=2, help="同一输入跑几次（给方差；默认 2）")
    ap.add_argument("--threshold-ms", type=float, default=0.0,
                    help="逐声明事件的阈值（默认 0 = 全打；仿 Lean 默认 100）")
    ap.add_argument("--json", dest="out", default=None)
    ap.add_argument("--bin", default=None)
    return ap.parse_args()


def profile_of(binary):
    if "/target/release/" in binary:
        return "release"
    if "/target/debug/" in binary:
        return "debug"
    return "unknown"


def decl_counts(course):
    """三层各自的声明数与文件数（静态扫源）。"""
    out = {}
    for layer in ("lib", "units", "units/solutions"):
        files = sorted(glob.glob(os.path.join(course, layer, "*.sokonanoda")))
        decls = 0
        for path in files:
            with open(path, errors="replace") as handle:
                decls += sum(1 for line in handle if DECL_RE.match(line))
        out[layer] = {"files": len(files), "decls": decls}
    return out


def run_once(binary, course, threshold_ms):
    """一次冷跑：清两处缓存 → 计时 → 收阶段统计 + 逐声明事件。"""
    cache = tempfile.mkdtemp(prefix="soko-profile-")
    project_artifacts = os.path.join(course, ".sokonanoda")
    subprocess.run(["rm", "-rf", project_artifacts], check=False)
    env = dict(os.environ)
    env["SOKONANODA_CACHE_DIR"] = cache
    env["SOKO_DECL_PROFILE"] = "1"
    env["SOKO_DECL_PROFILE_MS"] = str(threshold_ms)
    env["SOKO_STAGE_STATS"] = "1"
    start = time.time()
    proc = subprocess.run(
        [binary, "build", "--json", course],
        env=env, capture_output=True, text=True,
    )
    wall = time.time() - start
    stats = {}
    match = re.search(r"STAGE_STATS (.*)", proc.stderr)
    if match:
        for token in match.group(1).split():
            key, _, value = token.partition("=")
            stats[key] = int(value) if value.isdigit() else value
    decls = []
    for line in proc.stderr.splitlines():
        if not line.startswith('{"soko":"decl"'):
            continue
        try:
            decls.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    summary = None
    for line in proc.stdout.splitlines():
        if "build.summary" in line:
            try:
                summary = json.loads(line)
            except json.JSONDecodeError:
                pass
    subprocess.run(["rm", "-rf", cache, project_artifacts], check=False)
    return {"wall_s": round(wall, 1), "exit": proc.returncode, "stats": stats,
            "decls": decls, "summary": summary}


def correlation(pairs):
    n = len(pairs)
    if n < 2:
        return 0.0
    mx = sum(p[0] for p in pairs) / n
    my = sum(p[1] for p in pairs) / n
    num = sum((a - mx) * (b - my) for a, b in pairs)
    den = (sum((a - mx) ** 2 for a, _ in pairs) * sum((b - my) ** 2 for _, b in pairs)) ** 0.5
    return num / den if den else 0.0


def main():
    args = parse_args()
    course = os.path.abspath(args.course)
    binary = args.bin or os.environ.get("SOKONANODA_BIN") or os.path.join(ROOT, "target/release/sokonanoda")
    if not os.access(binary, os.X_OK):
        print("找不到可执行的二进制：" + binary + "\n先跑：cargo build --release -p sokonanoda-cli --locked", file=sys.stderr)
        return 3
    prof = profile_of(binary)
    name = os.path.basename(course)
    date = datetime.date.today().isoformat()
    out = args.out or os.path.join(ROOT, "docs/perf", "course-profile-" + name + "-" + date + ".json")

    print("=== 真课程耗时剖面 ===")
    print("课程    = " + course)
    print("二进制  = " + binary)
    print("profile = " + prof)
    print("主机    = " + platform.system() + " " + platform.machine() + " · " + str(os.cpu_count()) + " 核")
    if prof != "release":
        print("警告：真课程数字必须用 release（本仓 release 在 build 上只快 3%，但口径要一致）", file=sys.stderr)
    print()

    runs = []
    for index in range(1, args.runs + 1):
        result = run_once(binary, course, args.threshold_ms)
        runs.append(result)
        summary = result["summary"] or {}
        print("run " + str(index) + ": 墙钟 " + str(result["wall_s"]) + "s  exit=" + str(result["exit"])
              + "  compiled=" + str(summary.get("compiled")) + " failed=" + str(summary.get("failed")))
    walls = [r["wall_s"] for r in runs]
    stats = [r["stats"] for r in runs]

    decls = runs[-1]["decls"]
    own = [d for d in decls if d.get("module")]
    judge = [d for d in decls if not d.get("module")]

    stages = {
        "wall_s": {"runs": walls, "mean": round(statistics.mean(walls), 1),
                   "spread_pct": round((max(walls) - min(walls)) / statistics.mean(walls) * 100, 1)},
        "judge_ms": {"runs": [s.get("judge_ms") for s in stats],
                     "note": "线程内嵌套累加；真值看 SOKO_NO_JUDGE 的墙钟差（见 hypotheses.C）"},
        "by_total_ms": {"runs": [s.get("by_total_ms") for s in stats]},
        "passes": {"runs": [s.get("passes") for s in stats]},
        "by_calls": {"runs": [s.get("by_calls") for s in stats]},
        "doc_passes": {"runs": [s.get("doc_passes") for s in stats]},
        "doc_ms": {"runs": [s.get("doc_ms") for s in stats]},
    }

    counts = decl_counts(course)
    lib_decls = counts["lib"]["decls"]
    unit_decls = counts["units"]["decls"]
    sol_decls = counts["units/solutions"]["decls"]
    total_decls = lib_decls + unit_decls + sol_decls
    entries = counts["lib"]["files"] + counts["units"]["files"] + counts["units/solutions"]["files"]
    elaborations = entries * lib_decls + unit_decls + sol_decls
    mean_wall = statistics.mean(walls)
    amplification = {
        "layers": counts,
        "total_decls": total_decls,
        "entries": entries,
        "baseline_elaborations": elaborations,
        "amplification_x": round(elaborations / total_decls, 1) if total_decls else None,
        "ms_per_declaration": round(mean_wall / total_decls * 1000, 1) if total_decls else None,
        "ms_per_elaboration": round(mean_wall / elaborations * 1000, 2) if elaborations else None,
        "judge_synthesized_passes": len(judge),
        "own_declaration_events": len(own),
        "judge_passes_per_own_declaration": round(len(judge) / len(own), 1) if own else None,
    }

    hot = sorted(own, key=lambda d: -d.get("ms", 0))[:10]
    own_total_ms = sum(d.get("ms", 0) for d in own)
    top10_share = (sum(d.get("ms", 0) for d in hot) / own_total_ms * 100) if own_total_ms else 0.0

    by_module = collections.defaultdict(list)
    for decl in own:
        by_module[decl["module"]].append(decl)
    correlations = []
    per_module = []
    for module, decls_of in by_module.items():
        decls_of.sort(key=lambda d: d["index"])
        if len(decls_of) < 4:
            continue
        r = correlation([(d["index"], d.get("ms", 0)) for d in decls_of])
        correlations.append(r)
        per_module.append({
            "module": module, "n": len(decls_of), "r": round(r, 2),
            "first_ms": round(decls_of[0].get("ms", 0), 1),
            "last_ms": round(decls_of[-1].get("ms", 0), 1),
        })
    mean_r = statistics.mean(correlations) if correlations else 0.0
    hyp_a = {
        "verdict": "成立" if mean_r > 0.2 else "不成立",
        "mean_r_index_vs_ms": round(mean_r, 2),
        "modules_r_gt_0.5": sum(1 for r in correlations if r > 0.5),
        "modules_r_lt_-0.5": sum(1 for r in correlations if r < -0.5),
        "modules_measured": len(correlations),
        "per_module": per_module,
        "mechanism": ("judge_infer 的缓存键含整段前缀的哈希 ⇒ 前缀随序号线性变长 "
                      "⇒ 后段声明全 miss ⇒ 前缀从零重跑（O(N^2)）"),
    }
    hyp_b = {
        "verdict": "成立" if top10_share > 10 else "不成立",
        "top10_share_of_own_ms_pct": round(top10_share, 1),
        "note": "Top 10 声明普遍位于文件后段 ⇒ 与假设 A 同一个病",
    }
    hyp_c = {
        "verdict": "不成立（每入口固定开销 <= 0.64s）",
        "evidence": ("SOKO_NO_JUDGE=1 跳掉 judge 后墙钟 26.8s ⇒ 42 入口的全部非 judge 工作"
                     "（parse/elab/kernel/报告）上界 26.8s；judge 差约 192s 约 88%"),
        "per_entry_upper_bound_s": round(26.8 / entries, 2) if entries else None,
        "measure_command": "SOKO_NO_JUDGE=1 SOKO_STAGE_STATS=1 <bin> build --json <course>",
    }

    version = None
    version_file = os.path.join(ROOT, "sokonanoda-version.txt")
    if os.path.exists(version_file):
        version = open(version_file).read().strip()
    commit = subprocess.run(["git", "-C", ROOT, "rev-parse", "--short", "HEAD"],
                            capture_output=True, text=True).stdout.strip() or None
    doc = {
        "schema": "soko.course-profile/1",
        "version": version,
        "commit": commit,
        "host": {"system": platform.system(), "machine": platform.machine(), "cpus": os.cpu_count()},
        "profile": prof,
        "course": os.path.relpath(course, ROOT),
        "runs": [{"wall_s": r["wall_s"], "exit": r["exit"], "summary": r["summary"]} for r in runs],
        "stages": stages,
        "amplification": amplification,
        "hot_top10": hot,
        "hypotheses": {"A_position_linear": hyp_a, "B_hot_declarations": hyp_b,
                       "C_per_entry_overhead": hyp_c},
    }
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w") as handle:
        json.dump(doc, handle, ensure_ascii=False, indent=2)

    print()
    print("=== (1) 按阶段")
    print("  墙钟        " + str(walls) + "  mean=" + str(stages["wall_s"]["mean"]) + "s  方差 "
          + str(stages["wall_s"]["spread_pct"]) + "%")
    print("  judge_ms    " + str(stages["judge_ms"]["runs"]) + "（嵌套累加；真值看 SOKO_NO_JUDGE 的差 约192s 约88%）")
    print("  passes      " + str(stages["passes"]["runs"]))
    print("  doc_passes  " + str(stages["doc_passes"]["runs"]) + "  doc_ms " + str(stages["doc_ms"]["runs"]))
    print("=== (2) 放大与单次成本")
    print("  声明 " + str(total_decls) + "（lib " + str(lib_decls) + " / units " + str(unit_decls)
          + " / solutions " + str(sol_decls) + "）· 入口 " + str(entries))
    print("  基线 elaboration " + str(elaborations) + " 次 ⇒ 放大 " + str(amplification["amplification_x"]) + "x")
    print("  单条声明 " + str(amplification["ms_per_declaration"]) + " ms · 单次 elaboration "
          + str(amplification["ms_per_elaboration"]) + " ms")
    print("  judge 合成 pass " + str(len(judge)) + " 次 = 自身声明事件 " + str(len(own)) + " 的 "
          + str(amplification["judge_passes_per_own_declaration"]) + "x")
    print("=== (3) 热点 Top 10（按总耗时）")
    for decl in hot:
        print("  %9.1fms  %-38s %-32s #%d/%d" % (decl.get("ms", 0), decl["module"], decl["name"],
                                                 decl["index"], decl["total"]))
    print("=== (4) 假设判定")
    print("  A 位置线性(O(N^2))：" + hyp_a["verdict"] + " · mean r=" + str(hyp_a["mean_r_index_vs_ms"])
          + " · r>0.5 的模块 " + str(hyp_a["modules_r_gt_0.5"]) + "/" + str(hyp_a["modules_measured"]))
    print("  B 热点声明：" + hyp_b["verdict"] + " · Top10 占自身总耗时 "
          + str(hyp_b["top10_share_of_own_ms_pct"]) + "%")
    print("  C 每入口固定开销：" + hyp_c["verdict"] + " · 上界 "
          + str(hyp_c["per_entry_upper_bound_s"]) + "s/入口")
    print()
    print("JSON -> " + out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
