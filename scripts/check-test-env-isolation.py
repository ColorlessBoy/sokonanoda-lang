#!/usr/bin/env python3
"""**测试的环境档位隔离守卫**（2026-09-30）。

## 为什么需要它（真实事故，本仓发生过）

`crates/front/tests/judge_inplace_by.rs` 曾把**两条**判据放进**一个文件**：
一条要 `SOKO_JUDGE_INPLACE_BY=shadow`、另一条要 `=1`。而
`SOKO_JUDGE_INPLACE*` 走 `OnceLock` **只读一次环境**，**同一个集成测试文件里的
多个 `#[test]` 共享一个进程**（`cargo test` 默认多线程）⇒ 两条**抢同一个
`OnceLock`**、**谁先跑到谁定档** ⇒ 影子档那条的 `same` 恒为 0 ⇒ **间歇判红**
（实测 6 连跑 **4 红 2 绿**）。症状是**"单独跑绿、gate 里红"** ——
最容易被误读成"gate 有问题"的那一类 ✗。

⚠ 讽刺的是那个文件的**头注释写的正是"集成测试各自独立进程 ⇒ 天然隔离"** ——
声明与事实不符，而**没人核对**。⇒ 这条守卫把"核对"自动化 ✓。

## 判据

**同一个集成测试文件里**：若它调用 `std::env::set_var`（= 配档位），
则 `#[test]` **必须恰好一个**（一个档位一个文件 = 独立进程 = 真隔离）。

⚠ **为什么不许"两个测试设同一个值"就放行**：判据不是"值一样不一样"，
而是"**一个进程只该验一个档位**" —— 两个测试哪怕今天写同一个值，
下一个人改其中一个就变成竞态，而**改的人不会知道** ✗。宁可现在就分开。

## 豁免

确有理由（例如测试自己在**文件内串行**、或用 `--test-threads=1` 约定）⇒
在文件里写一行 `// soko:env-isolation-ok: <理由 ≥8 字>` ✓（与
`check-timing-evidence.py` 的 `soko:no-timing:` 同款机制）。

`--selftest`：故意坏的夹具**必须判红**（反向验证 —— 咬不住的守卫等于没有）。
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# 集成测试文件（`crates/*/tests/*.rs`）—— 单测（`src/**` 里的 `#[cfg(test)]`）
# 也共享进程，但它们更常见地串行/共用 fixture；这里先管事故发生的形态，
# 扩到 `src/**` 是后续可以做的收紧（不在本轮偷偷放宽 ✗）。
GLOBS = ("crates/*/tests/*.rs",)

TEST_ATTR = re.compile(r"^#\[test\]", re.MULTILINE)
# `std::env::set_var` / `env::set_var`（含 `use std::env;` 后的简写）
SET_VAR = re.compile(r"\benv::set_var\s*\(")
EXEMPT = re.compile(r"^[ \t]*//[ \t]*soko:env-isolation-ok:[ \t]*(\S.{7,})", re.MULTILINE)


def offending(text: str) -> tuple[int, bool]:
    """返回 (`#[test]` 个数, 是否用了 set_var)。"""
    return len(TEST_ATTR.findall(text)), bool(SET_VAR.search(text))


def check_file(path: Path) -> str | None:
    text = path.read_text(encoding="utf-8")
    tests, sets = offending(text)
    if not sets or tests <= 1:
        return None
    if EXEMPT.search(text):
        return None
    return (
        f"{path.relative_to(REPO)}：**{tests} 个 `#[test]`** 却调用 `env::set_var` "
        f"⇒ 它们**共享一个进程**、抢同一个 `OnceLock`/静态档位 ⇒ **间歇判红** ✗\n"
        f"    修法：**一个档位一个文件**（各自独立进程 = 真隔离 ✓）；\n"
        f"    确有理由（文件内串行等）⇒ 加一行 `// soko:env-isolation-ok: <理由 ≥8 字>`。"
    )


def selftest() -> int:
    bad = '''
use std::env;
#[test]
fn a() { env::set_var("SOKO_JUDGE_INPLACE_BY", "shadow"); }
#[test]
fn b() { env::set_var("SOKO_JUDGE_INPLACE_BY", "1"); }
'''
    good_split = '''
use std::env;
#[test]
fn a() { env::set_var("SOKO_JUDGE_INPLACE_BY", "shadow"); }
'''
    good_exempt = '''
// soko:env-isolation-ok: 这个文件里的测试自己用互斥锁串行，档位相同
use std::env;
#[test]
fn a() { env::set_var("X", "1"); }
#[test]
fn b() { env::set_var("X", "1"); }
'''
    good_no_env = '''
#[test]
fn a() {}
#[test]
fn b() {}
'''
    cases = [
        ("两个 test + set_var ⇒ 必须判红", bad, True),
        ("一个 test + set_var ⇒ 放行", good_split, False),
        ("两个 test + set_var + 显式豁免 ⇒ 放行", good_exempt, False),
        ("两个 test 但不碰环境 ⇒ 放行", good_no_env, False),
    ]
    failed = 0
    for label, text, want_red in cases:
        tests, sets = offending(text)
        red = bool(sets and tests > 1 and not EXEMPT.search(text))
        ok = red == want_red
        print(f"  [{'ok' if ok else 'XX'}] {label}：{'判红' if red else '放行'}"
              f"（期望 {'判红' if want_red else '放行'}）")
        if not ok:
            failed += 1
    print(f"check-test-env-isolation --selftest：{'✓ 全部符合预期' if not failed else f'✗ {failed} 条不符'}")
    return 1 if failed else 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    problems: list[str] = []
    seen = 0
    for glob in GLOBS:
        for path in sorted(REPO.glob(glob)):
            seen += 1
            got = check_file(path)
            if got:
                problems.append(got)
    if problems:
        print(f"test-env-isolation：{len(problems)} 条不通过 ✗")
        for p in problems:
            print(f"  ✗ {p}")
        print(
            "依据：2026-09-30 实测 —— `judge_inplace_by.rs` 两条判据抢同一个 `OnceLock`，"
            "**6 连跑 4 红 2 绿**（症状：单独跑绿、gate 里红）。\n"
            "详见 `docs/CI-FAILURES.md` 的同日第三条。"
        )
        return 1
    print(f"test-env-isolation: ✓ {seen} 个集成测试文件 —— 配档位的都只有一个 `#[test]`")
    return 0


if __name__ == "__main__":
    sys.exit(main())
