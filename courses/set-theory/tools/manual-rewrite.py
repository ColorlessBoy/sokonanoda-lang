#!/usr/bin/env python3
"""#5 **manual 桶的真改写器**：点名 → 中缀/前缀/后缀记法（`Set.mem α a A` ⇒ `a ∈ A` ✓）。

口径 ✓：
  · 记法表**从课程树里的记法声明读**（`infix[lr]:N " sym " => Head` / `prefix` / `postfix` / `notation` ✓）
    ⇒ 加一条新记法不用回来改本工具 ✓（不再"拆东墙补西墙" ✓）。
  · 模板按 kind 定型：infix ⇒ 取**最后 2 个实参**（`A SYM B` ✓）；prefix/postfix ⇒ 最后 1 个 ✓；
    `notation`（如 `∅`）⇒ 无操作数 ✓。**前导类型参数自然被丢掉** ✓（它们不在最后 k 个里 ✓）。
  · 判卷口径 = 与普查一致：**前缀判卷**（前言 + 该声明 ✓）⇒ 只判"这一处改得对不对" ✓。
  · ⚠ 本工具**默认只出候选与判卷**（`--sample` / `--dry` ✓）；`--land` 才真改，且逐文件双判据 + 红即回滚 ✓。

用法：
    python3 courses/set-theory/tools/manual-rewrite.py --sample 25      # 抽 25 处判卷（不改树 ✓）
    python3 courses/set-theory/tools/manual-rewrite.py --dry            # 全量只出候选（不改树 ✓）
"""
from __future__ import annotations
import argparse, collections, importlib.util, json, re, subprocess, sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
DECL = re.compile(r"^(theorem|def|abbrev|axiom|inductive|example|instance|structure)\b")


def _lint():
    spec = importlib.util.spec_from_file_location("nl", REPO / "scripts/notation-lint.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    mod.Linter([mod.REPO / r for r in mod.DEFAULT_ROOTS])
    return mod


BUILTIN_NOTATION = {"Eq": ("infix", "="), "Ne": ("infix", "≠"),
                    "Set.singleton": ("delim", "{a}"), "Set.pair": ("delim", "{a, b}")}


def notation_map(nl) -> dict[str, tuple[str, str]]:
    """head → (kind, symbol) ✓ —— 只认**课程树里真声明过**的记法 ✓。"""
    out: dict[str, tuple[str, str]] = {}
    files = subprocess.run(["git", "ls-files", "*.sokonanoda"], cwd=REPO,
                           capture_output=True, text=True).stdout.split()
    for rel in files:
        p = REPO / rel
        if not p.exists():
            continue
        for raw in p.read_text(encoding="utf-8").splitlines():
            m = nl.NOTATION_DECL.match(raw)
            if m:
                kind = raw.strip().split(":")[0]
                out.setdefault(m.group(2), (kind, m.group(1).strip()))
            else:
                m2 = nl.NOTATION_PLAIN.match(raw)
                if m2:
                    out.setdefault(m2.group(2), ("notation", m2.group(1).strip()))
    for k, v in BUILTIN_NOTATION.items():          # 内建/带定界符的记法 ✓
        out.setdefault(k, v)
    out.setdefault("Eq.{…}", BUILTIN_NOTATION["Eq"])   # ⚠ 命中的 rule 名带 `.{…}` ✓
    out.setdefault("Ne.{…}", BUILTIN_NOTATION["Ne"])
    return out


def decl_end(lines, idx):
    starts = [i for i, l in enumerate(lines) if DECL.match(l)]
    for k, s in enumerate(starts):
        e = starts[k + 1] if k + 1 < len(starts) else len(lines)
        if s <= idx < e:
            return e
    return len(lines)


def _skip_group(text: str, i: int) -> int:
    """跳过 `(…)`/`[…]`/`{…}`（含嵌套 ✓）。"""
    pairs = {"(": ")", "[": "]", "{": "}"}
    depth, j, n = 0, i, len(text)
    while j < n:
        if text[j] in pairs:
            depth += 1
        elif text[j] in ")]}":
            depth -= 1
            if depth == 0:
                return j + 1
        j += 1
    return n


def _args(code: str, start: int) -> list[tuple[int, int]]:
    """**Unicode 版**实参切分 ✓ —— 模块的 `_arg_spans` 要求 `c.isascii()` ✗ ⇒ `α`/`β` 一进来就 break ✗
    （实测 `Set.sUnion α F` 拿到 [] ✗）⇒ 本工具自带一份（非 ASCII 标识符照收 ✓，`∈`/`∩` 这类符号
    不是 alnum ⇒ 仍然正确地终止 ✓）。"""
    spans, i, n = [], start, len(code)
    while i < n:
        while i < n and code[i] in " \t":
            i += 1
        if i >= n:
            break
        c = code[i]
        if c in ")]},;:`":
            break
        if code.startswith("--", i) or code.startswith(":=", i) or code.startswith("->", i):
            break
        if c in "([{":
            j = _skip_group(code, i)
            spans.append((i, j)); i = j; continue
        if c.isalnum() or c == "_":                    # ⚠ 不限 ASCII ✓（α/β/γ 是这门课的常客 ✓）
            j = i
            while j < n and (code[j].isalnum() or code[j] in "_.'"):
                j += 1
            spans.append((i, j)); i = j; continue
        break                                          # 记法符号（∈/∩/⋃₀…）⇒ 实参到此为止 ✓
    return spans


def rewrite_line(nl, lines, hit, nots):
    """返回 (新行列表 | None, 说明) —— 只改**这一处**（同一行的其它命中不动 ✓）。"""
    raw = lines[hit["line"] - 1]
    code, _c = nl.split_line(raw)
    i = hit["col"] - 1
    m = re.match(r"(Eq|Ne)\.\{[^}]*\}|\b[A-Za-z_][\w.]*", code[i:])
    if not m:
        return None, "no-head"
    head = m.group(0)
    base = head.split(".")[0] + "." + head.split(".")[1] if head.startswith(("Eq.", "Ne.")) else head
    key = "Eq.{…}" if head.startswith("Eq.") else "Ne.{…}" if head.startswith("Ne.") else head
    if key not in nots and head not in nots:
        return None, f"无记法声明({head})"
    kind, sym = nots.get(key) or nots[head]
    after = i + len(head)                    # ⚠ 不用 `_universe_suffix_end`：它会越过实参 ✗（实测 [] ✗）
    spans = _args(code, after)
    if kind == "delim":
        k = 1 if sym == "{a}" else 2
    elif kind == "notation":
        k = 0
    elif kind in ("infix", "infixl", "infixr"):
        k = 2
    else:
        k = 1
    if len(spans) < k or (k == 0 and not spans and kind != "notation"):
        return None, "实参不足"
    if k == 0:
        rep = sym
        end = spans[-1][1] if spans else after
    else:
        ops = [code[a:b] for a, b in spans[-k:]]
        if kind == "delim":
            rep = "{" + ops[0] + "}" if k == 1 else "{" + ops[0] + ", " + ops[1] + "}"
        else:
            rep = (f"{ops[0]} {sym} {ops[1]}" if k == 2
                   else (f"{sym} {ops[0]}" if kind.startswith("prefix") else f"{ops[0]} {sym}"))
        end = spans[-1][1]
    new = list(lines)
    new[hit["line"] - 1] = code[:i] + rep + code[end:] + nl._comment_of(raw)
    return new, f"{kind}:{sym}"


def grade(nl, cand, line_idx, rel=None):
    probe = cand[:decl_end(cand, line_idx)]
    nl.CENSUS_SCRATCH.mkdir(parents=True, exist_ok=True)      # ⚠ 副件在 /tmp（**不进语料树** ✗）
    nl.CENSUS_PROBE.write_text("\n".join(probe) + "\n", encoding="utf-8")
    try:
        rc, data = nl.census_grade(nl.CENSUS_PROBE, nl.module_root_for(rel) if rel else None)
        failed = data.get("failed") or []
        return (rc == 0 and not failed), ((failed[0].get("message", "") if failed else "")[:110])
    finally:
        if nl.CENSUS_PROBE.exists():
            nl.CENSUS_PROBE.unlink()


def collect(nl, nots):
    files = subprocess.run(["git", "ls-files", "*.sokonanoda"], cwd=REPO,
                           capture_output=True, text=True).stdout.split()
    out = []
    for rel in files:
        p = REPO / rel
        if not p.exists() or nl._is_probe(p):
            continue
        lines = p.read_text(encoding="utf-8").splitlines()
        for h in nl.census_hits(p):
            cand, cls = nl.census_candidate(lines, h)
            if cand is not None or cls == "comment-skip":
                continue                                  # 机械类/注释类不归本工具 ✓
            new, note = rewrite_line(nl, lines, h, nots)
            if new is None:
                continue
            out.append({"file": rel, "line": h["line"], "rule": h["rule"], "note": note,
                        "before": lines[h["line"] - 1].strip(), "after": new[h["line"] - 1].strip()})
    return out


def _hits_on(nl, lines, idx):
    code, comment = nl.split_line(lines[idx])
    out = []
    if code.strip(): out += nl.scan_text(code, idx + 1, False)
    if comment.strip(): out += nl.scan_text(comment, idx + 1, True)
    return out


def land(nl, nots, sample_json: str) -> int:
    """把样本里**判绿**的条目落地 ✓：逐文件改（右→左 ✓）+ 删不再需要的豁免标记 + 双判据 ✓。"""
    rows = [r for r in json.loads(Path(sample_json).read_text(encoding="utf-8"))
            if r.get("verdict") == "green"]
    byf = collections.defaultdict(list)
    for r in rows: byf[r["file"]].append(r)
    landed = rev = rev_hits = 0
    for rel, rs in sorted(byf.items()):
        p = REPO / rel
        lines = p.read_text(encoding="utf-8").splitlines()
        picked = []
        for r in rs:
            hit = [h for h in nl.census_hits(p) if h["line"] == r["line"] and h["rule"] == r["rule"]]
            if not hit: continue
            new, _note = rewrite_line(nl, lines, hit[0], nots)
            if new is None: continue
            picked.append((r, hit[0]))
        if not picked: continue
        applied = []
        for r, h in sorted(picked, key=lambda t: (t[1]["line"], t[1]["col"]), reverse=True):
            new, _n = rewrite_line(nl, lines, h, nots)
            if new is None: continue
            lines = new; applied.append(r)
        if not applied: continue
        need = set()
        for i in range(len(lines)):
            if _hits_on(nl, lines, i): need.add(i); need.add(i - 1)
        out, removed = [], 0
        for i, ln in enumerate(lines):
            if nl.MARKER in ln and i not in need:
                removed += 1
                code, comment = nl.split_line(ln)
                if comment.strip() and code.strip(): out.append(code.rstrip())
                continue
            out.append(ln)
        p.write_text("\n".join(out) + "\n", encoding="utf-8")
        lint = subprocess.run([sys.executable, "scripts/notation-lint.py", "--root", rel,
                               "--root", "courses/set-theory/lib"], cwd=REPO,
                              capture_output=True, text=True)
        g = subprocess.run(["node", "scripts/soko", "query", "check", "--file", rel], cwd=REPO,
                           capture_output=True, text=True)
        try:
            data = json.loads(g.stdout)["data"]; failed = data["failed"]; cnt = data["counts"]
        except Exception:
            failed, cnt = [{"message": "parse"}], {}
        if lint.returncode == 0 and g.returncode == 0 and not failed:
            landed += len(applied)
            print(f"  ✓ {rel}: 改 {len(applied)} · 删标记 {removed} · checked={cnt.get('decl_checked')}", flush=True)
        else:
            subprocess.run(["git", "checkout", "--", rel], check=True, cwd=REPO)
            rev += 1; rev_hits += len(applied)
            print(f"  ✗ {rel}: 改 {len(applied)} ⇒ **回滚**（lint={lint.returncode} grade={g.returncode} "
                  f"failed={len(failed)}）", flush=True)
    print(f"manual 落地：**{landed} 处** · 回滚 {rev_hits} 处（{rev} 文件）✓", flush=True)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(prog="manual-rewrite.py")
    ap.add_argument("--sample", type=int, default=0)
    ap.add_argument("--dry", action="store_true")
    ap.add_argument("--land", default=None,
                    help="把样本判绿的条目**真落地**（逐文件双判据 + 红即回滚 ✓）")
    a = ap.parse_args()
    nl = _lint()
    nots = notation_map(nl)
    if a.land:
        return land(nl, nots, a.land)
    cands = collect(nl, nots)
    print(f"manual 候选改写：**{len(cands)}** 处 / {len(set(c['file'] for c in cands))} 文件 "
          f"（记法表 {len(nots)} 条 ✓）")
    if a.dry or not a.sample:
        json.dump(cands, open("/tmp/manual-cands.json", "w"), ensure_ascii=False, indent=1)
        print("→ /tmp/manual-cands.json ✓（未改树 ✓）")
        return 0
    # 抽样：按 rule 分层（每规则轮流取 ✓），跑判卷 ✓
    byrule = collections.defaultdict(list)
    for c in cands:
        byrule[c["rule"]].append(c)
    sample, i = [], 0
    while len(sample) < a.sample and any(len(v) > i for v in byrule.values()):
        for r in sorted(byrule):
            if len(byrule[r]) > i and len(sample) < a.sample:
                sample.append(byrule[r][i])
        i += 1
    res = []
    for k, c in enumerate(sample, 1):
        p = REPO / c["file"]
        lines = p.read_text(encoding="utf-8").splitlines()
        hit = [h for h in nl.census_hits(p) if h["line"] == c["line"] and h["rule"] == c["rule"]]
        if not hit:
            res.append({**c, "verdict": "skip"}); continue
        new, note = rewrite_line(nl, lines, hit[0], nots)
        if new is None:
            res.append({**c, "verdict": "no-rewrite", "note": note}); continue
        ok, msg = grade(nl, new, c["line"] - 1, c["file"])
        res.append({**c, "verdict": "green" if ok else "red", "diag": msg})
        print(f"  [{k}/{len(sample)}] {'绿 ✓' if ok else '红 ✗'} {c['file'].split('/')[-1]}:{c['line']} "
              f"· {note} · {c['rule'][:14]}" + ("" if ok else f"\n        {msg}"), flush=True)
    json.dump(res, open("/tmp/manual-sample.json", "w"), ensure_ascii=False, indent=1)
    g = sum(1 for r in res if r["verdict"] == "green")
    rd = sum(1 for r in res if r["verdict"] == "red")
    print(f"样本 {len(res)} 处 ⇒ **绿 {g} / 红 {rd}**（其余 skip/no-rewrite）· 绿率 {100*g/max(1,g+rd):.0f}%")
    print("按规则：", dict(collections.Counter((r["rule"][:12], r["verdict"]) for r in res)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
