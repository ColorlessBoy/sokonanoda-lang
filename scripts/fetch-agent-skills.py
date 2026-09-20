#!/usr/bin/env python3
"""Fetch the third-party agent skills this repository's maintainers use.

Why this is a *script* and not a vendored directory: the payload is other
people's MIT-licensed work that has nothing to do with the product. Committing
~30 skill bundles into a language repository would be noise in every diff and
would make the repo the wrong place to update them. So the payload is fetched
into `.dsh/skills/` (git-ignored, like `editor/vscode/bin/`) and the *recipe* is
committed (`dsh/agent-skills.json`).

Why `.dsh/skills/`: DSH scans skill roots in rank order —
`<root>/.dsh/skills` (100) → `<root>/.agents/skills` (200) → … (see
`docs/design/deepseek-harness.md` §1.2 D2). Rank 100 is the highest-priority
project-level root, it is not used by this repository for anything else, and
`crates/cli/tests/dsh.rs` guards only `skills/` ↔ `.agents/skills/`, so nothing
here can drift the product's own skill contract.

Normalisation is not cosmetic. DSH **drops an entire skill** whose frontmatter
carries Claude-Code-era camelCase invocation keys
(`disableModelInvocation` / `modelInvocable` / `userInvocable`) — see
`docs/design/deepseek-harness.md` §1.2 D3. Upstream packs are written for
Claude Code, so this script strips those keys and keeps only the keys DSH
accepts: `name`, `description`, `whenToUse`, `metadata`,
`disable-model-invocation`, `user-invocable`.

Usage:
  python3 scripts/fetch-agent-skills.py            # fetch + normalise
  python3 scripts/fetch-agent-skills.py --check    # verify, change nothing
  python3 scripts/fetch-agent-skills.py --list     # show the manifest

Network: needs GitHub. Behind a proxy, export `HTTPS_PROXY`. Python stdlib only.
Exit codes: 0 ok · 1 fetch/normalise failure · 2 usage.
"""

from __future__ import annotations

import argparse
import io
import json
import os
import re
import shutil
import sys
import tarfile
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "dsh" / "agent-skills.json"
DEST = ROOT / ".dsh" / "skills"

# DSH accepts exactly these frontmatter keys (D3). Anything else is dropped —
# a rejected key does not degrade the skill, it removes it from the catalogue.
DSH_KEYS = {
    "name",
    "description",
    "whenToUse",
    "metadata",
    "disable-model-invocation",
    "user-invocable",
}
# Keys known to make DSH drop the skill outright; their presence upstream is the
# whole reason normalisation exists.
POISON_KEYS = {"disableModelInvocation", "modelInvocable", "userInvocable"}

SKILL_NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def load_manifest() -> dict:
    with open(MANIFEST, encoding="utf-8") as fh:
        return json.load(fh)


def download(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "sokonanoda-agent-skills"})
    proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("https_proxy")
    if proxy:
        opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({"http": proxy, "https": proxy})
        )
    else:
        opener = urllib.request.build_opener()
    with opener.open(req, timeout=120) as resp:
        return resp.read()


def split_frontmatter(text: str) -> tuple[list[tuple[str, str]], str]:
    """Return (top-level key/value pairs, body).

    Only top-level scalar keys are collected; nested blocks (`metadata:`) are
    preserved verbatim as part of the rebuilt frontmatter, so this is a
    *line-oriented* pass rather than a YAML parser (stdlib has none, and the
    packs use a tiny subset).
    """
    if not text.startswith("---\n"):
        return [], text
    rest = text[4:]
    end = rest.find("\n---\n")
    if end < 0:
        return [], text
    block, body = rest[:end], rest[end + 5 :]
    pairs: list[tuple[str, str]] = []
    for line in block.splitlines():
        if not line.strip() or line[0] in " \t":
            continue  # nested/continuation line — handled by the caller
        if ": " in line:
            key, value = line.split(": ", 1)
            pairs.append((key.strip(), value.strip()))
        elif line.rstrip().endswith(":"):
            pairs.append((line.rstrip()[:-1].strip(), ""))
    return pairs, body


def normalise(path: Path, dir_name: str, notes: list[str]) -> bool:
    """Rewrite one SKILL.md's frontmatter into the DSH-accepted subset."""
    text = path.read_text(encoding="utf-8")
    pairs, body = split_frontmatter(text)
    if not pairs:
        notes.append(f"{dir_name}: no frontmatter — DSH requires `name`/`description`")
        return False

    values = dict(pairs)
    dropped = [k for k in values if k in POISON_KEYS]
    kept: list[str] = []
    for key, value in pairs:
        if key in POISON_KEYS:
            continue
        if key not in DSH_KEYS:
            continue
        kept.append(f"{key}: {value}" if value else f"{key}:")

    name = values.get("name", "")
    if name != dir_name:
        notes.append(f"{dir_name}: frontmatter name `{name}` != directory name")
        return False
    if not SKILL_NAME_RE.match(name):
        notes.append(f"{dir_name}: name `{name}` is not kebab-case")
        return False
    if not values.get("description"):
        notes.append(f"{dir_name}: empty description")
        return False

    path.write_text("---\n" + "\n".join(kept) + "\n---\n" + body, encoding="utf-8")
    if dropped:
        notes.append(f"{dir_name}: stripped {', '.join(sorted(dropped))}")
    return True


def fetch_pack(spec: dict, dest_root: Path, check_only: bool) -> tuple[int, list[str]]:
    """Extract the selected skills of one pack. Returns (count, notes)."""
    notes: list[str] = []
    url = f"https://codeload.github.com/{spec['repo']}/tar.gz/refs/heads/{spec['ref']}"
    if check_only:
        return 0, notes
    try:
        blob = download(url)
    except (urllib.error.URLError, TimeoutError, OSError) as exc:
        notes.append(f"{spec['repo']}: download failed ({exc})")
        return 0, notes

    written = 0
    with tarfile.open(fileobj=io.BytesIO(blob), mode="r:gz") as tar:
        # GitHub 的 tarball 顶层是 `<repo>-<ref>/`，所以真正的路径前缀是
        # `<top>/<strip>`。从第一个成员名里取顶层，别硬编码（仓库名与 ref
        # 都可能带 `-`，猜错会静默找不到任何文件——第一版就是这么错的）。
        members = {m.name: m for m in tar.getmembers()}
        top = ""
        for name in members:
            if "/" in name:
                top = name.split("/", 1)[0] + "/"
                break
        if not top:
            notes.append(f"{spec['repo']}: tarball has no top-level directory")
            return 0, notes
        for skill in spec["skills"]:
            prefix = f"{top}{spec['strip']}{skill}/"
            files = [m for n, m in members.items() if n.startswith(prefix) and m.isfile()]
            if not files:
                notes.append(f"{spec['repo']}: skill `{skill}` not found at {prefix}")
                continue
            out_dir = dest_root / skill
            if out_dir.exists():
                shutil.rmtree(out_dir)
            out_dir.mkdir(parents=True)
            for member in files:
                rel = member.name[len(prefix) :]
                target = (out_dir / rel).resolve()
                # 解包路径不得逃出目标目录（tarball 里的 `..` 是经典的
                # 目录穿越；这里来自第三方 tarball，不能假定它干净）。
                if not target.is_relative_to(out_dir.resolve()):
                    notes.append(f"{skill}: refused path escape {rel}")
                    continue
                target.parent.mkdir(parents=True, exist_ok=True)
                extracted = tar.extractfile(member)
                if extracted is None:
                    continue
                target.write_bytes(extracted.read())
            skill_md = out_dir / "SKILL.md"
            if not skill_md.is_file():
                notes.append(f"{skill}: bundle has no SKILL.md")
                shutil.rmtree(out_dir)
                continue
            if normalise(skill_md, skill, notes):
                written += 1
            else:
                shutil.rmtree(out_dir)
    return written, notes


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="verify without writing")
    parser.add_argument("--list", action="store_true", help="print the manifest")
    args = parser.parse_args()

    manifest = load_manifest()

    if args.list:
        total = 0
        for spec in manifest["packs"]:
            print(f"{spec['repo']}@{spec['ref']}  ({spec['license']})")
            print(f"  why: {spec['why']}")
            for skill in spec["skills"]:
                print(f"    - {skill}")
                total += 1
        print(f"\n{total} skills from {len(manifest['packs'])} packs")
        return 0

    if args.check:
        missing = []
        for spec in manifest["packs"]:
            for skill in spec["skills"]:
                if not (DEST / skill / "SKILL.md").is_file():
                    missing.append(skill)
        if missing:
            print(f"agent-skills: {len(missing)} missing -> {', '.join(missing)}")
            print("run: python3 scripts/fetch-agent-skills.py")
            return 1
        print(f"agent-skills: ok ({sum(len(p['skills']) for p in manifest['packs'])} present)")
        return 0

    DEST.mkdir(parents=True, exist_ok=True)
    total, notes = 0, []
    for spec in manifest["packs"]:
        count, pack_notes = fetch_pack(spec, DEST, check_only=False)
        total += count
        notes.extend(pack_notes)
        print(f"  {spec['repo']}: {count}/{len(spec['skills'])} skills")

    # 归属与许可随载荷一起落盘：别人打开 .dsh/skills/ 必须能看出这是谁的东西。
    lines = ["# 第三方 agent 技能（自动生成，勿手改）", "",
             "由 `python3 scripts/fetch-agent-skills.py` 生成；配方在 `dsh/agent-skills.json`。",
             "本目录已在 `.gitignore` 中，不入库。", ""]
    for spec in manifest["packs"]:
        lines.append(f"## {spec['repo']} @ {spec['ref']} — {spec['license']}")
        lines.append("")
        lines.append(spec["why"])
        lines.append("")
        for skill in spec["skills"]:
            lines.append(f"- `{skill}`")
        lines.append("")
    (DEST / "PROVENANCE.md").write_text("\n".join(lines), encoding="utf-8")

    if notes:
        print("\nnormalisation notes:")
        for note in notes:
            print(f"  - {note}")
    print(f"\nagent-skills: {total} skills -> {DEST.relative_to(ROOT)}")
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
