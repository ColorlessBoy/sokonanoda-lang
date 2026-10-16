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
#    `--new` 省略 ⇒ **自动解析**（`$SOKONANODA_BIN`/`$SOKO_BIN` → `$CARGO_TARGET_DIR` →
#    `target/`，见 `bin_candidates()` ✓）；受管环境请给 `SOKONANODA_BIN` ✓
python3 scripts/check-json-identity.py \
    --baseline /tmp/target-v852/debug/sokonanoda \
    --new target/debug/sokonanoda

# ③ 自检（判据本身会不会空转）：**三格都要咬住** · **离线 · 不需要真 CLI** ✓
python3 scripts/check-json-identity.py --selftest
```

## 防空转（两条，缺一不可）

* **两侧都要"答得上"**：每个入口的 stdout 必须非空 ⇒ 否则"两边都崩了"也会被算成
  "逐字节相同" ✗（`--min-answered`，默认 0.9）；
* **`--selftest`**：三格 —— ① 拿 `/bin/echo` 当假基线 ⇒ 必须报差异并 exit 1 ✓ ·
  ② 两侧同一份假 CLI ⇒ 必须 exit 0（**不许假红** ✓）· ③ 两侧都不答 ⇒
  `--min-answered` 必须 exit 2（**判据空转闸** ✓）（咬不住的守卫等于没有 ✓）。
  ⚠ 它**不依赖真 CLI**（2026-10-16 修 ✗→✓，洞 `CHECK-JSON-IDENTITY-NOT-WIRED-AT-ALL`）：
  改前 `selftest()` 硬编码 `target/debug/sokonanoda` ⇒ 干净 worktree 里没有它、
  受管环境（`scripts/ci-local.sh` 的 `CARGO_TARGET_DIR=/tmp/soko-target`）里产物也
  **不在那个路径** ⇒ `FileNotFoundError` **traceback**（rc=1 是**崩**出来的，
  不是咬出来的 ✗）⇒ 这条自检**接不进任何 gate** ✗。⇒ 现在**零前提**
  （可离线、可亚秒级 ✓），已接进 `scripts/ci-local.sh` 与 `ci.yml` 的 `queue-lint` job ✓。
"""

import argparse
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
COURSE = ROOT / "courses" / "set-theory"

# ── 新版二进制的解析（**唯一一处** ✓）────────────────────────────────────────
# ⚠ **为什么要它**（2026-10-16 修 ✗→✓，洞 `CHECK-JSON-IDENTITY-NOT-WIRED-AT-ALL`）：
# 改前 `--new` 默认硬编码 `target/debug/sokonanoda` ⇒ ① 干净 worktree 里**没有**那个文件
# ② 受管环境下产物在 `$CARGO_TARGET_DIR`（`scripts/ci-local.sh:46` 默认
# `/tmp/soko-target`）⇒ **即便构建过也不在那个路径** ✗ ⇒ `FileNotFoundError` traceback。
# 口径照本仓既有的正解：`scripts/release-preflight.sh:64` 的
# `${CARGO_TARGET_DIR:-$PWD/target}` + `scripts/soko` 的「**先探它真的能答**」✓。
BIN_ENV_VARS = ("SOKONANODA_BIN", "SOKO_BIN")


def bin_candidates() -> list[str]:
    """候选二进制（顺序 = 显式逃生门 → 受管构建目录 → 仓库构建 ✓）。**唯一一处** ✓。"""
    out: list[str] = [os.environ[v] for v in BIN_ENV_VARS if os.environ.get(v)]
    tgt = os.environ.get("CARGO_TARGET_DIR") or "target"
    if not os.path.isabs(tgt):
        tgt = str(ROOT / tgt)
    for prof in ("release", "debug"):
        out.append(os.path.join(tgt, prof, "sokonanoda"))
    for prof in ("release", "debug"):
        out.append(str(ROOT / "target" / prof / "sokonanoda"))
    seen: set[str] = set()
    uniq: list[str] = []
    for c in out:
        if c not in seen:
            seen.add(c)
            uniq.append(c)
    return uniq


def _answers(binary: str) -> bool:
    """探它**真的能答**（照 `scripts/soko` 的先例 ✓）：`version --json` rc=0 ✓。"""
    try:
        proc = subprocess.run(
            [binary, "version", "--json"], cwd=ROOT, capture_output=True, timeout=120
        )
    except (OSError, subprocess.SubprocessError):
        return False
    return proc.returncode == 0


def resolve_binary() -> str | None:
    """第一个**真能答**的候选；一个都没有 ⇒ `None`（调用方退 3，**不假装判过** ✓）。"""
    for cand in bin_candidates():
        if os.path.isfile(cand) and os.access(cand, os.X_OK) and _answers(cand):
            return cand
    return None


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
    try:
        proc = subprocess.run(
            [binary, "--json", rel], cwd=ROOT, env=env, capture_output=True, timeout=600
        )
    except OSError as exc:
        # ⚠ **前提不在 ⇒ 印人话 + 退 3**（不假装判过 ✗）：改前这里会冒 `FileNotFoundError`
        # **traceback**（读数不可归属 —— 读日志的人分不清"守卫咬不住"与"二进制不在" ✗）。
        print(
            f"✗ 前提不在：起不了 `{binary}`（{exc}）⇒ 基线/新版二进制不存在或不可执行。"
            f"出路：`cargo build -p sokonanoda-cli`，或给 `SOKONANODA_BIN` ✓"
            f"（**不假装判过** ✗）",
            file=sys.stderr,
        )
        raise SystemExit(3) from None
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


# 自检用的**假 CLI**（零前提 ✓）：`run_one()` 按 `[binary, "--json", rel]` 调它们 ✓。
_STUB_OK = (
    "#!/bin/sh\n"
    "# 假 CLI：同一份输入 ⇒ 同一份输出（验「相同 ⇒ 0」那一格 ✓）\n"
    "printf '{\"json\":\"%s\"}\\n' \"$2\"\n"
)
_STUB_SILENT = (
    "#!/bin/sh\n"
    "# 假 CLI：什么都不答（验 `--min-answered` 那一格 ✓）\n"
    "exit 0\n"
)


def _write_stub(path: pathlib.Path, body: str) -> None:
    path.write_text(body, encoding="utf-8")
    path.chmod(0o755)


def selftest() -> int:
    """**离线自检**（零前提 ✓）：三格都要咬住 —— 不同 ⇒ 1 · 相同 ⇒ 0 · 答不上 ⇒ 2。

    ⚠ **为什么改成不依赖真 CLI**（2026-10-16 修 ✗→✓，洞
    `CHECK-JSON-IDENTITY-NOT-WIRED-AT-ALL`）：改前只有第一格，且把"新版"硬编码成
    `target/debug/sokonanoda` ⇒ 干净 worktree / 受管环境（`CARGO_TARGET_DIR`）里
    **根本不在那个路径** ⇒ `FileNotFoundError` **traceback**（rc=1 是**崩**出来的、
    不是咬出来的 ✗）⇒ 这条自检**在任何地方都跑不成** ⇒ 也**接不进任何 gate** ✗。
    现在它**不依赖真 CLI**（`/bin/echo` 仍是"假基线" ⇒ **原意不变** ✓）⇒
    可离线 ✓ 亚秒级 ✓ **可接线** ✓（已接进 `scripts/ci-local.sh` 与 `ci.yml` 的
    `queue-lint` job ✓）。
    ⚠ **判据一条都没放宽** ✗：真 CLI 的全课程对拍路径（`--baseline` / `--new`）
    **逐字未动** ✓，而且从 **1 格加到 3 格**（新增「相同 ⇒ 0」与「答不上 ⇒ 2」✓）。
    """
    tmp = tempfile.mkdtemp(prefix="soko-json-identity-selftest-")
    try:
        ok = pathlib.Path(tmp) / "fake-cli-ok"
        silent = pathlib.Path(tmp) / "fake-cli-silent"
        _write_stub(ok, _STUB_OK)
        _write_stub(silent, _STUB_SILENT)
        cases = [
            ("假基线（/bin/echo）vs 会答的假 CLI ⇒ 必须判红", 1,
             lambda: sweep("/bin/echo", str(ok), 12, 0.0, True, limit=4)),
            ("同一份假 CLI 两边跑 ⇒ 必须判绿（不许假红）", 0,
             lambda: sweep(str(ok), str(ok), 12, 0.0, True, limit=4)),
            ("两侧都不答 ⇒ --min-answered 必须判红（判据空转闸）", 2,
             lambda: sweep(str(silent), str(silent), 12, 0.9, True, limit=4)),
        ]
        print(f"自检：{len(cases)} 格（离线 · 不需要真 CLI ✓）")
        bad = 0
        for name, want, fn in cases:
            got = fn()
            print(f"  {'✓' if got == want else '✗'} {name}（期望 {want} · 实得 {got}）")
            bad += got != want
        if bad:
            print(f"✗ 自检失败：{bad}/{len(cases)} 格不符 ⇒ 判据通道有问题 ✗",
                  file=sys.stderr)
            return 1
        print(f"✓ 自检通过：{len(cases)}/{len(cases)} 格都咬住 ✓（离线 · 不需要真 CLI ✓）")
        return 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--baseline", help="基线二进制（建议：已发布 tag 的构建 ✓）")
    ap.add_argument(
        "--new",
        help="新版二进制（默认：自动解析 —— `$SOKONANODA_BIN`/`$SOKO_BIN` → "
             "`$CARGO_TARGET_DIR` → `target/`，见 `bin_candidates()` ✓）",
    )
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
    ap.add_argument("--selftest", action="store_true", help="判据自检（三格都必须咬住）")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if not args.baseline:
        ap.error("--baseline 必填（或 --selftest）")
    new = args.new or resolve_binary()
    if not new:
        print(
            "✗ 前提不在：没探到**能答**的 sokonanoda 二进制（试过："
            + " · ".join(bin_candidates())
            + "）⇒ 出路：`cargo build -p sokonanoda-cli`，或给 `SOKONANODA_BIN` ✓"
            "（**不假装判过** ✗）",
            file=sys.stderr,
        )
        return 3
    return sweep(
        args.baseline, new, args.sample, args.min_answered, args.quiet, args.limit
    )


if __name__ == "__main__":
    sys.exit(main())
