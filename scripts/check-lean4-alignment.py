#!/usr/bin/env python3
# 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
"""**卡住 ⇒ 先读 Lean 4** 的守卫（用户 2026-10-05 拍板 ✓；规则见 `AGENTS.md`）。

## 为什么要有这条守卫 ✗

规则写进 `AGENTS.md` 只算"声明" ✓ —— 而本项目最常见的病就是
**「声明与守卫之间有条缝」**（`AGENTS.md` 自己写着「咬不住的守卫等于没有」✓）。
⇒ 这条守卫让规则**会拦红** ✓：**动了判定路径**的提交，必须带上「Lean 4 对照」的痕迹 ✓。

## 判据（一条 ✓，可执行 ✓）

暂存区里若出现**判定路径**的文件（见 `JUDGMENT_PATHS` ✓）⇒ 下列**任一**处必须含
`Lean 4` 或 `Lean4: n/a` ✓：

1. **提交信息**（`commit-msg` 钩子把信息文件作为 `$1` 传进来 ✓ —— **只有这一条路看得见本轮信息** ✓；
   ⚠ `pre-commit` 阶段 git **还没写** `.git/COMMIT_EDITMSG` ✗ ⇒ 那时读到的要么**不存在**、
   要么是**上一笔**的 ⇒ 本脚本在那一阶段**明说"不判定"**、绝不拿旧信息冒充本轮 ✗）；
2. **本轮 `docs/notes/HANDOFF-kernel.md` 的暂存增量** ✓（规则允许的另一种落点 ✓）。

**都不含 ⇒ exit 1**（拒绝提交 ✓）。⚠ **判不了 ≠ 绿** ✗：拿不到提交信息、`git` 不可用等情况
**大声说出来再放行** ✓（同 `docs-expiry-check.py` 的口径 ✓）。

## 两条"守卫没在判"的历史形状（2026-10-16 实测 ⇒ 已堵）

`LEAN4-ALIGNMENT-HOOK-CRASHES-ON-ENCODING`（**崩溃而不是判定** ✗）：旧写法是
`Path(args.message_file).read_text(encoding="utf-8")`，而 `git commit -F <文件>` 把**任意字节**
原样交给 git（git 不要求提交信息是 UTF-8）⇒ 非 UTF-8 信息 **抛 `UnicodeDecodeError`** ✗。
实测（真夹具仓 + 真 hook，`git commit -F <非 UTF-8 文件>`）：`byte 0xbc in position 79`
⇒ `Traceback` ⇒ rc=1 ⇒ hook 把它读成"拒绝提交" ⇒ **红得没有出路** ✗ —— 而且它连
**与判定路径无关**的提交一起拦（读信息在 `judge()` **之前**且**无条件**执行 ✗）。
⇒ 现在 `read_message_file()` 先按**字节**读、再 `decode("utf-8", errors="replace")`：
ASCII 部分逐字节保留 ⇒ 标记**照旧读得出来** ✓，替换字符 U+FFFD **造不出**新标记
⇒ **不可能把判红放宽成放行** ✓（"不许放宽判据"的另一半 ✓）。

`git commit -F` 那条**同族**（**判不了 ⇒ 放行 / 拿旧信息冒充本轮** ✗）：`pre-commit` 钩子
跑的是 `--check`（**不带** `--message-file`）⇒ 旧写法回落去读 `.git/COMMIT_EDITMSG`，
而那一阶段它要么**不存在**（⇒ 印「拿不到提交信息 ⇒ 判不了 ⇒ 放行」）、要么是**上一笔**的
（⇒ 拿它判 ⇒ 印出**假绿** `✓ 已在**提交信息**找到 Lean 4 对照` ✗ —— 实测：第二笔"真无标记"
的提交，pre-commit 那一行照样印 ✓，因为文件里还留着第一笔带标记的信息）。
⇒ 现在 `pre-commit` 阶段**明说"不判定"**（判定只由 `commit-msg` 做 ✓）——
判据本身**没有放宽**：`commit-msg` 从来就判得着（`$1` 一定在 ✓），改掉的是那个
**没有判据的读数** ✗→✓。

## 逃生门

`git commit --no-verify` / `SOKO_SKIP_HOOK=1` ✓ —— 但**必须在 `STATUS.md` 写明原因** ✗，不许当默认动作 ✗。

## 自检（判据通道自己也要有判据 ✓）

`python3 scripts/check-lean4-alignment.py --selftest` ⇒ 造 4 组假输入，
**故意缺对照的必须判红** ✗、带对照的必须放行 ✓（**咬得住的守卫** ✓）。
"""
from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
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


# ⚠ **删掉了 `commit_message()`**（2026-10-16）：它读 `.git/COMMIT_EDITMSG` 并
# `read_text(encoding="utf-8")` —— 两处都错 ✗：① 那个文件在 `pre-commit` 阶段是
# **上一笔**的信息（拿它判 ⇒ 假绿，见 `main()` 里那段）；② 它也会**抛** `UnicodeDecodeError`。
# 它**只有 `main()` 一个调用点**（`grep -rn commit_message` 全仓 = 只有定义处 ✓），
# 而那个调用点现在走"不判定"分支 ⇒ 留着就是留给下一个人的陷阱 ✗。


def read_message_file(path: str | Path) -> tuple[str | None, str]:
    """读提交信息文件 ⇒ `(文本, 说明)`。**永不抛** —— `UnicodeDecodeError` 也不行 ✗。

    ⚠ **为什么按字节读**：`git commit -F <文件>` 把**任意字节**原样交给 git（git 不要求
    提交信息是 UTF-8），而旧写法 `read_text(encoding="utf-8")` 遇到非 UTF-8 就**抛栈**
    （实测：`byte 0xbc in position 79` ⇒ `Traceback` + rc=1）⇒ **守卫崩溃而不是判定** ✗
    （见文件头 `LEAN4-ALIGNMENT-HOOK-CRASHES-ON-ENCODING` 那一段）。

    修法：`decode("utf-8", errors="replace")` —— ASCII 部分**逐字节保留** ⇒
    `Lean 4` / `Lean4: n/a` 这类标记**照旧读得出来** ✓；被替换的字节只变成 U+FFFD，
    而 U+FFFD **不是** ASCII ⇒ **造不出**新的标记 ⇒ 判据**只可能更准，不可能放宽** ✓。
    返回的 `说明` 非空 = "读到了，但信息不是合法 UTF-8"（**要印出来** ✓，否则这个形状
    又变成静默 ✗）；`文本 is None` = **真拿不到**（文件不在 / 读不动）。
    """
    p = Path(path)
    try:
        raw = p.read_bytes()
    except OSError as exc:
        return None, f"读不动 `{p}`（{type(exc).__name__}: {exc}）"
    try:
        return raw.decode("utf-8"), ""
    except UnicodeDecodeError as exc:
        return raw.decode("utf-8", errors="replace"), (
            f"提交信息**不是合法 UTF-8**（{exc}）⇒ 已按 `errors=\"replace\"` 读"
            f"（被替换的字节只会变成 U+FFFD，**不会**造出 `Lean 4` 标记 ⇒ 判据不放宽 ✓）")


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
    """**反向验证**（硬要求 ✓）：缺对照的必须判红 ✗、带对照的必须放行 ✓。

    两段：
    * **纯函数层** —— `judge()` 的四形（原样保留 ✓）；
    * **真 CLI 层**（2026-10-16 加）—— 在**真夹具仓**里跑**真脚本**（不是把逻辑抄一遍 ✗）：
      非 UTF-8 信息必须给出**判定**（不是栈 ✗）、`pre-commit` 阶段必须**明说"不判定"**
      （不许拿上一笔的信息印假绿 ✗）。⚠ 这一层是 `LEAN4-ALIGNMENT-HOOK-CRASHES-ON-ENCODING`
      与它那条同族的判据 —— 纯函数层**看不见**它们（两条都长在 `main()` 的读信息那一段 ✗）。
    """
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

    # ── 真 CLI 层（真夹具仓 + 真脚本 ✓）────────────────────────────────────────
    # ⚠ **为什么必须真起进程**：本缺口的两半都在 `main()` 里（读信息那一段），
    #   `judge()` 是纯函数 ⇒ 上面那 4 组**永远咬不住它们** ✗（这正是"守卫在、但没在判"）。
    with tempfile.TemporaryDirectory(prefix="soko-lean4-") as td:
        repo = Path(td)
        (repo / "scripts").mkdir()
        (repo / "crates" / "kernel").mkdir(parents=True)
        # 夹具仓里放**本文件**（逐字节 = 被测的那一份 ✓ ⇒ 测的是真脚本，不是复制品）
        me = Path(__file__).resolve()
        shutil.copyfile(me, repo / "scripts" / me.name)
        (repo / "crates" / "kernel" / "dummy.rs").write_text("fn main() {}\n", encoding="utf-8")
        for argv in (["init", "-q", "."], ["add", "-A"]):
            g = subprocess.run(["git", *argv], cwd=repo, capture_output=True, text=True)
            if g.returncode != 0:
                print(f"  ✗ 夹具仓建不起来（`git {' '.join(argv)}` rc={g.returncode}："
                      f"{g.stderr.strip()[:120]}）⇒ **不假装判过** ✗")
                return 1
        # 用户 case 的**逐字节形状**（报告原文：`byte 0xbc in position 79`）
        prefix = b"fix: align one spot in the ledger tool"
        non_utf8 = prefix + b" " * (79 - len(prefix)) + bytes([0xbc]) + b" trailing ascii\n"
        (repo / "msg-non-utf8.txt").write_bytes(non_utf8)
        (repo / "msg-non-utf8-marker.txt").write_bytes(non_utf8 + b"\nLean4: n/a\n")
        (repo / "msg-ok.txt").write_text("fix: 顺手改一处\n", encoding="utf-8")
        cli_cases = [
            # (名字, argv 尾部, 期望 rc, 必须**不**出现的字, 必须出现的字)
            ("**非 UTF-8 + 无对照 ⇒ 判定（判红，不是栈）**",
             ["--check", "--message-file", "msg-non-utf8.txt"], 1, "Traceback", "❌"),
            ("**非 UTF-8 + ASCII 段有 `Lean4: n/a` ⇒ 判定（放行）**",
             ["--check", "--message-file", "msg-non-utf8-marker.txt"], 0, "Traceback", "✓"),
            ("正对照：合法 UTF-8 + 无对照 ⇒ 判红",
             ["--check", "--message-file", "msg-ok.txt"], 1, "", "❌"),
            ("**`pre-commit` 形状（不带 `--message-file`）⇒ 明说「不判定」**",
             ["--check"], 0, "✓ 已在", "不判定"),
        ]
        for name, tail, want_rc, forbidden, required in cli_cases:
            proc = subprocess.run(
                [sys.executable, str(repo / "scripts" / me.name), *tail],
                cwd=repo, capture_output=True, text=True, encoding="utf-8",
                errors="replace", timeout=120,
            )
            blob = proc.stdout + proc.stderr
            ok = proc.returncode == want_rc
            note = "" if ok else f"rc={proc.returncode}（期望 {want_rc}）"
            if ok and forbidden and forbidden in blob:
                ok, note = False, f"出现了禁字 {forbidden!r}（= 崩溃 / 假绿 ✗）"
            if ok and required and required not in blob:
                ok, note = False, f"没有出现 {required!r}"
            bad += 0 if ok else 1
            print(f"  {'✓' if ok else '✗'} {name} ⇒ rc={proc.returncode}"
                  f"（期望 {want_rc}）{'' if ok else ' :: ' + note}")
    if bad:
        print(f"✗ 自检判红：{bad} 组与期望不符 ✗")
        return 1
    print(f"✓ 自检通过：{len(cases)} 组纯函数（含**故意缺对照必须判红** ✓）"
          f" + {len(cli_cases)} 组**真 CLI**（真夹具仓 + 真 hook 形状：非 UTF-8 必须判定 ✓、"
          f"pre-commit 必须明说「不判定」✓）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="卡住⇒先读 Lean 4 的守卫（用户 2026-10-05）")
    ap.add_argument("--selftest", action="store_true", help="判据通道自检（反向验证 ✓）")
    ap.add_argument("--check", action="store_true", help="检查暂存区（pre-commit 用 ✓）")
    ap.add_argument(
        "--message-file",
        help="提交信息文件（`commit-msg` 钩子传 $1 ✓ —— ⚠ `pre-commit` 阶段 git **还没写** "
             "`COMMIT_EDITMSG` ✗ ⇒ 那条分支在 pre-commit 里**永远不生效** ✗，实测过 ✓）",
    )
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    staged = staged_files()
    if staged is None:
        print("[lean4-guard] ⚠ 拿不到暂存区 ⇒ **判不了**（**判不了不是绿** ✗）⇒ 放行", file=sys.stderr)
        return 0
    if args.message_file:
        msg, note = read_message_file(args.message_file)
    else:
        # ⚠ **`pre-commit` 阶段：不判定**（2026-10-16 实测 ✗ —— 同族的"判不了/拿旧信息"那条）。
        # 那时 git **还没写**本轮 `COMMIT_EDITMSG` ⇒ 读到的要么**不存在**（旧写法印
        # 「拿不到提交信息 ⇒ 判不了 ⇒ 放行」）、要么是**上一笔**的（旧写法拿它判 ⇒ 印出
        # **假绿** `✓ 已在**提交信息**找到 Lean 4 对照` —— 实测：第二笔"真无标记"的提交，
        # pre-commit 那一行照样印 ✓）。两种都是**没有判据的读数** ⇒ 这里明说"不判定"。
        # 判据**没有放宽**：`commit-msg` 从来就判得着（`$1` 一定在 ✓），而且它才是判红的那个。
        print("[lean4-guard] pre-commit 阶段**看不到**本轮提交信息（git 还没写它）"
              "⇒ 这里只做暂存区预检、**不判定**（判定在 `commit-msg` 钩子 ✓）；"
              "要单跑判定：`--check --message-file <文件>`")
        return 0
    if msg is None:
        print(f"[lean4-guard] ⚠ 拿不到提交信息（{note}）⇒ **判不了**（**判不了不是绿** ✗）"
              f"⇒ 放行", file=sys.stderr)
        return 0
    if note:
        # ⚠ **非 UTF-8 必须印出来**（否则"读到了但读歪了"又变成静默 ✗）—— 但仍然**判定** ✓
        print(f"[lean4-guard] ⚠ {note}", file=sys.stderr)
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
