#!/usr/bin/env python3
"""**横向阈值 ratchet 的判据**（用户 2026-10-15【②】的横向落地；形状照
`scripts/check-attention-budget.py` 的 ratchet ✓ —— 登记表与 `tighten()` 在
`scripts/threshold_ratchet.py`，本文件只**判**）。

## 它判三件（都能机械判）

  * **状态不变量**（`state_problems`）：caps **永不高于种子** · **逐轮严格下降**（到 floor
    为止）· 同族关系（incremental 告警线 < 红线）· `round`/`history` 自洽
    ⇒ 红线 ①「**caps 没比上轮低 ⇒ 判红**」✓
  * **接缝**（`seam_problems`）：**每个消费点必须真的读 ratchet**（`cap("<id>")` 标记）
    —— A 层产出（状态里的 caps）、B 层消费（4 个守卫）⇒ 没有这条对账，**状态收紧而消费者
    还用常量 = 装饰** ✗（照 `scripts/audit-wire-fields.py` 的先例 ✓）；
    ⭐ 另有 **标签接缝**（2026-10-16 补，同一洞的横向那一半）：**标签里不许写 ratchet 的值**
    —— `.github/workflows/ci.yml` 的 step 名改前写着种子（`活文档 ≤10.0 MB`）而现行 caps
    已是 `9.50 MB` ✗ ⇒ 用户/评审在 job 列表里读到的是**旧值** ✗（判据 = `label_cap_hits()`，
    **单位必须一致** ⇒ `≤800 行`/`≤30` 这类**静态常量不误伤** ✓）。
  * **开轮纪律**（`judge_idle`）：基点之后落太多笔还没开轮 ⇒ 判红（ratchet 不许空转 ✗）；
    ⚠ **收敛之后不判**（再开轮也降不下去 ⇒ 那会是一条**没有出路**的红 ✗）

## 开轮（`--open-round`）

读数（7 个，一个不缺）**必须落进"收紧后"的上限** ⇒ 才 `tighten()` 一档 + 记 `history`
（**读数进 git ⇒ 逐轮可查** ✓）。⚠ **为什么不照抄"读数必须严格低于上轮读数"**：那对
**有天然地板**的量（STATUS 行数 / 归档体积 / target 体积）是**假红生成器** ✗ —— 理由与
逐条对照写在 `scripts/threshold_ratchet.py` 的 docstring（**同形不同病要如实标注** ✓）。

## 退出码

  0 = 状态健康 + 接缝齐 + 没空转 / 1 = 判红 / 2 = 用法错 /
  3 = 环境错（状态缺失/坏 ⇒ **不假装判过** ✗）

## 用法

```bash
python3 scripts/check-threshold-ratchet.py                    # 状态 + 接缝 + 开轮纪律
python3 scripts/check-threshold-ratchet.py --measure          # 另打印 7 个读数（跑 4 个守卫）
python3 scripts/check-threshold-ratchet.py --open-round       # 开一轮
python3 scripts/check-threshold-ratchet.py --init             # 只在落地那一次：写第 0 轮基线
python3 scripts/check-threshold-ratchet.py --selftest         # 正反两向（含"接缝"与"空转"）
python3 scripts/check-threshold-ratchet.py --source-rev cf259d1f   # 接缝判据打在**历史版本**上
```
"""
from __future__ import annotations

import argparse
import datetime
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import threshold_ratchet as R  # noqa: E402  （同目录库；`scripts/` 已在 sys.path 上 ✓）
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）

STATE = R.STATE
ROUND_MAX_COMMITS = 40          # 与 ② 同源：基点之后落这么多笔还没开轮 ⇒ 判红


# ── 读数（**一处定义**：直接调消费者自己的量法 ⇒ 不抄第二份口径 ✗）──────────────
def _load_consumer(rel: str):
    """把 4 个守卫当**模块**加载（它们的量法就是判据的读数 —— 不许在别处重算一遍 ✗）。"""
    path = ROOT / rel
    spec = importlib.util.spec_from_file_location(f"_consumer_{path.stem.replace('-', '_')}", path)
    mod = importlib.util.module_from_spec(spec)
    sys.path.insert(0, str(ROOT / "scripts"))
    spec.loader.exec_module(mod)
    return mod


def measure(verbose: bool = False, target_root: str | None = None) -> tuple[dict, list[str]]:
    """7 个读数 ⇒ `(读数, 量不到的项)`。⚠ **量不到 ≠ 0** ✗（0 会被读成"很干净"）。

    `target_root`：量 target 体积的那个根（默认 `<repo>/target`）。⚠ worktree 里构建常落在
    **受管编译缓存**（`scripts/compile-cache.py env` ⇒ `CARGO_TARGET_DIR` 在别处）⇒ 默认量不到；
    这时**要么在树里有 target 的地方跑，要么显式指**（`--target-root`）—— **绝不记 0** ✗。
    """
    os.chdir(ROOT)                      # 消费者是 cwd 相对的（`tracked()` = git ls-files ✓）
    out: dict = {}
    missing: list[str] = []
    docs = _load_consumer("scripts/docs-lint.py")
    _bad, stats = docs.check()
    out["docs_live_bytes"] = stats["live_bytes"]
    out["docs_archive_bytes"] = stats["archive_bytes"]
    status = _load_consumer("scripts/status-lint.py")
    out.update(status.reading())
    site = _load_consumer("scripts/check-site.py")
    out.update(site.reading())
    th = _load_consumer("scripts/target-hygiene.py")
    troot = Path(target_root) if target_root else Path(th.REPO_ROOT) / "target"
    tstats = th.scan(troot, troot.parent)
    if not tstats.get("exists"):
        # ⚠ 没有 target/（例：worktree 把构建放进受管缓存）⇒ **量不到**，不许记 0 ✗
        missing += ["target_incremental_warn_bytes", "target_incremental_max_bytes"]
    else:
        out["target_incremental_warn_bytes"] = tstats["incremental_bytes"]
        out["target_incremental_max_bytes"] = tstats["incremental_bytes"]
    for cap_id in list(out):
        if cap_id not in R.REGISTRY:
            out.pop(cap_id)
    if verbose:
        print("读数（7 个；量法 = 各守卫自己的那一段 ✓）：")
        for cap_id in R.REGISTRY:
            if cap_id in out:
                print(f"  {cap_id:<32} {R.human(cap_id, out[cap_id])}"
                      f"   （上限 {R.human(cap_id, R.cap(cap_id))}）")
            else:
                print(f"  {cap_id:<32} **量不到**（不记 0 ✗）")
    return out, missing


def commits_since(head: str | None) -> int | None:
    """基点之后的提交数（浅克隆/坏 sha ⇒ `None` ⇒ **不假装判过** ✗）。"""
    if not head:
        return None
    proc = subprocess.run(["git", "rev-list", "--count", f"{head}..HEAD"], cwd=str(ROOT),
                          capture_output=True, text=True,
                          env=git_env.clean_git_env())
    if proc.returncode != 0:
        return None
    return int(proc.stdout.strip() or 0)


def check(state_path: Path, source_rev: str | None, as_json: bool) -> int:
    try:
        st = R.state_load(state_path)
    except R.RatchetError as err:
        print(f"error: {err}", file=sys.stderr)
        return 3
    red = R.state_problems(st)
    red += R.seam_problems(ROOT, source_rev=source_rev, caps=dict(st.get("caps") or {}))
    n_since = commits_since(st.get("head"))
    if n_since is None:
        red.append(f"量不到基点 `{str(st.get('head'))[:12]}` 之后的提交数（浅克隆 / sha 不在历史上）"
                   f"⇒ **不假装判过** ✗（照 `check-attention-budget.py` 的口径：CI 要 `fetch-depth: 0`）")
    else:
        red += R.judge_idle(st, n_since, ROUND_MAX_COMMITS)
    if as_json:
        print(json.dumps({"ok": not red, "round": st.get("round"), "head": st.get("head"),
                          "caps": st.get("caps"), "commits_since": n_since, "red": red},
                         ensure_ascii=False, indent=2))
    else:
        print(f"阈值 ratchet：第 **{st.get('round')}** 轮（基点 `{str(st.get('head'))[:12]}` · "
              f"之后 {n_since} 笔）· 现行 caps：")
        for cap_id in R.REGISTRY:
            spec = R.REGISTRY[cap_id]
            mark = "**已到收敛目标**" if st["caps"].get(cap_id) <= spec["floor"] else ""
            print(f"  {cap_id:<32} {R.human(cap_id, st['caps'][cap_id]):>10}"
                  f"   （种子 {R.human(cap_id, spec['seed'])} · 每轮 −"
                  f"{R.human(cap_id, spec['step'])} · 目标 {R.human(cap_id, spec['floor'])}）{mark}")
        print(f"  ⓘ 不在 ratchet 里的同形项（**登记 + 逐条理由**）：{len(R.EXCLUDED)} 条"
              f"（见 `scripts/threshold_ratchet.py` 的 `EXCLUDED`）")
    if red:
        print(f"\n✗ 阈值 ratchet：**{len(red)} 条红** ✗", file=sys.stderr)
        for item in red:
            print(f"  - {item}", file=sys.stderr)
        return 1
    print(f"\n✓ 阈值 ratchet：状态健康（caps 单调不增、永不高于种子）· 接缝齐"
          f"（{len(R.REGISTRY)}/{len(R.REGISTRY)} 消费点都读它 + "
          f"{len(R.LABEL_SEAM)}/{len(R.LABEL_SEAM)} 处标签里没有 ratchet 的值）· 没空转 ✓")
    return 0


def open_round(state_path: Path, target_root: str | None = None) -> int:
    try:
        st = R.state_load(state_path)
    except R.RatchetError as err:
        print(f"error: {err}", file=sys.stderr)
        return 3
    reading, missing = measure(verbose=True, target_root=target_root)
    if missing:
        print(f"✗ **量不到 {missing}** ⇒ 开轮要**齐全**的读数（「没量到 ≠ 绿」✗）—— "
              f"在**有 target/** 的树上跑（例：主 checkout）✓", file=sys.stderr)
        return 1
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=str(ROOT), capture_output=True,
                          text=True, env=git_env.clean_git_env()).stdout.strip()
    now = datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")
    new, red = R.open_round(st, reading, now=now, head=head)
    if red:
        print(f"✗ **本轮不算开成**（{len(red)} 条）✗ —— ratchet **不动** "
              f"（`{state_path}` 一个字节没写 ✓）：", file=sys.stderr)
        for item in red:
            print(f"  - {item}", file=sys.stderr)
        return 1
    R.state_save(new, state_path)
    print(f"✓ ratchet **第 {st['round']} → {new['round']} 轮** ✓（`{state_path}`）")
    for cap_id in R.REGISTRY:
        print(f"  {cap_id:<32} {R.human(cap_id, st['caps'][cap_id]):>10} → "
              f"**{R.human(cap_id, new['caps'][cap_id])}**")
    return 0


def init(state_path: Path, target_root: str | None = None) -> int:
    if state_path.exists():
        print(f"✗ `{state_path}` 已存在 ⇒ `--init` 只写**第 0 轮基线**（别覆盖历史 ✗）",
              file=sys.stderr)
        return 1
    reading, missing = measure(verbose=True, target_root=target_root)
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=str(ROOT), capture_output=True,
                          text=True, env=git_env.clean_git_env()).stdout.strip()
    now = datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")
    caps = R.seeds()
    st = {"version": 1, "round": 0, "head": head, "at": now, "caps": caps,
          "history": [{"round": 0, "head": head, "at": now, "caps": caps,
                       "reading": reading or None,
                       "reading_missing": missing or None}]}
    R.state_save(st, state_path)
    print(f"✓ 第 0 轮基线写入 `{state_path}`：caps = **种子**（= 落地时那批常量 ⇒ "
          f"本次改动**一个阈值都没放松** ✓）")
    return 0


# ── 自检（正反两向；每条都要有"故意坏 ⇒ 必须判红"的臂 ✓）─────────────────────
def selftest() -> int:
    bad = 0
    total = 0

    def gate(label: str, cond: bool, detail: str = "") -> None:
        nonlocal bad, total
        total += 1
        ok = bool(cond)
        print(f"  {'✓' if ok else '✗'} {label}" + (f" —— {detail}" if detail else ""))
        if not ok:
            bad += 1

    print("① `tighten()`：**单调不增** + **永不高于种子** + 到 floor 停住（40 轮）")
    caps = R.seeds()
    mono = above = True
    prev = dict(caps)
    for _ in range(40):
        caps = R.tighten(caps)
        if any(caps[k] > prev[k] for k in R.REGISTRY):
            mono = False
        if any(caps[k] > R.REGISTRY[k]["seed"] for k in R.REGISTRY):
            above = False
        prev = dict(caps)
    gate("① 40 轮**单调不增**", mono)
    gate("① 40 轮**永不高于种子**（= 一个阈值都没放松 ✓）", above)
    gate("① 40 轮后**全部停在收敛目标**（floor 只停住收紧、不回升）",
         all(caps[k] == R.REGISTRY[k]["floor"] for k in R.REGISTRY),
         f"docs_live = {R.human('docs_live_bytes', caps['docs_live_bytes'])}")
    gate("① 同族关系（incremental 告警线 < 红线）逐轮成立",
         all(R.tighten(R.tighten(R.seeds()))["target_incremental_warn_bytes"]
             < R.tighten(R.tighten(R.seeds()))["target_incremental_max_bytes"] for _ in [0]))

    print("\n② **连开两轮 ⇒ 第二轮低于第一轮**（真状态推进；读数 = 落地实测那批）")
    live = {"docs_live_bytes": 6_410_000, "docs_archive_bytes": 950_000,
            "status_total_lines": 85, "site_text_bytes": 60_823,
            "site_hero_bytes": 221_763, "target_incremental_warn_bytes": int(3.5 * R.GIB),
            "target_incremental_max_bytes": int(3.5 * R.GIB)}
    st = {"version": 1, "round": 0, "head": "f" * 40, "at": "t0", "caps": R.seeds(),
          "history": [{"round": 0, "head": "f" * 40, "at": "t0", "caps": R.seeds(),
                       "reading": live}]}
    st1, red1 = R.open_round(st, live, now="t1", head="a" * 40)
    st2, red2 = R.open_round(st1, live, now="t2", head="b" * 40)
    gate("② 第 1 轮开成（无红）", not red1, str(red1))
    gate("② 第 2 轮开成（无红）", not red2, str(red2))
    gate("② **第二轮 caps 严格低于第一轮**（用户【②】的判据）",
         all(st2["caps"][k] < st1["caps"][k] for k in R.REGISTRY),
         f"docs_live {R.human('docs_live_bytes', st1['caps']['docs_live_bytes'])} → "
         f"{R.human('docs_live_bytes', st2['caps']['docs_live_bytes'])}")
    gate("② 两轮都**不高于种子**", all(st2["caps"][k] <= R.REGISTRY[k]["seed"] for k in R.REGISTRY))
    gate("② 状态自洽（`state_problems` 无红）", not R.state_problems(st2))

    print("\n③ 反向：**读数高于收紧后的上限 ⇒ 判红 + 状态不动**（坏输入 = **仓库自己记录的**"
          "历史病态读数 ✓）")
    hist_bad = dict(live, target_incremental_warn_bytes=82 * R.GIB,
                    target_incremental_max_bytes=82 * R.GIB,
                    status_total_lines=1534)          # 82 GB / 1534 行 = 两处 docstring 里的实测值
    st_before = dict(st)
    st_after, red3 = R.open_round(st, hist_bad, now="t1", head="a" * 40)
    gate("③ incremental 82 GiB（实测病态值）⇒ 判红", any("target_incremental" in r for r in red3))
    gate("③ STATUS 1534 行（实测病态值）⇒ 判红", any("status_total_lines" in r for r in red3))
    gate("③ 判红时**状态不动**（一个字节不写 ✓）", st_after == st_before)

    print("\n④ 反向：**读数不全 ⇒ 判红**（「没量到 ≠ 绿」）")
    _st, red4 = R.open_round(st, {"docs_live_bytes": 1}, now="t1", head="a" * 40)
    gate("④ 缺 6 个读数 ⇒ 判红 + 点名", any("读数不全" in r for r in red4))

    print("\n⑤ 反向：**caps 没比上轮低 / 高于种子 ⇒ 判红**（红线 ①）")
    hand = json.loads(json.dumps(st2))
    hand["caps"]["docs_live_bytes"] = R.REGISTRY["docs_live_bytes"]["seed"] + 1
    gate("⑤ 手改成**高于种子** ⇒ 判红（『放松阈值』）",
         any("高于种子" in r for r in R.state_problems(hand)))
    hand2 = json.loads(json.dumps(st2))
    hand2["history"][-1]["caps"]["docs_live_bytes"] = hand2["history"][-2]["caps"]["docs_live_bytes"]
    hand2["caps"]["docs_live_bytes"] = hand2["history"][-1]["caps"]["docs_live_bytes"]
    gate("⑤ 手改成**与上轮持平**（还没到 floor）⇒ 判红（『本轮没比上轮低』）",
         any("没比上轮低" in r for r in R.state_problems(hand2)))
    hand3 = json.loads(json.dumps(st2))
    hand3["caps"]["target_incremental_warn_bytes"] = hand3["caps"]["target_incremental_max_bytes"]
    gate("⑤ 手改成**告警线 ≥ 红线** ⇒ 判红（同族关系）",
         any("告警线 ≥ 红线" in r for r in R.state_problems(hand3)))

    print("\n⑥ 反向：**状态缺失/坏 ⇒ 响亮**（不是静默绿 ✗）")
    with tempfile.TemporaryDirectory() as tmp:
        miss = Path(tmp) / "nope.json"
        try:
            R.state_load(miss)
            gate("⑥ 缺状态 ⇒ `RatchetError`", False, "没抛")
        except R.RatchetError as err:
            gate("⑥ 缺状态 ⇒ `RatchetError`（+ 出路 `--init`）", "--init" in str(err))
        broken = Path(tmp) / "broken.json"
        broken.write_text("{ not json", encoding="utf-8")
        try:
            R.state_load(broken)
            gate("⑥ 坏 JSON ⇒ `RatchetError`", False, "没抛")
        except R.RatchetError:
            gate("⑥ 坏 JSON ⇒ `RatchetError`", True)
        empty = Path(tmp) / "empty.json"
        empty.write_text(json.dumps({"round": 0}), encoding="utf-8")
        try:
            R.state_load(empty)
            gate("⑥ 缺字段 ⇒ `RatchetError`", False, "没抛")
        except R.RatchetError as err:
            gate("⑥ 缺字段 ⇒ `RatchetError` + 点名缺的那个字段",
                 "缺字段" in str(err) and "head" in str(err), str(err)[:80])
        rc = main(["--state", str(miss)])
        gate("⑥ 真 CLI：状态缺失 ⇒ **rc=3**（不假装判过）", rc == 3, f"rc={rc}")

    print("\n⑦ **接缝**：消费点不读 ratchet ⇒ 判红（坏输入 = **改前的源码**，`--source-rev`）")
    gate(f"⑦ 正对照：**当前**源码 {len(R.REGISTRY)}/{len(R.REGISTRY)} 消费点都有标记 + "
         f"标签里没有 ratchet 的值 ⇒ 无红", not R.seam_problems(ROOT),
         str(R.seam_problems(ROOT))[:120])
    old = R.seam_problems(ROOT, source_rev="cf259d1f")
    gate("⑦ 反向：**改前**（`cf259d1f`）的 7 个消费者**都没有**标记 + 标签里写着 ratchet 的值 "
         "⇒ 判红 + 逐条点名",
         len(old) == len(R.REGISTRY) + len(R.LABEL_SEAM)
         and all("找不到" in r or "step 标签里写着 ratchet 的值" in r for r in old), f"{len(old)} 条")
    # ⚠ 真 CLI 臂要**自带一个合法状态**（否则它量到的是"状态缺失 ⇒ rc=3"而不是
    #   "接缝判红 ⇒ rc=1" ⇒ 那是一条**量错了对象**的臂 ✗；本会话第一版就是这么错的）。
    with tempfile.TemporaryDirectory() as tmp2:
        ok_state = Path(tmp2) / "ok.json"
        ok_state.write_text(json.dumps({
            "version": 1, "round": 0, "head": "f" * 40, "at": "t0", "caps": R.seeds(),
            "history": [{"round": 0, "head": "f" * 40, "at": "t0", "caps": R.seeds(),
                         "reading": live}]}), encoding="utf-8")
        rc = main(["--state", str(ok_state), "--source-rev", "cf259d1f"])
    gate("⑦ 反向：真 CLI `--source-rev cf259d1f`（同一份合法状态）⇒ **rc=1**", rc == 1, f"rc={rc}")

    print("\n⑦b **标签接缝**（caps 不许在 step 标签里写第二遍 —— 同一洞的横向那一半 ✓）")
    with tempfile.TemporaryDirectory() as tmp3:
        t3 = Path(tmp3)
        (t3 / ".github" / "workflows").mkdir(parents=True)
        (t3 / "scripts").mkdir()
        # 夹具把**消费者那一半**先弄绿（只留标签这一格可变 ⇒ 臂**量的是标签** ✓，不是别的 ✗）。
        # ⚠ **按文件聚合标记**（一个消费者可能消费**多个** id：`docs-lint.py` 两个 ·
        #   `check-site.py` 两个 · `target-hygiene.py` 两个 ⇒ 逐 id 覆盖写会只剩最后一个 ✗）。
        by_file: dict[str, list[str]] = {}
        for _spec in R.REGISTRY.values():
            by_file.setdefault(_spec["seam"][1], []).append(_spec["seam"][0])
        for _rel, _markers in by_file.items():
            _p = t3 / _rel
            _p.parent.mkdir(parents=True, exist_ok=True)
            _p.write_text("import threshold_ratchet\n" + "\n".join(_markers) + "\n",
                          encoding="utf-8")
        (t3 / ".github" / "workflows" / "ci.yml").write_text(
            "      - name: 文档预算 lint（体积上限见 ratchet）\n", encoding="utf-8")
        gate("⑦b 正对照：夹具的**消费者那一半**已绿（否则下面的臂量错了对象 ✗）",
             not R.seam_problems(t3), str(R.seam_problems(t3))[:90])
        # **反向**（改前那一行的形状）：标签里写着种子 ⇒ 必须判红
        (t3 / ".github" / "workflows" / "ci.yml").write_text(
            "      - name: 文档预算 lint（活文档 ≤10.0 MB · 入口 ≤800 行）\n", encoding="utf-8")
        _lab = R.seam_problems(t3)
        gate("⑦b 反向：标签里写着**种子**（`≤10.0 MB`）⇒ **只这一条**判红",
             len(_lab) == 1 and "step 标签里写着 ratchet 的值" in _lab[0],
             f"{len(_lab)} 条：{(_lab[0][:60] if _lab else '**没判红** ✗')}")
        # **正对照**：标签只说"查什么"、不说"查到多少" ⇒ 无红（**注释**里的历史读数不算 ✗）
        (t3 / ".github" / "workflows" / "ci.yml").write_text(
            "      # 实测 17.13 > 17.0 ⇒ 那条红没有出路 ✗\n"
            "      - name: 文档预算 lint（体积上限见 ratchet · 入口 ≤800 行）\n", encoding="utf-8")
        gate("⑦b 正对照：标签不带上限数字（注释里的历史读数**不算** ✗）⇒ 无红",
             not R.seam_problems(t3), str(R.seam_problems(t3))[:90])
        # **精度**：静态常量（**不是**任何 id 的 seed/caps/floor）⇒ **不许误伤** ✗→✓
        (t3 / ".github" / "workflows" / "ci.yml").write_text(
            "      - name: 文档预算 lint（入口 ≤800 行 · 设计 ≤150 行/冻结 · 每段 ≤30 · 净增 ≤60）\n",
            encoding="utf-8")
        gate("⑦b 正对照：**静态常量**（`≤800 行`/`≤150 行`/`≤30`/`≤60` 都不是 ratchet 的值）不误伤",
             not R.seam_problems(t3), str(R.seam_problems(t3))[:90])
        # **反向**：咬的不只是种子 —— **现行 caps** 写进标签同样判红 ✓
        _cur = R.label_cap_hits("      - name: x（活文档 ≤9.50 MB）\n",
                                {"docs_live_bytes": 9_500_000})
        gate("⑦b 反向：标签写着**现行 caps**（`≤9.50 MB`）也判红（不只咬种子 ✓）",
             bool(_cur), str(_cur)[:80])
        # **反向**：`--source-rev` 量不到 ⇒ 判红（**不假装判过** ✗；打在**真仓**上量一个假 rev ✓）
        gate("⑦b 反向：`--source-rev` 量不到 ⇒ 判红（不假装判过）",
             any("量不到" in r for r in
                 R.seam_problems(ROOT, source_rev="0000000000000000000000000000000000000000")))

    print("\n⑧ **开轮纪律**（空转 ⇒ 判红；**收敛之后不判** ⇒ 红必须有出路 ✓）")
    gate("⑧ 基点之后 ≤ 上限笔数 ⇒ 不判",
         not R.judge_idle(st, ROUND_MAX_COMMITS, ROUND_MAX_COMMITS))
    idle = R.judge_idle(st, ROUND_MAX_COMMITS + 1, ROUND_MAX_COMMITS)
    gate("⑧ 超过上限 ⇒ 判红 + 给出路 `--open-round`",
         bool(idle) and "--open-round" in idle[0])
    floored = dict(st, caps={k: v["floor"] for k, v in R.REGISTRY.items()})
    gate("⑧ **全部到收敛目标之后不判**（否则是一条没有出路的红 ✗）",
         not R.judge_idle(floored, 10_000, ROUND_MAX_COMMITS))

    print()
    if bad:
        print(f"✗ selftest 判红：{bad}/{total} 条不符 ✗")
        return 1
    print(f"✓ selftest 全过：{total}/{total} 条符合 ✓（含 ① 单调/不放松 · ② 连开两轮更低 · "
          f"③④⑤ 反向 · ⑥ 状态缺失响亮 · ⑦ 接缝打在改前源码上必红 · ⑦b 标签接缝（种子/现行 caps 都咬、静态常量不误伤）· ⑧ 空转）")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="横向阈值 ratchet（状态 + 接缝 + 开轮纪律）")
    ap.add_argument("--state", default=str(STATE), help="ratchet 状态文件")
    ap.add_argument("--measure", action="store_true", help="打印 7 个读数（跑 4 个守卫）")
    ap.add_argument("--open-round", action="store_true", help="开一轮（要求读数落进收紧后的上限）")
    ap.add_argument("--init", action="store_true", help="只在落地那一次：写第 0 轮基线")
    ap.add_argument("--json", action="store_true", help="机器可读")
    ap.add_argument("--target-root", default=None, metavar="DIR",
                    help="量 target 体积的根（默认 `<repo>/target`；worktree 里构建落在别处时显式指）")
    ap.add_argument("--source-rev", default=None, metavar="REV",
                    help="接缝判据打在**历史版本**的消费点源码上（咬住证明用）")
    ap.add_argument("--selftest", action="store_true", help="判据通道自检（正反两向）")
    args = ap.parse_args(argv)
    state_path = Path(args.state)

    if args.selftest:
        return selftest()
    if args.init:
        return init(state_path, args.target_root)
    if args.open_round:
        return open_round(state_path, args.target_root)
    if args.measure:
        _reading, missing = measure(verbose=True, target_root=args.target_root)
        if missing:
            print(f"⚠ 量不到：{missing}（**不记 0** ✗）", file=sys.stderr)
        return 0
    return check(state_path, args.source_rev, args.json)


if __name__ == "__main__":
    sys.exit(main())
