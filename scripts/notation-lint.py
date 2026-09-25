#!/usr/bin/env python3
"""courses 的「记法规则」检查器（notation lint）。

规则（2026-09-21 用户拍板，权威出处写在这里，避免下次又靠眼睛）：
  · 课程一律写 **Lean 4 记法**，不写「点名 + 前导类型/宇宙实参」的旧写法：
      `Eq.{1} (Set α) A B` → `A = B`          （用户原话：Eq.{1} 直接就是一个等于号）
      `Ne.{1} (Set α) A B` → `A ≠ B`
      `And X Y` → `X ∧ Y` / `Or` → `∨` / `Iff` → `↔` / `Not X` → `¬ X`
      `forall (x : T), p x` → `∀ (x : T), p x`
      `Exists T (fun (x : T) => p x)` → `∃ (x : T), p x`
      `->` → `→`
      `Set.mem α a A` → `a ∈ A`；`Set.subset α A B` → `A ⊆ B`；`Set.union/inter/
      sdiff/compl/powerset/empty/image/preimage/prod/singleton/pair` 同理
  · 基础类型的**隐式实参**与 Lean 对齐：`And.intro h1 h2` / `And.left h` /
    `Or.inl h` / `Exists.intro w hw` ……（写全 A B 的就是旧写法）
  · **代码与注释（含 `-- soko:hint`）都算**；tactic 块不动，term 可以保留。

**明说的边界（脚本按约定不报，别当成已对齐）**：
  · 宇宙多态的等式族**证明项** `Eq.refl` / `Eq.symm` / `Eq.trans` / `Eq.subst` /
    `Eq.rec` / `Eq.ndrec` / `Eq.mp` / `Eq.mpr` / `cast` / `congrArg`：仍要显式宇宙与
    参数（应用路径的宇宙层级推断是独立的一刀）。注意 `congrArg` 的参数顺序已改成
    Lean 的 `{α β} {a b} (f) (h)`——旧顺序 `congrArg.{1} α β f a b h` 会判红，
    脚本抓不到顺序，靠 `grade` 兜底。
  · `Set.univ α`：没有记法，且零元应用不在隐式插入覆盖内。

豁免（脚本内置，报告里会注明）：
  · `units/notation-cheatsheet*.sokonanoda`：教学装置，**故意**并列点名 ↔ 记法；
  · 行内/上一行写了 `-- soko:notation-ok: <理由>` 的旧写法（引擎边界等）。

用法：
    python3 scripts/notation-lint.py                 # 人读报告；有残留 ⇒ exit 1
    python3 scripts/notation-lint.py --json          # 单 JSON 对象
    python3 scripts/notation-lint.py --root courses/set-theory
    python3 scripts/notation-lint.py --list          # 只列文件名 + 计数

契约（与 docs/protocol.md 的 ok/exit 同口径）：
    exit 0 = 零残留；exit 1 = 有残留；exit 2 = 用法/IO 错误。
    `ok:false` 不是空结果——`--json` 永远给 counts 与 hits。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

# ── 扫描范围（用户拍板：卷 I + 入门课 + 共享画布）──────────────────────────
DEFAULT_ROOTS = [
    "courses/set-theory",
    "course",
    "playground.sokonanoda",
]

# ── 豁免（整文件）────────────────────────────────────────────────────────
EXEMPT_FILES = [
    "courses/set-theory/units/notation-cheatsheet.sokonanoda",
    "courses/set-theory/units/solutions/notation-cheatsheet-solution.sokonanoda",
]

MARKER = "soko:notation-ok"

# ── 旧写法：前缀形式的逻辑连接符 / 量词 ───────────────────────────────────
PREFIX_LOGIC = [
    ("And X Y", re.compile(r"\bAnd\s+[A-Za-z(_]"), "X ∧ Y"),
    ("Or X Y", re.compile(r"\bOr\s+[A-Za-z(_]"), "X ∨ Y"),
    ("Iff X Y", re.compile(r"\bIff\s+[A-Za-z(_]"), "X ↔ Y"),
    ("Not X", re.compile(r"\bNot\s+[A-Za-z(_]"), "¬ X"),
    ("forall", re.compile(r"\bforall\b"), "∀"),
    ("Exists X …", re.compile(r"\bExists\s+[A-Za-z(_]"), "∃ (x : T), p x"),
    ("->", re.compile(r"->"), "→"),
]

# ── 旧写法：显式宇宙/类型的点名叫法 ──────────────────────────────────────
POINTFUL = [
    ("Eq.{…}", re.compile(r"\bEq\.\{"), "a = b"),
    ("Ne.{…}", re.compile(r"\bNe\.\{"), "a ≠ b"),
    ("Set.mem", re.compile(r"\bSet\.mem\s"), "a ∈ A"),
    ("Set.subset", re.compile(r"\bSet\.subset\s"), "A ⊆ B"),
    ("Set.union", re.compile(r"\bSet\.union\s"), "A ∪ B"),
    ("Set.inter", re.compile(r"\bSet\.inter\s"), "A ∩ B"),
    ("Set.sdiff", re.compile(r"\bSet\.sdiff\s"), "A \\ B"),
    ("Set.compl", re.compile(r"\bSet\.compl\s"), "Aᶜ"),
    ("Set.powerset", re.compile(r"\bSet\.powerset\s"), "𝒫 A"),
    ("Set.empty", re.compile(r"\bSet\.empty\s"), "∅"),
    ("Set.image", re.compile(r"\bSet\.image\s"), "f '' A"),
    ("Set.preimage", re.compile(r"\bSet\.preimage\s"), "f ⁻¹' B"),
    ("Set.prod", re.compile(r"\bSet\.prod\s"), "A ×ˢ B"),
    ("Set.singleton", re.compile(r"\bSet\.singleton\s"), "{a}"),
    ("Set.pair", re.compile(r"\bSet\.pair\s"), "{a, b}"),
]

# ── **从记法声明自动派生**的点形式判据（2026-09-25：用户报"不通配"之后的修法）
#
# 起因：判据原来是一张**手维护**的表 —— 只能抓"当初想到的"点形式，每加一条新记法
# 就得有人记得回来补一行 ✗（用户原话："现在方案不是通配的，而是拆东墙补西墙的吗？"）。
# 现在改成：**扫仓库里所有记法声明**（`infix[lr]:N " sym " => Head` /
# `notation " sym " => Head`），为每个 **Head** 自动生成"源码里出现 `Head ` 应用
# ⇒ 判红、并告诉作者该写哪个符号" ✓。将来任何新记法（lib / 课程 / prelude 新增）
# **自动进判据** ✓，不需要再有人回来加一行 ✓。
# 手写表只作为**底座**（内核内建记法的声明不在 `.sokonanoda` 里，派不出来）✓。
NOTATION_DECL = re.compile(
    r'^\s*(?:infix[lr]?|prefix|postfix):\d+\s+"([^"]+)"\s*=>\s*([A-Za-z_][\w.]*)'
)
NOTATION_PLAIN = re.compile(r'^\s*notation\s+"([^"]+)"\s*=>\s*([A-Za-z_][\w.]*)')


def derived_pointful(files) -> list[tuple[str, re.Pattern, str]]:
    """扫 `files` 里的记法声明，生成点形式判据（按 Head 去重）。"""
    out: dict[str, tuple[str, re.Pattern, str]] = {}
    for path in files:
        try:
            text = path.read_text(encoding="utf-8")
        except OSError:
            continue
        for raw in text.splitlines():
            m = NOTATION_DECL.match(raw) or NOTATION_PLAIN.match(raw)
            if not m:
                continue
            sym, head = m.group(1).strip(), m.group(2)
            out.setdefault(
                head,
                (head, re.compile(r"\b" + re.escape(head) + r"\s"), sym),
            )
    return list(out.values())


# 由 `Linter.__init__` 填充（= 手写底座 + 从记法声明派生）。见 `merged_pointful`。
POINTFUL_ALL: list[tuple[str, re.Pattern, str]] = []


def merged_pointful(files) -> list[tuple[str, re.Pattern, str]]:
    """手写底座 + 从声明派生（按 label 去重，派生优先保留手写的修法提示）。"""
    seen = {label for label, _, _ in POINTFUL}
    out = list(POINTFUL)
    for label, pat, fix in derived_pointful(files):
        if label not in seen:
            seen.add(label)
            out.append((label, pat, fix))
    return out

# ── 旧写法：基础类型点名调用写全了前导隐式实参 ────────────────────────────
# name → (新写法允许的显式实参个数, 记法提示)。计数 > 允许值 ⇒ 旧写法。
PRELUDE_CALLS = {
    "And.intro": (2, "And.intro h1 h2"),
    "And.left": (1, "And.left h"),
    "And.right": (1, "And.right h"),
    "And.elim": (2, "And.elim f h"),
    "Or.inl": (1, "Or.inl h"),
    "Or.inr": (1, "Or.inr h"),
    "Or.elim": (3, "Or.elim f g h"),
    "Iff.intro": (2, "Iff.intro h1 h2"),
    "Iff.mp": (2, "Iff.mp h / Iff.mp h a"),
    "Iff.mpr": (2, "Iff.mpr h / Iff.mpr h a"),
    "Iff.refl": (0, "Iff.refl"),
    "Iff.symm": (1, "Iff.symm h"),
    "Iff.trans": (2, "Iff.trans h1 h2"),
    "Not.intro": (1, "Not.intro f"),
    "Not.elim": (2, "Not.elim h a"),
    "False.elim": (1, "False.elim h"),
    "absurd": (2, "absurd ha hna"),
    "Exists.intro": (2, "Exists.intro w hw"),
    "Exists.elim": (2, "Exists.elim h f"),
}

# `And.intro` 这类名字若在**声明位**（本文件定义它）出现，不算旧写法。
DECL_KEYWORDS = (
    "inductive",
    "ctor",
    "def",
    "theorem",
    "axiom",
    "example",
    "iota",
    "infix",
    "infixl",
    "infixr",
    "prefix",
    "postfix",
    "notation",
    "binder_notation",
)

_ATOM_STOP = set(" \t()[]{}<>,:;\"`")


def split_line(line: str) -> tuple[str, str]:
    """把一行切成 (代码, 注释)；注释保留 `--` 在内的原文。

    字符串字面量里的 `--` 不算注释（记法声明 `infix:50 " ∈ " => …` 里有 `"`）。
    """
    in_str = False
    i = 0
    n = len(line)
    while i < n:
        c = line[i]
        if c == '"':
            in_str = not in_str
            i += 1
            continue
        if not in_str and c == "-" and i + 1 < n and line[i + 1] == "-":
            return line[:i], line[i:]
        i += 1
    return line, ""


def _skip_group(text: str, i: int) -> int:
    """`text[i]` 是 `(`/`{`/`[` ⇒ 返回匹配右括号之后的下一位置。"""
    depth = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c in "({[":
            depth += 1
        elif c in ")}]":
            depth -= 1
            if depth == 0:
                return i + 1
        elif c == '"':
            i += 1
            while i < n and text[i] != '"':
                i += 1
        i += 1
    return i


def count_explicit_args(text: str, start: int) -> int:
    """数 `start` 之后、同一层里的实参个数（用于区分短写法与写全的旧写法）。

    只认「看起来像代码」的实参：ASCII 标识符/数字/`_`/`'`、`(`/`{`/`[` 组，
    或 `@` 前缀。中文散文里的 `` `And.left` 把… `` 因此不会被误数。
    """
    i = start
    n = len(text)
    args = 0
    while i < n:
        while i < n and text[i] in " \t":
            i += 1
        if i >= n:
            break
        c = text[i]
        if c in ")]},:;`":
            break
        if text.startswith("--", i) or text.startswith(":=", i):
            break
        if c == "@":
            i += 1
            continue
        if c in "({[":
            i = _skip_group(text, i)
            args += 1
            continue
        if c.isascii() and (c.isalnum() or c == "_"):
            j = i
            while j < n and text[j].isascii() and (
                text[j].isalnum() or text[j] in "_.'"
            ):
                j += 1
            # 类型标注 `(x : T)` 之类由上面的 `:` 挡住；这里把原子吃掉。
            i = j
            args += 1
            continue
        # 不是代码字符（中文、运算符、`→` 等）⇒ 实参到头了
        break
    return args


def _universe_suffix_end(text: str, i: int) -> int:
    """`Name.{u}` / `Name.{1, 2}` 之后的位置；没有则原样返回。"""
    if i < len(text) and text[i] == "." and i + 1 < len(text) and text[i + 1] == "{":
        return _skip_group(text, i + 1)
    return i


def _arity_hits(
    text: str, line_no: int, is_comment: bool, offset: int = 0
) -> list[dict]:
    """基础类型点名叫法的实参计数（`And.left A B h` ⇒ 旧写法）。"""
    hits: list[dict] = []
    for name, (allowed, fix) in PRELUDE_CALLS.items():
        for m in re.finditer(r"(?<![\w.])" + re.escape(name) + r"(?![\w])", text):
            after = _universe_suffix_end(text, m.end())
            if after >= len(text) or text[after] not in " \t":
                continue  # 只是提到这个名字（`Or.inl`），不是调用
            args = count_explicit_args(text, after)
            if args > allowed:
                hits.append(
                    {
                        "line": line_no,
                        "col": offset + m.start() + 1,
                        "kind": "comment" if is_comment else "code",
                        "rule": f"{name} 写全了前导参数（{args} 个实参）",
                        "fix": fix,
                        "text": text.strip(),
                    }
                )
    return hits


def scan_text(text: str, line_no: int, is_comment: bool) -> list[dict]:
    """扫一段（代码或注释）文本，返回该段的全部命中。

    注释里的**散文**不做实参计数（"`Eq.refl proves any equality…`" 会被数成
    11 个实参）——只数**反引号里的代码片段**；记法/点名叫法这两类正则仍然扫
    整段注释。
    """
    hits: list[dict] = []
    for label, pat, fix in PREFIX_LOGIC:
        for m in pat.finditer(text):
            hits.append(
                {
                    "line": line_no,
                    "col": m.start() + 1,
                    "kind": "comment" if is_comment else "code",
                    "rule": label,
                    "fix": fix,
                    "text": text.strip(),
                }
            )
    for label, pat, fix in (POINTFUL_ALL or POINTFUL):
        for m in pat.finditer(text):
            hits.append(
                {
                    "line": line_no,
                    "col": m.start() + 1,
                    "kind": "comment" if is_comment else "code",
                    "rule": label,
                    "fix": fix,
                    "text": text.strip(),
                }
            )
    if is_comment:
        for seg in text.split("`")[1::2]:  # 反引号里的代码片段
            off = text.index("`" + seg) + 1 if ("`" + seg) in text else 0
            hits.extend(_arity_hits(seg, line_no, True, off))
    else:
        hits.extend(_arity_hits(text, line_no, False))
    return hits


class Linter:
    def __init__(self, roots: list[Path]) -> None:
        self.roots = roots
        # 判据 = 手写底座 + **从记法声明派生**（见 derived_pointful 的说明）。
        # 存成模块级全局：消费点 `scan_text` 是自由函数，拿不到 `self` ✓。
        global POINTFUL_ALL
        POINTFUL_ALL = merged_pointful(self.files())

    def files(self) -> list[Path]:
        out: list[Path] = []
        for root in self.roots:
            if root.is_file():
                out.append(root)
            elif root.is_dir():
                # **只收真正的文件**：R-3 之后模块根下多了个 `.sokonanoda/`
                # 产物**目录**，而它的名字正好以 `.sokonanoda` 结尾 ⇒
                # `rglob("*.sokonanoda")` 会把它当命中，`read_text()` 随即抛
                # `IsADirectoryError` ✗ —— 那会让这条门禁**从判卷退化成崩溃**
                # （崩了就不判，等于没有守卫 ✗）。按"是不是文件"过滤是**通用**修法：
                # 以后任何同名目录/软链都不会再把它打崩 ✓。
                out.extend(sorted(p for p in root.rglob("*.sokonanoda") if p.is_file()))
        return out

    def scan_file(self, path: Path) -> tuple[list[dict], str]:
        rel = path.resolve().relative_to(REPO).as_posix()
        if rel in EXEMPT_FILES:
            return [], "exempt:cheatsheet"
        src = path.read_text(encoding="utf-8")
        lines = src.splitlines()
        hits: list[dict] = []
        current_inductive: str | None = None
        for idx, raw in enumerate(lines):
            code, comment = split_line(raw)
            # `inductive`/`ctor` 的**声明行**：本行里归纳类型自己的名字允许
            # `Exists A p` 这种结果位写法（那是定义，不是使用）。
            exempt_names: list[str] = []
            stripped = code.strip()
            first = stripped.split(None, 1)[0] if stripped else ""
            tokens = stripped.split()
            # **声明位**（`axiom` / `inductive` / `ctor`）：整行的代码是「定义」，
            # 不是「使用」——`axiom Exists.intro : … → Exists A p` 的结果位必须留
            # 点名形式（引擎按声明装它）。`def`/`theorem` 不整行豁免：它们的**类型**
            # 是使用点，照样要写记法。
            decl_line = first in ("axiom", "inductive", "ctor")
            if first == "inductive" and len(tokens) > 1:
                current_inductive = tokens[1].split("(")[0].split(":")[0].strip()
                exempt_names.append(current_inductive)
            elif first in DECL_KEYWORDS and len(tokens) > 1:
                # 匿名 `example : …` 的第二个 token 是 `:`（或带标注时的 `(`）⇒
                # 名字为空；空名字会让下面的豁免正则匹配一切、**静默吞掉整行**。
                decl_name = tokens[1].split("(")[0].split(":")[0].strip()
                if decl_name:
                    exempt_names.append(decl_name)
                if first == "ctor" and current_inductive:
                    exempt_names.append(current_inductive)
            local: list[dict] = []
            if code.strip() and not decl_line:
                local.extend(scan_text(code, idx + 1, False))
            if comment.strip():
                local.extend(scan_text(comment, idx + 1, True))
            for h in local:
                # 声明位豁免只免**被声明那个名字自己的**旧写法（`inductive
                # Exists` 行里的 `Exists A p`、`ctor intro` 的结果位），不免同一行
                # 里别的连接符：`def Iff (A B) := And …` 的 `And` 仍要报。
                rule_head = h["rule"].split()[0] if h["rule"] else ""
                heads = {rule_head}
                if "." in rule_head:
                    heads.add(rule_head.split(".")[0])
                if heads & set(exempt_names):
                    continue
                # `soko:notation-ok` 标记：本行或上一行写了就豁免本行。
                window = raw + (lines[idx - 1] if idx > 0 else "")
                if MARKER in window:
                    h["exempt"] = MARKER
                    continue
                hits.append(h)
        return hits, "scanned"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="notation-lint.py",
        description="courses 记法规则检查器（旧写法 → Lean 4 记法）",
    )
    parser.add_argument(
        "--root",
        action="append",
        default=None,
        help="扫描根（可重复；默认 courses/set-theory + course + playground.sokonanoda）",
    )
    parser.add_argument("--json", action="store_true", help="单 JSON 对象输出")
    parser.add_argument("--list", action="store_true", help="只列文件与计数")
    parser.add_argument("--quiet", action="store_true", help="无残留时不打印")
    args = parser.parse_args(argv)

    roots = [Path(r) for r in (args.root or DEFAULT_ROOTS)]
    missing = [str(r) for r in roots if not (REPO / r if not r.is_absolute() else r).exists()]
    if missing:
        print(f"notation-lint: 扫描根不存在：{', '.join(missing)}", file=sys.stderr)
        return 2
    resolved = [(r if r.is_absolute() else REPO / r) for r in roots]

    linter = Linter(resolved)
    per_file: list[dict] = []
    total = 0
    for path in linter.files():
        hits, status = linter.scan_file(path)
        if status.startswith("exempt"):
            per_file.append(
                {
                    "file": path.resolve().relative_to(REPO).as_posix(),
                    "exempt": status.split(":", 1)[1],
                    "hits": [],
                }
            )
            continue
        code = sum(1 for h in hits if h["kind"] == "code")
        comment = len(hits) - code
        total += len(hits)
        per_file.append(
            {
                "file": path.resolve().relative_to(REPO).as_posix(),
                "code": code,
                "comment": comment,
                "hits": hits,
            }
        )

    scanned = sum(1 for f in per_file if "exempt" not in f)
    if args.json:
        print(
            json.dumps(
                {
                    "ok": total == 0,
                    "scanned": scanned,
                    "files_with_hits": sum(1 for f in per_file if f.get("hits")),
                    "counts": {"total": total},
                    "files": per_file,
                },
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0 if total == 0 else 1

    if args.list:
        for f in per_file:
            if f.get("exempt"):
                print(f"  (exempt:{f['exempt']})  {f['file']}")
            else:
                n = len(f["hits"])
                if n:
                    print(f"  {n:4}  {f['file']}  (code {f['code']} / comment {f['comment']})")
        print(f"TOTAL {total}")
        return 0 if total == 0 else 1

    if total == 0:
        if not args.quiet:
            print(f"notation-lint: OK —— {scanned} 个文件零旧写法")
        return 0

    print(f"notation-lint: {total} 处旧写法（{scanned} 个文件）\n")
    for f in per_file:
        if not f.get("hits"):
            continue
        print(f"### {f['file']}  (code {f['code']} / comment {f['comment']})")
        for h in f["hits"]:
            print(
                f"  {h['line']:>5}:{h['col']:<3} [{h['kind']}] {h['rule']}"
                f"   →  {h['fix']}"
            )
        print()
    return 1


if __name__ == "__main__":
    sys.exit(main())
