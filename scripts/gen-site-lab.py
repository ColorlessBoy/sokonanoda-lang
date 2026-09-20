#!/usr/bin/env python3
"""Generate the six "lab" data files for the static site (site-rebuild S3/S7/S18/S19):

  site/data/walkthrough.json   一题的逐行真实目标状态（`query state` 的真实输出）
  site/data/events.json        一次判卷的真实 `--json` 事件流（含真拒绝）
  site/data/diagnostics.json   诊断码字典（解析 C1-language.md §4.4 + 实跑内核复现）
  site/data/timeline.json      真实时间线（git tag + CHANGELOG + STATUS + REQUIREMENTS §9）
  site/data/gaps.json          缺口台账（docs/gaps/ledger.jsonl + 汇总计数）
  site/data/evidence.json      质量证据数字（每条都带产生它的确切命令）

设计：docs/design/site-rebuild/spec/D2-information-architecture.md §2；
规格与取证：docs/design/site-rebuild/spec/D3-lab-data.md（每个文件的模式、源、
命令、缺失行为都写在那里，本文件的注释只讲"为什么这么做"）。

纪律（逐条沿用 scripts/gen-site-data.py，见 D2 §2「生成纪律」）：

* **纯 python3 标准库**，零第三方依赖；
* **确定性**：键排序 + 顺序稳定 ⇒ 重跑逐字节一致。`generated_at` 因此**不是**墙上
  时间，而是 **HEAD 提交时间**（`git log -1 --format=%cI HEAD`）——数据描述的是哪个
  仓库状态由 `source_commit` + `version` 钉死，读者据此判断是否过期；用墙上时间会让
  每次重跑都产生无意义的 diff。`generated_at_from` 字段把这条约定写进数据本身。
* **零伪造**：源缺失或解析失败 ⇒ 该字段/该条目留空，stderr 打一句人话原因。
  诊断字典里触发不了的码写 `"repro": null` + 原因，绝不用"差不多的输出"顶替；
  每条复现都由脚本**自己核对**（真输出里出现了那个码才算数）。
* **绝不触发工具链下载**：所有 CLI 调用带 `SOKONANODA_OFFLINE=1`（D2 §2、gen-site-data.py）。
* 只有脚本自己坏了才非零退出；源缺失是**被报告的省略**，不是崩溃。

用法：
  python3 scripts/gen-site-lab.py            # 生成全部六个文件（约一分钟）
  python3 scripts/gen-site-lab.py --only walkthrough,events   # 只跑某几节（调试用）
  python3 scripts/gen-site-lab.py --only evidence --with-tests # 额外真跑 cargo 数测试

`--only` 只影响**写盘**：没被点名的文件保持原样，便于增量调试。
`--with-tests` 会跑 `cargo test --list`（本机约 5 分钟，需要 DEVELOPER_DIR 绕过
Xcode 许可）并把结果缓存进 `.cache/site-lab/test-counts.json`；默认运行读那份缓存。
"""

import datetime
import fnmatch
import json
import os
import re
import shutil
import subprocess
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LAUNCHER = os.path.join("scripts", "soko")
OUT_DIR = os.path.join("site", "data")
# 生成过程中的临时文件（事件流用的判卷文件、import 复现件）都落在这里：
# `.cache` 是 .gitignore 的，不污染工作树，也不在"允许修改"的清单之外。
SCRATCH = os.path.join(".cache", "site-lab")

# 走查页选中的画布：卷 I 单元①（选它的理由见 D3 §walkthrough）。
WALKTHROUGH_FILE = os.path.join(
    "courses", "set-theory", "units", "unit01-sets-membership.sokonanoda"
)

# ── 其余五节的源（全部是**已存在的仓库文件**；缺一个就少一节，不猜） ──────────
GAPS_LEDGER = os.path.join("docs", "gaps", "ledger.jsonl")
C1_LANGUAGE = os.path.join("docs", "design", "site-rebuild", "content", "C1-language.md")
C4_ROADMAP = os.path.join("docs", "design", "site-rebuild", "content", "C4-status-roadmap.md")
CHANGELOG = os.path.join("editor", "vscode", "CHANGELOG.md")
STATUS_MD = "STATUS.md"
STATUS_ARCHIVE = os.path.join("docs", "STATUS-ARCHIVE.md")
REQUIREMENTS = "REQUIREMENTS.md"
CI_FAILURES = os.path.join("docs", "CI-FAILURES.md")
PERF_LEDGER = os.path.join("docs", "perf", "ledger.jsonl")
E2E_LEDGER = os.path.join("docs", "e2e", "ledger.jsonl")
COURSES_LEDGER = os.path.join("docs", "courses", "ledger.jsonl")
PLAYGROUND = "playground.sokonanoda"
SET_THEORY_GATE = os.path.join("courses", "set-theory", "tools", "check.py")
VSCODE_PACKAGE = os.path.join("editor", "vscode", "package.json")

FILES = (
    "walkthrough",
    "events",
    "diagnostics",
    "timeline",
    "gaps",
    "evidence",
)


# ─────────────────────────────────────────────────────────────────────────────
# 共用小工具
# ─────────────────────────────────────────────────────────────────────────────


def _read(rel_path):
    """Repo-relative file text, or "" when missing (never raises)."""
    try:
        with open(os.path.join(REPO_ROOT, rel_path), encoding="utf-8") as fh:
            return fh.read()
    except (FileNotFoundError, IsADirectoryError, UnicodeDecodeError):
        return ""


def _first_line(text):
    for line in (text or "").splitlines():
        if line.strip():
            return line.strip()
    return ""


def _warn(reason):
    """Report an omission on stderr — the discipline's 'say why' channel."""
    print(f"  ! {reason}", file=sys.stderr)


def _git(args, timeout=60):
    """Run git in the repo root; returns stdout or None."""
    try:
        proc = subprocess.run(
            ["git"] + args,
            capture_output=True,
            text=True,
            cwd=REPO_ROOT,
            timeout=timeout,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if proc.returncode != 0:
        return None
    return proc.stdout


def get_version():
    """[workspace.package].version from Cargo.toml, or None (same rule as gen-site-data.py)."""
    text = _read("Cargo.toml")
    if not text:
        return None
    m = re.search(r"\[workspace\.package\](.*?)(?:\n\[|\Z)", text, re.S)
    block = m.group(1) if m else text
    mm = re.search(r'^\s*version\s*=\s*"([^"]+)"', block, re.M)
    return mm.group(1) if mm else None


def get_source_commit():
    out = _git(["rev-parse", "--short", "HEAD"])
    return out.strip() if out else None


def get_generated_at():
    """HEAD 提交时间（ISO-8601，带时区）——确定性时钟，见模块 docstring。"""
    out = _git(["log", "-1", "--format=%cI", "HEAD"])
    return out.strip() if out else None


def cli(args, timeout=180):
    """Run the harness-neutral launcher with SOKONANODA_OFFLINE=1.

    Returns `(returncode, stdout, stderr)`; `returncode is None` means the launcher
    could not be started at all (missing file, timeout, OSError).
    """
    path = os.path.join(REPO_ROOT, LAUNCHER)
    if not os.path.isfile(path):
        return None, "", f"缺少启动器 {LAUNCHER}"
    env = dict(os.environ, SOKONANODA_OFFLINE="1")
    try:
        proc = subprocess.run(
            [path] + list(args),
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=REPO_ROOT,
            env=env,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return None, "", str(exc)
    return proc.returncode, proc.stdout, proc.stderr


def cli_json(args, timeout=180):
    """Parse a one-object `query <op>` answer. Returns `(obj, reason)`.

    `ok:false` is a **real answer** (e.g. the caret is outside any declaration),
    not a failure — only unparseable stdout is a failure here. `query check` also
    exits 1 on a rejection while still printing a full JSON object.
    """
    rc, out, err = cli(args, timeout=timeout)
    if rc is None:
        return None, f"{LAUNCHER} {' '.join(args)} 跑不起来：{err}"
    try:
        return json.loads(out), ""
    except json.JSONDecodeError:
        return None, (
            f"{LAUNCHER} {' '.join(args)} 的输出不是 JSON（退出码 {rc}）："
            f"{_first_line(err) or _first_line(out)}"
        )


def cli_ready():
    """Pre-flight `scripts/soko doctor --json`, exactly like gen-site-data.py."""
    obj, reason = cli_json(["doctor", "--json"], timeout=120)
    if obj is None:
        return False, reason
    if not obj.get("ready"):
        return False, "判卷环境未就绪（scripts/soko doctor ready=false）"
    return True, ""


def ensure_scratch():
    path = os.path.join(REPO_ROOT, SCRATCH)
    os.makedirs(path, exist_ok=True)
    return path


def envelope(data):
    """Add the three staleness fields every generated file must carry."""
    if data.get("version") is None:
        version = get_version()
        if version:
            data["version"] = version
        else:
            _warn("Cargo.toml 里读不到 [workspace.package].version —— 本次不带 version")
    commit = get_source_commit()
    if commit:
        data["source_commit"] = commit
    else:
        _warn("git rev-parse 失败 —— 本次不带 source_commit")
    generated_at = get_generated_at()
    if generated_at:
        data["generated_at"] = generated_at
        data["generated_at_from"] = "git-head-commit-time"
    else:
        _warn("git log 读不到 HEAD 提交时间 —— 本次不带 generated_at")
    return data


def write_json(name, data):
    """Write site/data/<name>.json — sorted keys, stable order, trailing newline."""
    out_dir = os.path.join(REPO_ROOT, OUT_DIR)
    os.makedirs(out_dir, exist_ok=True)
    path = os.path.join(out_dir, f"{name}.json")
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(data, fh, ensure_ascii=False, indent=2, sort_keys=True)
        fh.write("\n")
    size = os.path.getsize(path)
    print(f"wrote {os.path.relpath(path, REPO_ROOT)} ({size} bytes)")
    return size


def byte_line_starts(text):
    """Byte offset of the start of every 1-based line (index 0 = line 1).

    Spans from the kernel are **byte** offsets (台账 G-15) while Python slices are
    character based, so every offset→line mapping in this script goes through the
    encoded byte string, never through `text[:off]`.
    """
    raw = text.encode("utf-8")
    starts = [0]
    for i, byte in enumerate(raw):
        if byte == 0x0A:
            starts.append(i + 1)
    return raw, starts


def line_of_offset(starts, offset):
    """1-based line number containing a byte offset (binary search)."""
    lo, hi = 0, len(starts) - 1
    while lo < hi:
        mid = (lo + hi + 1) // 2
        if starts[mid] <= offset:
            lo = mid
        else:
            hi = mid - 1
    return lo + 1


# ─────────────────────────────────────────────────────────────────────────────
# walkthrough.json —— 一题的逐行真实目标状态（B2 走查页）
# ─────────────────────────────────────────────────────────────────────────────


def build_walkthrough():
    """每一行都问一次内核：这一行的目标状态是什么。

    采样规则：**不采样**——`WALKTHROUGH_FILE` 的每一行（1..N）都真的调用一次
    `query state`。选中的画布只有 83 行，全量覆盖的墙上时间可以接受，而"每一行
    都是内核说的"正是这一页的卖点（D2 原则 2/3）。落在声明之外的行拿不到目标
    （`ok:false` + `outside-declarations`），那是**真实答案**，原样记录。

    记录的是内核的原始字段，不做二次渲染：`goal` / `goal_runs`（语法高亮片段）
    / `goals[]`（多目标时的完整列表）/ `decl`（名字、种类、状态）。目标里的
    `@Eq.{1}` 这类 pp 文本是冻结内核的输出，逐字保留。
    """
    text = _read(WALKTHROUGH_FILE)
    if not text:
        _warn(f"走查画布 {WALKTHROUGH_FILE} 不存在 —— 本次不生成 walkthrough.json")
        return None

    ready, reason = cli_ready()
    if not ready:
        _warn(f"{reason} —— 本次不生成 walkthrough.json（不编造逐行状态）")
        return None

    abs_path = os.path.join(REPO_ROOT, WALKTHROUGH_FILE)
    _, starts = byte_line_starts(text)
    lines = text.splitlines()

    goals_obj, reason = cli_json(["query", "goals", "--file", abs_path])
    if goals_obj is None or not goals_obj.get("ok"):
        _warn(f"query goals 没答上来（{reason or goals_obj.get('error')}）—— 本次不生成 walkthrough.json")
        return None
    raw_goals = goals_obj.get("data") or []

    decls = []
    for entry in raw_goals:
        if not isinstance(entry, dict):
            continue
        decl = {
            "name": entry.get("name"),
            "kind": entry.get("kind"),
            "status": entry.get("status"),
            "start_line": line_of_offset(starts, entry["start"]) if "start" in entry else None,
            "end_line": line_of_offset(starts, entry["end"]) if "end" in entry else None,
            "ty": entry.get("ty"),
        }
        if entry.get("ty_runs"):
            decl["ty_runs"] = entry["ty_runs"]
        decls.append(decl)

    # 行 → 声明：用内核给声明的**字节** span 反查（G-15：offset 一律按字节解释）。
    spans = [
        (e["start"], e["end"], e.get("name"))
        for e in raw_goals
        if isinstance(e, dict) and "start" in e and "end" in e
    ]

    def decl_at(offset):
        for start, end, name in spans:
            if start <= offset < end:
                return name
        return None

    holes = []
    holes_obj, reason = cli_json(["query", "holes", "--file", abs_path])
    if holes_obj is None or not holes_obj.get("ok"):
        _warn(f"query holes 没答上来（{reason}）—— walkthrough 不带 hole 位置")
    else:
        for hole in (holes_obj.get("data") or {}).get("holes") or []:
            if not isinstance(hole, dict):
                continue
            item = {
                "id": hole.get("id"),
                "decl": hole.get("decl"),
                "redundant": hole.get("redundant"),
                "ty": hole.get("ty"),
            }
            if "start" in hole:
                item["start_line"] = line_of_offset(starts, hole["start"])
                item["start_col"] = hole["start"] - starts[item["start_line"] - 1] + 1
            holes.append(item)

    # `-- soko:hint` 梯子：每个声明问一次（游标放声明首行第 1 列，与走查页
    # 折叠块的锚点一致）。没有梯子的声明内核回空列表 —— 原样记录空列表。
    hints = {}
    for decl in decls:
        line = decl.get("start_line")
        if not line:
            continue
        obj, _reason = cli_json(
            ["query", "hints", "--file", abs_path, "--line", str(line), "--col", "1"]
        )
        ladder = ((obj or {}).get("data") or {}).get("hints") or []
        if ladder:
            hints[decl["name"]] = ladder

    line_states = []
    for number in range(1, len(lines) + 1):
        offset = starts[number - 1]
        # 游标放在**行尾**，不是第 1 列。
        #
        # `query state` 的语义是「光标处剩余的目标」，所以第 1 列拿到的是
        # **进入这一行之前**的状态——那会让 `by` 块里每一行都报同一声明的根
        # 状态，"写一行看一行"的推进感整个消失（第一版就是这个毛病，走查页
        # 作者如实报告了它，没有伪造状态去补）。
        #
        # 实测（playground.sokonanoda:278-282 的 `by` 块）：
        #   col 1  → step -1, 0, 1, 1, 2   （进入该行）
        #   行尾列 → step  0, 1, 2, 3, ?   （该行做完之后）
        # 行尾才是这一行**做成了什么**，也就是走查页要讲的事。
        #
        # 行尾列 = 行长 + 1；文件最后一行会越界，回退到行内最后一个字符。
        text = lines[number - 1]
        end_col = len(text.rstrip()) + 1
        obj, reason = cli_json(
            ["query", "state", "--file", abs_path, "--line", str(number), "--col", str(end_col)]
        )
        if obj is None and end_col > 1:
            # 越界（典型是文件末行）：退到行内最后一个字符重试一次。
            obj, reason = cli_json(
                ["query", "state", "--file", abs_path, "--line", str(number), "--col", str(end_col - 1)]
            )
        entry = {"line": number, "decl": decl_at(offset), "col": end_col}
        if obj is None:
            entry["ok"] = False
            entry["error"] = {"code": "query-failed", "message": reason}
            _warn(f"第 {number} 行的 query state 没答上来：{reason}")
        elif obj.get("ok"):
            data = obj.get("data") or {}
            entry["ok"] = True
            for key in ("goal", "goal_runs", "goals", "step", "total"):
                if key in data:
                    entry[key] = data[key]
            decl = data.get("decl")
            if isinstance(decl, dict):
                entry["decl_status"] = decl.get("status")
        else:
            entry["ok"] = False
            entry["error"] = obj.get("error")
        line_states.append(entry)

    data = {
        "file": WALKTHROUGH_FILE,
        "source": text,
        "line_count": len(lines),
        "sampling": "every-line",
        "decls": decls,
        "holes": holes,
        "hints": hints,
        "lines": line_states,
    }
    inside = sum(1 for e in line_states if e.get("ok"))
    print(
        f"  walkthrough: {WALKTHROUGH_FILE} · {len(lines)} 行全部问了内核 · "
        f"{inside} 行有目标状态 · {len(decls)} 个声明 · {len(holes)} 个洞 · "
        f"{len(hints)} 个声明带 hint 梯子"
    )
    return data


# ─────────────────────────────────────────────────────────────────────────────
# gaps.json —— 缺口台账（真实条目 + 汇总计数）
# ─────────────────────────────────────────────────────────────────────────────


def read_jsonl(rel_path):
    """Parse a JSONL ledger. Returns `(rows, reason)`; `reason` != "" means the
    whole file is unusable (missing / not one JSON object per line).

    Blank lines are skipped (both ledgers end with a newline); a malformed line
    is **not** silently dropped — a ledger that does not parse is a reported
    omission, because a half-parsed ledger would understate the counts.
    """
    text = _read(rel_path)
    if not text:
        return [], f"{rel_path} 不存在或读不出来"
    rows = []
    for number, line in enumerate(text.splitlines(), start=1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as exc:
            return [], f"{rel_path}:{number} 不是合法 JSON（{exc.msg}）"
        if not isinstance(row, dict):
            return [], f"{rel_path}:{number} 不是 JSON 对象"
        rows.append(row)
    if not rows:
        return [], f"{rel_path} 里没有条目"
    return rows, ""


def _exists(rel_path):
    """True when a repo-relative path names an existing file."""
    if not rel_path:
        return False
    return os.path.isfile(os.path.join(REPO_ROOT, rel_path))


def build_gaps():
    """每条缺口：id / 标题 / 种类 / 级别 / 状态 / 发现日期 + 复现与工作单是否存在。

    `repro_exists` / `wo_exists` 是**磁盘上真的有没有那个文件**，不是台账字段
    非空——台账字段可以指向一个已经删掉的文件，页面据此渲染链接就会 404。
    两个字段都记：`repro`（台账原值，可能是 null）与 `repro_exists`（实测）。
    """
    rows, reason = read_jsonl(GAPS_LEDGER)
    if not rows:
        _warn(f"{reason} —— 本次不生成 gaps.json（不编造缺口台账）")
        return None

    entries = []
    for row in rows:
        repro = row.get("repro")
        wo = row.get("wo")
        entries.append(
            {
                "id": row.get("id"),
                "title": row.get("title"),
                "kind": row.get("kind"),
                "severity": row.get("severity"),
                "status": row.get("status"),
                "found": row.get("found"),
                "found_by": row.get("found_by"),
                "fixed_in": row.get("fixed_in"),
                "repro": repro,
                "repro_exists": _exists(repro) if isinstance(repro, str) else False,
                "wo": wo,
                "wo_exists": _exists(wo) if isinstance(wo, str) else False,
                "owner": row.get("owner"),
            }
        )
    # 稳定顺序：id 字典序（台账本身按主题分组，顺序会随编辑漂移）。
    entries.sort(key=lambda e: (e["id"] or ""))

    def tally(key):
        counts = {}
        for entry in entries:
            value = entry.get(key)
            counts[value if isinstance(value, str) else "(none)"] = (
                counts.get(value if isinstance(value, str) else "(none)", 0) + 1
            )
        return dict(sorted(counts.items()))

    summary = {
        "entries": len(entries),
        "by_status": tally("status"),
        "by_severity": tally("severity"),
        "by_kind": tally("kind"),
        "by_fixed_in": tally("fixed_in"),
        "with_repro": sum(1 for e in entries if e["repro"]),
        "with_repro_on_disk": sum(1 for e in entries if e["repro_exists"]),
        "with_work_order": sum(1 for e in entries if e["wo"]),
        "with_work_order_on_disk": sum(1 for e in entries if e["wo_exists"]),
        "open": sum(1 for e in entries if e["status"] not in ("fixed", "workaround")),
    }
    print(
        f"  gaps: {GAPS_LEDGER} · {summary['entries']} 条 · "
        + " · ".join(f"{k}={v}" for k, v in summary["by_status"].items())
        + f" · repro 在盘 {summary['with_repro_on_disk']}/{summary['with_repro']}"
        + f" · WO 在盘 {summary['with_work_order_on_disk']}/{summary['with_work_order']}"
    )
    return {"source": GAPS_LEDGER, "summary": summary, "entries": entries}


# ─────────────────────────────────────────────────────────────────────────────
# events.json —— 一次真判卷的完整 `--json` 事件流（含内核阶段真拒绝）
# ─────────────────────────────────────────────────────────────────────────────

# 判卷样例画布：**故意小**，但把事件词汇表里每一种事件都真的产生一次。
# 最后一条是**内核阶段**的真拒绝（`kernel-expected-pi`）：签名 elaborate 得过、
# 类型也成立，是内核在判定证明项时说不——这正是"判定永远走 kernel"的活证据。
EVENTS_CANVAS = """-- 判卷样例：通过的声明 + 开着的练习 + 一条内核阶段的真拒绝。
def id (a : Prop) : Prop := a
def compose (a b c : Prop) (g : b -> c) (f : a -> b) : a -> c :=
  fun (x : a) => g (f x)
theorem and_comm (a b : Prop) (h : And a b) : And b a :=
  And.intro b a (And.right a b h) (And.left a b h)
example : True := sorry
theorem open_exercise (a : Prop) : a -> a := sorry
#check id
#reduce 1 + 2
#print id
theorem kernel_rejected : True := True.intro True.intro
"""


def _decl_lines(text):
    """name -> (start_line, end_line) for every declaration, via `query goals`.

    `decl.checked` / `example.checked` / `exercise.open` 事件**不带 span**，
    所以"这条事件指源码哪一行"只能从声明的字节 span 反查（G-15：字节 offset）。
    """
    abs_path = os.path.join(REPO_ROOT, SCRATCH, "events.sokonanoda")
    obj, reason = cli_json(["query", "goals", "--file", abs_path])
    if obj is None or not obj.get("ok"):
        _warn(f"query goals 没答上来（{reason or (obj or {}).get('error')}）—— 事件流不带声明行号")
        return {}, {}
    _, starts = byte_line_starts(text)
    by_name = {}
    raw = []
    for entry in obj.get("data") or []:
        if not isinstance(entry, dict) or "start" not in entry or "end" not in entry:
            continue
        start_line = line_of_offset(starts, entry["start"])
        end_line = line_of_offset(starts, entry["end"])
        by_name[entry.get("name")] = (start_line, end_line)
        raw.append(
            {
                "name": entry.get("name"),
                "kind": entry.get("kind"),
                "status": entry.get("status"),
                "start_line": start_line,
                "end_line": end_line,
                "ty": entry.get("ty"),
            }
        )
    raw.sort(key=lambda d: (d["start_line"], d["name"] or ""))
    return by_name, {"decls": raw}

def build_events():
    """跑一次 `grade --json`，把事件流原样收下并给每条事件标出源码行。

    **不筛事件**：`--json` 吐多少条就记多少条（含 `human` 字段），页面自己决定
    渲染哪些。每条事件额外挂 `line` / `line_end` / `decl` 三个**对齐字段**
    （原始事件对象原样放在 `event` 下，一个键都不改）。
    """
    ready, reason = cli_ready()
    if not ready:
        _warn(f"{reason} —— 本次不生成 events.json（不编造事件流）")
        return None

    scratch = ensure_scratch()
    canvas = os.path.join(scratch, "events.sokonanoda")
    with open(canvas, "w", encoding="utf-8") as fh:
        fh.write(EVENTS_CANVAS)

    rel_canvas = os.path.relpath(canvas, REPO_ROOT)
    rc, out, err = cli(["grade", canvas, "--json"])
    if rc is None:
        _warn(f"grade 跑不起来（{err}）—— 本次不生成 events.json")
        return None

    events = []
    bad_lines = 0
    for line in out.splitlines():
        if not line.strip():
            continue
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            bad_lines += 1
    if not events:
        _warn(f"grade --json 没有吐出任何事件（退出码 {rc}）—— 本次不生成 events.json")
        return None
    if bad_lines:
        _warn(f"grade --json 有 {bad_lines} 行不是合法 JSON —— 这些行被丢弃")

    by_name, decl_info = _decl_lines(EVENTS_CANVAS)
    _, starts = byte_line_starts(EVENTS_CANVAS)

    aligned = []
    # 无名事件（`example` 的 `exercise.open` 不带 `name`）按**源码顺序**对齐到
    # 内核自己报的 open 声明上：`query goals` 把 `example : True := sorry`
    # 叫 `example@7`（名字里带行号），顺序就是文件顺序。这是从内核的声明表推
    # 出来的，不是文本比对。
    unclaimed_open = [
        d["name"] for d in (decl_info or {}).get("decls", [])
        if d.get("status") == "open" and d.get("name")
    ]
    for index, event in enumerate(events):
        span = event.get("span")
        line = line_end = None
        if isinstance(span, dict):
            start = (span.get("start") or {}).get("offset")
            end = (span.get("end") or {}).get("offset")
            if isinstance(start, int):
                line = line_of_offset(starts, start)
            if isinstance(end, int):
                line_end = line_of_offset(starts, max(end - 1, 0))
        decl = None
        name = event.get("name")
        if name in by_name:
            decl = name
            if line is None:
                line, line_end = by_name[name]
            if name in unclaimed_open:
                unclaimed_open.remove(name)
        elif event.get("type") == "exercise.open" and unclaimed_open:
            decl = unclaimed_open.pop(0)
            if line is None:
                line, line_end = by_name.get(decl, (None, None))
        item = {"index": index, "type": event.get("type"), "event": event}
        if line is not None:
            item["line"] = line
            item["line_end"] = line_end if line_end is not None else line
        if decl is not None:
            item["decl"] = decl
        aligned.append(item)

    counts = {}
    for event in events:
        key = event.get("type")
        if isinstance(key, str):
            counts[key] = counts.get(key, 0) + 1

    stage_counts = {}
    for event in events:
        if event.get("type") in ("diagnostic", "warning"):
            key = event.get("stage") or event.get("type")
            stage_counts[key] = stage_counts.get(key, 0) + 1

    data = {
        "canvas": rel_canvas,
        "source": EVENTS_CANVAS,
        "command": f"{LAUNCHER} grade {rel_canvas} --json",
        "exit_code": rc,
        "counts": dict(sorted(counts.items())),
        "stage_counts": dict(sorted(stage_counts.items())),
        "decls": (decl_info or {}).get("decls", []),
        "events": aligned,
    }
    kernel_stage = [
        e for e in events
        if e.get("type") == "diagnostic" and e.get("stage") == "kernel"
    ]
    print(
        f"  events: {rel_canvas} · {len(events)} 条事件（退出码 {rc}）· "
        + " · ".join(f"{k}={v}" for k, v in sorted(counts.items()))
        + f" · 内核阶段拒绝 {len(kernel_stage)} 条"
    )
    if not kernel_stage:
        _warn("这次判卷没有产生内核阶段拒绝 —— events.json 的 stage_counts 如实记录")
    return data


# ─────────────────────────────────────────────────────────────────────────────
# diagnostics.json —— 诊断码字典（C1 §4.4 权威表 + 逐个真复现）
# ─────────────────────────────────────────────────────────────────────────────

# C1 §4.4 是**已核对的权威摘要**（60 错误码 + 4 警告，按阶段分组，hint 逐字），
# 本脚本只**解析**它，不重新推导——重新推导就会和它漂移（D3 §diagnostics）。
C1_TABLE_HEADING = "### 4.4"
C1_TABLE_END = "### 4.5"

_ROW_RE = re.compile(r"^\|\s*`([a-z0-9-]+)`\s*\|(.*?)\|(.*?)\|\s*$")
_HEAD_RE = re.compile(
    r"^####\s+(?:`([a-z]+)`\s*阶段|(警告))（(\d+)\s*个[，,]\s*(.*?)）\s*$"
)


def parse_c1_diagnostics():
    """C1 §4.4 的四张阶段表 + 警告表 → 64 条 {code, stage, meaning, hint}。

    分组头形状：``#### `parse` 阶段（12 个，`crates/front/src/diagnostic.rs:78-90`）``
    与 ``#### 警告（4 个，`crates/front/src/compile/warning.rs:31-34`）``。
    hint 单元格两种写法：逐字文案用「」包起来；`import-not-a-valid-module-name`
    那条写的是「**hint 随诊断携带**（…）」，照抄成 note 并标 `hint_kind`。
    """
    text = _read(C1_LANGUAGE)
    if not text:
        return None, f"{C1_LANGUAGE} 不存在或读不出来"
    lines = text.splitlines()
    start = end = None
    for i, line in enumerate(lines):
        if line.startswith(C1_TABLE_HEADING) and start is None:
            start = i
        elif start is not None and line.startswith(C1_TABLE_END):
            end = i
            break
    if start is None or end is None:
        return None, f"{C1_LANGUAGE} 里找不到 §4.4 的表格区间"

    codes = []
    stage = None
    declared = None
    source_ref = None
    for line in lines[start:end]:
        if line.startswith("####"):
            head = _HEAD_RE.match(line.strip())
            if not head:
                stage = None
                _warn(f"C1 §4.4 的分组头解析不了，跳过：{line.strip()[:60]}")
                continue
            stage = head.group(1) or "warning"
            declared = int(head.group(3))
            source_ref = head.group(4).strip().strip("`")
            continue
        if stage is None or not line.startswith("|"):
            continue
        row = _ROW_RE.match(line)
        if not row:
            continue
        code, meaning, hint = row.group(1), row.group(2).strip(), row.group(3).strip()
        hint_kind = "verbatim"
        if hint.startswith("「") and hint.endswith("」"):
            hint = hint[1:-1]
        else:
            hint_kind = "note"
        codes.append(
            {
                "code": code,
                "stage": stage,
                "meaning": meaning,
                "hint": hint,
                "hint_kind": hint_kind,
                "source_ref": source_ref,
            }
        )
    if not codes:
        return None, f"{C1_LANGUAGE} §4.4 里一条码都没解析出来"

    # 文档自己写了每组几个；实际行数对不上就**报出来**（文档漂移要被看见）。
    seen = {}
    for entry in codes:
        seen[entry["stage"]] = seen.get(entry["stage"], 0) + 1
    if declared is not None and seen.get(stage) != declared:
        _warn(f"C1 §4.4 最后一组声明 {declared} 个，实际解析出 {seen.get(stage)} 个")
    return codes, ""


# 复现表：**每一条都在写进脚本前跑过真 CLI 验过**（见 D3 §diagnostics 的取证）。
# 形状：`S(源码)` = 单文件；`M({文件: 源码}, entry=…)` = 多文件项目；
# `manifest=` 覆盖 sokonanoda.toml（`manifest-invalid` 用）；`mode="grade"` 表示
# 该码在 `query check` 的**入口视图**里看不到（依赖模块的病只以
# `import-dependency-failed` 进 `failed[]`），必须走 `grade --json` 的事件流。
def S(src):
    return {"src": src}


def M(files, entry, manifest=None, mode="check"):
    spec = {"files": files, "entry": entry, "mode": mode}
    if manifest is not None:
        spec["manifest"] = manifest
    return spec


_DEP_ENTRY = "import b\ndef a : Prop := b\n"

REPRO_TABLE = {
    # ── parse（12） ──────────────────────────────────────────────────────────
    "unexpected-token": S("def x : =\n"),
    "import-malformed": S("import\n"),
    "import-not-a-valid-module-name": S("import Foo-Bar\n"),
    "import-must-precede-declarations": S("def x : Prop := True\nimport Foo.Bar\n"),
    "unterminated-string": S('infix:50 " mem => Set.mem\n'),
    "notation-shape": S('infix " mem " => Set.mem\n'),
    "notation-unknown-symbol": S("def f (a b : Nat) : Prop := a \u2208 b\n"),
    "parse-namespace-mismatch": S("namespace Foo\ndef x : Prop := True\nend Bar\n"),
    "parse-namespace-unclosed": S("namespace Foo\ndef x : Prop := True\n"),
    "parse-namespace-shape": S("namespace\ndef x : Prop := True\n"),
    "set-literal-shape": S("def s : Nat := {1, 2, 3}\n"),
    # ── elab（29） ───────────────────────────────────────────────────────────
    "elab-unknown-identifier": S("def x : Nat := nosuchname\n"),
    "elab-unknown-constant": S("#check Foo.{1}\n"),
    "elab-unknown-universe-level": S(
        "def id : Sort u -> Sort u := fun (a : Sort u) => a\n"
    ),
    "elab-universe-arity": S("#check Eq.{u, v}\n"),
    "elab-untyped-binder": S("#check (fun x => x)\n"),
    "elab-hole-misplaced": S("def x : sorry := True.intro\n"),
    "elab-duplicate-declaration": S("def x : Prop := True\ndef x : Prop := True\n"),
    "elab-unknown-ctor-for-iota": S(
        "inductive C : Type\nctor a : C\nctor b : C\n"
        "rec C.rec {u} : (motive : (c : C) -> Sort u) -> (ma : motive a) -> "
        "(mb : motive b) -> (c : C) -> motive c\n"
        "iota nosuch :=\n  fun (motive : (c : C) -> Sort u) =>\n"
        "  fun (ma : motive a) =>\n  fun (mb : motive b) => ma\nend\n"
    ),
    "elab-ambiguous-ctor-alias": S(
        "inductive P1 : Type\nctor mk : P1\nend\n"
        "inductive P2 : Type\nctor mk : P2\nend\ndef x : P1 := mk\n"
    ),
    "elab-tactic-failed": S("theorem t : False := by exact True.intro\n"),
    "elab-match-bad-arm": S(
        "def f (b : Bool) : Bool := match b with | true true => false | false => true\n"
    ),
    "elab-match-not-inductive": S("def f (a : Prop) : Prop := match a with | a => a\n"),
    "elab-match-no-expected-type": S(
        "#check (match Nat.zero with | Nat.zero => Nat.zero | Nat.succ k => k)\n"
    ),
    "elab-match-non-exhaustive": S(
        "def f (b : Bool) : Bool := match b with | true => false\n"
    ),
    "elab-match-parameterized-unsupported": S(
        "inductive Box (A : Type) : Type\nctor mk (a : A) : Box A\nend\n"
        "def f (A : Type) (a : A) : A := match Box.mk A a with | mk x => x\n"
    ),
    "elab-let-type-query-failed": S("def f : Nat := let x := nosuch; x\n"),
    "elab-notation-unknown-target": S(
        'infix:50 " \u2295 " => Set.mem\ndef bar (a b : Nat) : Prop := a \u2295 b\n'
    ),
    "elab-notation-argument-unsolved": S(
        'def foo (A : Type) (a b : Nat) : Prop := True\ninfix:50 " \u2295 " => foo\n'
        "def bar (a b : Nat) : Prop := a \u2295 b\n"
    ),
    "elab-notation-ambiguous": S(
        'def foo (a b : Nat) : Nat := a\ndef qux (a b : Nat) : Nat := b\n'
        'infix:50 " \u2295 " => foo\ninfix:50 " \u2295 " => qux\n'
        "def bar (a b : Nat) : Nat := a \u2295 b\n"
    ),
    "elab-notation-no-candidate": S(
        'def foo (a b : Nat) : Prop := True\ndef qux (a b : Nat) : Prop := True\n'
        'infix:50 " \u2295 " => foo\ninfix:50 " \u2295 " => qux\n'
        "def bar (a b : Nat) : Prop := a \u2295 b\n"
    ),
    "elab-binder-notation-unsolved": S(
        'axiom Exists : (A : Type) -> (A -> Prop) -> Prop\n'
        'binder_notation "\u2203" => Exists\ntheorem t : Prop := \u2203 x, True\n'
    ),
    "elab-set-literal-unknown-target": S("def s : Nat := {1}\n"),
    # ── kernel（12） ─────────────────────────────────────────────────────────
    "kernel-rejected": S("theorem t : Eq.{1} Nat 1 2 := Eq.refl.{1} Nat 1\n"),
    "kernel-expected-sort": S("theorem t : (fun (x : Prop) => x) := True.intro\n"),
    "kernel-expected-pi": S("theorem bad : True := True.intro True.intro\n"),
    "kernel-theorem-not-prop": S("theorem t : Nat := Nat.zero\n"),
    "kernel-prop-not-cumulative": S("def T : Type := True\n"),
    "kernel-inductive-non-positive": S(
        "inductive T : Type\nctor mk : (T -> Nat) -> T\nend\n"
    ),
    "kernel-ctor-result-mismatch": S("inductive T : Type\nctor mk : Nat\nend\n"),
    "kernel-ctor-arg-invalid-app": S("inductive T : Type\nctor mk : (T Nat) -> T\nend\n"),
    "kernel-ctor-arg-not-type": S(
        "inductive T : Type\nctor mk : (Nat.zero) -> T\nend\n"
    ),
    "kernel-ctor-arg-too-large": S(
        "inductive T : Type\nctor mk : (Type 1) -> T\nend\n"
    ),
    "kernel-rec-rule-mismatch": S(
        "inductive C : Type\nctor a : C\nctor b : C\n"
        "rec C.rec {u} : (motive : (c : C) -> Sort u) -> (ma : motive a) -> "
        "(mb : motive b) -> (c : C) -> motive c\n"
        "iota b :=\n  fun (motive : (c : C) -> Sort u) =>\n  fun (ma : motive a) =>\n"
        "  fun (mb : motive b) => mb\n"
        "iota a :=\n  fun (motive : (c : C) -> Sort u) =>\n  fun (ma : motive a) =>\n"
        "  fun (mb : motive b) => ma\nend\n"
    ),
    # ── import（7） ──────────────────────────────────────────────────────────
    "import-not-found": S("import No.Such\n"),
    "import-cycle": M(
        {"a.sokonanoda": "import a\ndef a : Prop := True\n"}, "a.sokonanoda"
    ),
    "import-dependency-failed": M(
        {"a.sokonanoda": _DEP_ENTRY, "b.sokonanoda": "def b : Prop := nosuch\n"},
        "a.sokonanoda",
    ),
    "import-name-collision": M(
        {"a.sokonanoda": _DEP_ENTRY, "b.sokonanoda": "def a : Prop := True\n"},
        "a.sokonanoda",
    ),
    "import-prelude-conflict": M(
        {
            "a.sokonanoda": _DEP_ENTRY,
            "b.sokonanoda": "-- sokonanoda:prelude none\ndef b : Prop := True\n",
        },
        "a.sokonanoda",
    ),
    "manifest-invalid": M(
        {"a.sokonanoda": _DEP_ENTRY, "b.sokonanoda": "def b : Prop := True\n"},
        "a.sokonanoda",
        manifest="this is not toml\n",
    ),
    # 依赖模块**自己**没解析成功：病在 b 上，入口视图只报 import-dependency-failed，
    # 所以这一条记录的是 `grade --json` 的事件流（含 `module` 字段）。
    "import-module-invalid": M(
        {"a.sokonanoda": _DEP_ENTRY, "b.sokonanoda": "def b : = \n"},
        "a.sokonanoda",
        mode="grade",
    ),
    # ── 警告（4） ────────────────────────────────────────────────────────────
    "reserved-declaration-name": S("def Prop : Type := Prop\n"),
    "import-has-open-exercises": M(
        {"a.sokonanoda": _DEP_ENTRY, "b.sokonanoda": "def b : Prop := sorry\n"},
        "a.sokonanoda",
    ),
    "redundant-sorry": S("theorem t : True := True.intro sorry\n"),
    "open-shadowed-name": S(
        "namespace A\ndef x : Prop := True\nend A\n"
        "namespace B\ndef x : Prop := True\nend B\nopen A\nopen B\ndef y : Prop := x\n"
    ),
}

# 跑不出复现的 9 条：**逐条给出为什么**，绝不拿"差不多的输出"顶替（D3 §diagnostics）。
REPRO_ABSENT = {
    "unexpected-eof": (
        "`DiagnosticKind::UnexpectedEof` 在 0.61.0 从未被构造："
        "`crates/front/src/diagnostic.rs` 里只有码与 hint，`parser.rs` 没有任何构造点"
        "（输入提前结束一律先报 `unexpected-token`）"
    ),
    "elab-too-many-binders": (
        "触发条件是 `u16::try_from(嵌套 binder / 归纳参数个数)` 溢出，需要 >65535 个，"
        "最小复现不成立（构造出来的文件约 1 MB，不叫最小复现）"
    ),
    "elab-nat-literal-disabled": (
        "触发条件是内核 `config.nat_extension == false`（`builder.rs:208`），"
        "而 0.61.0 的 CLI 没有任何路径关掉它：`--bare` 与 `-- sokonanoda:prelude none` "
        "都实测仍接受 `1`（`elab.rs:1881` 的 `alloc_bignum`/`mk_nat_lit` 都返回 Some）"
    ),
    "elab-invalid-nat-literal": (
        "`elab.rs:1874` 的 `value.parse::<BigUint>()` 永不失败："
        "词法层 `lex_number`（`token.rs:395-407`）只累积 ASCII 数字，"
        "`0x10`/`1_000`/`1e3` 都被切成 `1` + 标识符（报的是 `elab-unknown-identifier`）"
    ),
    "elab-too-many-ctor-fields": (
        "同 `elab-too-many-binders`：需要 >65535 个构造子字段（`elab.rs:543` 的 u16 溢出）"
    ),
    "elab-apply-needs-a-term": (
        "`ErrorKind::ElabApplyNeedsATerm` 在 0.61.0 从未被构造："
        "`crates/front/src/compile/error.rs` 只有码与 hint（hint 还写着 0.22.0 就删掉的 "
        "`funapply`），`elab.rs`/`by.rs` 里没有构造点"
    ),
    "elab-apply-not-applicable": (
        "同上：`ErrorKind::ElabApplyNotApplicable` 无构造点；"
        "`by apply <项>` 结论不对时实测报 `elab-tactic-failed`"
    ),
    "elab-match-recursive-unsupported": (
        "触发条件是递归归纳的构造子字段**没有源类型**（`elab.rs:2339`），"
        "而 `ctor_field_binders`（`elab.rs:3441`）恒给字段补上源类型："
        "`chain_binders_after` 为箭头链补 `Some(domain)`，`Forall` 的 binder 自带注解；"
        "无注解 binder 会先在 `elab-untyped-binder` 被拒"
    ),
    "kernel-internal": (
        "内核内部错误路径（`assertion failed:` / `unwrap` / `internal error:`）："
        "设计上**不可由学习者代码触发**（G-09 的结论——断言永不外泄是不变量），"
        "仓库里唯一已知的可达路径随 G-03/WO-006 关闭"
    ),
}


def _normalize_output(obj):
    """把记录下来的 CLI 输出里的仓库绝对路径换成 `<repo>`。

    只做**一处**替换（`REPO_ROOT` → `<repo>`）：`manifest-invalid` 的 message
    里带清单绝对路径，原样落盘会让数据绑死在这台机器的检出目录上。替换是
    无损、可逆、且写进数据里的（`output_normalization` 字段），不是改写结论。
    """
    text = json.dumps(obj, ensure_ascii=False)
    return json.loads(text.replace(REPO_ROOT, "<repo>"))


def _run_repro(code, spec, scratch):
    """跑一条复现，返回 `(record, reason)`。

    **只在真输出里出现了这个码时**才算复现成功——脚本自己核对，所以内核行为
    一变，复现就会自动退化成 `repro: null` + stderr 的人话，而不是留下一条
    与事实不符的"复现"。
    """
    directory = os.path.join(scratch, "repro", code)
    shutil.rmtree(directory, ignore_errors=True)
    os.makedirs(directory)
    with open(os.path.join(directory, "sokonanoda.toml"), "w", encoding="utf-8") as fh:
        fh.write(spec.get("manifest", 'name = "repro"\n'))
    files = spec.get("files") or {"main.sokonanoda": spec["src"]}
    for name, text in files.items():
        with open(os.path.join(directory, name), "w", encoding="utf-8") as fh:
            fh.write(text)
    entry = os.path.join(directory, spec.get("entry", "main.sokonanoda"))
    rel_dir = os.path.relpath(directory, REPO_ROOT)
    rel_entry = os.path.relpath(entry, REPO_ROOT)

    if spec.get("mode") == "grade":
        rc, out, err = cli(["grade", entry, "--json"])
        if rc is None:
            return None, f"grade 跑不起来（{err}）"
        events, bad = [], 0
        for line in out.splitlines():
            if not line.strip():
                continue
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError:
                bad += 1
        observed = [
            e.get("code") for e in events
            if e.get("type") in ("diagnostic", "warning")
        ]
        output = _normalize_output(events)
        command = f"{LAUNCHER} grade {rel_entry} --json"
        mode = "grade-json"
    else:
        obj, reason = cli_json(["query", "check", "--file", entry])
        if obj is None:
            return None, reason
        rc, out, err = cli(["query", "check", "--file", entry])
        data = obj.get("data") or {}
        observed = [f.get("code") for f in data.get("failed") or []] + [
            w.get("code") for w in data.get("warnings") or []
        ]
        output = _normalize_output(obj)
        command = f"{LAUNCHER} query check --file {rel_entry}"
        mode = "query-check"
        bad = 0

    if code not in observed:
        return None, (
            f"复现没触发这个码（实际输出 {observed or '空'}）——"
            f"内核行为可能已变，本条记 repro: null"
        )
    record = {
        "mode": mode,
        "command": command,
        "exit_code": rc,
        "entry": spec.get("entry", "main.sokonanoda"),
        "dir": rel_dir,
        "manifest": spec.get("manifest", 'name = "repro"\n'),
        "files": files,
        "observed_codes": observed,
        "unparsed_lines": bad,
        "output": output,
    }
    return record, ""


def build_diagnostics():
    """C1 §4.4 的 64 条码，逐条挂上**真跑出来的**复现（跑不出来的写明原因）。"""
    codes, reason = parse_c1_diagnostics()
    if not codes:
        _warn(f"{reason} —— 本次不生成 diagnostics.json（不重新推导诊断表）")
        return None

    ready, why = cli_ready()
    if not ready:
        _warn(f"{why} —— diagnostics.json 不带复现（只留 C1 §4.4 的字典部分）")

    scratch = ensure_scratch()
    with_repro = 0
    for entry in codes:
        code = entry["code"]
        spec = REPRO_TABLE.get(code)
        if spec is None:
            entry["repro"] = None
            entry["repro_absent_reason"] = REPRO_ABSENT.get(
                code, "本脚本没有为这个码写复现（不是「跑不出来」，是还没写）"
            )
            continue
        if not ready:
            entry["repro"] = None
            entry["repro_absent_reason"] = f"判卷环境未就绪：{why}"
            continue
        record, fail = _run_repro(code, spec, scratch)
        if record is None:
            entry["repro"] = None
            entry["repro_absent_reason"] = fail
            _warn(f"{code} 的复现没成立：{fail}")
        else:
            entry["repro"] = record
            with_repro += 1

    # 复现表里有、C1 §4.4 里没有的码 = 两处漂移，报出来。
    known = {entry["code"] for entry in codes}
    for code in sorted(set(REPRO_TABLE) - known):
        _warn(f"复现表里的 {code} 不在 C1 §4.4 的字典里 —— 请核对两处")

    by_stage = {}
    for entry in codes:
        by_stage[entry["stage"]] = by_stage.get(entry["stage"], 0) + 1
    totals = {
        "codes": len(codes),
        "errors": sum(1 for e in codes if e["stage"] != "warning"),
        "warnings": by_stage.get("warning", 0),
        "by_stage": dict(sorted(by_stage.items())),
        "with_repro": with_repro,
        "without_repro": len(codes) - with_repro,
    }
    print(
        f"  diagnostics: C1 §4.4 解析出 {totals['codes']} 条码 "
        f"（错误 {totals['errors']} + 警告 {totals['warnings']}）· "
        + " · ".join(f"{k}={v}" for k, v in totals["by_stage"].items())
        + f" · 真复现 {with_repro}/{len(codes)}"
    )
    return {
        "source": f"{C1_LANGUAGE} §4.4",
        "table_heading": "4.4 诊断码全表（按阶段分组，含真实 hint）",
        "output_normalization": "记录下来的 CLI 输出里，仓库绝对路径已替换为 `<repo>`",
        "repro_dir": os.path.relpath(os.path.join(scratch, "repro"), REPO_ROOT),
        "totals": totals,
        "codes": codes,
    }


# ─────────────────────────────────────────────────────────────────────────────
# evidence.json —— 质量证据数字（每一条都带产生它的确切命令）
# ─────────────────────────────────────────────────────────────────────────────

# 昂贵测量的缓存：`--with-tests` 真跑 cargo 并把结果写在这里；默认运行**读**
# 这个缓存并如实标注（命令 / 值 / 测量时的 commit / 是否已过期）。这样默认
# 全量跑不需要 5 分钟构建，而数据里仍然是**量过的数**、不是估的。
TEST_CACHE = os.path.join(".cache", "site-lab", "test-counts.json")


def _count_lines(rel_path):
    """Non-empty line count of a JSONL ledger."""
    text = _read(rel_path)
    if not text:
        return None
    return sum(1 for line in text.splitlines() if line.strip())


def _count_glob(rel_dir, pattern):
    """Files matching a glob in a repo directory (non-recursive), sorted."""
    directory = os.path.join(REPO_ROOT, rel_dir)
    try:
        names = sorted(n for n in os.listdir(directory) if fnmatch.fnmatch(n, pattern))
    except OSError:
        return None
    return names


def _measure_course_gate():
    """卷 I 课程门禁的 `--json` 摘要（判据的唯一真相，绝不在这里重算）。

    `gen-site-data.py` 用同一条命令测同一个门禁（`measure_set_theory`）——
    两处都**不重新实现判据**，因为判据双实现必然漂移。
    """
    gate = os.path.join(REPO_ROOT, SET_THEORY_GATE)
    if not os.path.isfile(gate):
        return None, f"缺少 {SET_THEORY_GATE}"
    proc = subprocess.run(
        [sys.executable or "python3", gate, "--json"],
        capture_output=True,
        text=True,
        timeout=600,
        cwd=REPO_ROOT,
        env=dict(os.environ, SOKONANODA_OFFLINE="1"),
    )
    try:
        report = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return None, (
            f"课程门禁输出不是 JSON（退出码 {proc.returncode}）："
            f"{_first_line(proc.stderr) or _first_line(proc.stdout)}"
        )
    summary = report.get("summary")
    if not isinstance(summary, dict):
        return None, "课程门禁报告里没有 summary"
    course = report.get("course") if isinstance(report.get("course"), dict) else {}
    return {
        "targets": summary.get("targets"),
        "checked": summary.get("checked"),
        "open": summary.get("open"),
        "rejected": summary.get("rejected"),
        "solutions_open": summary.get("solutions_open"),
        "lib_open": summary.get("lib_open"),
        "canvas_open": summary.get("canvas_open"),
        "units": report.get("units"),
        "volumes": course.get("volumes"),
        "chapters": course.get("chapters"),
    }, ""


def _measure_playground():
    """`grade playground.sokonanoda --json` 的事件计数（按 `type`）。"""
    path = os.path.join(REPO_ROOT, PLAYGROUND)
    if not os.path.isfile(path):
        return None, f"缺少 {PLAYGROUND}"
    rc, out, err = cli(["grade", path, "--json"], timeout=300)
    if rc is None:
        return None, f"grade 跑不起来（{err}）"
    counts = {}
    for line in out.splitlines():
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        key = event.get("type")
        if isinstance(key, str):
            counts[key] = counts.get(key, 0) + 1
    if not counts:
        return None, "playground 判卷没有吐出任何事件"
    return counts, ""


def _measure_release_assets(version):
    """`gh release view` 的资产数（要 gh + 网络；拿不到就报省略，不猜）。"""
    if not version:
        return None, "不知道版本号"
    try:
        proc = subprocess.run(
            ["gh", "release", "view", f"v{version}", "--json", "assets", "-q", ".assets | length"],
            capture_output=True,
            text=True,
            timeout=90,
            cwd=REPO_ROOT,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return None, f"gh 跑不起来（{exc}）"
    if proc.returncode != 0:
        return None, f"gh 退出码 {proc.returncode}：{_first_line(proc.stderr)}"
    try:
        return int(proc.stdout.strip()), ""
    except ValueError:
        return None, f"gh 的输出不是数字：{proc.stdout.strip()[:60]}"


def _measure_tests():
    """真跑 cargo 数测试（**很贵**，只在 `--with-tests` 时跑），并写进缓存。

    本机 cargo 链接需要 `DEVELOPER_DIR=/Library/Developer/CommandLineTools`
    （Xcode 许可未接受，仓库已文档化的坑，不是仓库 bug）。
    """
    cargo_env = dict(os.environ)
    cargo_env.setdefault("DEVELOPER_DIR", "/Library/Developer/CommandLineTools")
    results = {}
    for key, extra in (("total", []), ("ignored", ["--ignored"])):
        argv = ["cargo", "test", "--workspace", "--locked", "--", "--list"] + extra
        try:
            proc = subprocess.run(
                argv, capture_output=True, text=True, timeout=3600,
                cwd=REPO_ROOT, env=cargo_env,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            return None, f"cargo 跑不起来（{exc}）"
        if proc.returncode != 0:
            return None, f"cargo 退出码 {proc.returncode}：{_first_line(proc.stderr)}"
        results[key] = sum(1 for line in proc.stdout.splitlines() if line.endswith(": test"))
    cache = {
        "command_total": "DEVELOPER_DIR=/Library/Developer/CommandLineTools "
                         "cargo test --workspace --locked -- --list | grep -c ': test'",
        "command_ignored": "DEVELOPER_DIR=/Library/Developer/CommandLineTools "
                           "cargo test --workspace --locked -- --list --ignored | grep -c ': test'",
        "total": results["total"],
        "ignored": results["ignored"],
        "runnable": results["total"] - results["ignored"],
        "measured_at": get_generated_at(),
        "measured_commit": get_source_commit(),
    }
    path = os.path.join(REPO_ROOT, TEST_CACHE)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(cache, fh, ensure_ascii=False, indent=2, sort_keys=True)
        fh.write("\n")
    return cache, ""


def _read_test_cache():
    """The measured-once cargo numbers, if `--with-tests` ran before."""
    try:
        with open(os.path.join(REPO_ROOT, TEST_CACHE), encoding="utf-8") as fh:
            cache = json.load(fh)
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(cache, dict) or "total" not in cache:
        return None
    cache["stale"] = cache.get("measured_commit") != get_source_commit()
    return cache


def build_evidence(with_tests=False):
    """每个数字都是**现在量出来的**（或 `--with-tests` 量过一次并记了命令）。

    拿不到的数**不进 JSON**，只在 stderr 说明为什么（D2 §2 的生成纪律）。
    """
    version = get_version()
    measured = {}

    def record(key, value, command, source, note=None):
        if value is None:
            return
        item = {"value": value, "command": command, "source": source}
        if note:
            item["note"] = note
        measured[key] = item

    record(
        "repo_version", version,
        "grep -n '^version' Cargo.toml  # [workspace.package]",
        "Cargo.toml",
    )

    pkg = _read(VSCODE_PACKAGE)
    ext_version = None
    if pkg:
        try:
            ext_version = json.loads(pkg).get("version")
        except json.JSONDecodeError:
            ext_version = None
    if ext_version:
        record(
            "extension_version", ext_version,
            "python3 -c \"import json;print(json.load(open('editor/vscode/package.json'))['version'])\"",
            VSCODE_PACKAGE,
            "必须与 repo_version 一致（契约测试 cargo_and_extension_versions_match）",
        )
    else:
        _warn(f"{VSCODE_PACKAGE} 读不到 version —— evidence 不带 extension_version")

    # ── 课程门禁（26 秒左右，判据不在这里重实现） ──────────────────────────
    gate, reason = _measure_course_gate()
    if gate:
        record(
            "course_gate", gate,
            "python3 courses/set-theory/tools/check.py --json  # → summary",
            SET_THEORY_GATE,
            "卷 I 的计数只从门禁来（gen-site-data.py 用同一条命令）",
        )
    else:
        _warn(f"课程门禁没量成（{reason}）—— evidence 不带 course_gate")

    # ── playground 事件计数 ────────────────────────────────────────────────
    play, reason = _measure_playground()
    if play:
        record(
            "playground_events", play,
            "scripts/soko grade playground.sokonanoda --json  # 按 type 计数",
            PLAYGROUND,
        )
    else:
        _warn(f"playground 没量成（{reason}）—— evidence 不带 playground_events")

    # ── 四本台账的行数（文件行数，不是解析后的条数） ────────────────────────
    ledgers = {
        "gaps_ledger": GAPS_LEDGER,
        "perf_ledger": PERF_LEDGER,
        "e2e_ledger": E2E_LEDGER,
        "courses_ledger": COURSES_LEDGER,
    }
    rows = {}
    for key, path in sorted(ledgers.items()):
        count = _count_lines(path)
        if count is None:
            _warn(f"{path} 不存在或读不出来 —— evidence 不带 {key}")
            continue
        rows[key] = count
        record(key, count, f"wc -l < {path}", path)
    if rows:
        measured["ledger_rows"] = {
            "value": dict(sorted(rows.items())),
            "command": "wc -l < docs/gaps/ledger.jsonl  # 其余三本同形",
            "source": "docs/{gaps,perf,e2e,courses}/ledger.jsonl",
        }

    # ── CI 失败台账条数（只数带日期的真条目，模板头不算） ────────────────────
    ci = _read(CI_FAILURES)
    if ci:
        count = sum(
            1 for line in ci.splitlines()
            if re.match(r"^#{2,3} \d{4}-\d{2}-\d{2}", line)
        )
        record(
            "ci_failures", count,
            "grep -cE '^#{2,3} [0-9]{4}-[0-9]{2}-[0-9]{2}' docs/CI-FAILURES.md",
            CI_FAILURES,
        )
    else:
        _warn(f"{CI_FAILURES} 不存在或读不出来 —— evidence 不带 ci_failures")

    # ── 缺口台账的复现件 / 工作单文件数 ─────────────────────────────────────
    for key, directory, pattern, command in (
        ("work_orders", os.path.join("docs", "gaps"), "WO-*.md", "ls docs/gaps/WO-*.md | wc -l"),
        ("repro_files", os.path.join("docs", "gaps", "repro"), "*", "ls docs/gaps/repro | wc -l"),
    ):
        names = _count_glob(directory, pattern)
        if names is None:
            _warn(f"{directory} 不存在或读不出来 —— evidence 不带 {key}")
            continue
        record(key, len(names), command, directory)

    # ── git 侧的事实 ───────────────────────────────────────────────────────
    tags = _git(["tag"])
    if tags is not None:
        tag_list = [t for t in tags.split() if t]
        record("git_tags", len(tag_list), "git tag | wc -l", "git")
    else:
        _warn("git tag 读不到 —— evidence 不带 git_tags")
    commits = _git(["log", "--oneline"])
    if commits is not None:
        record("git_commits", len(commits.splitlines()), "git log --oneline | wc -l", "git")
    first = _git(["log", "--reverse", "--format=%ad", "--date=short"])
    if first and first.splitlines():
        record(
            "first_commit_date", first.splitlines()[0].strip(),
            "git log --reverse --format=%ad --date=short | head -1", "git",
        )
    last = _git(["log", "-1", "--format=%ad", "--date=short"])
    if last:
        record(
            "latest_commit_date", last.strip(),
            "git log -1 --format=%ad --date=short", "git",
        )

    changelog = _read(CHANGELOG)
    if changelog:
        record(
            "changelog_versions",
            sum(1 for line in changelog.splitlines() if line.startswith("## [")),
            "grep -c '^## \\[' editor/vscode/CHANGELOG.md", CHANGELOG,
        )
    else:
        _warn(f"{CHANGELOG} 不存在或读不出来 —— evidence 不带 changelog_versions")

    # ── 仓库里的 .sokonanoda 文件数（不含 target/ 与 .cache/ 的生成物） ──────
    total_soko = 0
    for dirpath, dirnames, filenames in os.walk(REPO_ROOT):
        dirnames[:] = [
            d for d in dirnames if d not in ("target", ".git", "node_modules", ".cache")
        ]
        total_soko += sum(1 for n in filenames if n.endswith(".sokonanoda"))
    record(
        "sokonanoda_files", total_soko,
        "find . -name '*.sokonanoda' -not -path './target/*' -not -path './.cache/*' | wc -l",
        "repo",
        "`.cache/` 是本脚本自己的复现件暂存区（.gitignore），不算仓库源",
    )

    # ── 最新一次真 VS Code e2e（台账最后一行） ──────────────────────────────
    e2e_text = _read(E2E_LEDGER)
    if e2e_text:
        last_row = None
        for line in e2e_text.splitlines():
            if line.strip():
                try:
                    last_row = json.loads(line)
                except json.JSONDecodeError:
                    last_row = None
        if isinstance(last_row, dict):
            record(
                "e2e_latest", {
                    "version": last_row.get("version"),
                    "commit": last_row.get("commit"),
                    "date": last_row.get("date"),
                    "vscode": last_row.get("vscode"),
                    "tests": last_row.get("tests"),
                    "exit": last_row.get("exit"),
                },
                "tail -1 docs/e2e/ledger.jsonl", E2E_LEDGER,
            )
        else:
            _warn(f"{E2E_LEDGER} 最后一行不是 JSON 对象 —— evidence 不带 e2e_latest")

    # ── 发布资产数（gh + 网络） ────────────────────────────────────────────
    assets, reason = _measure_release_assets(version)
    if assets is not None:
        record(
            "release_assets", assets,
            f"gh release view v{version} --json assets -q '.assets | length'",
            "GitHub Release",
            "需要 gh 已登录 + 网络；拿不到就不进 JSON（C4 §7.1 第 41 行标 ⚠️ 的就是它）",
        )
    else:
        _warn(f"发布资产数没量成（{reason}）—— evidence 不带 release_assets")

    # ── 测试计数：要么真跑（--with-tests），要么读量过一次的缓存 ─────────────
    tests = None
    if with_tests:
        tests, reason = _measure_tests()
        if tests is None:
            _warn(f"cargo 测试计数没量成（{reason}）—— evidence 不带 test_counts")
    if tests is None:
        tests = _read_test_cache()
        if tests is not None:
            tests["provenance"] = "measured-once-cache"
    if tests is not None:
        measured["test_counts"] = {
            "value": {
                "total": tests.get("total"),
                "ignored": tests.get("ignored"),
                "runnable": tests.get("runnable"),
            },
            "command": tests.get("command_total"),
            "source": "cargo test --workspace --locked -- --list",
            "measured_at": tests.get("measured_at"),
            "measured_commit": tests.get("measured_commit"),
            "stale": bool(tests.get("stale")),
            "note": (
                "`--with-tests` 实测并缓存（cargo 在本机需要 DEVELOPER_DIR 绕过 Xcode "
                "许可，见 C4 §7.3）；默认运行读缓存，`stale` 表示测量时的 commit 已不是 HEAD"
            ),
        }
        if tests.get("stale"):
            _warn(
                f"测试计数是 {tests.get('measured_commit')} 时量的，HEAD 已是 "
                f"{get_source_commit()} —— evidence 里标了 stale: true"
            )
    else:
        _warn(
            "测试计数没量过：加 --with-tests 真跑 cargo（本机约 5 分钟）"
            "—— 本次 evidence 不带 test_counts（不估）"
        )

    print(f"  evidence: {len(measured)} 组数字（每组带产生它的命令）")
    return {
        "rule": "只收能复现的数字；每条带 command；拿不到的不进 JSON，只在 stderr 说明",
        "measured": measured,
    }


# ─────────────────────────────────────────────────────────────────────────────
# timeline.json —— 真实时间线（git tag + CHANGELOG + STATUS + REQUIREMENTS §9）
# ─────────────────────────────────────────────────────────────────────────────

# 三个硬事实（C4 §2.3）——写页面时忽略任何一条，时间线就是错的：
#   1. 有 CHANGELOG 条目但**没有 tag** 的版本（0.57.0 是用户可见的大版本）；
#   2. 若干版本的 CHANGELOG 日期比 tag 提交日期**早一天**（跨零点提交）；
#   3. 一天可以发很多版（2026-09-15 一天 24 个 tag）——必须**聚合**。
# 本节的日期口径**只选一种**：有 tag 取 tag 指向提交的日期，无 tag 取 CHANGELOG
# 标注日期；`date_source` 逐条写明用的是哪一种，页面不需要猜。

_CHANGELOG_HEAD_RE = re.compile(r"^## \[([0-9][0-9.]*)\] - (\d{4}-\d{2}-\d{2})")
_C4_SPINE_HEAD = "### 2.2"
_C4_SPINE_END = "### 2.3"
_C4_ROW_RE = re.compile(r"^\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*$")
_CN_DIGITS = {
    "零": 0, "一": 1, "二": 2, "两": 2, "三": 3, "四": 4,
    "五": 5, "六": 6, "七": 7, "八": 8, "九": 9,
}


def _cn_to_int(token):
    """中文数字 → int（到万）。与 `scripts/gen-site-data.py:_cn_to_int` 同一份
    规则：STATUS.md 的轮次写成「第一百〇八轮」，`\\d+` 永远匹配不上。"""
    total = section = num = 0
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
            section = num = 0
        elif ch.isdigit():
            num = num * 10 + int(ch)
    value = total + section + num
    return value or None


def _clean_md(text, limit=240):
    """Markdown 行内标记去掉、空白折叠、超长截断（截断会标出来）。"""
    plain = re.sub(r"[*`]", "", text or "")
    plain = re.sub(r"\s+", " ", plain).strip()
    if len(plain) <= limit:
        return plain, False
    return plain[: limit - 1].rstrip() + "…", True


def parse_changelog():
    """`editor/vscode/CHANGELOG.md` → [{version, date, one_liner, line}]。

    「用户可见一句话」= 该版本下的**第一条 bullet**（CHANGELOG 是权威文案，
    C4 §2.1 第 ② 条）。原文是多行折行，这里折叠成一行并去掉 Markdown 强调；
    超过 240 字的标 `truncated: true`，不假装它是全文。
    """
    text = _read(CHANGELOG)
    if not text:
        return [], f"{CHANGELOG} 不存在或读不出来"
    lines = text.splitlines()
    entries = []
    current = None
    for index, line in enumerate(lines, start=1):
        head = _CHANGELOG_HEAD_RE.match(line)
        if head:
            current = {
                "version": head.group(1),
                "date": head.group(2),
                "line": index,
                "one_liner": None,
                "truncated": False,
            }
            entries.append(current)
            continue
        if current is None or current["one_liner"] is not None:
            continue
        if line.startswith("- "):
            one_liner, truncated = _clean_md(line[2:])
            current["one_liner"] = one_liner
            current["truncated"] = truncated
    if not entries:
        return [], f"{CHANGELOG} 里一条版本条目都没解析出来"
    return entries, ""


def parse_c4_spine():
    """C4 §2.2 的精选时间线（39 行）→ 骨架条目。

    它只是**骨架**：本脚本拿 `git tag` 逐条核对，核不上就记进
    `verified.tags_missing`（不悄悄丢掉、也不假装核过）。
    """
    text = _read(C4_ROADMAP)
    if not text:
        return [], f"{C4_ROADMAP} 不存在或读不出来"
    lines = text.splitlines()
    start = end = None
    for i, line in enumerate(lines):
        if line.startswith(_C4_SPINE_HEAD) and start is None:
            start = i
        elif start is not None and line.startswith(_C4_SPINE_END):
            end = i
            break
    if start is None or end is None:
        return [], f"{C4_ROADMAP} 里找不到 §2.2 的表格区间"
    rows = []
    for line in lines[start:end]:
        if not line.startswith("|"):
            continue
        row = _C4_ROW_RE.match(line)
        if not row:
            continue
        date, version, one_liner, evidence = (g.strip() for g in row.groups())
        if date in ("日期", "---") or set(date) <= {"-", " "}:
            continue
        rows.append(
            {
                "date": date,
                "version": version,
                "one_liner": one_liner,
                "evidence": evidence,
            }
        )
    if not rows:
        return [], f"{C4_ROADMAP} §2.2 里一行都没解析出来"
    return rows, ""


def parse_status_rounds():
    """`STATUS.md` + `docs/STATUS-ARCHIVE.md` 的「本轮进度」标题。

    历史标题有四种真实形状（归档里全都在），所以解析分两步：**日期 + 剩余文本**
    一定能拿到；轮次号与标题尽力解析，解析不出来的**如实标 `round_parsed: false`
    并把原文留下**——归档标题是人写的散文，不是每一条都能机器读，但一条都不丢：

      （2026-09-19，第一百〇八轮：标题）
      （2026-09-19，第一百〇五轮（语言线）：标题）
      （2026-09-07 第四轮：标题）          ← 日期与轮次之间没有顿号
      （2026-09-17，第九十一轮续：标题）    ← 轮次号后有后缀
      （2026-09-07，接手 agent 第 1–3 轮）  ← 压根没有「第N轮：标题」结构
    """
    rounds = []
    for rel_path in (STATUS_MD, STATUS_ARCHIVE):
        text = _read(rel_path)
        if not text:
            _warn(f"{rel_path} 不存在或读不出来 —— timeline 少这一份的轮次")
            continue
        for line in text.splitlines():
            if not line.startswith("## 本轮进度"):
                continue
            raw = line.strip()
            head = re.match(r"^##\s*本轮进度（(\d{4}-\d{2}-\d{2})\s*[，,]?\s*(.*)）\s*$", raw)
            if not head:
                _warn(f"{rel_path} 的轮次标题连日期都读不出，跳过：{raw[:60]}")
                continue
            date, rest = head.group(1), head.group(2).strip()
            round_no = None
            qualifier = None
            title = rest
            parsed = re.match(r"^第([^轮]+)轮(（[^）]*）)?[：:](.*)$", rest)
            if parsed:
                round_no = _cn_to_int(parsed.group(1))
                qualifier = (parsed.group(2) or "").strip() or None
                title = parsed.group(3).replace("`", "").strip()
            rounds.append(
                {
                    "date": date,
                    "round": round_no,
                    "round_parsed": round_no is not None,
                    "qualifier": qualifier,
                    "title": title,
                    "raw": raw,
                    "source": rel_path,
                }
            )
    rounds.sort(key=lambda r: (r["date"], r["round"] or 0, r["source"], r["raw"]))
    unparsed = [r for r in rounds if not r["round_parsed"]]
    if unparsed:
        _warn(f"{len(unparsed)} 条轮次标题没有「第N轮：标题」结构（round 记 null，原文保留）")
    return rounds


def parse_requirements_log():
    """`REQUIREMENTS.md` §9 的 dated 记录（`- YYYY-MM-DD…`，可续行）。"""
    text = _read(REQUIREMENTS)
    if not text:
        return [], f"{REQUIREMENTS} 不存在或读不出来"
    lines = text.splitlines()
    start = None
    for i, line in enumerate(lines):
        if line.startswith("## 9."):
            start = i + 1
            break
    if start is None:
        return [], f"{REQUIREMENTS} 里找不到 §9"
    entries = []
    current = None
    for line in lines[start:]:
        if line.startswith("## ") and not line.startswith("## 9."):
            break
        if line.startswith("- "):
            body = line[2:].strip()
            match = re.match(r"^(\d{4}-\d{2}-\d{2})(.*)$", body)
            if match:
                current = {"date": match.group(1), "text": match.group(2).strip()}
                entries.append(current)
            else:
                current = None
            continue
        if current is not None and line.strip() and not line.startswith("#"):
            current["text"] = (current["text"] + " " + line.strip()).strip()
    for entry in entries:
        entry["text"], entry["truncated"] = _clean_md(entry["text"], 300)
    if not entries:
        return [], f"{REQUIREMENTS} §9 里一条 dated 记录都没解析出来"
    return entries, ""


def build_timeline():
    """把四份源拼成一条**可核对**的时间线，并把三个硬事实算出来。"""
    changelog, reason = parse_changelog()
    if not changelog:
        _warn(f"{reason} —— 本次不生成 timeline.json（时间线的一半是 CHANGELOG）")
        return None

    tag_names = [t for t in (_git(["tag"]) or "").split() if t]
    tag_names.sort(key=lambda t: [int(p) if p.isdigit() else p for p in re.split(r"[.]", t.lstrip("v"))])
    if not tag_names:
        _warn("git tag 读不到 —— 本次不生成 timeline.json（tag 日期是时间线的日期口径）")
        return None

    tags = []
    for name in tag_names:
        date = (_git(["log", "-1", "--format=%ad", "--date=short", name]) or "").strip()
        subject = (_git(["log", "-1", "--format=%s", name]) or "").strip()
        if not date:
            _warn(f"tag {name} 的提交日期读不到 —— 这条不带 date")
        tags.append({"tag": name, "version": name.lstrip("v"), "date": date or None,
                     "subject": subject or None})

    tag_by_version = {t["version"]: t for t in tags}
    changelog_by_version = {c["version"]: c for c in changelog}

    versions_without_tag = sorted(set(changelog_by_version) - set(tag_by_version))
    versions_without_changelog = sorted(set(tag_by_version) - set(changelog_by_version))
    if versions_without_tag:
        print(
            "  timeline: 有 CHANGELOG 但没 tag 的版本 "
            f"{len(versions_without_tag)} 个：{'、'.join(versions_without_tag)}"
        )
    if versions_without_changelog:
        _warn(
            "有 tag 但 CHANGELOG 里没有条目的版本："
            + "、".join(versions_without_changelog)
        )

    # 日期差一天：CHANGELOG 的日期 vs tag 提交日期（C4 §2.3 第 2 条）。
    date_skew = []
    for version in sorted(set(changelog_by_version) & set(tag_by_version)):
        cl_date = changelog_by_version[version]["date"]
        tag_date = tag_by_version[version]["date"]
        if cl_date and tag_date and cl_date != tag_date:
            date_skew.append(
                {"version": version, "changelog_date": cl_date, "tag_date": tag_date}
            )

    # 一天多个 tag：聚合（C4 §2.3 第 3 条）。
    per_day = {}
    for tag in tags:
        if tag["date"]:
            per_day.setdefault(tag["date"], []).append(tag["tag"])
    days = [
        {"date": date, "count": len(names), "tags": sorted(names)}
        for date, names in sorted(per_day.items())
    ]

    spine, reason = parse_c4_spine()
    if not spine:
        _warn(f"{reason} —— timeline 不带 C4 §2.2 的骨架")
    verified_rows = 0
    known_untagged = set(versions_without_tag)
    for row in spine:
        found, missing = [], []
        for version in re.findall(r"\d+\.\d+\.\d+", row["version"]):
            (found if version in tag_by_version else missing).append(version)
        row["tags_found"] = found
        row["tags_missing"] = missing
        if found:
            verified_rows += 1
        tag_date = tag_by_version[found[0]]["date"] if found else None
        if tag_date:
            row["tag_date"] = tag_date
            row["date_source"] = "tag-commit-date"
            row["date_matches_tag"] = tag_date == row["date"]
        else:
            row["date_source"] = "changelog-date"
            row["changelog_date"] = next(
                (changelog_by_version[v]["date"] for v in missing
                 if v in changelog_by_version),
                None,
            )
        # 「没 tag」只有**不在已知无 tag 名单**里才值得报：0.1.0/0.2.0/0.3.0/
        # 0.6.0/0.9.1/0.57.0 六个是 C4 §2.3 第 1 条写明的既成事实，不是漂移。
        unexpected = [v for v in missing if v not in known_untagged]
        if unexpected:
            _warn(
                f"C4 §2.2 的「{row['version']}」里有 tag 也没 CHANGELOG 条目的版本："
                + "、".join(unexpected)
            )

    rounds = parse_status_rounds()
    requirements, req_reason = parse_requirements_log()
    if req_reason:
        _warn(f"{req_reason} —— timeline 不带 REQUIREMENTS §9")

    facts = {
        "tag_count": len(tags),
        "changelog_versions": len(changelog),
        "versions_without_tag": versions_without_tag,
        "versions_without_tag_count": len(versions_without_tag),
        "versions_without_changelog": versions_without_changelog,
        "changelog_date_differs_from_tag_date": len(date_skew),
        "changelog_date_skew": date_skew,
        "days_with_tags": len(days),
        "tags_per_day": {d["date"]: d["count"] for d in days},
        "busiest_day": max(days, key=lambda d: d["count"]) if days else None,
        "spine_rows": len(spine),
        "spine_rows_with_tag": verified_rows,
        "rounds": len(rounds),
        "requirements_entries": len(requirements),
        "v0_57_0_has_tag": "0.57.0" in tag_by_version,
    }
    print(
        f"  timeline: {len(tags)} 个 tag · {len(changelog)} 个 CHANGELOG 版本 · "
        f"无 tag {len(versions_without_tag)} 个 · 日期差一天 {len(date_skew)} 个 · "
        f"最忙一天 {facts['busiest_day']['date']}（{facts['busiest_day']['count']} 个 tag）· "
        f"骨架 {len(spine)} 行（{verified_rows} 行核到 tag）· "
        f"{len(rounds)} 轮 · §9 {len(requirements)} 条"
    )
    return {
        "date_convention": (
            "有 tag 的版本取 **tag 指向提交的日期**（`git log -1 --format=%ad --date=short <tag>`）；"
            "无 tag 的版本取 `editor/vscode/CHANGELOG.md` 的标注日期。"
            "两种口径逐条记在 `date_source` 里，全表只用这一种约定。"
        ),
        "sources": {
            "tags": "git tag + git log -1 --format=%ad --date=short <tag>",
            "changelog": CHANGELOG,
            "spine": f"{C4_ROADMAP} §2.2（39 行精选，本脚本逐条对 git tag 核对）",
            "rounds": f"{STATUS_MD} + {STATUS_ARCHIVE}",
            "requirements": f"{REQUIREMENTS} §9",
        },
        "facts": facts,
        "days": days,
        "spine": spine,
        "tags": tags,
        "changelog": changelog,
        "rounds": rounds,
        "requirements": requirements,
    }


# ─────────────────────────────────────────────────────────────────────────────
# main（临时骨架，后续小节依次接上）
# ─────────────────────────────────────────────────────────────────────────────


def main(argv):
    only = None
    with_tests = "--with-tests" in argv
    for i, arg in enumerate(argv):
        if arg == "--only":
            if i + 1 >= len(argv):
                print("--only 后面要跟节名（逗号分隔）：" + ",".join(FILES), file=sys.stderr)
                return 2
            only = {name.strip() for name in argv[i + 1].split(",") if name.strip()}
    if only is not None:
        unknown = sorted(only - set(FILES))
        if unknown:
            print(
                f"未知的节名：{'、'.join(unknown)}；可选：" + ",".join(FILES),
                file=sys.stderr,
            )
            return 2
    wanted = (lambda name: only is None or name in only)

    if wanted("walkthrough"):
        data = build_walkthrough()
        if data:
            write_json("walkthrough", envelope(data))

    if wanted("gaps"):
        data = build_gaps()
        if data:
            write_json("gaps", envelope(data))

    if wanted("events"):
        data = build_events()
        if data:
            write_json("events", envelope(data))

    if wanted("diagnostics"):
        data = build_diagnostics()
        if data:
            write_json("diagnostics", envelope(data))

    if wanted("timeline"):
        data = build_timeline()
        if data:
            write_json("timeline", envelope(data))

    if wanted("evidence"):
        data = build_evidence(with_tests=with_tests)
        if data:
            write_json("evidence", envelope(data))

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
