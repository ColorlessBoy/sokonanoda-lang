#!/usr/bin/env python3
"""**冻结预算的「余量」守卫** —— 洞 `CI-FAILURES-FROZEN-BUDGET-BLOCKS-MANDATORY-APPEND`（2026-10-16）。

## 为什么有这个脚本（"两条纪律互相顶住"的解药）

`AGENTS.md`「CI 失败记录」写着**必须追加**：*每次 CI 红了，在 `docs/CI-FAILURES.md`
追加一条（原因/修复/预防）*。而 `scripts/docs-budget.json` 的 `frozen` 表把它**冻结在
749 行**、注释写着**只许减不许增**。2026-10-16 实测：**该文件正好 749 行 ⇒ 余量 0** ⇒
**下一次强制追加必然撞判据 ④** ✗ —— 而"抬上限"被用户 2026-09-26 明令禁止 ✗
（「把上限从 3 MB 抬到 10 MB **不算完成**」）⇒ 唯一的出路是**归档腾行**（判据 ⑥）。

⇒ 本守卫把「**强制追加的文档不许把余量吃到 0**」变成**可查的判据**：
余量 < `RESERVE_LINES` ⇒ 判红，且**红里带出路**（归档 + 点名，不是抬上限 ✗）。

## 判据（三条）

* **① 余量**（红线）：`APPEND_MANDATED` 里的每个文档，`上限 − 实测行数 ≥ RESERVE_LINES` ✓。
* **② 表不腐烂**：`APPEND_MANDATED` 的每个键都必须在 `docs-budget.json` 的 `frozen` 里
  （否则本守卫量的是一个**不受冻结**的文件 ⇒ 空转 ✗）。
* **③ 只报告、不判红**（**防假红生成器** ✗）：**其它**冻结文档余量 0 ⇒ 只印 `ⓘ`
  —— 它们是**事件驱动**的（"下次改它时顺手归档"），不像 `CI-FAILURES.md` 有**强制追加**
  的纪律顶着；把 31 份都判红会造出一条**没有出路**的红 ✗（实测：2026-10-16 有 **32** 份余量 0）。

## 用法

```bash
python3 scripts/check-frozen-headroom.py                  # 判据 ①②③（0=健康 / 1=判红 / 3=环境错）
python3 scripts/check-frozen-headroom.py --source-rev <rev>   # 在**钉死的旧 rev** 上量（咬住证明用）
python3 scripts/check-frozen-headroom.py --selftest       # 正反两向（含空转闸）
python3 scripts/check-frozen-headroom.py --json
```

⚠ **`--source-rev` 是咬住证明的入口**（`docs/guard-bite.jsonl`）：坏版本**必须钉一个固定的旧 rev**，
**不能写 `HEAD`** ✗（pr1 的坑：修好并提交之后 `HEAD` 就是修好的那一版 ⇒ 逐字节无效 ⇒ 假绿 ✗）。
本脚本自带**空转闸**：那个 rev 上量到的内容与工作区**逐字节相同** ⇒ `exit 3`（**绝不冒充咬住** ✗）。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUDGET = ROOT / "scripts" / "docs-budget.json"

# ⭐ **"强制追加"类**：这些文档有**纪律要求必须追加内容**（不是"下次改它时顺手"）
# ⇒ 余量 0 = **下一次追加必然撞判据 ④** ✗。**唯一一处** ✓（加新条目要同时给理由）。
APPEND_MANDATED: dict[str, str] = {
    "docs/CI-FAILURES.md": "`AGENTS.md`「CI 失败记录」：**每次 CI 红了，追加一条**（原因/修复/预防）",
}

RESERVE_LINES = 20  # 保留余量：至少够一条正常长度的 CI 失败记录（实测新条目 1–3 行 ⇒ 宽裕 ✓）


class EnvError(RuntimeError):
    """环境不在（**不假装判过** ✗）。"""


def _git(*args: str) -> tuple[int, str, str]:
    p = subprocess.run(["git", *args], cwd=str(ROOT), capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr


def frozen_caps() -> dict[str, int]:
    data = json.loads(BUDGET.read_text(encoding="utf-8"))
    fr = data.get("frozen")
    if not isinstance(fr, dict):
        raise EnvError(f"{BUDGET} 里没有 `frozen` 一节 ⇒ 量不了（**不假装判过** ✗）")
    return {k: v for k, v in fr.items() if isinstance(v, int)}


def _count_lines(text: str) -> int:
    if not text:
        return 0
    return text.count("\n") + (0 if text.endswith("\n") else 1)


def measurable(path: str) -> bool:
    """**与判据 ④ 同一个口径**：只量**活文档** ✓。

    `frozen` 表里有**存量条目不是活文档**（实测 2026-10-16：
    `docs/archive/status-removed-rounds-2026-10-13.md.gz` —— **二进制** ⇒ 解不了 utf-8 ✗）——
    判据 ④ 也是**遍历活文档**、拿 `frozen` 当**上限查表** ⇒ 那一条**从不被量** ✗。
    本守卫照同一口径（否则会在一个 `.gz` 上崩掉，而那与"余量"毫无关系 ✗）。
    """
    return not path.endswith(".gz") and not path.startswith("docs/archive/")


def measure_local(path: str) -> int | None:
    if not measurable(path):
        return None
    fp = ROOT / path
    if not fp.exists():
        return None
    try:
        return _count_lines(fp.read_text(encoding="utf-8"))
    except UnicodeDecodeError:
        return None


def measure_at(rev: str, path: str) -> int | None:
    if not measurable(path):
        return None
    rc, out, _err = _git("show", f"{rev}:{path}")
    if rc != 0:
        return None
    return _count_lines(out)


def judge(caps: dict[str, int], measured: dict[str, int | None]) -> tuple[list[str], list[str]]:
    """**纯函数**（自检直接驱动它 ✓）⇒ `(红, 只报告)`。"""
    red: list[str] = []
    notes: list[str] = []
    for path, why in APPEND_MANDATED.items():
        cap = caps.get(path)
        if cap is None:
            red.append(
                f"**② 表不腐烂** ✗：`{path}` 在 `APPEND_MANDATED` 里、却不在 `{BUDGET.name}` 的 "
                f"`frozen` 里 ⇒ 它**不受冻结** ⇒ 本守卫量的是空气 ✗"
                f"（要么登记进 frozen，要么从 APPEND_MANDATED 里去掉）"
            )
            continue
        n = measured.get(path)
        if n is None:
            red.append(f"**① 余量** ✗：`{path}` 量不到行数（文件不在？）⇒ **不假装判过** ✗")
            continue
        slack = cap - n
        if slack < RESERVE_LINES:
            red.append(
                f"**① 余量** ✗：`{path}` **余量 {slack} 行 < 保留 {RESERVE_LINES} 行**"
                f"（上限 {cap} · 实测 {n}）—— 它是「**强制追加**」类（{why}）⇒ "
                f"**下一次追加必然撞判据 ④**（只许减不许增 ✗）。出路 ⭐：**归档旧条目**进 "
                f"`docs/archive/*.gz` 并在 `docs/archive/README.md` **点名**（判据 ⑥）—— "
                f"**不是抬上限** ✗（用户 2026-09-26：「把上限从 3 MB 抬到 10 MB **不算完成**」）"
            )
    for path, cap in sorted(caps.items()):
        if path in APPEND_MANDATED:
            continue
        n = measured.get(path)
        if n is not None and cap - n <= 0:
            notes.append(f"`{path}` 余量 {cap - n} 行（**只报告**：它是事件驱动的，"
                         f"下次改它时顺手归档 ✓ —— 判红会是一条**没有出路**的红 ✗）")
    return red, notes


def _measure_all(caps: dict[str, int], rev: str | None) -> dict[str, int | None]:
    return {p: (measure_at(rev, p) if rev else measure_local(p)) for p in caps}


def _sha_local(path: str) -> str | None:
    if not measurable(path):
        return None
    fp = ROOT / path
    if not fp.exists():
        return None
    return hashlib.sha256(fp.read_bytes()).hexdigest()


def _sha_at(rev: str, path: str) -> str | None:
    if not measurable(path):
        return None
    rc, out, _err = _git("show", f"{rev}:{path}")
    return hashlib.sha256(out.encode("utf-8")).hexdigest() if rc == 0 else None


def idle_problem(rev_sha: dict[str, str | None], local_sha: dict[str, str | None]) -> str | None:
    """**空转闸**（**纯函数** ✓）：`--source-rev` 上量到的强制追加文档与工作区**逐字节相同** ⇒ 换版无效 ✗。

    ⚠ 这正是 **pr1 的坑**：坏版本原写 `HEAD` ⇒ 修好并提交之后 `HEAD` **就是修好的那一版**
    ⇒ `git show HEAD:<f>` 换版**逐字节无效** ⇒ bite rc=0 ⇒ 假绿 ✗ ⇒ 必须钉一个**固定的旧 rev** ✓。
    """
    pairs = [(p, rev_sha.get(p), local_sha.get(p)) for p in APPEND_MANDATED]
    if not pairs or any(r is None or l is None for _p, r, l in pairs):
        return None  # 量不到 ⇒ 由 `--source-rev` 的解析闸与「量不到行数」那条红负责（不在这里冒充 ✓）
    if all(r == l for _p, r, l in pairs):
        return (f"`--source-rev` 上 {len(pairs)} 个强制追加文档与**工作区逐字节相同** ⇒ "
                f"这个 rev **咬不住**（pr1 的坑：坏版本写 `HEAD` 就会这样 ✗）⇒ 换一个**固定的旧 rev** ✓")
    return None


def check(rev: str | None, as_json: bool) -> int:
    caps = frozen_caps()
    if rev:
        rc, _out, _err = _git("rev-parse", "--verify", f"{rev}^{{commit}}")
        if rc != 0:
            print(f"error: `--source-rev {rev}` 解析不到（浅克隆 / 坏 rev）⇒ "
                  f"**不假装判过** ✗（照 `check-threshold-ratchet.py` 的口径：CI 要 `fetch-depth: 0`）",
                  file=sys.stderr)
            return 3
        # ── 空转闸：那个 rev 与工作区**逐字节相同** ⇒ 换版无效 ⇒ 绝不冒充咬住 ✗ ──
        idle = idle_problem({p: _sha_at(rev, p) for p in APPEND_MANDATED},
                            {p: _sha_local(p) for p in APPEND_MANDATED})
        if idle:
            print(f"error: {idle}", file=sys.stderr)
            return 3
    measured = _measure_all(caps, rev)
    red, notes = judge(caps, measured)
    if as_json:
        print(json.dumps({"ok": not red, "rev": rev, "reserve": RESERVE_LINES,
                          "red": red, "notes": notes}, ensure_ascii=False, indent=2))
    else:
        where = f"（在 rev `{rev}` 上量）" if rev else "（在工作区量）"
        print(f"冻结余量守卫{where}：强制追加类 {len(APPEND_MANDATED)} 份 · 保留 {RESERVE_LINES} 行")
        for path, why in APPEND_MANDATED.items():
            cap, n = caps.get(path), measured.get(path)
            mark = "**余量 0**" if (cap is not None and n is not None and cap - n <= 0) else ""
            print(f"  {path:<34} 上限 {cap} · 实测 {n} · 余量 "
                  f"{(cap - n) if (cap is not None and n is not None) else '?'} {mark}")
        for note in notes:
            print(f"  ⓘ {note}")
        if red:
            print(f"✗ 冻结余量守卫：**{len(red)} 条红** ✗")
            for r in red:
                print(f"  - {r}")
        else:
            print("✓ 冻结余量守卫：强制追加类的余量都在保留线之上 ✓ · 表不腐烂 ✓")
    return 1 if red else 0


def selftest() -> int:
    fails: list[str] = []

    def gate(label: str, cond: bool) -> None:
        print(f"  {'✓' if cond else '✗'} {label}")
        if not cond:
            fails.append(label)

    caps = frozen_caps()
    print("① 真状态（工作区）")
    rc = check(None, as_json=False)
    gate("① 真状态 rc=0（余量在保留线之上）", rc == 0)
    for p in APPEND_MANDATED:
        gate(f"② 表不腐烂：`{p}` 在 frozen 里且文件在", p in caps and measure_local(p) is not None)
    gate("② 保留线 > 0", RESERVE_LINES > 0)

    print("③ 反向：余量 0 ⇒ 判红（**咬住本洞那条已存在的坏输入**）")
    zero = {p: caps[p] for p in APPEND_MANDATED}
    measured_zero = {p: caps[p] for p in APPEND_MANDATED}  # 实测 == 上限 ⇒ 余量 0
    red, _notes = judge(zero, measured_zero)
    gate("③ 余量 0 ⇒ 判红 rc 面（judge 出红）", bool(red))
    gate("③ 红里带**出路**（归档 + 点名，不是抬上限）",
         bool(red) and "归档" in red[0] and "不是抬上限" in red[0])
    print("④ 反向：余量正好 = 保留线 ⇒ **不**判红（边界不是假红 ✗）")
    ok = {p: caps[p] for p in APPEND_MANDATED}
    red_ok, _ = judge(ok, {p: caps[p] - RESERVE_LINES for p in APPEND_MANDATED})
    gate("④ 余量 == 保留线 ⇒ 不红", not red_ok)
    print("⑤ 反向：`APPEND_MANDATED` 里的文档不在 frozen ⇒ 判红（防表腐烂 ✗）")
    red_rot, _ = judge({}, {p: 1 for p in APPEND_MANDATED})
    gate("⑤ 不在 frozen ⇒ 判红", bool(red_rot) and "表不腐烂" in red_rot[0])
    print("⑥ 反向：**别的**冻结文档余量 0 ⇒ 只报告、**不判红**（防假红生成器 ✗）")
    caps_other = {p: caps[p] for p in APPEND_MANDATED} | {"fixture/other.md": 10}
    red_other, notes_other = judge(caps_other, {p: 1 for p in APPEND_MANDATED} | {"fixture/other.md": 10})
    gate("⑥ 非强制追加的余量 0 ⇒ 不红 + 有 ⓘ", not red_other and bool(notes_other))
    print("⑦ 空转闸：`--source-rev` 与工作区**逐字节相同** ⇒ 判空转（**绝不冒充咬住** ✗；pr1 的坑）")
    same = {p: "SAME" for p in APPEND_MANDATED}
    gate("⑦ rev 与工作区逐字节相同 ⇒ 空转被点出", idle_problem(same, dict(same)) is not None)
    gate("⑦b 真差异（本洞那条坏输入）⇒ **不**误报空转",
         idle_problem({p: "BAD" for p in APPEND_MANDATED}, same) is None)
    gate("⑦c 量不到（sha 为 None）⇒ 不在这里冒充（交给解析闸）",
         idle_problem({p: None for p in APPEND_MANDATED}, same) is None)
    print("⑧ 空转闸：`--source-rev` 解析不到 ⇒ exit 3（**不假装判过** ✗）")
    rc_bad = check("ffffffffffff", as_json=False)
    gate("⑧ 假 rev ⇒ rc=3", rc_bad == 3)

    if fails:
        print(f"\n✗ selftest 判红 {len(fails)} 条：{' / '.join(fails)}")
        return 1
    print("\n✓ selftest 全过：正对照 2 + 反向 4 + 空转闸 3 + 表不腐烂")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="冻结预算的余量守卫（强制追加类不许把余量吃到 0）")
    ap.add_argument("--check", action="store_true", help="判据 ①②③（默认动作）")
    ap.add_argument("--source-rev", metavar="REV",
                    help="在**钉死的旧 rev** 上量（咬住证明用；与工作区逐字节相同 ⇒ exit 3）")
    ap.add_argument("--selftest", action="store_true", help="正反两向 + 空转闸")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    try:
        return check(args.source_rev, args.json)
    except EnvError as err:
        print(f"error: {err}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
