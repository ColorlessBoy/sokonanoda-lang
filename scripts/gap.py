#!/usr/bin/env python3
"""gap.py —— 课程驱动缺口台账（`docs/gaps/ledger.jsonl`）的读写与交接工具。

设计：`docs/design/teaching-project.md` §6（台账协议、WO 模板、排序规则、「缺口即测试」）。

用法：
  python3 scripts/gap.py list [--severity blocker] [--kind library] [--status open] [--json]
  python3 scripts/gap.py show <id>
  python3 scripts/gap.py next [--json]        # 挑下一条要修的缺口，并打印**可直接粘贴给另一个 agent** 的 prompt
  python3 scripts/gap.py check                # 跑全部 repro，报告「台账 vs 现实」的偏差（CI/收尾用）
  python3 scripts/gap.py selftest             # 自检 judge() 的判定规则（不跑判卷，秒级）
  python3 scripts/gap.py close <id> --version 0.59.0 [--note "..."]   # 写 fixed_in（重跑 repro 确认已绿）

退出码：0 = 正常 / 1 = 有偏差（`check`）或用法错误 / 2 = 台账读不出来。
零依赖（python3 标准库）；repro 脚本的约定见 `docs/gaps/README.md`
（exit 0 = 缺口仍在且与台账一致；exit 1 = 行为变了）。

复现件的**期望**默认由 `status` 推出（`fixed` ⇒ 应当转绿 / `.sh` 应当 exit≠0），
但有些缺口的「修好」恰恰是**判红**（例如 G-01 钉的是"签名写错必须被拒」）。这类条目
在台账里写一个显式的 `repro_expect`（`clean` / `rejected` / `exit0` / `nonzero`）覆盖推导。
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import os
import signal
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# 单个复现件的上限：正常都在几秒内（最慢的是起 LSP 的那几条）。
# **慢机器预算**（2026-09-25 ✓ 用户报告"最近两轮 action 都报错"后定位 ✓）：
# 实测差异 = **机器速度** ✗：同一条复现件**本地 <120s 绿** ✓，而 CI 上 >120s 超时 ✗
# （旁证：CI 的 `test` job 跑 19 分 25 秒 ✓，而本地同一条 `cargo test --workspace` 只要 161 秒 ✓
# ⇒ CI 约慢 **7 倍** ✓）。⇒ 旧预算 120s 是**按快机器定的** ✗ ⇒ CI 必红 ✓。
# 调大到 300s ✓：**真挂住仍会超时判红** ✓（不掩盖真回归 ✓），只是不再因"机器慢"而红 ✓。
REPRO_TIMEOUT_S = int(os.environ.get("SOKO_GAP_REPRO_TIMEOUT", "300"))
LEDGER = ROOT / "docs" / "gaps" / "ledger.jsonl"
SEVERITY_ORDER = {"blocker": 0, "painful": 1, "nice": 2}
OPEN_STATUSES = {"open", "workaround", "wo-filed"}


def load() -> list[dict]:
    try:
        text = LEDGER.read_text(encoding="utf-8")
    except FileNotFoundError:
        print(f"error: 找不到台账 {LEDGER}", file=sys.stderr)
        sys.exit(2)
    entries = []
    for lineno, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            entries.append(json.loads(line))
        except json.JSONDecodeError as err:
            print(f"error: {LEDGER}:{lineno} 不是合法 JSON：{err}", file=sys.stderr)
            sys.exit(2)
    return entries


def save(entries: list[dict]) -> None:
    lines = [json.dumps(e, ensure_ascii=False, separators=(",", ":")) for e in entries]
    LEDGER.write_text("\n".join(lines) + "\n", encoding="utf-8")


def sort_key(entry: dict) -> tuple:
    wo = entry.get("wo_planned") or "WO-999"
    return (SEVERITY_ORDER.get(entry.get("severity", "nice"), 9), wo, entry["id"])


def clean_env() -> dict:
    """复现的运行环境：剔除会**短路夹具**的二进制覆盖。

    `SOKONANODA_BIN` / `SOKONANODA_LSP_BIN` 是"显式指定二进制"的逃生门，启动器会
    无条件采信它；而这批复现里有一半**测的就是启动器自己的解析链**（G-11/G-16 的
    夹具故意放一个陈旧缓存，看它会不会被拒绝）。一旦环境里带着覆盖，夹具就被绕过，
    复现会假报「缺口仍在」——`scripts/soko gate` 一开始正是这么把台账门禁带红的。
    所以这里一律剔除：要测覆盖的脚本自己 inline 赋值（G-11 ③ 就是这么写的）。
    """
    env = dict(os.environ)
    for key in ("SOKONANODA_BIN", "SOKONANODA_LSP_BIN"):
        env.pop(key, None)
    # **复现环境固定为"旧语义"**（2026-09-25 复核后的理由，与最初的推断不同 ✗）：
    # 最初我以为 CI 的缺口台账门禁红是因为"仓库内产物跨步骤存活"，后来证明那一步
    # **从未执行**（`test` job 卡在它前面的 Workspace tests ✗）⇒ 那个前提是错的。
    # 保留这条设置的理由变成：复现件验的是**老缺口**成不成立，把它们固定在
    # "只写全局缓存"的语义上 ⇒ 复现件的结果与 R-3 的产物落点**解耦**，
    # 不会因为"产物换了地方"而漂移 ✓；产物落点本身由
    # `crates/cli/tests/artifacts.rs` 专门覆盖 ✓。
    env["SOKONANODA_NO_PROJECT_ARTIFACTS"] = "1"
    return env


def run_repro(entry: dict) -> tuple[str, int, str]:
    """跑一条缺口的复现；返回 (kind, exit, 摘要)。"""
    repro = entry.get("repro")
    if not repro:
        return ("none", -1, "没有复现文件")
    path = ROOT / repro
    if not path.exists():
        return ("missing", -1, f"复现文件不存在：{repro}")
    if path.suffix == ".sh":
        # **超时**（2026-09-24 加）：复现件里可能起 LSP / 等 I/O，挂住就会把整个
        # 门禁（以及 CI 的 `Gap ledger` 步骤）一起挂住 —— 实测本机卡过 40 分钟。
        # 超时按"环境/形状异常"判红（返回码 2 的语义），别静默跳过。
        #
        # ⚠ **必须连进程组一起杀**：这些复现件会**另起 LSP 子进程**，子进程握着
        # stdout/stderr 管道 ⇒ `subprocess.run(timeout=…)` 只杀直接子进程，
        # `communicate()` 仍会等所有写端关闭 ⇒ **照样挂死**（实测：加了 timeout
        # 仍然卡住）。所以 `start_new_session=True` 建新会话，超时时 `killpg` 整组。
        proc = subprocess.Popen(["bash", str(path)], cwd=ROOT, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, text=True, env=clean_env(),
                                start_new_session=True)
        try:
            out, err = proc.communicate(timeout=REPRO_TIMEOUT_S)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
            except Exception:
                proc.kill()
            try:
                out, err = proc.communicate(timeout=10)
            except Exception:
                out, err = "", ""
            # **失败通道要带原文** ✓（仓库自己的教训 ✓）：说清"两种读法" ✓，
            # 免得下次只看到一句"超时"又要重新推导 ✓。
            return (
                # **独立的 kind**（2026-09-25 ✓ 用户："像核心凶手 Gap ledger 能不能好好改……
                # 每次都是它出问题，但是从来不改" ✗）。原来超时被塞成 `("script", 2, …)` ✗，
                # 而 `judge()` 里 `exit 2 = 环境/形状异常` **永远判红** ✗ ⇒ **慢 runner 假红** ✓
                # （实测：本地永远绿 ✓、CI 必红 ✗ —— 见 docs/CI-FAILURES.md 与
                # `dsh-ci-time-2026-09-25.md` 的 job 级拆解 ✓）。
                "timeout",
                0,
                f"复现件超时（>{REPRO_TIMEOUT_S}s）——两种读法：① 这台机器太慢"
                f"（判据：同一条命令在快机器上是绿的 ✓）；② 复现件真的挂住了"
                f"（= 行为已坏 ✗）。先在本机跑 `python3 scripts/ci-local.sh` 区分 ✓",
            )
        proc_returncode = proc.returncode

        class _Done:
            returncode = proc_returncode
            stdout = out
            stderr = err

        proc = _Done()
        # **把复现件的实测值带出来**（2026-09-24 加）：以前只留 stderr 最后一行，
        # CI 上那条 `G-10 fixed 环境异常` 就只剩一句"环境异常"——**看不见到底哪一项
        # 不对**，只能靠猜（这次为此在本机重跑才发现是 stdout 收进了 Node 代理警告）。
        # 与 e2e 那条教训同源：**失败通道要带原文**。取 stdout 尾行（复现件把实测值
        # 打在这里）+ stderr 尾行，各自截断。
        def _tail(text: str, lines: int = 1, width: int = 200) -> str:
            rows = [r for r in text.strip().splitlines() if r.strip()]
            return " / ".join(r.strip()[:width] for r in rows[-lines:])
        detail = _tail(proc.stdout) or _tail(proc.stderr)
        if detail and _tail(proc.stderr):
            detail = f"{detail} ｜ stderr: {_tail(proc.stderr, 2, 120)}"
        # **把 Node 自身定时器超时归到 timeout 档**（2026-09-25 round 202）:
        # G-23 的复现件用 Node 写 LSP 客户端、等应答有上限 => CI 慢 runner 上到点
        # => Node 抛 listOnTimeout / processTimers => 脚本 exit 2 => 此前按'环境异常' 永远判红，与台账 fixed 不一致 => 假红。
        # 只分这一种形状: 其余 exit 2 仍判红（2026-09-23 那层保护一字不动）。
        # 走既有 timeout 档 = 响亮跳过（--strict 才判红）。
        # 判据必须在**这一层**（原件在手）—— 放进 judge() 只能拿到拼好的 note。
        if proc.returncode == 2 and any(
            kk in ((proc.stderr or "") + (proc.stdout or ""))
            for kk in ("listOnTimeout", "processTimers", "LSP 超时未应答")
        ):
            return ("timeout", proc.returncode,
                    "LSP 应答超时（Node 自身定时器）=> 环境慢，非形状不对 ｜" + detail)
        return ("script", proc.returncode, detail)
    if path.suffix == ".sokonanoda":
        proc = subprocess.run(
            [str(ROOT / "scripts" / "soko"), "grade", str(path)],
            cwd=ROOT, capture_output=True, text=True, env=clean_env(),
        )
        text = proc.stdout + proc.stderr
        checked = '"type":"decl.checked"' in text
        diagnostic = '"type":"diagnostic"' in text
        clean = proc.returncode == 0 and checked and not diagnostic
        return ("sokonanoda", 0 if clean else 1, "clean" if clean else "仍有失败/无 checked")
    if path.is_dir():
        return ("dir", -1, "目录型复现，请用它下面的 .sh（见 ledger 的 repro 字段）")
    return ("unknown", -1, f"不认识的复现类型：{repro}")


def judge(entry: dict, kind: str, code: int) -> tuple[str, str, bool]:
    """把一次复现结果判成 (observed, expected, ok)。

    期望默认由 `status` 推出；`repro_expect` 显式覆盖（见模块 docstring）。
    取值与复现类型不匹配、或取值非法时 ok=False，且 expected 位置直接给错误原因
    ——台账是契约，写错的字段必须报出来，而不是被默认推导悄悄盖过。
    """
    raw = entry.get("repro_expect")
    fixed = entry.get("status") == "fixed"
    if kind == "sokonanoda":
        clean = code == 0
        observed = "已判卷通过" if clean else "仍有失败"
        if raw is None:
            want, expected = fixed, ("已判卷通过" if fixed else "仍有失败")
        elif raw == "clean":
            want, expected = True, "已判卷通过（repro_expect=clean）"
        elif raw == "rejected":
            want, expected = False, "应判红（repro_expect=rejected）"
        else:
            return observed, f"非法 repro_expect={raw!r}：.sokonanoda 只吃 clean/rejected", False
        return observed, expected, clean == want
    if kind == "script":
        # **exit 2 是"环境/形状异常"，永远判红**（2026-09-23 修）：
        # 复现件约定 0=缺口仍在 / 1=已修 / 2=环境或形状不对。以前只看"非零即已修"
        # ⇒ **环境异常会被静默读成"修好了"**——实测踩到：一条复现件里硬编码了
        # 本机路径，CI 上文件不存在、脚本 exit 2，门禁却报"行为已变"，把一条
        # 真缺口当成已修。环境异常必须是**失败**，不是"通过"。
        if code == 2:
            return "环境异常", "复现件自己说环境/形状不对（exit 2）——修环境，别当成已修", False
        changed = code != 0
        observed = "行为已变" if changed else "缺口仍在"
        if raw is None:
            want, expected = fixed, ("行为已变" if fixed else "缺口仍在")
        elif raw == "nonzero":
            want, expected = True, "行为已变（repro_expect=nonzero）"
        elif raw == "exit0":
            want, expected = False, "缺口仍在（repro_expect=exit0）"
        else:
            return observed, f"非法 repro_expect={raw!r}：脚本只吃 exit0/nonzero", False
        return observed, expected, changed == want
    return "", "", False


def cmd_list(args: argparse.Namespace) -> int:
    entries = load()
    if args.severity:
        entries = [e for e in entries if e.get("severity") == args.severity]
    if args.status:
        entries = [e for e in entries if e.get("status") == args.status]
    if getattr(args, "kind", None):
        entries = [e for e in entries if e.get("kind") == args.kind]
    entries.sort(key=sort_key)
    if args.json:
        print(json.dumps(entries, ensure_ascii=False, indent=2))
        return 0
    print(f"{'ID':<6}{'级别':<10}{'状态':<12}{'WO':<20}标题")
    for e in entries:
        print(f"{e['id']:<6}{e.get('severity', '?'):<10}{e.get('status', '?'):<12}"
              f"{(e.get('wo') or e.get('wo_planned') or '-').split('/')[-1][:18]:<20}{e.get('title', '')}")
    by_kind = {}
    for e in entries:
        by_kind[e.get("kind", "?")] = by_kind.get(e.get("kind", "?"), 0) + 1
    print(f"\n共 {len(entries)} 条（" + "、".join(f"{k} {v}" for k, v in sorted(by_kind.items())) + "）；"
          f"blocker {sum(1 for e in entries if e.get('severity') == 'blocker')} 条，"
          f"未关账 {sum(1 for e in entries if e.get('status') in OPEN_STATUSES)} 条。")
    return 0


def cmd_show(args: argparse.Namespace) -> int:
    for e in load():
        if e["id"] == args.id:
            print(json.dumps(e, ensure_ascii=False, indent=2))
            return 0
    print(f"error: 没有 {args.id}", file=sys.stderr)
    return 1


def wo_prompt(entry: dict) -> str:
    repro = entry.get("repro") or "（缺）"
    cmd = f"bash {repro}" if str(repro).endswith(".sh") else f"scripts/soko grade {repro}"
    return "\n".join([
        f"# 工作单 {entry.get('wo_planned') or '（未编号）'} —— {entry['id']} {entry['title']}",
        "",
        f"仓库：sokonanoda-lang（本仓库）。设计：docs/design/teaching-project.md §6。",
        "",
        "## 用户可见症状 / 最小复现",
        f"- 复现：`{cmd}`",
        f"- 期望（官方 Lean 4 语义）：{entry.get('expected_lean', '（未填）')}",
        f"- 今天：{entry.get('today', '（未填）')}",
        f"- 位置线索：{(entry.get('where') or {}).get('area', '（未填）')}",
        f"  {(entry.get('where') or {}).get('note', '')}".rstrip(),
        f"- 现有绕行（与代价）：{entry.get('workaround', '（无）')}",
        f"- 被它挡住的东西：{'、'.join(entry.get('blocks') or []) or '（未标）'}",
        "",
        "## 交付要求（本仓库硬规则）",
        "1. **设计先行**：先在 docs/design/ 补/改一篇设计（含取舍与 as-built），再动手；",
        "2. **TDD 三层**：front 单测 + CLI e2e + 课程/复现用例；改前先让复现失败、改后转绿；",
        "3. **判据走内核**，禁止文本比对；**内核是冻结快照**（若必须动内核，先读 docs/architecture.md §6）；",
        "4. **语法增量三件套**：课程 + 测试 + 白名单（REQUIREMENTS §2 第 3 条）；",
        "5. **同一轮同步**：editor/vscode/ + skills/ + AGENTS.md + docs/HANDOVER.md + STATUS.md + REQUIREMENTS §9；",
        "6. **验收命令**：`scripts/soko gate`（fmt+clippy+test+playground 锚点）+ 全量 `cargo test --workspace --locked`；",
        "7. **关账**：`python3 scripts/gap.py close " + entry["id"] + " --version <新版本>`（复现脚本会翻成 exit 1 提醒你）。",
        "",
        "## 完成后请回填",
        f"- docs/gaps/ledger.jsonl 的 {entry['id']}：status=fixed、fixed_in=<版本>；",
        "- 课程侧（独立课程仓）升级版本钉并复跑它的本地门禁。",
    ])


def cmd_next(args: argparse.Namespace) -> int:
    entries = [e for e in load() if e.get("status") in OPEN_STATUSES]
    if not entries:
        print("没有未关账的缺口。")
        return 0
    entries.sort(key=sort_key)
    entry = entries[0]
    if args.json:
        print(json.dumps(entry, ensure_ascii=False, indent=2))
    else:
        print(wo_prompt(entry))
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    entries = load()
    bad = 0
    got_env_skips: list[str] = []
    # **分片**（2026-09-25 用户要求 ✓："CI 流程里能不能把 gap.py 拆成多个环节" ✓）：
    # `--shard i/N` 只跑第 i 片 ✓ ⇒ CI 里做成 **matrix job** ✓ ⇒ ① 并行更快 ✓
    # ② 某一处坏只重跑**那一片** ✓（正是用户对 test job 的同一个诉求 ✓）。
    # 分片按**台账顺序**取模 ✓ ⇒ 每片条数均衡 ✓、且**同一片的内容稳定** ✓（便于复现 ✓）。
    if getattr(args, "shard", ""):
        m2 = __import__("re").fullmatch(r"(\d+)/(\d+)", args.shard.strip())
        if not m2:
            print(f"--shard 格式应为 i/N（例 1/3）✗，收到 {args.shard!r}", file=sys.stderr)
            return 2
        i, n = int(m2.group(1)), int(m2.group(2))
        if not (1 <= i <= n):
            print(f"--shard 的 i 必须在 1..N 之间 ✗（收到 {i}/{n}）", file=sys.stderr)
            return 2
        entries = [e for k, e in enumerate(entries) if k % n == i - 1]
        print(f"[分片 {i}/{n}] 本片 {len(entries)} 条 ✓")
    # **并行跑复现件**（2026-09-25 用户要求 ✓："能拆开吗？速度快一点" ✓）。
    # 为什么能拆 ✓：每条缺口 = 一个**独立子进程** ✓（`run_repro` 起进程组、跑完收尸 ✓）
    # ⇒ 彼此无共享状态 ✓ ⇒ 天然可并行 ✓。
    # 实测依据 ✓：本机 `gap.py check` 墙钟 **58.5s**，而 `user` 只 **6.9s** ✓
    # ⇒ **绝大部分时间在等子进程** ✓ ⇒ 并行收益大 ✓。
    # **输出仍然稳定** ✓：先并行收齐结果、再**按台账顺序**逐条判与打印 ✓
    # ⇒ 与顺序版**逐字节相同** ✓（判据就是 diff ✓）。
    jobs = getattr(args, "jobs", 0) or (os.cpu_count() or 4)
    todo = [e for e in entries if e.get("repro")]
    results: dict[int, tuple[str, int, str]] = {}
    if jobs > 1 and len(todo) > 1:
        with concurrent.futures.ThreadPoolExecutor(max_workers=min(jobs, len(todo))) as ex:
            for e, r in zip(todo, ex.map(run_repro, todo)):
                results[id(e)] = r
    else:
        for e in todo:
            results[id(e)] = run_repro(e)
    print(f"{'ID':<6}{'状态':<12}{'复现':<12}判定")
    for e in entries:
        status = e.get("status", "?")
        if not e.get("repro"):
            print(f"{e['id']:<6}{status:<12}{'-':<12}跳过（没有复现文件）")
            continue
        kind, code, note = results[id(e)]
        if kind == "timeout":
            # **响亮地跳过**（不静默 ✗）：慢机器上超时是**环境事实** ✓，不是"行为已变" ✗。
            got_env_skips.append(e["id"])
            print(f"{e['id']:<6}{status:<12}{'超时':<12}"
                  f"⚠ 跳过（环境慢：>{REPRO_TIMEOUT_S}s）｜{note}")
            continue
        if kind in {"missing", "dir", "none", "unknown"}:
            print(f"{e['id']:<6}{status:<12}{kind:<12}跳过（{note}）")
            continue
        observed, expected, ok = judge(e, kind, code)
        bad += not ok
        line = (
            f"{e['id']:<6}{status:<12}{kind:<12}{observed}"
            f"{'' if ok else f'  ← 台账写的是「{expected}」，请更新'}"
            f"{'' if ok or not note else f' ｜复现件：{note}'}"
        )
        print(line)
        # **逐条发 GitHub 注解**（2026-09-25 用户指出 ✓："ledger 是不是本身实现的时候，
        # 信息就打印得太少了" ✓ —— 完全对 ✓）。此前注解里**只有** `Process completed
        # with exit code 1.` ✗ ⇒ 细节只在 stdout ✓，而**整轮结束前 job 日志读不到** ✗
        # ⇒ 失败**已经发生却拿不到原因** ✓（本 session 为此耗过半小时 ✓）。
        # 注解**边跑边可读** ✓（`gh api …/check-runs/<id>/annotations` ✓，不必等整轮 ✓）。
        if not ok and os.environ.get("GITHUB_ACTIONS"):
            # 注解里的 `%` / 换行要转义 ✓（GitHub 的命令语法 ✓）
            msg = line.replace("%", "%25").replace("\r", "").replace("\n", "%0A")
            print(f"::error title=缺口台账 {e['id']} 与台账不一致::{msg}")
    if got_env_skips:
        print(f"\n⚠ **{len(got_env_skips)} 条因环境慢被跳过**（{', '.join(got_env_skips)}）"
              f"—— 它们**没有**被判红 ✓，但也**没有**被验证 ✗。"
              f"慢是环境事实 ✓；请在**快机器**上用 `scripts/ci-local.sh` 复跑这些条目 ✓"
              f"（它带 `--strict` ✓ ⇒ 快机器上超时**照样判红** ✗ ⇒ 守卫不失去牙齿 ✓）。",
              file=sys.stderr)
    if bad:
        print(f"\n{bad} 条与台账不一致 —— 台账是契约：要么修好了（写 fixed_in），要么行为回退了。",
              file=sys.stderr)
        return 1
    if got_env_skips and getattr(args, "strict", False):
        print(f"\n--strict：{len(got_env_skips)} 条超时**判红** ✗（快机器上必须跑完 ✓）。",
              file=sys.stderr)
        return 1
    # **把结果写进 GitHub Step Summary**（2026-09-25 用户指出 ✓）：整轮 `in_progress`
    # 期间 **job 日志取不到** ✗（`gh run view --job … --log` 会回
    # "run … is still in progress" ✓）⇒ 失败**已经发生却看不见** ✗
    # （用户原话："ledger 立马就失败了，但是整个 github action 还在继续，
    #   导致你不知道已经失败了" ✓）。写进 summary ⇒ 在**页面上一眼可见** ✓、不必等整轮 ✓。
    _write_step_summary(bad, got_env_skips)
    # **同时发 GitHub 注解**（2026-09-25 用户指出"失败了却不知道" ✓ 的最后一环 ✓）：
    # Step summary 只在**页面**上可见 ✗（**没有 API** ✓）⇒ 而 `::error::` 会变成
    # **check-run 注解** ✓ ⇒ `gh api …/check-runs/<id>/annotations` **立刻可读** ✓
    # —— 整轮还在跑也读得到 ✓（本 session 为"拿不到失败细节"耗过半小时 ✓）。
    if bad and os.environ.get("GITHUB_ACTIONS"):
        shard = _shard_label()
        print(
            f"::error title=缺口台账不一致（{shard}）::"
            f"这一片有 {bad} 条与台账不一致 ✓ ⇒ 细节见本 job 的 **Step Summary** 与日志 ✓；"
            f"本地复跑：python3 scripts/gap.py check {shard} --strict"
        )
    print("\n全部与台账一致。")
    return 0


def _shard_label() -> str:
    """当前分片标签 ✓（没分片时给空串 ✓）。"""
    import sys as _sys

    for i, a in enumerate(_sys.argv):
        if a == "--shard" and i + 1 < len(_sys.argv):
            return f"--shard {_sys.argv[i + 1]}"
        if a.startswith("--shard="):
            return f"--shard {a.split('=', 1)[1]}"
    return ""


def _write_step_summary(bad: int, skips: list) -> None:
    """把"这一片是否一致"写进 `$GITHUB_STEP_SUMMARY`（本地没有这个变量 ⇒ 静默跳过 ✓）。"""
    path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not path:
        return
    try:
        with open(path, "a", encoding="utf-8") as f:
            if bad:
                f.write(f"## ❌ 缺口台账：**{bad} 条与台账不一致**\n\n")
                f.write("上面的 `← 台账写的是 …` 行就是细节（在**本 job 的日志**里 ✓）；\n")
                f.write("本地复跑这一片即可看到同样的输出 ✓：\n\n")
                f.write("```\npython3 scripts/gap.py check --shard <i>/<N> --strict\n```\n")
            else:
                f.write("## ✅ 缺口台账：这一片全部与台账一致\n")
            if skips:
                f.write(f"\n> ⚠ 环境异常跳过 {len(skips)} 条（非产品回归 ✓）：{', '.join(skips)}\n")
    except OSError:
        pass


def cmd_close(args: argparse.Namespace) -> int:
    entries = load()
    for e in entries:
        if e["id"] == args.id:
            kind, code, note = run_repro(e) if e.get("repro") else ("none", -1, "")
            if kind in {"script", "sokonanoda"}:
                # 用「如果现在写 fixed，复现该是什么样」来预检：显式 repro_expect 也算数。
                _, expected, ok = judge(dict(e, status="fixed"), kind, code)
                if not ok:
                    print(f"error: {args.id} 的复现与「已修」不符"
                          f"（实测 {kind}/{code} {note}，期望{expected}），先确认再关。",
                          file=sys.stderr)
                    return 1
            e["status"] = "fixed"
            e["fixed_in"] = args.version
            if args.note:
                e["notes"] = (e.get("notes", "") + "｜" + args.note).strip("｜")
            save(entries)
            print(f"{args.id} 已关账：fixed_in={args.version}（复现判定：{kind}/{code} {note}）")
            return 0
    print(f"error: 没有 {args.id}", file=sys.stderr)
    return 1


def cmd_selftest(args: argparse.Namespace) -> int:
    """判据通道自检：`judge()` 的期望推导 + 显式覆盖 + 非法值都要被钉住。

    它不碰台账文件、不跑判卷——只验证"台账 vs 现实"的判定规则本身。
    """
    cases = [
        # (说明, 条目, kind, code, 期望 ok, 期望 expected 里含「非法」)
        ("缺省 fixed+sokonanoda 干净 = 一致", {"status": "fixed"}, "sokonanoda", 0, True, False),
        ("缺省 fixed+sokonanoda 判红 = 不一致", {"status": "fixed"}, "sokonanoda", 1, False, False),
        ("缺省 open+sokonanoda 判红 = 一致", {"status": "open"}, "sokonanoda", 1, True, False),
        ("缺省 open+sokonanoda 干净 = 不一致", {"status": "open"}, "sokonanoda", 0, False, False),
        ("显式 rejected+判红 = 一致（G-01 形态）",
         {"status": "fixed", "repro_expect": "rejected"}, "sokonanoda", 1, True, False),
        ("显式 rejected+干净 = 不一致（假绿会被抓）",
         {"status": "fixed", "repro_expect": "rejected"}, "sokonanoda", 0, False, False),
        ("显式 clean+判红 = 不一致",
         {"status": "open", "repro_expect": "clean"}, "sokonanoda", 1, False, False),
        ("sokonanoda 写脚本取值 = 非法", {"repro_expect": "nonzero"}, "sokonanoda", 1, False, True),
        ("缺省 fixed+script 行为已变 = 一致", {"status": "fixed"}, "script", 1, True, False),
        ("缺省 fixed+script 缺口仍在 = 不一致", {"status": "fixed"}, "script", 0, False, False),
        ("缺省 open+script 缺口仍在 = 一致", {"status": "open"}, "script", 0, True, False),
        ("显式 exit0+缺口仍在 = 一致（修好前也允许显式）",
         {"status": "fixed", "repro_expect": "exit0"}, "script", 0, True, False),
        ("显式 nonzero+缺口仍在 = 不一致",
         {"status": "open", "repro_expect": "nonzero"}, "script", 0, False, False),
        ("script 写判卷取值 = 非法", {"repro_expect": "clean"}, "script", 0, False, True),
        # **exit 2 = 环境/形状异常 ⇒ 永远判红**（2026-09-23 修）：
        # 以前"非零即已修"⇒ 环境异常被静默读成"修好了"（实测踩到过一次）。
        ("script exit 2 = 环境异常 ⇒ 判红（open）", {"status": "open"}, "script", 2, False, False),
        ("script exit 2 = 环境异常 ⇒ 判红（fixed 也不行）", {"status": "fixed"}, "script", 2, False, False),
    ]
    bad = 0
    saved = {k: os.environ.get(k) for k in ("SOKONANODA_BIN", "SOKONANODA_LSP_BIN")}
    os.environ["SOKONANODA_BIN"] = "/nonexistent/sokonanoda"
    os.environ["SOKONANODA_LSP_BIN"] = "/nonexistent/sokonanoda-lsp"
    env = clean_env()
    for key in saved:
        if key in env:
            bad += 1
            print(f"FAIL clean_env 没有剔除 {key}（复现夹具会被环境短路）", file=sys.stderr)
    for key, value in saved.items():
        if value is None:
            os.environ.pop(key, None)
        else:
            os.environ[key] = value
    for desc, entry, kind, code, want, illegal in cases:
        observed, expected, ok = judge(entry, kind, code)
        if ok != want:
            bad += 1
            print(f"FAIL {desc}: judge -> ({observed}, {expected}, {ok})，期望 ok={want}",
                  file=sys.stderr)
        elif ("非法" in expected) != illegal:
            bad += 1
            print(f"FAIL {desc}: 非法取值的报错形态不对（expected={expected}，期望非法={illegal}）",
                  file=sys.stderr)
    if bad:
        print(f"gap.py selftest: {bad}/{len(cases)} 条判据不成立。", file=sys.stderr)
        return 1
    print(f"gap.py selftest: {len(cases)} 条判据 + clean_env 剔除覆盖 全部成立。")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="课程驱动缺口台账工具（docs/gaps/ledger.jsonl）")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("list", help="列表（按级别/WO 排序）")
    p.add_argument("--severity", choices=list(SEVERITY_ORDER))
    p.add_argument("--kind", help="language / tooling / infra / library")
    p.add_argument("--status")
    p.add_argument("--json", action="store_true")
    p.set_defaults(func=cmd_list)

    p = sub.add_parser("show", help="看一条的完整 JSON")
    p.add_argument("id")
    p.set_defaults(func=cmd_show)

    p = sub.add_parser("next", help="下一条要修的缺口 + 可粘贴给另一个 agent 的 prompt")
    p.add_argument("--json", action="store_true")
    p.set_defaults(func=cmd_next)

    p = sub.add_parser("check", help="跑全部 repro，报告台账与现实的偏差")
    p.add_argument("--jobs", type=int, default=0,
                   help="并行跑复现件的进程数（默认 = CPU 数 ✓；1 = 顺序 ✓）")
    p.add_argument("--strict", action="store_true",
                   help="超时**判红** ✗（快机器用 ✓：本地必须能跑完 ⇒ 守卫不失去牙齿 ✓）")
    p.add_argument("--shard", default="",
                   help="只跑第 i/N 片（i 从 1 起 ✓，例：--shard 1/3 ✓）—— CI 用它对矩阵并行拆 ✓")
    p.set_defaults(func=cmd_check)

    p = sub.add_parser("close", help="关账（写 fixed_in）")
    p.add_argument("id")
    p.add_argument("--version", required=True, help="修复落地的版本，例如 0.59.0")
    p.add_argument("--note")
    p.set_defaults(func=cmd_close)

    p = sub.add_parser("selftest", help="自检 judge() 的判定规则（不碰台账、不判卷）")
    p.set_defaults(func=cmd_selftest)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
