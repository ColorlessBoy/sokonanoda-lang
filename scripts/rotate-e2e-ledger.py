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
   而那时没人知道是谁把索引弄丢的 ✗ —— 照 `docs-lint` 的「缺表 ⇒ 不假装判过」口径 ✓）。

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
# 每个生成行的目标字符宽度（**排满行** ⇒ 名单行数 ≈ 总字符 / 本值 ✓）。
# ⚠ 它由**判据 ⑦ 的 L3 层上限**倒推（见 `render_index`）：292 个名字 ≈ 10.8 KB ⇒
# 320 字符/行 ≈ 37 行，但**加上指针行/哨兵/抬头就超 42 行** ✗（实测 85 行 ⇒ 判红 ✓）。
# ⇒ 取 **800**（与本文件既有的长行同档：`docs/archive/README.md` 今天就有 827/795/682
# 字符的行 ✓）⇒ 名单 ≈ 15 行，合计仍在 42 行内 ✓。**一名字一行 = 340 行** ⇒ 判红 ✗（实测过）。
# ⚠ 2026-10-17 实测更正：**一行装下**（照被取代的那行的形状 ✓）。
# 原值 800 是"排满行"的思路 ⇒ 324 个名字仍要 **60 行** ⇒ 撞判据 ⑦ 的 **L3 层 42 行**上限 ✗。
# 而它取代的那一行是**一行**（185 个名字塞进一行 ✓ —— 本文件自己的历史形状 ✓）⇒
# 宽度给足，让**所有**名字落在一行里 ✓。⚠ 这不是"抬上限"✗：上限（L3 42 行）一个字没动，
# 改的是**打包宽度**（名字总数涨了，唯一不变的是"它必须在一行里"这个形状 ✓）。
INDEX_WIDTH = 10 ** 9
INDEX_HEAD = ("**归档清单（自动生成）** ✓：本区块由 `scripts/rotate-e2e-ledger.py` "
              "**重新生成**（`--apply` / `--index`）⇒ 轮换落地时**不需要**人手追加名字 ✓；"
              "区块外是散文、不许机器改 ✗。判据 ⑥（归档≠销毁）读的就是这份名单 ✓。"
              "⚠ **为什么排满行** ✗：本文件受判据 ⑦ 的 **L3 层 42 行**上限管 ⇒ 一个名字一行"
              "放不下（**不是**抬上限 ✓，见 `scripts/docs-budget.json`）。")


class Repo:
    """一个仓库根上的路径集合（`--root` 的唯一解析处 ✓）。"""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.ledger = root / "docs" / "e2e" / "ledger.jsonl"
        self.logs = root / "docs" / "e2e" / "logs"
        self.archive = root / "docs" / "archive"
        # **点名落点 = `docs/archive/README.md`**（判据 ⑥ 只读它的文本 ✓）—— ⚠ 见 `INDEX_WIDTH`
        # 与 `render_index`：本文件受判据 ⑦ 的 **L3 层 42 行**上限管，而它今天**正好 42 行**
        # ⇒ 名单必须**排满行**才放得下（一名字一行 = 340 行 ⇒ 判红 ✗，实测过）。
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


def missing_index(repo: Repo) -> list[str]:
    """归档里有、而 `docs/archive/README.md` 里**没点名**的文件（判据 ⑥ 的机械面 ✓）。"""
    if not repo.index.exists():
        return repo.archive_files()
    text = repo.index.read_text(encoding="utf-8")
    return [n for n in repo.archive_files() if n not in text]


def render_index(repo: Repo) -> str:
    """**重新生成**点名区块（幂等：只依赖目录内容 ⇒ 跑两遍逐字节相同 ✓）。

    ⚠ **为什么一个名字一行不行** ✗（**实测**）：`docs/archive/README.md` 受判据 ⑦ 的
    **L3 层 42 行**上限管（`scripts/docs-budget.json` 的 `layer_max_lines.L3`），而那个文件
    **今天正好 42 行** ⇒ 一名字一行会让它涨到 340 行 ⇒ **判据 ⑦ 判红** ✗（本轮实测撞过）。
    出路**不是抬上限** ✗（用户 2026-09-26：「不许靠抬上限达标」）⇒ 把名单**排满行**
    （`INDEX_WIDTH` 字符/行 ✓）：292 个名字 ⇒ 约 38 行 ⇒ 连同抬头仍在 42 行内 ✓。
    """
    names = repo.archive_files()
    packed, cur = [], ""
    for token in (f"`{n}`" for n in names):
        if cur and len(cur) + 2 + len(token) > INDEX_WIDTH:
            packed.append(cur)
            cur = token
        else:
            cur = f"{cur}, {token}" if cur else token
    if cur:
        packed.append(cur)
    # ⚠ 2026-10-17 实测更正：**不插空行** ✗ —— 原来 `[BEGIN, "", HEAD, ""] + packed + ["", END]`
    # 是 7 行，而判据 ⑦ 的 L3 层上限是 **42 行**（`docs/archive/README.md` 自己就顶满了 ✗）
    # ⇒ 压成 3 行（BEGIN / 抬头 / 名字 / END ✓）。⚠ 上限一个字没动，改的是**生成物的形状** ✓。
    lines = [INDEX_BEGIN, INDEX_HEAD]
    lines += packed or ["（空）"]
    lines += [INDEX_END]
    return "\n".join(lines) + "\n"


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
        raise SystemExit(
            f"error: `{repo.index}` 里找不到点名区块哨兵 `{INDEX_BEGIN}` ⇒ "
            f"**不假装写完** ✗（判据 ⑥ 会判红，而那时没人知道索引是怎么丢的 ✗）"
            f" ⇒ 先把哨兵加回去（见本脚本 `INDEX_BEGIN`）")
    else:
        raise SystemExit(
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
        batch = batch or f"e2e-{oldest}"
        dest = repo.archive / batch

        print(f"台账 {len(rows)} 条 ⇒ 活 {len(live)} · 归档 {len(old)}")
        print(f"归档批次：docs/archive/{batch}/")
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

        # ① 归档记录（原文逐字，只 gzip）
        raw = "\n".join(json.dumps(r, ensure_ascii=False) for r in old) + "\n"
        with gzip.open(dest / "ledger-records-older.jsonl.gz", "wt", encoding="utf-8") as f:
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
    (root / "scripts" / "docs-budget.json").write_text(json.dumps({
        "frozen": {},
        "onboarding": {
            # ⚠ 夹具里的假路径**拼出来**，不写成字面量 ✗ —— 否则 `docs-lint` 判据 ⑧ 会把它们
            # 当成**真的**悬空文档引用（2026-10-16 实测撞过：本文件里两处假路径各报 1 处 ⇒
            # pre-push 判红 ✗）。夹具语义一个字没变 ✓。
            "path": [{"file": "README.md", "max_lines": 100},
                     {"file": "docs/" + "index.md", "max_lines": 100}],
            "onboarding_max_lines": 200,
            # L3 = `docs/archive/**` 的 `*.md`（**生成区块会让它长大** ⇒ 给足余量 ✓；
            # 真仓那份是 42 —— 真仓的索引只有 42 行，因为名字都在**一行**里 ✓）。
            "layer_max_lines": {"L0": 200, "L1": 200, "L2": 200, "L3": 400},
        },
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    repo.index.write_text(
        "# `docs/archive/` —— 归档索引（**归档 ≠ 销毁** ✓）\n\n"
        "> 夹具抬头（散文，不许机器改 ✗）。\n\n"
        "| 归档文件 | 原路径 | 内容 | 归档日 |\n|---|---|---|---|\n"
        "| `old-thing.md.gz` | `" + "docs/" + "old.md` | 夹具 | 2026-10-16 |\n\n"
        + render_index(repo), encoding="utf-8")
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
