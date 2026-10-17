#!/usr/bin/env python3
"""**「跑历史判据的 job 必须有全历史」的类守卫**（洞 `EDITOR-JOB-BITE-STEP-EXIT-3`，2026-10-16）。

## 为什么有这个脚本（"修实例不修类"的解药）

同族**两次**事故，每次都是同一句话：

| # | 实例 | 形态 |
|---|------|------|
| ① | `queue-lint`（2026-10-15） | `ux-queue.py --check` 的祖先性断言在浅克隆里**判不了** ⇒ 假红 270 条 ⇒ **手加** `ci.yml` 的 `fetch-depth: 0` ✗ |
| ② | `editor`（2026-10-16 · run `38817419732` · step 11） | `check-guard-bite.py` 的 **C1 覆盖面**要算 `git diff <coverage_since>..HEAD`（基线 = `docs/guard-bite.jsonl` 抬头的 `# coverage_since:`）⇒ 浅克隆里**那个对象不在** ⇒ `git` **rc=128** ⇒ 判据 **exit 3** ⇒ **job 红**，而红的是**前提**、不是"咬不住" ✗ |

两次的修法都是**手工往某一个 job 的 checkout 加一行** ⇒
**这就是"修实例不修类"**：每次都在修**那个** job，没人把
"**跑历史判据的 job 必须有全历史**"变成可查的判据 ✗。

⚠ **同类第 ② 次是 CI 实测**（不是推理）：**同一个 run 里** `queue-lint`（**有**
`fetch-depth: 0`）跑**同一条** `check-guard-bite.py` 命令是 **success** ✓ ——
⇒ 唯一的差别就是**历史深度** ✓。本机用真 `git clone --depth 1` 夹具**逐字**复现了
CI 的那两行（`fatal: ambiguous argument 'cf259d1f..HEAD'` + `rc=3`）✓。

## 类不变量（四条，都能机械判）

* **R1 接线**：凡 job **真的跑**声明过的历史判据脚本（`HISTORY_DEPENDENT`），
  该 job 的 `actions/checkout` 就必须带 `fetch-depth: 0` ✓。
  —— 反过来说：**判据要读历史，就必须把历史给它** ✗。
* **R1b 提及不算跑**（2026-10-17 · 第 6 轮 `ng7` 加）：**非注释活行里点名了**声明过的脚本、
  而它**任何一条 `run:` 正文里都没有** ⇒ **判红**（那一跑**根本不存在** = 空转 ✗）。
  —— 反过来说：**step 名不是命令** ✗（`check-guard-bite.py` 的 `run_bodies()` docstring
  里早就写着这一句 ✓ —— 本脚本落地时只收了"注释不算"、**漏了 step 名** ✗）。
* **R2 表不腐烂**：`HISTORY_DEPENDENT` 点名的脚本必须**在仓库里存在** ⇒ 否则判红
  （表里留着一个已经不存在的脚本 = 表在腐烂 ✗）。
* **R3 空转闸**：这一跑至少要判到**一个** `(job, 脚本)` 对 ⇒ 否则判红 ——
  判据自己扫空了（正则坏了 / `ci.yml` 改了结构 / `HISTORY_DEPENDENT` 被清空）
  **不许当通过** ✗（"咬不住的守卫等于没有" ✓）。

## 两处刻意收窄的口径（都是**如实划的**，不是放宽 ✗）

* **只认 `run:` 正文**（复用 `check-guard-bite.py` 的 `run_bodies()` —— **不另抄一份** ✗）：
  `#` 开头的注释行**不算**"这个 job 跑了它" ✓，**step 名（`name:`）也不算** ✓。
  * 注释那一半的实测理由：`wiring-lint` 的注释里有 `ux-queue.py`，**粗扫会把它当消费者** ✗
    （本脚本落地时正是先粗扫、再逐条核实才发现这个假阳性 ✓）。
  * ⭐ **step 名那一半是 2026-10-17（第 6 轮 `ng7`）补的**：收窄前的口径是
    「**整个 job 块的活行**」= `ng3` §3.5 ① 记的「**提及级**」⇒ 它**挡不住**
    "删 `run:` 正文、只留 step 名"的空转 ✗（那个 job 照样被算成"跑了它"）。
    **先量后改**（`ng7` 在真 `ci.yml` 上的读数）：R1 判到的 **8** 个 `(job, 脚本)` 对
    **全部**有 `run:` 正文命中 ⇒ 收窄**一条都不放走**（0 格 · 0 条红变绿 ✓）；
    全仓 25 个 job 块 × 41 个守卫里 **step 名提及 = 0 处**（注释提及 16 处，本来就不算 ✓）
    ⇒ **收窄零代价** ✓。⭐ 并且同一轮新增 **R1b** 把那个洞**从"绿"变成"判红"**：
    收窄前"删正文 + 有 `fetch-depth: 0`" ⇒ **绿**（空转 ✗），收窄后 ⇒ **红** ✓。
* **`fetch-depth` 只认 checkout 那一步里的活行**（`^\\s*fetch-depth:\\s*['\\"]?0`）——
  **注释里提一句不算** ✗。实测理由：本仓的注释里**到处**在讲
  "`fetch-depth: 0` 不是可选项"（`ci.yml` 里 4 处）⇒ 按整块文本搜会**永远绿**
  ⇒ 那是**咬不住的守卫** ✗。

## 如实划的边界（**不是豁免** ✗）

`HISTORY_DEPENDENT` 是**声明**，不是推导 —— **新加一个历史判据脚本要一起加进来** ✓；
R2 只防"表里点名了不存在的东西"，**防不了"漏了一个"** ✗（那要靠评审 ✓）。
⚠ 但**漏了的代价已经降下来了**：`check-guard-bite.py` 现在会在浅克隆里**印出
可操作的出路**（"CI 里给跑这一步的那个 job 加 `fetch-depth: 0`"）✓ —— 不再是
一行读不懂的 `rc=128` ✗。

## 退出码

  0 = 通过 / 1 = 判红 / 2 = 用法错 / 3 = 环境错（读不到 `ci.yml` / 取不到钉死的旧 rev
  ⇒ **不假装判过** ✗，照 `check-guard-bite.py` 的口径 ✓）

## 用法

  python3 scripts/check-history-depth-wiring.py                  # 判现役 ci.yml
  python3 scripts/check-history-depth-wiring.py --selftest       # 正负对照（含真旧 rev ✓）
  python3 scripts/check-history-depth-wiring.py --ci-yml-rev 1cd4aba0   # 判历史某一版（咬住证明 ✓）
"""
import argparse
import importlib.util
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DEFAULT_CI = ROOT / ".github" / "workflows" / "ci.yml"

# ── 声明：**跑它们就要全历史**（每条带理由 ✓ 评审可见 ✓）─────────────────
#
# ⚠ 每条的"实测读数"= 真 `git clone --depth 1` 夹具（2026-10-16）里跑出来的 ——
#   写进理由里是为了让"为什么它算历史判据"**可复核** ✓（判据**不核**这一列：
#   它是**证据**，不是断言 ⇒ 免得把读数变成第二个会腐烂的表 ✗）。
HISTORY_DEPENDENT: list[tuple[str, str]] = [
    ("scripts/check-guard-bite.py",
     "C1 覆盖面算 `git diff <coverage_since>..HEAD`（基线 = 台账抬头的 `# coverage_since:`）"
     "；浅克隆实测 rc=3"),
    ("scripts/check-attention-budget.py",
     "簿记窗口 = 固定锚点..HEAD（`git log`）；浅克隆实测 rc=3"),
    ("scripts/check-threshold-ratchet.py",
     "开轮纪律 = 基点之后的提交数（`git log`）；浅克隆实测 rc=1"),
    ("scripts/check-queue-views.py",
     "C4 祖先性断言（`git merge-base --is-ancestor`）；浅克隆实测 rc=1"),
    ("scripts/ux-queue.py",
     "收口指针的祖先性断言（`git merge-base --is-ancestor`）；浅克隆实测 rc=1"),
    ("scripts/check-queue-rewrite-trigger.py",
     "咬住证明打在**固定旧 rev** 上（`git show <rev>:…`）；浅克隆实测 rc=3"),
    # ⚠ **2026-10-17 补**（`ng1` 普查 §3.C 第 ③ 组：它的 **CI 面**缺 ⇒ 接进 `queue-lint` ✓）——
    # **如实划的边界** ✗：本条的**判据本体不需要全历史**（`--check` / `--selftest` 浅克隆
    # **实测 rc=0** ✓）；**要全历史的是它的咬住证明**（`docs/guard-bite.jsonl` 那一行的
    # `--check --source-rev 9c8cfa90` ⇒ `git show` ⇒ 浅克隆**实测 rc=3** ✗）。
    # ⇒ 与本表里 `check-queue-rewrite-trigger.py`、以及**本脚本自指**那两条**同形** ✓
    # （"跑它就要全历史" = **连它的咬住证明一起跑**就要全历史 ✓，不是"判据本体读历史" ✗）。
    # ⚠ 反过来说：**同族的 `scripts/audit-notation-paths.py` 故意不登记** ✗ —— 它全文
    # **0 处** `git`/`subprocess`，判据与 `--self-test` 浅克隆都 rc=0 ⇒ 登记它 = 给一条
    # 不需要全历史的脚本发"要全历史"的假红（E18：守卫一旦叫狼来了就死了 ✗）。
    ("scripts/check-frozen-headroom.py",
     "咬住证明打在**固定旧 rev** 上（`--check --source-rev 9c8cfa90` ⇒ `git show <rev>:<path>`）"
     "；浅克隆实测 rc=3（判据/`--selftest` 本身浅克隆 rc=0 —— 同 `check-queue-rewrite-trigger.py` 的形态 ✓）"),
    # ⚠ **本脚本自己也算**（如实 ✓）：`--selftest` 的咬住证明要
    # `git show 1cd4aba0:.github/workflows/ci.yml` ⇒ 浅克隆里那个对象不在 ⇒ rc=3。
    # 自指是**故意的**：R1 因此也会核"谁跑本脚本，谁就得有全历史" ✓。
    ("scripts/check-history-depth-wiring.py",
     "咬住证明打在**固定旧 rev** 上（`git show <rev>:.github/workflows/ci.yml`）"),
]

# ⭐ **钉死的旧 rev**（pr1 的坑：坏版本**不能写 `HEAD`** ✗ —— 提交后 `HEAD`
# 就是修好的那一版 ⇒ 咬住证明会**自己失效**）。这一版 = CI **真红**的那一跑的
# `head_sha`（run `38817419732`）✓ —— 是**已存在的坏输入**，不是本脚本新造的 ✓。
PINNED_BAD_REV = "1cd4aba0"

# `fetch-depth` 的**活行**（注释里提一句不算 ✗ —— 见 docstring 的口径 ✓）。
FETCH_DEPTH_RE = re.compile(r"^\s*fetch-depth:\s*['\"]?0['\"]?\s*$")
CHECKOUT_RE = re.compile(r"uses:\s*actions/checkout@")
STEP_RE = re.compile(r"^\s*- ")
USES_RE = re.compile(r"^\s*uses:")


class EnvError(Exception):
    """环境/用法错（exit 3）—— 与 `check-guard-bite.py` 同口径 ✓。"""


def _load_guard_bite():
    """复用 `check-guard-bite.py` 的 `job_blocks` / `_JOB_KEY_RE` / `shallow_hint`
    （**不另抄一份** ✗，照 `check-ci-gate-coverage.py` 复用 `ci-push-range.py` 的先例 ✓）。

    ⚠ "job 块怎么切"只有**一处定义** ✓ —— 那边 C4 的接线闸用的就是同一条规则；
    这里再抄一份 = 两处会漂移 ✗。
    """
    path = HERE / "check-guard-bite.py"
    spec = importlib.util.spec_from_file_location("check_guard_bite", path)
    if spec is None or spec.loader is None:  # pragma: no cover - 环境问题
        raise EnvError(f"加载不了 {path} ✗")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def live_lines(block: str) -> list[str]:
    """job 块里的**活行**（`#` 开头的注释**不算** —— 见 docstring 的口径 ✓）。"""
    return [ln for ln in block.splitlines() if not ln.strip().startswith("#")]


def checkout_fetch_depth(block: str) -> bool | None:
    """这个 job 的 checkout 步骤有没有 `fetch-depth: 0`？

    `True` / `False`；**`None` = 这个 job 根本没有 `actions/checkout`** ✓
    （调用方当判红处理 —— 没有 checkout 就更不可能有历史 ✗）。

    ⚠ 只看**那一步自己的**那一段（到下一个 step 或下一个 `uses:` 为止）——
    否则 `ledger` / `e2e-ledger` 的 `fetch-depth: 0` 会被算到别的 job 头上 ✗。
    """
    lines = live_lines(block)
    for i, ln in enumerate(lines):
        if not CHECKOUT_RE.search(ln):
            continue
        seg: list[str] = []
        for nxt in lines[i + 1:]:
            if STEP_RE.match(nxt) or USES_RE.match(nxt):
                break
            seg.append(nxt)
        return any(FETCH_DEPTH_RE.match(nxt) for nxt in seg)
    return None


def wiring_problems(ci_text: str, declared: list[tuple[str, str]]) -> tuple[list[str], list[str]]:
    """R1（接线）+ R1b（提及不算跑）+ R2（表不腐烂）+ R3（空转闸）⇒ `(红行, 判到的对)` ✓。"""
    gb = _load_guard_bite()
    blocks = gb.job_blocks(ci_text)
    red: list[str] = []
    checked: list[str] = []

    # ── R1 接线 + R1b 提及不算跑 ────────────────────────────────────────
    for job, block in sorted(blocks.items()):
        live = "\n".join(live_lines(block))
        # ⭐ **"真的跑"= `run:` 正文**（`run_bodies()` —— 唯一一处 ✓）：注释**与 step 名**
        #   都不算 ✗（第 6 轮 `ng7` 收窄；读数与理由见模块 docstring 的"刻意收窄"节 ✓）。
        bodies = gb.run_bodies(block)
        used = sorted(rel for rel, _why in declared if rel in bodies)
        # ⭐ **R1b**：活行里点名了、但 `run:` 正文里没有它 ⇒ 那一跑**不存在**（空转 ✗）。
        mentioned = sorted(rel for rel, _why in declared if rel in live and rel not in bodies)
        if mentioned:
            named_m = " · ".join(f"`{r}`" for r in mentioned)
            red.append(f"**R1b 提及不算跑** ✗：`{job}` 的**活行**里点名了 {named_m}，"
                       f"而它的 **`run:` 正文里没有**它们 ⇒ 那些步**只剩名字**"
                       f"（`run:` 正文被删/改成别的命令 ✗）⇒ 那几条历史判据在 CI 上"
                       f"**根本不跑**（空转 ✗）。出路二选一：①在 `run:` 正文里真跑它 ✓；"
                       f"②把那个提及**挪进注释**（注释不算 ✓）或删掉 ✓")
        if not used:
            continue
        fd = checkout_fetch_depth(block)
        for rel in used:
            checked.append(f"{job}:{rel}")
        named = " · ".join(f"`{r}`" for r in used)
        if fd is None:
            red.append(f"**R1 接线** ✗：`{job}` 真的跑了历史判据 {named}，"
                       f"而它**没有 `actions/checkout`** ⇒ 历史拿不到 ⇒ 那些判据会以"
                       f"『环境不在』（`exit 3`）判红 ✗")
        elif not fd:
            red.append(f"**R1 接线** ✗：`{job}` 真的跑了历史判据 {named}，"
                       f"而它的 `actions/checkout` **没有 `fetch-depth: 0`** ⇒ 浅克隆 ⇒ "
                       f"基线那个对象不在 ⇒ `git` **rc=128** ⇒ 判据 **exit 3**"
                       f"（红的是**前提**、不是『咬不住』✗）⇒ 给这个 job 的 checkout 加 "
                       f"`fetch-depth: 0` ✓")

    # ── R2 表不腐烂 ─────────────────────────────────────────────────────
    for rel, _why in declared:
        if not (ROOT / rel).exists():
            red.append(f"**R2 表不腐烂** ✗：`HISTORY_DEPENDENT` 点名了 `{rel}`，"
                       f"而**仓库里没有它** ⇒ 表在腐烂 ✗（改那张表或把脚本加回来 ✓）")

    # ── R3 空转闸 ───────────────────────────────────────────────────────
    if not declared:
        red.append("**R3 空转** ✗：`HISTORY_DEPENDENT` 是**空的** ⇒ 这一跑**什么都没判**"
                   "（空转 ✗ —— 判据的输入必须真的存在 ✓）")
    elif not checked:
        red.append("**R3 空转** ✗：**没有任何** `(job, 脚本)` 对被判到 ⇒ "
                   "这一跑**什么都没查**（`ci.yml` 结构变了 / 声明与它脱节了 ✗）"
                   "⇒ 不许当通过 ✗")
    return red, checked


def read_ci_at_rev(rev: str) -> str:
    """`git show <rev>:.github/workflows/ci.yml` ⇒ 咬住证明打在**固定旧 rev** 上 ✓。"""
    out = subprocess.run(["git", "show", f"{rev}:.github/workflows/ci.yml"],
                         cwd=ROOT, capture_output=True, text=True,
                         encoding="utf-8", errors="replace",
                         env=git_env.clean_git_env())
    if out.returncode != 0:
        gb = _load_guard_bite()
        hint = gb.shallow_hint(ROOT)
        msg = (f"取不到钉死的旧 rev `{rev}`（`git show` rc={out.returncode}："
               f"{out.stderr.strip()[:200]}）⇒ **不假装判过** ✗")
        raise EnvError(msg + (f"\n       {hint}" if hint else ""))
    return out.stdout


def run_check(ci_text: str, declared: list[tuple[str, str]], label: str,
              quiet: bool = False) -> int:
    red, checked = wiring_problems(ci_text, declared)
    if not quiet:
        print(f"历史深度接线（{label}）：判到 {len(checked)} 个 `(job, 脚本)` 对 · "
              f"声明 {len(declared)} 条")
        for pair in checked:
            print(f"    ✓ {pair}")
    if red:
        print(f"\n✗ 历史深度接线 **{len(red)} 条红**：", file=sys.stderr)
        for m in red:
            print(f"  · {m}", file=sys.stderr)
        return 1
    if not quiet:
        print("✓ 历史深度接线：跑历史判据的 job 都拿到了全历史 ✓ · "
              "接线**不靠提及**（`run:` 正文里真的跑 ✓）")
    return 0


# ── selftest：正负对照（**咬得住历史 bug 的守卫才算数** ✓）─────────────────

def _strip_fetch_depth(text: str, job: str) -> str:
    """把 `job` 那个块里的 `fetch-depth: 0` **活行**删掉（负对照 ✓）。"""
    gb = _load_guard_bite()
    out: list[str] = []
    cur: str | None = None
    for ln in text.splitlines():
        m = gb._JOB_KEY_RE.match(ln)
        if m:
            cur = m.group(1)
        if cur == job and not ln.strip().startswith("#") and FETCH_DEPTH_RE.match(ln):
            continue
        out.append(ln)
    return "\n".join(out)


SYNTHETIC_JOB = """
  synthetic-shallow-history-job:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v5
      - run: python3 scripts/check-guard-bite.py --check --all
"""

# ⭐ **第 6 轮 `ng7` 的负对照夹具**：**删 `run:` 正文、只留 step 名** ✗ ——
# ⚠ 关键是它**带着** `fetch-depth: 0`：收窄前的提及级 R1 会把这个 job 算成
# "跑了 check-guard-bite.py" ⇒ 有全历史 ⇒ **绿**（空转 ✗）；R1b 必须把它判红 ✓。
SYNTHETIC_MENTION_ONLY_JOB = """
  synthetic-mention-only-job:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0
      - name: 跑历史判据 scripts/check-guard-bite.py（`run:` 正文被删掉了 ✗）
        run: echo hi
"""

# 正对照（**不许把真跑也判成提及** ✗）：同名 step 名 + `run:` 正文里**真的**跑它 ✓。
SYNTHETIC_REAL_RUN_JOB = """
  synthetic-real-run-job:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0
      - name: 跑历史判据（真跑 ✓）
        run: python3 scripts/check-guard-bite.py --check --all
"""


def selftest() -> int:
    """① 正对照（现役 `ci.yml` 必须绿）+ ② 真旧 rev + ③④⑤ 负对照 ⇒ 逐条印读数 ✓。"""
    gates: list[tuple[str, bool, str]] = []

    def gate(name: str, ok: bool, reading: str) -> None:
        gates.append((name, ok, reading))

    live = DEFAULT_CI.read_text(encoding="utf-8")
    base_red, base_checked = wiring_problems(live, HISTORY_DEPENDENT)
    gate("① 正对照：现役 `ci.yml` ⇒ **绿**（不许制造假红 ✗）",
         not base_red, f"红 {len(base_red)} 条 · 判到 {len(base_checked)} 对")
    gate("①b 正对照真的**判到了东西**（不是空转 ✓）",
         len(base_checked) >= 1, f"{len(base_checked)} 对：{base_checked}")

    # ② **咬住证明**：CI 真红的那一版（钉死旧 rev ✓，**不是** `HEAD` ✗）。
    old = read_ci_at_rev(PINNED_BAD_REV)
    old_red, _old_checked = wiring_problems(old, HISTORY_DEPENDENT)
    bite_named = any("editor" in m and "check-guard-bite.py" in m for m in old_red)
    gate(f"② 咬住证明：钉死旧 rev `{PINNED_BAD_REV}`（CI 真红的 `head_sha`）⇒ **判红**"
         f" + 点名 `editor` 与 `check-guard-bite.py` ✓",
         bool(old_red) and bite_named, f"红 {len(old_red)} 条 · 点名 editor={bite_named}")

    # ③ 负对照：撤掉 `queue-lint` 的 `fetch-depth: 0` ⇒ 必须红。
    q = _strip_fetch_depth(live, "queue-lint")
    q_red, _ = wiring_problems(q, HISTORY_DEPENDENT)
    gate("③ 负对照：撤掉 `queue-lint` 的 `fetch-depth: 0` ⇒ **判红** + 点名 queue-lint ✓",
         any("queue-lint" in m for m in q_red), f"红 {len(q_red)} 条")

    # ④ 负对照：新加一个"跑历史判据却没全历史"的 job ⇒ 必须红。
    s_red, _ = wiring_problems(live + SYNTHETIC_JOB, HISTORY_DEPENDENT)
    gate("④ 负对照：新加一个跑历史判据、却没有 `fetch-depth: 0` 的 job ⇒ **判红** ✓",
         any("synthetic-shallow-history-job" in m for m in s_red), f"红 {len(s_red)} 条")

    # ⑤ 防腐烂：表里点名一个不存在的脚本 ⇒ 必须红。
    ghost = HISTORY_DEPENDENT + [("scripts/no-such-guard.py", "合成的腐烂条目")]
    g_red, _ = wiring_problems(live, ghost)
    gate("⑤ 防腐烂：`HISTORY_DEPENDENT` 点名一个**不存在**的脚本 ⇒ **判红** ✓",
         any("no-such-guard.py" in m for m in g_red), f"红 {len(g_red)} 条")

    # ⑥ 空转闸：声明清空 ⇒ 必须红（不许"什么都没判"当通过 ✗）。
    e_red, _ = wiring_problems(live, [])
    gate("⑥ 空转闸：`HISTORY_DEPENDENT` 清空 ⇒ **判红**（空转不算通过 ✗）",
         bool(e_red), f"红 {len(e_red)} 条")

    # ⭐⭐ **第 6 轮 `ng7` 的反向验证**：**删 `run:` 正文、只留 step 名** ⇒ 必须判红 ✓
    #   （⚠ 夹具**带** `fetch-depth: 0` ⇒ 它**不可能**靠 R1 判红 ⇒ 红只能来自 R1b ✓
    #     —— 收窄前这一条是**绿**的（提及级把它算成"跑了" ✗））
    m_red, m_checked = wiring_problems(live + SYNTHETIC_MENTION_ONLY_JOB, HISTORY_DEPENDENT)
    m_named = any("R1b" in m and "synthetic-mention-only-job" in m for m in m_red)
    gate("⑦ 反向（第 6 轮）：**删 `run:` 正文、只留 step 名**（且 `fetch-depth: 0` 在 ✓）"
         "⇒ **判红** + 点名 R1b ✓",
         m_named, f"红 {len(m_red)} 条 · 点名 R1b={m_named} · 判到 {len(m_checked)} 对")

    # ⭐ 正对照（**不许把真跑也判成提及** ✗）：step 名 + `run:` 正文里真的跑它 ⇒ 不许红 ✓。
    r_red, r_checked = wiring_problems(live + SYNTHETIC_REAL_RUN_JOB, HISTORY_DEPENDENT)
    gate("⑧ 正对照：step 名点名了、`run:` 正文里**也真的跑**它 ⇒ **不许红**（R1b 不误伤 ✓）",
         not r_red and f"synthetic-real-run-job:scripts/check-guard-bite.py" in r_checked,
         f"红 {len(r_red)} 条 · 判到 {len(r_checked)} 对")

    print("── 历史深度接线 · selftest（正负对照）──")
    for name, ok, reading in gates:
        print(f"  {'✓' if ok else '✗'} {name} ⇒ {reading}")
    bad = [n for n, ok, _ in gates if not ok]
    if bad:
        print(f"\n✗ selftest **{len(bad)} 条红**：{bad}", file=sys.stderr)
        return 1
    print(f"\n✓ selftest：**{len(gates)} 条臂全绿** —— 现役绿 ✓ · 真旧 rev 咬得住 ✓ · "
          f"负对照/防腐烂/空转闸逐条红 ✓")
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="跑历史判据的 job 必须有全历史（类守卫）")
    ap.add_argument("--ci-yml", default=str(DEFAULT_CI), help="判哪一份 ci.yml（默认现役 ✓）")
    ap.add_argument("--ci-yml-rev", default=None,
                    help="改判 git 里**某一版** ci.yml（咬住证明打在钉死的旧 rev 上 ✓）")
    ap.add_argument("--selftest", action="store_true", help="正负对照（含真旧 rev ✓）")
    args = ap.parse_args(argv)

    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, ValueError, OSError):
            pass

    if args.selftest:
        try:
            return selftest()
        except EnvError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 3

    try:
        if args.ci_yml_rev:
            text, label = read_ci_at_rev(args.ci_yml_rev), f"git rev `{args.ci_yml_rev}`"
        else:
            path = Path(args.ci_yml)
            if not path.exists():
                raise EnvError(f"`{path}` 不在 ⇒ 判不了接线（**不假装判过** ✗）")
            text, label = path.read_text(encoding="utf-8"), path.name
        return run_check(text, HISTORY_DEPENDENT, label)
    except EnvError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
