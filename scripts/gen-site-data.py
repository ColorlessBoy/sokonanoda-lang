#!/usr/bin/env python3
"""Generate site/data/site.json — the single machine-readable source of truth
for the static GitHub Pages site.

Design: docs/design/site.md §3. No third-party dependencies — python3 stdlib only.

The site must never hand-write three things: version, unit count, and progress.
This script pulls each from its real source so the numbers cannot drift:

  version      <- Cargo.toml   [workspace.package] version
  units        <- course/course.json  (each unit flags its English mirror if present)
  round        <- STATUS.md    latest "## 本轮进度（日期，第N轮：标题）" header
  round_date   <-   (same header)
  round_title  <-   (same header)
  examples     <- examples/*.sokonanoda  filenames
  set_theory   <- courses/set-theory/course.json + **measured** by the course's
                  own gate (courses/set-theory/tools/check.py --json)

Counts are never hand-written anywhere: the 卷 I block is measured by running the
course gate, and every unit carries the gate's own `status`/`checked`/`open`.
The manifest may be the flat v1 array or the structured v2 object
(`soko.course/2`, ledger G-07): v2 additionally yields a `volumes` tree
(volume → chapter → units, with `prereqs`/`tags`/planned `quota`) whose unit
entries are the very same measured rows, so the page can group without a second
source of truth.
Measurement never triggers a toolchain download (`SOKONANODA_OFFLINE=1`) — the
repo explicitly refuses "在 CI 里 setup 下载二进制" (docs/design/course-gate-in-ci.md
§8) — so when no pinned CLI is resolvable the counts of the *previous* run are
carried over, and with nothing to carry the fields are simply left out.

Every field is parsed; if a source is missing or unparseable the field is left
out rather than fabricated. Output is deterministic (sorted keys, stable order).
"""

import datetime
import glob
import json
import os
import re
import subprocess
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# 卷 I《集合论》：单元清单 + 课程门禁（判据的唯一真相，见
# docs/design/teaching-project.md §P5 与 docs/design/course-gate-in-ci.md §2.4）。
SET_THEORY_DIR = os.path.join("courses", "set-theory")
SET_THEORY_GATE = os.path.join(SET_THEORY_DIR, "tools", "check.py")
SITE_JSON = os.path.join("site", "data", "site.json")
# 门禁的墙上时间预算：整卷暖缓存十几秒，给冷启动留足余量。不按目标数写死——
# 目标数由课程自己长（曾经写成 "34 targets"，课程长到 36 个时它就成了假的）。
GATE_TIMEOUT_S = 300


def _read(rel_path):
    path = os.path.join(REPO_ROOT, rel_path)
    try:
        with open(path, encoding="utf-8") as fh:
            return fh.read()
    except FileNotFoundError:
        return ""


def get_version():
    """Return [workspace.package].version from Cargo.toml, or None."""
    text = _read("Cargo.toml")
    if not text:
        return None
    # Isolate the [workspace.package] section so a top-level `version` (if any)
    # does not shadow the workspace package version.
    m = re.search(r"\[workspace\.package\](.*?)(?:\n\[|\Z)", text, re.S)
    block = m.group(1) if m else text
    mm = re.search(r'^\s*version\s*=\s*"([^"]+)"', block, re.M)
    return mm.group(1) if mm else None


def _parse_units(text):
    """Parse a course manifest — **both shapes** (ledger G-07).

    v1: a flat JSON array of `{file,title,title_en,unit}` (the intro course,
    `scripts/new-course-repo.sh` skeletons). v2: `{schema, name, title,
    volumes[].chapters[].units[]}` (卷 I), where the unit entries keep the v1
    shape verbatim. Returns `(units, volumes)`; `volumes` is `[]` for v1, so
    the site can group when the manifest says how and stay flat when it does
    not.
    """
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return [], []

    units = []
    volumes = []

    def read_unit(entry):
        if not isinstance(entry, dict):
            return None
        return {
            "file": entry.get("file", ""),
            "title": entry.get("title", ""),
            "title_en": entry.get("title_en", ""),
            "unit": entry.get("unit"),
        }

    if isinstance(data, list):
        for entry in data:
            unit = read_unit(entry)
            if unit is not None:
                units.append(unit)
    elif isinstance(data, dict):
        raw_volumes = data.get("volumes")
        if not isinstance(raw_volumes, list):
            return [], []
        for raw_volume in raw_volumes:
            if not isinstance(raw_volume, dict):
                continue
            volume = {
                "id": raw_volume.get("id", ""),
                "title": raw_volume.get("title", ""),
                "chapters": [],
            }
            raw_chapters = raw_volume.get("chapters")
            for raw_chapter in raw_chapters if isinstance(raw_chapters, list) else []:
                if not isinstance(raw_chapter, dict):
                    continue
                chapter = {
                    "id": raw_chapter.get("id", ""),
                    "title": raw_chapter.get("title", ""),
                    "prereqs": [p for p in raw_chapter.get("prereqs", []) if isinstance(p, str)]
                    if isinstance(raw_chapter.get("prereqs"), list) else [],
                    "tags": [t for t in raw_chapter.get("tags", []) if isinstance(t, str)]
                    if isinstance(raw_chapter.get("tags"), list) else [],
                    "quota": (raw_chapter.get("quota") or {}).get("exercises")
                    if isinstance(raw_chapter.get("quota"), dict) else None,
                    "units": [],
                }
                raw_units = raw_chapter.get("units")
                for raw_unit in raw_units if isinstance(raw_units, list) else []:
                    unit = read_unit(raw_unit)
                    if unit is None:
                        continue
                    chapter["units"].append(unit)
                    units.append(unit)
                volume["chapters"].append(chapter)
            volumes.append(volume)
    else:
        return [], []

    # Stable ordering by unit number (entries without a numeric unit sort last).
    units.sort(
        key=lambda u: (u.get("unit") is None, u.get("unit"))
        if isinstance(u.get("unit"), int)
        else (True, 0)
    )
    return units, volumes


def get_units():
    """Return the ordered unit list from course/course.json.

    Each item carries file/title/title_en/unit, and an extra `en_file` key when
    an English mirror exists under course/en/.
    """
    units, _volumes = _parse_units(_read(os.path.join("course", "course.json")))
    for unit in units:
        en_path = os.path.join("course", "en", unit["file"])
        if unit["file"] and os.path.exists(os.path.join(REPO_ROOT, en_path)):
            unit["en_file"] = en_path
    return units


def _first_line(text):
    """First non-empty line, for turning a subprocess failure into one sentence."""
    for line in (text or "").splitlines():
        if line.strip():
            return line.strip()
    return ""


def measure_set_theory():
    """Grade 卷 I through the course's own gate.

    Returns `({"rows": {file: {status, checked, open}}, "totals": {...}}, "")` on
    success, or `(None, reason)` when the gate could not be run.

    Two deliberate rules (both from docs/design/course-gate-in-ci.md):

    * the judging logic is **not** reimplemented here — counts only ever come
      from `check.py --json` ("判据双实现必然漂移", §2.3);
    * `SOKONANODA_OFFLINE=1` + a `scripts/soko doctor` pre-flight, because the
      repo explicitly does not download a toolchain to run the course gate (§8).
      Generating a page must never turn into a several-MB download; when no
      pinned CLI resolves, we report "not measured" instead of guessing.
    """
    launcher = os.path.join(REPO_ROOT, "scripts", "soko")
    gate = os.path.join(REPO_ROOT, SET_THEORY_GATE)
    for path in (launcher, gate):
        if not os.path.isfile(path):
            return None, f"缺少 {os.path.relpath(path, REPO_ROOT)}"
    env = dict(os.environ, SOKONANODA_OFFLINE="1")

    def run(argv, timeout):
        try:
            return subprocess.run(
                argv, capture_output=True, text=True,
                timeout=timeout, cwd=REPO_ROOT, env=env,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            return exc

    probe = run([launcher, "doctor", "--json"], timeout=120)
    if isinstance(probe, Exception):
        return None, f"启动器跑不起来：{probe}"
    if probe.returncode != 0:
        return None, (
            f"判卷环境未就绪（scripts/soko doctor 退出码 {probe.returncode}）："
            f"{_first_line(probe.stderr) or _first_line(probe.stdout)}"
        )

    proc = run([sys.executable or "python3", gate, "--json"], timeout=GATE_TIMEOUT_S)
    if isinstance(proc, Exception):
        return None, f"课程门禁跑不起来：{proc}"
    try:
        report = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return None, (
            f"课程门禁输出不是 JSON（退出码 {proc.returncode}）："
            f"{_first_line(proc.stderr)}"
        )

    targets = [
        t for t in report.get("targets", [])
        if isinstance(t, dict) and t.get("file")
    ]
    if not targets:
        return None, "课程门禁报告里没有目标"

    rows = {
        t["file"]: {
            "status": t.get("status", "unknown"),
            "checked": int(t.get("checked", 0) or 0),
            "open": int(t.get("open", 0) or 0),
        }
        for t in targets
    }
    # 总数优先用门禁自己的 summary（schema v2 起随报告给出）；没有时按它的**目标
    # 列表**求和，不按 `rows`（后者按文件去重）：门禁把 `lib/Demo` 判两次
    # （glob 一次 + 显式自检一次），所以它的汇总行比按文件去重的 `rows` 多出
    # 那几个目标 —— 这里必须与读者跑同一条命令看到的数字一致。
    #
    # 注释里同样不写死计数：本文件存在的理由就是数字只从实测来，写死的示例
    # 数字会随课程长大而变成谎言。
    summary = report.get("summary")
    if isinstance(summary, dict):
        totals = {
            "targets": int(summary.get("targets", len(targets))),
            "checked": int(summary.get("checked", 0)),
            "open": int(summary.get("open", 0)),
            "failed": int(summary.get("rejected", report.get("failed", 0) or 0)),
        }
    else:
        totals = {
            "targets": len(targets),
            "checked": sum(int(t.get("checked", 0) or 0) for t in targets),
            "open": sum(int(t.get("open", 0) or 0) for t in targets),
            "failed": int(report.get("failed", 0) or 0),
        }
    # 全体判负且零声明通过 = 判卷环境不可信（版本不符/二进制坏），不是"课程全坏"。
    # 官网上宁可不报数，也不把一次坏环境渲染成"整卷判负"。
    if totals["failed"] >= len(targets) and totals["checked"] == 0:
        return None, "全部目标判负且零声明通过（判卷环境不可信）"
    return {"rows": rows, "totals": totals}, ""


def _previous_set_theory():
    """The `set_theory` block already in site/data/site.json (last run), if any."""
    try:
        data = json.loads(_read(SITE_JSON) or "null")
    except json.JSONDecodeError:
        return {}
    block = data.get("set_theory") if isinstance(data, dict) else None
    return block if isinstance(block, dict) else {}


def get_set_theory():
    """Build the `set_theory` block: units from course.json + measured counts."""
    units, volumes = _parse_units(_read(os.path.join(SET_THEORY_DIR, "course.json")))
    if not units:
        return None

    measured, reason = measure_set_theory()
    block = {"course": SET_THEORY_DIR, "gate": SET_THEORY_GATE}
    if measured is not None:
        rows, totals = measured["rows"], measured["totals"]
        block["counts_source"] = "gate"
        block["measured_at"] = datetime.date.today().isoformat()
        block["totals"] = totals
        print(
            "set_theory: 门禁实测 "
            f"{totals['targets']} 目标 · {totals['checked']} checked · "
            f"{totals['open']} open · {totals['failed']} 判负"
        )
    else:
        previous = _previous_set_theory()
        rows = {
            u["file"]: u for u in previous.get("units", [])
            if isinstance(u, dict) and u.get("file") and "open" in u
        }
        totals = None
        if rows:
            block["counts_source"] = "previous-run"
            for key in ("measured_at", "totals"):
                if key in previous:
                    block[key] = previous[key]
            print(
                f"set_theory: 未实测（{reason}）——沿用上次实测的计数"
                + (f"（{previous.get('measured_at')}）" if previous.get("measured_at") else "")
            )
        else:
            block["counts_source"] = "none"
            print(f"set_theory: 未实测（{reason}）——本次不带计数")

    out_units = []
    for unit in units:
        row = rows.get(unit["file"]) if rows else None
        if row is not None:
            unit["status"] = row.get("status", "unknown")
            unit["checked"] = int(row.get("checked", 0) or 0)
            unit["open"] = int(row.get("open", 0) or 0)
        out_units.append(unit)
    block["units"] = out_units

    # v2：卷/章分组（台账 G-07）。单元的计数仍是**门禁实测**填进去的那一份
    # （按 file 对齐 `block["units"]`），章的配额只是清单里的计划数。
    if volumes:
        measured_by_file = {unit["file"]: unit for unit in out_units}
        out_volumes = []
        for volume in volumes:
            out_chapters = []
            for chapter in volume["chapters"]:
                out_chapter = {
                    "id": chapter["id"],
                    "title": chapter["title"],
                    "prereqs": chapter["prereqs"],
                    "tags": chapter["tags"],
                    "quota": chapter["quota"],
                    "units": [measured_by_file.get(u["file"], u) for u in chapter["units"]],
                }
                out_chapters.append(out_chapter)
            out_volumes.append(
                {"id": volume["id"], "title": volume["title"], "chapters": out_chapters}
            )
        block["volumes"] = out_volumes
    return block


_CN_DIGITS = {
    "零": 0, "一": 1, "二": 2, "两": 2, "三": 3, "四": 4,
    "五": 5, "六": 6, "七": 7, "八": 8, "九": 9,
}


def _cn_to_int(token):
    """Convert a Chinese/Arabic numeral token to int (handles up to 万).

    STATUS.md writes rounds as Chinese numerals (e.g. 第三十九轮), so a plain
    \\d+ would never match. Arabic digits are also accepted as a fallback.
    """
    total = 0
    section = 0
    num = 0
    for ch in token:
        if ch in _CN_DIGITS:
            num = _CN_DIGITS[ch]
        elif ch == "十":
            section += (num or 1) * 10
            num = 0
        elif ch == "百":
            section += (num or 1) * 100
            num = 0
        elif ch == "千":
            section += (num or 1) * 1000
            num = 0
        elif ch == "万":
            total += (section + (num or 0)) * 10000
            section = 0
            num = 0
        elif ch.isdigit():
            num = num * 10 + int(ch)
    value = total + section + num
    return value or None


def get_round():
    """Parse the newest '## 本轮进度（日期，第N轮：标题）' header.

    STATUS.md orders rounds newest-first, so the header we want is the **first**
    `## 本轮进度` line in the file. 只认第一个头、解析不了就**报错退出**：
    以前是"全文搜索第一个能匹配的头"，于是标题里带全角括号（`（多余的 sorry）`）
    让最新轮失配时，网站会**静默退回上一轮**（2026-09-18 实测：round 停在 97）。
    标题是 `docs/design/site.md` §2 写明的机器可读块，坏了要立刻被看见。

    轮次号后允许一个限定语（`第一百轮（语言线）：…`）——并行线（语言线/课程线）
    各自记轮是既有事实，限定语原样保留进标题，不丢信息；除此之外的形状仍然报错。
    """
    text = _read("STATUS.md")
    if not text:
        return {}
    header = re.search(r"^##\s*本轮进度.*$", text, re.M)
    if not header:
        return {}
    m = re.fullmatch(
        r"##\s*本轮进度（(\d{4}-\d{2}-\d{2})，第([^轮]+)轮(（[^）]*）)?：(.*)）",
        header.group(0).strip(),
    )
    if not m:
        raise SystemExit(
            "STATUS.md 最新一轮的标题解析不了（标题是网站进度页的机器可读块）：\n"
            f"  {header.group(0).strip()}\n"
            "期望形状：## 本轮进度（YYYY-MM-DD，第N轮：标题）"
        )
    qualifier = (m.group(3) or "").strip()
    title = m.group(4).replace("`", "").strip()
    return {
        "round_date": m.group(1),
        "round": _cn_to_int(m.group(2)),
        "round_title": f"{qualifier}{title}" if qualifier else title,
    }


def get_examples():
    """List examples/*.sokonanoda filenames (sorted)."""
    pattern = os.path.join(REPO_ROOT, "examples", "*.sokonanoda")
    return sorted(os.path.basename(p) for p in glob.glob(pattern))


def main():
    data = {}

    version = get_version()
    if version:
        data["version"] = version

    units = get_units()
    if units:
        data["units"] = units

    set_theory = get_set_theory()
    if set_theory:
        data["set_theory"] = set_theory

    data.update(get_round())  # round / round_date / round_title (only if found)

    examples = get_examples()
    if examples:
        data["examples"] = examples

    # tests_total is intentionally omitted: TESTING.md only carries it inside
    # prose-style per-round totals that are not a single stable field. We do not
    # fabricate a number (docs/design/site.md §3).

    out_dir = os.path.join(REPO_ROOT, "site", "data")
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, "site.json")
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(data, fh, ensure_ascii=False, indent=2, sort_keys=True)
        fh.write("\n")

    print("wrote", os.path.relpath(out_path, REPO_ROOT))


if __name__ == "__main__":
    main()
