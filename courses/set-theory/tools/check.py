#!/usr/bin/env python3
"""课程门禁（卷 I《集合论》）：判据 G1–G6，**与课程规模无关**。

设计：`docs/design/course-gate-in-ci.md`（判据 §3、三条坑 §4、输出 §5）、
`docs/design/course-manifest-v2.md`（G6 / 清单 v2，台账 G-07）。

判据（判红的**只有**这六条；计数从不参与判红）：

* **G1** 每个目标 `grade` 退出码 0（G-10：判据只认 `grade` 的退出码，不认 `query check`）；
* **G2** 目标存在：`course.json` 条目、`lib/`、每个画布（含非单元页面）的解答文件都在；
* **G3** 解答：`exercise.open == 0` 且 `decl.checked > 0`（退出码对合法 open 返回 0，
  只靠 G1 抓不到没填的洞）；
* **G4** 解答覆盖：画布每个具名 `exercise.open.name` 都能在对应解答的
  `decl.checked.name` 里找到（防止"删题代替填洞"）。**非单元页面**
  （`units/` 下不进 `course.json` 的画布，如记法对照页）走**同一套** G1/G3/G4：
  它们不是单元（没有单元号/配额），但同样是"有 `sorry` 的画布 + 有解答"。
* **G5** `lib` + Demo：`exercise.open == 0`（库里有 `sorry` 会让引用它的单元判卷失真）。
* **G6** 清单自洽（v2；v1 扁平清单天然满足）：volume/chapter id 唯一且非空、
  每个 unit 恰好属于一个 chapter（`file` 不重复）、`prereqs` 指向存在的 chapter id。
  **`quota.exercises` 与画布实际练习数的差额只报告、绝不判红**——课程门禁的
  设计原则是「只判形状、不锁计数」（设计 §2/§4.2）。

清单两种格式都读（台账 G-07，`docs/design/course-manifest-v2.md`）：v1 扁平数组与
v2 `{schema, volumes[].chapters[].units[]}`；`flatten_manifest()` 把 v2 展平成
**与 v1 同形**的单元列表，所以 G1–G5 的判定代码一行未改。

三条坑的工程化：

* **G-10**（`query check` 假绿）：判据只走 `grade`；`--selftest` 用一份故意坏的单元
  走**同一个判卷函数**，它必须被判负——判据通道失效时门禁自己红；
* **G-12**（相对路径打空模块根）：入口一律 `str(path.resolve())`；`--selftest` 从另一个
  cwd 再判一个带 `import lib.*` 的目标；
* **G-15 已修（WO-010，0.59.0）**（诊断位置自带单位与坐标空间）：失败信息三段式——权威段（标签 + 绝对路径 + 退出码）、
  参考段（诊断原样，标注 span 只作参考）、定位段（`--only <标签> --bisect`：按顶层
  声明边界二分，结论**不依赖诊断 span**）。

用法：

```bash
python3 courses/set-theory/tools/check.py                    # 人读表 + 汇总
python3 courses/set-theory/tools/check.py --selftest         # 判据通道自检（G-12 cwd 自检 + G6 清单自检）
python3 courses/set-theory/tools/check.py --json             # 机器可读（含计数）
python3 courses/set-theory/tools/check.py --only "单元 5" --bisect   # 二分定位
python3 courses/set-theory/tools/check.py --report /tmp/gate.json --summary "$GITHUB_STEP_SUMMARY" --annotations
```

退出码：**0** 全绿 / **1** 有目标被判负 / **2** 前置缺失或用法错误（无法判定 ≠ 绿）。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

COURSE = Path(__file__).resolve().parent.parent
COURSE_JSON = COURSE / "course.json"
LIB = COURSE / "lib"
UNITS = COURSE / "units"
SOLUTION_DIR = COURSE / "units" / "solutions"
SCHEMA = "soko.course.check/2"
LEDGER_SCHEMA = "soko.course-ledger/1"
LEDGER_DEFAULT = "docs/courses/ledger.jsonl"
GRADE_TIMEOUT = 180  # 单个目标判卷的墙钟上限（秒）；与课程规模无关

# 顶层声明的行首关键字（--bisect 的边界；设计 §8 未决①：namespace/section 也算边界）。
DECL_KEYWORDS = ("theorem", "def", "axiom", "inductive", "example", "namespace", "section")
DECL_START = re.compile(r"^(?:" + "|".join(DECL_KEYWORDS) + r")\b")


# ── 前置：判卷通道（§4.4：二进制必须与仓库同版本）──────────────────────────


class Prerequisite(Exception):
    """前置缺失/不可信：不判绿，退出码 2。"""


@dataclass(frozen=True)
class Channel:
    """判卷命令前缀 + 可信来源说明。"""

    argv: list[str]
    source: str
    path: str
    version: str | None


def repo_root() -> Path | None:
    for parent in [COURSE, *COURSE.parents]:
        if (parent / "scripts" / "soko").is_file():
            return parent
    return None


def binary_version(binary: Path) -> str | None:
    """二进制自报的版本；跑不起来/不说 `--version` ⇒ None。"""
    try:
        proc = subprocess.run([str(binary), "--version"], capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if proc.returncode != 0:
        return None
    match = re.search(r"(\d+\.\d+\.\d+)", proc.stdout or "")
    return match.group(1) if match else None


def repo_pin(root: Path) -> tuple[str | None, str]:
    """本检出的版本钉：`sokonanoda-version.txt` → `Cargo.toml`（课程仓没有 Cargo.toml）。"""
    version_txt = root / "sokonanoda-version.txt"
    if version_txt.is_file():
        for line in version_txt.read_text(encoding="utf-8").splitlines():
            value = line.strip()
            if not value or value.startswith("#"):
                continue
            match = re.fullmatch(r"v?(\d+\.\d+\.\d+)", value)
            if match:
                return match.group(1), version_txt.name
            break
    cargo = root / "Cargo.toml"
    if cargo.is_file():
        match = re.search(r'^version\s*=\s*"([^"]+)"', cargo.read_text(encoding="utf-8"), re.M)
        if match:
            return match.group(1), cargo.name
    return None, ""


def resolve_channel(root: Path, explicit: str | None) -> Channel:
    """解析判卷命令：显式 `--bin`/`SOKONANODA_BIN` → 直用（比对版本）；否则走启动器。"""
    override = explicit or os.environ.get("SOKONANODA_BIN")
    if override:
        binary = Path(override)
        if not binary.is_file():
            raise Prerequisite(f"--bin/SOKONANODA_BIN 指向的文件不存在：{binary}")
        reported = binary_version(binary)
        if reported is None:
            raise Prerequisite(f"判卷二进制跑不起来（或不回答 --version）：{binary}")
        pin, pin_source = repo_pin(root)
        if pin and reported != pin:
            raise Prerequisite(
                f"判卷二进制与仓库版本不一致，结果不可信：\n"
                f"  {binary} 自报 v{reported}，仓库钉是 v{pin}（{pin_source}）。\n"
                f"  请用当轮构建（`cargo build -p sokonanoda-cli --locked`）或 `scripts/soko update`。"
            )
        note = f"override（对上 {pin_source} 的 v{pin}）" if pin else "override（本检出没有版本钉，跳过比对）"
        return Channel([str(binary)], note, str(binary), reported)

    soko = root / "scripts" / "soko"
    if not soko.is_file():
        raise Prerequisite(f"找不到启动器 {soko}；也没有给出 --bin/SOKONANODA_BIN")
    node = shutil.which("node")
    if node is None:
        raise Prerequisite("找不到 node（`scripts/soko` 是 node 启动器）；或显式给出 --bin <sokonanoda>")
    proc = subprocess.run([node, str(soko), "version", "--json"], capture_output=True, text=True,
                          cwd=str(root), timeout=60)
    try:
        report = json.loads(proc.stdout)
    except json.JSONDecodeError:
        raise Prerequisite(
            f"`scripts/soko version --json` 没有给出 JSON（exit {proc.returncode}）："
            f"{(proc.stderr or proc.stdout).strip().splitlines()[-1] if (proc.stderr or proc.stdout).strip() else '无输出'}"
        ) from None
    cli = report.get("cli") or {}
    path, source = cli.get("path"), str(cli.get("source") or "")
    if not report.get("version"):
        raise Prerequisite(
            f"启动器解析不出期望版本（{report.get('version_error') or '没有版本钉'}）——判卷结果不可信。"
        )
    if not path or not Path(path).is_file():
        raise Prerequisite(f"启动器没有解析出可执行的 CLI（{path or 'nothing'} [{source or '?'}]）——不判绿。")
    if "STALE" in source or "unknown" in source or "unverified" in source:
        raise Prerequisite(
            f"启动器解析到的二进制不可信：{path} [{source}]。\n"
            f"  先跑 `scripts/soko update`（或 `scripts/soko doctor --json`）再重试——不判绿。"
        )
    return Channel([path], source, path, report.get("version"))


# ── 判卷（G-10：只认 `grade` 的退出码；G-12：只传绝对路径）─────────────────


@dataclass
class GradeResult:
    code: int
    checked: int = 0
    open: int = 0
    checked_names: list[str] = field(default_factory=list)
    open_names: list[str] = field(default_factory=list)
    diagnostics: list[dict] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)


def grade(channel: Channel, path: Path, *, cwd: Path | None = None) -> GradeResult:
    """判一个文件：`grade <绝对路径> --json`（退出码是唯一判据，事件流只用来计数/报错）。"""
    argv = [*channel.argv, "grade", "--json", str(path.resolve())]
    try:
        proc = subprocess.run(argv, capture_output=True, text=True,
                              cwd=str(cwd or COURSE), timeout=GRADE_TIMEOUT)
    except subprocess.TimeoutExpired:
        return GradeResult(code=124, notes=[f"判卷超过 {GRADE_TIMEOUT}s 未返回"])
    except OSError as error:
        return GradeResult(code=125, notes=[f"判卷命令跑不起来：{error}"])

    result = GradeResult(code=proc.returncode)
    for line in proc.stdout.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        kind = event.get("type")
        if kind == "decl.checked":
            result.checked += 1
            if event.get("name"):
                result.checked_names.append(str(event["name"]))
        elif kind == "exercise.open":
            result.open += 1
            if event.get("name"):
                result.open_names.append(str(event["name"]))
        elif kind == "diagnostic":
            result.diagnostics.append({key: event.get(key) for key in ("stage", "code", "message")})
    if not result.diagnostics:
        tail = [line for line in (proc.stderr or "").splitlines() if line.strip()]
        if tail:
            result.notes.append(tail[-1].strip())
    return result


_BISECT_SERIAL = 0


def grade_text(channel: Channel, origin: Path, text: str) -> GradeResult:
    """判一段文本（--bisect 的前缀）。

    临时文件必须落在 **origin 同目录**：`grade` 不支持 `--root`，模块根靠祖先清单发现
    （G-12 的安全形状）。文件名不带 `.sokonanoda` 后缀，任何 glob 都不会捡到它。
    """
    global _BISECT_SERIAL
    _BISECT_SERIAL += 1
    tmp = origin.parent / f".soko-bisect-{os.getpid()}-{_BISECT_SERIAL}.tmp"
    try:
        tmp.write_text(text, encoding="utf-8")
        return grade(channel, tmp)
    finally:
        tmp.unlink(missing_ok=True)


# ── 清单：v1 扁平数组 / v2 结构化对象（台账 G-07）────────────────────────────

MANIFEST_SCHEMA_V2 = "soko.course/2"


@dataclass
class Chapter:
    """v2 的一章：id/title + 先修 + 标签 + 配额 + 它的单元（保持文档序）。"""

    id: str
    title: str
    prereqs: list[str] = field(default_factory=list)
    tags: list[str] = field(default_factory=list)
    quota: int | None = None
    units: list[dict] = field(default_factory=list)
    volume_id: str = ""
    volume_title: str = ""


@dataclass
class ManifestInfo:
    """清单的结构面：v1 时为全零/空（向后兼容：v1 没有结构可谈）。"""

    source: Path
    volumes: int = 0
    chapters: list[Chapter] = field(default_factory=list)
    v2: bool = False


def _string_list(value) -> list[str]:
    if not isinstance(value, list):
        return []
    return [item for item in value if isinstance(item, str)]


def _quota_of(chapter: dict) -> int | None:
    quota = chapter.get("quota")
    if not isinstance(quota, dict):
        return None
    exercises = quota.get("exercises")
    return exercises if isinstance(exercises, int) and not isinstance(exercises, bool) else None


def flatten_manifest(value, source: Path) -> tuple[list[dict], list[str], ManifestInfo]:
    """把清单（v1 数组 / v2 对象）展平成 `(units, problems, info)`。

    展平后的 `units` 与 v1 条目**同形**（`file`/`title`/`title_en`/`unit`），
    另外挂两个只在门禁内部用的键：`_chapter`（章 id，v1 为空串）与
    `_volume`（卷 id）——G1–G5 的判定代码只读 v1 的那几个键，一行未改。

    **G6 的判红在这里产出**（结构非法：id 重复/缺失、unit 重复、prereqs 悬空），
    `problems` 走与 G2 同一条通道 ⇒ 判定/退出码/`--json` 自动跟上。
    """
    info = ManifestInfo(source=source)

    if isinstance(value, list):
        units = [entry for entry in value if isinstance(entry, dict)]
        return units, [], info

    if not isinstance(value, dict):
        raise Prerequisite("course.json 既不是数组（v1）也不是对象（v2）")

    schema = value.get("schema")
    if schema is not None and schema != MANIFEST_SCHEMA_V2:
        raise Prerequisite(
            f"course.json 的 schema 是 {schema!r}，只认 {MANIFEST_SCHEMA_V2!r}"
            f"（或没有 schema 的 v1 扁平数组）——不猜"
        )

    volumes = value.get("volumes", [])
    if not isinstance(volumes, list):
        raise Prerequisite("course.json 的 volumes 必须是数组")
    info.v2 = True
    info.volumes = len(volumes)

    problems: list[str] = []
    units: list[dict] = []
    seen_volumes: set[str] = set()
    seen_chapters: set[str] = set()
    seen_files: dict[str, str] = {}

    for index, volume in enumerate(volumes):
        if not isinstance(volume, dict):
            problems.append(f"G6：volumes[{index}] 不是对象")
            continue
        volume_id = volume.get("id")
        if not isinstance(volume_id, str) or not volume_id.strip():
            problems.append(f"G6：volumes[{index}] 缺 id")
            volume_id = f"<volumes[{index}]>"
        elif volume_id in seen_volumes:
            problems.append(f"G6：volume id 重复：{volume_id}")
        seen_volumes.add(volume_id)
        volume_title = volume.get("title") if isinstance(volume.get("title"), str) else ""

        chapters = volume.get("chapters", [])
        if not isinstance(chapters, list):
            problems.append(f"G6：volume {volume_id} 的 chapters 必须是数组")
            continue
        for position, chapter in enumerate(chapters):
            if not isinstance(chapter, dict):
                problems.append(f"G6：volume {volume_id} 的 chapters[{position}] 不是对象")
                continue
            chapter_id = chapter.get("id")
            if not isinstance(chapter_id, str) or not chapter_id.strip():
                problems.append(f"G6：volume {volume_id} 的 chapters[{position}] 缺 id")
                chapter_id = f"<{volume_id}.chapters[{position}]>"
            elif chapter_id in seen_chapters:
                problems.append(f"G6：chapter id 重复：{chapter_id}")
            seen_chapters.add(chapter_id)

            raw_units = chapter.get("units", [])
            if not isinstance(raw_units, list):
                problems.append(f"G6：chapter {chapter_id} 的 units 必须是数组")
                raw_units = []
            entry_units: list[dict] = []
            for raw in raw_units:
                if not isinstance(raw, dict):
                    problems.append(f"G6：chapter {chapter_id} 里有一个不是对象的单元条目")
                    continue
                file = raw.get("file")
                if not isinstance(file, str) or not file.strip():
                    problems.append(f"G6：chapter {chapter_id} 里有条目缺 file：{raw!r}")
                    continue
                if raw.get("unit") is None:
                    problems.append(f"G6：chapter {chapter_id} 的 {file} 缺 unit 号")
                if file in seen_files:
                    problems.append(
                        f"G6：单元 {file} 同时属于 {seen_files[file]} 与 {chapter_id}"
                        "（每个 unit 只能属于一个 chapter）"
                    )
                    continue
                seen_files[file] = chapter_id
                entry = dict(raw)
                entry["_chapter"] = chapter_id
                entry["_volume"] = volume_id
                entry_units.append(entry)
                units.append(entry)

            info.chapters.append(
                Chapter(
                    id=chapter_id,
                    title=chapter.get("title") if isinstance(chapter.get("title"), str) else "",
                    prereqs=_string_list(chapter.get("prereqs")),
                    tags=_string_list(chapter.get("tags")),
                    quota=_quota_of(chapter),
                    units=entry_units,
                    volume_id=volume_id,
                    volume_title=volume_title,
                )
            )

    # prereqs 只能在**全部** chapter id 收齐后判（允许前向引用；课程是长出来的）。
    for chapter in info.chapters:
        for prereq in chapter.prereqs:
            if prereq not in seen_chapters:
                problems.append(f"G6：chapter {chapter.id} 的 prereqs 指向不存在的 {prereq}")
    return units, problems, info


def quota_notes(value, source: Path, canvas_open: dict[str, int] | None = None) -> list[str]:
    """`quota.exercises` 与画布实际练习数的差额——**只报告**（设计 §2/§4.2）。

    `canvas_open` = `{单元 file: 该画布的实测练习数（exercise.open）}`；不给时
    只报计划数（`--selftest` 与清单自检用得上）。差额**永不**参与判红：
    课程还在长，锁死计数会让门禁从质量闸退化成记账本。
    """
    units, _, info = flatten_manifest(value, source)
    if not info.v2:
        return []
    by_file = {str(entry.get("file")): entry for entry in units}
    notes: list[str] = []
    for chapter in info.chapters:
        if chapter.quota is None:
            continue
        files = [str(entry.get("file")) for entry in chapter.units]
        planned = chapter.quota
        if canvas_open is None:
            notes.append(
                f"chapter {chapter.id} 计划练习 {planned} · 单元 {len(files)} 个（未判卷，无实测）"
            )
            continue
        measured = sum(canvas_open.get(file, 0) for file in files if file in by_file)
        delta = measured - planned
        sign = "+" if delta > 0 else ""
        notes.append(
            f"chapter {chapter.id} 计划练习 {planned} · 画布实测 {measured}（差额 {sign}{delta}）"
        )
    return notes



@dataclass
class Target:
    label: str
    path: Path
    kind: str  # lib | unit | solution | page
    canvas: Path | None = None
    unit: int | None = None


def solution_unit(path: Path) -> int | None:
    match = re.match(r"unit(\d+)", path.stem)
    return int(match.group(1)) if match else None


def page_solution(canvas: Path, course: Path = COURSE) -> Path:
    """非单元页面（`<画布名>.sokonanoda`）的解答路径；缺失由 G2 判负（不在这里报）。"""
    return course / "units" / "solutions" / f"{canvas.stem}-solution.sokonanoda"


def discover(course: Path = COURSE) -> tuple[list[Target], list[str], int, ManifestInfo, list[dict]]:
    """目标清单 = lib/*（含 Demo 复判一次）+ course.json 的单元 + 每个画布的解答。

    `units/` 下**不计入 `course.json` 的页面**（记法对照页 `notation-cheatsheet`，大纲 §4
    的"第二遍"）按 `units/*.sokonanoda` 里剩下的文件发现：它们不是单元（没有单元号、没有
    配额），但是画布——有 `sorry`、有解答，所以照样过 G1/G3/G4。**画布是发现入口**：
    解答缺了会被 G2 点名，而不是让整个页面悄悄从门禁里消失。

    清单两种格式都读（v1 数组 / v2 对象）：`flatten_manifest()` 展平成 v1 同形的
    单元列表，G1–G5 的判定因此一行未改；**G6 的结构非法**与 G2 走同一条
    `problems` 通道（同一个判负行、同一套退出码）。
    """
    # `course` 参数只给 `--selftest` 的临时夹具用（默认 = 本课程目录）：
    # 判据代码因此能在**故意坏的清单**上被验证，而不必先污染真清单。
    manifest_path = course / "course.json"
    lib_dir = course / "lib"
    units_dir = course / "units"
    solutions_dir = units_dir / "solutions"
    if not manifest_path.is_file():
        raise Prerequisite(f"找不到课程清单 {manifest_path}")
    try:
        raw = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise Prerequisite(f"course.json 读不出来或不是 JSON：{error}")
    units, problems, info = flatten_manifest(raw, manifest_path)
    if not units:
        raise Prerequisite("course.json 里没有任何单元")

    targets: list[Target] = []
    modules = sorted(lib_dir.glob("*.sokonanoda"))
    if not modules:
        problems.append(f"课程标准库是空的：{lib_dir}")
    for module in modules:
        targets.append(Target(f"lib {module.stem}", module, "lib"))
    targets.append(Target("lib 自检", lib_dir / "Demo.sokonanoda", "lib"))

    canvases: list[tuple[int, Path]] = []
    listed: set[Path] = set()
    for entry in units:
        rel, number = entry.get("file"), entry.get("unit")
        if not rel or number is None:
            problems.append(f"course.json 条目缺 file/unit：{entry!r}")
            continue
        canvas = course / str(rel)
        canvases.append((int(number), canvas))
        listed.add(canvas)
        targets.append(Target(f"单元 {number}", canvas, "unit", unit=int(number)))

    by_unit: dict[int, Path] = {}
    for solution in sorted(solutions_dir.glob("*-solution.sokonanoda")):
        number = solution_unit(solution)
        if number is None:
            continue  # 非单元页面的解答由**画布**那一侧发现（见下）
        if number in by_unit:
            problems.append(f"同一单元有两份解答：unit{number:02d} → {by_unit[number].name} / {solution.name}")
            continue
        by_unit[number] = solution

    for number, canvas in canvases:
        # 解答缺失由这一行自己报（status=missing），所以不进 `problems`（不重复计数）。
        solution = by_unit.get(number) or solutions_dir / f"unit{number:02d}-solution.sokonanoda"
        targets.append(Target(f"解答 unit{number:02d}", solution, "solution", canvas=canvas, unit=number))
    for number, solution in sorted(by_unit.items()):
        if all(number != seen for seen, _ in canvases):
            targets.append(Target(f"解答 unit{number:02d}（画布不在 course.json）", solution, "solution", unit=number))

    # 非单元页面：画布 + 解答成对进目标，判据与单元同一条 G1/G3/G4。
    for canvas in sorted(units_dir.glob("*.sokonanoda")):
        if canvas in listed:
            continue
        targets.append(Target(f"页面 {canvas.stem}", canvas, "page"))
        targets.append(Target(f"页面解答 {canvas.stem}", page_solution(canvas, course),
                             "solution", canvas=canvas))
    return targets, problems, len(canvases), info, units


# ── 判据 G1/G3/G4/G5 ────────────────────────────────────────────────────────


def evaluate(rows: list[dict], judge) -> None:
    """给每行挂上判负理由（G1/G3/G5 就地判定；G4 需要画布，按需补判）。"""
    for row in rows:
        if row["status"] != "ok":
            continue
        reasons = row["reasons"]
        if row["exit"] != 0:
            reasons.append("G1：`grade` 退出码非 0（解析/elaborate/内核拒绝）")
        if row["kind"] == "solution":
            if row["open"] > 0:
                reasons.append("G3：解答里还有未填的洞 " + "、".join(row["open_names"]))
            if row["checked"] == 0:
                reasons.append("G3：解答没有任何声明通过内核（`decl.checked == 0`）")
        if row["kind"] == "lib" and row["open"] > 0:
            reasons.append("G5：课程标准库里有 sorry " + "、".join(row["open_names"]))

    # G4 独立于 G1/G3：解答即使已经判负，也要说清"画布的哪些练习它没覆盖"。
    by_path = {row["path"]: row for row in rows if row.get("path")}
    for row in rows:
        if row["kind"] != "solution" or row["canvas"] is None or row["exit"] is None:
            continue
        canvas = by_path.get(row["canvas"])
        if canvas is None:
            # `--only 解答 …` 时画布不在选中集合里：按需补判一次（判卷结果有缓存）。
            result = judge(Path(row["canvas"]))
            canvas = {"exit": result.code, "open_names": result.open_names}
        if canvas["exit"] != 0:
            continue  # 画布自己判负 ⇒ G1 已报，覆盖度无从谈起
        covered = set(row["checked_names"])
        missing = [name for name in canvas["open_names"] if name not in covered]
        if missing:
            row["reasons"].append("G4：解答没有覆盖画布的练习 " + "、".join(missing))

    for row in rows:
        if row["status"] == "ok" and row["reasons"]:
            row["status"] = "rejected"


# ── --bisect（定位段：不依赖诊断 span，解析失败/多条错误时最稳）───────────────


def declaration_starts(text: str) -> list[tuple[int, str]]:
    """顶层声明边界：行首关键字扫描 ⇒ [(0-based 行号, 名字)]。"""
    found: list[tuple[int, str]] = []
    for index, line in enumerate(text.splitlines()):
        stripped = line.lstrip()
        if not stripped or stripped.startswith("--"):
            continue
        match = DECL_START.match(stripped)
        if not match:
            continue
        rest = stripped[match.end():].lstrip()
        name = match.group(0)
        token = re.split(r"[\s:({\[]", rest, maxsplit=1)[0] if rest else ""
        if token:
            name = f"{name} {token}"
        found.append((index, name))
    return found


def bisect(channel: Channel, target: Path) -> dict:
    """二分出**最后一个全绿前缀**与**第一个判红的声明**（结论不依赖诊断 span）。"""
    text = target.read_text(encoding="utf-8")
    lines = text.splitlines()
    starts = declaration_starts(text)
    if not starts:
        return {"file": str(target), "decls": 0, "green_decls": None, "green_lines": None,
                "first_bad": None, "first_bad_line": None, "note": "行首没扫到任何顶层声明"}
    results: dict[int, int] = {}

    def prefix_exit(count: int) -> int:
        if count not in results:
            if count >= len(starts):
                results[count] = grade(channel, target).code
            else:
                cut = starts[count][0]
                results[count] = grade_text(channel, target, "\n".join(lines[:cut]) + "\n").code
        return results[count]

    full = prefix_exit(len(starts))
    if full == 0:
        return {"file": str(target), "decls": len(starts), "green_decls": len(starts),
                "green_lines": len(lines), "first_bad": None, "first_bad_line": None,
                "note": "整份文件判绿，无需二分"}
    low, high = 0, len(starts)  # 不变量：前缀 low 个声明判绿、high 个判红（索引，不是课程规模）
    while low + 1 < high:
        middle = (low + high) // 2
        if prefix_exit(middle) == 0:
            low = middle
        else:
            high = middle
    name, line = starts[high - 1][1], starts[high - 1][0] + 1
    return {"file": str(target), "decls": len(starts), "green_decls": low,
            "green_lines": starts[low][0] if low < len(starts) else len(lines),
            "first_bad": name, "first_bad_line": line, "full_exit": full,
            "note": "依赖「前缀单调」假定；结论来自退出码，不含诊断 span"}


def render_bisect(target: Target, found: dict, check_py: Path) -> list[str]:
    lines = [f"--bisect {target.label}（{target.path}）"]
    if found.get("first_bad") is None:
        lines.append(f"  整份文件判绿：{found['decls']} 个顶层声明，无需二分。")
        return lines
    lines.append(f"  最后全绿的前缀 = 前 {found['green_decls']} 个顶层声明（到第 {found['green_lines']} 行）")
    lines.append(f"  第一个判红的声明 = {found['first_bad']}（第 {found['first_bad_line']} 行）")
    lines.append("  这个结论来自退出码而不是诊断 span；改完重跑："
                 f"python3 {check_py} --only \"{target.label}\" --bisect")
    return lines


# ── --selftest（G-10 的判据通道自检 + G-12 的 cwd 自检 + G6 的清单自检）────

BAD_UNIT = "theorem selftest_bad (P : Prop) (h : P) : P := by\n  exact bogus_name\n"
BISECT_UNIT = (
    "theorem selftest_ok1 (P : Prop) (h : P) : P := by\n"
    "  exact h\n"
    "\n"
    "theorem selftest_ok2 (P : Prop) : P -> P := by\n"
    "  intro h\n"
    "  exact h\n"
    "\n"
    "theorem selftest_bad3 (P : Prop) (h : P) : P := by\n"
    "  exact bogus_name\n"
)

SELFTEST_LIB = "def selftestId (P : Prop) : Prop := P\n"
SELFTEST_UNIT = "import lib.Logic\n\ndef u (P : Prop) : Prop := selftestId P\n"
SELFTEST_SOLUTION = SELFTEST_UNIT + "\ntheorem solved (P : Prop) : P -> P := fun h => h\n"

# G6 的三类**结构非法**：每一种都必须被判负（判据通道失效时自检自己红）。
SELFTEST_G6_BAD = {
    "重复的 chapter id": {
        "schema": MANIFEST_SCHEMA_V2,
        "volumes": [{"id": "I", "chapters": [
            {"id": "I.1", "units": [{"file": "units/a.sokonanoda", "unit": 1}]},
            {"id": "I.1", "units": []},
        ]}],
    },
    "prereqs 指向不存在的章": {
        "schema": MANIFEST_SCHEMA_V2,
        "volumes": [{"id": "I", "chapters": [
            {"id": "I.1", "prereqs": ["I.9"],
             "units": [{"file": "units/a.sokonanoda", "unit": 1}]},
        ]}],
    },
    "同一 unit 挂在两个章": {
        "schema": MANIFEST_SCHEMA_V2,
        "volumes": [{"id": "I", "chapters": [
            {"id": "I.1", "units": [{"file": "units/a.sokonanoda", "unit": 1}]},
            {"id": "I.2", "units": [{"file": "units/a.sokonanoda", "unit": 1}]},
        ]}],
    },
}

# G6 的**合法**清单（正控制）：结构面干净，配额差额只报告不判红。
SELFTEST_G6_GOOD = {
    "schema": MANIFEST_SCHEMA_V2,
    "volumes": [{"id": "I", "title": "卷", "chapters": [
        {"id": "I.1", "title": "章", "prereqs": [], "tags": ["t"],
         "quota": {"exercises": 99},
         "units": [{"file": "units/a.sokonanoda", "title": "A", "unit": 1}]},
    ]}],
}


def selftest_course_fixture(root: Path, manifest) -> Path:
    """造一个最小课程目录（lib + units + 解答），供 G6 自检用。"""
    (root / "lib").mkdir(parents=True, exist_ok=True)
    (root / "units" / "solutions").mkdir(parents=True, exist_ok=True)
    (root / "lib" / "Logic.sokonanoda").write_text(SELFTEST_LIB, encoding="utf-8")
    (root / "units" / "a.sokonanoda").write_text(SELFTEST_UNIT, encoding="utf-8")
    (root / "units" / "solutions" / "a-solution.sokonanoda").write_text(
        SELFTEST_SOLUTION, encoding="utf-8"
    )
    (root / "course.json").write_text(json.dumps(manifest, ensure_ascii=False), encoding="utf-8")
    return root


def selftest_g6(failures: list[str], tmp: Path) -> None:
    """G6 自检：三类结构非法必须判负，一份合法清单必须判绿（正控制）。"""
    for index, (label, manifest) in enumerate(SELFTEST_G6_BAD.items()):
        course = selftest_course_fixture(tmp / f"g6-bad-{index}", manifest)
        try:
            _, problems, _, _, _ = discover(course)
        except Prerequisite as error:
            failures.append(f"G6 自检失败（{label}）：清单被判成前置错误而不是结构判负：{error}")
            continue
        if not any(problem.startswith("G6") for problem in problems):
            failures.append(f"G6 自检失效：{label} 没有被判负（problems={problems}）")

    good = selftest_course_fixture(tmp / "g6-good", SELFTEST_G6_GOOD)
    try:
        _, problems, _, info, units = discover(good)
    except Prerequisite as error:
        failures.append(f"G6 自检失败（正控制）：合法 v2 清单被判成前置错误：{error}")
        return
    if problems:
        failures.append(f"G6 自检失效（正控制）：合法 v2 清单被判负（problems={problems}）")
    if info.volumes != 1 or len(info.chapters) != 1:
        failures.append(f"G6 自检失败（正控制）：卷/章计数错（volumes={info.volumes} chapters={len(info.chapters)}）")
    # 配额 99 vs 画布 0 道 sorry：**只报告**，绝不进 problems。
    notes = quota_notes(SELFTEST_G6_GOOD, good / "course.json", {"units/a.sokonanoda": 0})
    if not any("99" in note for note in notes):
        failures.append(f"G6 自检失效：配额差额没有被报告（notes={notes}）")
    if len(units) != 1:
        failures.append(f"G6 自检失败（正控制）：展平后应有 1 个单元，实得 {len(units)}")


def selftest(channel: Channel, check_py: Path) -> int:
    failures: list[str] = []
    tmp = Path(tempfile.mkdtemp(prefix="soko-course-selftest-"))

    # ① 正控制 + G-12：从**另一个 cwd** 判一个真的、带 `import lib.*` 的目标。
    control = LIB / "Demo.sokonanoda"
    result = grade(channel, control, cwd=tmp)
    if result.code != 0:
        failures.append(
            f"正控制失败：{control.name} 从 {tmp} 判卷 exit={result.code}"
            f"（判卷通道坏了，或绝对路径/模块根回归——G-12）"
        )
    elif result.checked == 0:
        failures.append(f"正控制失败：{control.name} 一个声明都没 checked")

    # ② 变异：故意坏的单元（合法前缀 + `exact bogus_name`）必须被判负。
    bad = tmp / "selftest-bad.sokonanoda"
    bad.write_text(BAD_UNIT, encoding="utf-8")
    result = grade(channel, bad, cwd=tmp)
    if result.code == 0:
        failures.append("判据通道失效（G-10 类）：故意坏的声明被判绿——门禁结论不可信")

    # ③ 二分：3 个声明、第 3 个坏 ⇒ 必须点名第 3 个（且不靠诊断 span）。
    triple = tmp / "selftest-bisect.sokonanoda"
    triple.write_text(BISECT_UNIT, encoding="utf-8")
    found = bisect(channel, triple)
    if found.get("first_bad") != "theorem selftest_bad3":
        failures.append(f"二分自检失败：期望点名 theorem selftest_bad3，实得 {found.get('first_bad')!r}")
    elif found.get("green_decls") != 2:
        failures.append(f"二分自检失败：期望最后全绿前缀 = 前 2 个声明，实得 {found.get('green_decls')!r}")

    # ④ G6 清单自检：三类结构非法必须判负 + 一份合法 v2 清单必须判绿。
    selftest_g6(failures, tmp)

    print(f"--selftest：判卷通道 [{channel.source}] {channel.path}"
          + (f"（v{channel.version}）" if channel.version else ""))
    if failures:
        for line in failures:
            print(f"  ✗ {line}")
        print("--selftest FAIL：判据通道不可信，门禁不判绿。")
        return 1
    print("  ✓ 正控制（另一个 cwd + import lib.*）判绿 · ✓ 故意坏的单元被判负 · "
          "✓ 二分点名坏声明 · ✓ G6 三类结构非法被判负 + 合法 v2 判绿")
    print("--selftest PASS")
    return 0


# ── 输出（人读 / JSON / 注解 / step summary / 台账）─────────────────────────


def gh_escape(text: str) -> str:
    return text.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def gh_property(text: str) -> str:
    return gh_escape(text).replace(":", "%3A").replace(",", "%2C")


def failure_block(row: dict, check_py: Path) -> list[str]:
    shown_exit = row["exit"] if row["exit"] is not None else "—"
    head = f"✗ {row['label']}  exit={shown_exit}  {row['file']}"
    if row["diagnostics"]:
        first = row["diagnostics"][0]
        head += f"  stage={first['stage']} code={first['code']} message={first['message']}"
    lines = [head]
    for reason in row["reasons"]:
        lines.append(f"    {reason}")
    for note in row["notes"]:
        lines.append(f"    {note}")
    if row["diagnostics"]:
        lines.append("    参考段：诊断原样如上（span 自带行列、可信）；要看「第一个判红的声明」仍用下面的二分。")
    if row["exit"]:
        lines.append(f"    定位：python3 {check_py} --only \"{row['label']}\" --bisect")
    else:
        lines.append("    `grade` 没判红（exit 0 或文件不存在）⇒ --bisect 不适用；按上面的理由定位。")
    return lines


def summary_markdown(rows: list[dict], summary: dict, channel: Channel) -> str:
    lines = ["### 课程门禁 · 卷 I《集合论》（G1–G6，与规模无关）", ""]
    lines.append(f"判卷通道：`{channel.path}` [{channel.source}]"
                 + (f" · v{channel.version}" if channel.version else ""))
    lines.append("")
    lines.append("| 状态 | checked | open | 目标 |")
    lines.append("|---|---:|---:|---|")
    for row in rows:
        mark = {"ok": "ok", "rejected": "**判负**", "missing": "**缺失**"}.get(row["status"], row["status"])
        lines.append(f"| {mark} | {row['checked']} | {row['open']} | `{row['file']}` |")
    lines.append("")
    lines.append(f"**{summary['targets']} 个目标 —— {summary['checked']} checked · "
                 f"{summary['open']} open · {summary['rejected']} 个被判负**")
    return "\n".join(lines) + "\n"


def git_commit(root: Path) -> str | None:
    try:
        proc = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"],
                              capture_output=True, text=True, timeout=15)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return proc.stdout.strip() if proc.returncode == 0 else None


def ledger_entry(rows: list[dict], summary: dict, root: Path) -> str:
    pin, _ = repo_pin(root)
    entry = {
        "schema": LEDGER_SCHEMA,
        "version": pin,
        "commit": git_commit(root),
        "date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "targets": summary["targets"],
        "checked": summary["checked"],
        "open": summary["open"],
        "rejected": summary["rejected"],
        "solutions_open": summary["solutions_open"],
        "rows": [{"label": row["label"], "status": row["status"],
                  "checked": row["checked"], "open": row["open"]} for row in rows],
    }
    return json.dumps(entry, ensure_ascii=False)


# ── main ────────────────────────────────────────────────────────────────────


def parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="check.py",
        description="课程门禁（卷 I 集合论）：判据 G1–G6，与课程规模无关。",
    )
    parser.add_argument("--json", action="store_true", help="机器可读报告（含计数）打到 stdout")
    parser.add_argument("--only", metavar="标签", action="append", default=[],
                        help="只判匹配这个标签的目标（可重复；子串匹配）")
    parser.add_argument("--bisect", action="store_true",
                        help="按顶层声明边界二分出第一个判红的声明（不依赖诊断 span）")
    parser.add_argument("--selftest", action="store_true",
                        help="判据通道自检：故意坏的单元必须被判负（另含 G-12 的 cwd 自检）")
    parser.add_argument("--annotations", action="store_true",
                        help="失败逐条打 GitHub 注解（::error file=…::，不带行号）")
    parser.add_argument("--report", metavar="路径", help="把 --json 报告写到文件（CI artifact）")
    parser.add_argument("--summary", metavar="路径", help="把 markdown 表追加到文件（$GITHUB_STEP_SUMMARY）")
    parser.add_argument("--ledger", nargs="?", const=LEDGER_DEFAULT, metavar="路径",
                        help=f"追加一条台账（默认 {LEDGER_DEFAULT}）")
    parser.add_argument("--bin", metavar="路径", dest="bin_path",
                        help="判卷二进制（默认取 $SOKONANODA_BIN，再退回 scripts/soko）")
    return parser.parse_args(argv)


def run(rows: list[dict], judge, channel: Channel, args: argparse.Namespace, check_py: Path,
        info: ManifestInfo | None = None, manifest: list[dict] | None = None) -> int:
    evaluate(rows, judge)

    summary = {
        "targets": len(rows),
        "checked": sum(row["checked"] for row in rows),
        "open": sum(row["open"] for row in rows),
        "rejected": sum(1 for row in rows if row["status"] != "ok"),
        "solutions_open": sum(row["open"] for row in rows if row["kind"] == "solution"),
        "lib_open": sum(row["open"] for row in rows if row["kind"] == "lib"),
        "canvas_open": sum(row["open"] for row in rows if row["kind"] == "unit"),
    }

    # ── 清单结构面（v2；台账 G-07）─────────────────────────────────────────
    # 每行挂上它所属的卷/章（**只加不删**：v1 时是 null，消费者照旧读 v1 的键），
    # 章的配额差额**只报告**（`quota_notes`；判红永远只看 G1–G6 的结构面）。
    if info is not None:
        by_file = {str(entry.get("file")): entry for entry in (manifest or [])}
        for row in rows:
            entry = by_file.get(row["file"])
            row["volume"] = entry.get("_volume") if entry else None
            row["chapter"] = entry.get("_chapter") if entry else None
    canvas_open = {row["file"]: row["open"] for row in rows if row["kind"] == "unit"}
    quota: list[str] = []
    if info is not None and info.v2:
        try:
            quota = quota_notes(json.loads(info.source.read_text(encoding="utf-8")),
                                info.source, canvas_open)
        except (OSError, json.JSONDecodeError, Prerequisite):
            quota = []  # 清单在 discover() 里已经判过了；这里只补报告，不再判负

    report = {
        "schema": SCHEMA,
        "units": sum(1 for row in rows if row["kind"] == "unit"),
        "course": {"root": str(COURSE), "units": sum(1 for row in rows if row["kind"] == "unit"),
                   "volumes": info.volumes if info else 0,
                   "chapters": len(info.chapters) if info else 0},
        "channel": {"path": channel.path, "source": channel.source, "version": channel.version},
        "summary": summary,
        "quota_notes": quota,
        "targets": rows,
        "failed": summary["rejected"],
    }

    annotations = [row for row in rows if row["status"] != "ok"]
    if args.annotations:
        stream = sys.stderr if args.json else sys.stdout
        for row in annotations:
            detail = "；".join(row["reasons"]) or (row["notes"][0] if row["notes"] else "判负")
            print(f"::error file={gh_property(row['file'])}::{gh_escape(row['label'])} 判负"
                  f"（exit {row['exit']}）{gh_escape(detail)}", file=stream)

    bisected: list[str] = []
    if args.bisect:
        for row in rows:
            if not row.get("path") or not Path(row["path"]).is_file() or not row["exit"]:
                continue  # 文件不在、或 `grade` 判绿（判负只来自 G3/G4/G5）⇒ 二分不适用
            if row["status"] == "ok" and not args.only:
                continue  # 全量跑时只二分判负的目标；--only 选了谁就二分谁
            found = bisect(channel, Path(row["path"]))
            row["bisect"] = found
            bisected.extend(render_bisect(Target(row["label"], Path(row["path"]), row["kind"]), found, check_py))
    if bisected:
        report["bisect"] = bisected

    # 落盘放在最后：artifact 里的报告与 stdout 的 `--json` 是同一份（含 bisect）。
    if args.report:
        try:
            Path(args.report).write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n",
                                         encoding="utf-8")
        except OSError as error:
            print(f"warning: 写不了报告（{args.report}）：{error}", file=sys.stderr)
    if args.summary:
        try:
            with open(args.summary, "a", encoding="utf-8") as handle:
                handle.write(summary_markdown(rows, summary, channel))
        except OSError as error:
            print(f"warning: 写不了 step summary（{args.summary}）：{error}", file=sys.stderr)
    if args.ledger is not None:
        path = Path(args.ledger)
        if not path.is_absolute():
            path = (repo_root() or COURSE) / path
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "a", encoding="utf-8") as handle:
            handle.write(ledger_entry(rows, summary, repo_root() or COURSE) + "\n")

    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 1 if summary["rejected"] else 0

    print(f"{'状态':<10}{'checked':>8}{'open':>6}  目标")
    # v2：按卷/章分段（同章的目标连续打印，段头只印一次）；v1 不印段头（逐字节照旧）。
    heading = None
    for row in rows:
        marker = (row.get("volume"), row.get("chapter"))
        if info is not None and info.v2 and row.get("chapter") and marker != heading:
            heading = marker
            title = next((c.title for c in info.chapters if c.id == row["chapter"]), "")
            print(f"\n── 卷 {row['volume']} · 章 {row['chapter']} {title} ──")
        print(f"{row['status']:<10}{row['checked']:>8}{row['open']:>6}  {row['file']}")
        if row["status"] != "ok":
            for line in failure_block(row, check_py):
                print(line)
    if quota:
        print("\n配额对照（G6 报告，**不判红**）：")
        for note in quota:
            print(f"  {note}")
    if bisected:
        print()
        for line in bisected:
            print(line)
    print(f"\n{summary['targets']} 个目标 —— {summary['checked']} checked · "
          f"{summary['open']} open · {summary['rejected']} 个被判负")
    return 1 if summary["rejected"] else 0


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    check_py = Path(__file__).resolve()
    root = repo_root()
    if root is None:
        print("error: 找不到语言仓根的 scripts/soko（本课程应建在 sokonanoda-lang 内）", file=sys.stderr)
        return 2
    try:
        channel = resolve_channel(root, args.bin_path)
        if args.selftest:
            return selftest(channel, check_py)
        targets, problems, unit_count, info, units = discover()
    except Prerequisite as error:
        print(f"error: 前置缺失，门禁不判绿（exit 2）：\n{error}", file=sys.stderr)
        return 2

    if args.only:
        wanted: list[Target] = []
        for needle in args.only:
            # 先精确：门禁自己打印的 `--only "单元 5"` 必须只选中那一个（子串会把 1/10/11/12 一网打尽）。
            exact = [target for target in targets if target.label == needle]
            wanted.extend(exact or [target for target in targets if needle in target.label])
        unique: list[Target] = []
        for target in wanted:
            if target not in unique:
                unique.append(target)
        if not unique:
            print(f"error: --only 没匹配到任何目标：{args.only}\n"
                  f"  可用标签：" + "、".join(target.label for target in targets), file=sys.stderr)
            return 2
        targets = unique

    cache: dict[Path, GradeResult] = {}

    def judge(path: Path) -> GradeResult:
        key = path.resolve()
        if key not in cache:
            cache[key] = grade(channel, key)
        return cache[key]

    rows: list[dict] = []
    for target in targets:
        if not target.path.is_file():
            rows.append({"label": target.label, "file": str(target.path.relative_to(COURSE)),
                         "kind": target.kind, "status": "missing", "exit": None, "checked": 0,
                         "open": 0, "checked_names": [], "open_names": [], "diagnostics": [],
                         "notes": [], "reasons": ["G2：目标文件不存在"], "path": str(target.path),
                         "canvas": str(target.canvas) if target.canvas else None})
            continue
        result = judge(target.path)
        rows.append({"label": target.label, "file": str(target.path.relative_to(COURSE)),
                     "kind": target.kind, "status": "ok", "exit": result.code,
                     "checked": result.checked, "open": result.open,
                     "checked_names": result.checked_names, "open_names": result.open_names,
                     "diagnostics": result.diagnostics, "notes": result.notes, "reasons": [],
                     "path": str(target.path),
                     "canvas": str(target.canvas) if target.canvas else None})

    if problems:
        # G6 的结构非法自带前缀（判据名要能对上），G2 的补齐——别叠成「G2：G6：…」。
        rows.append({"label": "课程结构（G2/G6）", "file": "course.json", "kind": "course",
                     "status": "missing", "exit": None, "checked": 0, "open": 0,
                     "checked_names": [], "open_names": [], "diagnostics": [],
                     "notes": problems,
                     "reasons": [line if re.match(r"^G\d：", line) else f"G2：{line}"
                                 for line in problems],
                     "path": None, "canvas": None})
    if unit_count == 0:
        print("error: course.json 里没有可判的单元", file=sys.stderr)
        return 2
    return run(rows, judge, channel, args, check_py, info, units)


if __name__ == "__main__":
    sys.exit(main())
