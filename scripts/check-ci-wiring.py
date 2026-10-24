#!/usr/bin/env python3
"""**「CI 的某条前提在不在」的类守卫** —— **一本登记表 + 一个驱动器**（两层）。

## 这一条是什么（`SUB-narrow-guards-to-one-layer` ③ 的第一步）

条目原文（`docs/ux-queue-events.jsonl:853` 的 `submit` 事件）把 B 组里五条判据归成**一条关注点**：

> 【③ CI 接线守卫】B 组里 5 → **1**
>    五个都是同一条判据：「**CI 的某条前提在不在**」
>    · ci-gate-coverage：改了某范围 ⇒ 该跑的门禁跑没跑
>    · history-depth-wiring：跑历史判据的 job 有全历史吗
>    · test-env-isolation：测试的环境档位隔离了吗
>    · one-run：一次只让一条 run 活着吗
>    · multi-session：两条只有 main 出得起的红线
>    ⇒ 合成一个 `check-ci-wiring.py`：**登记表（前提名 → 断言 → 落在哪个 job） + 一个驱动器** ✓
>    ⇒ 判据：一条命令跑完所有"CI 前提" ✓

本轮（第 5 轮）并进来**两条**（五条里的前两条），剩下的三条留在各自的文件里、
**登记表已经为它们留好格子**（加一层 = 加一段 + 登记一行 ✓）。

| 层 | 原来是谁 | 判什么 |
|---|---|---|
| ① `coverage` | `scripts/check-ci-gate-coverage.py` | **job 会读到的东西，必须在"决定它跑不跑"的那个范围里**（洞 `PATHS-FILTER-CLASS`） |
| ② `history` | `scripts/check-history-depth-wiring.py` | **判据要读历史，就必须把历史给它**（洞 `EDITOR-JOB-BITE-STEP-EXIT-3`） |

两层的形状**逐条同族**：都读 `.github/workflows/ci.yml`、都是"job 做了 X ⇒ 前提 Y 必须在"、
都带 R1/R2/R3 三条类不变量、都在 `ci.yml` + `scripts/ci-local.sh` 两处接线、
都声明 `class=B` / 同一个锚 `A-RELEASE-NOT-BLOCKED`（**仓库自己的分类** ✓）——
⇒ 同一关注点、同一批接线点 ✓（量的读数见 `.cache/ux/fix-gs5/VERDICT.md` §2）。

## 层① `coverage`（原文逐字保留 ✓ —— 只加了断言 id 与登记表）

**「按改动范围决定跑不跑」的类守卫** —— 洞 `PATHS-FILTER-CLASS`（2026-10-15）。

### 为什么有这个层（"修实例不修类"的解药）

`docs/CI-FAILURES.md` 记着**同族四次**事故，每次都是同一句话：

| # | 实例 | 形态 |
|---|------|------|
| ① | `scripts/**`（2026-09-25） | 改了 `scripts/gap.py` ⇒ 三个标志全 false ⇒ `gates`/`ledger`/`contract` 全 **skipped** ⇒ **刚改的判据没人验** ✗ |
| ② | `.github/workflows/**`（2026-09-25 round 161） | 只改工作流 ⇒ `ledger`/`gates`/`test` 全 skipped ⇒ **刚改的那条 job 永远得不到验证** ✗ |
| ③ | `docs/gaps/**`（2026-10-11，G-110） | 只改复现件 ⇒ `ledger` skipped ⇒ **刚修好的复现件永远得不到验证** ✗ |
| ④ | **push 范围**（2026-10-15，`PUSH-RANGE-HIDES-RELEASE`） | 纯 docs 推送按**自己**的 `before..sha` 算 ⇒ 把**还没被验过的祖先**藏掉 ⇒ `v0.91.0` 发不出去 ✗ |

前三次的修法都是**手工往 `filters:` 里加一行**（见 `ci.yml` 的注释）——
⇒ 第四次换个触发源就再犯一次 ✗。**这就是"修实例不修类"**：
每次都在修**那个**漏掉的路径，没人把"**范围算法本身**"变成可查的判据。

### 类不变量（三条，都能机械判）

* **R1 门控覆盖**：凡 `if:` 里挂 `needs.changes.outputs.<flag>` 的 job，
  它 **step 真正消费的每一条仓库路径**都必须被**门控它的那个过滤器**命中 ✓。
  —— 反过来说：**job 会读到的东西，必须在"决定它跑不跑"的那个范围里** ✗。
  历史①②③ 全是这条的违反（`scripts/**` / `.github/workflows/**` / `docs/gaps/**`
  被某个 job 消费，却不在门控它的过滤器里）。
* **R2 标志↔组对应**：每个出现在 `if:` 里的 `changes` 标志都要在 `filters:` 里有
  对应组 ✓（写错组名 ⇒ 那个标志恒 false ⇒ job 恒 skipped ✗，而 YAML 不会报错 ✗）。
* **R3 数据依赖（声明式）**：判据**读进来当数据**的路径不一定出现在 `run:` 文本里
  —— 实测 `ledger` 跑 `python3 scripts/gap.py check`，而 `docs/gaps/ledger.jsonl`
  是那个脚本**内部**读的 ⇒ 纯文本抽取**咬不住实例③** ✗。这类依赖**声明**在
  [`DATA_DEPENDENCIES`] 里（每条带理由 ✓、评审可见 ✓），由本层核对覆盖 ✓；
  表里点名的 job 不存在或不再被门控 ⇒ **判红**（防表自己腐烂 ✗）。

⚠ 三处的**语义**都按 **job 级析取**判（`if: A || B` ⇒ A、B 任一命中即可 ✓）——
要求"每个组都要有"会制造假红 ✗（实测：`.github/workflows/**` 只在 `rust` 组里，
而每个门控 job 的 `if` 都含 `rust` ⇒ 已经够了 ✓）。

### 顶层条目：**按实际仓库枚举**（洞 `TOP-LEVEL-TABLE-GOES-BLIND`，2026-10-16 ✗→✓）

R1 要先回答"这个 token 像不像仓库路径"（不然 `python3` / `--json` 也会被当路径 ✗）
—— 那一步的输入是**仓库顶层条目**。修前它是一张**硬编码的 28 条名表**，而仓里实测
**29** 个一级条目（缺 `.gitignore`）⇒ **新加一个顶层目录 ⇒ 它下面的路径认不出来 ⇒
R1 静默变瞎** ✗（"静默"是要害：判据照样印"违反 0 条"，没有任何读数告诉你它瞎了 ✗）。
⇒ 现在**枚举**（`discover_top_level()`，`git ls-files` 的一级名 ✓）⇒ 新加一级条目
**自动**进面、没有第二张表要维护 ✓；枚举坏了（不是 git 检出 / 根指错 / 地板不齐）
⇒ **exit 2 不假装判过** ✓（详见 `TOP_LEVEL_FLOOR` 那段注释的实测读数 ✓）。

## 层② `history`（原文逐字保留 ✓ —— 只加了断言 id 与登记表）

**「跑历史判据的 job 必须有全历史」的类守卫**（洞 `EDITOR-JOB-BITE-STEP-EXIT-3`，2026-10-16）。

同族**两次**事故，每次都是同一句话：

| # | 实例 | 形态 |
|---|------|------|
| ① | `queue-lint`（2026-10-15） | `ux-queue.py --check` 的祖先性断言在浅克隆里**判不了** ⇒ 假红 270 条 ⇒ **手加** `ci.yml` 的 `fetch-depth: 0` ✗ |
| ② | `editor`（2026-10-16 · run `38817419732` · step 11） | `check-guard-bite.py` 的 **C1 覆盖面**要算 `git diff <coverage_since>..HEAD`（基线 = `docs/guard-bite.jsonl` 抬头的 `# coverage_since:`）⇒ 浅克隆里**那个对象不在** ⇒ `git` **rc=128** ⇒ 判据 **exit 3** ⇒ **job 红**，而红的是**前提**、不是"咬不住" ✗ |

两次的修法都是**手工往某一个 job 的 checkout 加一行** ⇒ **这就是"修实例不修类"** ✗。

⚠ **同类第 ② 次是 CI 实测**（不是推理）：**同一个 run 里** `queue-lint`（**有**
`fetch-depth: 0`）跑**同一条** `check-guard-bite.py` 命令是 **success** ✓ ——
⇒ 唯一的差别就是**历史深度** ✓。本机用真 `git clone --depth 1` 夹具**逐字**复现了
CI 的那两行（`fatal: ambiguous argument 'cf259d1f..HEAD'` + `rc=3`）✓。

### 类不变量（四条，都能机械判）

* **R1 接线**：凡 job **真的跑**声明过的历史判据脚本（`HISTORY_DEPENDENT`），
  该 job 的 `actions/checkout` 就必须带 `fetch-depth: 0` ✓。
  —— 反过来说：**判据要读历史，就必须把历史给它** ✗。
* **R1b 提及不算跑**（2026-10-17 · 第 6 轮 `ng7` 加）：**非注释活行里点名了**声明过的脚本、
  而它**任何一条 `run:` 正文里都没有** ⇒ **判红**（那一跑**根本不存在** = 空转 ✗）。
  —— 反过来说：**step 名不是命令** ✗（`check-guard-bite.py` 的 `run_bodies()` docstring
  里早就写着这一句 ✓ —— 本层落地时只收了"注释不算"、**漏了 step 名** ✗）。
* **R2 表不腐烂**：`HISTORY_DEPENDENT` 点名的脚本必须**在仓库里存在** ⇒ 否则判红
  （表里留着一个已经不存在的脚本 = 表在腐烂 ✗）。
* **R3 空转闸**：这一跑至少要判到**一个** `(job, 脚本)` 对 ⇒ 否则判红 ——
  判据自己扫空了（正则坏了 / `ci.yml` 改了结构 / `HISTORY_DEPENDENT` 被清空）
  **不许当通过** ✗（"咬不住的守卫等于没有" ✓）。

### 两处刻意收窄的口径（都是**如实划的**，不是放宽 ✗）

* **只认 `run:` 正文**（复用 `check-guard-bite.py` 的 `run_bodies()` —— **不另抄一份** ✗）：
  `#` 开头的注释行**不算**"这个 job 跑了它" ✓，**step 名（`name:`）也不算** ✓。
  * 注释那一半的实测理由：`wiring-lint` 的注释里有 `ux-queue.py`，**粗扫会把它当消费者** ✗。
  * ⭐ **step 名那一半是 2026-10-17（第 6 轮 `ng7`）补的**：收窄前的口径是
    「**整个 job 块的活行**」⇒ 它**挡不住**"删 `run:` 正文、只留 step 名"的空转 ✗。
    **先量后改**（`ng7` 在真 `ci.yml` 上的读数）：R1 判到的 **8** 个 `(job, 脚本)` 对
    **全部**有 `run:` 正文命中 ⇒ 收窄**一条都不放走**（0 格 · 0 条红变绿 ✓）；
    全仓 25 个 job 块 × 41 个守卫里 **step 名提及 = 0 处**（注释提及 16 处，本来就不算 ✓）
    ⇒ **收窄零代价** ✓。⭐ 并且同一轮新增 **R1b** 把那个洞**从"绿"变成"判红"** ✓。
* **`fetch-depth` 只认 checkout 那一步里的活行**（`^\\s*fetch-depth:\\s*['\\"]?0`）——
  **注释里提一句不算** ✗。实测理由：本仓的注释里**到处**在讲
  "`fetch-depth: 0` 不是可选项"（`ci.yml` 里 4 处）⇒ 按整块文本搜会**永远绿**
  ⇒ 那是**咬不住的守卫** ✗。

### 如实划的边界（**不是豁免** ✗）

`HISTORY_DEPENDENT` 是**声明**，不是推导 —— **新加一个历史判据脚本要一起加进来** ✓；
R2 只防"表里点名了不存在的东西"，**防不了"漏了一个"** ✗（那要靠评审 ✓）。
⚠ 但**漏了的代价已经降下来了**：`check-guard-bite.py` 现在会在浅克隆里**印出
可操作的出路**（"CI 里给跑这一步的那个 job 加 `fetch-depth: 0`"）✓。

## 驱动器（一本登记表 + 一个驱动器 ✓）

```
python3 scripts/check-ci-wiring.py                       # 两层都跑（默认 ✓）
python3 scripts/check-ci-wiring.py --layer coverage       # 只跑层①（`ci.yml` 的 wiring-lint ✓）
python3 scripts/check-ci-wiring.py --layer history        # 只跑层②（`ci.yml` 的 queue-lint ✓）
python3 scripts/check-ci-wiring.py --json                 # 机器可读（agent 用 ✓）
python3 scripts/check-ci-wiring.py --selftest             # 登记表对账 + 绿跑对账 + 两层的正负对照
python3 scripts/check-ci-wiring.py --ci-yml-rev 1cd4aba0  # 层② 的咬住证明（钉死的旧 rev ✓）
```

⚠ **`--root <dir>` 是层① 的夹具缝**（`--selftest` 的 ⑥ 与咬住证明用它指合成仓）：
顶层条目与默认 `--ci` 都从它取。**不是逃生门** ✗ —— `scripts/ci-local.sh` 与 `ci.yml`
两条接线**逐字不传它** ⇒ 判的还是本仓 ✓。

## 退出码（两层口径**逐字保留** ✓）

  0 = 通过 · 1 = 判红 · 2 = 用法或环境错（层① 口径：枚举不了顶层条目 / 读不了 `ci.yml`）·
  3 = 前提不在（层② 口径：取不到钉死的旧 rev ⇒ **不假装判过** ✗，照 `check-guard-bite.py` ✓）

⚠ 两层的退出码**不是**新定的：层① 原来是 `0/1/2`、层② 原来是 `0/1/2/3`
（`scripts/ci-local.sh` 的注释逐条记着"三条都必须拦" ✓）⇒ 合并**一个都没动** ✗。
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DEFAULT_CI_REL = Path(".github") / "workflows" / "ci.yml"
DEFAULT_CI = ROOT / DEFAULT_CI_REL

sys.path.insert(0, str(HERE))  # 同目录库，照 `threshold_ratchet.py` 的先例 ✓
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）


class EnvError(Exception):
    """环境/用法错 —— **两层口径不同**（层① `2` · 层② `3` ✓）。

    ⚠ 合并**不许**把两层的退出码并成一个 ✗：`scripts/ci-local.sh` 与
    `docs/guard-bite.jsonl` 都逐条钉着它们（"三条都必须拦" ✓）。
    """

    def __init__(self, msg: str, code: int = 2):
        super().__init__(msg)
        self.code = code


# ═══════════════════════════════════════════════════════════════════════════
# 层① `coverage` —— 原文 `scripts/check-ci-gate-coverage.py`（逐字保留 ✓）
# ═══════════════════════════════════════════════════════════════════════════

# ── **仓库顶层条目**：判"这个 token 像不像仓库路径"用它 ✓ ────────────────────
#
# ⭐ **按实际仓库枚举**（洞 `TOP-LEVEL-TABLE-GOES-BLIND`，2026-10-16 ✗→✓）。
# 修前这里是一张**硬编码的 28 条名表**，而仓里实测 **29** 个一级条目（缺 `.gitignore`）
# ⇒ **新加一个顶层目录 ⇒ 它下面的路径 token 认不出来 ⇒ R1 静默变瞎** ✗（"静默"是要害：
# 判据照样印"违反 0 条"，没有任何读数告诉你它瞎了 ✗）⇒ 现在从 `git ls-files` 的一级名
# **枚举**（`discover_top_level()`）⇒ 新加一级条目**自动**进面 ✓、没有第二张表要维护 ✓。
#
# ⚠ **为什么是 `git ls-files`（已跟踪）而不是"扫目录"或 `--others`**（**实测** ✓，
# 别改成后者 ✗）：`.ci-verified/` 是 **CI 自己写出来的未跟踪目录**（`mark-verified` 的
# `printf … > .ci-verified/sha`），它**不在** `.gitignore` 里（实测
# `git check-ignore .ci-verified/sha` ⇒ rc=1）⇒ 一旦把未跟踪条目算进面，实测**当场
# 2 条假红**（`R1 mark-verified 消费 .ci-verified[/sha]` ✗）—— 而那是 CI **写出去的
# 产物**、不是仓库路径 ⇒ 假红会让这条守卫变成"叫狼来了"，而 `ci.yml` 的 `E18` 教训
# 正是：守卫一旦叫狼来了，正确的反应就变成"加豁免" ⇒ 它就死了 ✗。
# ⚠ **代价如实标注** ✗：一个**还没 `git add`** 的新顶层目录看不见 ⇒ 本判据量的是
# "**仓库里**的路径"（未跟踪的东西按定义还不在仓库里 ✓）；`--selftest` 的 ⑥ 就是拿
# "新加顶层目录"当夹具、把这条口径钉住 ✓。
#
# ⚠ **枚举的地板**（`TOP_LEVEL_FLOOR`）：这几个一级条目**必须**在枚举结果里 —— 少一个
# 就说明**枚举坏了**（不是 git 检出 / 根指错 / 命令被改坏）⇒ **判环境错（exit 2）**，
# 绝不退化成"什么都认不出" ⇒ 那正是本洞的形态（**静默变瞎** ✗）✓。
TOP_LEVEL_FLOOR = (".github", "Cargo.toml", "courses", "crates", "docs", "scripts")

# `if:` 里出现的 `needs.changes.outputs.<flag>`。
FLAG_RE = re.compile(r"needs\.changes\.outputs\.([A-Za-z_][\w-]*)")
# step 文本里的候选路径 token（不含空格、含 `/` 或以已知顶层名开头）。
TOKEN_RE = re.compile(r"[A-Za-z0-9_./-]+")

# ── R3 **数据依赖**（声明式，每条带理由 ✓）────────────────────────────────
#
# 为什么需要它：判据**读进来当数据**的路径不一定出现在 `run:` 文本里。
# 实测：`ledger` 跑 `python3 scripts/gap.py check`，而 `docs/gaps/ledger.jsonl`
# 是那个脚本**内部**读的 ⇒ `run:` 里根本没有 `docs/gaps` 字样 ✗ ⇒ 纯文本抽取
# **咬不住实例③**（撤掉 `docs/gaps/**` 过滤器时不会判红 ✗ —— 而"咬不住的守卫
# 等于没有"，见 `AGENTS.md` 验证设计纪律 3 ✓）。
#
# ⚠ 这张表是**声明**，不是推导 —— 加 job / 加数据源时要一起改 ✓；
# `--selftest` 的负对照 ④ 就是靠它才咬得住实例③ ✓；表里点名的 job 不存在或
# 不再被门控 ⇒ **判红**（防这张表自己腐烂 ✗）。
DATA_DEPENDENCIES = [
    (
        "ledger",
        "docs/gaps/ledger.jsonl",
        "`gap.py check` 内部读台账本体与复现件（实例③：只改复现件时 `ledger` 被 skip ✗）",
    ),
]


def _load_push_range():
    """复用 `ci-push-range.py` 的 `load_filters` / `glob_to_regex`（**不另抄一份** ✗）。"""
    path = HERE / "ci-push-range.py"
    spec = importlib.util.spec_from_file_location("ci_push_range", path)
    if spec is None or spec.loader is None:  # pragma: no cover - 环境问题
        raise EnvError(f"加载不了 {path} ✗")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_ci(path):
    from _yaml import safe_load  # 仓库内 vendored PyYAML（无外部依赖）

    try:
        data = safe_load(Path(path).read_text(encoding="utf-8"))
        return data or {}
    except Exception as exc:  # noqa: BLE001
        raise EnvError(f"读不了 {path}：{exc}")


def gated_flags(job):
    """job 的 `if:` 里挂了哪些 `changes` 标志（**析取**语义：任一为真就跑 ✓）。"""
    return set(FLAG_RE.findall(str(job.get("if", ""))))


def job_text(job):
    """job 里"**会读到仓库文件**"的那些文本。

    ⚠ 三条**收窄**（每一条都是为了让判据**不制造假红** ✗ —— 守卫一旦叫狼来了，
    正确的反应就变成"加豁免"，那它就死了 ✓，教训见 `ci.yml` 的 `E18` 注释）：

    * 只看 `run:` —— `with:` 里的 `path:` 是 **artifact/cache 的输出路径**（写出，不是读入 ✗）；
    * **去掉注释行** —— `run:` 块里的注释经常**描述**产物（实测 `docs/e2e/` 就只出现在注释里 ✗）；
    * **去掉 `git add` / `git rm`** —— 那是**提交产物**，不是消费 ✓。
    """
    lines = []
    for step in job.get("steps") or []:
        if not isinstance(step, dict) or not step.get("run"):
            continue
        for line in str(step["run"]).splitlines():
            stripped = line.strip()
            if stripped.startswith("#"):
                continue
            if stripped.startswith(("git add", "git rm", "git checkout", "git commit")):
                continue
            lines.append(line)
    return "\n".join(lines)


def discover_top_level(root: Path) -> tuple[str, ...]:
    """**按实际仓库枚举**顶层条目名（`git ls-files` 的一级名 ✓）。

    ⚠ **三条不许静默**（静默返回一张**更窄**的表 = 本洞的形态本身 ✗）：
    git 跑不起来 / 枚举出 0 条 / `TOP_LEVEL_FLOOR` 少一个 ⇒ 一律 `EnvError`（exit 2）
    —— **不假装判过** ✓（与 `check-guard-bite.py` 拿不到 `git diff` 时 `exit 3` 同口径 ✓）。

    ⚠ `env=git_env.clean_git_env()`（洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`）：
    `GIT_DIR` **优先于 cwd** 决定仓库发现 ⇒ 不清就会从链接工作树的 hook 里枚举到
    **调用者的**仓 ✗（`scripts/githooks/pre-push` → `ci-local.sh` 正是这条路 ✓）。
    """
    proc = subprocess.run(["git", "ls-files", "--cached", "-z"], cwd=root,
                          capture_output=True, text=True, errors="replace",
                          env=git_env.clean_git_env())
    if proc.returncode != 0:
        raise EnvError(
            f"枚举不了顶层条目：`git ls-files` 在 `{root}` 上 rc={proc.returncode}"
            f"（{proc.stderr.strip()[:200]}）⇒ 这不是一个 git 检出 ⇒ **不假装判过** ✗"
        )
    names = {p.split("/", 1)[0] for p in proc.stdout.split("\0") if p}
    if not names:
        raise EnvError(
            f"枚举不了顶层条目：`git ls-files` 在 `{root}` 上返回 **0 条** ⇒ 空表意味着"
            f"**什么 token 都认不出来** = 本洞的形态 ✗ ⇒ **不假装判过** ✗"
        )
    missing = [n for n in TOP_LEVEL_FLOOR if n not in names]
    if missing:
        raise EnvError(
            f"顶层条目枚举**退化了**：地板 {missing} 不在结果里（枚举到 {len(names)} 条："
            f"{sorted(names)}）⇒ 这不是「仓库里没有它」、是**枚举坏了** ⇒ **不假装判过** ✗"
            f"（要么修枚举、要么改地板 —— 别删判据 ✗）"
        )
    return tuple(sorted(names))


def looks_like_repo_path(token, top_level):
    """像不像**仓库相对路径**：顶层名打头（含 `/` 或以已知顶层文件打头）。

    `top_level` = `discover_top_level()` 的结果（**每次跑现算** ⇒ 新加一级条目自动进面 ✓）。
    """
    if token.startswith("-") or "://" in token:
        return False
    head = token.split("/")[0]
    if head not in top_level:
        return False
    # 纯文件名（无 `/`）只认已知的顶层文件；顶层**目录**要带 `/` 才算路径。
    if "/" not in token and "." in head and head in top_level:
        return True
    return "/" in token


def consumed_paths(job, top_level):
    """job 消费的**仓库路径**（去重、排序；只保留像路径的 token ✓）。"""
    out = set()
    for token in TOKEN_RE.findall(job_text(job)):
        token = token.rstrip(".,;:'\"")
        if looks_like_repo_path(token, top_level):
            out.add(token)
    return sorted(out)


def classify(token, filters, glob_to_regex):
    """这条路径落在哪些过滤器组里（**唯一一份**过滤器实现来自 `ci.yml` ✓）。"""
    hits = set()
    for name, patterns in filters.items():
        for pattern in patterns:
            if glob_to_regex(pattern).match(token):
                hits.add(name)
                break
    return hits


def coverage_problems(doc, filters, glob_to_regex, top_level):
    """层① 的 R1 + R2 + R3 ⇒ `([(断言 id, ok, 说明)], 产出的断言 id 集合)`。

    ⚠ **读数与合并前逐字相同** ✗：`res` 里放的就是原文 `res.append(...)` 的那些条目
    （绿路的**逐 token** 那一档原文**不记** ⇒ 这里也不记 ⇒ `报告` 里"判据 N 条"与
    合并前同值 ✓）。**断言 id** 是新加的（`W-C1` / `W-C1g` / `W-C2` / `W-C3`），
    而**"这一跑产出了哪些断言"**（`produced`）单独返回 —— 它包含绿路那一条
    ⇒ 驱动器才能对账（`gs3` 的判例 (v)：**只删代码里的断言 ⇒ 照样判红** ✗）✓。

    `top_level` = `discover_top_level()` 的结果（**显式传进来** ⇒ 判据无全局状态、
    `--selftest` 的 ⑥ 才能在**合成仓**的那份表上跑 ✓）。
    """
    jobs = doc.get("jobs") or {}
    res = []
    produced: set[str] = set()

    # ── R1 门控覆盖 ────────────────────────────────────────────────────
    gated_jobs = 0
    for jid, job in sorted(jobs.items()):
        if not isinstance(job, dict):
            continue
        flags = gated_flags(job)
        if not flags:
            continue
        gated_jobs += 1
        # **工作流文件本身**被它定义的每个 job 消费 ⇒ 它必须落在**某个**门控组里 ✓
        # （实例②：只改 `.github/workflows/**` ⇒ 三个标志全 false ⇒ 刚改的那条 job
        #  永远得不到验证 ✗）。按 **job 级析取**判，不是"每组都要有" ✗ ——
        # `.github/workflows/**` 在 `rust` 组里，而每个门控 job 的 `if` 都含 `rust`
        # ⇒ 已经够了 ✓；要求"每个组都有"会制造假红 ✗。
        for token in consumed_paths(job, top_level) + [".github/workflows/ci.yml"]:
            produced.add("W-C1")   # ⭐ 绿路也要"产出"（读数不记 ⇒ 见 docstring ✓）
            hits = classify(token, filters, glob_to_regex)
            if not hits:
                res.append((
                    "W-C1",
                    False,
                    f"R1 `{jid}` 消费 `{token}`，但它**不在任何过滤器组里** ✗ "
                    f"⇒ 那个路径的改动**不会**让这个 job 跑 ⇒ 改了没人验 ✗"
                    f"（门控标志 {sorted(flags)}）",
                ))
            elif not (hits & flags):
                res.append((
                    "W-C1",
                    False,
                    f"R1 `{jid}` 消费 `{token}`，但它只落在 {sorted(hits)} 组里，"
                    f"而门控这个 job 的是 {sorted(flags)} ✗ "
                    f"⇒ 改那个路径**不会**让这个 job 跑 ✗",
                ))
    res.append(("W-C1g", gated_jobs > 0,
                f"R1 有 {gated_jobs} 个 job 挂在 `changes` 标志上（实测）"))
    produced.add("W-C1g")

    # ── R2 门控标志 ↔ 过滤器组 一一对应 ────────────────────────────────
    gating_groups = set()
    for job in jobs.values():
        if isinstance(job, dict):
            gating_groups |= gated_flags(job)
    res.append(("W-C2", bool(gating_groups),
                f"R2 门控了 job 的过滤器组：{sorted(gating_groups)}"))
    produced.add("W-C2")
    for name in sorted(gating_groups):
        res.append((
            "W-C2",
            name in filters,
            f"R2 门控标志 `{name}` 在 `filters:` 里有对应组（实测 {sorted(filters)}）",
        ))

    # ── R3 数据依赖（声明式，见 `DATA_DEPENDENCIES` 的说明）────────────
    for jid, path, why in DATA_DEPENDENCIES:
        produced.add("W-C3")
        job = jobs.get(jid)
        if not isinstance(job, dict):
            res.append(("W-C3", False,
                        f"R3 表里点名了 job `{jid}`，但 `ci.yml` 里没有它 ✗（表腐烂了）"))
            continue
        flags = gated_flags(job)
        if not flags:
            res.append((
                "W-C3",
                False,
                f"R3 表里点名了 job `{jid}`，但它**不再被 `changes` 门控** ✗ "
                f"⇒ 这条声明已经没有意义（表腐烂了）",
            ))
            continue
        hits = classify(path, filters, glob_to_regex)
        res.append((
            "W-C3",
            bool(hits & flags),
            f"R3 `{jid}` 读 `{path}`（{why}）⇒ 必须被门控它的过滤器命中"
            f"（实测落 {sorted(hits)}，门控标志 {sorted(flags)}）",
        ))
    return res, produced


def report_coverage(res, top_level):
    bad = [msg for _id, ok, msg in res if not ok]
    for _id, ok, msg in res:
        if not ok:
            print(f"✗ {msg}")
    # ⭐ **把枚举结果印出来**（评审可见 ✓）：它决定了"哪些 token 进 R1 的面"——
    #   修前那张硬编码表**从来不印** ⇒ 少了条目没人看得见（本洞之所以"静默" ✓）。
    print(
        f"  ⓘ 顶层条目 {len(top_level)} 个（按 `git ls-files` **实际枚举** ✓ ⇒ 新加一级条目"
        f"**自动**进面、没有第二张表 ✗）：{' · '.join(top_level)}"
    )
    print(
        f"  ⓘ 判据 {len(res)} 条，违反 {len(bad)} 条 —— "
        f"口径：R1 job 消费的每条仓库路径必须被门控它的过滤器命中 · "
        f"R2 标志↔组一一对应 · R3 声明的数据依赖必须被门控 ✓"
    )
    return 1 if bad else 0


# ─────────────────────────── 层① 自检（--selftest）───────────────────────────

def _rewrite_filters(text, drop):
    """从 `filters:` 内联块里删掉一行（`drop` = 要删的 pattern 字面量，含引号）。"""
    out, dropped = [], 0
    for line in text.splitlines():
        if drop in line and line.strip().startswith("-"):
            dropped += 1
            continue
        out.append(line)
    return "\n".join(out) + "\n", dropped


def _add_ungated_job(text, consumed="editor/vscode/test-extension-host.js"):
    """加一个**消费 `consumed` 却只被 `rust` 门控**的 job（实例①的换皮 ✓）。"""
    block = (
        "\n  selftest-ungated:\n"
        "    needs: changes\n"
        "    if: needs.changes.outputs.rust == 'true'\n"
        "    runs-on: ubuntu-24.04\n"
        "    steps:\n"
        f"      - run: node {consumed}\n"
    )
    return text.rstrip("\n") + "\n" + block


# `--selftest` ⑥ 的合成仓**照真仓的形状**（拷真文件的形状、**不是打桩** ✗）：
# 地板那几个一级条目各有一个文件在里面 —— 空目录**不进** `git ls-files` ⇒ 枚举看不见它 ✓。
_FIXTURE_FLOOR_FILES = (".github/workflows/.keep", "Cargo.toml", "courses/.keep",
                        "crates/.keep", "docs/.keep", "scripts/.keep")


def _fixture_repo(tmp: Path, files: tuple[str, ...]):
    """合成一个**真 git 仓**（`files` 逐个落盘并 `git add` ⇒ 进 `git ls-files` ✓）。

    ⚠ **夹具跑 git 一律用 `git_env.clean_git_env()`**（洞
    `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`，2026-10-16）：`GIT_DIR` **优先于 cwd** ⇒
    不清的话夹具的 `git init` 会把 `core.bare=true` 写进**调用者的** `.git/config`、
    并让后续每一句 `git add` 落在真仓上 ✗。
    """
    root = tmp / "repo"
    root.mkdir(parents=True)
    env = git_env.clean_git_env()
    init = subprocess.run(["git", "init", "-q"], cwd=root, capture_output=True, env=env)
    if init.returncode != 0:
        return None, f"`git init` rc={init.returncode}：{init.stderr.decode('utf-8', 'replace')[:120]}"
    for rel in files:
        p = root / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text("// 夹具（照真仓的形状 ✓）\n", encoding="utf-8")
    add = subprocess.run(["git", "add", "-A"], cwd=root, capture_output=True, env=env)
    if add.returncode != 0:
        return None, f"`git add -A` rc={add.returncode}：{add.stderr.decode('utf-8', 'replace')[:120]}"
    return root, None


def coverage_selftest(ci_path, push_range, top_level, produced: set[str] | None = None):
    """层① 的正负对照（见模块文档的判据表 ✓）⇒ `(exit, [臂 id])`。

    ⚠ `produced`（可选）：把每条臂的**断言 id** 记进去 ⇒ 驱动器能对账
    "这一跑产出了哪些自检臂" ✓（`gs3` 的 (v) 同口径 ✓）。
    """
    base_text = Path(ci_path).read_text(encoding="utf-8")
    results = []

    def run(arm: str, text, label, expect_red, top_level=top_level, needle=None):
        if produced is not None:
            produced.add(arm)
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "ci.yml"
            p.write_text(text, encoding="utf-8")
            doc = load_ci(p)
            filters = push_range.load_filters(p)
            res, _ids = coverage_problems(doc, filters, push_range.glob_to_regex, top_level)
            reds = [msg for _id, ok, msg in res if not ok]
            ok = bool(reds) == expect_red
            # **归属**（不许"有红就算咬住" ✗）：`needle` 给了就要求那一条红**点名它** ✓
            if ok and expect_red and needle is not None and not any(needle in m for m in reds):
                ok = False
                reds = [f"夹具的红**没有点名** `{needle}`：{reds[:2]}"]
            results.append((ok, label, reds[:2]))

    # ① 正对照：现役 ci.yml 的逐字副本必须绿 ✓（不许制造假红 ✗）
    run("S-C1", base_text, "① 正对照：现役 ci.yml 逐字副本 ⇒ 必须**绿**", expect_red=False)
    # ②③④ 负对照：把历史三次事故的修法**撤掉** ⇒ 必须判红 ✓
    for arm, drop, label in [
        ("S-C2", '"scripts/**"', "② 撤掉 `scripts/**`（实例①）⇒ 必须**红**"),
        ("S-C3", '".github/workflows/**"', "③ 撤掉 `.github/workflows/**`（实例②）⇒ 必须**红**"),
        ("S-C4", '"docs/gaps/**"', "④ 撤掉 `docs/gaps/**`（实例③）⇒ 必须**红**"),
    ]:
        text, dropped = _rewrite_filters(base_text, drop)
        if dropped == 0:
            if produced is not None:
                produced.add(arm)
            results.append((False, f"{label} —— **夹具没改到东西**（`{drop}` 没找到）✗", []))
            continue
        run(arm, text, f"{label}（实测删了 {dropped} 行）", expect_red=True)
    # ⑤ 换皮：新加一个消费 `editor/**` 却只被 `rust` 门控的 job ⇒ 必须判红 ✓
    run("S-C5", _add_ungated_job(base_text),
        "⑤ 新加消费 `editor/**` 却只被 `rust` 门控的 job ⇒ 必须**红**", expect_red=True)
    # ⑥ **动态枚举**（洞 `TOP-LEVEL-TABLE-GOES-BLIND` 的反向验证 ✓）：合成仓里**新加一个
    #   顶层目录** `newtop/` ⇒ ①枚举必须**看到**它（写死表 ⇒ 这一臂必红 ✗）；
    #   ②它下面的路径被门控 job 消费 ⇒ R1 **必须判红并点名它**（改前：认不出来 ⇒ 静默绿 ✗）；
    #   ③地板不齐的仓 ⇒ **必须拒绝判**（不静默退化成一张更窄的表 ✗）。
    with tempfile.TemporaryDirectory() as tmp:
        repo, ferr = _fixture_repo(Path(tmp), _FIXTURE_FLOOR_FILES + ("newtop/thing.js",))
        if repo is None:
            if produced is not None:
                produced.add("S-C6a")
            results.append((False, f"⑥ 合成仓夹具环境错（{ferr}）⇒ 这一臂判不了 ✗", []))
        else:
            found = discover_top_level(repo)
            combined = tuple(sorted(set(top_level) | set(found)))
            seen = "newtop" in found and looks_like_repo_path("newtop/thing.js", combined)
            if produced is not None:
                produced.add("S-C6a")
            results.append((
                seen,
                "⑥a **动态枚举**：合成仓里新加顶层目录 `newtop/` ⇒ `discover_top_level()` "
                "必须**看到**它（硬编码表 ⇒ 必红 ✗）",
                [] if seen else [f"`discover_top_level(合成仓)` = {found} ✗"],
            ))
            run("S-C6b", _add_ungated_job(base_text, "newtop/thing.js"),
                "⑥b 那个**新顶层目录**里的路径被门控 job 消费 ⇒ R1 **必须判红并点名它**"
                "（改前：token 认不出来 ⇒ **静默绿** ✗）",
                expect_red=True, top_level=combined, needle="newtop/thing.js")
    with tempfile.TemporaryDirectory() as tmp:
        bare, ferr = _fixture_repo(Path(tmp), ("newtop/thing.js",))
        if produced is not None:
            produced.add("S-C6c")
        if bare is None:
            results.append((False, f"⑥c 合成仓夹具环境错（{ferr}）⇒ 这一臂判不了 ✗", []))
        else:
            try:
                got = discover_top_level(bare)
                results.append((False, "⑥c **地板闸**：地板不齐的仓必须拒绝判 ⇒ 这一臂判不了 ✗",
                                [f"`discover_top_level` 返回了 {got}（没拒绝 ✗）"]))
            except EnvError as exc:
                results.append((True, f"⑥c **地板闸**：地板不齐的仓 ⇒ 拒绝判（不静默退化 ✓）："
                                      f"{str(exc)[:90]}…", []))

    bad = 0
    for ok, label, reds in results:
        print(f"{'✓' if ok else '✗'} {label}")
        if not ok:
            bad += 1
            for msg in reds:
                print(f"    {msg}")
    print(f"  ⓘ 层① 自检 {len(results)} 条，未咬住 {bad} 条 —— "
          f"口径：正对照必须绿、负对照（②–⑤、⑥b）与两条断言（⑥a/⑥c）必须咬住 ✓")
    return (1 if bad else 0), results


# ═══════════════════════════════════════════════════════════════════════════
# 层② `history` —— 原文 `scripts/check-history-depth-wiring.py`（逐字保留 ✓）
# ═══════════════════════════════════════════════════════════════════════════

# ── 声明：**跑它们就要全历史**（每条带理由 ✓ 评审可见 ✓）─────────────────
#
# ⚠ 每条的"实测读数"= 真 `git clone --depth 1` 夹具（2026-10-16）里跑出来的 ——
#   写进理由里是为了让"为什么它算历史判据"**可复核** ✓（判据**不核**这一列：
#   它是**证据**，不是断言 ⇒ 免得把读数变成第二个会腐烂的表 ✗）。
HISTORY_DEPENDENT: list[tuple[str, str]] = [
    ("scripts/check-guard-bite.py",
     "C1 覆盖面算 `git diff <coverage_since>..HEAD`（基线 = 台账抬头的 `# coverage_since:`）"
     "；浅克隆实测 rc=3"),
    # ⚠ **2026-10-17 改名**（第 6 轮 `gs6`：`check-attention-budget.py` +
    # `check-threshold-ratchet.py` 合并成 `check-budgets.py` ✓，`SUB-narrow-guards-to-one-layer` ②）
    # —— 两层**各自**都要全历史（`--layer attention` 的簿记窗口 = 固定锚点..HEAD；
    # `--layer threshold` 的开轮纪律 = 基点之后的提交数）⇒ 合并后**一条**登记覆盖两层 ✓。
    ("scripts/check-budgets.py",
     "**两层都要**：层① 的簿记窗口 = 固定锚点..HEAD（`git log`；浅克隆实测 rc=3）**+** "
     "层② 的开轮纪律 = 基点之后的提交数（`git log`；浅克隆实测 rc=1）"),
    ("scripts/check-queue-integrity.py",
     "**两层都要**：复核层 C4 的祖先性断言（`git merge-base --is-ancestor`；浅克隆实测 rc=1）"
     "**+** 触发器层的咬住证明打在**固定旧 rev** 上（`--bite` ⇒ `git cat-file`/`git show <rev>:…`；"
     "浅克隆实测 rc=3）—— 2026-10-17 合并前这是**两条**（`check-queue-views.py` + "
     "`check-queue-rewrite-trigger.py`）✓"),
    ("scripts/ux-queue.py",
     "收口指针的祖先性断言（`git merge-base --is-ancestor`）；浅克隆实测 rc=1"),
    # ⚠ **2026-10-17 补**（`ng1` 普查 §3.C 第 ③ 组：它的 **CI 面**缺 ⇒ 接进 `queue-lint` ✓）——
    # **如实划的边界** ✗：本条的**判据本体不需要全历史**（`--check` / `--selftest` 浅克隆
    # **实测 rc=0** ✓）；**要全历史的是它的咬住证明**（`docs/guard-bite.jsonl` 那一行的
    # `--check --source-rev 9c8cfa90` ⇒ `git show` ⇒ 浅克隆**实测 rc=3** ✗）。
    # ⇒ 与本表里 `check-queue-integrity.py`（触发器层的咬住证明）、以及**本层自指**那两条**同形** ✓
    # （"跑它就要全历史" = **连它的咬住证明一起跑**就要全历史 ✓，不是"判据本体读历史" ✗）。
    # ⚠ 反过来说：**同族的 `scripts/audit-notation-paths.py` 故意不登记** ✗ —— 它全文
    # **0 处** `git`/`subprocess`，判据与 `--self-test` 浅克隆都 rc=0 ⇒ 登记它 = 给一条
    # 不需要全历史的脚本发"要全历史"的假红（E18：守卫一旦叫狼来了就死了 ✗）。
    ("scripts/check-frozen-headroom.py",
     "咬住证明打在**固定旧 rev** 上（`--check --source-rev 9c8cfa90` ⇒ `git show <rev>:<path>`）"
     "；浅克隆实测 rc=3（判据/`--selftest` 本身浅克隆 rc=0 —— 同 `check-queue-integrity.py` 触发器层的形态 ✓）"),
    # ⚠ **2026-10-17 补**（第 3 轮 `gs3`：`check-doc-drift.py` + `check-launcher-extension-link.py`
    # 合并成 `check-drift.py` ✓，`SUB-narrow-guards-to-one-layer` ①）——
    # **如实划的边界** ✗（**与上面那条同形** ✓）：本条的**判据本体不需要全历史**
    # （默认 / `--json` / `--layer` / `--selftest` 浅克隆**实测 rc=0** ✓ —— 全文唯一的 `git`
    # 调用在 `_git_show()` 里，只服务 `--rev`）；**要全历史的是它的咬住证明**：
    # `docs/guard-bite.jsonl` 那一行的 `--rev 755e5302^` ⇒ `git show <rev>:<path>` ⇒
    # 浅克隆里那个对象不在 ⇒ `_git_show()` 返回 `None` ⇒ **rc=2**（形状异常）⇒ 咬住证明
    # **空转**（"这条守卫咬不住"的**假红** ✗）。
    # ⇒ 登记进本表 = 让 R1 把"跑它的 job 必须有全历史"变成可查的 ✓（**本轮实测**：
    # `wiring-lint` 原来**没有** `fetch-depth: 0` ⇒ 登记后 R1 **当场判红**，本轮给它补上 ✓）。
    ("scripts/check-drift.py",
     "咬住证明打在**固定旧 rev** 上（`--rev 755e5302^` ⇒ `git show <rev>:<path>`；"
     "浅克隆实测 rc=2 —— 判据/`--selftest` 本身浅克隆 rc=0，同 `check-frozen-headroom.py` 的形态 ✓）"),
    # ⚠ **2026-10-17 补**（第 7 轮 `ng8`：把 `check-site.py` 接进 `ci.yml` 的 `queue-lint` ✓）——
    # ⭐ **本条与上面那三条"要全历史的是咬住证明"不同形** ✗：它的**判据本体**就读 tag ✓
    # （形态同 `check-attention-budget.py` 的"窗口 = `git log`"✓）。
    # 链路：`check-site.py:496-502` 的 `check_data()` ⇒ `gen-site-data.py --check` ⇒
    # `latest_release_tag()`（`gen-site-data.py:46-65`）= **本地** `git tag --list v*`
    # ⇒ 浅克隆里一个 tag 都没有 ⇒ 生成器 **rc=3** ⇒ `check_data()` 把它折成
    # **rc=1**（`raise Failure("解析不出已发布 tag（浅克隆缺 tag？）—— 无法判定，不判绿")`）。
    # **实测**（真 `git clone --depth 1 --no-tags file://…` 夹具，2026-10-17）：夹具 tag 数
    # **0** · `check-site.py` **rc=1**（9/10 · `✗ data`）· `gen-site-data.py --check` **rc=3** ✓。
    # ⚠ `pages.yml:61-67` 早就把这条前提写在注释里（"`fetch-depth: 0` **是必需的，不是优化**"）
    # —— 只是**没有任何判据核它** ✗；登记进本表 = 把它从注释变成可查的 ✓。
    ("scripts/check-site.py",
     "`data` 项 = `gen-site-data.py --check` ⇒ **本地** `git tag --list v*`（站点写的是已发布"
     "版本的事实）；浅克隆（`--depth 1 --no-tags`）里 tag 数 0 ⇒ 实测 rc=1"
     "（`✗ data 解析不出已发布 tag（浅克隆缺 tag？）`）"),
    # ⚠ **本层自己也算**（如实 ✓）：咬住证明要
    # `git show 1cd4aba0:.github/workflows/ci.yml` ⇒ 浅克隆里那个对象不在 ⇒ rc=3。
    # 自指是**故意的**：R1 因此也会核"谁跑本层，谁就得有全历史" ✓。
    # ⭐ **2026-10-17 改名**（第 5 轮 `gs5`：`check-ci-gate-coverage.py` +
    # `check-history-depth-wiring.py` 合并成 **本文件** ✓，`SUB-narrow-guards-to-one-layer` ③）——
    # **行数一条没变**（合并前那条自指指向 `scripts/check-history-depth-wiring.py` ✓）。
    ("scripts/check-ci-wiring.py",
     "层② 的咬住证明打在**固定旧 rev** 上（`--ci-yml-rev 1cd4aba0` ⇒ `git show <rev>:…`）"),
]

# ⭐ **钉死的旧 rev**（pr1 的坑：坏版本**不能写 `HEAD`** ✗ —— 提交后 `HEAD`
# 就是修好的那一版 ⇒ 咬住证明会**自己失效**）。这一版 = CI **真红**的那一跑的
# `head_sha`（run `38817419732`）✓ —— 是**已存在的坏输入**，不是本层新造的 ✓。
PINNED_BAD_REV = "1cd4aba0"

# `fetch-depth` 的**活行**（注释里提一句不算 ✗ —— 见 docstring 的口径 ✓）。
FETCH_DEPTH_RE = re.compile(r"^\s*fetch-depth:\s*['\"]?0['\"]?\s*$")
CHECKOUT_RE = re.compile(r"uses:\s*actions/checkout@")
STEP_RE = re.compile(r"^\s*- ")
USES_RE = re.compile(r"^\s*uses:")


def _load_guard_bite():
    """复用 `check-guard-bite.py` 的 `job_blocks` / `_JOB_KEY_RE` / `shallow_hint`
    （**不另抄一份** ✗，照层① 复用 `ci-push-range.py` 的先例 ✓）。

    ⚠ "job 块怎么切"只有**一处定义** ✓ —— 那边 C4 的接线闸用的就是同一条规则；
    这里再抄一份 = 两处会漂移 ✗。
    """
    path = HERE / "check-guard-bite.py"
    spec = importlib.util.spec_from_file_location("check_guard_bite", path)
    if spec is None or spec.loader is None:  # pragma: no cover - 环境问题
        raise EnvError(f"加载不了 {path} ✗", code=3)
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


def history_problems(ci_text: str, declared: list[tuple[str, str]]):
    """层② 的 R1（接线）+ R1b（提及不算跑）+ R2（表不腐烂）+ R3（空转闸）。

    ⇒ `([(断言 id, False, 红行)], 判到的 (job, 脚本) 对, 产出的断言 id 集合)` ✓。

    ⚠ **读数与合并前逐字相同** ✗（`res` 里就是原来那串**红行**，一条不多）—— 多的是
    **断言 id**（`W-H1`/`W-H2`/`W-H3`/`W-H4`）与**"这一跑产出了哪些断言"**
    （`produced`：**绿路也算产出** ✓ —— `gs3` 的 (v)：真绿跑必须把全部断言产出，
    否则"只删代码里的断言"没有任何读数 ✗）。
    """
    gb = _load_guard_bite()
    blocks = gb.job_blocks(ci_text)
    res: list[tuple[str, bool, str]] = []
    produced: set[str] = set()
    checked: list[str] = []

    # ── R1 接线 + R1b 提及不算跑 ────────────────────────────────────────
    mentioned_any = False
    for job, block in sorted(blocks.items()):
        live = "\n".join(live_lines(block))
        # ⭐ **"真的跑"= `run:` 正文**（`run_bodies()` —— 唯一一处 ✓）：注释**与 step 名**
        #   都不算 ✗（第 6 轮 `ng7` 收窄；读数与理由见模块 docstring 的"刻意收窄"节 ✓）。
        bodies = gb.run_bodies(block)
        used = sorted(rel for rel, _why in declared if rel in bodies)
        # ⭐ **R1b**：活行里点名了、但 `run:` 正文里没有它 ⇒ 那一跑**不存在**（空转 ✗）。
        mentioned = sorted(rel for rel, _why in declared if rel in live and rel not in bodies)
        if mentioned:
            mentioned_any = True
            produced.add("W-H2")
            named_m = " · ".join(f"`{r}`" for r in mentioned)
            res.append(("W-H2", False,
                        f"**R1b 提及不算跑** ✗：`{job}` 的**活行**里点名了 {named_m}，"
                        f"而它的 **`run:` 正文里没有**它们 ⇒ 那些步**只剩名字**"
                        f"（`run:` 正文被删/改成别的命令 ✗）⇒ 那几条历史判据在 CI 上"
                        f"**根本不跑**（空转 ✗）。出路二选一：①在 `run:` 正文里真跑它 ✓；"
                        f"②把那个提及**挪进注释**（注释不算 ✓）或删掉 ✓"))
        if not used:
            continue
        fd = checkout_fetch_depth(block)
        produced.add("W-H1")   # ⭐ 绿路也要"产出"（读数不记 ⇒ 见 docstring ✓）
        for rel in used:
            checked.append(f"{job}:{rel}")
        named = " · ".join(f"`{r}`" for r in used)
        if fd is None:
            res.append(("W-H1", False,
                        f"**R1 接线** ✗：`{job}` 真的跑了历史判据 {named}，"
                        f"而它**没有 `actions/checkout`** ⇒ 历史拿不到 ⇒ 那些判据会以"
                        f"『环境不在』（`exit 3`）判红 ✗"))
        elif not fd:
            res.append(("W-H1", False,
                        f"**R1 接线** ✗：`{job}` 真的跑了历史判据 {named}，"
                        f"而它的 `actions/checkout` **没有 `fetch-depth: 0`** ⇒ 浅克隆 ⇒ "
                        f"基线那个对象不在 ⇒ `git` **rc=128** ⇒ 判据 **exit 3**"
                        f"（红的是**前提**、不是『咬不住』✗）⇒ 给这个 job 的 checkout 加 "
                        f"`fetch-depth: 0` ✓"))
        # ⚠ 绿路**不记读数**（读数与合并前逐字相同 ✗）—— 只记 `produced` ✓。
    if not mentioned_any:
        produced.add("W-H2")

    # ── R2 表不腐烂 ─────────────────────────────────────────────────────
    produced.add("W-H3")   # ⭐ 绿路也要"产出"（读数不记 ⇒ 见 docstring ✓）
    for rel, _why in declared:
        if not (ROOT / rel).exists():
            res.append(("W-H3", False,
                        f"**R2 表不腐烂** ✗：`HISTORY_DEPENDENT` 点名了 `{rel}`，"
                        f"而**仓库里没有它** ⇒ 表在腐烂 ✗（改那张表或把脚本加回来 ✓）"))

    # ── R3 空转闸 ───────────────────────────────────────────────────────
    produced.add("W-H4")   # ⭐ 绿路也要"产出"（读数不记 ⇒ 见 docstring ✓）
    if not declared:
        res.append(("W-H4", False,
                    "**R3 空转** ✗：`HISTORY_DEPENDENT` 是**空的** ⇒ 这一跑**什么都没判**"
                    "（空转 ✗ —— 判据的输入必须真的存在 ✓）"))
    elif not checked:
        res.append(("W-H4", False,
                    "**R3 空转** ✗：**没有任何** `(job, 脚本)` 对被判到 ⇒ "
                    "这一跑**什么都没查**（`ci.yml` 结构变了 / 声明与它脱节了 ✗）"
                    "⇒ 不许当通过 ✗"))
    return res, checked, produced


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
        raise EnvError(msg + (f"\n       {hint}" if hint else ""), code=3)
    return out.stdout


def run_history(ci_text: str, declared: list[tuple[str, str]], label: str,
                quiet: bool = False):
    """层② 的判据 ⇒ `(exit, 产出的断言 id 集合)` ✓。"""
    res, checked, ids = history_problems(ci_text, declared)
    if not quiet:
        print(f"历史深度接线（{label}）：判到 {len(checked)} 个 `(job, 脚本)` 对 · "
              f"声明 {len(declared)} 条")
        for pair in checked:
            print(f"    ✓ {pair}")
    red = [msg for _id, ok, msg in res if not ok]
    if red:
        print(f"\n✗ 历史深度接线 **{len(red)} 条红**：", file=sys.stderr)
        for m in red:
            print(f"  · {m}", file=sys.stderr)
        return 1, ids
    if not quiet:
        print("✓ 历史深度接线：跑历史判据的 job 都拿到了全历史 ✓ · "
              "接线**不靠提及**（`run:` 正文里真的跑 ✓）")
    return 0, ids


# ── 层② 自检：正负对照（**咬得住历史 bug 的守卫才算数** ✓）─────────────────

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


def history_selftest(produced: set[str] | None = None):
    """① 正对照（现役 `ci.yml` 必须绿）+ ② 真旧 rev + ③④⑤ 负对照 ⇒ 逐条印读数 ✓。

    ⇒ `(exit, [臂])`；每条臂的**断言 id**（`S-H1`…`S-H8`）记进 `produced` ✓。
    """
    gates: list[tuple[str, bool, str]] = []

    def gate(arm: str, name: str, ok: bool, reading: str) -> None:
        if produced is not None:
            produced.add(arm)
        gates.append((name, ok, reading))

    live = DEFAULT_CI.read_text(encoding="utf-8")
    base_res, base_checked, _bi = history_problems(live, HISTORY_DEPENDENT)
    base_red = [m for _i, ok, m in base_res if not ok]
    gate("S-H1", "① 正对照：现役 `ci.yml` ⇒ **绿**（不许制造假红 ✗）",
         not base_red, f"红 {len(base_red)} 条 · 判到 {len(base_checked)} 对")
    gate("S-H2", "①b 正对照真的**判到了东西**（不是空转 ✓）",
         len(base_checked) >= 1, f"{len(base_checked)} 对：{base_checked}")

    # ② **咬住证明**：CI 真红的那一版（钉死旧 rev ✓，**不是** `HEAD` ✗）。
    old = read_ci_at_rev(PINNED_BAD_REV)
    old_res, _old_checked, _oi = history_problems(old, HISTORY_DEPENDENT)
    old_red = [m for _i, ok, m in old_res if not ok]
    bite_named = any("editor" in m and "check-guard-bite.py" in m for m in old_red)
    gate("S-H3", f"② 咬住证明：钉死旧 rev `{PINNED_BAD_REV}`（CI 真红的 `head_sha`）⇒ **判红**"
                 f" + 点名 `editor` 与 `check-guard-bite.py` ✓",
         bool(old_red) and bite_named, f"红 {len(old_red)} 条 · 点名 editor={bite_named}")

    # ③ 负对照：撤掉 `board` 的 `fetch-depth: 0` ⇒ 必须红（2026-10-24：queue-lint 拆成
    #    board / guard-bite / budgets / site / hygiene 五个 job ⇒ 夹具改成 board）。
    q = _strip_fetch_depth(live, "board")
    q_res, _qc, _qi = history_problems(q, HISTORY_DEPENDENT)
    q_red = [m for _i, ok, m in q_res if not ok]
    gate("S-H4", "③ 负对照：撤掉 `board` 的 `fetch-depth: 0` ⇒ **判红** + 点名 board ✓",
         any("board" in m for m in q_red), f"红 {len(q_red)} 条")

    # ④ 负对照：新加一个"跑历史判据却没全历史"的 job ⇒ 必须红。
    s_res, _sc, _si = history_problems(live + SYNTHETIC_JOB, HISTORY_DEPENDENT)
    s_red = [m for _i, ok, m in s_res if not ok]
    gate("S-H5", "④ 负对照：新加一个跑历史判据、却没有 `fetch-depth: 0` 的 job ⇒ **判红** ✓",
         any("synthetic-shallow-history-job" in m for m in s_red), f"红 {len(s_red)} 条")

    # ⑤ 防腐烂：表里点名一个不存在的脚本 ⇒ 必须红。
    ghost = HISTORY_DEPENDENT + [("scripts/no-such-guard.py", "合成的腐烂条目")]
    g_res, _gc, _gi = history_problems(live, ghost)
    g_red = [m for _i, ok, m in g_res if not ok]
    gate("S-H6", "⑤ 防腐烂：`HISTORY_DEPENDENT` 点名一个**不存在**的脚本 ⇒ **判红** ✓",
         any("no-such-guard.py" in m for m in g_red), f"红 {len(g_red)} 条")

    # ⑥ 空转闸：声明清空 ⇒ 必须红（不许"什么都没判"当通过 ✗）。
    e_res, _ec, _ei = history_problems(live, [])
    e_red = [m for _i, ok, m in e_res if not ok]
    gate("S-H7", "⑥ 空转闸：`HISTORY_DEPENDENT` 清空 ⇒ **判红**（空转不算通过 ✗）",
         bool(e_red), f"红 {len(e_red)} 条")

    # ⭐⭐ **第 6 轮 `ng7` 的反向验证**：**删 `run:` 正文、只留 step 名** ⇒ 必须判红 ✓
    #   （⚠ 夹具**带** `fetch-depth: 0` ⇒ 它**不可能**靠 R1 判红 ⇒ 红只能来自 R1b ✓
    #     —— 收窄前这一条是**绿**的（提及级把它算成"跑了" ✗））
    m_res, m_checked, _mi = history_problems(live + SYNTHETIC_MENTION_ONLY_JOB, HISTORY_DEPENDENT)
    m_red = [m for _i, ok, m in m_res if not ok]
    m_named = any("R1b" in m and "synthetic-mention-only-job" in m for m in m_red)
    gate("S-H8", "⑦ 反向（第 6 轮）：**删 `run:` 正文、只留 step 名**（且 `fetch-depth: 0` 在 ✓）"
                 "⇒ **判红** + 点名 R1b ✓",
         m_named, f"红 {len(m_red)} 条 · 点名 R1b={m_named} · 判到 {len(m_checked)} 对")

    # ⭐ 正对照（**不许把真跑也判成提及** ✗）：step 名 + `run:` 正文里真的跑它 ⇒ 不许红 ✓。
    r_res, r_checked, _ri = history_problems(live + SYNTHETIC_REAL_RUN_JOB, HISTORY_DEPENDENT)
    r_red = [m for _i, ok, m in r_res if not ok]
    gate("S-H9", "⑧ 正对照：step 名点名了、`run:` 正文里**也真的跑**它 ⇒ **不许红**（R1b 不误伤 ✓）",
         not r_red and f"synthetic-real-run-job:scripts/check-guard-bite.py" in r_checked,
         f"红 {len(r_red)} 条 · 判到 {len(r_checked)} 对")

    print("── 历史深度接线 · selftest（正负对照）──")
    for name, ok, reading in gates:
        print(f"  {'✓' if ok else '✗'} {name} ⇒ {reading}")
    bad = [n for n, ok, _ in gates if not ok]
    if bad:
        print(f"\n✗ selftest **{len(bad)} 条红**：{bad}", file=sys.stderr)
        return 1, gates
    print(f"\n✓ selftest：**{len(gates)} 条臂全绿** —— 现役绿 ✓ · 真旧 rev 咬得住 ✓ · "
          f"负对照/防腐烂/空转闸逐条红 ✓")
    return 0, gates


# ═══════════════════════════════════════════════════════════════════════════
# 登记表 + 驱动器（**一本登记表 + 一个驱动器** ✓ —— `gs2`/`gs3`/`gs4` 的判例 (i)）
# ═══════════════════════════════════════════════════════════════════════════

# 每条断言：id → (层, 判什么, 判据入口)。
#
# ⚠ **登记表在文件里**（不是外部锚）⇒ 删一条要改**两处**（`CRITERIA` 与
# `REQUIRED_ASSERTIONS`）—— 评审看得见 ✓；真正的外部记录是 `docs/guard-bite.jsonl`
# 那一行（它写的是**坏输入**与**命令**，不是断言清单 ✓）—— 与 `gs2`/`gs3`/`gs4` 同形 ✓。
CRITERIA: dict[str, tuple[str, str, str]] = {
    # ── 层① 的绿路判据（R1/R2/R3）──────────────────────────────────────
    "W-C1": ("coverage", "R1 门控覆盖：门控 job 消费的每条仓库路径必须被门控它的过滤器命中",
             "coverage_problems()"),
    "W-C1g": ("coverage", "R1 空转闸：至少要有一个 job 挂在 `changes` 标志上",
              "coverage_problems()"),
    "W-C2": ("coverage", "R2 标志↔组：每个 `if:` 里的 `changes` 标志都要在 `filters:` 里有组",
             "coverage_problems()"),
    "W-C3": ("coverage", "R3 数据依赖：声明的数据路径必须被门控它的过滤器命中",
             "coverage_problems()"),
    # ── 层② 的绿路判据（R1/R1b/R2/R3）─────────────────────────────────
    "W-H1": ("history", "R1 接线：真的跑历史判据的 job 必须带 `fetch-depth: 0`",
             "history_problems()"),
    "W-H2": ("history", "R1b 提及不算跑：活行里点名而 `run:` 正文里没有 ⇒ 判红",
             "history_problems()"),
    "W-H3": ("history", "R2 表不腐烂：`HISTORY_DEPENDENT` 点名的脚本必须在仓库里",
             "history_problems()"),
    "W-H4": ("history", "R3 空转闸：至少要判到一个 `(job, 脚本)` 对",
             "history_problems()"),
    # ── 层① 的自检臂 ───────────────────────────────────────────────────
    "S-C1": ("coverage", "自检①正对照：现役 `ci.yml` 逐字副本必须绿", "coverage_selftest()"),
    "S-C2": ("coverage", "自检②负对照：撤掉 `scripts/**` ⇒ 判红（实例①）", "coverage_selftest()"),
    "S-C3": ("coverage", "自检③负对照：撤掉 `.github/workflows/**` ⇒ 判红（实例②）",
             "coverage_selftest()"),
    "S-C4": ("coverage", "自检④负对照：撤掉 `docs/gaps/**` ⇒ 判红（实例③）", "coverage_selftest()"),
    "S-C5": ("coverage", "自检⑤负对照：新加消费 `editor/**` 却只被 `rust` 门控的 job ⇒ 判红",
             "coverage_selftest()"),
    "S-C6a": ("coverage", "自检⑥a：合成仓新加顶层目录 ⇒ `discover_top_level()` 必须看到它",
              "coverage_selftest()"),
    "S-C6b": ("coverage", "自检⑥b：那个新顶层目录里的路径被门控 job 消费 ⇒ R1 判红并点名",
              "coverage_selftest()"),
    "S-C6c": ("coverage", "自检⑥c：地板不齐的仓 ⇒ 拒绝判（exit 2，不静默退化）",
              "coverage_selftest()"),
    # ── 层② 的自检臂 ───────────────────────────────────────────────────
    "S-H1": ("history", "自检①正对照：现役 `ci.yml` ⇒ 绿", "history_selftest()"),
    "S-H2": ("history", "自检①b：正对照真的判到了东西（不是空转）", "history_selftest()"),
    "S-H3": ("history", f"自检②咬住证明：钉死旧 rev `{PINNED_BAD_REV}` ⇒ 判红 + 点名 editor",
             "history_selftest()"),
    "S-H4": ("history", "自检③负对照：撤掉 `queue-lint` 的 `fetch-depth: 0` ⇒ 判红",
             "history_selftest()"),
    "S-H5": ("history", "自检④负对照：新加没全历史的历史判据 job ⇒ 判红", "history_selftest()"),
    "S-H6": ("history", "自检⑤防腐烂：表里点名不存在的脚本 ⇒ 判红", "history_selftest()"),
    "S-H7": ("history", "自检⑥空转闸：声明清空 ⇒ 判红", "history_selftest()"),
    "S-H8": ("history", "自检⑦反向（`ng7`）：删 `run:` 正文只留 step 名 ⇒ 判红并点名 R1b",
             "history_selftest()"),
    "S-H9": ("history", "自检⑧正对照：step 名 + 真跑 ⇒ 不许红（R1b 不误伤）", "history_selftest()"),
}

# **冻结期望**（删一条登记 ⇒ 判红 ✓）：与 `CRITERIA` 逐条对账（`gs2`/`gs3`/`gs4` 判例 (i)）。
REQUIRED_ASSERTIONS = frozenset({
    "W-C1", "W-C1g", "W-C2", "W-C3",
    "W-H1", "W-H2", "W-H3", "W-H4",
    "S-C1", "S-C2", "S-C3", "S-C4", "S-C5", "S-C6a", "S-C6b", "S-C6c",
    "S-H1", "S-H2", "S-H3", "S-H4", "S-H5", "S-H6", "S-H7", "S-H8", "S-H9",
})

# **绿路必须产出的断言**（`gs3` 的判例 (v)：**只删代码里的断言 ⇒ 照样判红** ✗）。
GREEN_PATH_ASSERTIONS = frozenset({"W-C1", "W-C1g", "W-C2", "W-C3",
                                   "W-H1", "W-H2", "W-H3", "W-H4"})

# **自检必须产出的臂**（同一条判例，管自检那一半 ✓）。
SELFTEST_ASSERTIONS = frozenset(i for i in REQUIRED_ASSERTIONS if i.startswith("S-"))

LAYERS = ("coverage", "history")


def registry_problems() -> list[str]:
    """**登记表对账**（验收 ③ 的前两半）：缺登记 / 未登记进期望 / 入口没牙 ✓。"""
    red: list[str] = []
    missing = sorted(REQUIRED_ASSERTIONS - set(CRITERIA))
    if missing:
        red.append(f"**断言缺登记** ✗：{missing} 在冻结期望（`REQUIRED_ASSERTIONS`）里、"
                   f"却**不在**登记表（`CRITERIA`）里 ⇒ 删掉一条断言必须**两处**都改，"
                   f"评审才看得见 ✓")
    extra = sorted(set(CRITERIA) - REQUIRED_ASSERTIONS)
    if extra:
        red.append(f"**断言未登记进期望** ✗：{extra} 在登记表里、却**不在**冻结期望里 ⇒ "
                   f"新加断言要一起改 `REQUIRED_ASSERTIONS`（否则期望会腐烂 ✗）")
    for aid, (layer, _what, entry) in sorted(CRITERIA.items()):
        if layer not in LAYERS:
            red.append(f"**断言层名错** ✗：`{aid}` 的层是 `{layer}`（只能是 {list(LAYERS)}）")
        fn = entry.split("(", 1)[0]
        if not callable(globals().get(fn)):
            red.append(f"**断言没牙** ✗：`{aid}` 的判据入口 `{fn}()` 不存在/不可调用 ⇒ "
                       f"登记了也没有可执行的那一条 ✗")
    return red


def produced_problems(produced: set[str], expected: frozenset, label: str) -> list[str]:
    """**绿跑/自检必须把全部断言产出**（`gs3` 判例 (v)）：缺产出 ⇒ 报**缺的那条** ✓。"""
    missing = sorted(expected - produced)
    if not missing:
        return []
    return [f"**断言缺产出** ✗：{missing} 在冻结期望（{label}）里、但**这一次根本没产出它** ⇒ "
            f"代码里的那条断言被删了/改名了（登记表留着也没用 ✗）"]


def green_run(root: Path, ci_path: Path):
    """真绿跑：两层都在**现役** `ci.yml` 上判一遍 ⇒ `(ids, red, cov, top_level, push_range)`。

    ⚠ `ids` 是**产出的断言 id**（含绿路那几条 ✓ —— 见 `coverage_problems` /
    `history_problems` 的 docstring：**读数不记、产出要记** ✗✓）。
    """
    push_range = _load_push_range()
    top_level = discover_top_level(root)
    doc = load_ci(ci_path)
    filters = push_range.load_filters(ci_path)
    cov, cov_ids = coverage_problems(doc, filters, push_range.glob_to_regex, top_level)
    hist, _checked, hist_ids = history_problems(ci_path.read_text(encoding="utf-8"),
                                                HISTORY_DEPENDENT)
    red = [m for _i, ok, m in cov if not ok] + [m for _i, ok, m in hist if not ok]
    return cov_ids | hist_ids, red, cov, top_level, push_range


def selftest(root: Path, ci_path: Path) -> int:
    """登记表对账 + **绿跑对账** + 两层的正负对照（一条命令跑完 ✓）。"""
    bad = 0
    print("── CI 接线 · selftest（登记表 + 绿跑对账 + 两层正负对照）──")

    # ① 登记表对账
    reg = registry_problems()
    for m in reg:
        print(f"  ✗ {m}")
    bad += len(reg)
    if not reg:
        print(f"  ✓ 登记表：{len(CRITERIA)} 条断言逐条对账（缺 0 · 多 0 · 没牙 0 ✓）")

    # ② 绿跑对账（`gs3` 判例 (v)：真绿跑必须把全部断言产出 ✓）
    green_ids, green_red, _cov, top_level, push_range = green_run(root, ci_path)
    gp = produced_problems(green_ids, GREEN_PATH_ASSERTIONS,
                           f"绿路 {len(GREEN_PATH_ASSERTIONS)} 条")
    for m in gp:
        print(f"  ✗ {m}")
    bad += len(gp)
    if not green_red:
        print(f"  ✓ 绿跑：现役 `ci.yml` 两层都绿（产出 {len(green_ids)} 条断言 id ✓）")
    else:
        # 现役树真红 ⇒ **照样印出来**（不假装 ✓）；红的是判据自己的输入 ⇒ 计数进 bad。
        for m in green_red:
            print(f"  ✗ 现役树判红：{m}")
        bad += len(green_red)

    # ③ 两层的自检（臂的 id 一起对账 ✓）
    produced_arms: set[str] = set()
    cov_rc, _cov_arms = coverage_selftest(ci_path, push_range, top_level, produced_arms)
    bad += 1 if cov_rc else 0
    hist_rc, _hist_arms = history_selftest(produced_arms)
    bad += 1 if hist_rc else 0
    sp = produced_problems(produced_arms, SELFTEST_ASSERTIONS,
                           f"自检 {len(SELFTEST_ASSERTIONS)} 条臂")
    for m in sp:
        print(f"  ✗ {m}")
    bad += len(sp)

    if bad:
        print(f"\n✗ CI 接线 selftest：**{bad} 条红** ⇒ 判据咬不住 / 登记表与代码漂移 ✗",
              file=sys.stderr)
        return 1
    print(f"\n✓ CI 接线 selftest 全过：登记表 {len(CRITERIA)}/{len(REQUIRED_ASSERTIONS)} 逐条对账 + "
          f"绿跑产出 {len(green_ids)} 条 + 自检臂 {len(produced_arms)} 条 ✓")
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(
        description="「CI 的某条前提在不在」的类守卫（一本登记表 + 一个驱动器 · 两层）")
    ap.add_argument("--layer", choices=(*LAYERS, "both"), default="both",
                    help="只跑一层（默认两层都跑 ✓）")
    ap.add_argument("--selftest", action="store_true",
                    help="登记表对账 + 绿跑对账 + 两层正负对照（含真旧 rev ✓）")
    ap.add_argument("--json", action="store_true", help="机器可读（agent 用 ✓）")
    ap.add_argument("--root", default=None,
                    help="仓库根（层① 的顶层条目与默认 `--ci` 都从这里取；默认 = 本脚本所在仓）")
    ap.add_argument("--ci", default=None,
                    help="工作流文件（默认 `<root>/.github/workflows/ci.yml`）")
    ap.add_argument("--ci-yml-rev", default=None,
                    help="层② 改判 git 里**某一版** ci.yml（咬住证明打在钉死的旧 rev 上 ✓）")
    args = ap.parse_args(argv)

    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")
        except (AttributeError, ValueError, OSError):
            pass

    root = Path(args.root).resolve() if args.root else ROOT
    ci = Path(args.ci).resolve() if args.ci else root / DEFAULT_CI_REL
    layer = args.layer

    # ⚠ `--ci-yml-rev` 只对层② 有意义 ⇒ 显式点名层① 就是**判不了**（不假装判过 ✗）。
    if args.ci_yml_rev and layer == "coverage":
        print("error: `--ci-yml-rev` 只服务层②（`history`）⇒ 对层① 判不了"
              "（**不假装判过** ✗）：请用 `--layer history --ci-yml-rev <rev>` 或不带 `--layer` ✓",
              file=sys.stderr)
        return 3
    if args.ci_yml_rev and layer == "both":
        layer = "history"

    try:
        if args.selftest:
            return selftest(root, ci)

        codes: dict[str, int] = {}
        produced: dict[str, list[str]] = {}
        if layer in ("coverage", "both"):
            push_range = _load_push_range()
            top_level = discover_top_level(root)
            doc = load_ci(ci)
            filters = push_range.load_filters(ci)
            cov, cov_ids = coverage_problems(doc, filters, push_range.glob_to_regex, top_level)
            produced["coverage"] = sorted(cov_ids)
            codes["coverage"] = report_coverage(cov, top_level)
        if layer in ("history", "both"):
            if args.ci_yml_rev:
                text, label = read_ci_at_rev(args.ci_yml_rev), f"git rev `{args.ci_yml_rev}`"
            else:
                path = Path(ci)
                if not path.exists():
                    raise EnvError(f"`{path}` 不在 ⇒ 判不了接线（**不假装判过** ✗）", code=3)
                text, label = path.read_text(encoding="utf-8"), path.name
            rc, ids = run_history(text, HISTORY_DEPENDENT, label)
            produced["history"] = sorted(ids)
            codes["history"] = rc
        code = max(codes.values()) if codes else 0
        if args.json:
            print(json.dumps({
                "schema": "soko.ci-wiring/1",
                "layer": layer,
                "layers": {k: {"exit": v, "assertions": produced.get(k, [])}
                           for k, v in codes.items()},
                "exit": code,
            }, ensure_ascii=False))
        return code
    except EnvError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return exc.code


if __name__ == "__main__":
    sys.exit(main())
