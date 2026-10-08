#!/usr/bin/env python3
"""G-95 自断言复现：**`⊆` 假设被 `apply` 之后目标显示成展开体 `A a`（丢记法）**。

用户原话（2026-10-08）：「sorry 这一行的目标 `A a` 有办法显示成 `a ∈ A` 吗」，
例子就是下面 `Canvas.sokonanoda` 里那 7 行。

**机制**（这条缺口最容易修错的地方）：显示层（`front::display` 的记法折叠）只能折
**记法目标名还在文本里**的项 —— `Set.mem a A` → `a ∈ A` ✓。而 `apply h a` 的新目标
**不是**源文本，它是内核 whnf `A ⊆ B`（= `Set.subset A B`）之后**定义体的实例**：

  · 体写展开形态 `∀ x, A x → B x` ⇒ 新目标是 `A a` ⇒ 折叠无从下手
    （`A`/`a` 都是局部变量，文本里没有 `Set.mem`）✗；
  · 体写点名 `∀ x, Set.mem x A → Set.mem x B` ⇒ 新目标是 `Set.mem a A` ⇒ 折成 `a ∈ A` ✓。

Lean/Mathlib 同款：`Set.Subset s₁ s₂ := ∀ ⦃a⦄, a ∈ s₁ → a ∈ s₂`（体里就是记法层级的项）。

**判据（两半，缺一不可）**：
  ① **真库**（`courses/set-theory/lib/Set.sokonanoda`，`--root` 指到课程目录）⇒
     目标**必须**含 `∈`（这是用户看得见的那一层 ✓）；
  ② **反向夹具**（体写成展开形态 `A x`）⇒ 目标**必须**是 `A a` —— 证明这条判据
     **咬得住**旧行为 ✓；若连它也变成 `∈`，说明折叠已能反演定义体 ⇒ 形状变了、
     **需要人看** ⇒ exit 2。

退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
  0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（需要人看）
"""

from __future__ import annotations

import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
SOKO = ROOT / "scripts" / "soko"
COURSE = ROOT / "courses" / "set-theory"
NODE = shutil.which("node")

# ① 真库：用户报的那个例子（逐字）。
CANVAS = "\n".join(
    [
        "import lib.Set",
        "",
        "theorem mem_of_subset (α : Type) (A B : Set α) (h : A ⊆ B) (a : α) :",
        "    a ∈ A → a ∈ B := by",
        "  intro hA",
        "  apply h a",
        "  sorry",
        "",
    ]
)

# ② 反向夹具：同一条路，但定义体写成**展开形态**（旧写法）⇒ 目标应当是 `A a`。
REVERSE = "\n".join(
    [
        "import lib.Set",
        "",
        "def mysubset {α : Type} (A B : Set α) : Prop := ∀ (x : α), A x → B x",
        "",
        "theorem probe_unfolded (α : Type) (A B : Set α) (h : mysubset A B) (a : α) :",
        "    a ∈ A → a ∈ B := by",
        "  intro hA",
        "  apply h a",
        "  sorry",
        "",
    ]
)


def query_goal(entry: pathlib.Path) -> str:
    """`sorry` 那一行的目标文本（`query state` = `soko/stateAt` 的同一条真相层）。"""
    line = next(
        i for i, text in enumerate(entry.read_text(encoding="utf-8").splitlines(), start=1)
        if text.strip() == "sorry"
    )
    env = dict(os.environ)
    for key in ("SOKONANODA_BIN", "SOKONANODA_LSP_BIN"):
        env.pop(key, None)
    proc = subprocess.run(
        [
            NODE, str(SOKO), "query", "state",
            "--file", str(entry),
            "--line", str(line), "--col", "3",
            # **绝对路径 + 显式模块根**（台账 G-12：相对路径 + 祖先清单会让模块根退化 ✓）。
            "--root", str(COURSE),
            "--compact",
        ],
        capture_output=True, text=True, env=env, cwd=str(ROOT),
    )
    if proc.returncode != 0:
        raise RuntimeError(f"query state 退出码 {proc.returncode}：{proc.stderr.strip()}")
    answer = json.loads(proc.stdout)
    if not answer.get("ok"):
        raise RuntimeError(f"query state 答 ok:false：{answer.get('error')}")
    return (answer["data"].get("goal") or "").strip()


def main() -> int:
    if NODE is None:
        print("   → 需要 node（scripts/soko 是 Node 启动器）", file=sys.stderr)
        return 2
    if not (COURSE / "lib" / "Set.sokonanoda").is_file():
        print(f"   → 找不到课程库 {COURSE / 'lib' / 'Set.sokonanoda'}", file=sys.stderr)
        return 2

    workdir = pathlib.Path(tempfile.mkdtemp(prefix="g95-"))
    try:
        canvas = workdir / "Canvas.sokonanoda"
        canvas.write_text(CANVAS, encoding="utf-8")
        reverse = workdir / "Reverse.sokonanoda"
        reverse.write_text(REVERSE, encoding="utf-8")

        real = query_goal(canvas)
        unfolded = query_goal(reverse)
        print("== 两半判据 ==")
        print(f"   [① 真库 lib.Set]       目标 = {real!r}   含 `∈` = {'∈' in real}")
        print(f"   [② 反向（体写 A x）]   目标 = {unfolded!r}   含 `∈` = {'∈' in unfolded}")

        if "∈" in unfolded:
            print(
                "   → 反向夹具也折成了记法 ⇒ 显示层已经能反演定义体，本条的机制描述作废："
                "需要人看（更新台账与设计，而不是改夹具）✗",
                file=sys.stderr,
            )
            return 2
        if "∈" not in real:
            print(
                "结论：G-95 仍在 —— 真库的目标是展开体（`A a`），显示层折不回 `a ∈ A`。"
                "根因是 `lib/Set.sokonanoda` 的 `subset` 定义体写成了展开形态。",
                file=sys.stderr,
            )
            return 0
        print("结论：G-95 已修 —— 真库的目标保持记法目标名，显示层折成 `a ∈ A` ✓。")
        return 1
    except (RuntimeError, StopIteration) as error:
        print(f"   → {error}", file=sys.stderr)
        return 2
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
