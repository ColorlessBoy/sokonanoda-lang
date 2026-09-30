#!/usr/bin/env python3
"""**文档过期日期机制**（2026-09-30 用户要求 ✓）—— 每份活文档必须有过期日期，
到期后由**后续 agent 审查**：有特定理由才续期，没理由就删除或归档 ✓。

用户原话 ✓：「构建文档管理系统，每个文档必须有过期日期，到期后由后续 agent 审查，
有特定理由才继续保留」+「git 提交前自动检测所有文档是否快过期、是否需要续期」✓。

## 为什么是**集中登记表**而不是每份文档加 frontmatter ✗

判据 `scripts/docs-lint.py` ④ 把每份既有非入口文档**冻结在当前行数**（**只许减不许增** ✓）。
"给每份文档加一行 `> 过期：…`" 会让 **103 份**文档同时超预算 ⇒ **必然判红** ✗ ⇒
权威只能是**一份集中登记表**：`scripts/docs-expiry.json`（本脚本读写它 ✓）。
文档正文**不加**任何标记行（理由同上：那一行就是预算 ✗）。

## 口径（与 `docs-lint.py` **同一活文档口径** ✓）

活文档 = `git ls-files`（含未跟踪、排除 ignored）里的 **`*.md`**，且满足：
  * 仓库根 `*.md`（`README.md` / `AGENTS.md` / …），**或**
  * `docs/**`（**不含** `docs/archive/` `docs/perf/` `docs/gaps/` `docs/e2e/`）。
⚠ `docs-lint` 的活文档集**还含** `docs/**` 下的非 `.md`（脚本/台账）—— 那些不是"文档"，
本脚本只管 `.md` ✓（差异写在这里，免得两份口径各说各话 ✗）。

## 三类报告 + 退出码（**这就是"提交前自动检测"的判据** ✓）

  ① **已过期**（`days_left < 0`）⇒ **错误**，必须审查后处置（续期并写理由 / 删除 / 归档）；
  ② **快过期**（`0 ≤ days_left ≤ --warn-days`，默认 30 天）⇒ **提醒**，不阻塞；
  ③ **未登记**（活文档不在登记表里 / 登记表里的日期非法）⇒ **错误**，新文档必须登记；
  ④ **幽灵条目**（登记表里有、活文档集合里没有 —— 删除或归档之后留下的）⇒ **提醒**，`--prune` 清掉。

退出码：**0** = 只有提醒（②④）或全绿 · **1** = 有 ①③（**阻塞提交** ✓）· **2** = 用法错误。
⚠ 与 `docs-lint.py` 同一条纪律：**没扫到 ≠ 绿** ✗ —— 活文档 0 份一律 exit 1 ✓。

## 用法

  python3 scripts/docs-expiry-check.py --check                    # 报告（人类可读）
  python3 scripts/docs-expiry-check.py --check --json             # 机器可读（agent 用这条）
  python3 scripts/docs-expiry-check.py --renew <文件> --date 2027-09-30 \
      --reason "仍是活规范：docs-lint 判据输入"                    # 续期（**必须写理由** ✓）
  python3 scripts/docs-expiry-check.py --seed --reason "初始登记（第 512 轮）"
                                                                 # 给未登记文档按分档批量登记
  python3 scripts/docs-expiry-check.py --prune                    # 清掉幽灵条目（已删/已归档）
  python3 scripts/docs-expiry-check.py --selftest                 # 判据自检（四条判据必须咬得住）

## 到期审查流程（**后续 agent 的义务** ✓）

已过期文档的处置 = **三选一**，都要落 commit（评审可见 ✓）：

  1. **续期**（仍被引用 / 仍是活规范 / 有追溯价值）⇒ `--renew … --reason "…"`
     —— `--reason` **必填且 ≥8 字** ✓：没有理由就续期 = 机制失效 ✗；
  2. **删除**（没人引用、结论已并入他处）⇒ `git rm` + `--prune`；
  3. **归档**（有追溯价值但不该占活文档）⇒ 移进 `docs/archive/` 并在
     `docs/archive/README.md` **点名**（判据 ⑥ 会查 ✓）+ `--prune`。

**提交前自动检测**：`scripts/githooks/pre-commit`（`scripts/install-hooks.sh` 装 ✓）
—— 有 ①③ 就**拒绝提交** ✓；逃生门 `SOKO_SKIP_HOOK=1` / `git commit --no-verify`（要留痕 ✓）。
"""
import datetime as dt
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "scripts" / "docs-expiry.json"

# 与 `docs-lint.py` 的 `live_docs()` 同一口径的**目录**排除（用户 2026-09-30 明确点名 ✓）
SKIP_DIRS = ("docs/archive/", "docs/perf/", "docs/gaps/", "docs/e2e/")

WARN_DAYS_DEFAULT = 30
MIN_REASON = 8  # 续期理由下限（**没有理由的续期 = 机制失效** ✗）

# 分档（用户 2026-09-30：「入口/规范类 12 个月，过程/量测记录类 3 个月」✓）
TIERS = {
    "entry": 365,    # 入口 / 权威（接手必读链）
    "spec": 365,     # 活规范（开发者参考、设计契约）
    "process": 90,   # 过程 / 计划 / 交接（写完就该凉）
    "record": 90,    # 量测 / 台账 / 快照（读数会过时）
}
MAX_RENEW_DAYS = 731  # 单次续期的上限（2 年）—— 逼出**周期性复审**，不许一次续到天荒地老 ✗

ENTRY_DOCS = {
    "README.md", "AGENTS.md", "ROADMAP.md", "REQUIREMENTS.md", "STATUS.md",
    "docs/ONBOARDING.md", "docs/README.md", "docs/architecture.md", "docs/protocol.md",
    "docs/TESTING.md", "docs/PERF.md", "docs/RELEASE.md", "docs/E2E.md",
    "docs/LESSONS.md", "docs/CI-FAILURES.md", "docs/vscode-dev-guide.md",
}
PROCESS_HINTS = (
    "plan", "handoff", "proposal", "measure", "ledger", "as-built", "brief",
    "notes/", "archive", "snapshot", "state", "log",
)


# ── 活文档口径 ──────────────────────────────────────────────────────────────

def tracked() -> set[str]:
    """`git ls-files`（含未跟踪、排除 ignored）—— 与 `docs-lint.py` 同一条命令 ✓。"""
    out = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    return {p for p in out.split("\0") if p}


def live_docs() -> list[str]:
    """活文档（**过期机制口径**）= 根 `*.md` + `docs/**.md`（不含 4 个被点名的目录）✓。"""
    out = []
    for f in tracked():
        if not f.endswith(".md") or f.startswith(SKIP_DIRS):
            continue
        if f.startswith("docs/") or "/" not in f:
            out.append(f)
    return sorted(out)


def tier_of(path: str) -> str:
    """按**文档性质**分档（`--seed` 用；`--renew` 可用 `--tier` 覆盖 ✓）。"""
    if path in ENTRY_DOCS:
        return "entry"
    low = path.lower()
    if path.startswith("docs/notes/") or any(h in low for h in PROCESS_HINTS):
        return "process" if "plan" in low or "handoff" in low or "proposal" in low else "record"
    return "spec"


# ── 登记表读写 ──────────────────────────────────────────────────────────────

def load_registry() -> dict:
    if not REGISTRY.exists():
        return {"_comment": REGISTRY_COMMENT, "policy": policy_block(), "documents": {}}
    return json.loads(REGISTRY.read_text(encoding="utf-8"))


def save_registry(reg: dict) -> None:
    reg["_comment"] = REGISTRY_COMMENT
    reg["policy"] = policy_block()
    reg["documents"] = dict(sorted(reg.get("documents", {}).items()))
    REGISTRY.write_text(
        json.dumps(reg, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


REGISTRY_COMMENT = (
    "**文档过期日期登记表**（2026-09-30 用户要求）—— 本表是**权威** ✓：每份活文档的"
    "过期日期 + 上次续期日 + **续期理由**。口径与判据见 `scripts/docs-expiry-check.py`"
    "（`--check` / `--json` / `--renew` / `--seed` / `--prune` / `--selftest`）。"
    "⚠ **不许手改**：一律用 `--renew` / `--seed`（理由必填 ≥8 字 ⇒ 机制才有效 ✓）。"
    "⚠ 为什么不是每份文档加 frontmatter：`docs-lint.py` ④ 把既有非入口文档**冻结在当前行数**"
    "（只许减不许增）⇒ 103 份文档同时加一行 = **必然判红** ✗（详见脚本 docstring）。"
    "到期处置三选一：**续期**（`--renew` + 理由）/ **删除** / **归档**（`docs/archive/README.md` 点名）"
    "—— 处置后跑 `--prune` 清幽灵条目 ✓。"
)


def policy_block() -> dict:
    return {
        "_comment": (
            "warn_days = 快过期提醒窗口（天，只提醒不阻塞）· tiers = 分档天数"
            "（entry/spec 12 个月 · process/record 3 个月）· max_renew_days = 单次续期上限"
            "（逼出周期性复审 ✓）。改这里 = 改机制 ⇒ 必须在 commit message 写理由 ✓。"
        ),
        "warn_days": WARN_DAYS_DEFAULT,
        "tiers": dict(TIERS),
        "max_renew_days": MAX_RENEW_DAYS,
    }


# ── 判定（纯函数 ⇒ 可自检 ✓）───────────────────────────────────────────────

def parse_date(s: str):
    try:
        return dt.date.fromisoformat(s)
    except (ValueError, TypeError):
        return None


def evaluate(docs: list[str], reg: dict, today: dt.date, warn_days: int) -> dict:
    """**纯判定**：给定文档集 / 登记表 / 今天 ⇒ 四类结果（不读盘、不打印 ✓）。"""
    entries = reg.get("documents") or {}
    expired, expiring, fresh, unregistered, malformed = [], [], [], [], []
    for f in sorted(set(docs)):
        e = entries.get(f)
        exp = (e or {}).get("expires")
        if not e or not exp:
            unregistered.append(f)
            continue
        d = parse_date(exp)
        if d is None:
            malformed.append({"path": f, "expires": exp})
            continue
        item = {
            "path": f,
            "expires": exp,
            "days_left": (d - today).days,
            "tier": e.get("tier"),
            "renewed": e.get("renewed"),
            "reason": e.get("reason"),
        }
        left = item["days_left"]
        if left < 0:
            expired.append(item)
        elif left <= warn_days:
            expiring.append(item)
        else:
            fresh.append(item)
    known = set(docs)
    stale = [
        {"path": f, "expires": (entries[f] or {}).get("expires")}
        for f in sorted(entries)
        if f not in known
    ]
    for bucket in (expired, expiring):
        bucket.sort(key=lambda x: x["days_left"])
    blocking = bool(expired or unregistered or malformed or not docs)
    return {
        "today": today.isoformat(),
        "warn_days": warn_days,
        "counts": {
            "live": len(set(docs)),
            "registered": len(set(docs)) - len(unregistered) - len(malformed),
            "expired": len(expired),
            "expiring": len(expiring),
            "unregistered": len(unregistered),
            "malformed": len(malformed),
            "stale": len(stale),
            "fresh": len(fresh),
        },
        "expired": expired,
        "expiring": expiring,
        "unregistered": unregistered,
        "malformed": malformed,
        "stale": stale,
        "ok": not blocking,
    }


def warn_days_of(reg: dict, override: int | None) -> int:
    if override is not None:
        return override
    v = (reg.get("policy") or {}).get("warn_days")
    return v if isinstance(v, int) else WARN_DAYS_DEFAULT


# ── 报告 ────────────────────────────────────────────────────────────────────

def render(res: dict) -> str:
    c = res["counts"]
    lines = [
        f"docs-expiry {res['today']}（警戒线 {res['warn_days']} 天）· "
        f"活文档 {c['live']} 份 · 已登记 {c['registered']} 份"
    ]
    if c["live"] == 0:
        lines.append("✗ 活文档 **0 份** ⇒ **没扫到 ≠ 绿** ✗（检查 cwd 与 `git ls-files`）")
        return "\n".join(lines)

    def head(n, title, tail=""):
        lines.append(f"\n{n} {title}（{tail}）")

    if res["expired"]:
        head("①", "**已过期** ⇒ 必须审查后处置", f"{c['expired']} 份 · **阻塞提交**")
        for it in res["expired"]:
            lines.append(
                f"  ✗ {it['path']}：过期 {-it['days_left']} 天（{it['expires']}）"
                f" · 上次续期 {it['renewed'] or '—'} · 档 {it['tier'] or '—'}"
                f" · 理由：{(it['reason'] or '—')[:60]}"
            )
        lines.append(
            "      处置三选一：**续期** `--renew <文件> --date <新过期日> --reason \"…\"`"
            " / **删除** / **归档**（`docs/archive/README.md` 点名）⇒ 后两者再 `--prune`"
        )
    if res["expiring"]:
        head("②", "快过期 ⇒ 提醒续期（**不阻塞**）", f"{c['expiring']} 份 · ≤{res['warn_days']} 天")
        for it in res["expiring"]:
            when = "**今天到期**" if it["days_left"] == 0 else f"还剩 {it['days_left']} 天"
            lines.append(f"  ⚠ {it['path']}：{when}（{it['expires']}）· 档 {it['tier'] or '—'}")
    if res["unregistered"]:
        head("③", "**未登记过期日期** ⇒ 新文档必须登记", f"{c['unregistered']} 份 · **阻塞提交**")
        for f in res["unregistered"]:
            lines.append(f"  ✗ {f}（档建议：{tier_of(f)}）")
        lines.append(
            "      处置：`--renew <文件> --date <过期日> --reason \"…\"`，或一次登记全部"
            " `--seed --reason \"初始登记\"`"
        )
    if res["malformed"]:
        head("④", "登记表日期**非法** ⇒ 机制失效", f"{c['malformed']} 份 · **阻塞提交**")
        for it in res["malformed"]:
            lines.append(f"  ✗ {it['path']}：expires={it['expires']!r}（要 `YYYY-MM-DD`）")
    if res["stale"]:
        head("⑤", "登记表**幽灵条目** ⇒ 已删/已归档，请清理（**不阻塞**）", f"{c['stale']} 份")
        for it in res["stale"]:
            lines.append(f"  ⚠ {it['path']}（登记过期 {it['expires']}）")
        lines.append("      处置：`--prune`（删除或归档之后跑 ✓）")

    if res["ok"]:
        lines.append(
            f"\n✓ 无已过期 / 无未登记 ⇒ 放行（快过期 {c['expiring']} 份只是提醒 ✓）"
        )
    else:
        lines.append(
            f"\n✗ 结论：已过期 {c['expired']} + 未登记 {c['unregistered']} + 非法 {c['malformed']}"
            f" ⇒ **exit 1（提交会被 pre-commit 拦住）**"
        )
    return "\n".join(lines)


# ── 模式：check ─────────────────────────────────────────────────────────────

def cmd_check(as_json: bool, today: dt.date, warn_days_override: int | None) -> int:
    docs = live_docs()
    reg = load_registry()
    res = evaluate(docs, reg, today, warn_days_of(reg, warn_days_override))
    if as_json:
        print(json.dumps(res, ensure_ascii=False, indent=2))
    else:
        print(render(res))
    return 0 if res["ok"] else 1


# ── 模式：renew / seed ──────────────────────────────────────────────────────

def valid_reason(reason: str | None) -> str | None:
    if not reason or len(reason.strip()) < MIN_REASON:
        return None
    return reason.strip()


def apply_renew(reg: dict, path: str, expires: dt.date, renewed: dt.date,
                reason: str, tier: str | None) -> dict:
    reg.setdefault("documents", {})[path] = {
        "expires": expires.isoformat(),
        "renewed": renewed.isoformat(),
        "tier": tier or tier_of(path),
        "reason": reason,
    }
    return reg


def cmd_renew(path: str, date_s: str | None, reason: str | None, tier: str | None,
              today: dt.date) -> int:
    docs = live_docs()
    if path not in docs:
        print(
            f"✗ {path} 不在活文档集合里（根 `*.md` + `docs/**.md`，不含 "
            f"{' / '.join(SKIP_DIRS)}）⇒ 拒绝登记（**别给不存在的文档发通行证** ✗）",
            file=sys.stderr,
        )
        return 2
    r = valid_reason(reason)
    if r is None:
        print(
            f"✗ 续期**必须写理由**（`--reason`，≥{MIN_REASON} 字）—— 用户要求"
            "「**有特定理由才继续保留**」✓；没有理由就该删除或归档 ✗",
            file=sys.stderr,
        )
        return 2
    if tier is not None and tier not in TIERS:
        print(f"✗ 未知档 {tier!r}（可选：{' / '.join(TIERS)}）", file=sys.stderr)
        return 2
    if not date_s:
        print("✗ `--renew` 要 `--date YYYY-MM-DD`（新的过期日期）", file=sys.stderr)
        return 2
    exp = parse_date(date_s)
    if exp is None:
        print(f"✗ `--date {date_s}` 不是 `YYYY-MM-DD`", file=sys.stderr)
        return 2
    if exp <= today:
        print(f"✗ 新过期日 {exp} 必须**晚于今天** {today}（续一个已经过期的日期没有意义 ✗）",
              file=sys.stderr)
        return 2
    if (exp - today).days > MAX_RENEW_DAYS:
        print(
            f"✗ 新过期日 {exp} 距今 {(exp - today).days} 天 > 上限 {MAX_RENEW_DAYS} 天 ⇒ "
            f"**不许一次续到天荒地老** ✗（这条上限的作用就是逼出周期性复审 ✓）",
            file=sys.stderr,
        )
        return 2
    reg = load_registry()
    old = (reg.get("documents") or {}).get(path)
    apply_renew(reg, path, exp, today, r, tier)
    save_registry(reg)
    print(f"✅ 已续期：{path}\n   过期日 {old.get('expires') if old else '（新登记）'} → "
          f"**{exp}**（距今 {(exp - today).days} 天）· 档 {tier or tier_of(path)}\n"
          f"   理由：{r}")
    return 0


def cmd_seed(reason: str | None, base_s: str | None, today: dt.date) -> int:
    r = valid_reason(reason)
    if r is None:
        print(f"✗ `--seed` 也要 `--reason`（≥{MIN_REASON} 字）：批量登记同样是"
              "「有理由才留」的一环 ✓", file=sys.stderr)
        return 2
    base = parse_date(base_s) if base_s else today
    if base is None:
        print(f"✗ `--base {base_s}` 不是 `YYYY-MM-DD`", file=sys.stderr)
        return 2
    docs = live_docs()
    reg = load_registry()
    entries = reg.setdefault("documents", {})
    added = []
    for f in docs:
        if f in entries and (entries[f] or {}).get("expires"):
            continue
        t = tier_of(f)
        exp = base + dt.timedelta(days=TIERS[t])
        apply_renew(reg, f, exp, base, f"{r}（分档 {t} = {TIERS[t]} 天）", t)
        added.append((f, t, exp.isoformat()))
    if not added:
        print("✓ 没有未登记的活文档 ⇒ `--seed` 无需改动 ✓")
        return 0
    save_registry(reg)
    print(f"✅ 已登记 {len(added)} 份（基准日 {base}）· 分档天数 {TIERS}")
    by_tier: dict[str, int] = {}
    for f, t, exp in added:
        by_tier[t] = by_tier.get(t, 0) + 1
    print("   分档计数：" + " · ".join(f"{k} {v}" for k, v in sorted(by_tier.items())))
    for f, t, exp in added[:10]:
        print(f"   + {f}（{t} ⇒ {exp}）")
    if len(added) > 10:
        print(f"   … 其余 {len(added) - 10} 份见 {REGISTRY.relative_to(ROOT)}")
    return 0


def cmd_prune(today: dt.date) -> int:
    docs = set(live_docs())
    reg = load_registry()
    entries = reg.get("documents") or {}
    gone = [f for f in sorted(entries) if f not in docs]
    if not gone:
        print("✓ 登记表没有幽灵条目 ✓")
        return 0
    for f in gone:
        del entries[f]
    reg["documents"] = entries
    save_registry(reg)
    print(f"✅ 已清掉 {len(gone)} 条幽灵条目（文件已不在活文档集合）：")
    for f in gone:
        print(f"   - {f}")
    return 0


# ── 自检（**咬不住的守卫等于没有** ✗ ⇒ 每条判据都要被证明咬得住 ✓）──────────

def selftest() -> int:
    today = dt.date(2026, 9, 30)
    docs = ["README.md", "docs/old.md", "docs/soon.md", "docs/fresh.md", "docs/none.md"]
    reg = {"documents": {
        "README.md": {"expires": "2027-09-30", "tier": "entry", "reason": "入口"},
        "docs/old.md": {"expires": "2026-09-01", "tier": "spec", "reason": "旧"},
        "docs/soon.md": {"expires": "2026-10-10", "tier": "spec", "reason": "快到期"},
        "docs/fresh.md": {"expires": "2027-01-01", "tier": "spec", "reason": "还早"},
        "docs/gone.md": {"expires": "2027-01-01", "tier": "spec", "reason": "幽灵"},
        "docs/bad.md": {"expires": "明天", "tier": "spec", "reason": "非法日期"},
    }}
    res = evaluate(docs + ["docs/bad.md"], reg, today, 30)
    c = res["counts"]
    checks = [
        ("① 已过期咬得住", [i["path"] for i in res["expired"]] == ["docs/old.md"]),
        ("① 过期 ⇒ 阻塞（ok=False）", res["ok"] is False),
        ("② 快过期咬得住（30 天窗口）", [i["path"] for i in res["expiring"]] == ["docs/soon.md"]),
        ("② 还早的不报", "docs/fresh.md" not in [i["path"] for i in res["expiring"]]),
        ("③ 未登记咬得住", res["unregistered"] == ["docs/none.md"]),
        ("④ 非法日期咬得住（不是静默当绿 ✗）", [i["path"] for i in res["malformed"]] == ["docs/bad.md"]),
        ("⑤ 幽灵条目咬得住", [i["path"] for i in res["stale"]] == ["docs/gone.md"]),
        ("计数自洽", c["expired"] == 1 and c["expiring"] == 1 and c["unregistered"] == 1
         and c["malformed"] == 1 and c["stale"] == 1 and c["live"] == 6),
    ]
    # ② 反向：只有快过期时**不许**阻塞（否则"提醒"就变成了"拦路" ✗）
    ok_only = evaluate(["docs/soon.md"], reg, today, 30)
    checks.append(("② 反向：只快过期 ⇒ 不阻塞（ok=True）", ok_only["ok"] is True))
    # 零扫描 ≠ 绿
    checks.append(("空文档集 ⇒ 不绿（**没扫到 ≠ 绿** ✗）", evaluate([], reg, today, 30)["ok"] is False))
    # 窗口边界：0 天 = 今天到期 ⇒ 提醒（不是过期）
    edge = evaluate(["docs/soon.md"], {"documents": {"docs/soon.md": {"expires": "2026-09-30"}}},
                    today, 30)
    checks.append(("边界：今天到期 ⇒ 提醒而非过期", edge["expiring"] and not edge["expired"]))
    # 窗口可调：--warn-days 1 ⇒ 10 天外的**不报**
    narrow = evaluate(["docs/soon.md"], reg, today, 1)
    checks.append(("窗口可调：warn_days=1 ⇒ 不报 10 天后的", not narrow["expiring"] and narrow["ok"]))
    # 分档推断（--seed 的判据）
    checks.append(("分档：入口 ⇒ entry", tier_of("AGENTS.md") == "entry"))
    checks.append(("分档：计划类 ⇒ process", tier_of("docs/design/e2-plan.md") == "process"))
    checks.append(("分档：量测类 ⇒ record", tier_of("docs/design/p1a-measurements.md") == "record"))
    checks.append(("分档：其余 ⇒ spec", tier_of("docs/design/notation-subset.md") == "spec"))
    # 理由下限（**没有理由的续期 = 机制失效** ✗）
    checks.append(("理由下限：短理由被拒", valid_reason("续期") is None and valid_reason("") is None))
    checks.append(("理由下限：正常理由通过", valid_reason("仍是活规范：判据输入") is not None))
    # 登记表**必须真的在活文档集合里**才能登记
    checks.append(("活文档口径非空（零扫描 ≠ 绿）", len(live_docs()) > 0))
    checks.append(("活文档口径排除 archive/perf/gaps/e2e",
                   not any(f.startswith(SKIP_DIRS) for f in live_docs())))
    checks.append(("活文档口径 = 同一 git ls-files 口径",
                   all(f in tracked() for f in live_docs())))

    ok = all(hit for _, hit in checks)
    for label, hit in checks:
        print(f"  {'✓' if hit else '✗'} {label}")
    print(f"docs-expiry --selftest：{sum(1 for _, h in checks if h)}/{len(checks)} "
          f"条判据咬得住 {'✓' if ok else '✗（有判据咬不住！）'}")
    return 0 if ok else 1


# ── main ────────────────────────────────────────────────────────────────────

USAGE = ("用法：docs-expiry-check.py [--check] [--json] [--warn-days N] [--today YYYY-MM-DD]\n"
         "      | --renew <文件> --date YYYY-MM-DD --reason \"…\" [--tier entry|spec|process|record]\n"
         "      | --seed --reason \"…\" [--base YYYY-MM-DD]\n"
         "      | --prune | --selftest | --help")


def main(argv: list[str]) -> int:
    # ⚠ **不许静默回落** ✗（同 `docs-lint.py` 的教训：错拼的参数被忽略、回落成全量检查还 exit 0 ✗）
    known = {"--check", "--json", "--selftest", "--prune", "--help", "-h",
             "--renew", "--seed", "--date", "--reason", "--tier", "--base",
             "--warn-days", "--today"}
    unknown = [a for a in argv if a.startswith("-") and a not in known]
    if unknown:
        print(f"✗ 未知参数 {unknown} ⇒ 拒绝执行（**不许静默回落** ✗）", file=sys.stderr)
        return 2

    def opt(name: str) -> str | None:
        if name in argv:
            i = argv.index(name)
            if i + 1 >= len(argv):
                print(f"✗ {name} 缺值", file=sys.stderr)
                return None
            return argv[i + 1]
        return None

    if "--help" in argv or "-h" in argv:
        print(__doc__)
        return 0
    if "--selftest" in argv:
        return selftest()

    today = dt.date.today()
    if "--today" in argv:
        t = parse_date(opt("--today") or "")
        if t is None:
            print("✗ `--today` 要 `YYYY-MM-DD`", file=sys.stderr)
            return 2
        today = t
    warn = None
    if "--warn-days" in argv:
        try:
            warn = int(opt("--warn-days") or "")
        except ValueError:
            print("✗ `--warn-days` 要整数", file=sys.stderr)
            return 2

    if "--renew" in argv:
        return cmd_renew(opt("--renew") or "", opt("--date"), opt("--reason"),
                         opt("--tier"), today)
    if "--seed" in argv:
        return cmd_seed(opt("--reason"), opt("--base"), today)
    if "--prune" in argv:
        return cmd_prune(today)
    if "--check" in argv or "--json" in argv or len(argv) == 0:
        return cmd_check("--json" in argv, today, warn)
    print(USAGE, file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
