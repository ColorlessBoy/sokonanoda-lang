#!/usr/bin/env python3
"""G-26 自断言复现：**Infoview 的「目标」栏不显示记法（点名形式）**。

用户原话：「infoview 里的 goal 展现没有用 notation 的方式」。

台账 `today`（0.63.0 实测，修前）——光标停在 `sorry` 上（`query state`，
也就是 `soko/stateAt` 的同一条真相层）：

    源文件写的是：  theorem mem_of_subset (α : Type) (A B : Set α) : A ⊆ B → ∀ a, a ∈ A → a ∈ B
    目标栏显示的是：forall (α : Type 0) (A B : Set α),
                    Set.subset α A B -> (forall (a : α), Set.mem α a A -> Set.mem α a B)

**goal 文本有四个生产者，不是一个**（这是本条最容易修错的地方）：

  | 生产者 | 谁在用 | 文本来源 | 记法 |
  |---|---|---|---|
  | 根状态（`step:-1`，光标在 `by`/`sorry`） | `soko/stateAt` 目标栏 | `DeclState.ty_text` = **内核 pp** | **丢** |
  | 无 `by` 的开练习 | `soko/goals` 的 `decl.goal` | `render_expr(ty)` | 保留 |
  | 声明列表的 `ty` | 声明卡片 | 同根状态（内核 pp） | **丢** |
  | `by` 步进 | 目标栏进度 | `render_expr`，但经 `canonical_goal*`/`apply`/`cases` 会被换成 pp 文本 | 部分丢 |

  所以断言要**按 surface 分开**——本脚本钉的是**根状态**（用户看到的那一个）。

**不能走内核 pp**（调研结论，2026-09-21）：`pp_expr` 同时是
`#check`/`#reduce`/`#print` 的出口 ⇒ 改它会动 `--json` 字节。安全的路是
front 侧的**显示边界重写**（`docs/notes/course-lean-style/printback-feasibility.md` §4）。

修后契约：光标在 `sorry` 上时，目标的 `goal` 文本里出现源文件用的记法
（`⊆` 或 `∈`），且 `Eq` 之类的点名形式仍可回退。

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
NODE = shutil.which("node")

LIB = "\n".join(
    [
        "def Set (α : Type) : Type := α -> Prop",
        "def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a",
        'infix:50 " ∈ " => Set.mem',
        "def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x",
        'infix:50 " ⊆ " => Set.subset',
        "",
    ]
)

# 课程的实际写法：`:= by` + 下一行 `sorry`（卷 I 的 30 处练习都是这个形状）。
# **这正是缺口的关键**：带 `by` ⇒ 走「根状态」生产者（内核 pp，丢记法）；
# 不带 `by` 的 `:= sorry` 走「无 by 的开练习」生产者（`render_expr`，**保留**记法）。
# 注意 `→ ∀ …` 要加括号——`X → ∀ y, P` 今天**不解析**（已另立缺口 G-28）。
CANVAS = "\n".join(
    [
        "import SetLib",
        "",
        "theorem with_by (α : Type) (A B : Set α) : A ⊆ B → (∀ (a : α), a ∈ A → a ∈ B) := by",
        "  sorry",
        "",
        "theorem without_by (α : Type) (A B : Set α) : A ⊆ B := sorry",
        "",
    ]
)

# 记法字符（源文件用了哪些，目标文本里就该出现哪些）。
NOTATION = ("⊆", "∈")
# 点名形式的痕迹（修后不应再主导显示）。
POINTFUL = ("Set.subset", "Set.mem")


def query_state(entry: pathlib.Path, line: int, col: int) -> dict:
    env = dict(os.environ)
    for key in ("SOKONANODA_BIN", "SOKONANODA_LSP_BIN"):
        env.pop(key, None)
    proc = subprocess.run(
        [
            NODE, str(SOKO), "query", "state",
            "--file", str(entry), "--line", str(line), "--col", str(col), "--compact",
        ],
        capture_output=True, text=True, env=env, cwd=str(ROOT),
    )
    if proc.returncode != 0:
        raise RuntimeError(f"query state 退出码 {proc.returncode}：{proc.stderr.strip()}")
    return json.loads(proc.stdout)


def main() -> int:
    if NODE is None:
        print("   → 需要 node（scripts/soko 是 Node 启动器）", file=sys.stderr)
        return 2

    workdir = pathlib.Path(tempfile.mkdtemp(prefix="g26-"))
    try:
        (workdir / "SetLib.sokonanoda").write_text(LIB, encoding="utf-8")
        entry = workdir / "Canvas.sokonanoda"
        entry.write_text(CANVAS, encoding="utf-8")

        lines = CANVAS.splitlines()

        def state_at(needle: str, nth: int) -> tuple[str, str, int, int]:
            """第 `nth` 次出现 `needle` 的那一行（1 基）上的状态。"""
            seen = 0
            for i, text in enumerate(lines, start=1):
                if needle in text:
                    seen += 1
                    if seen == nth:
                        return probe(i, text.index(needle) + 1)
            raise RuntimeError(f"夹具里找不到第 {nth} 个 {needle!r}")

        def probe(line: int, col: int) -> tuple[str, str, int, int]:
            answer = query_state(entry, line, col)
            if not answer.get("ok"):
                raise RuntimeError(f"query state({line}:{col}) 答 ok:false：{answer.get('error')}")
            data = answer["data"]
            return (data.get("goal") or "", (data.get("decl") or {}).get("name") or "", line, col)

        print("== 两个生产者，两种行为 ==")
        results = {}
        for label, nth, expect_notation in (("带 by（课程写法）", 1, True), ("不带 by（对照）", 2, False)):
            goal, decl, line, col = state_at("sorry", nth)
            has_notation = any(ch in goal for ch in NOTATION)
            pointful = [name for name in POINTFUL if name in goal]
            results[label] = has_notation
            print(f"   [{label}] {decl} @ {line}:{col}")
            print(f"      显示 = {goal}")
            print(f"      含记法 = {has_notation} · 残留点名 = {pointful}")

        if not results["带 by（课程写法）"]:
            print(
                "结论：G-26 仍在——课程写法（`:= by` + `sorry`）的目标文本是点名形式，"
                "与源文件写的记法不一致。",
                file=sys.stderr,
            )
            return 0
        if not results["不带 by（对照）"]:
            print("   → 对照（不带 by）反而丢了记法 ⇒ 需要人看。", file=sys.stderr)
            return 2
        print("结论：G-26 已修——两个生产者的目标文本都用上了源文件的记法。")
        return 1
    except RuntimeError as error:
        print(f"   → {error}", file=sys.stderr)
        return 2
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
