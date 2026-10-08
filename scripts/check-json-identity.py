#!/usr/bin/env python3
"""全课程 `--json` **逐字节对拍**（发版大节点的红线判据 · 2026-10-08 本线批次收尾立的）。

## 为什么需要它

本仓库的红线是「同一批输入**接受/拒绝不变 · 事件计数不变 · golden 与 `--json` 逐字节
不变**」✓（`AGENTS.md` 硬规则 1）。日常用**贪心三件**（复现件 + 反向验证 + 受影响文件
逐字节）✓；**发版节点**要一次量**整门课**。

以前这条靠手搓命令 ✗（每次写法还不一样 ✗）⇒ 立成脚本 ✓：**同一份输入、两条基线、
退出码 + stdout 逐字节比** ✓。

## 用法

```bash
# ① 基线：从**已发布的 tag** 构建（不是"上一轮构建" ✗ —— 那等于自证）
git worktree add /tmp/v852 v0.85.2
(cd /tmp/v852 && CARGO_TARGET_DIR=/tmp/target-v852 cargo build -p sokonanoda-cli)

# ② 对拍（默认全量；--sample 3 只取 1/3，日常用）
python3 scripts/check-json-identity.py \
    --baseline /tmp/target-v852/debug/sokonanoda \
    --new target/debug/sokonanoda

# ③ 自检（判据本身会不会空转）：故意拿一个"假基线" ⇒ **必须判红**
python3 scripts/check-json-identity.py --selftest
```

## 防空转（两条，缺一不可）

* **两侧都要"答得上"**：每个入口的 stdout 必须非空 ⇒ 否则"两边都崩了"也会被算成
  "逐字节相同" ✗（`--min-answered`，默认 0.9）；
* **`--selftest`**：拿 `/bin/echo` 当假基线 ⇒ 必须报差异并 exit 1 ✓
  （咬不住的守卫等于没有 ✓）。
"""

import argparse
import os
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
COURSE = ROOT / "courses" / "set-theory"


def entries(sample: int) -> list[str]:
    """课程入口：全部 `lib/` + 每 `sample` 个 unit 取 1 + 每 `sample` 个解答取 1。"""
    # ⚠ **只枚举文件** ✗✓（2026-10-08 发版点实测）：`rglob("*.sokonanoda")` 会把
    # **模块根产物目录** `<模块根>/.sokonanoda/` **也当成一个"入口"** ✗ —— 于是
    # 同一棵树在两个时刻枚举出 **251**（目录不在）与 **252**（目录在）两种总数，
    # 读数不可比 ✗；那个"入口"还会被两侧同样地拒（空 stdout）⇒ 记成"相同"却
    # **没答上**（`--min-answered` 兜住 ✓，但数字是假的 ✗）。⇒ 判据只认文件 ✓。
    allf = sorted(
        str(p.relative_to(ROOT))
        for p in COURSE.rglob("*.sokonanoda")
        if p.is_file()
    )
    libs = [f for f in allf if "/lib/" in f]
    units = [f for f in allf if "/units/" in f and "/solutions/" not in f]
    sols = [f for f in allf if "/solutions/" in f]
    gaps = [f for f in allf if "/gaps/" in f]
    # ⚠ `gaps/` 下是**故意坏**的复现件（期望"被拒"）⇒ 与"逐字节对拍"不是同一件事，
    # 但两侧仍应**逐字节一致** ⇒ 一并纳入 ✓（它们同样在 `--json` 契约里 ✓）。
    return libs + units[::sample] + sols[::sample] + gaps


def run_one(binary: str, rel: str, cache: str) -> tuple[int, bytes]:
    shutil.rmtree(cache, ignore_errors=True)
    env = dict(
        os.environ,
        SOKONANODA_CACHE_DIR=cache,
        SOKONANODA_NO_PROJECT_ARTIFACTS="1",
    )
    proc = subprocess.run(
        [binary, "--json", rel], cwd=ROOT, env=env, capture_output=True, timeout=600
    )
    return proc.returncode, proc.stdout


def sweep(
    baseline: str, new: str, sample: int, min_answered: float, quiet: bool, limit: int = 0
) -> int:
    files = entries(sample)
    if limit:
        files = files[:limit]
    if not files:
        print("✗ 一个课程入口都没枚举到 —— 课程仓分开了？", file=sys.stderr)
        return 2
    same = diff = answered = 0
    bad: list[tuple[str, int, int]] = []
    for i, rel in enumerate(files, 1):
        base = run_one(baseline, rel, "/tmp/soko-json-identity-base")
        fresh = run_one(new, rel, "/tmp/soko-json-identity-new")
        if base[1]:
            answered += 1
        if base == fresh:
            same += 1
        else:
            diff += 1
            bad.append((rel, base[0], fresh[0]))
        if not quiet and i % 25 == 0:
            print(f"  … {i}/{len(files)}（相同 {same} · 不同 {diff}）", flush=True)
    ratio = answered / len(files)
    print(
        f"全课程 --json 对拍：**{same}/{len(files)} 逐字节相同** ✓ · {diff} 处不同 · "
        f"答得上 {answered}/{len(files)}（{ratio:.0%}）"
    )
    print(f"  基线 = {baseline}\n  新版 = {new}")
    for rel, a, b in bad[:10]:
        print(f"  ✗ {rel}（exit {a} vs {b}）")
    if ratio < min_answered:
        print(
            f"✗ **判据空转**：只有 {ratio:.0%} 的入口答得上（< {min_answered:.0%}）—— "
            f"两侧都崩了也会被算成「逐字节相同」✗",
            file=sys.stderr,
        )
        return 2
    return 1 if diff else 0


def selftest() -> int:
    """假基线（`/bin/echo`）⇒ **必须**判红 ✓（否则这条判据咬不住任何东西 ✗）。"""
    print("自检：拿 /bin/echo 当假基线 ⇒ 期望判红（exit 1）")
    code = sweep("/bin/echo", "target/debug/sokonanoda", 12, 0.0, True, limit=4)
    if code == 1:
        print("✓ 自检通过：假基线被判红 ✓")
        return 0
    print(f"✗ 自检失败：假基线没有被判红（exit {code}）⇒ 判据空转 ✗", file=sys.stderr)
    return 1


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--baseline", help="基线二进制（建议：已发布 tag 的构建 ✓）")
    ap.add_argument("--new", default="target/debug/sokonanoda", help="新版二进制")
    ap.add_argument(
        "--sample", type=int, default=1, help="unit/解答的抽样步长（1 = 全量 ✓）"
    )
    ap.add_argument(
        "--min-answered",
        type=float,
        default=0.9,
        help="「答得上」的最低比例（防空转 ✓）",
    )
    ap.add_argument("--quiet", action="store_true")
    ap.add_argument(
        "--limit", type=int, default=0, help="只跑前 N 个入口（冒烟用；0 = 不限 ✓）"
    )
    ap.add_argument("--selftest", action="store_true", help="判据自检（必须判红）")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if not args.baseline:
        ap.error("--baseline 必填（或 --selftest）")
    return sweep(
        args.baseline, args.new, args.sample, args.min_answered, args.quiet, args.limit
    )


if __name__ == "__main__":
    sys.exit(main())
