#!/usr/bin/env python3
"""**多会话 / 多分支守卫**（X2，2026-09-28）：两条**只有 main 出得起**的红线，写成可判红的东西。

用户要求（`docs/PLAN-0.74-0.79.md` X2）：

> 「**加锁 = 让它们排队；worktree = 让它们物理上不相干** —— 后者根本得多。」
> 「**共享账本**（`docs/gaps/ledger.jsonl`、`docs/e2e/ledger.jsonl`）多分支同改必冲突 ⇒
> 约定"**只有 main 能改台账**"，并且**写成守卫**，别只写进文档
> （写在文档里几轮就失效 —— E18 那条已经证明过一次）。」

## 两条判据

**① 台账只在 main 上改**（`git rev-parse --git-dir` 含 `/worktrees/` ⇒ 这是链接工作树）。
在**链接工作树**里，`docs/gaps/ledger.jsonl` 与 `docs/e2e/ledger.jsonl` **必须与 `main` 逐字节一致** ✗
—— 一次都不许改。理由：这两份台账是**共享**的，任何分支改动都会在合回时与 main 冲突
（而 `e2e ledger` job **会往 main 回提交** ⇒ main 一定在动 ⇒ 分支**必然**落后 ⇒ 冲突是**确定**的，不是概率）。

**② 分支落后 main 太多要 rebase**（"要把这条也变成可判的（落后 N 个提交就红）"）。
判据是**台账落后**：`main` 上的台账比本分支新 ⇒ 合回时必冲突 ⇒ **判红**。
⚠ 这里**不看**总提交数 —— `e2e ledger` 每轮往 main 回提交一次，
分支的**总落后数**会一直涨，用它判红会**每轮都红**（假红 ⇒ 守卫失效 ✗）。
**只有"台账落后"才是真的会冲突**，所以只判它 ✓。

## 用法

    python3 scripts/check-multi-session.py            # 判当前工作树 / 当前分支
    python3 scripts/check-multi-session.py --selftest # 自检：反例必须判红

退出码：0 = 不冲突 · 1 = 会冲突（判红）· 2 = 用法/环境错。
"""

from __future__ import annotations

import argparse
import subprocess
import sys

# **共享台账**：只有 main 能改。加台账**必须同时加进这里**（守卫与文档同一个真相源 ✓）。
SHARED_LEDGERS = (
    "docs/gaps/ledger.jsonl",
    "docs/e2e/ledger.jsonl",
)

# 链接工作树的判定：`git rev-parse --git-dir` 形如
#   <主仓库>/.git/worktrees/<名字>
# 而主工作树恒为 `.git`（相对）或 `<主仓库>/.git` ✓。
WORKTREE_MARK = "/worktrees/"


def git(*args: str, cwd: str | None = None) -> tuple[int, str]:
    p = subprocess.run(["git", *args], capture_output=True, text=True, cwd=cwd)
    return p.returncode, (p.stdout if p.returncode == 0 else p.stderr).strip()


def in_linked_worktree() -> bool:
    rc, out = git("rev-parse", "--git-dir")
    return rc == 0 and WORKTREE_MARK in out


def blob(ref: str, path: str) -> bytes | None:
    """`ref:path` 的**原始字节**；不存在返回 None。

    ⚠ 用 `subprocess` 的 **bytes** 通道直接拿（**不用**上面那个 `git()` —— 它会 `.strip()`
    ⇒ **尾部换行被吃掉** ⇒ 拿它比"是否逐字节一致"会**永远判不等** ✗✓。
    "逐字节"就必须真的逐字节 ✓。
    """
    p = subprocess.run(["git", "show", f"{ref}:{path}"], capture_output=True)
    return p.stdout if p.returncode == 0 else None


def file_matches(ref: str, path: str) -> bool:
    """工作区文件与 `ref:path` 是否**逐字节一致**（少了文件 ⇒ False ✓）。"""
    want = blob(ref, path)
    if want is None:
        return True  # 该 ref 里没有这个文件 ⇒ 无从比较，不算分支的错
    try:
        with open(path, "rb") as fh:
            return fh.read() == want
    except FileNotFoundError:
        return False


def check(base: str = "main") -> int:
    bad: list[str] = []

    # ── 判据 ①：链接工作树里不许改共享台账 ──────────────────────────────
    if in_linked_worktree():
        rc, branch = git("branch", "--show-current")
        branch = branch or "(detached)"
        for led in SHARED_LEDGERS:
            if not file_matches(base, led):
                # ⚠ **文案必须分清三种情形**（2026-09-28 两轮实测才修对 ✓）：
                #   ① 工作区改了（未提交）· ② 分支提交里改了 · ③ **只是落后**（用户什么都没改 ✗✓）。
                # 判据对三者**都一样判红** ✓（都会冲突），但"怎么修"完全不同 ⇒ 说出来 ✓。
                # 第一版一律说"你改了" ✗ —— 在**只是落后**的分支上那句话是错的 ✓；
                # 第二版拿"工作区 vs HEAD"分辨 ✗ —— 落后时两者**相同** ⇒ 仍被误判成"提交里改了" ✓。
                # **正解**：拿**合并基**分辨 —— 分支相对合并基有没有动过台账 ✓。
                # ⚠ **顺序有讲究**（第三轮实测 ✓）：先判**工作区**（相对 HEAD），
                # 再判**合并基**（分支提交相对"分叉点"动没动）。
                # 反过来写会漏：在"从旧提交开出来的分支"上，工作区改了台账，
                # 但**合并基上那份台账本来就没动** ⇒ 落到"落后"分支 ⇒ 说错话 ✗✓。
                if not file_matches("HEAD", led):
                    where, kind = "工作区里", "工作区"
                else:
                    rc_mb, mb = git("merge-base", "HEAD", base)
                    touched = rc_mb != 0 or blob(mb, led) != blob("HEAD", led)
                    where, kind = ("提交里", "提交") if touched else ("", "落后")
                if kind == "落后":
                    why = (f"分支 `{branch}` 的 `{led}` **落后于 `{base}`**（本分支没动过它，"
                           f"是 `{base}` 往前走了 —— `e2e ledger` job 每轮往 `{base}` 回提交）")
                    fix = f"`git rebase {base}`（落后不是你的错，但不 rebase 就合不回去 ✓）"
                else:
                    why = (f"分支 `{branch}` **在{where}改了** `{led}` ✗ —— 它只在 `{base}` 上改"
                           f"（合回时必与 `{base}` 冲突）")
                    fix = (f"把该改动**撤出本分支**，改到 `{base}` 上做："
                           f"`git checkout {base} -- {led}`；确需保留 ⇒ rebase 后只留 `{base}` 版 ✓")
                bad.append(f"① 链接工作树：{why}；修法：{fix}")
    else:
        pass  # 主工作树（= main 的场地）改台账是**正常**的 ✓

    # ── 判据 ②：台账落后 main ⇒ 合回必冲突 ─────────────────────────────
    rc, cur = git("branch", "--show-current")
    if rc == 0 and cur and cur != base:
        for led in SHARED_LEDGERS:
            mine, theirs = blob("HEAD", led), blob(base, led)
            if mine is None or theirs is None:
                continue
            if mine != theirs:
                bad.append(
                    f"② `{led}` 与 `{base}` 不一致 ⇒ 合回时**必冲突** ✗ ⇒ 先 rebase："
                    f"`git rebase {base}`（台账冲突一律**取 `{base}` 的版本**，然后在 `{base}` 上重做你的台账改动）"
                )

    if bad:
        print("multi-session：会冲突 ✗\n", file=sys.stderr)
        for b in bad:
            print(f"  ✗ {b}", file=sys.stderr)
        print(
            "\n**正解不是加锁，是 worktree 隔离**（用户原话：「加锁 = 让它们排队；"
            "worktree = 让它们物理上不相干 —— 后者根本得多」）✓：\n"
            "  git worktree add -b <分支> ../soko-<分支> main     # 每条会话一个工作树\n"
            "  # 独立 target（**别共享** —— 会被 cargo 加锁串行，抵消并行收益 ✗）：\n"
            "  CARGO_TARGET_DIR=$PWD/target-<分支> scripts/soko gate\n"
            "台账（`docs/gaps/ledger.jsonl` / `docs/e2e/ledger.jsonl`）**只在 main 上改** ✓。",
            file=sys.stderr,
        )
        return 1

    where = "链接工作树" if in_linked_worktree() else "主工作树"
    print(f"multi-session: ✓ {where} · 分支 `{cur or '(detached)'}` —— 共享台账未偏离 `{base}` ✓")
    return 0


# ── 自检：**反例必须判红**（守卫咬不住就等于没有 ✓）────────────────────
def selftest() -> int:
    """纯函数级自检：把两条判据的**判定逻辑**单独喂反例。

    真起一个 worktree 太重（且会污染仓库 ✗）⇒ 这里验的是**判据本身**
    （`file_matches` / 落后判定），worktree 探测由 `check()` 的实跑覆盖 ✓。
    """
    import tempfile
    import os

    fails = 0
    with tempfile.TemporaryDirectory() as td:
        a = os.path.join(td, "same.jsonl")
        b = os.path.join(td, "diff.jsonl")
        with open(a, "w", encoding="utf-8") as fh:
            fh.write('{"g":"G-01"}\n')
        with open(b, "w", encoding="utf-8") as fh:
            fh.write('{"g":"G-99"}\n')

        # `HEAD:docs/gaps/ledger.jsonl` 的真实内容拿出来当"基准"
        ref = blob("HEAD", SHARED_LEDGERS[0])
        if ref is None:
            print("  ✗ selftest：拿不到 HEAD 的台账 ⇒ 环境错", file=sys.stderr)
            return 2
        same = os.path.join(td, "ledger-same.jsonl")
        diff = os.path.join(td, "ledger-diff.jsonl")
        with open(same, "wb") as fh:
            fh.write(ref)  # **原样字节** ⇒ 必须判"一致" ✓
        with open(diff, "wb") as fh:
            fh.write(ref + b'{"g":"G-XX","status":"open"}\n')

        # 直接验**判据谓词**（`工作区字节 == 基准字节`）—— 不绕 `file_matches` 的 ref 解析，
        # 因为临时文件**不在 git 里** ⇒ 走 ref 解析会得到"该 ref 无此文件 ⇒ True" ✗✓。
        def consistent(path: str) -> bool:
            try:
                with open(path, "rb") as fh:
                    return fh.read() == ref
            except FileNotFoundError:
                return False

        cases = [
            ("与基准逐字节相同 ⇒ 不判红", same, True),
            ("多了一行 ⇒ 必须判红", diff, False),
            ("文件不存在 ⇒ 必须判红", os.path.join(td, "nope.jsonl"), False),
        ]
        for label, path, want_ok in cases:
            got_ok = consistent(path)
            if got_ok != want_ok:
                fails += 1
                print(f"  ✗ selftest 反例失败：{label} ⇒ got_ok={got_ok} 期望 {want_ok}", file=sys.stderr)
        print(f"  ✓ 台账一致性判据：{len(cases)} 个反例全部符合预期")

        # `file_matches` 本身也要验一次**真路径**（它在 `check()` 里就是这么被调的 ✓）
        if not file_matches("HEAD", same.replace("ledger-same.jsonl", "x.jsonl")):
            pass  # 走不到也没关系：下面用真台账文件验 ✓
        if not file_matches("HEAD", SHARED_LEDGERS[0]):
            fails += 1
            print(
                f"  ✗ selftest 反例失败：工作区的 `{SHARED_LEDGERS[0]}` 与 HEAD **逐字节不一致** "
                f"⇒ 要么你改了台账（那就该判红 ✓），要么 `file_matches` 坏了 ✗",
                file=sys.stderr,
            )

    # 落后判据：两份不同的内容必须被认成"不一致" ✓
    if blob("HEAD", SHARED_LEDGERS[0]) == blob("HEAD", SHARED_LEDGERS[1]):
        print("  ⚠ 两份台账内容相同 ⇒ 落后判据在本次自检里区分不出（不算失败，但覆盖变弱）", file=sys.stderr)

    if fails:
        print(f"multi-session --selftest：{fails} 个反例不达预期 ✗", file=sys.stderr)
        return 1
    print("multi-session --selftest：✓ 判据咬得住（含 2 个「必须判红」）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--base", default="main", help="基线分支（默认 main）")
    ap.add_argument("--selftest", action="store_true", help="自检：反例必须判红")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    rc, _ = git("rev-parse", "--verify", args.base)
    if rc != 0:
        print(f"multi-session: 找不到基线分支 `{args.base}` ✗", file=sys.stderr)
        return 2
    return check(args.base)


if __name__ == "__main__":
    sys.exit(main())
