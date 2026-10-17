#!/usr/bin/env python3
"""e2e 台账**轮换**（记录 + 日志**一起**搬进 `docs/archive/`）+ **归档索引自动生成**。

## 为什么需要它（2026-10-16 用户拍板「可以轮换，而且把它自动化」✓）

`docs-lint` 判据 ①（活文档 ≤ 上限）被 `docs/e2e/logs/` 撑破 ⇒ **pre-push 判红 ⇒ 所有推送被挡** ✗。
而**每一个**日志都被 `docs/e2e/ledger.jsonl` 的记录 `log` 字段引用（实测孤儿 0 个）
⇒ **单独归档日志会让 `e2e ledger` job 报「记录引用的日志不存在」** ✗（2026-10-16 实测撞过）。

⇒ 唯一正确的形状 = **台账记录与日志一起轮换**（照 `docs/archive/e2e-2026-10-15/` 的先例 `71f017b2`）。
⚠ 而它是一条**跑步机**：每跑一次 e2e 加约 2 个日志（~80 KB）⇒ 手工腾空间**一轮就被吃回去** ✗
⇒ 所以**两半都必须自动** ✓：
  * **轮换**（记录 + 日志一起搬）—— `--apply`；
  * **点名**（`docs-lint` 判据 ⑥：`docs/archive/**` 每个文件都要在
    `docs/archive/README.md` 里被点名）—— 同一趟 `--apply` 里**重新生成**那个区块 ✓
    （第一半落地时是**手写脚本**把 82 个名字塞进一行 ✗ ⇒ 这里把它变成**可重复的一条命令** ✓）。

## 用法

```bash
python3 scripts/rotate-e2e-ledger.py            # 干跑：印将要发生什么（默认 ✓，零写入）
python3 scripts/rotate-e2e-ledger.py --apply    # 真做：轮换 + 重生成归档索引
python3 scripts/rotate-e2e-ledger.py --check    # 判据：活记录的日志都在吗 + 归档索引齐吗（0/1）
python3 scripts/rotate-e2e-ledger.py --selftest # 反向验证：夹具必须被轮换 + 判据 ⑥ 仍 0
```

## 不变量（本脚本必须守住）

1. **活台账里的每一条记录，其 `log` 指向的文件必须存在**（否则 `e2e ledger` job 判红 ✗）；
2. **归档的记录仍写原路径**（`docs/e2e/logs/<名字>.log`）—— 照 2026-10-15 先例，
   到 `docs/archive/<批次>/` 取同名 `.gz` ✓；
3. **只搬「只被归档记录引用」的日志** —— 还被活记录引用的**一个都不许动** ✗；
4. **幂等**：重复跑不改变结果 ✓（索引区块由目录**重新生成**，不是追加 ⇒ 跑两遍逐字节相同 ✓）；
5. **判据 ⑥ 不许被绕过** ✗：`docs/archive/README.md` 里**点名区块的哨兵行缺失** ⇒
   **响亮退出（rc=2）**，**绝不**静默跳过（跳过一次 = 下一次 `docs-lint` ⑥ 判红，
   而那时没人知道是谁把索引弄丢的 ✗ —— 照 `docs-lint` 的「缺表 ⇒ 不假装判过」口径 ✓）；
6. **点名区块恰好一行** ✓（判据 ⑦ 的 L3 口径）：`docs/archive/README.md` = **41 行散文
   + 1 行区块 = 42 行 = `layer_max_lines.L3`** ⇒ 区块多一行就判红 ✗（2026-10-17 实测：
   折行 ⇒ 区块 23 行 ⇒ 真仓 64 行 > 42 ✗）。**不是**抬上限 ✓；
7. **绝不覆盖归档里已有的东西** ✗（**归档≠销毁** ✓）：**批次目录不复用** —— 默认名撞车 ⇒
   自动换名 `-2`/`-3`（无人值守那条路必须继续能跑 ✓）；显式 `--batch` 撞车 ⇒ **响亮拒绝**
   （rc=2，名字是人点名的、机器不许偷偷改 ✗）。2026-10-17 实测洞
   `ROTATION-OVERWRITES-ARCHIVED-RECORDS`：`docs/archive/e2e-2026-10-15/` 已有 **571** 条
   归档记录，新轮换的 **48** 条**覆盖**了它 ⇒ **523 条被销毁** ✗，而**没有任何判据报红** ✗
   （⑥ 只查"文件有没有被点名"，不查"记录少了没有" ⇒ **咬不住** ⇒ 靠本脚本自己的
   `--selftest` 造出撞车来咬 ✓）。

## `--root`（唯一一处）

默认 = 本脚本所在仓库（`Path(__file__).parent.parent`）✓；`--root` 只给 `--selftest` 的
夹具用（**夹具一律在临时目录里**，真仓库零写入 ✓）。
"""
from __future__ import annotations

import argparse
import gzip
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import git_env  # noqa: E402  （同目录库：**干净 git 环境**的唯一出处 ✓）

DEFAULT_KEEP = 60

# **点名区块的哨兵**（`docs-lint` 判据 ⑥ 的机器面）：
# `docs/archive/README.md` 里 `<!-- 归档索引:auto:BEGIN -->` 与 `END` 之间是**生成的** ✓
# ⇒ 区块外的散文（口径、为什么只能这样、那几行指针）一个字都不动 ✗。
INDEX_BEGIN = "<!-- 归档索引:auto:BEGIN -->"
INDEX_END = "<!-- 归档索引:auto:END -->"
# 区块**整体只占一行**（哨兵 + 抬头 + 名单全挤在同一行 ✓）—— 算术见 `render_index`：
# `docs/archive/README.md` = **41 行散文 + 1 行区块 = 42 行** = 判据 ⑦ 的 `L3` 上限
# ⇒ **区块多一行就判红** ✗（2026-10-17 实测：按 800 字符折行 ⇒ 区块 23 行 ⇒ 真仓 64 行 ✗）。
INDEX_HEAD = ("**归档清单（自动生成）** ✓：本区块由 `scripts/rotate-e2e-ledger.py` "
              "**重新生成**（`--apply` / `--index`）⇒ 轮换落地时**不需要**人手追加名字 ✓；"
              "区块外是散文、不许机器改 ✗。判据 ⑥（归档≠销毁）读的就是这份名单 ✓。"
              "⚠ **为什么整块挤在一行** ✗：本文件受判据 ⑦ 的 **L3 层 42 行**上限管，"
              "而它**正好 42 行**（41 行散文 + 本行）⇒ 区块**多一行就判红**"
              "（2026-10-17 实测：折行 ⇒ 区块 23 行 ⇒ 本文件 64 行 ✗）。"
              "**不是**抬上限 ✓（见 `scripts/docs-budget.json`）。")


def _die(msg: str) -> None:
    """**响亮退出（rc=2 = 用法/状态错 ✓）**，绝不静默继续 ✗。

    ⚠ **为什么不是 `raise SystemExit(msg)`**：Python 的 `SystemExit(<字符串>)` 的退出码是
    **1**（2026-10-17 实测）⇒ 与本脚本白纸黑字写的 **rc=2**（不变量 5）**不符** ✗ ——
    照仓库口径「**凡写 rc=X，就核它真的是 X**」⇒ 要么把话改掉、要么把行为改成真的 2；
    这里选后者（`2 = 用法错` 与 `docs-lint` / `check-guard-bite` 同口径 ✓）。
    `--selftest` 直接断言 `e.code == 2` ✓（原来只断言消息里含"不覆盖" ⇒ **咬不住 rc** ✗）。
    """
    print(msg, file=sys.stderr)
    raise SystemExit(2)


class Repo:
    """一个仓库根上的路径集合（`--root` 的唯一解析处 ✓）。"""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.ledger = root / "docs" / "e2e" / "ledger.jsonl"
        self.logs = root / "docs" / "e2e" / "logs"
        self.archive = root / "docs" / "archive"
        # **点名落点 = `docs/archive/README.md`**（判据 ⑥ 只读它的文本 ✓）—— ⚠ 见 `render_index`：
        # 本文件受判据 ⑦ 的 **L3 层 42 行**上限管，而它**正好 42 行**（41 行散文 + 1 行区块 ✓）
        # ⇒ 区块**只许占一行**（折行 / 一名字一行 ⇒ 判红 ✗，2026-10-17 实测过）。
        self.index = self.archive / "README.md"

    # ── 读 ────────────────────────────────────────────────────────────────
    def rows(self) -> list[dict]:
        if not self.ledger.exists():
            return []
        out = []
        for line in self.ledger.read_text(encoding="utf-8").splitlines():
            s = line.strip()
            if s:
                out.append(json.loads(s))
        return out

    def archive_files(self) -> list[str]:
        """`docs/archive/**` 的**文件**名（除索引自己）—— 判据 ⑥ 点名口径 = basename ✓。"""
        if not self.archive.exists():
            return []
        return sorted(p.name for p in self.archive.rglob("*")
                      if p.is_file() and p != self.index)


def log_name(rec: dict) -> str | None:
    lg = rec.get("log")
    if not isinstance(lg, str) or not lg:
        return None
    return Path(lg).name


def missing_logs(repo: Repo, rows: list[dict]) -> list[str]:
    return [n for r in rows if (n := log_name(r)) and not (repo.logs / n).exists()]


def _archived_records(path: Path) -> list[str]:
    """归档的记录 `.jsonl.gz` 逐行（**只读**；夹具用 ✓ —— 不变量 ⑦ 的读数靠它 ✓）。"""
    if not path.exists():
        return []
    with gzip.open(path, "rt", encoding="utf-8") as f:
        return [ln for ln in f.read().splitlines() if ln.strip()]


def missing_index(repo: Repo) -> list[str]:
    """归档里有、而 `docs/archive/README.md` 里**没点名**的文件（判据 ⑥ 的机械面 ✓）。"""
    if not repo.index.exists():
        return repo.archive_files()
    text = repo.index.read_text(encoding="utf-8")
    return [n for n in repo.archive_files() if n not in text]


def render_index(repo: Repo) -> str:
    """**重新生成**点名区块 —— **恰好一行**（幂等：只依赖目录内容 ⇒ 跑两遍逐字节相同 ✓）。

    ⚠ **为什么必须是一行**（2026-10-17 实测 ✗→✓）：`docs/archive/README.md` 受判据 ⑦ 的
    **L3 层 42 行**上限管（`scripts/docs-budget.json` 的 `layer_max_lines.L3`），而它
    **正好 42 行 = 41 行散文 + 1 行区块** ⇒ 区块**每多占一行，L3 就超一行** ✗。
    修前这里把名字按 800 字符折行 ⇒ 区块 **23 行** ⇒ 本文件 **64 行** ⇒ **判据 ⑦ 判红** ✗
    （真仓 324 个名字上的实测读数；同族的「一名字一行」= 340 行 ⇒ 更红 ✗）。
    出路**不是抬上限** ✗（用户 2026-09-26：「不许靠抬上限达标」）⇒ 把**整块**（哨兵 +
    抬头 + 名单）挤进**一行** —— 这**正是**它取代的那一行的形状（实测 `10 501` 字符的一行 ✓）。

    行数算术（**判据 ⑦ 的机械面**）：`README.md` 行数 = 41（散文，区块外、机器不碰 ✓）
    + 1（本区块）= **42 = L3 上限** ⇒ 区块**只许占 1 行**。`--selftest` 把这条不变量接到
    **真的** `docs-lint` ⑦ 上：夹具的 `layer_max_lines.L3` = **散文行数 + 1** ✓（不是留余量 ✗
    —— 2026-10-17 之前那里写死 400，余量正好把多行区块放过去了 ✗）。
    """
    names = repo.archive_files()
    body = ", ".join(f"`{n}`" for n in names) if names else "（空）"
    return f"{INDEX_BEGIN} {INDEX_HEAD} {body} {INDEX_END}\n"


def write_index(repo: Repo) -> bool:
    """把区块写回 `docs/archive/README.md` ⇒ 是否发生了改动。

    哨兵缺失/不配对 ⇒ `SystemExit(2)`（**绝不静默跳过** ✗，见模块 docstring 不变量 5 ✓）。
    """
    text = repo.index.read_text(encoding="utf-8") if repo.index.exists() else ""
    block = render_index(repo)
    if INDEX_BEGIN in text and INDEX_END in text:
        new = re.sub(re.escape(INDEX_BEGIN) + r".*?" + re.escape(INDEX_END) + r"\n?",
                     block, text, count=1, flags=re.S)
    elif INDEX_BEGIN not in text and INDEX_END not in text:
        _die(
            f"error: `{repo.index}` 里找不到点名区块哨兵 `{INDEX_BEGIN}` ⇒ "
            f"**不假装写完** ✗（判据 ⑥ 会判红，而那时没人知道索引是怎么丢的 ✗）"
            f" ⇒ 先把哨兵加回去（见本脚本 `INDEX_BEGIN`）")
    else:
        _die(
            f"error: `{repo.index}` 的哨兵**只出现一个**（BEGIN/END 必须配对）⇒ "
            f"**不假装写完** ✗")
    if new == text:
        return False
    repo.index.write_text(new, encoding="utf-8")
    return True


def cmd_check(repo: Repo) -> int:
    rows = repo.rows()
    missing = missing_logs(repo, rows)
    if missing:
        print(f"✗ 活台账引用的日志不存在：{len(missing)} 个 ✗")
        for m in missing[:5]:
            print(f"   · docs/e2e/logs/{m}")
        return 1
    print(f"✓ 活台账 {len(rows)} 条 · 引用的日志全部存在 ✓")
    unindexed = missing_index(repo)
    if unindexed:
        print(f"✗ 归档里有 {len(unindexed)} 个文件没在 docs/archive/README.md 里点名 "
              f"（判据 ⑥ 会判红 ✗）")
        for n in unindexed[:5]:
            print(f"   · {n}")
        return 1
    print(f"✓ 归档 {len(repo.archive_files())} 个文件**全部**被点名 ✓")
    return 0


def batch_dir(repo: Repo, batch: str, explicit: bool) -> str:
    """批次名 ⇒ **不复用**的批次名（不变量 ⑦ / 洞 `ROTATION-OVERWRITES-ARCHIVED-RECORDS` ✓）。

    ⚠ **为什么**（2026-10-17 实测 ✗）：默认批次名 = `e2e-<最老被归档记录的日期>`，而**上一次**
    轮换的批次可能正好同名 —— 真仓实测：`docs/archive/e2e-2026-10-15/` 里已有 **571** 条归档
    记录，新的 **48** 条**覆盖**了它 ⇒ **523 条被销毁** ✗（而且没有任何判据报红 ✗）。归档层是
    **只许加不许改**的（`docs/archive/README.md`：**归档≠销毁** ✓）⇒
      * **默认**名撞车 ⇒ 自动换名 `-2`/`-3` …（无人值守那条路必须继续能跑 ✓；**响亮印出** ✓）；
      * **显式** `--batch` 撞车 ⇒ **响亮拒绝**（rc=2）—— 名字是人点名的，机器不许偷偷改 ✗。

    ⚠ **横向排查**（同类还有哪些）：本脚本往归档里写**两种**东西 —— 记录 `.jsonl.gz` 与日志
    `.gz` —— 它们**共用**这一把锁（批次目录不复用）⇒ 一次挡住两个 ✓（另加记录文件那处
    兜底断言 ✓）；不这么做就得各写一条"同名不同内容 ⇒ 拒绝"的判据，而那条**永远跑不到** ✗。
    """
    def taken(name: str) -> bool:
        d = repo.archive / name
        return d.exists() and any(d.iterdir())   # 非空 ⇒ 已属于某个批次 ⇒ 不复用 ✗

    if not taken(batch):
        return batch
    if explicit:
        _die(
            f"error: 批次目录 `docs/archive/{batch}/` 已存在且非空 ⇒ **不覆盖** ✗"
            f"（归档≠销毁 ✓；不变量 ⑦）⇒ 换一个 `--batch` 名字，或去掉 `--batch` 让脚本自动换名")
    n = 2
    while taken(f"{batch}-{n}"):
        n += 1
    print(f"⚠ 批次目录 `docs/archive/{batch}/` 已存在 ⇒ **不复用**（复用 = 覆盖归档记录 ✗）"
          f"⇒ 本批落到 `docs/archive/{batch}-{n}/` ✓")
    return f"{batch}-{n}"


def cmd_rotate(repo: Repo, keep: int, batch: str | None, apply: bool) -> int:
    rows = repo.rows()
    if len(rows) <= keep:
        print(f"✓ 台账 {len(rows)} 条 ≤ 保留 {keep} ⇒ 无需轮换 ✓")
    else:
        old, live = rows[:-keep], rows[-keep:]
        live_logs = {n for r in live if (n := log_name(r))}
        old_logs = {n for r in old if (n := log_name(r))}
        movable = sorted(old_logs - live_logs)  # 不变量 ③：还被活记录引用的一个都不动 ✗
        kept_shared = sorted(old_logs & live_logs)

        oldest = str(old[0].get("date") or "")[:10] or "unknown"
        name = batch_dir(repo, batch or f"e2e-{oldest}", explicit=batch is not None)
        dest = repo.archive / name

        print(f"台账 {len(rows)} 条 ⇒ 活 {len(live)} · 归档 {len(old)}")
        print(f"归档批次：docs/archive/{name}/")
        print(f"  记录 ⇒ ledger-records-older.jsonl.gz（{len(old)} 条）")
        print(f"  日志 ⇒ {len(movable)} 个搬走（.gz）")
        if kept_shared:
            print(f"  ⚠ {len(kept_shared)} 个日志被活记录**也**引用 ⇒ **原地不动** ✓")
        for n in movable[:5]:
            print(f"   · {n}")

        if not apply:
            print("（干跑；加 --apply 真做）")
            return 0

        dest.mkdir(parents=True, exist_ok=True)

        # ① 归档记录（原文逐字，只 gzip）—— ⚠ **兜底断言**：批次目录刚判过"不复用" ✓，
        # 所以这里**不该**已存在；真出现 ⇒ 宁可响亮失败也**绝不覆盖** ✗（不变量 ⑦）。
        rec = dest / "ledger-records-older.jsonl.gz"
        if rec.exists():
            _die(f"error: `{rec}` 已存在 ⇒ **不覆盖** ✗（归档≠销毁 ✓；不变量 ⑦）")
        raw = "\n".join(json.dumps(r, ensure_ascii=False) for r in old) + "\n"
        with gzip.open(rec, "wt", encoding="utf-8") as f:
            f.write(raw)

        # ② 归档日志（只搬 movable ✓）
        moved = 0
        for n in movable:
            src = repo.logs / n
            if not src.exists():
                continue
            with open(src, "rb") as fi, gzip.open(dest / f"{n}.gz", "wb") as fo:
                shutil.copyfileobj(fi, fo)
            src.unlink()
            moved += 1

        # ③ 活台账 = 保留的那些（逐字）
        repo.ledger.write_text(
            "\n".join(json.dumps(r, ensure_ascii=False) for r in live) + "\n",
            encoding="utf-8")
        print(f"✓ 归档 {len(old)} 条记录 · 搬走 {moved} 个日志 ⇒ 活台账 {len(live)} 条")

    # ④ **点名**（判据 ⑥）—— 与轮换**同一趟**做 ✓：轮换完不点名 ⇒ 下一次 docs-lint 判红 ✗
    changed = write_index(repo)
    print(f"{'✓ 归档索引已重新生成' if changed else '✓ 归档索引已是最新（幂等 ✓）'}"
          f"：docs/archive/README.md · 点名 {len(repo.archive_files())} 个文件")
    if not apply:
        print("（干跑；加 --apply 真写）")
    return 0


# ── 反向验证（夹具）──────────────────────────────────────────────────────────
FIX_KEEP = 10
FIX_ROWS = 16


def _fixture(root: Path, repo: Repo) -> None:
    """造一个**自包含**夹具：真 git 仓 + 真台账 + 真日志 + 真归档索引（零真仓写入 ✓）。

    ⚠ "零真仓写入"要**靠 `env=` 兑现** ✗→✓（洞 `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`）：
    `GIT_DIR` **优先于 cwd** ⇒ 从链接工作树的 hook 里跑时，下面这句 `git init` 会把
    `core.bare=true` 写进**调用者的仓**、夹具仓根本建不起来 ✗（实测读数见
    `scripts/git_env.py`）。
    """
    (root / "docs" / "archive").mkdir(parents=True, exist_ok=True)
    (root / "docs" / "e2e" / "logs").mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "init", "-q", str(root)], check=True, capture_output=True,
                   env=git_env.clean_git_env())
    (root / "README.md").write_text("# fixture\n", encoding="utf-8")
    (root / "docs" / "index.md").write_text("# fixture docs\n", encoding="utf-8")
    # `docs-lint` 要读的两份判据输入：
    #   · `docs/threshold-ratchet.json` —— ①②⑥ 的 caps（`threshold_ratchet` 按 `__file__`
    #     解析 ⇒ 它本来读的就是真仓那份；这里放一份只为让夹具**自述**完整 ✓）；
    #   · `scripts/docs-budget.json` —— 判据 ⑦ 的接手路径权威（`docs-lint` 按 **cwd** 读 ✓）。
    #     ⚠ **不照抄真仓那份** ✗：它通篇是 `docs/**` 指针 ⇒ 照抄会引来一大片
    #     「⑧ 新增悬空文档引用」（夹具里没有那些文档 ⇒ 判红的是**别的东西**、读数不可归属 ✗）。
    #     ⇒ 写一份**最小**的（口径同真仓：`onboarding.path` + `layer_max_lines` ✓）。
    real = Path(__file__).resolve().parent.parent
    (root / "scripts").mkdir(parents=True, exist_ok=True)
    shutil.copyfile(real / "docs" / "threshold-ratchet.json",
                    root / "docs" / "threshold-ratchet.json")
    # `docs/archive/README.md` 的**散文**（区块外、机器不许碰 ✓）—— 先拼出来：判据 ⑦ 的
    # `layer_max_lines.L3` 要按它的行数**算死**（见下 ✓）。
    prose = ("# `docs/archive/` —— 归档索引（**归档 ≠ 销毁** ✓）\n\n"
             "> 夹具抬头（散文，不许机器改 ✗）。\n\n"
             "| 归档文件 | 原路径 | 内容 | 归档日 |\n|---|---|---|---|\n"
             "| `old-thing.md.gz` | `" + "docs/" + "old.md` | 夹具 | 2026-10-16 |\n\n")
    (root / "scripts" / "docs-budget.json").write_text(json.dumps({
        "frozen": {},
        "onboarding": {
            # ⚠ 夹具里的假路径**拼出来**，不写成字面量 ✗ —— 否则 `docs-lint` 判据 ⑧ 会把它们
            # 当成**真的**悬空文档引用（2026-10-16 实测撞过：本文件里两处假路径各报 1 处 ⇒
            # pre-push 判红 ✗）。夹具语义一个字没变 ✓。
            "path": [{"file": "README.md", "max_lines": 100},
                     {"file": "docs/" + "index.md", "max_lines": 100}],
            "onboarding_max_lines": 200,
            # L3 = `docs/archive/**` 的 `*.md` = **散文行数 + 1**（生成区块**只许占一行** ✓）——
            # 照真仓的算术：真仓 **41 行散文 + 1 = 42** = `docs-budget.json` 的 `L3` ✓。
            # ⚠ 2026-10-17 之前这里写死 **400**（"给足余量" ✗）⇒ **余量正好把多行区块放过去了** ✗：
            # 真仓实测 64 行 > 42 **判红**，而夹具的 `docs-lint` ⑦ 却是绿的 ⇒ 夹具**咬不住** ✗。
            # 预算**必须与真仓同形**（区块只占 1 行）⇒ 多行区块在夹具里就会让 ⑦ 判红 ✓。
            "layer_max_lines": {"L0": 200, "L1": 200, "L2": 200,
                                "L3": len(prose.splitlines()) + 1},
        },
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    repo.index.write_text(prose + render_index(repo), encoding="utf-8")
    # 台账：`FIX_ROWS` 条、每条一个日志（日期递增 ⇒ 最老的进归档 ✓）
    rows = []
    for i in range(FIX_ROWS):
        day = 1 + i // 2
        name = f"2026-10-{day:02d}-fix{i:02d}-vc1.138.0.log"
        (root / "docs" / "e2e" / "logs" / name).write_text(f"fixture log {i}\n" * 40,
                                                           encoding="utf-8")
        rows.append({"date": f"2026-10-{day:02d}T0{i % 10}:00:00Z",
                     "commit": f"{i:040x}", "vscode": "1.138.0",
                     "log": f"docs/e2e/logs/{name}"})
    repo.ledger.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                           encoding="utf-8")
    # 归档里先放一个**只被归档记录引用**的旧日志（轮换不许动它 —— 它不是本轮 movable ✗）
    (root / "docs" / "archive" / "2026-10-01-prev.log.gz").write_bytes(
        gzip.compress(b"previous batch\n"))


def cmd_selftest() -> int:
    """**反向验证**：夹具上「台账超窗 ⇒ 必须触发轮换」+「判据 ⑥ 仍 0」✓（不许恒真 ✗）。"""
    results: list[tuple[str, bool]] = []
    with tempfile.TemporaryDirectory(prefix="rotate-e2e-selftest-") as tmp:
        root = Path(tmp)
        repo = Repo(root)
        _fixture(root, repo)
        docs_lint = Path(__file__).resolve().parent / "docs-lint.py"

        # ① 正对照：夹具**确实超窗**（否则下面的"必须轮换"就是恒真 ✗）
        results.append((f"夹具超窗（{FIX_ROWS} 条 > 保留 {FIX_KEEP}）",
                        len(repo.rows()) > FIX_KEEP))
        before = len(repo.rows())

        # ② 轮换必须**真的发生**（记录变少 + 日志搬走 + 归档里出现批次）
        cmd_rotate(repo, FIX_KEEP, None, apply=True)
        after = len(repo.rows())
        results.append((f"轮换真的发生（{before} ⇒ {after} 条 · 活 {FIX_KEEP}）",
                        after == FIX_KEEP))
        moved = sorted(p.name for p in repo.archive.glob("e2e-2026-10-01/*.gz"))
        results.append((f"日志随记录一起搬走（{len(moved)} 个 .gz 进归档）",
                        len(moved) >= (before - FIX_KEEP)))

        # ③ 判据 ⑥ **仍 0**：`docs-lint` 在夹具上 rc=0（点名齐 ⇒ 不判红 ✓）
        lint = subprocess.run([sys.executable, str(docs_lint)], cwd=root,
                              capture_output=True, text=True)
        results.append((f"docs-lint rc=0（判据 ⑥ 归档索引齐 ✓）：{lint.stdout.strip()[:70]}",
                        lint.returncode == 0))
        results.append(("脚本自己的 --check rc=0", cmd_check(repo) == 0))

        # ④ **不许恒真** ✗：把点名抹掉 ⇒ `--check` 必须判红（判据 ⑥ 的机械面真的咬住 ✓）
        text = repo.index.read_text(encoding="utf-8")
        stale = re.sub(re.escape(INDEX_BEGIN) + r".*?" + re.escape(INDEX_END),
                       INDEX_BEGIN + "\n" + INDEX_END, text, count=1, flags=re.S)
        repo.index.write_text(stale, encoding="utf-8")
        results.append(("点名被抹掉 ⇒ --check 判红（rc=1）", cmd_check(repo) == 1))
        lint2 = subprocess.run([sys.executable, str(docs_lint)], cwd=root,
                               capture_output=True, text=True)
        results.append(("点名被抹掉 ⇒ docs-lint 判红（判据 ⑥ 有牙 ✓）", lint2.returncode != 0))

        # ⑤ 重新生成 ⇒ 判据 ⑥ 回到 0（**红有出路** ✓）+ 幂等（跑两遍逐字节相同 ✓）
        write_index(repo)
        again = subprocess.run([sys.executable, str(docs_lint)], cwd=root,
                               capture_output=True, text=True)
        results.append(("重新生成 ⇒ docs-lint 回到 rc=0", again.returncode == 0))
        first = repo.index.read_text(encoding="utf-8")
        write_index(repo)
        results.append(("幂等：再生成一次逐字节相同",
                        repo.index.read_text(encoding="utf-8") == first))

        # ⑤b 哨兵缺失 ⇒ **响亮拒绝**，且退出码真的是 docstring 承诺的 **rc=2** ✓（不变量 5）——
        #     ⚠ 修前这里是 `raise SystemExit(字符串)` ⇒ Python 给的是 **rc=1** ✗，而白纸黑字
        #     写的是 rc=2（2026-10-17 实测对不上）⇒ 判据改成断言 `e.code`（不是消息文本 ✗）。
        saved = repo.index.read_text(encoding="utf-8")
        repo.index.write_text(re.sub(r"<!-- 归档索引:auto:(BEGIN|END) -->\n?", "", saved),
                              encoding="utf-8")
        try:
            write_index(repo)
            code = 0
        except SystemExit as e:
            code = e.code if isinstance(e.code, int) else 1
        results.append((f"哨兵缺失 ⇒ 响亮拒绝 rc={code}（不变量 5 · 承诺的 rc=2 ✓）", code == 2))
        repo.index.write_text(saved, encoding="utf-8")

        # ⑥ **区块恰好一行**（判据 ⑦ 的 L3 口径的机械面 ✓）：夹具的 `layer_max_lines.L3` =
        #    「散文行数 + 1」（照真仓 **41 + 1 = 42** ✓）⇒ 区块多一行 ⇒ **真的** `docs-lint` ⑦
        #    判红（上面那条 `docs-lint rc=0` 就会失败 ✓）。真仓实测：折行 ⇒ 区块 23 行 ⇒ 64 行 ✗。
        block = render_index(repo)
        results.append((f"索引区块恰好 1 行（README "
                        f"{len(repo.index.read_text(encoding='utf-8').splitlines())} 行 · "
                        f"区块 {block.count(chr(10))} 行）", block.count("\n") == 1))

    # ⑦ **批次撞车不许销毁归档**（不变量 ⑦ / 洞 `ROTATION-OVERWRITES-ARCHIVED-RECORDS` ✗→✓）
    #    真仓实测（2026-10-17）：默认批次名 `e2e-2026-10-15` 撞上已有批次 ⇒ **571 条**归档记录
    #    被 **48 条**覆盖 ⇒ **523 条销毁** ✗（而**没有任何判据报红** ✗）。这里把"撞车"造出来：
    #      ① **默认**名撞车 ⇒ 自动换名 ⇒ 老批次**逐字节未动** ✓ + 新批次自带自己的记录 ✓；
    #      ② **显式** `--batch` 撞车 ⇒ **响亮拒绝**（rc=2 口径 ✓）且**一个字节都没写** ✓。
    with tempfile.TemporaryDirectory(prefix="rotate-e2e-collision-") as tmp2:
        root2 = Path(tmp2)
        repo2 = Repo(root2)
        _fixture(root2, repo2)
        cmd_rotate(repo2, FIX_KEEP, None, apply=True)
        rec_old = repo2.archive / "e2e-2026-10-01" / "ledger-records-older.jsonl.gz"
        old_bytes, old_rows = rec_old.read_bytes(), _archived_records(rec_old)

        def _add(day: str, tag: str, rows: list[dict]) -> None:
            name = f"{day}-{tag}-vc1.138.0.log"
            (root2 / "docs" / "e2e" / "logs" / name).write_text("x\n" * 40, encoding="utf-8")
            rows.append({"date": f"{day}T00:00:00Z", "commit": tag.rjust(40, "0"),
                         "vscode": "1.138.0", "log": f"docs/e2e/logs/{name}"})

        extra: list[dict] = []
        _add("2026-10-01", "early", extra)                  # 最老 ⇒ 默认批次名仍是 10-01 ⇒ 撞车 ✓
        for i in range(6):
            _add(f"2026-10-{9 + i // 2:02d}", f"late{i}", extra)
        repo2.ledger.write_text(
            "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in extra + repo2.rows()),
            encoding="utf-8")
        cmd_rotate(repo2, FIX_KEEP, None, apply=True)       # 撞车 ⇒ 必须自动换名 ✓
        new_dir = repo2.archive / "e2e-2026-10-01-2"
        new_rows = _archived_records(new_dir / "ledger-records-older.jsonl.gz")
        results.append((f"默认批次名撞车 ⇒ 自动换名（{new_dir.name} ✓）", new_dir.is_dir()))
        results.append(("撞车后**老批次逐字节未动**（归档≠销毁 ✓）",
                        rec_old.read_bytes() == old_bytes))
        results.append((f"新批次自带记录（旧 {len(old_rows)} 条未动 · 新 {len(new_rows)} 条 ✓）",
                        len(old_rows) == 6 and len(new_rows) == 7))
        results.append(("撞车后 --check 仍 rc=0（点名齐 ✓）", cmd_check(repo2) == 0))
        lint3 = subprocess.run([sys.executable, str(docs_lint)], cwd=root2,
                               capture_output=True, text=True)
        results.append(("撞车后 docs-lint 仍 rc=0（⑥/⑦ 都不红 ✓）", lint3.returncode == 0))

        # ② 显式 `--batch` 撞车 ⇒ **响亮拒绝**，且**一个字节都没写** ✓
        more: list[dict] = []
        for i in range(6):
            _add(f"2026-10-{12 + i // 2:02d}", f"more{i}", more)
        repo2.ledger.write_text(
            repo2.ledger.read_text(encoding="utf-8")
            + "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in more),
            encoding="utf-8")
        try:
            cmd_rotate(repo2, FIX_KEEP, "e2e-2026-10-01", apply=True)
            code = 0
        except SystemExit as e:
            code = e.code if isinstance(e.code, int) else 1
        results.append((f"显式 --batch 撞车 ⇒ 响亮拒绝 rc={code}（不覆盖 · 承诺的 rc=2 ✓）",
                        code == 2))
        results.append(("拒绝时**一个字节都没写**（台账仍 16 条 · 老批次未动 ✓）",
                        len(repo2.rows()) == 16 and rec_old.read_bytes() == old_bytes))

    for label, ok in results:
        print(f"  {'✓' if ok else '✗'} {label}")
    bad = [label for label, ok in results if not ok]
    print(f"rotate-e2e-ledger --selftest：{len(results) - len(bad)}/{len(results)}"
          + (f" ⇒ **判红**：{' · '.join(bad)}" if bad else " ✓"))
    return 1 if bad else 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=None, help="仓库根（默认 = 本脚本所在仓库；夹具用 ✓）")
    ap.add_argument("--apply", action="store_true", help="真做（默认干跑）")
    ap.add_argument("--check", action="store_true", help="判据：日志都在吗 + 归档索引齐吗")
    ap.add_argument("--index", action="store_true", help="只重新生成归档索引（不轮换）")
    ap.add_argument("--selftest", action="store_true", help="反向验证（临时夹具，零真仓写入）")
    ap.add_argument("--keep", type=int, default=DEFAULT_KEEP,
                    help=f"活台账保留多少条（默认 {DEFAULT_KEEP}）")
    ap.add_argument("--batch", default=None, help="归档批次名（默认按最老被归档记录的日期）")
    a = ap.parse_args()

    if a.selftest:
        return cmd_selftest()

    root = Path(a.root).resolve() if a.root else Path(__file__).resolve().parent.parent
    repo = Repo(root)

    if a.check:
        return cmd_check(repo)
    if a.index:
        changed = write_index(repo)
        print(f"{'✓ 已重新生成' if changed else '✓ 已是最新（幂等 ✓）'}："
              f"docs/archive/README.md · 点名 {len(repo.archive_files())} 个文件")
        return 0
    return cmd_rotate(repo, a.keep, a.batch, a.apply)


if __name__ == "__main__":
    sys.exit(main())
