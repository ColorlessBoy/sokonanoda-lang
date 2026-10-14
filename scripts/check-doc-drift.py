#!/usr/bin/env python3
"""harness 接线的文档漂移守卫（洞 `DOC-DRIFT-CHECKER-NOT-COMMITTED`，2026-10-15 LINK-B）。

## 为什么需要它

cliA 为本轮三处文档漂移写了一个 checker（`.cache/ux/fix-cliA/check-doc-drift.py`），
但那是**证据件、不 commit** ⇒ 对齐做完了，却**没有守卫钉住它**：三处随时可以再漂回去 ✗
（与 `AGENTS.md`「**咬不住的守卫等于没有**」同族）。本文件把它收进 `scripts/`（`gate` 里跑），
并补上 `--selftest`（反向验证：改回旧文必须判红 ✓）。

## 判据：**先量事实、再对文档**

**事实**（从实现里量出来，不是抄文档 ✓）：

| # | 事实 |
|---|---|
| F1 | `opencode.json` **没有** `lsp` 块（LSP 由启动插件接线） |
| F2 | shim `.opencode/lsp/sokonanoda-lsp.sh` **只做转发**（`exec "$launcher" lsp`） |
| F3 | shim 声明 `SOKONANODA_BIN` 是逃生门 |
| F4 | `scripts/soko` 认 `SOKONANODA_OFFLINE`、**不认** `SOKONANODA_LSP_OFFLINE`（后者是**二进制**下载路的合法别名，见 `crates/cli/src/env/target.rs`，不是启动器这条路的名字） |
| F5 | 插件改写 `lsp.sokonanoda.command` |

**文档**（三处，必须与上面的事实一致）：

| # | 处 | 断言 |
|---|---|---|
| D1 | `README.md` 的 `Editor feedback for agents` 段 | 点名插件 · 不把接线归给 `opencode.json` · shim = forward only · 用 `SOKONANODA_OFFLINE`（不用 `SOKONANODA_LSP_OFFLINE`）· 点名 `SOKONANODA_BIN` |
| D2 | `skills/README.md` 的 shim 段 | 「只做转发」· 「自己不选二进制」· 点名 `SOKONANODA_BIN` |
| D3 | `editor/vscode/README.md` 的 opencode 段 | 点名插件 · 不把接线归给 `opencode.json` |

⚠ 文档侧**只锚定「谁接线 / 谁是转发 / 认哪个环境变量 / 逃生门叫什么」这类事实**，
**不冻结散文** —— 换词、重排、加句子都不判红 ✓（判红的只有"事实被写反"）。

## 用法

    python3 scripts/check-doc-drift.py             # 判据（gate）
    python3 scripts/check-doc-drift.py --json      # 机器可读
    python3 scripts/check-doc-drift.py --selftest  # 反向验证：三处改回旧文 ⇒ 必须判红并点名

退出码：0 = 一致 · 1 = **漂移**（判红并点名哪一处）· 2 = 形状异常（锚点/段落找不到 ⇒ 夹具坏了）
        · 3 = 仓库形状不对（缺文件；与 gate 其它步同一条纪律：探不到就 exit 3，绝不静默通过 ✗）。
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve()
REPO = HERE.parent.parent

README_ANCHOR = "Editor feedback for agents"
SKILLS_ANCHOR = "sokonanoda-lsp.sh"
EDITOR_ANCHOR = ".opencode/plugins/sokonanoda.ts"

# 反向验证用的**旧文**（`git show 755e5302^` 的原文；`--selftest` 把它们放回去，
# 三处必须各自判红 ✓）。**只作夹具**，不是文档的第二个真相。
OLD_README = """Editor feedback for agents needs no extra tooling: the repo-root
`opencode.json` starts the LSP through `.opencode/lsp/sokonanoda-lsp.sh`, which
resolves an installed VS Code extension binary, a local build, or the
version-pinned GitHub Release binary (auto-download; set
`SOKONANODA_LSP_OFFLINE=1` to forbid the network step). CLI-based agents get
the same contract via `--json` and `watch`."""
OLD_SKILLS = """其他 harness 若需要 LSP：`.opencode/lsp/sokonanoda-lsp.sh` shim →
`scripts/soko lsp`。"""
OLD_EDITOR = """The repo-root `opencode.json` already wires the language server for"""


def paragraph(text: str, anchor: str) -> str | None:
    """锚点所在的整段（到空行为止）。找不到 ⇒ `None`（判红，而不是丢栈 ✗）。"""
    idx = text.find(anchor)
    if idx < 0:
        return None
    start = text.rfind("\n\n", 0, idx) + 2
    end = text.find("\n\n", idx)
    return text[start : end if end >= 0 else len(text)]


def _read(root: Path, rel: str, overrides: dict[str, str]) -> str:
    return overrides.get(rel, (root / rel).read_text(encoding="utf-8"))


def run(root: Path, overrides: dict[str, str] | None = None) -> list[dict]:
    """全部判据。`overrides`（相对路径 → 文本）供 `--selftest` 喂反例 ✓。"""
    ov = overrides or {}
    results: list[dict] = []

    def add(cid: str, what: str, ok: bool, detail: str = "") -> None:
        results.append({"id": cid, "what": what, "ok": bool(ok), "detail": detail})

    # ── 事实（从实现里量）────────────────────────────────────────────────
    cfg_text = _read(root, "opencode.json", ov)
    cfg = json.loads(cfg_text)
    add("F1", "opencode.json 没有 `lsp` 块", "lsp" not in cfg, f"keys={list(cfg.keys())}")

    shim = _read(root, ".opencode/lsp/sokonanoda-lsp.sh", ov)
    add("F2", "shim 只做转发（`exec \"$launcher\" lsp`）", 'exec "$launcher" lsp' in shim)
    add("F3", "shim 声明 `SOKONANODA_BIN` 是逃生门", "SOKONANODA_BIN" in shim)

    soko = _read(root, "scripts/soko", ov)
    # ⚠ 判据是**行为**（读哪个环境变量），不是"这个名字出现过没有" —— 2026-10-15 LINK-B
    # 实测：第一版按裸名字计数，结果**咬到了自己的注释**（gate 里那句「认 X 不认 Y」
    # 把 Y 写了出来 ⇒ 假红 ✗）。守卫不该禁止**记录**这个事实，只该禁止**读它** ✓。
    n_off = soko.count("process.env.SOKONANODA_OFFLINE")
    n_lsp_off = soko.count("process.env.SOKONANODA_LSP_OFFLINE")
    add(
        "F4",
        "scripts/soko **读** SOKONANODA_OFFLINE、从不读 SOKONANODA_LSP_OFFLINE",
        n_off > 0 and n_lsp_off == 0,
        f"process.env.OFFLINE={n_off} process.env.LSP_OFFLINE={n_lsp_off}",
    )

    plugin = _read(root, ".opencode/plugins/sokonanoda.ts", ov)
    add("F5", "插件改写 `lsp.sokonanoda.command`", "lsp.sokonanoda.command" in plugin)

    # ── 文档（必须与事实一致）────────────────────────────────────────────
    readme = _read(root, "README.md", ov)
    para = paragraph(readme, README_ANCHOR)
    if para is None:
        add("D1", f"README.md 有 `{README_ANCHOR}` 段", False, "锚点找不到（段落被改名/删掉）")
    else:
        add("D1a", "README: 点名插件是接线的那一方", ".opencode/plugins/sokonanoda.ts" in para)
        add("D1b", "README: 不把接线归给 `opencode.json`", "`opencode.json` starts" not in para)
        add("D1c", "README: shim 是 forward only", "forward only" in para)
        add("D1d", "README: 用 SOKONANODA_OFFLINE、不用 SOKONANODA_LSP_OFFLINE",
            "SOKONANODA_OFFLINE" in para and "SOKONANODA_LSP_OFFLINE" not in para)
        add("D1e", "README: 点名逃生门 SOKONANODA_BIN", "SOKONANODA_BIN" in para)

    skills = _read(root, "skills/README.md", ov)
    seg = paragraph(skills, SKILLS_ANCHOR)
    if seg is None:
        add("D2", f"skills/README.md 有含 `{SKILLS_ANCHOR}` 的段", False, "锚点找不到")
    else:
        add("D2a", "skills/README: shim「只做转发」", "只做转发" in seg)
        add("D2b", "skills/README: 「自己不选二进制」", "自己不选二进制" in seg)
        add("D2c", "skills/README: 点名逃生门 SOKONANODA_BIN", "SOKONANODA_BIN" in seg)

    editor = _read(root, "editor/vscode/README.md", ov)
    # 这条是**全文件**断言（不依赖段落锚点）⇒ 锚点被删时它照样判红 ✓
    add("D3b", "editor/vscode/README: 不把接线归给 `opencode.json`",
        "repo-root `opencode.json` already wires" not in editor)
    eseg = paragraph(editor, EDITOR_ANCHOR)
    if eseg is None:
        add("D3", f"editor/vscode/README.md 有含 `{EDITOR_ANCHOR}` 的段", False, "锚点找不到")
    else:
        add("D3a", "editor/vscode/README: 点名插件接线", EDITOR_ANCHOR in eseg)

    return results


def _print(results: list[dict], as_json: bool) -> None:
    if as_json:
        print(json.dumps({"results": results, "ok": all(r["ok"] for r in results)}, ensure_ascii=False, indent=2))
        return
    print("harness 接线文档漂移守卫（scripts/check-doc-drift.py）")
    for r in results:
        print(f"  {'✓' if r['ok'] else '✗'} {r['id']} {r['what']}{(' — ' + r['detail']) if r['detail'] else ''}")
    bad = [r["id"] for r in results if not r["ok"]]
    print("  ⇒ " + ("文档与实现一致 ✓" if not bad else f"漂移（判红）：{', '.join(bad)} ✗"))


def _counterexamples(root: Path) -> list[tuple[str, dict[str, str], tuple[str, ...]]]:
    """三处旧文（每处一个反例）+ 该处**必须**判红的 id ✓。"""
    readme = (root / "README.md").read_text(encoding="utf-8")
    skills = (root / "skills/README.md").read_text(encoding="utf-8")
    editor = (root / "editor/vscode/README.md").read_text(encoding="utf-8")

    def swap(text: str, anchor: str, old: str) -> str | None:
        seg = paragraph(text, anchor)
        return None if seg is None else text.replace(seg, old, 1)

    cases = []
    r1 = swap(readme, README_ANCHOR, OLD_README)
    if r1 is not None:
        cases.append(("README.md（旧文：opencode.json 接线 + SOKONANODA_LSP_OFFLINE）",
                      {"README.md": r1}, ("D1b", "D1d")))
    r2 = swap(skills, SKILLS_ANCHOR, OLD_SKILLS)
    if r2 is not None:
        cases.append(("skills/README.md（旧文：没有「只做转发」）", {"skills/README.md": r2}, ("D2a",)))
    r3 = swap(editor, EDITOR_ANCHOR, OLD_EDITOR)
    if r3 is not None:
        cases.append(("editor/vscode/README.md（旧文：repo-root opencode.json 接线）",
                      {"editor/vscode/README.md": r3}, ("D3b",)))
    return cases


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--json", action="store_true", help="机器可读")
    ap.add_argument("--selftest", action="store_true", help="反向验证：三处旧文必须判红")
    args = ap.parse_args(argv)

    needed = ["opencode.json", ".opencode/lsp/sokonanoda-lsp.sh", ".opencode/plugins/sokonanoda.ts",
              "scripts/soko", "README.md", "skills/README.md", "editor/vscode/README.md"]
    missing = [rel for rel in needed if not (REPO / rel).exists()]
    if missing:
        print(f"check-doc-drift: 缺文件 {', '.join(missing)} ⇒ exit 3 ✗", file=sys.stderr)
        return 3

    if args.selftest:
        cases = _counterexamples(REPO)
        if len(cases) < 3:
            print(f"check-doc-drift --selftest: 只造出 {len(cases)}/3 个反例（锚点找不到）⇒ exit 2 ✗",
                  file=sys.stderr)
            return 2
        bad = 0
        for name, ov, must_be_red in cases:
            results = run(REPO, ov)
            red = {r["id"] for r in results if not r["ok"]}
            hit = [cid for cid in must_be_red if cid in red]
            ok = len(hit) == len(must_be_red)
            bad += 0 if ok else 1
            print(f"  {'✓' if ok else '✗'} 反例：{name} ⇒ 判红 {sorted(red) or '（无！）'}")
        if bad:
            print(f"  ⇒ **咬不住**：{bad}/3 个反例没被判红 ✗", file=sys.stderr)
            return 1
        print("  ⇒ 反向验证成立：三处旧文各自判红并点名 ✓（守卫真的在咬）")
        return 0

    results = run(REPO)
    _print(results, args.json)
    return 0 if all(r["ok"] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
