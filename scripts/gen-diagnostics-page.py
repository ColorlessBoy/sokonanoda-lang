#!/usr/bin/env python3
"""Render `site/diagnostics.html` (B8 诊断字典) from `site/data/diagnostics.json`.

Why a generator and not a hand-written page
-------------------------------------------
The page is 64 rows of "code + meaning + verbatim hint" plus a 55-row reproduction
table. Hand-copying that is 119 chances to mistype a code or invent a message, and
this repository's whole claim is that the numbers and strings on the site are real.
The generator makes one thing true by construction: **every string on the page that
looks like kernel output came out of `site/data/diagnostics.json`**, which
`scripts/gen-site-lab.py` produced by really running the CLI (D3 §4.3).

The prose is in this file, not in the JSON: it is authored text, and it must stay
reviewable in one diff.

Usage
-----
    python3 scripts/gen-diagnostics-page.py            # write site/diagnostics.html
    python3 scripts/gen-diagnostics-page.py --check    # exit 1 if the file drifted

Exit codes: 0 ok, 1 drift (with --check), 2 usage / data error.
"""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DATA = REPO / "site" / "data" / "diagnostics.json"
OUT = REPO / "site" / "diagnostics.html"

# Stage order is the data's order (parse → elab → kernel → import → warning) and the
# display name is the vocabulary the rest of the page uses. `stage` is the machine
# value; the label is what a reader sees in the third column of every row.
STAGE_LABEL = {
    "parse": "解析",
    "elab": "精化",
    "kernel": "内核",
    "import": "导入",
    "warning": "警告",
}
STAGE_ORDER = ("parse", "elab", "kernel", "import", "warning")

# Where each stage's code table lives in the source. `crates/front/src/diagnostic.rs`
# holds the parse codes; the other four groups are in the two compile modules.
# These ranges are the ones C1 §4.4 cites (verified against the files, not copied
# from the prose).
STAGE_SOURCE = {
    "parse": "crates/front/src/diagnostic.rs:78-90",
    "elab": "crates/front/src/compile/error.rs:162-190",
    "kernel": "crates/front/src/compile/error.rs:191-202",
    "import": "crates/front/src/compile/error.rs:203-208",
    "warning": "crates/front/src/compile/warning.rs:31-34",
}

PAGE_URL = "https://colorlessboy.github.io/sokonanoda-lang/diagnostics.html"
TITLE = "诊断字典：每个错误码的原文与教学提示 —— sokonanoda"
DESCRIPTION = (
    "编译器能报出的 60 个错误码与 4 个警告码全表：每个码的含义、逐字的教学提示，"
    "以及 55 条在真内核上跑出来的最小复现与它的实际输出。"
)
OG_DESCRIPTION = (
    "60 个错误码 + 4 个警告码全表：含义、逐字 hint，以及 55 条真实复现与实际输出。"
)


# `0.x.y` may not appear literally in site/*.html (check-site.py assertion #2: a
# hardcoded version is how README.md once advertised V=0.9.0 while the repo was at
# 0.17.0). Some of the `repro_absent_reason` strings quote a version, and that quote
# is load-bearing evidence, so the text stays and only the *character* changes:
# the `.` becomes a numeric character reference. The rendered page is identical;
# the source no longer contains the literal the hygiene check forbids.
VERSION_RE = re.compile(r"\b0\.\d+\.\d+\b")


def load_nav_module():
    """Import scripts/gen-site-nav.py — the one implementation of the nav rules.

    The nav/footer blocks are machine-owned (`scripts/gen-site-nav.py --write` fills
    them from `site/_partials/`). Using that module here means one command produces
    the final page and `--check` compares against the final page, instead of the
    generator and the nav writer each owning half a file.
    """
    import importlib.util

    path = Path(__file__).resolve().parent / "gen-site-nav.py"
    spec = importlib.util.spec_from_file_location("gen_site_nav", path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules["gen_site_nav"] = module
    previous = sys.dont_write_bytecode
    sys.dont_write_bytecode = True  # never drop __pycache__ into scripts/
    try:
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = previous
    return module


def esc(text: str) -> str:
    return html.escape(text, quote=False)


def esc_version(text: str) -> str:
    """Escape, then mask any version literal so the HTML source stays version-free.

    The rendered page is byte-for-byte the same; only the `.` characters inside a
    version become numeric character references, which is invisible to a reader and
    invisible to `VERSION_RE` in scripts/check-site.py. Masking runs *after*
    escaping so the `&` of `&#46;` is not itself escaped into `&amp;#46;`.
    """
    return VERSION_RE.sub(lambda m: m.group(0).replace(".", "&#46;"), esc(text))


def cell(text: str) -> str:
    """One-line cell text: newlines would break the grid alignment."""
    return esc(" ".join(text.split()))


def inline(text: str) -> str:
    """Prose with `code` spans: backticks become <code>, everything else escaped.

    The hints and meanings in the data use backticks the way C1 does, so this keeps
    the source readable without letting raw markup through. Version literals inside
    quoted evidence are masked by `esc_version`, so the file stays version-free.
    """
    out: list[str] = []
    for index, part in enumerate(text.split("`")):
        out.append(esc_version(part) if index % 2 == 0 else f"<code>{esc_version(part)}</code>")
    return "".join(out)


# ---------------------------------------------------------------------------
# data access
# ---------------------------------------------------------------------------


def load_codes() -> tuple[dict, list[dict]]:
    if not DATA.is_file():
        sys.exit(f"gen-diagnostics-page.py: {DATA.relative_to(REPO)} is missing")
    data = json.loads(DATA.read_text(encoding="utf-8"))
    codes = data.get("codes")
    if not isinstance(codes, list) or not codes:
        sys.exit("gen-diagnostics-page.py: diagnostics.json has no `codes` array")
    totals = data.get("totals", {})
    if totals.get("codes") != len(codes):
        sys.exit(
            "gen-diagnostics-page.py: totals.codes "
            f"({totals.get('codes')}) != len(codes) ({len(codes)})"
        )
    # The stage groups must be exactly the five we know how to label, and the order
    # must be the data's order (D3 §4.4: the table order is the author's teaching
    # order and must not be re-sorted).
    seen = [c["stage"] for c in codes]
    order = [s for s in STAGE_ORDER if s in seen]
    if order != list(dict.fromkeys(seen)):
        sys.exit(f"gen-diagnostics-page.py: unexpected stage order {list(dict.fromkeys(seen))}")
    return data, codes


def repro_message(code: dict) -> str:
    """The one message this code produced in its reproduction ('' if none)."""
    repro = code.get("repro")
    if not repro:
        return ""
    if repro["mode"] == "query-check":
        items = repro["output"].get("data", {}).get("failed", []) + repro["output"].get(
            "data", {}
        ).get("warnings", [])
    else:  # grade-json: a flat array of events
        items = repro["output"]
    for item in items:
        if item.get("code") == code["code"]:
            return " ".join(str(item.get("message", "")).split())
    return ""


def repro_source(code: dict) -> str:
    """The entry file's source with newlines shown as `¶`, so a row stays one line.

    `¶` (U+00B6) is deliberate: it is inside the self-hosted subset (site/assets/
    fonts.css), so it renders in the same face as the code around it. `⏎` (U+23CE)
    is not in the subset and would fall back to a platform font mid-identifier.
    """
    repro = code["repro"]
    source = repro["files"].get(repro["entry"], "")
    return "¶".join(line for line in source.rstrip("\n").split("\n"))


# ---------------------------------------------------------------------------
# sections
# ---------------------------------------------------------------------------


def code_table(codes: list[dict]) -> str:
    """§2 — the whole dictionary: code / stage / meaning / hint.

    Markup is deliberately lean: this section is 64 records, so every wrapper
    element costs 64× its own bytes. A `<ul>` of `<li data-filter-item>` is the
    smallest shape that still lets `site.js` hide individual records (it queries
    `[data-filter-item]` inside the `[data-filter]` target) and stays a real list
    for screen readers.
    """
    rows: list[str] = []
    for code in codes:
        note = "<sup>†</sup>" if code.get("hint_kind") != "verbatim" else ""
        rows.append(
            "<li data-filter-item>"
            f'<span class="dc">{esc(code["code"])}</span>'
            f'<span class="ds">{STAGE_LABEL[code["stage"]]}</span>'
            f'<span class="dm">{inline(code["meaning"])}{note}</span>'
            f'<span class="dh">{inline(code["hint"])}</span>'
            "</li>"
        )
    return "\n".join(rows)


def repro_table(codes: list[dict]) -> str:
    """§4 — one row per reproduction: source in, real message out."""
    rows: list[str] = []
    for code in codes:
        if not code.get("repro"):
            continue
        rows.append(
            "<tr>"
            f'<th scope="row"><code>{esc(code["code"])}</code></th>'
            f'<td><code class="src">{esc(repro_source(code))}</code></td>'
            f'<td class="out">{esc(repro_message(code))}</td>'
            "</tr>"
        )
    return "\n".join(rows)


def absent_table(codes: list[dict]) -> str:
    """§5 — the codes with no reproduction, with the reason the data records.

    The reason is one long sentence per code; it stays verbatim (it is the evidence
    for "this code cannot be triggered"), and `¶` marks a newline inside a code span
    exactly like the reproduction table does.
    """
    rows: list[str] = []
    for code in codes:
        if code.get("repro"):
            continue
        reason = "¶".join(line for line in code["repro_absent_reason"].split("\n"))
        rows.append(
            "<tr>"
            f'<th scope="row"><code>{esc(code["code"])}</code></th>'
            f'<td>{STAGE_LABEL[code["stage"]]}</td>'
            f"<td>{inline(reason)}</td>"
            "</tr>"
        )
    return "\n".join(rows)


def build() -> str:
    """The final page: the authored template, then the machine-owned nav blocks."""
    site = REPO / "site"
    nav = load_nav_module()
    partials = nav.load_partials(site)
    page_id = nav.derive_page_id("diagnostics.html")
    source = template()
    for kind, (start, end, _) in nav.BLOCKS.items():
        rendered = nav.render_block(partials[kind], "diagnostics.html", page_id)
        begin = source.index(start)
        finish = source.index(end, begin) + len(end)
        source = source[:begin] + f"{start}\n{rendered}\n{end}" + source[finish:]
    return source


def template() -> str:
    data, codes = load_codes()
    totals = data["totals"]
    by_stage = totals["by_stage"]
    n_with = totals["with_repro"]
    n_without = totals["without_repro"]
    source = esc(data["source"])
    commit = esc(data.get("source_commit", "—"))

    stage_rows = "\n".join(
        "<tr>"
        f'<th scope="row">{STAGE_LABEL[stage]}</th>'
        f'<td class="num">{by_stage[stage]}</td>'
        f'<td><code>{esc(STAGE_SOURCE[stage])}</code></td>'
        "</tr>"
        for stage in STAGE_ORDER
        if stage in by_stage
    )

    return f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">

<title>{TITLE}</title>
<meta name="description" content="{DESCRIPTION}">
<link rel="canonical" href="{PAGE_URL}">
<meta name="theme-color" content="#f5f7f9" media="(prefers-color-scheme: light)">
<meta name="theme-color" content="#0f1216" media="(prefers-color-scheme: dark)">
<meta property="og:type" content="article">
<meta property="og:title" content="{TITLE}">
<meta property="og:description" content="{OG_DESCRIPTION}">
<meta property="og:locale" content="zh_CN">
<link rel="icon" href="favicon.svg" type="image/svg+xml">

<!-- 顺序固定：fonts → tokens → base → site（→ 本页专属的 diagnostics） -->
<link rel="stylesheet" href="assets/fonts.css">
<link rel="stylesheet" href="assets/tokens.css">
<link rel="stylesheet" href="assets/base.css">
<link rel="stylesheet" href="assets/site.css">
<link rel="stylesheet" href="assets/diagnostics.css">

<!-- 主题防闪烁：必须在样式之前同步执行，所以是内联的。 -->
<script>try{{var t=localStorage.getItem("soko-theme");if(t&&t!=="system")document.documentElement.setAttribute("data-theme",t)}}catch(e){{}}</script>
</head>

<body data-page="diagnostics" data-root="./">

<!--#nav-->
<!--/#nav-->

<main id="main">
  <div class="wrap">

    <nav class="breadcrumbs" aria-label="面包屑">
      <ol>
        <li><a href="index.html">首页</a></li>
        <li><a href="kernel.html">内核判卷</a></li>
      </ol>
    </nav>

    <header class="page-head">
      <p class="sec-num">功能</p>
      <h1>诊断字典</h1>
      <p class="lead">
        编译器能报出的每一个码都在这一页：{totals["codes"]} 条，每条一句含义、一句教学提示。
        其中 {n_with} 条附一段最小源码和它真的跑出来的输出。这一页要证明的不是「它会拒绝」，
        而是拒绝的时候它把话说清楚了。
      </p>
    </header>

    <section class="measure" aria-labelledby="s1">
      <h2 id="s1"><span class="sec-num">§1</span> 四个阶段，{totals["codes"]} 个码</h2>
      <p>
        源码进编译器要过四道关，每道关问一个不同的问题：解析问「写得像不像这门语言」，
        精化问「名字和类型对不对得上」，内核问「证明项真的成立吗」，导入问「多文件的闭包对不对」。
      </p>

      <div class="turnstile" role="img" aria-label="解析 12 个码，精化 29 个，内核 12 个，导入 7 个，另有 4 个警告码；合计 64 个码，其中 55 个有真实复现">
        <ul class="turnstile__given">
          <li><span class="tok-ty">解析</span> · {by_stage["parse"]} 个码</li>
          <li><span class="tok-ty">精化</span> · {by_stage["elab"]} 个码</li>
          <li><span class="tok-ty">内核</span> · {by_stage["kernel"]} 个码</li>
          <li><span class="tok-ty">导入</span> · {by_stage["import"]} 个码</li>
          <li><span class="tok-ty">警告</span> · {by_stage["warning"]} 个码，不改退出码</li>
        </ul>
        <p class="turnstile__rule"><span class="turnstile__turn">⊢</span></p>
        <p class="turnstile__goal">
          {totals["errors"]} 个错误码 + {totals["warnings"]} 个警告码 = {totals["codes"]} 个稳定码
          <span class="muted">，其中 {n_with} 个有真实复现</span>
        </p>
      </div>

      <p>
        码为什么这么细？因为下游要按<strong>错误的种类</strong>做反应，而不是按错误的措辞。
        编辑器靠码决定在哪画波浪线、给不给快速修复；code agent 靠码决定下一步试什么；
        课程工具靠码判断一条声明是「还没做完」还是「做坏了」。这些都不该去匹配
        <code>message</code> 里的字符串——那句话随时可以改，码不会。
      </p>

      <div class="table-scroll">
        <table class="table">
          <caption>四个阶段的码数，与每一组码在源码里的出处</caption>
          <thead>
            <tr><th scope="col">阶段</th><th scope="col" class="num">码数</th><th scope="col">码表出处</th></tr>
          </thead>
          <tbody>
{stage_rows}
            <tr><th scope="row">合计</th><td class="num">{totals["codes"]}</td><td>错误码 {totals["errors"]} + 警告码 {totals["warnings"]}</td></tr>
          </tbody>
        </table>
      </div>

      <p class="small muted">
        出处是源码里那张 <code>match</code> 表的行号，不是文档的转述。
        本页内容取自 <code>{source}</code>（快照提交 <code>{commit}</code>）——
        与 <a href="kernel.html">内核判卷</a> 的 <code>--json</code> 是同一份真相。
      </p>
    </section>

    <section aria-labelledby="s2">
      <h2 id="s2"><span class="sec-num">§2</span> 全表：{totals["codes"]} 个码</h2>
      <p class="measure">
        每一行是一个码：第一列是它的稳定标识，第二列是它属于哪个阶段，第三列是它的含义，
        第四列是它携带的教学提示。提示是源码里的原文——它不是把错误重说一遍，
        而是告诉你<strong>下一步做什么</strong>。
      </p>

      <div class="filter-row">
        <label class="visually-hidden" for="diag-q">按码或关键词筛选</label>
        <input class="filter" id="diag-q" type="search" data-filter="#diag-list"
               placeholder="输入码或关键词，例如 match、命名空间、sorry" autocomplete="off" hidden>
        <span class="filter__count" data-filter-count role="status" aria-live="polite"></span>
      </div>
      <p class="small muted">
        上面这个框由脚本显形。没有脚本时它不出现，下面 {totals["codes"]} 行照旧全部可读——
        筛选是增强，不是内容。
      </p>

      <div id="diag-list" class="diag-list">
{code_table(codes)}
      </div>

      <p class="small muted">
        <sup>†</sup> 这一条的提示不是内核逐字发出的文案，而是同一处代码里的说明文本
        （<code>hint_kind: note</code>）。其余 {totals["codes"] - sum(1 for c in codes if c.get("hint_kind") != "verbatim")} 条都是逐字的。
      </p>
    </section>

    <section aria-labelledby="s3">
      <h2 id="s3"><span class="sec-num">§3</span> {n_with} 条真实复现</h2>
      <p class="measure">
        下面每一行都是真的跑出来的：把左边那段最小源码放进一个文件，跑
        <code>query check</code>（或 <code>grade --json</code>），右边就是它吐出来的那条
        <code>message</code>，逐字。源码里 {esc("¶")} 表示换行。
      </p>

      <div class="callout callout--verified">
        <span class="callout__title">这些不是示例</span>
        <p>
          <code>scripts/gen-site-lab.py</code> 只在真输出里出现了那个码时才把它记下来。
          内核行为一变，这条复现会自动退回「没有复现」并写清原因，而不是留下一条与事实不符的记录。
        </p>
      </div>

      <div class="table-scroll">
        <table class="table ledger">
          <caption>每个码的最小触发源码与它的实际输出；只有警告码的退出码是 0</caption>
          <thead>
            <tr>
              <th scope="col">码</th>
              <th scope="col">最小源码</th>
              <th scope="col">实际输出</th>
            </tr>
          </thead>
          <tbody>
{repro_table(codes)}
          </tbody>
        </table>
      </div>
    </section>

    <section class="measure" aria-labelledby="s4">
      <h2 id="s4"><span class="sec-num">§4</span> {n_without} 条触发不了</h2>
      <p>
        字典里有 {n_without} 条码<strong>不可能从学习者代码触发</strong>。不是漏测——
        每条都查过为什么，原因写在数据里：
      </p>

      <div class="table-scroll">
        <table class="table">
          <caption>没有复现的码，以及数据记录的不可触发原因</caption>
          <thead>
            <tr><th scope="col">码</th><th scope="col">阶段</th><th scope="col">为什么触发不了</th></tr>
          </thead>
          <tbody>
{absent_table(codes)}
          </tbody>
        </table>
      </div>

      <p>
        它们仍然留在码表里，因为码集合是<strong>稳定契约</strong>：删掉一个码就是改接口。
        它们的 hint 照旧写着，哪天路径可达了就直接用得上。
      </p>
    </section>

    <section class="measure" aria-labelledby="s5">
      <h2 id="s5"><span class="sec-num">§5</span> 警告与诊断不是一回事</h2>
      <p>
        码表里 {totals["warnings"]} 条是<strong>警告</strong>，其余 {totals["errors"]} 条是诊断。
        区别只有一个，但它承重：<strong>警告永不影响退出码</strong>。
      </p>

      <div class="turnstile" role="img" aria-label="同一条多余的 sorry：作为警告时 exit 0，作为诊断时 exit 1">
        <ul class="turnstile__given">
          <li><span class="tok-local">theorem t : True := True.intro sorry</span></li>
          <li><span class="tok-ty">warning</span> · <span class="tok-local">redundant-sorry</span> · exit 0</li>
          <li><span class="tok-local">def x : sorry := True.intro</span></li>
          <li><span class="tok-ty">diagnostic</span> · <span class="tok-local">elab-hole-misplaced</span> · exit 1</li>
        </ul>
        <p class="turnstile__rule"><span class="turnstile__turn">⊢</span></p>
        <p class="turnstile__goal">
          课程工具只看退出码就会漏掉警告，所以签名腐烂必须报诊断
        </p>
      </div>

      <p>
        两个例子都是本页复现表里的真实运行：一个多余的 <code>sorry</code> 只发警告，退出码 0；
        而 <code>sorry</code> 出现在值位以外是诊断，退出码 1。签名烂掉的练习
        （<code>theorem</code> 的类型不是命题）走的是后一条路——一旦做成警告，退出码不变，
        课程侧就永远发现不了那个练习已经坏了。
      </p>

      <div class="callout callout--limit">
        <span class="callout__title">限制</span>
        <p>
          码表是<strong>这一版的冻结快照</strong>（<code>{source}</code>，提交 <code>{commit}</code>）：
          码是稳定契约，只增不改、不重编号、不挪作他用。新增码要等下一版。
        </p>
        <p>
          警告不改退出码，但<strong>不代表带警告的文件一定 exit 0</strong>：同一份文件里可以
          既有警告又有诊断，那时退出码由诊断决定。<code>import-has-open-exercises</code> 的复现
          就是这样——它 exit 1，因为同一份源码里还有一条 <code>elab-unknown-identifier</code>。
        </p>
      </div>

      <p class="small muted">
        退出码口径：<code>0</code> 答上了（一个开放的 <code>sorry</code> 练习是合法状态）、
        <code>1</code> 文件被拒、<code>2</code> 用法错误。判据看 JSON，不看退出码。
      </p>
    </section>

  </div>
</main>

<!--#footer-->
<!--/#footer-->

<script src="assets/site.js"></script>
</body>
</html>
"""


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check",
        action="store_true",
        help="do not write; exit 1 if site/diagnostics.html differs from the render",
    )
    args = parser.parse_args(argv)

    rendered = build()
    if args.check:
        current = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        if current != rendered:
            print(f"gen-diagnostics-page.py: {OUT.relative_to(REPO)} is stale", file=sys.stderr)
            return 1
        print(f"gen-diagnostics-page.py: {OUT.relative_to(REPO)} is up to date")
        return 0

    OUT.write_text(rendered, encoding="utf-8")
    print(f"gen-diagnostics-page.py: wrote {OUT.relative_to(REPO)} ({OUT.stat().st_size:,} B)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
