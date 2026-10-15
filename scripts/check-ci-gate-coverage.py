#!/usr/bin/env python3
"""**「按改动范围决定跑不跑」的类守卫** —— 洞 `PATHS-FILTER-CLASS`（2026-10-15）。

## 为什么有这个脚本（"修实例不修类"的解药）

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

## 类不变量（两条，都能机械判）

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
  [`DATA_DEPENDENCIES`] 里（每条带理由 ✓、评审可见 ✓），由本判据核对覆盖 ✓；
  表里点名的 job 不存在或不再被门控 ⇒ **判红**（防表自己腐烂 ✗）。

⚠ 三处的**语义**都按 **job 级析取**判（`if: A || B` ⇒ A、B 任一命中即可 ✓）——
要求"每个组都要有"会制造假红 ✗（实测：`.github/workflows/**` 只在 `rust` 组里，
而每个门控 job 的 `if` 都含 `rust` ⇒ 已经够了 ✓）。

## 判据

* `--check`（默认）：对**真** `ci.yml` 跑 R1 + R2 + R3，逐条印读数；违反 ⇒ exit 1。
* `--selftest`：合成夹具上**正负对照**都要咬住 ——
  ① 现役 `ci.yml` 的**逐字副本**必须绿 ✓（**正对照**：不许制造假红 ✗）；
  ② 把 `scripts/**` 从 `rust` 过滤器里删掉 ⇒ **必须判红**（实例①复现 ✓）；
  ③ 把 `.github/workflows/**` 从某个门控组删掉 ⇒ **必须判红**（实例②复现 ✓）；
  ④ 把 `docs/gaps/**` 删掉 ⇒ **必须判红**（实例③复现 ✓）；
  ⑤ 加一个消费 `editor/**` 却只被 `rust` 门控的 job ⇒ **必须判红** ✓。
  ⇒ **咬得住历史 bug 的守卫才算数**（`AGENTS.md` 验证设计纪律 3 ✓）。

## 与 `scripts/ci-push-range.py` 的分工（不重复实现 ✗）

* 那边管**范围是从哪算的**（实例④：基点 = 上一个已验证点）；
* 本脚本管**范围里有没有把 job 要读的东西藏掉**（实例①②③）。
* 过滤器的**唯一一份**实现仍只在 `ci.yml` 的 `filters:` 里 ✓ ——
  本脚本**读它**（`load_filters`，与 `ci-push-range.py` 同一段代码，
  通过 `importlib` 复用、不另抄一份 ✗）。

退出码：0 = 通过 / 1 = 判红 / 2 = 用法或环境错（与 `ci-push-range.py` 同口径 ✓）。
"""
import argparse
import importlib.util
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_CI = HERE.parent / ".github" / "workflows" / "ci.yml"

# 仓库顶层条目（`git ls-files` 的一级名）：判"这个 token 像不像仓库路径"用它 ✓。
# 不写死全部 —— 只用来**收窄** token 识别，避免把 `python3` / `--json` 当路径 ✗。
TOP_LEVEL = (
    ".agents", ".devcontainer", ".github", ".opencode", ".vscode",
    "course", "courses", "crates", "docs", "dsh", "editor", "examples",
    "fuzz", "prelude", "reference", "scripts", "site", "skills",
    "AGENTS.md", "Cargo.lock", "Cargo.toml", "NOTICE.md", "README.md",
    "REQUIREMENTS.md", "ROADMAP.md", "STATUS.md", "opencode.json",
    "playground.sokonanoda",
)

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


class EnvError(Exception):
    """环境/用法错（exit 2）—— 与 `ci-push-range.py` 同口径 ✓。"""


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
    try:
        import yaml
    except ImportError:  # pragma: no cover - 环境问题
        raise EnvError("需要 pyyaml（与 scripts/ci-yml-lint.py 同款依赖）✗")
    try:
        return yaml.safe_load(Path(path).read_text(encoding="utf-8")) or {}
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


def looks_like_repo_path(token):
    """像不像**仓库相对路径**：顶层名打头（含 `/` 或以已知顶层文件打头）。"""
    if token.startswith("-") or "://" in token:
        return False
    head = token.split("/")[0]
    if head not in TOP_LEVEL:
        return False
    # 纯文件名（无 `/`）只认已知的顶层文件；顶层**目录**要带 `/` 才算路径。
    if "/" not in token and "." in head and head in TOP_LEVEL:
        return True
    return "/" in token


def consumed_paths(job):
    """job 消费的**仓库路径**（去重、排序；只保留像路径的 token ✓）。"""
    out = set()
    for token in TOKEN_RE.findall(job_text(job)):
        token = token.rstrip(".,;:'\"")
        if looks_like_repo_path(token):
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


def check(doc, filters, glob_to_regex):
    """R1 + R2 + R3。返回 `[(ok, 说明)]`（与 `ci-push-range.py --check-wiring` 同形 ✓）。"""
    jobs = doc.get("jobs") or {}
    res = []

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
        for token in consumed_paths(job) + [".github/workflows/ci.yml"]:
            hits = classify(token, filters, glob_to_regex)
            if not hits:
                res.append((
                    False,
                    f"R1 `{jid}` 消费 `{token}`，但它**不在任何过滤器组里** ✗ "
                    f"⇒ 那个路径的改动**不会**让这个 job 跑 ⇒ 改了没人验 ✗"
                    f"（门控标志 {sorted(flags)}）",
                ))
            elif not (hits & flags):
                res.append((
                    False,
                    f"R1 `{jid}` 消费 `{token}`，但它只落在 {sorted(hits)} 组里，"
                    f"而门控这个 job 的是 {sorted(flags)} ✗ "
                    f"⇒ 改那个路径**不会**让这个 job 跑 ✗",
                ))
    res.append((gated_jobs > 0, f"R1 有 {gated_jobs} 个 job 挂在 `changes` 标志上（实测）"))

    # ── R2 门控标志 ↔ 过滤器组 一一对应 ────────────────────────────────
    gating_groups = set()
    for job in jobs.values():
        if isinstance(job, dict):
            gating_groups |= gated_flags(job)
    res.append((bool(gating_groups), f"R2 门控了 job 的过滤器组：{sorted(gating_groups)}"))
    for name in sorted(gating_groups):
        res.append((
            name in filters,
            f"R2 门控标志 `{name}` 在 `filters:` 里有对应组（实测 {sorted(filters)}）",
        ))

    # ── R3 数据依赖（声明式，见 `DATA_DEPENDENCIES` 的说明）────────────
    for jid, path, why in DATA_DEPENDENCIES:
        job = jobs.get(jid)
        if not isinstance(job, dict):
            res.append((False, f"R3 表里点名了 job `{jid}`，但 `ci.yml` 里没有它 ✗（表腐烂了）"))
            continue
        flags = gated_flags(job)
        if not flags:
            res.append((
                False,
                f"R3 表里点名了 job `{jid}`，但它**不再被 `changes` 门控** ✗ "
                f"⇒ 这条声明已经没有意义（表腐烂了）",
            ))
            continue
        hits = classify(path, filters, glob_to_regex)
        res.append((
            bool(hits & flags),
            f"R3 `{jid}` 读 `{path}`（{why}）⇒ 必须被门控它的过滤器命中"
            f"（实测落 {sorted(hits)}，门控标志 {sorted(flags)}）",
        ))
    return res


def report(res):
    bad = [msg for ok, msg in res if not ok]
    for ok, msg in res:
        if not ok:
            print(f"✗ {msg}")
    print(
        f"  ⓘ 判据 {len(res)} 条，违反 {len(bad)} 条 —— "
        f"口径：R1 job 消费的每条仓库路径必须被门控它的过滤器命中 · "
        f"R2 标志↔组一一对应 · R3 声明的数据依赖必须被门控 ✓"
    )
    return 1 if bad else 0


# ─────────────────────────── 自检（--selftest）───────────────────────────

def _rewrite_filters(text, drop):
    """从 `filters:` 内联块里删掉一行（`drop` = 要删的 pattern 字面量，含引号）。"""
    out, dropped = [], 0
    for line in text.splitlines():
        if drop in line and line.strip().startswith("-"):
            dropped += 1
            continue
        out.append(line)
    return "\n".join(out) + "\n", dropped


def _add_ungated_job(text):
    """加一个**消费 `editor/**` 却只被 `rust` 门控**的 job（实例①的换皮 ✓）。"""
    block = (
        "\n  selftest-ungated:\n"
        "    needs: changes\n"
        "    if: needs.changes.outputs.rust == 'true'\n"
        "    runs-on: ubuntu-24.04\n"
        "    steps:\n"
        "      - run: node editor/vscode/test-extension-host.js\n"
    )
    return text.rstrip("\n") + "\n" + block


def selftest(ci_path, push_range):
    """正负对照（见模块文档的判据表 ✓）。"""
    base_text = Path(ci_path).read_text(encoding="utf-8")
    results = []

    def run(text, label, expect_red):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "ci.yml"
            p.write_text(text, encoding="utf-8")
            doc = load_ci(p)
            filters = push_range.load_filters(p)
            res = check(doc, filters, push_range.glob_to_regex)
            reds = [msg for ok, msg in res if not ok]
            ok = bool(reds) == expect_red
            results.append((ok, label, reds[:2]))

    # ① 正对照：现役 ci.yml 的逐字副本必须绿 ✓（不许制造假红 ✗）
    run(base_text, "① 正对照：现役 ci.yml 逐字副本 ⇒ 必须**绿**", expect_red=False)
    # ②③④ 负对照：把历史三次事故的修法**撤掉** ⇒ 必须判红 ✓
    for drop, label in [
        ('"scripts/**"', "② 撤掉 `scripts/**`（实例①）⇒ 必须**红**"),
        ('".github/workflows/**"', "③ 撤掉 `.github/workflows/**`（实例②）⇒ 必须**红**"),
        ('"docs/gaps/**"', "④ 撤掉 `docs/gaps/**`（实例③）⇒ 必须**红**"),
    ]:
        text, dropped = _rewrite_filters(base_text, drop)
        if dropped == 0:
            results.append((False, f"{label} —— **夹具没改到东西**（`{drop}` 没找到）✗", []))
            continue
        run(text, f"{label}（实测删了 {dropped} 行）", expect_red=True)
    # ⑤ 换皮：新加一个消费 `editor/**` 却只被 `rust` 门控的 job ⇒ 必须判红 ✓
    run(_add_ungated_job(base_text), "⑤ 新加消费 `editor/**` 却只被 `rust` 门控的 job ⇒ 必须**红**",
        expect_red=True)

    bad = 0
    for ok, label, reds in results:
        print(f"{'✓' if ok else '✗'} {label}")
        if not ok:
            bad += 1
            for msg in reds:
                print(f"    {msg}")
    print(f"  ⓘ 自检 {len(results)} 条，未咬住 {bad} 条 —— 口径：正对照必须绿、四个负对照必须红 ✓")
    return 1 if bad else 0


def main(argv=None):
    parser = argparse.ArgumentParser(description="「按改动范围决定跑不跑」的类守卫")
    parser.add_argument("--ci", default=str(DEFAULT_CI), help="工作流文件（默认 ci.yml）")
    parser.add_argument("--selftest", action="store_true", help="正负对照自检")
    parser.add_argument("--check", action="store_true", help="对真 ci.yml 跑 R1 + R2 + R3（默认动作）")
    args = parser.parse_args(argv)
    try:
        push_range = _load_push_range()
        if args.selftest:
            return selftest(args.ci, push_range)
        doc = load_ci(args.ci)
        filters = push_range.load_filters(args.ci)
        return report(check(doc, filters, push_range.glob_to_regex))
    except EnvError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
