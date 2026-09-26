#!/usr/bin/env python3
"""文档预算 lint（用户 2026-09-26 要求「文档太重了，还没实现多少东西文档先爆炸了」✓）
—— 见 `REQUIREMENTS.md` §9 与 `docs/design/docs-diet.md`。

**为什么** ✓：`docs/` 曾 **8.4 MB / 419 文件 / 105,753 行** ✗，而其中"活规范"只占
一小部分 —— 过程记录、已收口批次的计划、调研笔记、遗留临时物全堆在一起 ✗。
**清理一次没用** ✗：没有判据就一定回胖 ✗ ⇒ 把"文档重量"变成**机械可判** ✓。

**六条判据**（逐条可判 ✓）：
  ① **活文档总量** ≤ `MAX_LIVE_BYTES` —— 活文档 = 仓根 `*.md`（git 跟踪）
     + `docs/**`（git 跟踪），**不含** `docs/archive/**`（归档是"可追溯" ✓，
     不是"每次都要读" ✗）；
  ② **单文件** ≤ `MAX_FILE_LINES` 行（任何活文档 `.md`）；
  ③ **入口文件** ≤ `MAX_ENTRY_LINES` 行 —— 接手必读的那几份必须短 ✓；
  ④ **设计文档预算**：`docs/design/**` 的**新增** `.md` ≤ `MAX_NEW_DESIGN_LINES` 行；
     **既有**（**非入口** ✓）文件按 `scripts/docs-budget.json` **冻结**
     （**只许减不许增** ✗，要放宽必须手改那份 JSON ⇒ 评审可见 ✓）；
     **入口文件不进冻结** ✓ —— 它们由 ③ 管：计划会随新批次**合法长大** ✓，
     冻结它必然**误红** ✗（2026-09-26 实测过一次真阻塞 ⇒ 用户要求"修判据不是改冻结值" ✓）；
  ⑤ **禁垃圾**：`docs/**` 下不得有 `.tmpdir` / `.tmp` / `.tmp-<pid>` / `.DS_Store`
     / `*.orig` / `*.rej` / `*~` —— **含未跟踪的本地残留** ✗（判据要咬得住
     2026-09-26 实测的那两类：`.e2-plan.md.*.tmpdir/` 与 `*.json.tmp-<pid>` ✓）；
  ⑥ **归档可追溯**（红线：**归档≠销毁** ✓）：`docs/archive/**` 每个文件都必须
     在 `docs/archive/README.md` 里被**点名** ✓，且归档总量 ≤ `MAX_ARCHIVE_BYTES`
     （防"归档变成垃圾场" ✗）。

`--freeze` 重算冻结表（**只会收紧** ✓，放宽要手改）；`--json` 出机器可读摘要。

退出码：0 = 通过 ✓ / 1 = 判红 ✗ / 2 = 用法或环境错。
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

# ── 预算常量（改这里 = 改判据 ⇒ 必须写理由 ✓）────────────────────────────────
MAX_LIVE_BYTES = 3_000_000  # ① 活文档总量（本轮实测 2.96 MB；清理前 8.84 MB ⇒ 33.5%）
MAX_FILE_LINES = 2000  # ② 任何单个活文档
MAX_ENTRY_LINES = 800  # ③ 接手必读的入口文件
MAX_NEW_DESIGN_LINES = 150  # ④ docs/design/** 新增文件
MAX_NEW_LINES_DEFAULT = 400  # ④ 其它目录新增文件
MIN_FROZEN = 20  # ④ 冻结下限（空/极短文件也允许 20 行，免得一行都不能加）
MAX_ARCHIVE_BYTES = 2_500_000  # ⑥ 归档总量（本轮实测 2.12 MB）

BUDGET = Path("scripts/docs-budget.json")
ARCHIVE = Path("docs/archive")
ARCHIVE_INDEX = ARCHIVE / "README.md"

# ③ 接手必读的入口文件（AGENTS.md 的阅读顺序 + 计划入口）
ENTRY_FILES = {
    "AGENTS.md",
    "README.md",
    "REQUIREMENTS.md",
    "ROADMAP.md",
    "STATUS.md",
    "docs/README.md",
    "docs/HANDOVER.md",
    "docs/E2-HANDOVER.md",
    "docs/design/e2-plan.md",
}

# ⑤ 垃圾模式（路径 → 是否目录）
JUNK_DIR_SUFFIX = ".tmpdir"
JUNK_FILE_RE = re.compile(r"(\.tmp$|\.tmp-\d+$|\.orig$|\.rej$|~$|~\d+$|^\.DS_Store$)")


def tracked() -> set[str]:
    """**会被提交的**文件（跟踪的 + 未跟踪但没被 ignore 的 ✓）。

    为什么带上未跟踪的 ✗：本地新写的文档在 `git add` 之前也该算进预算 ✓ ——
    否则"先写一大篇、再提交"就能绕过判据 ✗（本仓的纪律：**咬不住的守卫等于没有** ✓）。
    被 ignore 的本地 scratch（`.cache/`、`.sokonanoda/` …）不算 ✓。
    """
    out = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return {p for p in out.split("\0") if p}


def line_count(path: Path) -> int:
    return len(path.read_text(encoding="utf-8", errors="replace").splitlines())


def live_docs(files: set[str]) -> list[str]:
    """① 的"活文档"：根 *.md + docs/**（不含 docs/archive/**）。"""
    out = []
    for f in files:
        if f.startswith("docs/archive/"):
            continue
        if f.startswith("docs/") or ("/" not in f and f.endswith(".md")):
            out.append(f)
    return sorted(out)


def load_budget() -> dict[str, int]:
    if not BUDGET.exists():
        return {}
    return json.loads(BUDGET.read_text(encoding="utf-8")).get("frozen", {})


def scan_junk() -> list[str]:
    """⑤ 扫 docs/ 下的垃圾（**含未跟踪** ✗）。"""
    bad = []
    for dirpath, dirnames, filenames in os.walk("docs"):
        for d in list(dirnames):
            if d.endswith(JUNK_DIR_SUFFIX):
                bad.append(f"{dirpath}/{d}/（临时目录）")
                dirnames.remove(d)
        for f in filenames:
            if JUNK_FILE_RE.search(f):
                bad.append(f"{dirpath}/{f}")
    return sorted(bad)


def check() -> tuple[list[str], dict]:
    files = tracked()
    live = live_docs(files)
    bad: list[str] = []

    # ① 活文档总量
    live_bytes = sum(Path(f).stat().st_size for f in live if Path(f).exists())
    if live_bytes > MAX_LIVE_BYTES:
        bad.append(
            f"① 活文档总量 {live_bytes / 1e6:.2f} MB > 上限 {MAX_LIVE_BYTES / 1e6:.2f} MB"
        )

    # ②③④ 行数
    budget = load_budget()
    for f in live:
        p = Path(f)
        if p.suffix != ".md" or not p.exists():
            continue
        n = line_count(p)
        if n > MAX_FILE_LINES:
            bad.append(f"② {f}：{n} 行 > 单文件上限 {MAX_FILE_LINES}")
        if f in ENTRY_FILES and n > MAX_ENTRY_LINES:
            bad.append(f"③ {f}：{n} 行 > 入口文件上限 {MAX_ENTRY_LINES}")
        cap = budget.get(f)
        if f in ENTRY_FILES:
            # **入口文件不进冻结**（2026-09-26 用户要求 ✓，修**判据**而不是改冻结值 ✓）：
            # 入口文件有自己的专属上限（判据 ③ ≤800 行 ✓ —— 它抓的是"变成巨石" ✗），
            # 而"冻结在当前行数"在"**计划随新批次长大**"时**必然误红** ✗ ——
            # 实测：另一会话往 `e2-plan.md` 加「批次 N」+138 行（**合法推进** ✓）
            # ⇒ 撞死"冻结在 379" ✗ ⇒ **真阻塞**：它会让**所有人**（含后续 notation 的推送）
            # 都推不动 ✗。⇒ 正确形态：**③ 管入口文件，④ 只管非入口的既有文档** ✓。
            pass
        elif cap is None:
            cap = MAX_NEW_DESIGN_LINES if f.startswith("docs/design/") else MAX_NEW_LINES_DEFAULT
            if n > cap:
                bad.append(f"④ {f}：{n} 行 > 新增文件预算 {cap}（没在 {BUDGET} 里）")
        elif n > cap:
            bad.append(
                f"④ {f}：{n} 行 > 冻结预算 {cap}（**只许减不许增** ✓；"
                f"确需长大 ⇒ `--freeze` 重新基线，或手改 {BUDGET} ⇒ 评审可见 ✓）"
            )

    # ⑤ 垃圾
    bad += [f"⑤ 垃圾残留：{j}" for j in scan_junk()]

    # ⑥ 归档可追溯
    arch_files = []
    if ARCHIVE.exists():
        arch_files = sorted(
            str(p) for p in ARCHIVE.rglob("*") if p.is_file() and p != ARCHIVE_INDEX
        )
        arch_bytes = sum(Path(f).stat().st_size for f in arch_files)
        if arch_bytes > MAX_ARCHIVE_BYTES:
            bad.append(
                f"⑥ 归档总量 {arch_bytes / 1e6:.2f} MB > 上限 {MAX_ARCHIVE_BYTES / 1e6:.2f} MB"
            )
        if not ARCHIVE_INDEX.exists():
            bad.append(f"⑥ 缺归档索引 {ARCHIVE_INDEX}（归档必须有可追溯路径）")
        else:
            index = ARCHIVE_INDEX.read_text(encoding="utf-8")
            for f in arch_files:
                if Path(f).name not in index:
                    bad.append(f"⑥ {f} 没在 {ARCHIVE_INDEX} 里点名（归档≠销毁）")
    else:
        arch_bytes = 0

    stats = {
        "live_bytes": live_bytes,
        "live_files": sum(1 for f in live if Path(f).exists()),
        "archive_bytes": arch_bytes,
        "archive_files": len(arch_files),
        "max_live_bytes": MAX_LIVE_BYTES,
    }
    return bad, stats


def selftest() -> int:
    """**反向验证**（`AGENTS.md`：**咬不住的守卫等于没有** ✓）。

    把判据**逐条故意弄红一次** ⇒ 每条都必须被报出来 ✗；再加**三条方向性**用例
    （用户 2026-09-26 要求 ✓）：**回胖必须判红** ✓、**正常推进必须不误红** ✓、
    **非入口的冻结仍要咬** ✓。改完**一律还原** ✓（`finally` 里还原，跑完再核对
    工作区回到原样 ✓）。判据：`python3 scripts/docs-lint.py --selftest` ⇒ exit 0 + `9/9`。

    为什么要有它 ✗：本仓**三次**出现过"判据永远绿"的摆设 ✓ —— 一条判据如果
    没人证明它咬得住，就不能算守卫 ✓；而**误红**同样是坏判据 ✗（它会把所有人的
    推送一起拦下 ⇒ 真阻塞 ✓）。
    """
    results: list[tuple[str, str, bool]] = []

    def append_lines(path: str, n: int, line: str = "x\n"):
        """往活文档追加 n 行，返回还原函数（`finally` 里必须调 ✓）。"""
        p = Path(path)
        old = p.read_text(encoding="utf-8")
        p.write_text(old + line * n, encoding="utf-8")
        return lambda: p.write_text(old, encoding="utf-8")

    # ① 活文档总量 + ② 单文件行数 + ④ 冻结预算：往一份活文档灌 ≈800 KB / 2200 行
    p = Path("docs/LESSONS.md")
    if p.exists():
        old = p.read_text(encoding="utf-8")
        p.write_text(old + ("填" * 200 + "\n") * 2200, encoding="utf-8")  # ≈800 KB / 2200 行
        try:
            bad, _ = check()
        finally:
            p.write_text(old, encoding="utf-8")
        results.append(("①", "活文档总量 > 3.0 MB", any(b.startswith("①") for b in bad)))
        results.append(("②", "单文件 > 2000 行", any(b.startswith("②") for b in bad)))
        results.append(("④", "冻结预算被撑大", any(b.startswith("④") for b in bad)))

    # ③ 入口文件 > 800 行
    if Path("REQUIREMENTS.md").exists():
        restore = append_lines("REQUIREMENTS.md", 700)
        try:
            bad, _ = check()
        finally:
            restore()
        results.append(("③", "入口文件 > 800 行", any(b.startswith("③") for b in bad)))

    # ③/④ **方向性**（用户 2026-09-26 要求 ✓）：入口文件（`e2-plan` 这类**计划**）
    # ① 正常推进（+138 行 = 「批次 N」的量级）⇒ **不误红** ✓
    # ② 回胖（+300 行）⇒ 由 ③ 判红 ✓（**不是**由冻结判红 —— 冻结不管入口文件 ✓）
    # ③ 非入口的冻结文档回胖（+300）⇒ 由 ④ 判红 ✓（棘轮仍然活着 ✓）
    ep = Path("docs/design/e2-plan.md")
    if ep.exists():
        restore = append_lines("docs/design/e2-plan.md", 138)
        try:
            bad, _ = check()
        finally:
            restore()
        results.append(
            ("③", "入口计划 +138 行（正常推进）⇒ **不误红**", not any(
                b.startswith(("③", "④")) and "e2-plan" in b for b in bad))
        )
        restore = append_lines("docs/design/e2-plan.md", 300)
        try:
            bad, _ = check()
        finally:
            restore()
        results.append(
            ("③", "入口计划 +300 行（回胖）⇒ 判红", any(
                b.startswith("③") and "e2-plan" in b for b in bad))
        )
    nsub = Path("docs/design/notation-subset.md")
    if nsub.exists():
        restore = append_lines("docs/design/notation-subset.md", 300)
        try:
            bad, _ = check()
        finally:
            restore()
        results.append(
            ("④", "非入口冻结文档 +300 行 ⇒ 判红（棘轮活着）", any(
                b.startswith("④") and "notation-subset" in b for b in bad))
        )

    # ⑤ 垃圾残留
    junk = Path("docs/__docs-lint-selftest.tmp")
    junk.write_text("junk", encoding="utf-8")
    try:
        bad, _ = check()
    finally:
        junk.unlink()
    results.append(("⑤", "docs/** 垃圾残留", any(b.startswith("⑤") for b in bad)))

    # ⑥ 归档文件没被索引点名
    stray = Path("docs/archive/__selftest-unindexed.md.gz")
    stray.write_bytes(b"\x1f\x8b")
    try:
        bad, _ = check()
    finally:
        stray.unlink()
    results.append(("⑥", "归档文件未被点名", any(b.startswith("⑥") for b in bad)))

    ok = all(hit for _, _, hit in results)
    for code, label, hit in results:
        print(f"  {'✓' if hit else '✗'} {code} {label}")
    print(
        f"docs-lint --selftest：{sum(1 for _, _, h in results if h)}/{len(results)} "
        f"条判据咬得住 {'✓' if ok else '✗（有判据咬不住！）'}"
    )
    return 0 if ok else 1


def freeze() -> int:
    """把当前行数写进冻结表 —— **只收紧** ✓（已存在的更小 cap 不会被抬高）。

    **入口文件不写进来** ✓（2026-09-26 用户要求 ✓）：它们由判据 ③（≤800 行）管 ——
    计划会随新批次合法长大 ✓，冻结它必然误红 ✗（详见 `check()` 里那段注释）。
    """
    files = tracked()
    old = load_budget()
    new: dict[str, int] = {}
    tightened = 0
    for f in live_docs(files):
        p = Path(f)
        if p.suffix != ".md" or not p.exists() or f in ENTRY_FILES:
            continue
        n = max(line_count(p), MIN_FROZEN)
        cap = min(old[f], n) if f in old else n
        if f in old and cap < old[f]:
            tightened += 1
        new[f] = cap
    BUDGET.write_text(
        json.dumps(
            {
                "_comment": (
                    "文档预算冻结表（REQUIREMENTS.md §9 / docs/design/docs-diet.md）："
                    "每个既有**非入口**活文档的行数上限 —— **只许减不许增**。"
                    "**入口文件不在此表**（它们由判据 ③ ≤800 行管 ✓ —— 计划会随新批次"
                    "合法长大，冻结它必然误红 ✗）。"
                    "`python3 scripts/docs-lint.py --freeze` 只会收紧；"
                    "要放宽必须手改本文件（评审可见）。"
                ),
                "frozen": dict(sorted(new.items())),
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    added = len(new) - len([f for f in new if f in old])
    print(
        f"docs-budget.json 已更新：{len(new)} 条（新增 {added} · 收紧 {tightened}）"
    )
    return 0


def main(argv: list[str]) -> int:
    if "--help" in argv or "-h" in argv:
        print(__doc__)
        return 0
    if "--freeze" in argv:
        return freeze()
    if "--selftest" in argv:
        return selftest()
    try:
        bad, stats = check()
    except subprocess.CalledProcessError as e:  # git 不可用
        print(f"docs-lint: 环境错：{e}", file=sys.stderr)
        return 2
    if "--json" in argv:
        print(json.dumps({**stats, "ok": not bad, "problems": bad}, ensure_ascii=False, indent=2))
        return 1 if bad else 0
    if bad:
        print(f"docs-lint：{len(bad)} 条不通过 ✗", file=sys.stderr)
        for b in bad:
            print(f"  - {b}", file=sys.stderr)
        return 1
    print(
        f"docs-lint ✓ 活文档 {stats['live_files']} 个 / {stats['live_bytes'] / 1e6:.2f} MB "
        f"≤ {MAX_LIVE_BYTES / 1e6:.2f} MB ✓ · 归档 {stats['archive_files']} 个 / "
        f"{stats['archive_bytes'] / 1e6:.2f} MB ✓ · 垃圾 0 ✓ · 归档索引齐 ✓"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
