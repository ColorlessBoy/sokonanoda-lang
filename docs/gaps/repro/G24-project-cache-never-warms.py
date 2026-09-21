#!/usr/bin/env python3
"""G-24 自断言复现：**清单 `requires` 与二进制版本漂移 ⇒ 项目编译缓存被静默关掉**。

用户原话：「没有实现编译后的文件加速 vscode 处理」「新打开一个文件就有临时编译」。

台账 `today`（0.63.0 实测，修前）——同一个 3 行项目、隔离缓存、`build` 连跑两次：

    requires = "0.61"  → 第一次 0 hit, 1 compiled ｜ 第二次 **0 hit, 1 compiled**  ← 永不入缓存
    requires = "0.1"   → 第一次 0 hit, 1 compiled ｜ 第二次 **0 hit, 1 compiled**  ← 永不入缓存
    requires = "0.63"  → 第一次 0 hit, 1 compiled ｜ 第二次 **1 hit, 0 compiled**  ← 正常
    无清单             → 第一次 0 hit, 1 compiled ｜ 第二次 **1 hit, 0 compiled**  ← 正常

`courses/set-theory/` 的实测：`query check` 冷跑 **4.873s**，`build` 之后仍是 4.836s；
把清单的 `requires` 从 `"0.61"` 改成 `"0.63"` 后同一个命令立刻 `1 hit`。

**根因（用户 2026-09-21 判定）**：**开发过程没有自动提升项目清单的 `requires`**。
`courses/set-theory` 与语言仓**同仓共同开发**，版本本来应当一致；它手写着
`requires = "0.61"`，而二进制已经 0.63.0 ⇒ 漂移。

**漂移为什么能关掉整个缓存**（机制）：`version_warning`
（`crates/front/src/project/manifest.rs:135-144`）只比 **major.minor**
⇒ `ProjectPlan.requires_warning = Some(…)`（`crates/front/src/project/mod.rs:235`）
⇒ `ProjectReport::is_clean()`（`crates/front/src/project/report.rs:158-166`）为假
⇒ `store_if_clean`（`crates/cli/src/project_cache.rs:33-41`）**永不写缓存**
⇒ 卷 I 每个文件每次打开/每次按键都从零重编整个闭包。

**两条写缓存路径规则不一致**（顺带发现）：`build`/`check` 走
`store_if_clean(…, project.is_clean())`（`crates/cli/src/build.rs:136`、
`crates/cli/src/check.rs:83`）；而 `query` 走 `set_cached_entry`
（`crates/front/src/query/mod.rs:214`）**不看 `is_clean`**。所以"`query` 跑一次之后
`build` 就命中了"——**同一个项目的缓存命不命中，取决于你先跑了哪条命令**。

**`sorry` 不是原因**：`sorry` 只体现为 `exercise_open` 计数，**不在**
`module.report.warnings` 里；实测"入口带 `sorry`、无清单"的项目**能正常入缓存**。

**"静默"是这条缺口的另一半**：`query check`（JSON 视图）里**看不到任何提示**
（`warnings: []`）；只有 `check` 的人类视图往 **stderr** 打一行
`warning[manifest-version]`（`crates/cli/src/check.rs:163-165`）——而编辑器走的是
JSON / LSP 通道。

修后契约（三条都要）：
  ① **`requires` 不再手写漂移**：`courses/` 与 `course/` 的清单版本与仓库版本
     **同一来源**（或由门禁挡住漂移）——这是用户判定的根因；
  ② 即使漂移发生，**也不该静默关掉缓存**（漂移是可回放的确定性事实），
     且必须在 JSON / LSP 视图里**可见**；
  ③ 两条写缓存路径（`build`/`check` 与 `query`）**规则一致**。

退出码约定（全部 repro 脚本一致，见 docs/gaps/README.md）：
  0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（需要人看）
"""

from __future__ import annotations

import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[3]
SOKO = ROOT / "scripts" / "soko"
NODE = shutil.which("node")

LIB_LINES = [
    "def Set (α : Type) : Type := α -> Prop",
    "def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a",
    'infix:50 " ∈ " => Set.mem',
    "def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x",
    'infix:50 " ⊆ " => Set.subset',
]
for i in range(1, 13):
    LIB_LINES.append(f"theorem lib_lemma_{i} (α : Type) (A B : Set α) (h : A ⊆ B) : A ⊆ B := h")
LIB = "\n".join(LIB_LINES) + "\n"

CANVAS = "\n".join(
    [
        "import SetLib",
        "",
        "theorem closed_one (α : Type) (A B : Set α) (h : A ⊆ B) : A ⊆ B := h",
        "",
        "theorem open_one (α : Type) (A B : Set α) : A ⊆ B := sorry",
        "",
    ]
)

BUILD_SUMMARY = re.compile(r"(\d+)\s+hit,\s*(\d+)\s+compiled,\s*(\d+)\s+failed")
# 版本漂移的痕迹（形态由实现定，这里只认关键字）。
DRIFT_HINT = re.compile(r"manifest-version|requires", re.IGNORECASE)


def run(args: list[str], cache_dir: pathlib.Path) -> tuple[float, subprocess.CompletedProcess]:
    env = dict(os.environ)
    env["SOKONANODA_CACHE_DIR"] = str(cache_dir)
    for key in ("SOKONANODA_NO_CACHE", "SOKONANODA_BIN", "SOKONANODA_LSP_BIN"):
        env.pop(key, None)
    start = time.perf_counter()
    proc = subprocess.run(
        [NODE, str(SOKO), *args], capture_output=True, text=True, env=env, cwd=str(ROOT)
    )
    return time.perf_counter() - start, proc


def measure(workdir: pathlib.Path, label: str) -> dict:
    """`build` 连跑两次（隔离缓存）：第二次必须命中。

    **不能用 `query` 打头**：`query` 走 `set_cached_entry`（不看 `is_clean`），
    会先把条目写进去，把这条缺口掩盖掉——这正是"两条写缓存路径规则不一致"。
    """
    entry = workdir / "Canvas.sokonanoda"
    cache_dir = workdir / "cache"
    first, first_proc = run(["build", str(entry)], cache_dir)
    second, second_proc = run(["build", str(entry)], cache_dir)
    summary = BUILD_SUMMARY.search(second_proc.stdout or "")
    if summary is None:
        raise RuntimeError(f"读不出第二次 build 的摘要：{(second_proc.stdout or '').strip()!r}")
    hits, compiled, failed = (int(g) for g in summary.groups())
    _, query_proc = run(["query", "check", "--file", str(entry)], cache_dir)
    if query_proc.returncode != 0:
        raise RuntimeError(f"query check 退出码 {query_proc.returncode}：{query_proc.stderr.strip()}")
    print(
        f"   [{label}] build#1 {first * 1000:.0f}ms · build#2 {second * 1000:.0f}ms "
        f"= {hits} hit/{compiled} compiled"
    )
    return {
        "first": first,
        "second": second,
        "hits": hits,
        "json": query_proc.stdout or "",
    }


def fixture(root: pathlib.Path, manifest: str | None) -> pathlib.Path:
    root.mkdir(parents=True, exist_ok=True)
    (root / "SetLib.sokonanoda").write_text(LIB, encoding="utf-8")
    (root / "Canvas.sokonanoda").write_text(CANVAS, encoding="utf-8")
    if manifest is not None:
        (root / "sokonanoda.toml").write_text(manifest, encoding="utf-8")
    return root


def main() -> int:
    if NODE is None:
        print("   → 需要 node（scripts/soko 是 Node 启动器）", file=sys.stderr)
        return 2

    base = pathlib.Path(tempfile.mkdtemp(prefix="g24-"))
    try:
        # 对照组：无清单 ⇒ 今天就能入缓存（证明量具本身没问题）。
        control = measure(fixture(base / "control", None), "无清单（对照）")

        # 实验组：清单 requires 与二进制漂移 ⇒ 今天永不入缓存。
        stale = measure(
            fixture(
                base / "stale",
                '# 模拟 courses/set-theory：requires 落后于二进制\nname = "stale"\nrequires = "0.1"\n',
            ),
            "requires 漂移",
        )
    except RuntimeError as error:
        print(f"   → {error}", file=sys.stderr)
        return 2
    finally:
        shutil.rmtree(base, ignore_errors=True)

    if control["hits"] < 1:
        print("   → 对照组（无清单）也没入缓存 ⇒ 量具或环境有问题，需要人看。", file=sys.stderr)
        return 2

    caches = stale["hits"] >= 1
    visible = bool(DRIFT_HINT.search(stale["json"]))

    print()
    print("== 清单 requires 漂移时：缓存是否仍然工作 + 漂移是否可见 ==")
    print(f"   ① 漂移下缓存仍然工作（第二次 build 命中）= {caches}")
    print(f"   ② 漂移在 `query check` 的 JSON 里可见 = {visible}")

    if caches and visible:
        print("结论：G-24 已修——requires 漂移不再关掉缓存，且漂移在 JSON 视图里可见。")
        return 1
    if caches and not visible:
        print(
            "结论：G-24 仍在——缓存虽然工作了，但 requires 漂移在 JSON 视图里仍不可见"
            "（编辑器走的就是这条通道）。",
            file=sys.stderr,
        )
        return 0
    print(
        "结论：G-24 仍在——清单 requires 与二进制漂移时，项目编译缓存被静默关掉"
        f"（第二次 build 仍是 {stale['hits']} hit，JSON 里可见 = {visible}）。",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
