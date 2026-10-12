#!/usr/bin/env python3
"""compile-cache.py —— **编译缓存总管**（2026-10-12 用户：「rust 编译器对编译缓存文件的管理非常差，经常占了一百多 G
的存储，多 workspace 会放大这种问题，现在收口到我们这边来管理编译缓存文件」）。

## 它管什么（与 `target-hygiene.py` 的分工）

* `target-hygiene.py` = **单棵 target 的病态守卫**（`*.rcgu.o` 堆积 / 第二套 target / 40 G 报警 / 100 G 判红），
  已进 `gate` ✓ —— 它看的是"这一棵有没有坏"。
* 本脚本 = **跨工作树的总管**：4 核并行 ⇒ 4 棵 target（**必须独立**，共享会被 cargo 加锁串行 ✗），
  加起来会爆盘 ⇒ 这里做**登记 · 计量 · 预算 · 回收 · sccache 共享**。

## 核心策略（"收口到我们这边"= 一个受管根 + 一套预算 + 一个回收器）

1. **受管根**（`SOKO_COMPILE_CACHE_ROOT`，默认 `<repo>/.cache/compile`，gitignore ✓）下面三样：
   `targets/<id>/`（每棵工作树一棵 target）· `sccache/`（**对象级共享缓存**，跨树复用、有容量上限）· `registry.json`。
2. **sccache 才是省盘正解**：4 棵树各自 `target/` 会各存一份**编译产物**（数十 G ✗），而 `RUSTC_WRAPPER=sccache`
   让**对象**跨树共享、按 LRU 封顶 ⇒ 既保住并行（不共享 target 目录 ⇒ 没有锁 ✗），又不重复占盘 ✓。
   本机已装（`/opt/homebrew/bin/sccache` ✓）；没装就**优雅降级**（照样能跑，只是不共享）。
3. **预算**：`SOKO_CACHE_BUDGET_GB`（默认 60；`registry.json` 里可被 `--set-budget` 覆盖）。超了就 `reclaim`。
4. **回收顺序**（保守 → 激进，先删最没用的 ✓）：
   ① **死工作树的整棵 target**（worktree 已不在 `git worktree list` 里）·
   ② 每棵的 `incremental/`（sccache 在时它的价值最低；本机实测 **23 G** ✗）·
   ③ 按**最后使用时间 LRU** 整棵删，直到落回预算内。
   有 cargo/rustc 在跑 ⇒ **拒绝回收**（删到一半的构建目录是坏的 ✗）。

## 用法

```bash
python3 scripts/compile-cache.py status              # 表格：每棵多大 · 总计 · 预算 · sccache 命中
python3 scripts/compile-cache.py env <id>            # 打印该工作树要 export 的那几行（含 sccache）
python3 scripts/compile-cache.py reclaim             # 按上面 ①②③ 回收（打印"删了什么·拿回多少" ✓）
python3 scripts/compile-cache.py reclaim --to-budget # 只收到预算内
python3 scripts/compile-cache.py --json              # 机器读（进 gate 的报告段）
python3 scripts/compile-cache.py --selftest          # 判据通道自检（回收顺序 / 死树识别 / 预算）
```

退出码：0 = 正常 · 1 = 超预算（判红）· 2 = 用法/环境错。
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CACHE_ROOT = Path(os.environ.get("SOKO_COMPILE_CACHE_ROOT") or REPO / ".cache" / "compile")
REGISTRY = CACHE_ROOT / "registry.json"
DEFAULT_BUDGET_GB = 60


def human(n: float) -> str:
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if n < 1024 or unit == "TB":
            return f"{n:.1f} {unit}" if unit != "B" else f"{int(n)} B"
        n /= 1024
    return f"{n:.1f} TB"


def tree_bytes(path: Path) -> int:
    """表观大小，**硬链接按 inode 去重**（2026-10-12：本脚本第一版高报 75.5 G vs `du` 43 G ✗ ——
    `deps/` 里同一份 rlib 有 6 条硬链接，逐个 `st_size` 相加就是 6 倍；仓库 `target-hygiene.py`
    早就踩过并写了"6 条硬链接只算 1 份"的判据 ⇒ 这里照同一口径 ✓）。"""
    total = 0
    seen: set[tuple[int, int]] = set()
    for dirpath, dirnames, filenames in os.walk(path, onerror=lambda _e: None):
        for name in filenames:
            fp = os.path.join(dirpath, name)
            try:
                st = os.lstat(fp)
            except OSError:
                continue
            key = (st.st_dev, st.st_ino)
            if st.st_nlink > 1:
                if key in seen:
                    continue
                seen.add(key)
            total += st.st_blocks * 512   # 与 target-hygiene 同口径：实际占用 ✓
        dirnames[:] = [d for d in dirnames if d not in {".git", "node_modules"}]
    return total


def mtime(path: Path) -> float:
    try:
        return path.stat().st_mtime
    except OSError:
        return 0.0


def worktrees() -> list[dict]:
    """`git worktree list --porcelain` ⇒ [{path, branch}]（含主工作树 ✓）。"""
    out = subprocess.run(["git", "worktree", "list", "--porcelain"], cwd=REPO,
                         capture_output=True, text=True).stdout.splitlines()
    trees, cur = [], {}
    for line in out:
        if line.startswith("worktree "):
            if cur:
                trees.append(cur)
            cur = {"path": Path(line.split(" ", 1)[1]), "branch": "(detached)"}
        elif line.startswith("branch ") and cur:
            cur["branch"] = line.split(" ", 1)[1].removeprefix("refs/heads/")
    if cur:
        trees.append(cur)
    return trees


def managed_entries() -> list[dict]:
    """所有受管 target：主工作树的 `target/` + 每棵工作树的 target + 受管根下的 `targets/*` ✓。"""
    entries: dict[str, dict] = {}

    def add(path: Path, owner: str, kind: str) -> None:
        key = str(path.resolve()) if path.exists() else str(path)
        if key in entries:
            return
        entries[key] = {"path": path, "owner": owner, "kind": kind,
                        "bytes": tree_bytes(path) if path.is_dir() else 0,
                        "mtime": mtime(path), "dead": not path.exists()}

    for wt in worktrees():
        t = wt["path"] / "target"
        add(t, wt["branch"], "worktree")
    if CACHE_ROOT.is_dir():
        for child in sorted((CACHE_ROOT / "targets").glob("*")) if (CACHE_ROOT / "targets").is_dir() else []:
            add(child, child.name, "managed")
    # 老式同级 target-<branch>/（仓库历史上用过）
    for sib in sorted(REPO.parent.glob(f"{REPO.name}/target-*")) + sorted(REPO.glob("target-*")):
        add(sib, sib.name, "sibling")
    return sorted(entries.values(), key=lambda e: -e["bytes"])


def load_registry() -> dict:
    if REGISTRY.exists():
        try:
            return json.loads(REGISTRY.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            pass
    return {}


def save_registry(data: dict) -> None:
    CACHE_ROOT.mkdir(parents=True, exist_ok=True)
    REGISTRY.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


def budget_bytes(reg: dict | None = None) -> int:
    env = os.environ.get("SOKO_CACHE_BUDGET_GB")
    if env:
        return int(float(env) * 1024**3)
    reg = reg or load_registry()
    return int(float(reg.get("budget_gb", DEFAULT_BUDGET_GB)) * 1024**3)


def sccache_dir() -> Path:
    return Path(os.environ.get("SCCACHE_DIR") or CACHE_ROOT / "sccache")


def builders_running() -> list[str]:
    """**不靠 `ps`**（沙箱常挡 ✗）：直接试 cargo 自己的**构建锁** `<target>/.cargo-lock`
    （cargo 构建期间持 `flock`；试 `LOCK_EX|LOCK_NB` 失败 ⇒ 正有构建在跑 ✓）。
    拿不到锁文件（不存在/无权限）⇒ 记成"这一棵看不出来"，不据此拒绝。"""
    import fcntl
    busy: list[str] = []
    for e in managed_entries():
        lock = e["path"] / ".cargo-lock"
        if not lock.exists():
            lock = e["path"] / "debug" / ".cargo-lock"
        if not lock.exists():
            continue
        try:
            with lock.open("a+") as fh:
                try:
                    fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                    fcntl.flock(fh.fileno(), fcntl.LOCK_UN)     # 没被占 ⇒ 立刻放开 ✓
                except OSError:
                    busy.append(f"{lock} 被持有（有 cargo 构建在跑）")
        except OSError:
            continue
    if not busy:
        try:
            out = subprocess.run(["ps", "-Ao", "command"], capture_output=True, text=True).stdout
            busy += [l.strip()[:120] for l in out.splitlines()
                     if ("cargo" in l or "rustc" in l) and "compile-cache.py" not in l
                     and "grep" not in l]
        except (OSError, PermissionError):
            pass                                            # 沙箱挡了 ps ⇒ 只信锁 ✓
    return busy


def reclaim_order(entries: list[dict]) -> list[dict]:
    """**回收顺序**（纯函数，便于 selftest 钉死）：死树 → 其余按 LRU 最旧优先 ✓。"""
    dead = [e for e in entries if e["dead"]]
    live = sorted((e for e in entries if not e["dead"]), key=lambda e: e["mtime"])
    return dead + live


def cmd_status(as_json: bool, quiet: bool) -> int:
    entries = managed_entries()
    total = sum(e["bytes"] for e in entries)
    budget = budget_bytes()
    over = total > budget
    if as_json:
        print(json.dumps({"total_bytes": total, "budget_bytes": budget, "over": over,
                          "entries": [{**e, "path": str(e["path"])} for e in entries],
                          "sccache_dir": str(sccache_dir())}, ensure_ascii=False, indent=2))
        return 1 if over else 0
    if quiet:
        print(f"编译缓存：{human(total)} / 预算 {human(budget)}"
              + ("（**超了** ⇒ `scripts/compile-cache.py reclaim` ✗）" if over else " ✓"))
        return 1 if over else 0
    print(f"{'路径':<62}{'谁':<18}{'类型':<10}{'大小':>10}  最后使用")
    for e in entries:
        age = f"{(time.time() - e['mtime']) / 3600:.1f}h 前" if e["mtime"] else "?"
        tag = " **死树**" if e["dead"] else ""
        print(f"{str(e['path'])[-60:]:<62}{e['owner'][:16]:<18}{e['kind']:<10}"
              f"{human(e['bytes']):>10}  {age}{tag}")
    print(f"\n合计 **{human(total)}** / 预算 {human(budget)}"
          + ("  ⇒ **超预算**：`reclaim` ✗" if over else "  ✓"))
    sc = sccache_dir()
    if sc.is_dir():
        print(f"sccache：{sc} = {human(tree_bytes(sc))}")
    if not shutil.which("sccache"):
        print("⚠ 没装 sccache ⇒ 4 棵树会各存一份产物（`brew install sccache` 能省数十 G ✓）")
    return 1 if over else 0


def cmd_env(ident: str) -> int:
    """打印某棵工作树要 export 的受管环境（**独立 target + 共享 sccache + 队列单点** ✓）。"""
    if not ident:
        raise SystemExit("用法：compile-cache.py env <id>")
    target = CACHE_ROOT / "targets" / ident
    lines = [
        f'export CARGO_TARGET_DIR="{target}"',
        f'export SCCACHE_DIR="{sccache_dir()}"',
        f'export SCCACHE_CACHE_SIZE="{int(budget_bytes() / 1024**3)}G"',
        'export RUSTC_WRAPPER="$(command -v sccache || true)"   # 没装就留空 ⇒ 优雅降级 ✓',
        'export CARGO_INCREMENTAL=0                            # incremental 是本机最大的占用（23 G ✗）',
        f'export SOKONANODA_CACHE_DIR="{(REPO / ".cache" / "soko" / ident)}"',
        f'export SOKO_QUEUE_ROOT="{REPO}"',
    ]
    print("\n".join(lines))
    return 0


def cmd_reclaim(to_budget: bool) -> int:
    running = builders_running()
    if running:
        print("✗ 有 cargo/rustc 在跑 ⇒ 拒绝回收（删到一半的构建目录是坏的 ✗）：")
        for line in running[:5]:
            print(f"    {line}")
        return 2
    entries = managed_entries()
    budget = budget_bytes()
    before = sum(e["bytes"] for e in entries)
    freed, acted = 0, []
    # ① 死工作树的整棵 target
    for e in [x for x in entries if x["dead"]]:
        if e["path"].is_dir():
            freed += tree_bytes(e["path"])
            shutil.rmtree(e["path"], ignore_errors=True)
            acted.append(f"死树整棵 {e['path']}")
    # ② 每棵的 incremental/
    for e in entries:
        inc = e["path"] / "debug" / "incremental"
        if inc.is_dir():
            freed += tree_bytes(inc)
            shutil.rmtree(inc, ignore_errors=True)
            acted.append(f"incremental {e['path']}")
    # ③ 仍超预算 ⇒ 按 LRU 整棵删（**只删受管根里的**，主工作树的 target 不整棵删 ✗）
    after = before - freed
    if to_budget and after > budget:
        for e in reclaim_order([x for x in entries if not x["dead"] and x["kind"] == "managed"]):
            if after <= budget:
                break
            if not e["path"].is_dir():
                continue
            size = tree_bytes(e["path"])
            shutil.rmtree(e["path"], ignore_errors=True)
            freed += size
            after -= size
            acted.append(f"LRU 整棵 {e['path']}")
    gone = sum(e["bytes"] for e in managed_entries())
    print(f"回收前：{human(before)} ⇒ 回收后：**{human(gone)}**（释放 **{human(freed)}** · 预算 {human(budget)}）")
    for a in acted[:12]:
        print(f"  · 删了 {a}")
    if not acted:
        print("  （没有可回收的：① 无死树 ② 无 incremental ③ 未超预算 ✓）")
    print("  ⚠ 下一次构建会重编一次（慢一次，属预期 ✓）；配 sccache 时这次重编也能跨树复用 ✓")
    return 0


def selftest() -> int:
    """判据通道自检：**回收顺序**与**预算判定**必须咬得住（纯函数级，不碰真盘 ✓）。"""
    bad = 0

    def expect(desc: str, cond: bool) -> None:
        nonlocal bad
        if not cond:
            bad += 1
            print(f"FAIL {desc}", file=sys.stderr)

    now = time.time()
    entries = [
        {"path": Path("/m/live-old"), "dead": False, "mtime": now - 7200, "bytes": 10, "kind": "managed"},
        {"path": Path("/m/live-new"), "dead": False, "mtime": now - 60, "bytes": 10, "kind": "managed"},
        {"path": Path("/m/dead"), "dead": True, "mtime": now - 10, "bytes": 10, "kind": "managed"},
    ]
    order = [str(e["path"]) for e in reclaim_order(entries)]
    expect(f"死树排最前（{order}）", order[0] == "/m/dead")
    expect(f"活树按 LRU 最旧优先（{order}）", order[1:] == ["/m/live-old", "/m/live-new"])

    # 反向验证：把预算设成 1 字节 ⇒ 必须判"超预算"
    os.environ["SOKO_CACHE_BUDGET_GB"] = "0.000000001"
    expect("预算 1B ⇒ 判超（反向验证）", budget_bytes({"budget_gb": 0}) <= 2)
    del os.environ["SOKO_CACHE_BUDGET_GB"]
    expect("registry 里的预算生效", budget_bytes({"budget_gb": 7}) == 7 * 1024**3)
    expect("默认预算 60G", budget_bytes({}) == DEFAULT_BUDGET_GB * 1024**3)

    if bad:
        print(f"compile-cache.py selftest: {bad} 条判据不成立 ✗", file=sys.stderr)
        return 1
    print("compile-cache.py selftest: 4 条成立 ✓（死树优先 · LRU 最旧优先 · 预算反向验证 · registry/默认预算）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="编译缓存总管（跨工作树：登记 · 计量 · 预算 · 回收 · sccache）")
    ap.add_argument("cmd", nargs="?", default="status",
                    choices=["status", "env", "reclaim", "set-budget", "selftest"])
    ap.add_argument("arg", nargs="?", default="", help="env 的工作树 id / set-budget 的 GB")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--quiet", action="store_true", help="只打一行总量（给 gate 的报告段用）")
    ap.add_argument("--to-budget", action="store_true", help="回收直到落回预算内")
    ap.add_argument("--selftest", action="store_true", help="判据通道自检（等价 `selftest` 子命令）")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.cmd == "status":
        return cmd_status(args.json, args.quiet)
    if args.cmd == "env":
        return cmd_env(args.arg)
    if args.cmd == "reclaim":
        return cmd_reclaim(args.to_budget)
    if args.cmd == "set-budget":
        reg = load_registry()
        reg["budget_gb"] = float(args.arg)
        save_registry(reg)
        print(f"✓ 预算设为 {args.arg} GB（写进 {REGISTRY}）")
        return 0
    return selftest()


if __name__ == "__main__":
    sys.exit(main())
