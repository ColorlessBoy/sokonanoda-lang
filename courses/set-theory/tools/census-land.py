#!/usr/bin/env python3
"""#5 **落地**：把台账里裁定为**绿**的条目真改（改写 + 删不再需要的 `soko:notation-ok` ✓）。

⚠ **arity 模型（§三 的教训，本脚本的核心 ✓）**：`And.left A B h x` 里 `x` 是**结果上的应用实参** ✗，
不是前导实参 ⇒ 正确改法是 `And.left h x` ✓，**不是** `And.left x` ✗。所以落地**只用已验证的那一改法**：
  · `via_alt` 的条目 ⇒ **少删一个前导实参**（第二改法复测判绿的那一版 ✓）；
  · 其余条目 ⇒ 普查改写器那一版（它是在判卷下判绿的 ✓）。
逐文件验：`notation-lint`（课程根 + lib，含派生记法 ✓）+ `query check`（failed=0 ✓）⇒ 红即 `git checkout` 回滚 ✓。

用法：
    python3 courses/set-theory/tools/census-land.py --batch 40        # 落一批
    python3 courses/set-theory/tools/census-land.py --all             # 一直落到没有绿为止 ✓
    python3 courses/set-theory/tools/census-land.py --dry             # 只报还要落多少（不改 ✓）
"""
from __future__ import annotations
import argparse, collections, importlib.util, json, subprocess, sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
LEDGER = REPO / "courses/set-theory/gaps/census-ledger.json"
ROOTS = ["courses/set-theory/units", "courses/set-theory/lib", "course", "playground.sokonanoda"]


def _lint():
    spec = importlib.util.spec_from_file_location("nl", REPO / "scripts/notation-lint.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    mod.Linter([mod.REPO / r for r in mod.DEFAULT_ROOTS])      # ⚠ 派生记法表必须先建 ✓
    return mod


def _hits_on(nl, lines, idx):
    code, comment = nl.split_line(lines[idx])
    out = []
    if code.strip(): out += nl.scan_text(code, idx + 1, False)
    if comment.strip(): out += nl.scan_text(comment, idx + 1, True)
    return out


def _rewrite(nl, lines, rec, via_alt: bool):
    """按**已验证的**改法改一行 ✓（`via_alt` ⇒ 少删一个前导实参 ✓）。"""
    raw = lines[rec["line"] - 1]
    code, _c = nl.split_line(raw)
    head = rec["rule"].split()[0]
    allowed = nl.PRELUDE_CALLS[head][0]
    i = rec["col"] - 1
    if not code.startswith(head, i):
        return None
    after = nl._universe_suffix_end(code, i + len(head))
    spans = nl._arg_spans(code, after)
    keep = len(spans) - allowed - (1 if via_alt else 0)
    if keep < 1 or len(spans) <= allowed:
        return None
    # ⚠ `allowed == 0`（如 `Iff.refl`）⇒ 全删 ⇒ 收尾取**最后一个实参的结尾** ✓（模块同款处理 ✓）
    cut_from = spans[0][0]
    cut_to = spans[keep][0] if keep < len(spans) else spans[-1][1]
    new = list(lines)
    new[rec["line"] - 1] = code[:cut_from] + code[cut_to:] + nl._comment_of(raw)
    return new


def _verify(rel: str):
    # 逐文件验用「该文件 + lib」：lib 里有全部课程记法声明 ⇒ 派生记法表完整 ✓ 且比全仓快 ✓
    # （全仓 lint 与课程门禁在批次收尾各跑一次，作为最终判据 ✓）
    lint = subprocess.run([sys.executable, "scripts/notation-lint.py",
                           "--root", rel, "--root", "courses/set-theory/lib"],
                          capture_output=True, text=True, cwd=REPO)
    grade = subprocess.run(["node", "scripts/soko", "query", "check", "--file", rel],
                           capture_output=True, text=True, cwd=REPO)
    try:
        data = json.loads(grade.stdout)["data"]
        failed = data["failed"]; counts = data["counts"]
    except Exception:
        failed, counts = [{"message": "parse"}], {}
    return lint.returncode, grade.returncode, len(failed), counts


def land(batch: int, do_all: bool, dry: bool) -> int:
    nl = _lint()
    doc = json.loads(LEDGER.read_text(encoding="utf-8"))
    led = doc["ledger"]
    todo = [r for r in led if r["verdict"] == "green"]
    if dry:
        print(f"census-land --dry：待落地 **{len(todo)}** 处 / "
              f"{len(set(r['file'] for r in todo))} 文件 ✓（未改任何内容 ✓）")
        return 0
    tot_landed = tot_files = tot_rev = tot_rev_hits = 0
    while todo:
        chunk = todo[:batch]
        byf = collections.defaultdict(list)
        for r in chunk: byf[r["file"]].append(r)
        ok_files = 0
        for rel, rows in sorted(byf.items()):
            p = REPO / rel
            before = p.read_text(encoding="utf-8")
            lines = before.splitlines()
            hits = nl.census_hits(p)
            picked = []
            for r in rows:
                m = [h for h in hits if h["line"] == r["line"] and h["rule"] == r["rule"]]
                if not m:
                    r["verdict"] = "done"; r["landed"] = "already"; continue
                picked.append((r, m[0]))
            if not picked:
                continue
            # ⚠⚠ **必须基于演进中的 `lines` 逐条改**（从右往左 ✓）：早先每条的 `new` 都基于**采集时**的
            #     原始行 ⇒ 同一文件在一批里只有**最后一次**生效 ✗（实测：日志"改 2"实际只落 1 ✗）。
            applied = []
            for r, h in sorted(picked, key=lambda t: (t[1]["line"], t[1]["col"]), reverse=True):
                new = _rewrite(nl, lines, h, bool(r.get("via_alt")))
                if new is None:
                    r["verdict"] = "done"; r["landed"] = "no-op"; continue
                lines = new
                applied.append(r)
            if not applied:
                continue
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
            lrc, grc, nfail, counts = _verify(rel)
            if lrc == 0 and grc == 0 and nfail == 0:
                ok_files += 1
                for r in applied:                       # ⚠ 只标记**真改上的**那些 ✓
                    r["verdict"] = "done"; r["landed"] = "green"
                tot_landed += len(applied)
                print(f"  ✓ {rel}: 改 {len(applied)} · 删标记 {removed} · checked={counts.get('decl_checked')}", flush=True)
            else:
                subprocess.run(["git", "checkout", "--", rel], check=True, cwd=REPO)
                for r in applied:
                    r["verdict"] = "red"; r["bucket"] = "①def应用补不出前导命题（典型报文 `期望 Sort(0)`）"
                    r["diag"] = "落地判负 ⇒ 回滚（待内核 ✓）"; r["landed"] = "reverted"
                tot_rev += 1; tot_rev_hits += len(applied)
                print(f"  ✗ {rel}: 改 {len(applied)} ⇒ **回滚**（lint={lrc} grade={grc} failed={nfail}）", flush=True)
        tot_files += ok_files
        LEDGER.write_text(json.dumps(doc, ensure_ascii=False, indent=1), encoding="utf-8")
        todo = [r for r in led if r["verdict"] == "green"]
        print(f"[批 {len(chunk)}] 累计落地 {tot_landed} 处 / {tot_files} 文件 · 回滚 {tot_rev_hits} 处 "
              f"({tot_rev} 文件) · 剩绿 {len(todo)}", flush=True)
        if not do_all:
            break
    print(f"census-land 完：**落地 {tot_landed} 处 / {tot_files} 文件 · 回滚 {tot_rev_hits} 处** ✓", flush=True)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(prog="census-land.py")
    ap.add_argument("--batch", type=int, default=40)
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--dry", action="store_true")
    a = ap.parse_args()
    return land(a.batch, a.all, a.dry)


if __name__ == "__main__":
    sys.exit(main())
