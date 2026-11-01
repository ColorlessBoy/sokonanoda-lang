#!/usr/bin/env python3
"""**strict 文本通道棘轮**（P3 第一刀，2026-11-01）—— 三个文本 oracle 入口**只许减、不许增**。

## 第一性原理（为什么这条守卫存在）

strict 是**参数绑定策略**，不是第二套 type oracle；正确关系是 `strict ≤ loose`
（`REQUIREMENTS.md` §9.39「禁止 print→parse」· `reports/2026-11-01-p3-spec.md`）。
应用路径解隐式实参时曾经走文本往返：`operand_type_expr_explicit`（全显式 pp 文本 →
`parse` 回项）、`infer_type_text`（类型文本）、以及 `judge.rs` 的 `pp=explicit` 缓存键。
**根上的修法是删除这些文本往返**（typed telescope + 元变量），不是给它们加缓存。

本脚本是**棘轮**：基线 = 本分支实测的已知调用点（逐条已归类，见
`reports/2026-11-01-p3-inventory.md`）；**任何新增 ⇒ 判红**。数字只许降，降到 0 后收口。

## 两把尺（**别混**，2026-11-01 实测踩到的坑）

| 尺 | 口径 | 本分支实测 | 注释 |
|---|---|---:|---|
| **提及**（mentions） | 裸名字出现在**代码行**（跳过 `//` / `///`） | **4 / 25 / 1 = 30** | 含定义、`use` 导入、兄弟函数 `infer_type_text_inplace` / `_at` / `_at_peeled` |
| **调用**（calls） | `name(` 的精确调用语法 | **2 / 7 / 1 = 10** | 棘轮的**硬**口径（定义行是 `name<'a>(`，不算调用） |

P3 实施单 §1 表里的 `4 / 25 / 1` 是**提及**（= 30 个盘点点）；旧分支的 `2 / 6 / 1` 是**调用**。
两者差 = 1 个定义 + 1 个 `use` + 20 条兄弟函数/内部调用 —— 逐条列在归类表里。

## 三条判据（可执行）

    python3 scripts/audit-strict-paths.py --self-test    # 反向验证：故意造的违规必须被抓到
    python3 scripts/audit-strict-paths.py --strict-only  # **strict 专属 = 0**（P3 的验收线）
    python3 scripts/audit-strict-paths.py --all --json   # 30 个盘点点的逐点清单
    python3 scripts/audit-strict-paths.py --strict-only --root <dir>  # 咬住证明（扫别的树）

## 归类（每条都在 `SITES` 里，键 = 文件 + 所在函数 + 代码片段）

* `shared` —— **共用**：loose 档也跑（本语料两档都实测 > 0，或静态证据见 `why`）；
* `loose` —— **loose 专属**：本语料 loose 跑、strict 0（**不是** strict 专属 ⇒ 不在删除名单）；
* `strict` —— **strict 专属**：只在 strict 档跑（本分支 **0 条**）；
* `def` / `import` / `internal` —— 通道定义、`use` 导入、兄弟函数的内部接线（不是判定调用点）。

`--strict-only` = `strict` 条数 + **未分类**条数（**fail-closed**：证不出共用的新点一律按
strict 算 ⇒ 新加一行文本调用**必然**判红，不用等谁记得改表）。`compile/kernel_app*.rs`
是**整模块 strict 专属**（`kernel_app::effective(dialect) == dialect.strict()`）⇒ 落在那里的
文本调用**直接按 `strict` 计**（结构性判据，见 `STRICT_ONLY_FILES`）。

退出码：0 = 干净 / 1 = 新增或越基线 / 2 = 扫描错误。
"""

from __future__ import annotations

import json
import re
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
ROOT = REPO / "crates" / "front" / "src"

# 三个文本 oracle 入口（**裸名字** = 提及；`name(` = 调用）
FAMILIES = {
    "operand_type_expr_explicit": re.compile(r"(?<!\w)operand_type_expr_explicit"),
    "infer_type_text": re.compile(r"(?<!\w)infer_type_text"),
    "pp=explicit": re.compile(r"pp=explicit"),
}
CALLS = {
    "operand_type_expr_explicit": re.compile(r"(?<!\w)operand_type_expr_explicit\s*\("),
    "infer_type_text": re.compile(r"(?<!\w)infer_type_text\s*\("),
    "pp=explicit": re.compile(r'pp=explicit"'),
}

# **实测基线（2026-11-01，HEAD = `a98af66b`，S6 已并入）** —— 不许拍脑袋：
#   提及 4/25/1（= P3 实施单 §1 的 30 个盘点点）
#   调用 2/7/1（= 旧脚本的计量口径；旧分支写 2/6/1，那棵树与这棵不同）
# 归类表：`reports/2026-11-01-p3-inventory.md`（30 行逐点 + 判据）。
MENTION_BASELINE = {
    "operand_type_expr_explicit": 4,
    "infer_type_text": 25,
    "pp=explicit": 1,
}
CALL_BASELINE = {
    "operand_type_expr_explicit": 2,
    "infer_type_text": 7,
    "pp=explicit": 1,
}

# **结构性 strict 专属**：`kernel_app*.rs` 的每条入口都由
# `kernel_app::effective(dialect)`（= `dialect.strict()`，P2-12 S6 起）守卫 ⇒
# 落在这里的文本调用只可能在 strict 档跑到（`compile_tests` 的 `TEST_OVERRIDE` 只给单测）。
STRICT_ONLY_FILES = re.compile(r"(^|/)kernel_app[^/]*\.rs$")

# 30 个盘点点（`--all` 逐点输出；`--strict-only` 只数 `strict` + 未分类）。
# 字段：file（相对 `crates/front/src`）· fam（三个入口之一）· fn（所在函数）·
#       snippet（代码行里的判别子串）· cls · why（归类判据）· nth（同键第几条，默认 0）。
S = [
    # ── operand_type_expr_explicit（4 条提及；2 条真调用）──────────────────────
    dict(file="compile/elab.rs", fam="operand_type_expr_explicit", fn="operand_type_expr_explicit",
         snippet="pub(crate) fn operand_type_expr_explicit<'a>(", cls="def",
         why="通道定义本体（全显式 pp 版第二候选），不是调用点"),
    dict(file="compile/elab.rs", fam="operand_type_expr_explicit", fn="try_implicit_application_with",
         snippet="operand_type_expr_explicit(ctx, scope, a, env)", cls="shared",
         why="老路 `alt_type` 第二候选（`second_candidate_enabled` 门，与 dialect 无关）；"
             "实测 loose=654 / strict=0（strict 只在回落后才可能进这里）"),
    dict(file="compile/named_arg.rs", fam="operand_type_expr_explicit", fn="<top>",
         snippet="elab_expr, operand_type_expr_explicit,", cls="import",
         why="`use` 导入（无运行时行为）"),
    dict(file="compile/named_arg.rs", fam="operand_type_expr_explicit", fn="apply",
         snippet="operand_type_expr_explicit(ctx, scope, a, env)", cls="shared",
         why="具名实参 `apply` 的第二候选（具名实参两档都收 ⇒ 不按 dialect 分叉）；"
             "语料未触达（课程无具名实参），静态判据见 `named_arg.rs:152` 的 `alt_type`"),
    # ── infer_type_text 家族（25 条提及；7 条 `infer_type_text(` 真调用）────────
    dict(file="apply_probe.rs", fam="infer_type_text", fn="level_hint_of_inplace",
         snippet="let text = crate::compile::elab::infer_type_text_inplace(", cls="shared",
         why="tactic `by` 的层级提示（`by.rs:353`）；实测 54/54"),
    dict(file="apply_probe.rs", fam="infer_type_text", fn="level_hint_of_inplace",
         snippet="crate::compile::elab::infer_type_text_inplace(", cls="shared",
         why="同上（第二个问法）；实测 52/52"),
    dict(file="by.rs", fam="infer_type_text", fn="restore_universe_levels",
         snippet="crate::compile::elab::infer_type_text_inplace(", cls="shared",
         why="tactic 重建脊的层级恢复；实测 52/52"),
    dict(file="by.rs", fam="infer_type_text", fn="apply_tactic",
         snippet="crate::compile::elab::infer_type_text_inplace(", cls="shared",
         why="`apply`/`cases` 的期望类型就地路；实测 266/266"),
    dict(file="by.rs", fam="infer_type_text", fn="apply_tactic",
         snippet="crate::compile::elab::infer_type_text_inplace_expected(", cls="shared",
         why="tactic 路径（与 dialect 无关）；语料未触达，静态：`by.rs` 的 `cases` 走同一条"),
    dict(file="by.rs", fam="infer_type_text", fn="apply_tactic",
         snippet="crate::compile::elab::infer_type_text_inplace_expected(", cls="shared",
         why="同上（另一处 `cases` 分支）", nth=1),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="application_arg_expected",
         snippet="let ty_text = infer_type_text(ctx, scope, head, env, Some(univ), false)?;",
         cls="shared", why="应用实参期望类型（老路续跑，两档都进）；实测 loose=5035 / strict=4937"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="universe_level_text_of_operands",
         snippet="let Some(ty_text) = infer_type_text(", cls="shared",
         why="记法层级推断（`notation_levels.rs` 与 recursion 都问）；实测 4304/3857"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="universe_level_text_of_operands",
         snippet="infer_type_text_inplace(", cls="shared",
         why="同上第二问（就地版）；实测 4304/3857"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="inplace_render_type",
         snippet="ef.infer_type_text_at_peeled(", cls="shared",
         why="`judge_render_type_inplace` → tactic（`by.rs:857/925`），与 dialect 无关"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_type_text_inplace",
         snippet="pub(crate) fn infer_type_text_inplace<'a>(", cls="def", why="就地通道定义"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_type_text_inplace",
         snippet="infer_type_text_inplace_expected(env, ctx, binder_srcs, operand, binder_count, univ, None)",
         cls="internal", why="就地通道内部的接线（带期望类型版），由上面各入口驱动"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_type_text_inplace_expected",
         snippet="pub(crate) fn infer_type_text_inplace_expected<'a>(", cls="def", why="就地通道定义"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_type_text_inplace_expected",
         snippet="quiet_catch(|| ef.infer_type_text_at(", cls="internal",
         why="就地通道内部的 `at` 变体（同一入口）"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_type_text",
         snippet="pub(crate) fn infer_type_text<'a>(", cls="def", why="慢路通道定义"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_type_text",
         snippet="match infer_type_text_inplace(", cls="internal",
         why="慢路内部的就地分支（`InplaceMode::On`）"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_type_text",
         snippet="let new = infer_type_text_inplace(", cls="internal",
         why="慢路内部的影子分支（`InplaceMode::Shadow`，只做比对）"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="operand_type_expr",
         snippet="let text = infer_type_text(ctx, scope, operand, env, None, explicit)?;", cls="shared",
         why="操作数类型（应用/记法/pp 共用）；实测 loose=29262 / strict=101"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="operand_type_expr_explicit",
         snippet="infer_type_text(ctx, scope, operand, env, None, true)", cls="shared",
         why="全显式 pp 通道本体（只由上一格 `operand_type_expr_explicit` 的调用者驱动）"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_expected_level",
         snippet="match infer_type_text_inplace(env, ctx, &srcs, expected_src, needed.len(), None) {",
         cls="shared", why="match/期望类型层级（两档都问）；实测 5/5"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_expected_level",
         snippet="infer_type_text_inplace(env, ctx, &srcs, expected_src, needed.len(), None)",
         cls="shared", why="同上（影子/慢路分支）；实测 5/5"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_recursor_universes",
         snippet="infer_type_text(ctx, scope, expected, Some(&mut env), None, false)", cls="shared",
         why="递归子消去层级（`Nat.rec`/`And.rec` 一类，两档都走）；实测 6/6"),
    dict(file="compile/elab.rs", fam="infer_type_text", fn="infer_const_universes_with",
         snippet="let ty_text = infer_type_text(ctx, scope, first, Some(&mut env), None, false);",
         cls="shared", why="常量宇宙层推断（应用臂两档都进）；实测 62/5"),
    dict(file="compile/named_arg.rs", fam="infer_type_text", fn="next_arg_expected",
         snippet="let ty_text = crate::compile::elab::infer_type_text(", cls="shared",
         why="`@` 脊与 `apply` 共用的「下一实参期望类型」（全显式 pp 档）；实测 1094/906"),
    dict(file="judge.rs", fam="pp=explicit", fn="judge_infer_cache_key",
         snippet='"pp=explicit"', cls="shared",
         why="判定缓存键的 pp 标记（`explicit_pp_active()`，与 dialect 无关）；实测 13401/1578"),
    dict(file="judge.rs", fam="infer_type_text", fn="judge_infer_inplace_with_explicit",
         snippet="crate::compile::elab::infer_type_text_inplace(", cls="shared",
         why="全显式就地判定的受控入口（由 `infer_type_text`/tactic 调用）；实测 654/0（随调用者）"),
]


def enclosing_fn(lines: list[str], idx: int) -> str:
    """`idx`（0 基）所在函数名；找不到 ⇒ `<top>`。"""
    pat = re.compile(r"\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)")
    for j in range(idx, -1, -1):
        m = pat.match(lines[j])
        if m:
            return m.group(1)
    return "<top>"


def scan(root: Path) -> list[dict]:
    """扫**代码行**（跳过 `//` 注释与 `///` 文档注释）。

    每条命中 = `{file, line, fn, fam, call, text}`；`file` 相对 `root`。
    """
    hits: list[dict] = []
    for f in sorted(root.rglob("*.rs")):
        try:
            lines = f.read_text(encoding="utf-8").splitlines()
        except OSError as e:
            raise RuntimeError(f"读取失败：{f}: {e}") from e
        rel = f.relative_to(root).as_posix()
        for i, line in enumerate(lines):
            if line.lstrip().startswith("//"):
                continue
            for fam, pat in FAMILIES.items():
                if not pat.search(line):
                    continue
                hits.append(
                    {
                        "file": rel,
                        "line": i + 1,
                        "fn": enclosing_fn(lines, i),
                        "fam": fam,
                        "call": bool(CALLS[fam].search(line)),
                        "text": line.strip(),
                    }
                )
    return hits


def classify(hits: list[dict], sites: list[dict], strict_files: re.Pattern | None) -> tuple[list[dict], list[dict]]:
    """把每条命中映射到 `SITES` 的一条 ⇒ (已分类, 未分类)。"""
    # `nth` 按 **(文件, 入口, 函数, 片段)** 这一把键计数（同键可有多条：
    # `by.rs::apply_tactic` 的两个 `infer_type_text_inplace_expected(` 行文本相同）。
    seen: dict[tuple, int] = {}
    classified: list[dict] = []
    unknown: list[dict] = []
    for h in hits:
        site = None
        for s in sites:
            if s["file"] != h["file"] or s["fam"] != h["fam"] or s["fn"] != h["fn"]:
                continue
            if s["snippet"] not in h["text"]:
                continue
            key = (s["file"], s["fam"], s["fn"], s["snippet"])
            n = seen.get(key, 0)
            if n != s.get("nth", 0):
                continue
            seen[key] = n + 1
            site = s
            break
        row = dict(h)
        if site is not None:
            row["cls"] = site["cls"]
            row["why"] = site["why"]
        elif strict_files is not None and strict_files.search(h["file"]):
            row["cls"] = "strict"
            row["why"] = (
                f"落在 strict 专属模块（{h['file']}）：`kernel_app*` 的入口由 "
                "`kernel_app::effective(dialect) == dialect.strict()` 守卫 ⇒ 只在 strict 档跑"
            )
        else:
            row["cls"] = "unclassified"
            row["why"] = "未归类（新点 ⇒ fail-closed，按 strict 计）"
        # **未分类**是独立通道（它既算 strict_only，也要在 `--all` 里被点名）；
        # 结构性 strict 点仍是「已分类」（`cls = strict`）。
        (unknown if row["cls"] == "unclassified" else classified).append(row)
    return classified, unknown


def tally(rows: list[dict]) -> tuple[dict, dict]:
    mentions = {k: 0 for k in FAMILIES}
    calls = {k: 0 for k in FAMILIES}
    for r in rows:
        mentions[r["fam"]] += 1
        if r["call"]:
            calls[r["fam"]] += 1
    return mentions, calls


def evaluate(
    root: Path,
    sites: list[dict],
    mention_baseline: dict,
    call_baseline: dict,
    strict_files: re.Pattern | None,
) -> dict:
    hits = scan(root)
    rows, unknown = classify(hits, sites, strict_files)
    # ⚠ 计数必须含**未分类**那批（否则新点会既不进基线比较、也不进 strict_only ⇒ 漏判）
    mentions, calls = tally(rows + unknown)
    strict_only = sum(1 for r in rows if r["cls"] == "strict") + len(unknown)
    over_mentions = {k: v for k, v in mentions.items() if v > mention_baseline.get(k, 0)}
    over_calls = {k: v for k, v in calls.items() if v > call_baseline.get(k, 0)}
    # 两把判据**分开**（`--all` = 全量棘轮；`--strict-only` = P3 的验收线）：
    #   * `--all`：提及/调用越基线、未分类、strict 专属 > 0 ⇒ 红；
    #   * `--strict-only`：**只**看 strict 专属 + 未分类（fail-closed：新点证不出共用 ⇒ 按 strict 算）。
    ok_all = not over_mentions and not over_calls and not unknown and strict_only == 0
    ok_strict = strict_only == 0 and not unknown
    return {
        "ok": ok_all,
        "ok_all": ok_all,
        "ok_strict_only": ok_strict,
        "root": str(root),
        "mentions": mentions,
        "mentions_baseline": mention_baseline,
        "calls": calls,
        "calls_baseline": call_baseline,
        "strict_only": strict_only,
        "over_mentions": over_mentions,
        "over_calls": over_calls,
        "unknown": unknown,
        "sites": rows,
    }


def report(result: dict, mode: str) -> int:
    if "--json" in sys.argv or mode == "json":
        print(json.dumps(result, ensure_ascii=False))
        return 0 if result["ok"] else 1
    m, mb = result["mentions"], result["mentions_baseline"]
    c, cb = result["calls"], result["calls_baseline"]
    print(
        "audit-strict-paths: 提及 "
        + "/".join(str(m[k]) for k in FAMILIES)
        + f"（基线 {'/'.join(str(mb[k]) for k in FAMILIES)}） · 调用 "
        + "/".join(str(c[k]) for k in FAMILIES)
        + f"（基线 {'/'.join(str(cb[k]) for k in FAMILIES)}） · strict 专属 {result['strict_only']}"
    )
    if result["over_mentions"] or result["over_calls"]:
        mb = result["mentions_baseline"]
        cb = result["calls_baseline"]
        print(
            "✗ 新增文本通道：提及 "
            + ", ".join(f"{k} +{v - mb.get(k, 0)}" for k, v in result["over_mentions"].items())
            + "；调用 "
            + ", ".join(f"{k} +{v - cb.get(k, 0)}" for k, v in result["over_calls"].items()),
            file=sys.stderr,
        )
        print(
            "  根上的修法是删除 print→parse（typed telescope + 元变量），不是加缓存。\n"
            "  依据：REQUIREMENTS.md §9.39 · reports/2026-11-01-p3-inventory.md",
            file=sys.stderr,
        )
    for r in result["unknown"]:
        print(
            f"✗ 未归类文本点 {r['file']}:{r['line']}（{r['fn']} · {r['fam']}）"
            " —— 新点必须先归类（fail-closed 按 strict 计）",
            file=sys.stderr,
        )
    if result["ok"]:
        print("✓ 文本通道只减不增（strict 专属 = 0）")
    else:
        print(
            f"✗ strict 专属 {result['strict_only']} 条 · 未分类 {len(result['unknown'])} 条"
            f"（模式 {result['mode']}）",
            file=sys.stderr,
        )
    return 0 if result["ok"] else 1


def self_test() -> int:
    """**反向验证**：故意造的违规必须被抓到（三个方向 + 一条不误伤）。"""
    ok = True

    def bad(msg: str) -> None:
        nonlocal ok
        ok = False
        print(f"strict-paths self-test ✗ {msg}", file=sys.stderr)

    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        (root / "shared.rs").write_text(
            "fn shared_fn() {\n"
            "    let _ = infer_type_text(ctx, scope, a, env, None, false);\n"
            "}\n",
            encoding="utf-8",
        )
        (root / "kernel_app.rs").write_text(
            "fn kernel_side() {\n"
            "    let _ = operand_type_expr_explicit(ctx, scope, a, env);\n"
            "}\n",
            encoding="utf-8",
        )
        (root / "mystery.rs").write_text(
            "fn mystery() {\n"
            "    // infer_type_text( —— 注释不该算\n"
            "    let _ = infer_type_text_inplace(x);\n"
            "}\n",
            encoding="utf-8",
        )
        sites = [
            dict(file="shared.rs", fam="infer_type_text", fn="shared_fn",
                 snippet="infer_type_text(ctx, scope, a, env, None, false)", cls="shared", why="t"),
        ]
        strict_files = re.compile(r"(^|/)kernel_app[^/]*\.rs$")
        base_m = {"operand_type_expr_explicit": 1, "infer_type_text": 1, "pp=explicit": 0}
        base_c = {"operand_type_expr_explicit": 1, "infer_type_text": 1, "pp=explicit": 0}

        r = evaluate(root, sites, base_m, base_c, strict_files)
        # `mystery.rs` 的 `infer_type_text_inplace(` 是**代码行**（是真提及）；注释那行不算。
        if r["mentions"] != {"operand_type_expr_explicit": 1, "infer_type_text": 2, "pp=explicit": 0}:
            bad(f"提及计数不对：{r['mentions']}")
        if not r["strict_only"]:
            bad("kernel_app.rs 里的文本调用必须按 strict 计")
        if any(u["file"] == "kernel_app.rs" for u in r["unknown"]):
            bad("`kernel_app.rs` 的命中必须归类成 strict（不是未分类）")
        if not any(x["file"] == "kernel_app.rs" and x["cls"] == "strict" for x in r["sites"]):
            bad("`kernel_app.rs` 的命中没有落到 strict")
        if any("// infer_type_text" in x["text"] for x in r["sites"] + r["unknown"]):
            bad("注释行被误计")

        # ① 干净：只有共享点 + 已声明的 strict 点 ⇒ 把 strict 那条从基线里去掉后要**绿**
        (root / "kernel_app.rs").write_text("fn kernel_side() {\n    let _ = 1;\n}\n", encoding="utf-8")
        (root / "mystery.rs").write_text("fn mystery() {\n    let _ = 1;\n}\n", encoding="utf-8")
        clean = evaluate(root, sites, base_m, base_c, strict_files)
        if not clean["ok"] or clean["strict_only"] != 0:
            bad(f"干净树必须绿：{clean}")

        # ② 往**共享点**加一行 ⇒ 提及/调用越基线 ⇒ 红（咬住「只许降」）
        (root / "shared.rs").write_text(
            "fn shared_fn() {\n"
            "    let _ = infer_type_text(ctx, scope, a, env, None, false);\n"
            "    let _ = infer_type_text(ctx, scope, b, env, None, false);\n"
            "}\n",
            encoding="utf-8",
        )
        r = evaluate(root, sites, base_m, base_c, strict_files)
        if r["ok"] or not r["over_calls"]:
            bad("往共享点加一行必须判红（calls 越基线）")

        # ③ 往 **strict 专属模块**加一行 ⇒ `--strict-only` 红；删掉 ⇒ 绿
        (root / "shared.rs").write_text(
            "fn shared_fn() {\n    let _ = infer_type_text(ctx, scope, a, env, None, false);\n}\n",
            encoding="utf-8",
        )
        (root / "kernel_app.rs").write_text(
            "fn kernel_side() {\n    let _ = infer_type_text(ctx, scope, a, env, None, false);\n}\n",
            encoding="utf-8",
        )
        r = evaluate(root, sites, base_m, base_c, strict_files)
        if r["strict_only"] != 1 or r["ok"]:
            bad(f"strict 专属点必须被数到且判红：strict_only={r['strict_only']} ok={r['ok']}")
        (root / "kernel_app.rs").write_text("fn kernel_side() {\n    let _ = 1;\n}\n", encoding="utf-8")
        r = evaluate(root, sites, base_m, base_c, strict_files)
        if not r["ok"]:
            bad("删掉 strict 专属点后必须回到绿")

        # ④ 未分类点（fail-closed）⇒ 红
        (root / "mystery.rs").write_text(
            "fn mystery() {\n    let _ = infer_type_text_inplace(x);\n}\n", encoding="utf-8"
        )
        r = evaluate(root, sites, base_m, base_c, strict_files)
        if r["ok"] or not r["unknown"]:
            bad("未分类点必须判红（fail-closed）")

    print("strict-paths self-test ✓（strict 专属点必红 · 未分类必红 · 注释不计 · 删掉即绿）")
    return 0 if ok else 1


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return self_test()
    modes = [m for m in ("--all", "--strict-only") if m in argv]
    if len(modes) > 1:
        print("audit-strict-paths: --all 与 --strict-only 只选一个", file=sys.stderr)
        return 2
    # `--root <dir>`：扫另一棵树（**咬住证明**与夹具用；默认 = `crates/front/src`）。
    root = ROOT
    if "--root" in argv:
        try:
            root = Path(argv[argv.index("--root") + 1]).resolve()
        except IndexError:
            print("audit-strict-paths: --root 需要一个目录", file=sys.stderr)
            return 2
        if not root.is_dir():
            print(f"audit-strict-paths: --root 不是目录：{root}", file=sys.stderr)
            return 2
    try:
        result = evaluate(root, S, MENTION_BASELINE, CALL_BASELINE, STRICT_ONLY_FILES)
    except RuntimeError as e:
        print(f"audit-strict-paths: {e}", file=sys.stderr)
        return 2
    result["mode"] = modes[0] if modes else "--all"
    if result["mode"] == "--strict-only":
        result["ok"] = result["ok_strict_only"]
    return report(result, "json" if "--json" in argv else "text")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
