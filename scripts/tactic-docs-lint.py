#!/usr/bin/env python3
"""tactic 清单的**编译之外**守卫（L4，设计 `docs/design/tactic-docs.md` §4.2）。

**为什么另有一条 Python 守卫**：Rust 侧那五条判据（`crates/front/tests/tactic_docs.rs`）
走的是**公开 API + 表**，它挡不住一种写法：**绕开 `is_tactic_keyword`，直接往
`parse_tactic_inner` 里加一条 `kw == "…"` 臂**（那时关键字不被认作 tactic、
功能其实用不了，但表与手写清单可能都还是"齐"的 ✗）。本脚本直接读**源码字面量**
与表比对 ⇒ 一个在 Rust 编译/测试侧、一个在文本侧，**两头咬** ✓。

**比什么**（三处必须一一对应）：
  ① `crates/front/src/tactics.rs` 的 `name: "…"`（**唯一真相**，14 条）
  ② `crates/front/src/parser.rs` 里 `parse_tactic_inner` 的 `kw == "…"` 字面量
  ③ `reference/tactics/*.md` 的文件名（`README.md` 是索引，不算）

**为什么这条是"文本比对"却不违反「判定禁文本比对」**（AGENTS.md 硬规则 4）：
那条红线管的是**判卷**（学生写的证明对不对，一律交内核）✗；
本脚本判的是**本仓自己的源码与文档是否漂移**，与 `scripts/notation-lint.py`、
`scripts/audit-wire-fields.py` 同类 ✓。

用法：
    python3 scripts/tactic-docs-lint.py            # 人读报告；有漂移 ⇒ exit 1
    python3 scripts/tactic-docs-lint.py --json     # 单 JSON 对象
    python3 scripts/tactic-docs-lint.py --selftest # 判据自检（三个方向都必须判红）

契约（与 `docs/protocol.md` 的 ok/exit 同口径）：
    exit 0 = 三处一致；exit 1 = 有漂移；exit 2 = 用法/IO 错误。
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
TACTICS_RS = REPO / "crates" / "front" / "src" / "tactics.rs"
PARSER_RS = REPO / "crates" / "front" / "src" / "parser.rs"
DOCS_DIR = REPO / "reference" / "tactics"
INDEX = "README.md"

# 表里的条目形如：  name: "intro",
NAME_RE = re.compile(r'^\s*name:\s*"([a-z]+)",', re.M)
# parser **臂头**形如：  TokenKind::Ident(kw) if kw == "intro" => {
#
# ⚠ **必须匹配整个臂头**（含行首的 `TokenKind::Ident(kw) if` 与结尾的 `=> {`），
# 不能只匹配 `kw == "…"` ✗ —— `parse_tactic_inner` 里还有一处**不是臂**的同形检查：
# `have h : T := by` 那一支写的是
# `if matches!(&self.peek().kind, TokenKind::Ident(kw) if kw == "by")` ⇒
# 只匹配 `kw ==` 会把 `by` 当成一条 tactic 臂，**当场误红**（本轮实测踩到，
# 已加 `--selftest` 的 `臂头 vs 内层检查` 那条把它钉住 ✓）。
ARM_RE = re.compile(r'^\s*TokenKind::Ident\(kw\) if kw == "([a-z]+)" => \{', re.M)


def table_names(src: str) -> list[str]:
    """① `tactics.rs` 的 name 列（**保序**：白名单文案的顺序就是它）。"""
    return NAME_RE.findall(src)


def parse_tactic_inner_body(src: str) -> str:
    """切出 `fn parse_tactic_inner` 的函数体（到下一个 `fn ` 为止）。

    ⚠ 必须**切片**而不是全文扫 `kw == "…"` ✗ —— 全文还有别的 `kw ==` 用法
    （`have h : T := by` 那个 `kw == "by"` 就在本函数里，也不是 tactic 臂；
    `is_tactic_keyword` 旧实现里还有一整份名单）。切片 + 白名单字符集（`[a-z?]+`）
    两条一起用，才咬得住正确的对象 ✓。
    """
    start = src.index("fn parse_tactic_inner")
    rest = src[start:]
    nxt = rest.find("\n    fn ", 1)
    return rest if nxt < 0 else rest[:nxt]


def parser_arms(src: str) -> list[str]:
    """② parser 的 tactic 臂关键字（保序）。"""
    return ARM_RE.findall(parse_tactic_inner_body(src))


_ARM_FIXTURE = """fn parse_tactic_inner(&mut self) -> Result<Tactic> {
    match &tok.kind {
        TokenKind::Ident(kw) if kw == "intro" => {
            self.bump();
        }
        TokenKind::Ident(kw) if kw == "have" => {
            let (value, end) = if matches!(&self.peek().kind, TokenKind::Ident(kw) if kw == "by")
            {
                (HaveValue::By(Vec::new()), tok.span.end)
            } else {
                (HaveValue::Term(Expr::Hole { span: tok.span }), tok.span.end)
            };
        }
        _ => Err(self.error_here("x")),
    }
}

    fn parse_axiom(&mut self) -> Result<Command> {"""
# 同一份夹具里还塞了另一个函数，确保切片只到本函数为止 ✓。



def doc_files(docs_dir: pathlib.Path) -> list[str]:
    """③ 文档文件名（去掉索引）。"""
    return sorted(p.stem for p in docs_dir.glob("*.md") if p.name != INDEX)


def check(table: list[str], arms: list[str], docs: list[str]) -> list[str]:
    """三处比对；返回问题列表（空 = 一致）。**纯函数**，`--selftest` 直接喂它。"""
    bad: list[str] = []
    if not table:
        bad.append("① `tactics.rs` 里没解析出任何 `name:` —— 抽取规则失效（没扫到 ≠ 绿 ✗）")
    if not arms:
        bad.append("② `parser.rs` 里没解析出任何 `kw == \"…\"` 臂 —— 抽取规则失效（没扫到 ≠ 绿 ✗）")
    if not docs:
        bad.append("③ `reference/tactics/` 里没有正文文件 —— 抽取规则失效（没扫到 ≠ 绿 ✗）")
    if bad:
        return bad

    ts, ar, dc = set(table), set(arms), set(docs)

    for name in sorted(ts - ar):
        bad.append(f"表里有 `{name}`，但 `parse_tactic_inner` 没有它的臂 ⇒ 表里加了行、parser 忘了加臂")
    for name in sorted(ar - ts):
        bad.append(f"`parse_tactic_inner` 有 `{name}` 的臂，但表里没有它 ⇒ 加臂没进表（白名单认不出它）")
    for name in sorted(ts - dc):
        bad.append(f"表里有 `{name}`，但没有 `reference/tactics/{name}.md`")
    for name in sorted(dc - ts):
        bad.append(f"有 `reference/tactics/{name}.md`，但表里没有它 ⇒ 孤儿文档")
    if sorted(table) != sorted(set(table)):
        dup = sorted({n for n in table if table.count(n) > 1})
        bad.append(f"表里有重复条目：{dup}")
    # **表序**也是契约（"未知 tactic"文案照它拼）⇒ 单列一条，别混进集合比对。
    if table != sorted(table, key=table.index):
        bad.append("表的 name 顺序与出现顺序不一致（不该发生；抽取规则可能错了）")
    return bad


def run() -> tuple[int, dict]:
    try:
        table = table_names(TACTICS_RS.read_text(encoding="utf-8"))
        arms = parser_arms(PARSER_RS.read_text(encoding="utf-8"))
        docs = doc_files(DOCS_DIR)
    except OSError as e:
        return 2, {"ok": False, "error": f"IO 错误：{e}"}
    except ValueError as e:
        return 2, {"ok": False, "error": f"抽取失败：{e}"}

    bad = check(table, arms, docs)
    report = {
        "ok": not bad,
        "counts": {"table": len(table), "parser_arms": len(arms), "doc_files": len(docs)},
        "order": table,
        "problems": bad,
    }
    return (0 if not bad else 1), report


def selftest() -> int:
    """**判据自检**：三个方向都必须判红，控制组必须判绿 ✓（AGENTS.md：咬不住的守卫等于没有）。"""
    good = (["intro", "exact", "sorry"], ["intro", "exact", "sorry"], ["intro", "exact", "sorry"])
    cases = [
        ("控制组（一致）", good, []),
        ("表多一条 / parser 少一条", (["intro", "exact", "have"], ["intro", "exact"], ["intro", "exact", "have"]), ["have"]),
        ("parser 多一条 / 表少一条（绕开白名单加臂）", (["intro"], ["intro", "hax"], ["intro"]), ["hax"]),
        ("文档缺一篇", (["intro", "exact"], ["intro", "exact"], ["intro"]), ["exact.md"]),
        ("孤儿文档", (["intro"], ["intro"], ["intro", "ghost"]), ["ghost.md"]),
        ("三处全空（没扫到 ≠ 绿 ✗）", ([], [], []), ["没扫到"]),
        ("表里重复条目", (["intro", "intro"], ["intro"], ["intro"]), ["重复"]),
    ]
    # ⚠ **抽取规则本身也要判**：`have … := by` 的内层 `kw == "by"` **不是**臂 ⇒
    # 只许抽到 `intro`/`have` 两条（本轮实测：旧正则会多抽一个 `by` 而误红 ✗）。
    got = parser_arms(_ARM_FIXTURE)
    if got != ["intro", "have"]:
        print(f"  ✗ 臂头抽取：期望 ['intro', 'have']，实际 {got}")
        failed += 1
    else:
        print("  ✓ 臂头抽取：内层 `kw == \"by\"` 没被当成臂")
    cases_total = len(cases) + 1
    failed = 0
    for label, (table, arms, docs), want in cases:
        problems = check(table, arms, docs)
        if not want:
            if problems:
                print(f"  ✗ {label}：控制组必须判绿，却报了 {problems}")
                failed += 1
            else:
                print(f"  ✓ {label}：判绿")
        else:
            hit = any(any(w in p for w in want) for p in problems)
            if not hit:
                print(f"  ✗ {label}：必须判红（含 {want}），实际 {problems}")
                failed += 1
            else:
                print(f"  ✓ {label}：判红")
    print(f"\nselftest：{cases_total - failed}/{cases_total} 通过")
    return 0 if failed == 0 else 1


def main() -> int:
    ap = argparse.ArgumentParser(add_help=True, description="tactic 清单三处一致性守卫（L4）")
    ap.add_argument("--json", action="store_true", help="输出单 JSON 对象")
    ap.add_argument("--selftest", action="store_true", help="判据自检（故意坏的三份输入必须被拒）")
    args = ap.parse_args()

    if args.selftest:
        return selftest()

    code, report = run()
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True))
        return code
    if report.get("error"):
        print(f"tactic-docs-lint：{report['error']}")
        return code
    c = report["counts"]
    if report["ok"]:
        print(
            f"tactic-docs-lint ✓ 表 {c['table']} 条 · parser 臂 {c['parser_arms']} 条 · "
            f"文档 {c['doc_files']} 篇 —— 三处一致"
        )
    else:
        print("tactic-docs-lint：有漂移 ✗")
        for p in report["problems"]:
            print(f"  - {p}")
    return code


if __name__ == "__main__":
    sys.exit(main())
