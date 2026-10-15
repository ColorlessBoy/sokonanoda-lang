#!/usr/bin/env python3
"""**咬住证明**（`docs/guard-bite.jsonl`）：守卫必须**咬得住一条已存在的坏输入**。

## 为什么要有这个文件（2026-10-15 用户【④】："停止守卫的守卫 ⇒ 出路是端到端判据"）

用户原话：

    现状循环：建守卫 → 假绿 → 建守卫看守卫 → …（**没有出口**）✗
    纠正：**任何新守卫必须自带"咬住证明"** —— 必须有一条**已存在的坏输入**
         （历史 bug / 用户的 case）被它咬住，且这条证明要**跑给人看**（一条命令 + 实测读数）✓
         仓库**已有**这条规则（「咬不住的守卫等于没有」）—— **只是没强制** ✗→✓
    判据：新守卫的交付物里没有"咬住证明" ⇒ **不算收口** ✗

⚠ 那条规则**原文在 `AGENTS.md:289`**（验证设计纪律 §3「反向验证是硬要求」）——
本文件**没有新造一条规则**，只是把**已有的**那条从"纪律"变成"判据" ✓
（用户说的就是这个：**"只是没强制"** ✗→✓）。

## 判据（三条，都能机械判）

* **C1 覆盖**：`--since <基点>..HEAD` 里**新增/修改的每一个守卫**都要在台账里有一行 ✓
  ⇒ 缺 ⇒ 判红（"新守卫的交付物里没有咬住证明 ⇒ 不算收口" ✗）。
  ⚠ 覆盖基线（`coverage_since`，写在台账抬头的注释里）**之前**就存在的守卫是
  **grandfathered**（不在强制面内）—— 这是**如实划的口径**，不是豁免 ✗：
  它们的清单由 `--list` 印出来（**评审可见** ✓），要收进来就补一行 ✓。
* **C2 咬得住**：每一行的 `pass` 命令必须 `rc == expect_rc`，且 **`bite` 命令必须
  `rc == expect_rc` 且 `expect_rc != 0`** ✗→✓ ——
  即"这条守卫在**那条已存在的坏输入**上**真的报红**" ✓。
  ⚠ `bite` 期望 0 ⇒ **判红**（"它咬不住" ✗）；`bite` 真的返回 0 ⇒ **判红** ✓。
* **C3 翻账（用户【①】）**：`--reopen-empty` —— 咬住证明**失效**的那一行，
  如果它点名了 `queue_id`，就**把那条已 done 的条目 reopen**（回 `todo`）✓，
  **不是**只记一条新账 ✗ ⇒ **"绿"不再免费** ✓。
* **C4 接线（2026-10-16，CI 实测红 ✗→✓）**：每条证明**声明它在哪个 CI job 里跑**
  （台账字段 `ci_job`，缺省 = `queue-lint`）⇒ 那个 job 必须在 `.github/workflows/ci.yml`
  里**真的存在**、且**真的有一跑它**（那个 job 的文本里出现 `--job <名字>`）✗→✓。
  ⚠ 为什么必须分派（**实测**）：`queue-lint` 是 `timeout-minutes: 5`、**不装
  `editor/vscode/node_modules`、也没有真 VS Code** —— 而第三层那几条证明要的正是那些
  （`scripts/third-layer-bite.sh` 环境缺失 ⇒ **exit 3**，而 `pass` 期望 0 ⇒ 判红 ✓）。
  ⇒ "一条命令扫全部"（`--all` 不分工）在 CI 上**必然红**，红的是"环境不在"、
  **不是**"守卫咬不住" ✗ —— 那是**假红**（读数不可归属 ✗）。
  ⚠ **这不是放宽** ✗：C4 保证**没有一条证明能靠"搬去一个不存在的 job"蒙混** ——
  声明了 job 而**没人跑它** ⇒ 判红 ✓（**空转闸** ✓）。要一次全跑（开发机）用 `--job all` ✓。
  ⚠ **分派的代价如实标注** ✗：`ci_job: <非默认>` 的那几条只在那个 job 的触发条件成立时
  跑（`editor` job 的条件是 `rust || editor` 改动）⇒ **纯 docs 推送不跑第三层那几条** ——
  它们**物理上**跑不进 `queue-lint` 的 5 分钟预算 ✓（不是口径松了 ✓）。

## 一处定义（`AGENTS.md` 硬规则 0(b)）

**守卫的枚举**（`guard_scripts()`）与**证明怎么跑**（`run_proof()`）**只有这一份** ✓；
`scripts/check-attention-budget.py --open-round` **调本文件**、不另抄 ✗。

## 退出码

  0 = 覆盖齐 + 每条都咬得住 / 1 = 判红 / 2 = 用法错 / 3 = 环境错（台账缺失等，**不假装判过** ✗）

## 用法

  python3 scripts/check-guard-bite.py                 # C1（基线..HEAD 的守卫）+ C2（逐条真跑）
  python3 scripts/check-guard-bite.py --since <sha>   # 覆盖面改成 <sha>..HEAD
  python3 scripts/check-guard-bite.py --all           # C2 跑**本 job 的**每一条（不只范围内的）
  python3 scripts/check-guard-bite.py --all --job all # C2 跑**台账里每一条**（开发机；CI 分派跑 ✗）
  python3 scripts/check-guard-bite.py --list          # 只印：台账覆盖了谁 · 谁是 grandfathered
  python3 scripts/check-guard-bite.py --reopen-empty  # ⭐ ① 翻账：咬不住的 ⇒ reopen 它关掉的那条
  python3 scripts/check-guard-bite.py --selftest      # 正反两向（含"bite 返回 0 ⇒ 必须判红"）
"""
from __future__ import annotations

import argparse
import fnmatch
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / "docs" / "guard-bite.jsonl"
NONBITING_FIXTURE = ROOT / "scripts" / "fixtures" / "guard-bite-nonbiting.jsonl"
CI_YML = ROOT / ".github" / "workflows" / "ci.yml"

# ── **证明在哪个 CI job 里跑**（C4；唯一一处 ✓）───────────────────────────────
# 缺省 = 跑 `--all` 的那个"每推必跑"的 job（`ci.yml` 的 `queue-lint`，无 `if` 过滤器 ✓）。
DEFAULT_JOB = "queue-lint"
# `--job all` = **不分工**（开发机一次全跑；CI 分派跑，理由见 docstring 的 C4 ✓）。
JOB_ALL = "all"
# job 名的合法形状（`ci.yml` 里 `  <name>:` 那一层）
_JOB_KEY_RE = re.compile(r"^  ([A-Za-z0-9_-]+):(\s|$)")

# ── **守卫的枚举**（唯一一处 ✓）──────────────────────────────────────────────
# 口径 = `scripts/` 下**判红用的**脚本（`check-*` / `*-lint.py` / 两本台账 / 审计）。
# ⚠ 加了一类新命名 ⇒ 只改这里一处（`check-attention-budget.py` 通过 CLI 复用 ✓）。
GUARD_GLOBS = ("scripts/check-*.py", "scripts/*-lint.py",
               "scripts/gap.py", "scripts/target-hygiene.py",
               "scripts/audit-*.py")
# 证明的超时（秒）：证明里可能起子进程 ⇒ 挂住会把门禁一起挂住（`gap.py` 的教训 ✓）
PROOF_TIMEOUT_S = 300


def guard_scripts(root: Path = ROOT) -> list[str]:
    """**全部守卫**（仓库相对路径，排序）。**唯一一处** ✓。"""
    found: set[str] = set()
    for pattern in GUARD_GLOBS:
        for path in root.glob(pattern):
            if path.is_file():
                found.add(path.relative_to(root).as_posix())
    return sorted(found)


def is_guard(path: str) -> bool:
    return any(fnmatch.fnmatch(path, g) for g in GUARD_GLOBS)


def entry_job(e: dict) -> str:
    """这条证明声明在哪个 CI job 里跑（缺省 = `DEFAULT_JOB` ✓）。"""
    return str(e.get("ci_job") or DEFAULT_JOB)


def job_blocks(text: str) -> dict[str, str]:
    """`ci.yml` 里每个 job 的**文本块**（`  <name>:` 到下一个同缩进的键）。

    只做**文本**扫描（与 `scripts/check-ci-gate-coverage.py` 同一条路 ✓）——
    C4 要的是"这个 job 里到底有没有那一跑"，不是"YAML 能不能解析" ✗。
    """
    blocks: dict[str, list[str]] = {}
    cur: str | None = None
    for line in text.splitlines():
        m = _JOB_KEY_RE.match(line)
        if m:
            cur = m.group(1)
            blocks.setdefault(cur, [])
        if cur is not None:
            blocks[cur].append(line)
    return {k: "\n".join(v) for k, v in blocks.items()}


def wiring_problems(root: Path, entries: list[dict]) -> list[str]:
    """**C4 接线**：每条证明声明的 `ci_job` 都要在 `ci.yml` 里存在且**真的跑它** ✓。

    ⚠ 这是"分派"的**空转闸**（docstring C4）：没有它，把一条证明标成
    `ci_job: 某个不存在的 job` 就等于**把它藏起来** ✗ —— 那才是放宽 ✗。
    """
    jobs = sorted({entry_job(e) for e in entries})
    if not jobs:
        return []
    yml = root / ".github" / "workflows" / "ci.yml"
    if not yml.exists():
        return [f"**C4 接线** ✗：`{yml}` 不在 ⇒ 判不了 `ci_job` 的接线"
                f"（**不假装判过** ✗）"]
    blocks = job_blocks(yml.read_text(encoding="utf-8", errors="replace"))
    red: list[str] = []
    for j in jobs:
        blk = blocks.get(j)
        if blk is None:
            red.append(f"**C4 接线** ✗：台账里有条目声明 `ci_job: {j}`，"
                       f"而 `{yml.name}` 里**没有这个 job** ⇒ 那条证明**没有任何 CI 会跑它**"
                       f"（藏起来 = 空转 ✗）")
            continue
        if "check-guard-bite.py" not in blk:
            red.append(f"**C4 接线** ✗：`ci_job: {j}` 的那个 job 里**没有任何一步**跑 "
                       f"`check-guard-bite.py` ⇒ 那批证明在 CI 里**空转** ✗")
            continue
        if j != DEFAULT_JOB and f"--job {j}" not in blk:
            red.append(f"**C4 接线** ✗：`ci_job: {j}` 的那个 job 里**没有** `--job {j}` "
                       f"⇒ 它跑的是**默认 job 的那批**，这批仍然没人跑 ✗")
    return red


def _git(root: Path, argv: list[str]) -> tuple[str, str | None]:
    out = subprocess.run(["git", *argv], cwd=root, capture_output=True, text=True)
    if out.returncode != 0:
        return "", f"`git {' '.join(argv)}` rc={out.returncode}：{out.stderr.strip()[:200]}"
    return out.stdout, None


def changed_guards(root: Path, since: str | None) -> tuple[list[str], str | None]:
    """`since..HEAD` 里**新增/修改**的守卫（`--diff-filter=AMR` ⇒ 只算加/改/改名 ✓）。"""
    if not since:
        return [], None
    out, err = _git(root, ["diff", "--name-only", "--diff-filter=AMR",
                           f"{since}..HEAD"])
    if err:
        return [], err
    return sorted({p for p in out.split() if is_guard(p)}), None


def shallow_hint(root: Path) -> str:
    """本仓若是**浅克隆** ⇒ 给出**可操作的**人话（否则空串 ✓）。

    ⚠ 为什么要有（2026-10-16 **CI 实测** ✗→✓，`EDITOR-JOB-BITE-STEP-EXIT-3`）：
    `editor` job 的 checkout 没有 `fetch-depth: 0` ⇒ 覆盖面基线（台账抬头的
    `# coverage_since:`）那个对象不在 ⇒ `changed_guards` 拿到 `git` 的 rc=128 ⇒
    本文件**只印一行原始 git 报错**，读日志的人**看不出缺什么、更不知道怎么办** ✗。
    同族的 `check-attention-budget.py` / `check-queue-views.py` / `check-threshold-ratchet.py`
    **早就**给了人话（"浅克隆 ⇒ CI 里给这个 job `fetch-depth: 0`"）⇒ 这里补齐，口径一致 ✓。

    ⚠ **只改话术，不改判据** ✗：退出码仍是 3（"环境不在，**不假装判过**"），
    C1/C2 一个字没动 ⇒ **绝不静默判过** ✓。
    """
    try:
        out = subprocess.run(["git", "rev-parse", "--is-shallow-repository"],
                             cwd=root, capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.SubprocessError):
        return ""
    if out.returncode != 0 or out.stdout.strip() != "true":
        return ""
    return ("本仓是**浅克隆**（`git rev-parse --is-shallow-repository` = true ⇒ "
            "**历史深度不够**，覆盖面基线那个对象不在 ⇒ 不是『这条守卫咬不住』 ✗）"
            "⇒ 出路：本机 `git fetch --unshallow`；**CI 里给跑这一步的那个 job 加 "
            "`fetch-depth: 0`** ✓（实测 `editor` job 缺的正是它：run 38817419732 · "
            "step 11；同 run 里 `queue-lint` 有它 ⇒ success ✓）。"
            "⚠ 别改判据来消这条红 ✗ —— 缺的是**前提**。")


def load_ledger(path: Path) -> tuple[list[dict], str, str | None]:
    """读台账 ⇒ (行, coverage_since, 错误)。`#` 开头的行是注释（照 `ux-queue-extras` 的先例 ✓）。"""
    if not path.exists():
        return [], "", (f"咬住证明台账不存在：`{path}` ⇒ **不假装判过** ✗。"
                        f"格式与写法见本文件 docstring；"
                        f"抬头的 `# coverage_since: <sha>` 是覆盖基线 ✓")
    entries: list[dict] = []
    coverage_since = ""
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line:
            continue
        if line.startswith("#"):
            if "coverage_since:" in line:
                coverage_since = line.split("coverage_since:", 1)[1].strip()
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError as err:
            return [], coverage_since, f"{path}:{lineno} 不是合法 JSON：{err}"
        missing = [k for k in ("guard", "bug", "pass", "bite") if k not in entry]
        if missing:
            return [], coverage_since, f"{path}:{lineno} 缺字段 {missing}"
        for half in ("pass", "bite"):
            if not isinstance(entry[half], dict) or "cmd" not in entry[half] \
                    or "expect_rc" not in entry[half]:
                return [], coverage_since, (f"{path}:{lineno} 的 `{half}` 要 "
                                            f"{{'cmd': …, 'expect_rc': …}}")
        entries.append(entry)
    return entries, coverage_since, None


def run_proof(cmd: str, root: Path, timeout: int = PROOF_TIMEOUT_S) -> tuple[int, str]:
    """跑一条证明 ⇒ (rc, 尾行摘要)。**唯一一处** ✓（`shell=True`：证明是**一条命令** ✓）。"""
    try:
        proc = subprocess.run(cmd, shell=True, cwd=root, capture_output=True, text=True,
                              encoding="utf-8", errors="replace", timeout=timeout)
    except subprocess.TimeoutExpired:
        return 124, f"证明超时（>{timeout}s）⇒ 按**判红**处理（不许静默跳过 ✗）"
    tail = [r.strip() for r in (proc.stdout + proc.stderr).splitlines() if r.strip()]
    return proc.returncode, " / ".join(r[:160] for r in tail[-2:])


def check(root: Path, ledger_path: Path, since: str | None, run_all: bool,
          reopen_empty: bool = False, quiet: bool = False,
          job: str = DEFAULT_JOB) -> int:
    """C1 覆盖 + C2 咬得住（+ C3 翻账 + C4 接线）。返回退出码 ✓。

    `job` = 这一次跑**哪个 CI job 的那批证明**（`JOB_ALL` ⇒ 不分工，开发机用 ✓）。
    """
    entries, coverage_since, err = load_ledger(ledger_path)
    if err:
        print(f"error: {err}", file=sys.stderr)
        return 3
    by_guard = {e["guard"]: e for e in entries}
    base = since or coverage_since
    if not base:
        print("error: 台账抬头没有 `# coverage_since: <sha>`，也没给 `--since` ⇒ "
              "覆盖面算不了（**不假装判过** ✗）", file=sys.stderr)
        return 3

    scope, cerr = changed_guards(root, base)
    if cerr:
        # ⚠ **响亮 + 印出缺什么**（2026-10-16 CI 实测）：光印 `git` 的 rc=128
        # 读不出"缺的是历史深度" ⇒ 浅克隆时补一行**可操作**的人话 ✓。
        # 退出码**仍是 3**（环境不在 ⇒ 不假装判过 ✗）—— 只是话术，不是放宽 ✓。
        print(f"error: {cerr}", file=sys.stderr)
        hint = shallow_hint(root)
        if hint:
            print(f"error: {hint}", file=sys.stderr)
        return 3

    # **本 job 的那批**（C4 的分派面 ✓）：`JOB_ALL` = 不分工。
    pool = entries if job == JOB_ALL else [e for e in entries if entry_job(e) == job]

    red: list[str] = []
    if not quiet:
        print(f"咬住证明（台账 `{ledger_path.name}` · 覆盖基线 `{base[:12]}` · "
              f"job `{job}`）：{len(entries)} 条证明 · 本 job {len(pool)} 条 · "
              f"基线以来新增/修改的守卫 {len(scope)} 个")

    # ── C2 空转闸：`--job <名字>` 必须在台账里**真有人声明** ────────────────
    #   ⚠ 没有这一条，`--job 打错的名字` ⇒ **0 条证明、全绿** ⇒ "跑了"其实是空转 ✗
    #   （与 C4 对称：C4 管"声明的 job 有人跑"，这一条管"跑的名字有人声明" ✓）。
    if job != JOB_ALL and not pool:
        red.append(f"**C2 空转** ✗：`--job {job}` 在台账里**没有任何条目**声明它 ⇒ "
                   f"这一跑**什么都没跑**（空转 ✗；台账里声明的 job："
                   f"{' · '.join(sorted({entry_job(e) for e in entries}))}）")

    # ── C1 覆盖 ─────────────────────────────────────────────────────────
    missing = [g for g in scope if g not in by_guard]
    if missing:
        red.append("**C1 覆盖** ✗：基线以来新增/修改的守卫**没有咬住证明** ⇒ 本轮不算收口："
                   + " · ".join(f"`{g}`" for g in missing)
                   + f"（在 `{ledger_path.name}` 补一行：`guard` / `bug`（**已存在的坏输入**）"
                     f"/ `pass` / `bite`（`expect_rc` **非 0** ✓））")
    elif not quiet:
        print(f"  ✓ C1 覆盖：基线以来的 {len(scope)} 个守卫都有证明"
              + (f"（{', '.join(scope)}）" if scope else "（本窗没有新增/修改的守卫）"))

    # ── C4 接线（分派的空转闸 ✓）──────────────────────────────────────────
    c4 = wiring_problems(root, entries)
    red.extend(c4)
    if not quiet and not c4:
        jobs = sorted({entry_job(e) for e in entries})
        print(f"  ✓ C4 接线：台账点名的 {len(jobs)} 个 job 都在 `{CI_YML.name}` 里且**真的跑它**"
              f"（{' · '.join(jobs)}）")

    # ── C2 咬得住 ───────────────────────────────────────────────────────
    targets = pool if run_all else [e for e in pool if e["guard"] in scope]
    if run_all:
        # `--all` 另加一条：台账里点名的守卫**必须真的存在**（防表自己腐烂 ✗）
        for e in entries:
            if not (root / e["guard"]).exists():
                red.append(f"**C2 台账腐烂** ✗：`{e['guard']}` 在台账里、但文件不存在 ✗")
    if not quiet:
        print(f"  C2 逐条真跑（{len(targets)} 条；`bite` 必须**非 0** ⇒ 它真的咬住了那条坏输入 ✓）")
    empty: list[dict] = []
    for e in targets:
        g = e["guard"]
        if not (root / g).exists():
            red.append(f"**C2** ✗：`{g}` 不在仓库里（台账点名了一个不存在的守卫）✗")
            continue
        prc, ptail = run_proof(e["pass"]["cmd"], root)
        brc, btail = run_proof(e["bite"]["cmd"], root)
        p_ok = prc == e["pass"]["expect_rc"]
        b_ok = brc == e["bite"]["expect_rc"] and e["bite"]["expect_rc"] != 0
        if not quiet:
            print(f"    {'✓' if p_ok else '✗'} {g} `pass` rc={prc}"
                  f"（期望 {e['pass']['expect_rc']}）")
            print(f"    {'✓' if b_ok else '✗'} {g} `bite` rc={brc}"
                  f"（期望 {e['bite']['expect_rc']}，**必须非 0**）"
                  + (f" ⇒ {btail[:110]}" if b_ok else ""))
        if e["bite"]["expect_rc"] == 0:
            red.append(f"**C2** ✗：`{g}` 的 `bite.expect_rc = 0` ⇒ **它咬不住** "
                       f"（咬住证明必须是「坏输入 ⇒ **非 0**」✗→✓）")
        elif not p_ok:
            red.append(f"**C2** ✗：`{g}` 的 `pass` rc={prc}（期望 {e['pass']['expect_rc']}）"
                       f" ⇒ 守卫自己坏了：{ptail[:160]}")
        elif not b_ok:
            red.append(f"**C2** ✗：`{g}` 的 `bite` rc={brc}（期望 {e['bite']['expect_rc']}）"
                       f" ⇒ **这条守卫咬不住它声称咬住的那条坏输入**"
                       f"（`{e.get('bug', '')[:80]}`）：{btail[:160]}")
            empty.append(e)

    # ── C3 翻账（用户【①】）─────────────────────────────────────────────
    if reopen_empty and empty:
        print(f"\n⭐ **翻账**（用户 2026-10-15【①】：假绿要 **reopen 它关掉的那条**，"
              f"不只是记新账 ✓）：{len(empty)} 条守卫空了", file=sys.stderr)
        for e in empty:
            qid = e.get("queue_id")
            if not qid:
                print(f"  · `{e['guard']}` 的台账行**没有 `queue_id`** ⇒ 不知道该 reopen 谁 ✗ "
                      f"（**如实说**：这条只能靠外部验收方手工翻账）", file=sys.stderr)
                continue
            cmd = [sys.executable, str(root / "scripts" / "ux-queue.py"),
                   "--advance", qid, "--to", "todo", "--reason",
                   f"守卫 {e['guard']} 的咬住证明已失效（假绿）⇒ 翻账（check-guard-bite --reopen-empty）"]
            out = subprocess.run(cmd, cwd=root, capture_output=True, text=True,
                                 encoding="utf-8", errors="replace")
            if out.returncode == 0:
                print(f"  ✓ **reopen `{qid}`**（{e['guard']}）⇒ 回 `todo` ✓", file=sys.stderr)
            else:
                # ⚠ **实测的可行性缺口**（2026-10-16，`file:line` = `ux-queue.py:2768-2787`）：
                #   来源处置**已关账**（extras 的 `disposition=fixed`）时，`--advance --to todo`
                #   **故意**拒（否则造出「来源关账 + 调度器 todo」= 反向两层漂移 ✗）。
                #   ⇒ 用户 2026-10-15【①】假设的机制（"队列已有 `--advance … --to todo --reason`"）
                #   **只在来源没关账时成立** ✗ —— 而 extras 里 201/208 条都是 `fixed` ✗。
                #   ⇒ 翻账要**两步**，且第一步是 `--triage`（**接口 agent 契约里没有** ✗）。
                #   本文件**不绕过**它（写权限是队列自己的口径 ✓）—— **如实印出可复制的两步** ✓。
                reason = (f"守卫 {e['guard']} 的咬住证明已失效（假绿）⇒ 翻账："
                          f"{str(e.get('bug', ''))[:80]}")
                print(f"  ✗ `{qid}` reopen 没成（rc={out.returncode}）—— "
                      f"**来源已关账时 `--advance` 回不去**（`ux-queue.py:2768` 的口径 ✓），"
                      f"可复制的两步（**第一步是 `--triage`** ⇒ 接口 agent 契约里没有 ✗，"
                      f"只有主进程能做）：\n"
                      f"      ① python3 scripts/ux-queue.py --triage {qid} --to deferred "
                      f"--reason '{reason}'\n"
                      f"      ② python3 scripts/ux-queue.py --advance {qid} --to todo "
                      f"--reason '{reason}'\n"
                      f"    （**假绿本身另记一条**——用户【①】的后半句：\n"
                      f"      `python3 scripts/ux-queue.py --schedule {qid}-FALSE-GREEN "
                      f"--facet 主线 --severity high --repro '（无复现件：判据见 where）' "
                      f"--why '<守卫 {e['guard']} 的咬住证明失效 ⇒ 那条 done 是假绿>' "
                      f"--where '{e['guard']} 的 bite 命令'`）\n"
                      f"    {out.stderr.strip()[:160]}", file=sys.stderr)

    if red:
        print(f"\n✗ 咬住证明 **{len(red)} 条红**：", file=sys.stderr)
        for m in red:
            print(f"  · {m}", file=sys.stderr)
        return 1
    if not quiet:
        print("✓ 咬住证明：覆盖齐 + 每条都**真的咬住了**一条已存在的坏输入 ✓")
    return 0


def list_coverage(root: Path, ledger_path: Path) -> int:
    """`--list`：谁有证明 · 谁是 grandfathered（**评审可见** ✓，不是豁免 ✗）。"""
    entries, coverage_since, err = load_ledger(ledger_path)
    if err:
        print(f"error: {err}", file=sys.stderr)
        return 3
    covered = {e["guard"] for e in entries}
    allg = guard_scripts(root)
    print(f"台账 `{ledger_path}`：{len(entries)} 条证明 · 覆盖基线 `{coverage_since[:12]}`")
    for e in entries:
        print(f"  ✓ {e['guard']}  [job {entry_job(e)}]\n"
              f"      坏输入：{e.get('bug', '')[:110]}\n"
              f"      bite：`{e['bite']['cmd'][:110]}` ⇒ 期望 rc={e['bite']['expect_rc']}")
    for j in sorted({entry_job(e) for e in entries}):
        n = sum(1 for e in entries if entry_job(e) == j)
        print(f"  · job `{j}`：{n} 条" + ("（默认，`--all` 就跑这批 ✓）"
                                          if j == DEFAULT_JOB else "（`--job %s` ✓）" % j))
    rest = [g for g in allg if g not in covered]
    print(f"\n⚠ **grandfathered**（基线之前就存在、**不在强制面内** ✗ —— "
          f"如实划的口径，不是豁免 ✓）：{len(rest)} 个")
    for g in rest:
        print(f"    · {g}")
    return 0


# ── 判据通道自检（反向验证：故意造出该现象 ⇒ 必须判红）──────────────────────
def selftest(job: str = DEFAULT_JOB) -> int:
    bad = 0

    def gate(name: str, ok: bool, note: str = "") -> None:
        nonlocal bad
        if not ok:
            bad += 1
        print(f"  {'✓' if ok else '✗'} {name}{('：' + note) if note else ''}")

    print(f"咬住证明自检（**真跑**，每条都带正反两向 ✓ · job `{job}`）：")

    # ── 正对照：**真台账**里**本 job 的**每条逐条跑 ⇒ 必须 rc=0，且 bite 必须真的非 0 ──
    rc = check(ROOT, LEDGER, None, run_all=True, quiet=True, job=job)
    gate(f"正对照：**真台账**（`--all` 逐条真跑 · job `{job}`）⇒ rc=0", rc == 0, f"rc={rc}")
    entries, _cs, err = load_ledger(LEDGER)
    gate("正对照：真台账读得进来且非空", not err and bool(entries),
         f"{len(entries)} 条" + (f" · {err}" if err else ""))
    gate("正对照：每条 `bite.expect_rc` 都是**非 0**（咬住 = 坏输入 ⇒ 红 ✓）",
         all(e["bite"]["expect_rc"] != 0 for e in entries),
         " · ".join(f"{Path(e['guard']).name}→{e['bite']['expect_rc']}" for e in entries))
    gate("正对照：每条都有 `bug`（**已存在的坏输入**是必填 ✓）",
         all(str(e.get("bug", "")).strip() for e in entries))

    # ── 反向①：**bite 返回 0** ⇒ 必须判红（"它咬不住" ✗）──────────────────
    with tempfile.TemporaryDirectory(prefix="soko-bite-") as td:
        tmp = Path(td)
        # 覆盖基线 = HEAD ⇒ 范围内没有守卫 ⇒ 只考 C2
        head, _herr = _git(ROOT, ["rev-parse", "HEAD"])
        head = head.strip()
        nonbiting = tmp / "nonbiting.jsonl"
        nonbiting.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-attention-budget.py",
                          "bug": "故意：bite 命令永远返回 0（= 咬不住）",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "true", "expect_rc": 0}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_nb = check(ROOT, nonbiting, None, run_all=True, quiet=True)
        gate("反向①：`bite.expect_rc = 0`（**咬不住**）⇒ 判红 rc=1", rc_nb == 1, f"rc={rc_nb}")

        # ── 反向②：**bite 真的返回 0**（期望非 0）⇒ 必须判红 ──────────────
        lying = tmp / "lying.jsonl"
        lying.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-attention-budget.py",
                          "bug": "故意：期望它报红，它却返回 0",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "true", "expect_rc": 1}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_lie = check(ROOT, lying, None, run_all=True, quiet=True)
        gate("反向②：`bite` 期望 1 却返回 0（**守卫空了**）⇒ 判红 rc=1",
             rc_lie == 1, f"rc={rc_lie}")

        # ── 反向③：**pass 坏了** ⇒ 必须判红 ───────────────────────────────
        broken = tmp / "broken.jsonl"
        broken.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-attention-budget.py",
                          "bug": "故意：守卫自己坏了",
                          "pass": {"cmd": "false", "expect_rc": 0},
                          "bite": {"cmd": "false", "expect_rc": 1}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_b = check(ROOT, broken, None, run_all=True, quiet=True)
        gate("反向③：`pass` 返回 1（期望 0）⇒ 判红 rc=1", rc_b == 1, f"rc={rc_b}")

        # ── 反向④：**C1 覆盖** —— 范围内有守卫但台账里没有 ⇒ 必须判红 ──────
        # 夹具：`--since` 指向一个真守卫被改过的提交 ⇒ 范围非空；台账空 ⇒ 必红。
        out, _e = _git(ROOT, ["log", "--format=%H", "--diff-filter=AMR",
                              "-1", "--", "scripts/check-attention-budget.py"])
        touch = out.strip().splitlines()[-1] if out.strip() else ""
        if touch:
            empty_ledger = tmp / "empty.jsonl"
            empty_ledger.write_text(f"# coverage_since: {head}\n", encoding="utf-8")
            rc_cov = check(ROOT, empty_ledger, touch + "~1", run_all=True, quiet=True)
            gate("反向④：**C1 覆盖** —— 范围内改了守卫、台账里没有 ⇒ 判红 rc=1",
                 rc_cov == 1, f"rc={rc_cov}（since={touch[:12]}~1）")
        else:
            gate("反向④：**C1 覆盖** —— 范围内改了守卫、台账里没有 ⇒ 判红 rc=1",
                 False, "**夹具前提没成立**（git log 没找到改过该文件的提交）✗")

        # ── 反向⑤：**台账腐烂**（点名不存在的守卫）⇒ 必须判红 ────────────
        rotten = tmp / "rotten.jsonl"
        rotten.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-does-not-exist.py",
                          "bug": "故意：点一个不存在的守卫",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "false", "expect_rc": 1}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_r = check(ROOT, rotten, None, run_all=True, quiet=True)
        gate("反向⑤：台账点名**不存在的守卫** ⇒ 判红 rc=1", rc_r == 1, f"rc={rc_r}")

        # ── 反向⑥：台账**缺失** ⇒ rc=3（**不假装判过** ✗）────────────────
        rc_m = check(ROOT, tmp / "nope.jsonl", None, run_all=True, quiet=True)
        gate("反向⑥：台账**不存在** ⇒ rc=3（不假装判过 ✗）", rc_m == 3, f"rc={rc_m}")

        # ── 反向⑦：**C4 接线** —— 声明了一个 `ci.yml` 里没有的 job ⇒ 判红 ──
        #   （**分派的空转闸**：没有它，把证明标成不存在的 job 就等于藏起来 ✗）
        #   ⚠ 这条夹具**其余都合法**（pass/bite 都对）⇒ 判红**可归属**到 C4 ✓。
        ghost = tmp / "ghost-job.jsonl"
        ghost.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-attention-budget.py",
                          "ci_job": "no-such-job-in-ci-yml",
                          "bug": "故意：把证明声明到一个不存在的 CI job（= 藏起来）",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "false", "expect_rc": 1}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_g = check(ROOT, ghost, None, run_all=True, quiet=True)
        gate("反向⑦：**C4 接线** —— `ci_job` 指向不存在的 job（藏起来 = 空转）⇒ 判红 rc=1",
             rc_g == 1, f"rc={rc_g}")

        # ── 反向⑧：job 存在、但**里面没有那一跑** ⇒ 判红（同上，另一档）──────
        unwired = tmp / "unwired-job.jsonl"
        unwired.write_text(
            f"# coverage_since: {head}\n"
            + json.dumps({"guard": "scripts/check-attention-budget.py",
                          "ci_job": "lint-fmt",
                          "bug": "故意：job 是真的，但里面没有 `--job lint-fmt` 那一跑",
                          "pass": {"cmd": "true", "expect_rc": 0},
                          "bite": {"cmd": "false", "expect_rc": 1}},
                         ensure_ascii=False) + "\n", encoding="utf-8")
        rc_u = check(ROOT, unwired, None, run_all=True, quiet=True)
        gate("反向⑧：job 存在但**没有那一跑**（`--job <名字>` 不在）⇒ 判红 rc=1",
             rc_u == 1, f"rc={rc_u}")

        # ── 反向⑨：`--job <没人声明的名字>` ⇒ 判红（**空转闸**：0 条不许算绿 ✗）──
        rc_e = check(ROOT, LEDGER, None, run_all=True, quiet=True, job="no-such-job")
        gate("反向⑨：`--job <台账里没人声明的名字>`（0 条 = 空转）⇒ 判红 rc=1",
             rc_e == 1, f"rc={rc_e}")

    # ── 正对照：**真坏输入**被咬住的形状（`--until c82097fe` = 历史最差点）──
    entries2, _cs2, _e2 = load_ledger(LEDGER)
    hist = next((e for e in entries2
                 if e["guard"].endswith("check-attention-budget.py")), None)
    if hist:
        rc_h, tail_h = run_proof(hist["bite"]["cmd"], ROOT)
        gate("⭐ 正对照：**历史最差点**（`--until c82097fe` ⇒ 19.32 : 1）**真被咬住**",
             rc_h == hist["bite"]["expect_rc"] and rc_h != 0,
             f"rc={rc_h} · {tail_h[:100]}")
    else:
        gate("⭐ 正对照：历史最差点被咬住", False, "台账里没有 check-attention-budget 那一行 ✗")

    print(f"\n自检：{'✓ 全过' if bad == 0 else f'✗ {bad} 组不对'}"
          f"（正对照 4 + 反向 9 + 真跑台账）")
    return 0 if bad == 0 else 1


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="咬住证明（守卫必须咬得住一条已存在的坏输入）")
    ap.add_argument("--root", default=str(ROOT))
    ap.add_argument("--ledger", default=str(LEDGER))
    ap.add_argument("--since", default=None, help="覆盖面 = <sha>..HEAD（默认台账的 coverage_since）")
    ap.add_argument("--all", action="store_true",
                    help="C2 跑**本 job 的**每一条（不只范围内的）")
    ap.add_argument("--job", default=DEFAULT_JOB,
                    help=f"跑哪个 CI job 的那批证明（台账的 `ci_job`；缺省 `{DEFAULT_JOB}`；"
                         f"`{JOB_ALL}` = 不分工，开发机一次全跑 ✓）。"
                         f"⚠ 每个 `ci_job` 都必须在 `ci.yml` 里**真的有一跑** ⇒ 否则 C4 判红 ✗")
    ap.add_argument("--check", action="store_true",
                    help="（默认动作；显式写出来给调用方用 ✓ —— `check-attention-budget.py "
                         "--open-round` 就是这么调的 ✓）")
    ap.add_argument("--list", action="store_true", help="只印覆盖情况（含 grandfathered 清单）")
    ap.add_argument("--reopen-empty", action="store_true",
                    help="⭐ ① 翻账：咬不住的 ⇒ reopen 它关掉的那条（回 todo）")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args(argv)

    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, ValueError, OSError):
            pass

    if args.selftest:
        return selftest(args.job)

    root = Path(args.root).resolve()
    ledger = Path(args.ledger)
    if not ledger.is_absolute():
        ledger = (root / ledger).resolve()
    if args.list:
        return list_coverage(root, ledger)
    return check(root, ledger, args.since, args.all, reopen_empty=args.reopen_empty,
                 job=args.job)


if __name__ == "__main__":
    sys.exit(main())
