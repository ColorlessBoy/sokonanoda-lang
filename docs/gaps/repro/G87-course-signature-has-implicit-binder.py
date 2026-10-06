#!/usr/bin/env python3
"""G-87 判据：课程签名里的隐式 binder 必须**恰好**是契约清单（棘轮）里的那些。

背景（为什么这条缺口现在是这个形状）
------------------------------------
G-87 登记时的形态（2026-10-04）是「课程签名里出现了隐式 binder ⇒ **IA-1（隐式实参
插入）的前提被打破**」：`crates/cli/tests/notation.rs` 的
`no_course_signature_uses_an_implicit_binder` 断言课程签名里**一个**隐式 binder 都
不许有 —— 那是「IA-1 落地时课程零改动也全绿」的前提。

后来的事实（台账 `expected_lean` 的第二条路）：
  * **IA-1 已落地**（`implicit_prefix` + 求解器，G-40/G-42 已修）；
  * 卷 I 课程库按 **IA-2 / T-N13（0.79.0）** *有意*把前导类型参数隐式化
    （与 Lean 4 写法对齐）⇒ 前提被**有意的契约变更**取代 ✗，不是回归 ✗；
  * 那条闸因此改成**棘轮白名单** `INTENTIONAL`（「加签名要改这里，评审可见」✓）。

所以本判据现在断言的是**棘轮契约**（与 Rust 守卫**同一份清单** ✓，清单只有一处：
`crates/cli/tests/notation.rs` 的 `INTENTIONAL`）：

    课程签名里的隐式 binder 集合  ==  INTENTIONAL 登记集合

两个方向都要咬住：
  * **多**：出现未登记的隐式签名 ⇒ 缺口仍在（exit 0）—— 这正是 IA-1 会改变课程
    行为的面，必须评审后才许出现 ✓；
  * **少**：清单里的声明不再有隐式 binder（改名 / 改回显式）⇒ 也报 exit 0
    —— 棘轮烂了（清单与现实脱节）同样要人来看 ✓。

为什么不用旧版的 `grep '^(theorem|def|…)[^:]*\\{[^}]*:[^}]*\\}'` ✗
--------------------------------------------------------------
旧复现件**两个方向都不准**（2026-10-06 实测）：
  * **多报**：它不认识 `INTENTIONAL` ⇒ 把 59 条**有意**的隐式签名全报成「缺口仍在」✗；
  * **漏报**：关键字表里没有 `abbrev` ⇒ `lib/Set` 的 `abbrev triple {α : Type} …`
    从来没被它看见 ✗（实测：grep 命中 57 条、白名单 59 条，差的正是 `Set.triple`
    与命名空间限定名的写法差异）；
  * 只看**单行**：`theorem t` 换行再写 `{α : Type}` 的形状它看不见 ✗。

本脚本改成**源级扫描器**（按声明收集「签名」文本，再判括号组是不是 binder 位）：
  * 声明关键字与守卫同口径：`def` / `abbrev` / `theorem` / `example` / `axiom`
    （`inductive` **不在**守卫判据内 ⇒ 本脚本同样跳过，保持"同一条判据" ✓）；
  * 签名 = 声明关键字 → 顶层 `:=`（无 `:=` 的 `axiom` 到下一个声明行）之间的文本；
  * 隐式 binder = **binder 位**上的 `{…}` 且组内有顶层 `:`，binder 位 = ① 第一个顶层
    `:` 之前（前导 binder 位 ✓），或 ② 处于 `∀` 的 binder 列表里（`∀` 与它的 `,`
    之间，任意深度 ✓）；
  * 体里的集合字面量（`{a}` / `:= {a}`）**不在**签名里 ⇒ 不算 ✗（旧版的假阳性面）；
  * 命名空间按解析器同款规则累积（`namespace Set` 里 `def mem` ⇒ `Set.mem`）。

覆盖强度（诚实记账 ✓）
----------------------
本扫描器对「`∀ {…}` 出现在**任意深度**（含箭头定义域、应用实参）」比 Rust 守卫的
`implicit_binders_in` **略宽**（守卫只递归 `Forall` 的体与 `Arrow` 的余域 ✗）。若两者
不一致，以**更宽**的为准：那说明课程里真出现了隐式 binder，需要评审 ✓（不是假红 ✓）。

用法
----
    python3 docs/gaps/repro/G87-course-signature-has-implicit-binder.py
    python3 docs/gaps/repro/G87-course-signature-has-implicit-binder.py --selftest
    python3 docs/gaps/repro/G87-course-signature-has-implicit-binder.py --json

退出码（`docs/gaps/README.md` 约定 ✓）：0 = 缺口仍在 / 1 = 已清 / 2 = 环境或形状不对。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
GUARD_RS = Path("crates/cli/tests/notation.rs")
TREES = ("courses/set-theory", "course")

#: 与守卫同口径的声明关键字（`parser.rs::parse_command` 的声明族）。
DECL_KEYWORDS = ("def", "abbrev", "theorem", "example", "axiom")
#: 守卫**不看**的声明族（`implicit_binders_in` 的 match 里没有它）⇒ 本脚本同样跳过。
GUARD_SKIPS = ("inductive",)

#: 守卫里写死的两处**教学例外**（题目本身就在教隐式 binder，只在入门课 `course/`）。
TEACHING_EXCEPTIONS = ("Eq.symm", "eq_refl_prop")

#: 语句起始关键字（用于判定"上一条声明到哪里结束" —— 无 `:=` 的 `axiom` 靠它收口）。
STATEMENT_KEYWORDS = DECL_KEYWORDS + GUARD_SKIPS + (
    "import",
    "namespace",
    "end",
    "open",
    "export",
    "scoped",
    "infix",
    "infixl",
    "infixr",
    "prefix",
    "postfix",
    "notation",
    "binder_notation",
    "#check",
    "#reduce",
    "#print",
)

#: 扫描到的声明数下限（形状守卫）：低于它说明扫描器坏了（比如解析提前收摊 ✗）。
MIN_DECLS = 2000
#: 清单条数下限（守卫自己也断言 `scanned > 30` ⇒ 同一层意思）。
MIN_WHITELIST = 40


def strip_comments(src: str) -> list[str]:
    """去掉 `--` 行注释（**字符串里的 `--` 不算** ✓ —— 记法声明里就有 `" ∈ "`）。"""
    out = []
    for line in src.split("\n"):
        buf = []
        i = 0
        in_str = False
        while i < len(line):
            c = line[i]
            if in_str:
                if c == "\\" and i + 1 < len(line):
                    buf.append(line[i : i + 2])
                    i += 2
                    continue
                if c == '"':
                    in_str = False
                buf.append(c)
                i += 1
                continue
            if c == '"':
                in_str = True
                buf.append(c)
                i += 1
                continue
            if c == "-" and i + 1 < len(line) and line[i + 1] == "-":
                break
            buf.append(c)
            i += 1
        out.append("".join(buf))
    return out


def join_ns(prefix: str | None, name: str) -> str:
    """与 `parser.rs::qualify_decl_name` 同规则：名字本身带点则**拼接** ✓。"""
    return name if prefix is None else f"{prefix}.{name}"


def skip_string(text: str, i: int) -> int:
    """`text[i] == '"'` ⇒ 返回闭引号的下标（没闭合就到末尾）。"""
    i += 1
    while i < len(text):
        if text[i] == "\\":
            i += 2
            continue
        if text[i] == '"':
            return i
        i += 1
    return len(text) - 1


def match_brace(text: str, i: int) -> int:
    """`text[i] == '{'` ⇒ 返回配对 `}` 的下标（没配对上就到末尾）。"""
    depth = 0
    while i < len(text):
        c = text[i]
        if c == '"':
            i = skip_string(text, i) + 1
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return len(text) - 1


def has_top_level_colon(inner: str) -> bool:
    depth = 0
    i = 0
    while i < len(inner):
        c = inner[i]
        if c == '"':
            i = skip_string(inner, i) + 1
            continue
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif c == ":" and depth == 0:
            return True
        i += 1
    return False


def binder_names(inner: str) -> str:
    """`α β γ : Type` ⇒ `α β γ`（只用于打印 ✓）。"""
    depth = 0
    i = 0
    while i < len(inner):
        c = inner[i]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif c == ":" and depth == 0:
            return " ".join(inner[:i].split())
        i += 1
    return " ".join(inner.split())


def first_top_level_colon(header: str) -> int:
    depth = 0
    i = 0
    while i < len(header):
        c = header[i]
        if c == '"':
            i = skip_string(header, i) + 1
            continue
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif c == ":" and depth == 0:
            return i
        i += 1
    return len(header)


def implicit_binders(header: str) -> list[str]:
    """签名文本里的隐式 binder（binder 位上的 `{…}` 且组内有顶层 `:`）。

    binder 位（三条，对齐 `parser.rs` 的 `parse_binder_group` / `named_group_ahead`）：
      ① 第一个顶层 `:` **之前** —— 声明的前导 binder 位 ✓；
      ② `∀` 的 binder 列表里（`∀` 与它的 `,` 之间，任意深度 ✓）；
      ③ 紧跟 `→` / `->` 的箭头定义域位（`axiom ext : {α : Type} → …` ✓
         —— 解析器把这种组展开成 `Forall` 链，守卫的 `Forall` 臂能看见它 ✓）。
    """
    lead_end = first_top_level_colon(header)
    out: list[str] = []
    depth = 0
    forall_depth: int | None = None
    i = 0
    while i < len(header):
        c = header[i]
        if c == '"':
            i = skip_string(header, i) + 1
            continue
        if c == "∀":
            forall_depth = depth
            i += 1
            continue
        if c == "{":
            close = match_brace(header, i)
            inner = header[i + 1 : close]
            in_binder_pos = (
                (i < lead_end)
                or (forall_depth is not None and depth == forall_depth)
                or arrow_ahead(header, close + 1)
            )
            if in_binder_pos and has_top_level_colon(inner):
                out.append(binder_names(inner))
            i = close + 1
            continue
        if c in "([":
            depth += 1
        elif c in ")]":
            depth -= 1
        elif c == "," and forall_depth is not None and depth == forall_depth:
            forall_depth = None
        i += 1
    return out


def arrow_ahead(header: str, i: int) -> bool:
    """`header[i:]` 跳过空白后是不是 `→` / `->`（箭头定义域位 ✓）。"""
    while i < len(header) and header[i] in " \t\n":
        i += 1
    return header.startswith("→", i) or header.startswith("->", i)


def starts_statement(line: str) -> bool:
    """行首（**第 0 列**）是不是一条新语句 —— 声明在课程里一律顶格 ✓。"""
    if not line or line[0] in " \t":
        return False
    m = re.match(r"([A-Za-z_#][A-Za-z0-9_#]*)", line)
    return bool(m) and m.group(1) in STATEMENT_KEYWORDS


def collect_header(lines: list[str], i: int, rest: str) -> tuple[str, int]:
    """从声明行剩余部分开始，收齐**签名**文本；返回 (签名, 声明末行下标)。"""
    buf: list[str] = []
    depth = 0
    text = rest
    j = i
    while True:
        k = 0
        in_str = False
        cut = None
        while k < len(text):
            c = text[k]
            if in_str:
                if c == "\\":
                    k += 2
                    continue
                if c == '"':
                    in_str = False
                k += 1
                continue
            if c == '"':
                in_str = True
                k += 1
                continue
            if c in "([{":
                depth += 1
            elif c in ")]}":
                depth -= 1
            elif c == ":" and k + 1 < len(text) and text[k + 1] == "=" and depth == 0:
                cut = k
                break
            k += 1
        if cut is not None:
            buf.append(text[:cut])
            return "\n".join(buf), j
        buf.append(text)
        j += 1
        if j >= len(lines):
            return "\n".join(buf), j - 1
        nxt = lines[j]
        if depth == 0 and starts_statement(nxt):
            return "\n".join(buf), j - 1
        text = nxt


def scan_source(src: str, rel: str) -> tuple[list[dict], int]:
    """返回 (命中列表, 扫到的声明数)。"""
    lines = strip_comments(src)
    ns: list[str] = []
    hits: list[dict] = []
    total = 0
    i = 0
    while i < len(lines):
        line = lines[i]
        s = line.strip()
        if not s:
            i += 1
            continue
        if line[:1] not in (" ", "\t"):
            m = re.match(r"([A-Za-z_#][A-Za-z0-9_#]*)", s)
            kw = m.group(1) if m else None
            if kw == "namespace":
                name = s[len("namespace") :].strip()
                ns.append(join_ns(ns[-1] if ns else None, name))
                i += 1
                continue
            if kw == "end":
                # **裸 `end` 收的是归纳块**（`parse_inductive_block` 的 `end` ✗ 不吃名字），
                # 只有 `end <Name>` 才闭合 `namespace` ✓（`parse_end_command` 要求带名字 ✓）。
                if s[len("end") :].strip() and ns:
                    ns.pop()
                i += 1
                continue
            if kw in DECL_KEYWORDS or kw in GUARD_SKIPS:
                header, last = collect_header(lines, i, s[len(kw) :])
                total += 1
                if kw in DECL_KEYWORDS:
                    rest = s[len(kw) :].lstrip()
                    nm = re.match(r"([A-Za-z_][A-Za-z0-9_.']*)", rest)
                    if nm:
                        name = join_ns(ns[-1] if ns else None, nm.group(1))
                    elif kw == "example":
                        name = "<example>"
                    else:
                        name = "<?>"
                    binders = implicit_binders(header)
                    if binders:
                        hits.append(
                            {
                                "file": rel,
                                "name": name,
                                "keyword": kw,
                                "line": i + 1,
                                "binders": binders,
                            }
                        )
                i = last + 1
                continue
        i += 1
    return hits, total


def scan_tree(root: Path, tree: str) -> tuple[list[dict], int]:
    hits: list[dict] = []
    total = 0
    base = root / tree
    if not base.is_dir():
        return hits, total
    for path in sorted(base.rglob("*.sokonanoda")):
        try:
            src = path.read_text(encoding="utf-8")
        except OSError:
            continue
        rel = f"{tree}/{path.relative_to(base).as_posix()}"
        h, n = scan_source(src, rel)
        hits.extend(h)
        total += n
    return hits, total


def parse_whitelist(text: str) -> set[tuple[str, str]]:
    """从守卫源码里取 `INTENTIONAL` 清单（**唯一权威** ✓，本脚本不另存一份）。"""
    m = re.search(r"const INTENTIONAL: &\[\(&str, &str\)\] = &\[(.*?)\n\];", text, re.S)
    if not m:
        raise ValueError("在守卫源码里找不到 `const INTENTIONAL: &[(&str, &str)] = &[…]`")
    return set(re.findall(r'\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,?\s*\)', m.group(1)))


def allowed(file: str, name: str, whitelist: set[tuple[str, str]]) -> bool:
    if (file, name) in whitelist:
        return True
    # 守卫的两处教学例外（只在入门课 `course/` 下、只按名字）。
    return file.startswith("course/") and name in TEACHING_EXCEPTIONS


def check(
    trees: list[tuple[Path, str]], whitelist: set[tuple[str, str]]
) -> dict:
    hits: list[dict] = []
    total = 0
    for root, tree in trees:
        h, n = scan_tree(root, tree)
        hits.extend(h)
        total += n
    found = {(h["file"], h["name"]) for h in hits}
    wl = set(whitelist)
    unreviewed = [h for h in hits if not allowed(h["file"], h["name"], wl)]
    rot = sorted(wl - found)
    exceptions = sorted(
        p
        for p in found
        if p not in wl and p[0].startswith("course/") and p[1] in TEACHING_EXCEPTIONS
    )
    return {
        "decls": total,
        "found": len(found),
        "whitelist": len(wl),
        "exceptions": len(exceptions),
        "unreviewed": unreviewed,
        "rot": rot,
    }


def report(res: dict) -> None:
    print(
        f"扫到声明 {res['decls']} 条 · 签名里有隐式 binder 的 {res['found']} 条"
        f"（契约清单 {res['whitelist']} 条 + 教学例外 {res['exceptions']} 条）"
    )
    for h in res["unreviewed"]:
        print(
            f"  ✗ 未登记：{h['file']}:{h['line']} {h['keyword']} {h['name']} "
            f"→ {{{', '.join(h['binders'])}}}"
        )
    for file, name in res["rot"]:
        print(f"  ✗ 清单腐烂：{file}:{name} 已不再有隐式 binder（改名 / 改回显式？）")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="G-87：课程签名隐式 binder 的棘轮契约")
    ap.add_argument("--selftest", action="store_true", help="跑夹具自检（不扫真课程）")
    ap.add_argument("--json", action="store_true", help="出 JSON")
    ap.add_argument("--repo", default=str(REPO), help="仓库根（自检/反验用）")
    args = ap.parse_args(argv)

    if args.selftest:
        return selftest()

    repo = Path(args.repo)
    guard = repo / GUARD_RS
    if not guard.is_file():
        print(f"G-87: 找不到守卫源码 {GUARD_RS}（模块根不对？）", file=sys.stderr)
        return 2
    try:
        whitelist = parse_whitelist(guard.read_text(encoding="utf-8"))
    except ValueError as e:
        print(f"G-87: {e}", file=sys.stderr)
        return 2
    if len(whitelist) < MIN_WHITELIST:
        print(
            f"G-87: 契约清单只解析出 {len(whitelist)} 条（< {MIN_WHITELIST}）⇒ 形状不对",
            file=sys.stderr,
        )
        return 2

    res = check([(repo, t) for t in TREES], whitelist)
    if res["decls"] < MIN_DECLS:
        print(
            f"G-87: 只扫到 {res['decls']} 条声明（< {MIN_DECLS}）⇒ 扫描器/课程树形状不对",
            file=sys.stderr,
        )
        return 2

    if args.json:
        print(json.dumps(res, ensure_ascii=False, indent=2))
        return 0 if (res["unreviewed"] or res["rot"]) else 1

    if res["unreviewed"] or res["rot"]:
        report(res)
        print(
            "缺口仍在 ✗：课程签名里的隐式 binder 与契约清单不一致 ——\n"
            "  · 「未登记」= IA-1（隐式实参插入）会在这条声明上生效 ⇒ 必须评审后\n"
            "    把它加进 `crates/cli/tests/notation.rs` 的 `INTENTIONAL` ✓；\n"
            "  · 「清单腐烂」= 清单里的声明不再隐式 ⇒ 同轮把它从清单里删掉 ✓。",
            file=sys.stderr,
        )
        return 0

    if not args.json:
        report(res)
    print(
        "已清 ✓：课程签名里的隐式 binder 与契约**逐条**对得上 ✓ —— "
        f"契约清单 {res['whitelist']} 条 + 入门课教学例外 {res['exceptions']} 条 "
        f"= {res['found']} 条（IA-1 的作用面已被钉死 ✓）"
    )
    return 1


# ── 自检（夹具）────────────────────────────────────────────────────────────
#: 夹具：每个文件的内容 + 期望「签名里有隐式 binder」的声明名（`None` = 一条都不该有）。
FIXTURES: dict[str, tuple[str, list[str]]] = {
    "courses/set-theory/lib/Fixture.sokonanoda": (
        """-- 夹具：单行前导隐式 binder ✓
namespace A

def t1 {α : Type} (a : α) : Prop := True

-- 夹具：换行写的前导隐式 binder（旧 grep 看不见 ✗）
theorem t2
    {α : Type}
    (a : α) : True := True.intro

-- 夹具：只有体/陈述里的集合字面量 ⇒ **不算**隐式签名 ✓
theorem t3 (a : Nat) : a ∈ ({a} : Set Nat) := by sorry

-- 夹具：`abbrev`（旧 grep 的关键字表里没有它 ✗）
abbrev triple {α : Type} (a b c : α) : Set α := fun (x : α) => x = a

-- 夹具：`axiom`（无 `:=`，靠下一条声明收口 ✓）
axiom ax {α : Type} : α → α

-- 夹具：`example`（匿名，守卫记作 `<example>` ✓）
example {α : Type} (a : α) : a = a := Eq.refl a

-- 夹具：显式 binder ⇒ 不算 ✓
def t4 (α : Type) (a : α) : Prop := True

-- 夹具：注释里的声明不算 ✓
-- def ghost {α : Type} : Prop := True

-- 夹具：类型里嵌套的 `∀ {…}` ✓
axiom nested : (∀ {α : Type}, α → α) → Prop

-- 夹具：`inductive` **不在**守卫判据内 ⇒ 同口径跳过 ✓
inductive Ind (A : Type) : Type
ctor mk : Ind A
end

end A
""",
        ["A.t1", "A.t2", "A.triple", "A.ax", "<example>", "A.nested"],
    ),
    "course/unit-fixture.sokonanoda": (
        """-- 夹具：入门课的教学例外（题目本身教隐式 binder ✓ 守卫写死放行 ✓）
theorem eq_refl_prop {a : Prop} : a → a := fun (h : a) => h

-- 夹具：入门课里**别的**隐式签名仍要报 ✗
theorem other {a : Prop} : a → a := fun (h : a) => h
""",
        ["eq_refl_prop", "other"],
    ),
}


def selftest() -> int:
    """夹具自检：扫描器的**两个方向**都要能咬住（含反向验证 ✓）。"""
    failures: list[str] = []
    with tempfile.TemporaryDirectory(prefix="g87-selftest-") as tmp:
        root = Path(tmp)
        for rel, (body, _) in FIXTURES.items():
            p = root / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(body, encoding="utf-8")

        # ① 期望命中集：把夹具声明的期望写成一份"守卫源码"（同一解析路径 ✓）。
        expected: set[tuple[str, str]] = set()
        for rel, (_, names) in FIXTURES.items():
            for n in names:
                expected.add((rel, n))
        # 入门课的两处**教学例外**由守卫写死放行 ⇒ 正向清单里**故意不登记**它们，
        # 用来验证例外这条路真的通 ✓。
        exceptions = {
            (rel, n)
            for rel, n in expected
            if rel.startswith("course/") and n in TEACHING_EXCEPTIONS
        }
        rs = "const INTENTIONAL: &[(&str, &str)] = &[\n"
        for rel, name in sorted(expected - exceptions):
            rs += f'    ("{rel}", "{name}"),\n'
        rs += "];\n"
        guard = root / "guard.rs"
        guard.write_text(rs, encoding="utf-8")

        # ② 正向：清单**覆盖**夹具里所有隐式签名（教学例外靠守卫写死的白名单 ✓）
        #    ⇒ 已清（unreviewed 空、rot 空）。
        res = check([(root, "courses/set-theory"), (root, "course")], parse_whitelist(rs))
        if res["unreviewed"]:
            failures.append(
                "正向：清单已覆盖全部夹具，却仍报未登记 → "
                + ", ".join(f"{h['file']}:{h['name']}" for h in res["unreviewed"])
            )
        if res["rot"]:
            failures.append(f"正向：夹具清单腐烂 {res['rot']}（扫描器漏了声明？）")
        if res["found"] != len(expected):
            failures.append(
                f"正向：命中 {res['found']} 条、期望 {len(expected)} 条"
                f"（{sorted(expected)} vs 实得 {res['found']}）"
            )

        # ③ **反向验证**：从清单里删掉一条 ⇒ 必须当场报「未登记」✗（咬得住 ✓）。
        broken = set(expected)
        victim = ("courses/set-theory/lib/Fixture.sokonanoda", "A.triple")
        broken.discard(victim)
        res2 = check([(root, "courses/set-theory"), (root, "course")], broken)
        got = {(h["file"], h["name"]) for h in res2["unreviewed"]}
        if got != {victim}:
            failures.append(f"反向：删掉 {victim} 后应只报它，实得 {sorted(got)}")

        # ④ **反向验证（另一方向）**：清单里留一条现实中不存在的 ⇒ 必须报「清单腐烂」✗。
        broken2 = set(expected) | {("courses/set-theory/lib/Fixture.sokonanoda", "A.ghost")}
        res3 = check([(root, "courses/set-theory"), (root, "course")], broken2)
        if res3["rot"] != [("courses/set-theory/lib/Fixture.sokonanoda", "A.ghost")]:
            failures.append(f"反向（腐烂）：应只报 A.ghost，实得 {res3['rot']}")

        # ⑤ 形状守卫：`inductive` 不在判据内（与守卫同口径 ✓）。
        if any(h["name"].startswith("Ind") for h in res["unreviewed"]):
            failures.append("形状：`inductive` 不该进判据（守卫同口径）")

    if failures:
        print("✗ 自检失败：", file=sys.stderr)
        for f in failures:
            print(f"  · {f}", file=sys.stderr)
        return 2
    print(
        "✓ 自检通过：夹具 "
        f"{len(FIXTURES)} 个文件 · 期望命中 {len(expected)} 条 · "
        "正向（覆盖 ⇒ 已清）✓ · 反向（删一条 ⇒ 报未登记）✓ · "
        "反向（留一条不存在的 ⇒ 报清单腐烂）✓ · `inductive` 同口径跳过 ✓"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
