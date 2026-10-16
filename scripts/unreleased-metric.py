#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""**发版读数（交付维度）**：`未发版 N 笔 · 其中用户面 M · 距上次发版 H 小时`。

## 为什么有这个脚本（用户 2026-10-16 11:2x 的追问 · 队列 `SUB-release-metric-measures-unreleased`）

用户引编排者自己的「会话累计」表追问：

    清账 Agent 不会检查还有多少没发版吧，我看它每次给我发的会话累计

那一格写的是「⭐ 发版 ⭐ **v0.91.0 已发布并闭环（26 资产）** ✓」—— 而同一时刻实测：

    最新 tag v0.91.0（2026-10-15 12:39）· tag 之后 **209 笔**提交（全都没发版）
    其中用户面 34 条 · 距今 **22.9 小时** ⇒ 用户手上仍是 0.91.0，今天修的一条都没到

⇒ **不是撒谎，是量具瞎** ✗：那一格量的是「**历史上发过版吗**」—— 一个**二进制**事实，
一旦为真**永远为真** ⇒ 它**永远不会**提醒「该发版了」✗。同一张表里「队列未清 32 → 8」
「用户提交 11 条已收口」都是**可数、会涨会降**的 ⇒ **只有「发版」那一格是死的** ✓。
⇒ 本脚本就是那一格的**量具**：把它换成**未发版量**（可数 · 会涨 · 会判红）✓。

## 口径（**唯一一处** = 本文件的 `TIERS` 表；`--explain` 印出来给评审）

一笔提交按**最高面**归类（优先级 = 表里的顺序）⇒ 分项**恰好剖分**全部提交（分项之和 = 总数 ✓）：

| 面 | 路径 | 为什么算这一面 |
|---|---|---|
| **产物** | `crates/**` `Cargo.toml` `Cargo.lock` `editor/**` | 进发布产物 —— 8 CLI + 8 LSP tarball 与 9 个 VSIX 都由它们构建（`docs/RELEASE.md` §2） |
| **课程** | `courses/**` `examples/**` `playground.sokonanoda` | 用户**打开并判卷**的语料 |
| **仓库交付** | `site/**` `skills/**` `.agents/**` `dsh/**` | 官网数据 · agent 手册（用户与他的 agent 直接读） |
| **工具** | `scripts/**` `.github/**` `docs/gaps/**` | 门禁 / CI / 复现件 —— 改的是**用户看到的判定结果**，但不进产物 |
| **簿记** | 其余（`docs/**` 除 gaps · `STATUS.md` · 根 `*.md` · 队列台账…） | 关于工作的账，不是交付 |

**用户面 = 产物 ∪ 课程 ∪ 仓库交付 ∪ 工具**（= 不是「簿记」的那些）✓ —— 与用户自己点名的那串
（front/cli/lsp/editor/courses/elab/kernel/parser/cache/**guard/gates/ci**）**口径一致**，
但**判据在这里、不在提示词里** ✗：路径表机械可判，改口径必须改这一处（评审可见 ✓）。

## 判红线（用户【②】）

    未发版提交 > `MAX_COMMITS`(50)  **或**  距上次发版 > `MAX_HOURS`(24)  ⇒ **红**（rc=1）

⇒ 效果：**它不能 22 小时不发版还自报 ✓** ✗→✓。

## 为什么它在 pre-push 那一步**只出声、不当闸**（`scripts/ci-local.sh` 故意不传播 rc ✗）

发版是**人工 ops 动作**（bump 两处版本 → push main → `ci.yml:426` 的 auto-tag ⇒ `release.yml`），
**不是**代码能自己解决的事 ⇒ 拿它判红会是一条**没有出路的红**（照
`scripts/check-attention-budget.py` 的 `judge_round()` 先例：再开轮也降不下去 ⇒ 判红会是
一条没有出路的红 ✗ ⇒ 只告警）。⇒ 本脚本的 rc 服务的是**报告那一格**（读者看 rc/文字），
pre-push 那一步只**出声** + 写明出路（走一次发布闭环）✓。

## ⚠ 发版机制的另一半：**读数为什么只会长**

`ci.yml:445-459` 的 auto-tag 是**版本号触发**的：`Cargo.toml` 的版本**没有**对应 tag 时才
打 tag 并发版（`git ls-remote --exit-code --tags origin refs/tags/v<ver>` ⇒ 命中就
`::notice::… already exists — nothing to release` 然后 `exit 0`）；**不动版本号推 main
⇒ 只跑 CI、不发版**（`docs/RELEASE.md:4-8`）。⇒ 版本号停在 `0.91.0` 的每一天，
未发版量都**只增不减** —— 这正是本读数要让它**看得见**的东西 ✓。

## 用法 / 退出码

    python3 scripts/release-metric.py                # 报告那一格（一行）+ 明细；超线 ⇒ rc=1
    python3 scripts/release-metric.py --json         # 机器读数（rc 同上）
    python3 scripts/release-metric.py --explain      # 印口径表（路径 → 面），rc=0
    python3 scripts/release-metric.py --selftest     # 合成 git 夹具，正反两向（咬住证明的 `pass`）
    # 冻结输入的钉子（咬住证明 / 反向验证用；默认 = 活仓的 HEAD 与最新 v* tag）
    --tag <ref>   --head <rev>   --now <iso8601>   --max-commits N   --max-hours H

  0 = 未超线 ✓ · 1 = 超线 ✗ · 2 = **环境/形状异常**（无 `v*` tag · rev 解析不了 ·
      `--now` 早于 tag）—— **拿不到读数绝不静默通过** ✗
"""

from __future__ import annotations

import argparse
import datetime as dt
import fnmatch
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# ── **口径（唯一一处 ✓）** ────────────────────────────────────────────────────
# 顺序 = 归类优先级（一笔提交按**最高面**归类 ⇒ 分项恰好剖分全部提交 ✓）。
TIERS: list[tuple[str, tuple[str, ...], str]] = [
    ("产物", ("crates/*", "Cargo.toml", "Cargo.lock", "editor/*"),
     "进发布产物：8 CLI + 8 LSP tarball 与 9 个 VSIX 都由它们构建"),
    ("课程", ("courses/*", "examples/*", "playground.sokonanoda"),
     "用户打开并判卷的语料"),
    ("仓库交付", ("site/*", "skills/*", ".agents/*", "dsh/*"),
     "官网数据 · agent 手册（用户与他的 agent 直接读）"),
    ("工具", ("scripts/*", ".github/*", "docs/gaps/*"),
     "门禁 / CI / 复现件 —— 改用户看到的判定结果，但不进产物"),
]
BOOK = "簿记"
BOOK_WHY = "其余（docs/** 除 gaps · STATUS.md · 根 *.md · 队列台账…）：关于工作的账，不是交付"

# 判红线（用户【②】的建议值；`--max-commits` / `--max-hours` 可覆盖，供反向验证用）
MAX_COMMITS = 50
MAX_HOURS = 24.0

GIT = ["git"]


class ShapeError(Exception):
    """环境/形状异常 ⇒ exit 2（**绝不静默通过** ✗）。"""


def git(root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess:
    p = subprocess.run(GIT + list(args), cwd=str(root), capture_output=True,
                       text=True, encoding="utf-8", errors="replace")
    if check and p.returncode != 0:
        raise ShapeError(f"`git {' '.join(args)}` ⇒ rc={p.returncode}：{(p.stderr or p.stdout).strip()[:200]}")
    return p


def parse_iso(text: str) -> dt.datetime:
    """ISO8601 → 带时区的 datetime（无时区 ⇒ 按 UTC，如实写在 `--now` 的 help 里 ✓）。"""
    try:
        t = dt.datetime.fromisoformat(text.strip().replace("Z", "+00:00"))
    except ValueError as err:
        raise ShapeError(f"`--now {text}` 不是 ISO8601：{err}") from err
    return t.replace(tzinfo=dt.timezone.utc) if t.tzinfo is None else t


def classify(files: list[str]) -> str:
    """一笔提交落在哪个面（**最高面** ✓）。"""
    for name, globs, _why in TIERS:
        for g in globs:
            if any(fnmatch.fnmatch(f, g) for f in files):
                return name
    return BOOK


def measure(root: Path, *, tag_ref: str | None, head_ref: str, now: dt.datetime | None) -> dict:
    """核心读数（**只读 git**，不联网 ✓）。形状异常 ⇒ `ShapeError`。"""
    if git(root, "rev-parse", "--git-dir", check=False).returncode != 0:
        raise ShapeError(f"{root} 不是 git 仓库 ⇒ 量不出发版读数")
    head = git(root, "rev-parse", "--verify", f"{head_ref}^{{commit}}").stdout.strip()
    if tag_ref:
        tag = tag_ref
        if git(root, "rev-parse", "--verify", f"{tag_ref}^{{commit}}", check=False).returncode != 0:
            raise ShapeError(f"`--tag {tag_ref}` 在本仓里解析不成提交")
    else:
        found = git(root, "tag", "--merged", head, "--list", "v[0-9]*",
                    "--sort=-v:refname").stdout.split()
        if not found:
            raise ShapeError(f"{head[:8]} 上没有任何 `v*` tag ⇒ 量不出「距上次发版」，"
                             "**不静默通过** ✗（真要量就用 `--tag <ref>` 显式钉一个）")
        tag = found[0]
    tag_commit = git(root, "rev-parse", f"{tag}^{{commit}}").stdout.strip()
    tag_time = parse_iso(git(root, "log", "-1", "--format=%cI", tag_commit).stdout)
    now = now or dt.datetime.now().astimezone()
    hours = (now - tag_time).total_seconds() / 3600.0
    if hours < 0:
        raise ShapeError(f"`--now`（{now.isoformat()}）早于 tag `{tag}` 的时间"
                         f"（{tag_time.isoformat()}）⇒ 形状异常，不判")

    raw = git(root, "log", "--format=%x1e%H%x1f%s", "--name-only",
              f"{tag_commit}..{head}").stdout
    commits: list[tuple[str, str, list[str]]] = []
    for chunk in raw.split("\x1e"):
        if not chunk.strip():
            continue
        head_line, _, rest = chunk.partition("\n")
        sha, _, subject = head_line.partition("\x1f")
        files = [f for f in rest.split("\n") if f.strip()]
        commits.append((sha.strip(), subject.strip(), files))

    tiers: dict[str, int] = {name: 0 for name, _g, _w in TIERS}
    tiers[BOOK] = 0
    for _sha, _subject, files in commits:
        tiers[classify(files)] += 1
    user_facing = len(commits) - tiers[BOOK]

    over: list[str] = []
    if len(commits) > MAX_COMMITS:
        over.append("commits")
    if hours > MAX_HOURS:
        over.append("hours")
    return {
        "head": head, "head_short": head[:8],
        "latest_tag": tag, "tag_commit": tag_commit, "tag_time": tag_time.isoformat(),
        "now": now.isoformat(), "hours": round(hours, 2),
        "commits": len(commits), "user_facing": user_facing,
        "product": tiers["产物"], "tiers": tiers,
        "thresholds": {"max_commits": MAX_COMMITS, "max_hours": MAX_HOURS},
        "over": over, "ok": not over,
    }


def cell_line(m: dict, max_commits: int, max_hours: float) -> str:
    """**报告那一格**（一行，可直接抄进「会话累计」表 ✓）。"""
    verdict = "✓ 绿" if m["ok"] else "✗ 红"
    return (f"⭐ 发版 | 未发版 {m['commits']} 笔（用户面 {m['user_facing']} · "
            f"产物 {m['product']}）· 距上次发版 {m['hours']:.1f}h · 最新 tag {m['latest_tag']} "
            f"⇒ {verdict}（红线：>{max_commits} 笔 或 >{max_hours:g}h）")


def render(m: dict, max_commits: int, max_hours: float) -> str:
    lines = [cell_line(m, max_commits, max_hours)]
    parts = " · ".join(f"{name} {m['tiers'][name]}" for name, _g, _w in TIERS)
    lines.append(f"  ↳ 分面（一笔按**最高面**归类 ⇒ 分项恰好剖分 {m['commits']} 笔）："
                 f"{parts} · {BOOK} {m['tiers'][BOOK]}")
    lines.append("  ↳ 口径：用户面 = 产物 ∪ 课程 ∪ 仓库交付 ∪ 工具（`--explain` 印路径表；"
                 "唯一出处 = 本脚本的 `TIERS`）")
    if m["over"]:
        why = " · ".join({"commits": f"未发版 {m['commits']} 笔 > {max_commits}",
                          "hours": f"距今 {m['hours']:.1f}h > {max_hours:g}h"}[k]
                         for k in m["over"])
        lines.append(f"  ↳ 触线：{why}")
        lines.append("  ↳ 出路：走一次发布闭环（bump 两处版本 → push main → "
                     "`ci.yml:426` 的 auto-tag ⇒ `release.yml`）—— "
                     "**不动版本号推 main 只跑 CI、不发版**（`docs/RELEASE.md:4-8`）")
    return "\n".join(lines)


def explain() -> str:
    lines = ["口径表（**唯一一处** = `scripts/release-metric.py` 的 `TIERS`；"
             "一笔按**最高面**归类 ⇒ 分项恰好剖分 ✓）：", ""]
    for i, (name, globs, why) in enumerate(TIERS, 1):
        lines.append(f"  {i}. **{name}**：{' '.join(f'`{g}`' for g in globs)}")
        lines.append(f"     ↳ {why}")
    lines.append(f"  {len(TIERS) + 1}. **{BOOK}**：{BOOK_WHY}")
    lines.append("")
    lines.append("用户面 = 产物 ∪ 课程 ∪ 仓库交付 ∪ 工具（= 不是「簿记」的那些）✓")
    lines.append(f"红线：未发版 > {MAX_COMMITS} 笔 **或** 距上次发版 > {MAX_HOURS:g}h ⇒ 红（rc=1）")
    return "\n".join(lines)


# ── `--selftest`：**合成 git 夹具**（真 `git init`，不是打桩 ✓）正反两向 ───────────
BASE = dt.datetime(2026, 1, 1, 0, 0, tzinfo=dt.timezone.utc)


def _commit(root: Path, rel: str, when: dt.datetime, subject: str) -> None:
    p = root / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    with p.open("a", encoding="utf-8") as fh:
        fh.write("x\n")
    env = {"GIT_AUTHOR_DATE": when.isoformat(), "GIT_COMMITTER_DATE": when.isoformat()}
    git(root, "add", "-A")
    subprocess.run(GIT + ["commit", "-q", "-m", subject], cwd=str(root), check=True,
                   capture_output=True, env={**_env(), **env})


def _env() -> dict:
    import os
    e = dict(os.environ)
    e.setdefault("GIT_AUTHOR_NAME", "fixture")
    e.setdefault("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
    e.setdefault("GIT_COMMITTER_NAME", "fixture")
    e.setdefault("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
    return e


def _run_cli(argv: list[str], root: Path) -> tuple[int, str]:
    """在**子进程**里跑本脚本（与咬住证明跑的路径逐字相同 ✓）。

    ⚠ `--root` **必须显式给夹具根** ✗：本脚本默认的根是**自己所在的那个仓**
    （`ROOT = __file__/..`）—— 少了这个参数，夹具臂会**静默量真仓**
    （实测踩到过：7 臂里 6 臂红，红的是"量错对象"而不是"判据坏了" ✓）。
    """
    p = subprocess.run([sys.executable, str(Path(__file__).resolve()),
                        "--root", str(root), *argv],
                       cwd=str(root), capture_output=True, text=True,
                       encoding="utf-8", errors="replace")
    return p.returncode, (p.stdout or "") + (p.stderr or "")


def selftest() -> int:
    fails: list[str] = []
    arms = 0

    def arm(name: str, ok: bool, reading: str) -> None:
        nonlocal arms
        arms += 1
        print(f"  {'✓' if ok else '✗'} {name} ⇒ {reading}")
        if not ok:
            fails.append(name)

    with tempfile.TemporaryDirectory(prefix="release-metric-fixture-") as td:
        root = Path(td)
        git(root, "init", "-q")
        git(root, "config", "user.email", "fixture@example.invalid")
        git(root, "config", "user.name", "fixture")

        # 夹具：tag 落在第 1 笔上；第 2 笔动 crates/（产物面）· 第 3 笔动 `STATUS.md`（簿记）
        _commit(root, "crates/a.rs", BASE, "feat(front): a")
        git(root, "tag", "v1.0.0")
        _commit(root, "crates/b.rs", BASE + dt.timedelta(hours=1), "feat(front): b")
        _commit(root, "STATUS.md", BASE + dt.timedelta(hours=1), "docs(status): x")
        now = (BASE + dt.timedelta(hours=2)).isoformat()

        # ① 读数正确：2 笔未发版，用户面 1（产物 1 · 簿记 1）
        rc, out = _run_cli(["--json", "--now", now], root)
        m = json.loads(out[out.index("{"):out.rindex("}") + 1]) if "{" in out else {}
        arm("① 读数（2 笔 / 用户面 1 / 产物 1 / 簿记 1）",
            rc == 0 and m.get("commits") == 2 and m.get("user_facing") == 1
            and m.get("tiers", {}).get("产物") == 1 and m.get("tiers", {}).get("簿记") == 1,
            f"rc={rc} commits={m.get('commits')} 用户面={m.get('user_facing')} tiers={m.get('tiers')}")

        # ② **反向验证（用户写死的形状）**：0 笔 ⇒ **必须印 0** 且**绿**（不许恒印警告 ✗）
        rc, out = _run_cli(["--tag", "HEAD", "--head", "HEAD", "--now", now], root)
        arm("② 反向：距上次发版 0 笔 ⇒ 印 0 且绿（不恒真 ✗）",
            rc == 0 and "未发版 0 笔" in out and "✓ 绿" in out,
            f"rc={rc} · {out.strip().splitlines()[0][:90] if out.strip() else '(空)'}")

        # ③ 判红线：>50 笔 ⇒ rc=1（同一夹具上再堆 60 笔）
        for i in range(60):
            _commit(root, f"scripts/s{i}.py", BASE + dt.timedelta(hours=2), f"chore: s{i}")
        rc, out = _run_cli(["--now", now], root)
        arm("③ 超线（62 笔 > 50）⇒ rc=1 + 印出笔数",
            rc == 1 and "未发版 62 笔" in out and "✗ 红" in out,
            f"rc={rc} · {out.strip().splitlines()[0][:90]}")

        # ④ **读数不是写死的**：把红线抬到 1000 笔 / 1000h ⇒ 同一夹具**转绿**
        rc, out = _run_cli(["--now", now, "--max-commits", "1000", "--max-hours", "1000"], root)
        arm("④ 抬红线到 1000 ⇒ 转绿（它真在读那个数 ✓）",
            rc == 0 and "未发版 62 笔" in out and "✓ 绿" in out,
            f"rc={rc} · {out.strip().splitlines()[0][:90]}")

        # ⑤ 只超小时数也判红（hours 那条腿单独咬得住）
        rc, out = _run_cli(["--now", now, "--max-commits", "1000", "--max-hours", "0.5"], root)
        arm("⑤ 只超小时（2.0h > 0.5h）⇒ rc=1",
            rc == 1 and "✗ 红" in out, f"rc={rc}")

    # ⑥ 没有 `v*` tag ⇒ **exit 2**（拿不到读数绝不静默通过 ✗）
    with tempfile.TemporaryDirectory(prefix="release-metric-notag-") as td:
        root = Path(td)
        git(root, "init", "-q")
        git(root, "config", "user.email", "fixture@example.invalid")
        git(root, "config", "user.name", "fixture")
        _commit(root, "a.txt", BASE, "chore: a")
        rc, out = _run_cli([], root)
        arm("⑥ 没有任何 v* tag ⇒ rc=2 + 人话", rc == 2 and "量不出" in out,
            f"rc={rc} · {out.strip().splitlines()[-1][:80] if out.strip() else '(空)'}")

    # ⑦ `--head` 解析不了 ⇒ **exit 2**
    with tempfile.TemporaryDirectory(prefix="release-metric-badhead-") as td:
        root = Path(td)
        git(root, "init", "-q")
        git(root, "config", "user.email", "fixture@example.invalid")
        git(root, "config", "user.name", "fixture")
        _commit(root, "a.txt", BASE, "chore: a")
        git(root, "tag", "v1.0.0")
        rc, out = _run_cli(["--head", "deadbeef"], root)
        arm("⑦ `--head deadbeef` ⇒ rc=2 + 人话", rc == 2 and "deadbeef" in out, f"rc={rc}")

    print(f"\n{'✓' if not fails else '✗'} `--selftest`：{arms - len(fails)}/{arms} 臂成立"
          + (f"；判红：{fails}" if fails else " ✓"))
    return 0 if not fails else 1


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="发版读数（交付维度）：未发版 N 笔 · 用户面 M · 距上次发版 H 小时",
        formatter_class=argparse.RawDescriptionHelpFormatter, epilog=explain())
    ap.add_argument("--tag", help="钉住「上次发版」的 ref（默认 = head 上版本号最大的 `v*` tag）")
    ap.add_argument("--head", default="HEAD", help="钉住「现在」的 rev（默认 HEAD）")
    ap.add_argument("--now", help="钉住「现在」的时刻（ISO8601；无时区按 UTC）—— 反向验证用")
    ap.add_argument("--max-commits", type=int, default=MAX_COMMITS,
                    help=f"未发版笔数的红线（默认 {MAX_COMMITS}）")
    ap.add_argument("--max-hours", type=float, default=MAX_HOURS,
                    help=f"距上次发版小时数的红线（默认 {MAX_HOURS:g}）")
    ap.add_argument("--json", action="store_true", help="机器读数")
    ap.add_argument("--explain", action="store_true", help="只印口径表（路径 → 面），rc=0")
    ap.add_argument("--selftest", action="store_true", help="合成 git 夹具正反两向（咬住证明的 pass）")
    ap.add_argument("--root", default=str(ROOT), help="仓库根（默认 = 本脚本的上一级）")
    args = ap.parse_args(argv)

    if args.selftest:
        return selftest()
    if args.explain:
        print(explain())
        return 0

    root = Path(args.root).resolve()
    try:
        m = measure(root, tag_ref=args.tag, head_ref=args.head,
                    now=parse_iso(args.now) if args.now else None)
    except ShapeError as err:
        print(f"✗ 发版读数**量不出来**（exit 2，不静默通过 ✗）：{err}", file=sys.stderr)
        return 2

    # 覆盖阈值只影响判定与呈现（读数本身不变 ✓）
    m["thresholds"] = {"max_commits": args.max_commits, "max_hours": args.max_hours}
    m["over"] = ([k for k, cond in (("commits", m["commits"] > args.max_commits),
                                    ("hours", m["hours"] > args.max_hours)) if cond])
    m["ok"] = not m["over"]
    m["cell"] = cell_line(m, args.max_commits, args.max_hours)

    if args.json:
        print(json.dumps(m, ensure_ascii=False, indent=1))
    else:
        print(render(m, args.max_commits, args.max_hours))
    return 0 if m["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
