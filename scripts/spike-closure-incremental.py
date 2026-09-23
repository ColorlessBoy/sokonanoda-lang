#!/usr/bin/env python3
"""T-K20 的 spike：量「同进程依次打开多个入口」里有多少编译是白跑的。

设计 `docs/design/closure-incremental.md`。它只**量**，不改任何行为。

对每个入口：① 取闭包的模块集合（`query project`）；② 冷编译一次计时（全新缓存）；
③ 检查"上一个入口已编好的模块集合"是不是本次闭包的**合法前缀**
（downward-closed：每个已编模块的依赖也都在集合里）——这是 K2-b 的**正确性守卫**；
④ 算出"只编新增模块"能省多少。

用法：python3 scripts/spike-closure-incremental.py [--json]
"""
from __future__ import annotations

import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "sokonanoda"

# 单调链：每个入口的闭包都是上一个的超集（这样"边际 = 本次 − 上次"才成立）。
CHAIN = [
    "courses/set-theory/units/unit01-sets-membership.sokonanoda",
    "courses/set-theory/units/unit08-images-preimages.sokonanoda",
    "courses/set-theory/units/unit12-synthesis.sokonanoda",
]


def run(args: list[str], env: dict[str, str]) -> tuple[int, str]:
    p = subprocess.run(args, capture_output=True, text=True, env=env, cwd=str(ROOT))
    return p.returncode, p.stdout


def probe(rel: str) -> dict | None:
    """冷编译一次（全新缓存目录）并取闭包结构。"""
    cache = tempfile.mkdtemp(prefix="spike-closure-")
    try:
        env = dict(os.environ, SOKONANODA_CACHE_DIR=cache)
        t0 = time.time()
        code, out = run([str(BIN), "query", "project", "--file", rel, "--compact"], env)
        ms = int((time.time() - t0) * 1000)
        if code != 0:
            return None
        data = json.loads(out)["data"]
        project = data.get("project")
        if project is None:
            return None
        modules = project["modules"]
        return {
            "entry": rel,
            "ms": ms,
            "modules": [m["name"] for m in modules],
            "imports": {m["name"]: list(m["imports"]) for m in modules},
        }
    finally:
        shutil.rmtree(cache, ignore_errors=True)


def downward_closed(done: set[str], imports: dict[str, list[str]]) -> bool:
    """`done` 是不是本次闭包的合法前缀：每个已编模块的依赖也在 `done` 里。"""
    for name in done:
        for dep in imports.get(name, []):
            if dep not in done and dep in imports:
                return False
    return True


def main() -> int:
    if not BIN.exists():
        print(f"需要 release 构建：{BIN}", file=sys.stderr)
        return 2
    results = []
    done: set[str] = set()
    total_today = 0
    total_k2b = 0
    for rel in CHAIN:
        info = probe(rel)
        if info is None:
            print(f"跳过（编译失败）：{rel}", file=sys.stderr)
            continue
        mods = set(info["modules"])
        new = sorted(mods - done)
        reusable = sorted(mods & done)
        closed = downward_closed(mods & done, info["imports"]) if reusable else True
        marginal = info["ms"] - (results[-1]["ms"] if results and reusable else 0)
        info.update(new=new, reusable=reusable, closed=closed, marginal=max(marginal, 0))
        results.append(info)
        total_today += info["ms"]
        total_k2b += info["marginal"] if closed else info["ms"]
        done |= mods

    print(f'{"入口":44} {"模块":>4} {"冷编译ms":>9} {"只编新增":>9} {"合法前缀":>8}  新增模块')
    for r in results:
        short = r["entry"].replace("courses/set-theory/", "")
        mark = "✓" if r["closed"] else "✗（整编）"
        print(
            f'{short:44} {len(r["modules"]):>4} {r["ms"]:>9} {r["marginal"]:>9} {mark:>8}'
            f'  {" ".join(m.replace("lib.", "").replace("units.", "") for m in r["new"])}'
        )
    print()
    print(f"依次打开三个入口：今天 {total_today}ms → K2-b {total_k2b}ms"
          f"（{(total_k2b - total_today) / total_today * 100:+.1f}%）")
    if "--json" in sys.argv:
        print(json.dumps({"schema": "soko.spike/1", "chain": results,
                          "total_today_ms": total_today, "total_k2b_ms": total_k2b},
                         ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
