#!/usr/bin/env python3
"""课程清单 v2 与判据 **G6**（清单自洽）的单测（台账 G-07）。

跑法：

```bash
python3 courses/set-theory/tools/test_manifest_v2.py     # exit 0 = 全绿
```

为什么单独一个文件：`check.py --selftest` 需要一台**真的判卷二进制**（它跑真
`grade`），而 G6 是**纯清单逻辑**——它不该为了被测试而先要求一个工具链。
这里用 `importlib` 直接加载 `check.py` 的模块对象，再喂给它**临时课程目录**与
一份**假通道**（`Channel(argv=[…])` 从不被 G6 路径调用），于是这些用例是
秒级、零依赖、可在任何机器上跑的。

覆盖：

* **展平**：v1 数组 / v2 对象 → 同一个单元列表（顺序、标题、单元号）；
* **G6 判红的四类结构非法**：volume id 重复、chapter id 重复、
  同一 unit 挂在两个章、`prereqs` 指向不存在的 chapter id；
* **G6 只报告不判红的**：`quota.exercises` 与画布练习数的差额、空章；
* **G1–G5 语义不变**：展平后的目标标签/顺序与 v1 清单逐个相同；
* **成本台账**（`docs/courses/ledger.jsonl`，设计 §4.6）：合成记录的字段齐全，
  已提交的台账逐行合法、字段齐全、`date`/`version` 形状对（没跑过 `--ledger`
  就没有文件——默认关闭；`version` 只判 `x.y.z`，历史条目允许与当前版本钉不同）。
"""

from __future__ import annotations

import importlib.util
import json
import re
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
COURSE = HERE.parent

spec = importlib.util.spec_from_file_location("course_check", HERE / "check.py")
assert spec and spec.loader, "check.py must be importable"
check = importlib.util.module_from_spec(spec)
# dataclasses 会在装饰时回查 `sys.modules[cls.__module__]`，所以必须先注册再 exec。
sys.modules[spec.name] = check
spec.loader.exec_module(check)

FAILURES: list[str] = []


def fail(message: str) -> None:
    FAILURES.append(message)


def expect(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


# ── 夹具：一个最小课程目录（lib + units + 画布 + 解答）──────────────────────

LIB_SRC = "def fixtureId (P : Prop) : Prop := P\n"
UNIT_SRC = "import lib.Logic\n\ndef u (P : Prop) : Prop := fixtureId P\n"
# 每份解答多两个声明（G3 要 checked>0，G4 要覆盖画布的每个具名练习）。
SOLUTION_SRC = UNIT_SRC + "\ntheorem solved_a (P : Prop) : P -> P := fun h => h\ntheorem solved_b (P : Prop) : P -> P := fun h => h\n"


def fixture(root: Path, manifest: dict | list, unit_names: list[str]) -> Path:
    (root / "lib").mkdir(parents=True)
    (root / "units" / "solutions").mkdir(parents=True)
    (root / "lib" / "Logic.sokonanoda").write_text(LIB_SRC, encoding="utf-8")
    for name in unit_names:
        (root / "units" / f"{name}.sokonanoda").write_text(UNIT_SRC, encoding="utf-8")
        (root / "units" / "solutions" / f"{name}-solution.sokonanoda").write_text(
            SOLUTION_SRC, encoding="utf-8"
        )
    path = root / "course.json"
    path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    return path


def unit(file: str, number: int, title: str = "") -> dict:
    return {"file": file, "title": title or f"单元 {number}", "unit": number}


def run_discover(manifest_path: Path, course_dir: Path):
    """在**临时课程目录**上跑 discover()：只换课程根，判据代码一行不改。

    返回 `(targets, problems, unit_count, info, units)`——门禁的完整发现结果。
    """
    return check.discover(course_dir)


def discover_parts(manifest_path: Path, course_dir: Path):
    """`(targets, problems, unit_count)`——大多数用例只关心这三样。"""
    targets, problems, unit_count, _info, _units = run_discover(manifest_path, course_dir)
    return targets, problems, unit_count


def structure_problems(problems: list[str]) -> list[str]:
    return [p for p in problems if p.startswith("G6")]


# ── ① 展平：v1 与 v2 给出同一个单元列表 ─────────────────────────────────────

V2 = {
    "schema": "soko.course/2",
    "name": "fixture",
    "title": "Fixture",
    "volumes": [
        {
            "id": "I",
            "title": "Volume One",
            "chapters": [
                {
                    "id": "I.1",
                    "title": "First",
                    "prereqs": [],
                    "tags": ["alpha"],
                    "quota": {"exercises": 2},
                    "units": [unit("units/a.sokonanoda", 1, "A"), unit("units/b.sokonanoda", 2, "B")],
                },
                {
                    "id": "I.2",
                    "title": "Second",
                    "prereqs": ["I.1"],
                    "tags": ["beta"],
                    "quota": {"exercises": 1},
                    "units": [unit("units/c.sokonanoda", 3, "C")],
                },
            ],
        }
    ],
}

V1 = [
    unit("units/a.sokonanoda", 1, "A"),
    unit("units/b.sokonanoda", 2, "B"),
    unit("units/c.sokonanoda", 3, "C"),
]


def case_flatten_matches_v1() -> None:
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        v1_path = fixture(root / "v1", V1, ["a", "b", "c"])
        v2_path = fixture(root / "v2", V2, ["a", "b", "c"])
        v1_targets, v1_problems, v1_units = discover_parts(v1_path, root / "v1")
        v2_targets, v2_problems, v2_units = discover_parts(v2_path, root / "v2")

        labels = lambda targets: [(t.label, t.kind, t.unit) for t in targets]
        expect(
            labels(v1_targets) == labels(v2_targets),
            f"v2 展平后的目标必须与 v1 逐个相同：\n  v1={labels(v1_targets)}\n  v2={labels(v2_targets)}",
        )
        expect(v1_units == 3 and v2_units == 3, f"单元数：v1={v1_units} v2={v2_units}")
        expect(not v1_problems, f"v1 清单不该有结构问题：{v1_problems}")
        expect(not v2_problems, f"合法的 v2 清单不该判负：{v2_problems}")


def case_v2_units_keep_v1_shape() -> None:
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        path = fixture(root, V2, ["a", "b", "c"])
        targets, problems, _ = discover_parts(path, root)
        names = [t.path.name for t in targets if t.kind == "unit"]
        expect(
            names == ["a.sokonanoda", "b.sokonanoda", "c.sokonanoda"],
            f"展平顺序 = 文档顺序（卷→章→单元）：{names}",
        )
        expect(not problems, f"合法 v2 清单：{problems}")


# ── ② G6 判红的四类结构非法 ────────────────────────────────────────────────


def case_duplicate_chapter_id_is_rejected() -> None:
    manifest = json.loads(json.dumps(V2))
    manifest["volumes"][0]["chapters"][1]["id"] = "I.1"
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        path = fixture(root, manifest, ["a", "b", "c"])
        _, problems, _ = discover_parts(path, root)
        g6 = structure_problems(problems)
        expect(
            any("I.1" in p and "重" in p or "duplicate" in p for p in g6),
            f"重复的 chapter id 必须判红：{problems}",
        )


def case_duplicate_volume_id_is_rejected() -> None:
    manifest = json.loads(json.dumps(V2))
    second = json.loads(json.dumps(manifest["volumes"][0]))
    second["chapters"] = []
    manifest["volumes"].append(second)
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        path = fixture(root, manifest, ["a", "b", "c"])
        _, problems, _ = discover_parts(path, root)
        expect(
            any(p.startswith("G6") and "I" in p for p in problems),
            f"重复的 volume id 必须判红：{problems}",
        )


def case_unit_in_two_chapters_is_rejected() -> None:
    manifest = json.loads(json.dumps(V2))
    manifest["volumes"][0]["chapters"][1]["units"].append(unit("units/a.sokonanoda", 1, "A"))
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        path = fixture(root, manifest, ["a", "b", "c"])
        _, problems, _ = discover_parts(path, root)
        expect(
            any("units/a.sokonanoda" in p for p in structure_problems(problems)),
            f"同一 unit 挂在两个章必须判红：{problems}",
        )


def case_prereq_to_unknown_chapter_is_rejected() -> None:
    manifest = json.loads(json.dumps(V2))
    manifest["volumes"][0]["chapters"][1]["prereqs"] = ["I.1", "I.9"]
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        path = fixture(root, manifest, ["a", "b", "c"])
        _, problems, _ = discover_parts(path, root)
        expect(
            any("I.9" in p for p in structure_problems(problems)),
            f"prereqs 指向不存在的 chapter id 必须判红：{problems}",
        )


def case_missing_file_or_unit_is_rejected() -> None:
    manifest = json.loads(json.dumps(V2))
    manifest["volumes"][0]["chapters"][0]["units"].append({"title": "没有 file"})
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        path = fixture(root, manifest, ["a", "b", "c"])
        _, problems, _ = discover_parts(path, root)
        expect(
            any("file" in p for p in structure_problems(problems)),
            f"v2 里缺 file/unit 的条目必须判红（G2 语义不变）：{problems}",
        )


# ── ③ G6 只报告不判红：配额差额 / 空章 ──────────────────────────────────────


def case_quota_difference_is_reported_not_rejected() -> None:
    manifest = json.loads(json.dumps(V2))
    # 画布上没有 sorry ⇒ 实际练习数 0，配额写 99：**只报告**。
    manifest["volumes"][0]["chapters"][0]["quota"] = {"exercises": 99}
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        path = fixture(root, manifest, ["a", "b", "c"])
        targets, problems, _ = discover_parts(path, root)
        expect(not problems, f"配额差额绝不判红（只判形状、不锁计数）：{problems}")
        notes = check.quota_notes(manifest, path)
        expect(
            any("99" in note for note in notes),
            f"差额必须被**报告**出来：{notes}",
        )
        # 有实测时给出差额本身（画布 0 道 vs 计划 99 ⇒ -99）。
        measured = check.quota_notes(
            manifest, path, {u["file"]: 0 for u in manifest["volumes"][0]["chapters"][0]["units"]}
        )
        expect(
            any("-99" in note for note in measured),
            f"实测差额要算出来：{measured}",
        )


def case_empty_chapter_is_allowed() -> None:
    manifest = json.loads(json.dumps(V2))
    manifest["volumes"][0]["chapters"].append(
        {"id": "I.3", "title": "Empty", "prereqs": ["I.2"], "tags": [], "units": []}
    )
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        path = fixture(root, manifest, ["a", "b", "c"])
        _, problems, _ = discover_parts(path, root)
        expect(not problems, f"空章是合法的生长状态（只报告）：{problems}")


def case_v1_manifest_has_no_g6_problems() -> None:
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        path = fixture(root, V1, ["a", "b", "c"])
        _, problems, _ = discover_parts(path, root)
        expect(not problems, f"v1 扁平清单不受 G6 影响（向后兼容）：{problems}")
        expect(check.quota_notes(V1, path) == [], "v1 没有配额可报告")


def case_unknown_schema_is_a_prerequisite_error() -> None:
    with tempfile.TemporaryDirectory(prefix="soko-g6-") as tmp:
        root = Path(tmp)
        path = fixture(root, {"schema": "soko.course/99", "volumes": []}, ["a"])
        try:
            run_discover(path, root)
        except check.Prerequisite as error:
            expect("soko.course/99" in str(error), f"报错要点名 schema：{error}")
            return
        fail("未知 schema 必须走 Prerequisite（不判绿、也不静默当 v1）")


def case_cost_ledger_fields_and_committed_file() -> None:
    """成本台账（设计 course-manifest-v2.md §4.6）：字段齐全 + 已提交的台账合法。

    `--ledger` 默认关闭（CI 不往仓库里写文件），所以这里判两件事：
    ① 合成一条记录必须带齐 `LEDGER_FIELDS`（日期/课程名/目标数/checked/open/
    判负/用时 ms/版本），且值就是给进去的；
    ② 仓库里**已经**有 `docs/courses/ledger.jsonl` 时，每一行都是合法 JSON、
    字段齐全、`date`/`version` 形状对（人工追加的那一次不许写坏）。

    `version` 只判形状 `x.y.z`、**不**要求等于当前版本钉：台账是**历史**，版本
    bump 之后旧条目天然带着旧版本号——拿它跟当前钉比会在每次 bump 时假红。
    """
    entry = json.loads(check.ledger_entry(
        [{"label": "单元 1", "status": "ok", "checked": 2, "open": 1}],
        {"targets": 1, "checked": 2, "open": 1, "rejected": 0, "solutions_open": 0},
        check.repo_root() or check.COURSE, course="fixture", elapsed_ms=42,
    ))
    missing = [key for key in check.LEDGER_FIELDS if key not in entry]
    expect(not missing, f"成本台账缺字段：{missing}")
    expect(entry.get("course") == "fixture", f"课程名要原样写进去：{entry}")
    expect(entry.get("elapsed_ms") == 42, f"用时 ms 要原样写进去：{entry}")
    expect(entry.get("rejected") == 0 and entry.get("targets") == 1,
           f"目标数/判负要对上 summary：{entry}")

    ledger = (check.repo_root() or check.COURSE) / check.LEDGER_DEFAULT
    if not ledger.is_file():
        return  # 没跑过 --ledger 就没有文件——默认关闭，这不是失败
    for number, line in enumerate(ledger.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            fail(f"{ledger.name} 第 {number} 行不是 JSON：{error}")
            continue
        missing = [key for key in check.LEDGER_FIELDS if key not in record]
        expect(not missing, f"{ledger.name} 第 {number} 行缺字段：{missing}")
        expect(record.get("schema") == check.LEDGER_SCHEMA,
               f"{ledger.name} 第 {number} 行 schema 不对：{record.get('schema')!r}")
        version = record.get("version")
        expect(version is None or (isinstance(version, str) and re.fullmatch(r"\d+\.\d+\.\d+", version)),
               f"{ledger.name} 第 {number} 行 version 不是 x.y.z（历史条目允许与当前钉不同）：{version!r}")
        date = record.get("date")
        expect(isinstance(date, str) and re.fullmatch(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z", date),
               f"{ledger.name} 第 {number} 行 date 不是 UTC ISO-8601：{date!r}")
        expect(isinstance(record.get("elapsed_ms"), int) and record["elapsed_ms"] >= 0,
               f"{ledger.name} 第 {number} 行 elapsed_ms 不是非负整数：{record.get('elapsed_ms')!r}")


CASES = [
    case_flatten_matches_v1,
    case_v2_units_keep_v1_shape,
    case_duplicate_chapter_id_is_rejected,
    case_duplicate_volume_id_is_rejected,
    case_unit_in_two_chapters_is_rejected,
    case_prereq_to_unknown_chapter_is_rejected,
    case_missing_file_or_unit_is_rejected,
    case_quota_difference_is_reported_not_rejected,
    case_empty_chapter_is_allowed,
    case_v1_manifest_has_no_g6_problems,
    case_unknown_schema_is_a_prerequisite_error,
    case_cost_ledger_fields_and_committed_file,
]


def main() -> int:
    for case in CASES:
        before = len(FAILURES)
        try:
            case()
        except Exception as error:  # 用例自己炸了也算失败（不许静默）
            fail(f"{case.__name__} 抛异常：{type(error).__name__}: {error}")
        print(f"{'ok  ' if len(FAILURES) == before else 'FAIL'} {case.__name__}")
    if FAILURES:
        print(f"\n{len(FAILURES)} 个断言失败：")
        for line in FAILURES:
            print(f"  ✗ {line}")
        return 1
    print(f"\n{len(CASES)}/{len(CASES)} passed（G6 与 v2 展平）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
