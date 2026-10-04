#!/usr/bin/env python3
"""**卡住 ⇒ 先读 Lean 4** 的守卫（用户 2026-10-05 拍板 ✓；规则见 `AGENTS.md`）。

## 为什么要有这条守卫 ✗

规则写进 `AGENTS.md` 只算"声明" ✓ —— 而本项目最常见的病就是
**「声明与守卫之间有条缝」**（`AGENTS.md` 自己写着「咬不住的守卫等于没有」✓）。
⇒ 这条守卫让规则**会拦红** ✓：**动了判定路径**的提交，必须带上「Lean 4 对照」的痕迹 ✓。

## 判据（一条 ✓，可执行 ✓）

暂存区里若出现**判定路径**的文件（见 `JUDGMENT_PATHS` ✓）⇒ 下列**任一**处必须含
`Lean 4` 或 `Lean4: n/a` ✓：

1. **提交信息**（`.git/COMMIT_EDITMSG` ✓ —— `git commit -m/-F` 会在 pre-commit **之前**写好它 ✓）；
2. **本轮 `docs/notes/HANDOFF-kernel.md` 的暂存增量** ✓（规则允许的另一种落点 ✓）。

**都不含 ⇒ exit 1**（拒绝提交 ✓）。⚠ **判不了 ≠ 绿** ✗：拿不到提交信息、`git` 不可用等情况
**大声说出来再放行** ✓（同 `docs-expiry-check.py` 的口径 ✓）。

## 逃生门

`git commit --no-verify` / `SOKO_SKIP_HOOK=1` ✓ —— 但**必须在 `STATUS.md` 写明原因** ✗，不许当默认动作 ✗。

## 自检（判据通道自己也要有判据 ✓）

`python3 scripts/check-lean4-alignment.py --selftest` ⇒ 造 4 组假输入，
**故意缺对照的必须判红** ✗、带对照的必须放行 ✓（**咬得住的守卫** ✓）。
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# **判定路径** ✓ —— 动了这些地方就得有 Lean 4 对照（用户 2026-10-05 ✓）。
JUDGMENT_PATHS = (
    "crates/front/src/compile/elab.rs",
    "crates/front/src/compile/implicit.rs",
    "crates/front/src/compile/meta.rs",
)
JUDGMENT_PREFIXES = ("crates/kernel/",)

# 认可的痕迹（**两种拼法都收** ✓ —— 免得因为空格被判红 ✗）。
MARKERS = ("Lean 4", "Lean4: n/a", "Lean4:n/a")

HANDOFF = "docs/notes/HANDOFF-kernel.md"


def is_judgment_path(path: str) -> bool:
    return path in JUDGMENT_PATHS or path.startswith(JUDGMENT_PREFIXES)


def touches_judgment_path(staged: list[str]) -> list[str]:
    return [p for p in staged if is_judgment_path(p)]


def has_marker(text: str) -> bool:
    return any(m in text for m in MARKERS)


def staged_files() -> list[str] | None:
    """暂存区文件名；`None` = **判不了**（不是"空" ✗）。"""
    try:
        out = subprocess.run(
            ["git", "diff", "--cached", "--name-only"],
            cwd=ROOT, capture_output=True, text=True, timeout=20,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if out.returncode != 0:
        return None
    return [l for l in out.stdout.splitlines() if l.strip()]


def commit_message() -> str | None:
    """提交信息；`None` = 拿不到（判不了 ✗）。"""
    try:
        git_dir = subprocess.run(
            ["git", "rev-parse", "--git-dir"],
            cwd=ROOT, capture_output=True, text=True, timeout=20,
        )
        if git_dir.returncode != 0:
            return None
        p = Path(git_dir.stdout.strip())
        if not p.is_absolute():
            p = ROOT / p
        f = p / "COMMIT_EDITMSG"
        return f.read_text(encoding="utf-8") if f.is_file() else None
    except (OSError, subprocess.SubprocessError):
        return None


def handoff_delta() -> str:
    """本轮 `HANDOFF-kernel.md` 的**暂存增量** ✓（规则允许的第二种落点 ✓）。"""
    try:
        out = subprocess.run(
            ["git", "diff", "--cached", "--", HANDOFF],
            cwd=ROOT, capture_output=True, text=True, timeout=20,
        )
        return out.stdout if out.returncode == 0 else ""
    except (OSError, subprocess.SubprocessError):
        return ""


def judge(staged: list[str], message: str, handoff: str) -> tuple[int, str]:
    """**判据本体**（自检直接调它 ✓ —— 与 `gap.py` 的 `judge()` 同款 ✓）。"""
    hit = touches_judgment_path(staged)
    if not hit:
        return 0, "本次未触及判定路径 ⇒ 无需 Lean 4 对照 ✓"
    if has_marker(message) or has_marker(handoff):
        where = "提交信息" if has_marker(message) else f"{HANDOFF} 增量"
        return 0, f"触及判定路径 {len(hit)} 个 ⇒ 已在**{where}**找到 Lean 4 对照 ✓"
    return 1, (
        f"触及判定路径 {len(hit)} 个（{', '.join(hit[:3])}{'…' if len(hit) > 3 else ''}）"
        f" ⇒ 但**提交信息**与 **{HANDOFF} 增量**里都**没有** `Lean 4` / `Lean4: n/a` ✗\n"
        "  ⇒ 用户 2026-10-05 规则：**卡住 ⇒ 先读 Lean 4**，第一次动手前先交「Lean 4 对照」\n"
        "     （3–6 行：哪节文件 / Lean 怎么做 / 我们怎么做 / 差异与决定 ✓）\n"
        f"     · 落点二选一：**提交信息** ✓ 或 **{HANDOFF}** ✓\n"
        "     · 纯性能/构建/缓存类 ⇒ 写 `Lean4: n/a` ✓"
    )


def selftest() -> int:
    """**反向验证**（硬要求 ✓）：缺对照的必须判红 ✗、带对照的必须放行 ✓。"""
    cases = [
        # (名字, staged, message, handoff, 期望 exit)
        ("只改文档 ⇒ 放行", ["docs/x.md"], "", "", 0),
        ("改判定路径 + 无对照 ⇒ **判红**", ["crates/front/src/compile/elab.rs"], "fix: 顺手改", "", 1),
        ("改判定路径 + 提交信息有对照 ⇒ 放行", ["crates/front/src/compile/elab.rs"],
         "fix: 按 Lean 4 对齐（App.lean:1176）", "", 0),
        ("改判定路径 + 交接单增量有对照 ⇒ 放行", ["crates/kernel/src/expr.rs"],
         "fix: 顺手改", "+Lean4: n/a（纯缓存键调整）", 0),
    ]
    bad = 0
    for name, staged, msg, hand, want in cases:
        got, why = judge(staged, msg, hand)
        ok = got == want
        bad += 0 if ok else 1
        print(f"  {'✓' if ok else '✗'} {name} ⇒ exit {got}（期望 {want}）{'' if ok else ' :: ' + why[:80]}")
    if bad:
        print(f"✗ 自检判红：{bad} 组与期望不符 ✗")
        return 1
    print("✓ 自检通过：4 组（含**故意缺对照必须判红** ✓）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="卡住⇒先读 Lean 4 的守卫（用户 2026-10-05）")
    ap.add_argument("--selftest", action="store_true", help="判据通道自检（反向验证 ✓）")
    ap.add_argument("--check", action="store_true", help="检查暂存区（pre-commit 用 ✓）")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    staged = staged_files()
    if staged is None:
        print("[lean4-guard] ⚠ 拿不到暂存区 ⇒ **判不了**（**判不了不是绿** ✗）⇒ 放行", file=sys.stderr)
        return 0
    msg = commit_message()
    if msg is None:
        print("[lean4-guard] ⚠ 拿不到提交信息 ⇒ **判不了**（同上 ✗）⇒ 放行", file=sys.stderr)
        return 0
    code, why = judge(staged, msg, handoff_delta())
    if code == 0:
        print(f"[lean4-guard] ✓ {why}")
        return 0
    print(f"[lean4-guard] ❌ {why}", file=sys.stderr)
    print("  逃生门：`git commit --no-verify` / `SOKO_SKIP_HOOK=1` ✓"
          "（**必须在 STATUS.md 写明原因** ✗）", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
