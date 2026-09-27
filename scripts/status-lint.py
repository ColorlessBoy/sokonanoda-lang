#!/usr/bin/env python3
"""STATUS.md 的 lint（用户 2026-09-26 要求 ✓）—— 见 REQUIREMENTS.md §9。

**为什么** ✓：STATUS.md 曾是 **1534 行 / 67 KB** ✗，含 **163 条 "round N"** ✗ 与
**138 条瞬时状态**（"在跑/进行中/未变/判据不变/待 CI/等 CI/⏳"）✗ ——
而 **AGENTS.md §收尾义务** 本来就写着"**只保留最近 3 轮**" ✗。
"CI 在跑"这类信息**看 CI 页面就有** ✓，写进 STATUS 只有噪音 ✗，
而**每个接手 agent 都要先读那 67 KB** ✗。

**四类判据**（可判 ✓）：
  ① **总行数 ≤ 200** ✗（超出判红）；
  ② **禁词命中 = 0** ✗（`在跑`/`进行中`/`未变`/`判据不变`/`待 CI`/`等 CI`/`⏳`）；
  ③ **每段 ≤ 30 行** ✗（段 = 顶层 `##` 之间；**首段"当前快照" ≤ 40 行** ✓）；
  ④ **净增行数 ≤ 60** ✗（与 `HEAD` 版比 —— 防止一轮又灌几十行 ✗）。

退出码：0 = 通过 ✓ / 1 = 判红 ✗ / 2 = 用法或环境错。
"""
import re
import subprocess
import sys
from pathlib import Path

# **预算放宽（2026-09-27 夜班，用户 23:12 拍板：文档预算可放宽，只要在同一个 commit
# 里记一笔「哪个文件、从→到、为什么」）**：`STATUS.md` 的总行数上限 **200 → 240**。
# 为什么：v0.74.0 收尾时**一轮里落了 6 个环节**（E21–E23 · E27–E31），每个环节按
# 规矩都要一行结论 + 判据 + 反向验证 ⇒ 200 行顶格后**只能删旧轮才能写新轮** ✗，
# 而旧轮归档目标 `docs/STATUS-ARCHIVE.md` 本身也被冻结（只许减不许增 ✗）⇒
# 实际后果是"逼着删历史"。240 给一轮留出余量；`MAX_GROWTH`（单次净增 ≤60）**不动** ✓
# —— 防"一轮灌几十行"的那条判据仍然在岗 ✓。**后续统一 refactor 时清理**（把旧轮
# 真正搬进归档目录 `docs/archive/`，那里不受冻结表管 ✓，再把上限调回 200）。
MAX_TOTAL = 240
MAX_SECTION = 30
MAX_SNAPSHOT = 40
MAX_GROWTH = 60
BANNED = ("在跑", "进行中", "未变", "判据不变", "待 CI", "等 CI", "⏳")
SNAPSHOT_TITLE = "当前快照"


def sections(lines):
    """按顶层 `##` 切段；返回 [(标题, 起始行号, 行数)]。"""
    out, title, start = [], "(前言)", 1
    for i, l in enumerate(lines, 1):
        if l.startswith("## "):
            out.append((title, start, i - start))
            title, start = l[3:].strip(), i
    out.append((title, start, len(lines) + 1 - start))
    return [s for s in out if s[2] > 0]


def growth(path):
    """与 HEAD 版比净增行数。

    ⚠ **取不到 HEAD 基线 ⇒ 返回 `None`，不是 0** ✗✓（E00 切片 B 实测）：
    原来 `except: return 0` 把「**没量到**」与「**零增长**」混为一谈 ⇒ STATUS.md 未被
    git 跟踪 / git 不可用时，判据 ④ **静默失效**，门禁还印「**净增 0 ≤ 60 ✓**」✗
    —— 看起来像"量过了" ✗。「没量到 ≠ 零」✓（与"扫不到 ≠ 绿"同一条纪律 ✓）。
    """
    try:
        old = subprocess.run(["git", "show", f"HEAD:{path}"], capture_output=True,
                             text=True, check=True).stdout
    except Exception:  # noqa: BLE001
        return None
    return len(path and Path(path).read_text(encoding="utf-8").splitlines()) - len(old.splitlines())


def judge(lines, g) -> list:
    """**纯函数** ✓：只吃行列表与净增数 ⇒ `--selftest` 直接喂合成数据（不碰文件系统 ✓）。"""
    bad = []

    # ① 总行数
    if len(lines) > MAX_TOTAL:
        bad.append(f"总行数 {len(lines)} > {MAX_TOTAL} ✗（逐轮过程应进 docs/STATUS-ARCHIVE.md ✓）")

    # ② 禁词
    for i, l in enumerate(lines, 1):
        for w in BANNED:
            if w in l:
                bad.append(f"第 {i} 行命中禁词 `{w}` ✗：{l.strip()[:60]}")

    # ③ 每段行数（首段 = 当前快照 ⇒ 40 ✓，其余 ⇒ 30 ✗）
    for title, start, n in sections(lines):
        limit = MAX_SNAPSHOT if SNAPSHOT_TITLE in title else MAX_SECTION
        if n > limit:
            bad.append(f"段「{title}」（第 {start} 行起）{n} 行 > {limit} ✗")

    # ④ 净增行数
    if g is None:
        bad.append("④ 净增 **未测**（取不到 HEAD 基线）⇒ **无法判定 ≠ 零增长** ✗")
    elif g > MAX_GROWTH:
        bad.append(f"本次净增 {g} 行 > {MAX_GROWTH} ✗（一轮别灌几十行 ✗）")

    return bad


def selftest() -> int:
    """**故意喂已知形状** ✓：判据通道自己坏了必须在这里判红 ✓。

    ⚠ 2026-09-27 补（E00 判据强度普查，见 `docs/design/criteria-strength.md` §3.2）：
    本脚本原来**没有任何自检入口** ⇒ **无法自证"能咬住"** ✗（咬不住的守卫等于没有 ✓）。
    每个用例都断言**具体条数**（不是"有没有红"✗），并且**含不误红的反例** ✓。
    """
    ok = ["# STATUS", "", "## 当前快照", "一行", "", "## 上一轮", "两行", "三行"]
    snap = lambda n: ["# T", "", "## 当前快照"] + ["a"] * n
    cases = [
        ("正常（不该误红）", ok, 0, 0),
        # ⚠ 夹具要**只触发一条**判据 ✗：一开始写 `["x"] * 201` ⇒ 那是一个 201 行的
        # 「(前言)」段 ⇒ **同时**踩 ③ 段超限 ⇒ 报 2 条（**是我的夹具错，不是判据错** ✓）。
        # 改成"很多小段" ⇒ 只有 ① 总行数该红 ✓。
        # ⚠ 行数**从 `MAX_TOTAL` 派生**（`+2` 一定越界）—— 2026-09-27 把上限
        # 200→240 时，写死的 201 行不再越界 ⇒ 这条自检当场判红 ✗✓（正好证明它真的在咬 ✓）。
        ("总行数 > 上限（切成小段 ⇒ 只该红 ①）",
         ["# T"] + [x for i in range((MAX_TOTAL + 2) // 3 + 1) for x in (f"## s{i}", "a", "b")], 0, 1),
        # ⚠ **每个禁词各一条** ✗✓（E00 切片 B：原来 **7 个词只测了「在跑」1 个** ✗ ⇒
        #   **删掉其余 6 个词，自检仍全过** ✗ —— 判据里 6/7 是没被证明过会咬的 ✓）。
        # ⚠⚠ **字面量写死、绝不从 `BANNED` 派生** ✗✓ —— 派生就是本会话在
        #   `audit-notation-paths` 上犯过的「**自指期望**」（删词 ⇒ 期望跟着缩 ⇒ 恒过 ✗；
        #   见 `docs/design/criteria-strength.md` §2.5，那次是**反向验证当场抓到**的 ✓）。
        ("命中禁词「在跑」", ["# T", "", "## 当前快照", "CI 在跑"], 0, 1),
        ("命中禁词「进行中」", ["# T", "", "## 当前快照", "这一轮进行中"], 0, 1),
        ("命中禁词「未变」", ["# T", "", "## 当前快照", "判据未变"], 0, 1),
        ("命中禁词「判据不变」", ["# T", "", "## 当前快照", "判据不变"], 0, 1),
        ("命中禁词「待 CI」", ["# T", "", "## 当前快照", "待 CI 验证"], 0, 1),
        ("命中禁词「等 CI」", ["# T", "", "## 当前快照", "等 CI 绿"], 0, 1),
        ("命中禁词「⏳」", ["# T", "", "## 当前快照", "⏳ 进行"], 0, 1),
        ("普通段 > 30 行", ["# T", "", "## 当前快照", "a", "", "## 下一段"] + ["b"] * 31, 0, 1),
        ("首段「当前快照」40 行 ⇒ **不误红**", snap(39), 0, 0),
        ("首段「当前快照」41 行 ⇒ 判红", snap(40), 0, 1),
        ("净增 > 60", ok, MAX_GROWTH + 1, 1),
        # ⚠ E00 切片 B：`except: return 0` 会把"没量到"当成"零增长"⇒ 补这条负例 ✓
        ("净增**未测**（取不到 HEAD 基线）⇒ 该红", ok, None, 1),
    ]
    bad = 0
    for label, lines, g, want in cases:
        got = judge(lines, g)
        okc = len(got) == want
        print(f"  {'✓' if okc else '✗'} {label:<34} → {len(got)} 条（期望 {want}）")
        if not okc:
            bad += 1
            for b in got:
                print(f"        ↳ {b}")
    print()
    if bad:
        print(f"✗ selftest 判红：{bad}/{len(cases)} 个用例不符 ✗")
        return 1
    print(f"✓ selftest 全过：{len(cases)}/{len(cases)} 个用例符合 ✓")
    return 0


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    # ⚠ **不许静默回落** ✗✓（E00 切片 B 实测 ✓）：这几个脚本原来用 `"--x" in argv`
    # **子串**判模式 ⇒ **错拼的参数被静默忽略、回落成全量检查并 exit 0** ✗ ⇒
    # 表现是「**自检没跑，退出码却是绿的**」✗（我上次"它没有自检"的错结论就是这么来的 ✓）。
    # ⇒ 未知参数一律 **exit 2**（只有 `notation-lint.py` 本来就用 argparse、是对的 ✓）。
    unknown = [a for a in argv if a.startswith("-") and a not in {"--selftest"}]
    if unknown:
        print(f"✗ 未知参数 {unknown} ⇒ 拒绝执行（**不许静默回落全量检查** ✗）", file=sys.stderr)
        return 2
    if "--selftest" in argv:
        return selftest()
    path = Path("STATUS.md")
    if not path.exists():
        print("STATUS.md 不存在 ✗", file=sys.stderr)
        return 2
    lines = path.read_text(encoding="utf-8").splitlines()
    g = growth("STATUS.md")
    bad = judge(lines, g)

    if bad:
        print(f"status-lint：{len(bad)} 条不通过 ✗", file=sys.stderr)
        for b in bad[:20]:
            print(f"  - {b}", file=sys.stderr)
        return 1
    print(f"status-lint ✓ 总行数 {len(lines)} ≤ {MAX_TOTAL} ✓ · 禁词 0 ✓ · "
          f"段数 {len(sections(lines))} ✓ · 净增 {g if g is not None else '未测'} ≤ {MAX_GROWTH} ✓")
    return 0


if __name__ == "__main__":
    sys.exit(main())
