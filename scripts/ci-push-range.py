#!/usr/bin/env python3
"""`changes` job 的**推送范围基点** —— 洞 `PUSH-RANGE-HIDES-RELEASE`（2026-10-15，high）。

## 病根（实测，不是推演）
`dorny/paths-filter@v4` 在 `push` 事件上按**本次推送自己的范围**算 changed files：
`src/main.ts:144` 取 `payload.before`；`src/main.ts:168-196` 在 `base == head`（都是分支短名
`main`）时用 `beforeSha` ⇒ `git diff <before>..<sha>`（`src/git.ts:56`）。
⇒ **纯 docs 推送把「还没被任何一轮 CI 验过的祖先」从视野里藏掉** ✗。实测链条：

* 发版推送 `cb8edd77`（动 `Cargo.toml`/`Cargo.lock`/`courses/**`）那一轮重活**全排上** ✓
  （run `38662022022`：31 个 job，29 success），但它 **①** 红在 `ledger (1)`（G-25 探针，
  00:51:10）**②** 00:56:36 被 cancel ⇒ **没有任何一轮绿过它** ✗；
* 紧随其后的**纯 docs** 推送（`c11cbc10`，run `38663715016`）按**自己的范围**算 ⇒
  `rust/courses` 全 false ✗ ⇒ `gates-fast`/`ledger`/`contract`/`test`/`e2e`/`gates-course`
  全 **skipped** ✗（**19 个 job 只跑 7 个** ✓）⇒ `auto-tag` **从不触发** ✗ ——
  它的 `needs` 里有 skipped，而 skip **沿依赖链传播**（机制见 `ci.yml` 里 `e2e-ledger`
  的注释，2026-09-26 实验证实 ✓）。
* 同形**第二次**实测（更干净的一对）：`84f6233b`（真修复，run `38657810157`，9 个 job 被
  **cancelled**）被 30 秒后 `8e92cd6d`（纯 docs，run `38657859995`，**7 success + 12 skipped**）
  顶掉 ⇒ 那个修复**从未被验证** ✗。

⇒ `v0.91.0` 发不出去（远端 tag 最新 `v0.90.0`），而 `main` 上零 cargo 的人连 `soko` 都跑不起来
（版本钉 0.91.0 · 缓存标记 0.90.0 ⇒ stale ⇒ 拒绝运行 ✗）。
**同族第四次**：`scripts/**` → `.github/workflows/**` → `docs/gaps/**` → **push 范围** ✓，
共同病根都是「**改了判据却没人验**」✓。

## 修法（A）：基点 = **上一个已验证点**（不是「上一次推送」）
本脚本只做一件事：**算出 `base`**，交给 `dorny/paths-filter` 的 `base:` 入参
（`src/main.ts:32` 有这个入参 ✓；给了 40 位 sha 就走 `git.getChanges(base, head)` ✓）。
**分类（`rust`/`courses`/`editor`）仍然只有一份实现** = `ci.yml` 的 `filters:` 块 ✓ ——
本脚本在 `--explain`/`--selftest` 里**读同一份**做判据（`load_filters()`），不另立一份 ✗。

取基点的顺序（**每一步都朝「更保守」退**，绝不朝「更少验证」退 ✗）：

| # | 来源 | 含义 | 何时用 |
|---|------|------|--------|
| ① | `.ci-verified/sha`（`actions/cache` 标记）| **上一个已验证点**：重活真跑过且全绿的那一轮的 sha | 常态 |
| ② | 最近的 `vX.Y.Z` tag（`git ls-remote --tags --refs`）| 最近一次发版 = 最近一个已验证点 | 标记还没写过（本修复落地后的**第一轮** ✓）/ 缓存被淘汰 |
| ③ | 仓库根提交（必要时 `git fetch --unshallow`）| 还没有任何已验证点 ⇒ **保守** | 无 tag 无标记的仓库 |
| ④ | `github.event.before`（`--before`）| **旧口径**，只给反向验证当负对照 | 判据自检 |

⚠ **为什么不是「上一次成功的 run」** ✗：本修复落地前，纯 docs 轮的「绿」是**假绿** ——
它只说明那一推**自己**的范围干净，不说明祖先被验过 ⇒ 拿它当基点会把发版那批**继续藏掉** ✗
（实测：`c11cbc10` 的 run 是 success ✓ 而发版那批仍未验 ✗）。
标记只由 `ci.yml` 的 `mark-verified` job 在「重活真跑过且全绿」时写 ⇒ 语义上才是已验证点 ✓。

⚠ **首推（`before` 全 0）交回 dorny 自己的 `NULL_SHA` 分支** ✓（`src/main.ts:180-192`：
「Initial push detected - all files will be listed as added」✓）—— 那时没有「上一个已验证点」，
而 dorny 的这条分支**本来就保守**（全部文件都算 added ✓）。

## 判据
* `--selftest`：在**合成夹具仓库**上逐条跑（含**正对照**：连续两次纯 docs 推送 ⇒ 仍不跑重活 ✓；
  含**负对照**：旧口径下同一推送 rust=false ⇒ 洞复现 ✓）；用**真** `ci.yml` 的 filters 判分类 ✓。
* `--check-wiring`：咬 `ci.yml` 的接线本身（`changes` 真的把算出来的 `base` 传给了
  `dorny/paths-filter` ✗✓；标记的 save/restore 前缀一致 ✗✓；写标记的 job 的 `if` 带状态函数 ✓）。
* `--cross-check`：CI 里**对账** —— 本脚本（可本地单测的那份）的读数 vs dorny 的读数；
  **危险方向**（脚本说重活该跑、dorny 说干净 ⇒ 洞还在 ✗）**判红** ✗，
  安全方向（脚本干净、dorny 脏 ⇒ 只是多跑）只 `::warning::` ✓。

退出码：0 = 通过 / 1 = 判红 / 2 = 用法或环境错（与 `scripts/ci-yml-lint.py` 同口径 ✓）。
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_FILTERS = HERE.parent / ".github" / "workflows" / "ci.yml"
DEFAULT_MARKER = ".ci-verified/sha"
CACHE_PREFIX = "ci-verified-marker-"
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
TAG_RE = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")
NULL_SHA = "0" * 40


class EnvError(Exception):
    """用法/环境错（exit 2）——与「判红」（exit 1）分开，别把环境问题当结论 ✓。"""


# ───────────────────────────── git ─────────────────────────────

def git(repo, *args, timeout=60, allow_fail=False):
    """在 `repo` 里跑 git。⚠ **清掉 `GIT_DIR` 一族**（CI 的 checkout 可能留着，
    那会让「夹具仓库」的 git 命令打到**外层仓库**上 ✗ —— 夹具污染 = 判据污染 ✓）。"""
    env = {k: v for k, v in os.environ.items()
           if k not in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE",
                        "GIT_OBJECT_DIRECTORY", "GIT_COMMON_DIR", "GIT_ALTERNATE_OBJECT_DIRECTORIES")}
    env["GIT_TERMINAL_PROMPT"] = "0"  # 不许在 CI 里挂住等凭据 ✓
    try:
        p = subprocess.run(["git", "-C", str(repo), *args], capture_output=True,
                           text=True, timeout=timeout, env=env)
    except subprocess.TimeoutExpired:
        if allow_fail:
            return subprocess.CompletedProcess([], 124, "", f"timeout after {timeout}s")
        raise EnvError(f"git {' '.join(args)} 超时（{timeout}s）✗")
    if p.returncode != 0 and not allow_fail:
        raise EnvError(f"git {' '.join(args)} 失败（rc={p.returncode}）：{p.stderr.strip()[:300]}")
    return p


def peel(repo, sha):
    """`<sha>^{commit}` ⇒ commit sha（tag 对象也 peel ✓）；本地没有就 None。"""
    if not sha or not SHA_RE.match(sha):
        return None
    p = git(repo, "rev-parse", "--verify", "--quiet", f"{sha}^{{commit}}", allow_fail=True)
    return p.stdout.strip() if p.returncode == 0 and p.stdout.strip() else None


def ensure_commit(repo, sha, fetch=True):
    """本地能用就返回 commit sha；否则按 **dorny 的同一条路**取回：
    `git fetch --depth=1 --no-tags origin <sha>`（`src/git.ts:269-289` 的 `ensureRefAvailable` ✓）。
    ⚠ 实测（本机对真远端 `ColorlessBoy/sokonanoda-lang`）：`v0.90.0` 的 commit 与 `cb8edd77`
    都能按 sha 取回 rc=0 ✓（GitHub 供任意**可达** sha ✓）⇒ 浅克隆（checkout 默认 depth=1）
    里也能拿到「上一个已验证点」的对象 ✓。"""
    got = peel(repo, sha)
    if got:
        return got
    if not fetch:
        return None
    remotes = git(repo, "remote", allow_fail=True)
    if "origin" not in remotes.stdout.split():
        return None  # 夹具仓库没有远端 ⇒ 别去 fetch（免得把噪声当读数 ✓）
    p = git(repo, "fetch", "--depth=1", "--no-tags", "origin", sha, timeout=180, allow_fail=True)
    if p.returncode != 0:
        return None
    return peel(repo, sha)


def is_shallow(repo):
    p = git(repo, "rev-parse", "--is-shallow-repository", allow_fail=True)
    return p.returncode == 0 and p.stdout.strip() == "true"


def last_tag(repo):
    """最近的 `vX.Y.Z` tag ⇒ `(name, sha|None)`。`ls-remote` 优先（浅克隆里没有 tag ✗），
    失败退回本地 tag（老仓库/离线 ✓）。"""
    cands = []
    p = git(repo, "ls-remote", "--tags", "--refs", "origin", timeout=60, allow_fail=True)
    if p.returncode == 0 and p.stdout.strip():
        for line in p.stdout.splitlines():
            parts = line.split()
            if len(parts) == 2:
                cands.append((parts[1].replace("refs/tags/", ""), parts[0]))
    else:
        p = git(repo, "tag", "--list", "v*", allow_fail=True)
        cands = [(n.strip(), None) for n in p.stdout.splitlines() if n.strip()]
    best = None
    for name, sha in cands:
        m = TAG_RE.match(name)
        if not m:
            continue  # 只认纯 x.y.z（仓库纪律：版本号只能纯 x.y.z ✓）
        key = tuple(int(x) for x in m.groups())
        if best is None or key > best[0]:
            best = (key, name, sha)
    if best is None:
        return None, None
    _, name, sha = best
    if sha is None:
        p = git(repo, "rev-parse", f"{name}^{{commit}}", allow_fail=True)
        sha = p.stdout.strip() if p.returncode == 0 else None
    return name, sha


def root_commit(repo):
    """仓库根提交（保守兜底）。⚠ **必须确认拿到的是真根** ✗：浅克隆里
    `rev-list --max-parents=0` 会返回**浅边界**（depth=1 时就是 HEAD 自己 ✗ ⇒ 空范围 ⇒
    「全都不跑」✗✗）⇒ 先 `--unshallow`，仍浅就**放弃**（返回 None ✓）。"""
    if is_shallow(repo):
        git(repo, "fetch", "--unshallow", timeout=180, allow_fail=True)
        if is_shallow(repo):
            return None
    p = git(repo, "rev-list", "--max-parents=0", "HEAD", allow_fail=True)
    if p.returncode != 0:
        return None
    lines = [l.strip() for l in p.stdout.splitlines() if l.strip()]
    return lines[-1] if lines else None


def read_marker(path):
    """标记文件里第一个 40 位 sha（坏值 ⇒ 空 ⇒ 走保守退路 ✓，绝不把坏值喂给 git ✗）。"""
    try:
        text = Path(path).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""
    for line in text.splitlines():
        line = line.strip()
        if SHA_RE.match(line):
            return line
    return ""


# ─────────────────────── 范围 + 分类 ───────────────────────

def resolve_base(repo, head, marker_path, before=None):
    """⇒ `(base, why)`。`base` 为空字符串 = 交回 dorny 自己的事件分支（首推 ✓）。"""
    if before and SHA_RE.match(before) and before == NULL_SHA:
        return "", "initial-push(首推：交回 dorny 的 NULL_SHA 分支 ⇒ 全部文件按 added ✓)"
    m = read_marker(marker_path)
    if m:
        got = ensure_commit(repo, m)
        if got:
            return got, "marker(上一个已验证点：重活真跑过且全绿的那一轮 ✓)"
        print(f"::warning::标记 {m} 取不回来（对象不在且 fetch 失败）⇒ 退到 tag ✓", file=sys.stderr)
    name, sha = last_tag(repo)
    if sha:
        got = ensure_commit(repo, sha)
        if got:
            return got, f"tag(最近一次发版 {name} = 最近一个已验证点 ✓)"
        print(f"::warning::tag {name} 取不回来 ⇒ 继续退 ✓", file=sys.stderr)
    root = root_commit(repo)
    if root:
        print("::warning::没有已验证点标记、也没有 tag ⇒ 退到**仓库根提交**（保守）✓；"
              "⚠ 根提交**自身引入**的文件不在 `git diff` 范围里 ✗（边界见脚本文档）", file=sys.stderr)
        return root, "root(无标记无 tag ⇒ 保守：从根提交算 ✓)"
    if before and SHA_RE.match(before):
        print("::warning::连根提交都拿不到 ⇒ 退回**旧口径** `before..sha` ✗（保守性不足，"
              "只该发生在浅克隆 + 无 tag + 无标记的仓库）", file=sys.stderr)
        return before, "before(兜底·⚠ 保守性不足)"
    raise EnvError("解析不出基点 ✗（没有标记 / 没有 tag / 没有 before）")


def changed_files(repo, base, head):
    """`--no-renames` 与 dorny 一致 ✓（`src/git.ts:65`：rename 会被拆成 delete+add ✓，
    否则「从 crates/ 改名出去」会被算成只碰了非 rust 路径 ✗）。"""
    if not base:
        p = git(repo, "ls-files", "-z", timeout=120)
        return [f for f in p.stdout.split("\0") if f]
    p = git(repo, "diff", "--no-renames", "--name-only", "-z", f"{base}..{head}", timeout=120)
    return [f for f in p.stdout.split("\0") if f]


def parse_filters_block(text):
    """`filters: |` 的内联块 ⇒ `{组名: [模式]}`（唯一一份清单在 `ci.yml` ✓）。"""
    out, cur = {}, None
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        m = re.match(r"^([A-Za-z_][\w-]*):\s*$", line)
        if m:
            cur = m.group(1)
            out.setdefault(cur, [])
            continue
        m = re.match(r"^-\s*[\"']?(.+?)[\"']?\s*$", line)
        if m and cur:
            out[cur].append(m.group(1))
    return out


def load_filters(path):
    """从真 `ci.yml` 读**唯一一份**过滤器清单 ✓（不另立一份 ✗）。"""
    try:
        import yaml
    except ImportError:  # pragma: no cover
        raise EnvError("需要 pyyaml（与 scripts/ci-yml-lint.py 同款依赖）✗")
    try:
        doc = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    except Exception as exc:  # noqa: BLE001
        raise EnvError(f"读不了 {path}：{exc}")
    for job in (doc.get("jobs") or {}).values():
        if not isinstance(job, dict):
            continue
        for step in job.get("steps") or []:
            if isinstance(step, dict) and "dorny/paths-filter" in str(step.get("uses", "")):
                raw = (step.get("with") or {}).get("filters")
                if isinstance(raw, str) and "\n" in raw:
                    return parse_filters_block(raw)
                raise EnvError(f"{path} 的 filters 不是内联块（本脚本只支持内联 ✓）")
    raise EnvError(f"{path} 里找不到 dorny/paths-filter 的 filters ✗")


def glob_to_regex(pat):
    """picomatch 形状（只覆盖本仓库用到的形状 ✓）：`**` 跨 `/`，`*`/`?` 不跨 ✓。"""
    out, i, n = [], 0, len(pat)
    while i < n:
        c = pat[i]
        if c == "*":
            if i + 1 < n and pat[i + 1] == "*":
                i += 2
                if i < n and pat[i] == "/":
                    i += 1
                    out.append("(?:.*/)?")
                else:
                    out.append(".*")
                continue
            out.append("[^/]*")
        elif c == "?":
            out.append("[^/]")
        else:
            out.append(re.escape(c))
        i += 1
    return re.compile("^" + "".join(out) + "$")


def classify(files, filters):
    """⇒ `(flags, matched)`；`flags[组]` = 该组有没有命中（= dorny 的 `outputs.<组>` ✓）。"""
    flags, matched = {}, {}
    for name, pats in filters.items():
        regexes = [glob_to_regex(p) for p in pats]
        hit = [f for f in files if any(r.match(f) for r in regexes)]
        flags[name] = bool(hit)
        matched[name] = hit
    return flags, matched


def evaluate(repo, head, marker_path, filters, before=None, legacy=None):
    if legacy:
        # **旧口径**（`before..sha`）：只给反向验证/负对照用 —— 它正是本洞的形状 ✗
        base, why = legacy, "before(旧口径：按**本次推送自己的范围** before..sha 算 ✗)"
    else:
        base, why = resolve_base(repo, head, marker_path, before=before)
    files = changed_files(repo, base, head)
    flags, matched = classify(files, filters)
    return {"base": base, "why": why, "files": files, "flags": flags, "matched": matched}


# ─────────────────────── 接线判据（--check-wiring）───────────────────────

def check_wiring(path):
    """咬 `ci.yml` 的接线本身（**改了判据却没人验**的解药 ✓）。返回 `[(ok, 说明)]`。"""
    import yaml
    doc = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    jobs = doc.get("jobs") or {}
    changes = jobs.get("changes") or {}
    steps = changes.get("steps") or []
    res = []

    def find(pred):
        return [s for s in steps if isinstance(s, dict) and pred(s)]

    pf = find(lambda s: "dorny/paths-filter" in str(s.get("uses", "")))
    res.append((len(pf) == 1, f"changes job 恰好一个 dorny/paths-filter step（实测 {len(pf)} 个）"))
    if len(pf) != 1:
        return res
    pf = pf[0]
    base_expr = str((pf.get("with") or {}).get("base", ""))
    m = re.search(r"steps\.([A-Za-z_][\w-]*)\.outputs\.base", base_expr)
    res.append((bool(m), f"paths-filter 的 `base:` 引用了某个 step 的输出（实测 `{base_expr}`）"))
    if not m:
        return res
    sid = m.group(1)
    producer = [s for s in steps if isinstance(s, dict) and str(s.get("id", "")) == sid]
    res.append((len(producer) == 1, f"`base` 的生产者 step `{sid}` 存在（实测 {len(producer)} 个）"))
    if len(producer) != 1:
        return res
    run = str(producer[0].get("run", ""))
    res.append(("ci-push-range.py" in run,
                f"生产者 step 跑的是本脚本（实测 run=`{run.strip()[:60]}`）"))

    # 标记：restore 的前缀必须与 save 的 key 前缀一致（漂移 ⇒ 标记永远取不到 ⇒ 每轮都跑重活 ✗）
    restores = find(lambda s: "actions/cache/restore" in str(s.get("uses", "")))
    prefixes = []
    for s in restores:
        rk = str((s.get("with") or {}).get("restore-keys", ""))
        prefixes += [l.strip() for l in rk.splitlines() if l.strip().startswith(CACHE_PREFIX)]
    res.append((bool(prefixes), f"changes job 里有一条 `{CACHE_PREFIX}` 的 restore-keys（实测 {prefixes}）"))
    saves = []
    for jid, job in jobs.items():
        if not isinstance(job, dict):
            continue
        for s in job.get("steps") or []:
            if isinstance(s, dict) and "actions/cache/save" in str(s.get("uses", "")):
                key = str((s.get("with") or {}).get("key", ""))
                if key.startswith(CACHE_PREFIX):
                    saves.append((jid, key, job.get("if", "")))
    res.append((len(saves) == 1, f"恰好一个 job 写 `{CACHE_PREFIX}` 标记（实测 {[s[0] for s in saves]}）"))
    if len(saves) == 1 and prefixes:
        jid, key, jif = saves[0]
        res.append((key.startswith(prefixes[0]),
                    f"save 的 key（`{key}`）以 restore 的前缀（`{prefixes[0]}`）开头 ✓"))
        res.append((any(fn in str(jif) for fn in ("always()", "!cancelled()", "success()", "failure()")),
                    f"写标记的 job `{jid}` 的 `if` 带状态函数 ✓（needs 里有 skipped 时不许被拖成 skipped ✗）"))
    return res


# ─────────────────────── 判据自检（--selftest）───────────────────────

def _fixture(tmp, tag=True):
    """合成夹具：c0(tag) → c1 docs → c2 **rust（发版那批）** → c3 docs → c4 docs → c5 rust → c6 docs
    + 一条 side 分支（非祖先）s1 + c7 courses。形状照抄实测链条 ✓。"""
    repo = Path(tmp) / "fixture"
    repo.mkdir()

    def write(rel, text):
        p = repo / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text, encoding="utf-8")

    def commit(msg):
        git(repo, "add", "-A")
        git(repo, "-c", "user.name=t", "-c", "user.email=t@example.com",
            "commit", "-q", "-m", msg)
        return git(repo, "rev-parse", "HEAD").stdout.strip()

    git(repo, "init", "-q", "-b", "main")
    write("README.md", "r\n")
    write("Cargo.toml", 'version = "0.1.0"\n')
    c0 = commit("c0: 初始（rust：Cargo.toml）")
    if tag:
        git(repo, "tag", "v0.1.0")
    write("notes/note.md", "n\n")
    c1 = commit("c1: docs")
    write("Cargo.toml", 'version = "0.2.0"\n')
    write("crates/k.rs", "// k\n")
    c2 = commit("c2: 发版那批（rust）")
    write("docs/e2e/ledger.jsonl", "{}\n")
    c3 = commit("c3: docs（bot 那一类）")
    write("notes/queue.md", "q\n")
    c4 = commit("c4: docs")
    write("crates/k2.rs", "// k2\n")
    c5 = commit("c5: rust")
    write("notes/after.md", "a\n")
    c6 = commit("c6: docs")
    write("courses/u.md", "u\n")
    c7 = commit("c7: courses")
    git(repo, "checkout", "-q", "-b", "side", c1)
    write("crates/side.rs", "// side\n")
    s1 = commit("s1: side 分支上的 rust（非 main 祖先）")
    git(repo, "checkout", "-q", "main")
    return repo, {"c0": c0, "c1": c1, "c2": c2, "c3": c3, "c4": c4,
                  "c5": c5, "c6": c6, "c7": c7, "s1": s1}


def selftest():
    """逐条判据（**含正/负对照** ✓）。返回 `(ok, 行列表)`。"""
    filters = load_filters(DEFAULT_FILTERS)  # 用真 ci.yml 的清单 ✓
    tmp = tempfile.mkdtemp(prefix="ci-push-range-")
    lines, ok = [], True
    try:
        repo, c = _fixture(tmp)
        marker = Path(tmp) / "marker"
        cases = [
            # (名字, from, marker 内容, before, 期望 rust/courses/editor, 期望 why 关键字)
            ("修复后：纯 docs 推送把**未验证的祖先**（c2 发版那批）算进来",
             c["c3"], None, None, (True, False, False), "tag"),
            ("正对照①：标记=已验证点 c2 ⇒ **第一次**纯 docs 仍不跑重活",
             c["c3"], c["c2"], None, (False, False, False), "marker"),
            ("正对照②：**连续第二次**纯 docs（c4）仍不跑重活 ✓（最容易做错的一处）",
             c["c4"], c["c2"], None, (False, False, False), "marker"),
            ("标记=c2 时，真 rust 推送（c5）照跑重活 ✓",
             c["c5"], c["c2"], None, (True, False, False), "marker"),
            ("标记推进到 c5（那一轮绿了）⇒ 之后的纯 docs 又干净 ✓",
             c["c6"], c["c5"], None, (False, False, False), "marker"),
            ("标记失效（对象不存在）⇒ 保守退到 tag（不是静默变干净 ✗）",
             c["c3"], "1" * 40, None, (True, False, False), "tag"),
            ("标记 == 本次 sha（重跑已验证的 sha）⇒ 空范围 ⇒ 不跑 ✓",
             c["c6"], c["c6"], None, (False, False, False), "marker"),
            ("非祖先标记（main 被 rebase 的形状）⇒ 保守：不静默藏掉 c2 的 rust ✓",
             c["c6"], c["s1"], None, (True, False, False), "marker"),
            ("courses-only 推送在范围内 ⇒ courses=true 而 rust=false ✓",
             c["c7"], c["c6"], None, (False, True, False), "marker"),
            ("**负对照（旧口径）**：同一推送按 `before..sha` 算 ⇒ rust=false ⇒ **洞复现** ✗",
             c["c3"], None, None, (False, False, False), "before", c["c2"]),
        ]
        for case in cases:
            name, frm, mk, before, want, why_kw = case[:6]
            legacy = case[6] if len(case) > 6 else None
            if mk is None:
                marker.unlink(missing_ok=True)
            else:
                marker.write_text(mk + "\n", encoding="utf-8")
            got = evaluate(repo, frm, marker, filters, before=before, legacy=legacy)
            flags = tuple(got["flags"].get(k, False) for k in ("rust", "courses", "editor"))
            good = flags == want and why_kw in got["why"]
            ok &= good
            lines.append(f"  {'✓' if good else '✗'} {name}\n"
                         f"      base={got['base'][:8] or '(空)'} why={got['why'][:52]} "
                         f"rust/courses/editor={flags} 期望={want}")

        # ③ 无 tag 无标记（保守退路）：另一棵树，不做 tag
        repo2 = Path(tmp) / "notag"
        repo2.mkdir()
        (repo2 / "Cargo.toml").write_text("v\n", encoding="utf-8")
        git(repo2, "init", "-q", "-b", "main")
        git(repo2, "add", "-A")
        git(repo2, "-c", "user.name=t", "-c", "user.email=t@example.com", "commit", "-q", "-m", "a0")
        a0 = git(repo2, "rev-parse", "HEAD").stdout.strip()
        (repo2 / "notes-x.md").write_text("x\n", encoding="utf-8")
        git(repo2, "add", "-A")
        git(repo2, "-c", "user.name=t", "-c", "user.email=t@example.com", "commit", "-q", "-m", "a1 docs")
        a1 = git(repo2, "rev-parse", "HEAD").stdout.strip()
        got = evaluate(repo2, a1, Path(tmp) / "none", filters)
        good = got["base"] == a0 and "root" in got["why"]
        ok &= good
        lines.append(f"  {'✓' if good else '✗'} 无标记无 tag ⇒ 退到仓库根提交（保守 ✓，不是浅边界 ✗）"
                     f"\n      base={got['base'][:8]}（期望 {a0[:8]}）why={got['why'][:40]}")

        # ④ 首推（before 全 0）⇒ 交回 dorny 的 NULL_SHA 分支（base 为空 ✓）
        got = evaluate(repo, c["c3"], Path(tmp) / "none", filters, before=NULL_SHA)
        good = got["base"] == "" and "initial-push" in got["why"] and got["flags"]["rust"]
        ok &= good
        lines.append(f"  {'✓' if good else '✗'} 首推（before 全 0）⇒ base 交回 dorny（空）且全文件算 added ✓"
                     f"\n      base='{got['base']}' why={got['why'][:40]} rust={got['flags']['rust']}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return ok, lines


# ─────────────────────── CI 对账（--cross-check）───────────────────────

def cross_check():
    """脚本读数 vs dorny 读数。**危险方向判红** ✓（脚本说该跑、dorny 说干净 ⇒ 洞还在 ✗）。"""
    groups = ("rust", "courses", "editor")
    ours = {g: os.environ.get(f"SOKO_RANGE_{g.upper()}", "").strip() for g in groups}
    theirs = {g: os.environ.get(f"SOKO_DORNY_{g.upper()}", "").strip() for g in groups}
    if any(v not in ("true", "false") for v in list(ours.values()) + list(theirs.values())):
        raise EnvError(f"对账输入不齐：脚本={ours} dorny={theirs}（都要 true/false）✗")
    bad = [g for g in groups if ours[g] == "true" and theirs[g] == "false"]
    extra = [g for g in groups if ours[g] == "false" and theirs[g] == "true"]
    print(f"对账：脚本={ours} · dorny={theirs}")
    for g in extra:
        print(f"::warning::`{g}`：脚本判干净、dorny 判脏 ⇒ 多跑重活（安全方向 ✓，但两份读数已漂移 ✗）")
    for g in bad:
        print(f"::error::`{g}`：脚本判**该跑重活**、dorny 判**干净** ⇒ 推送范围仍然把未验证的改动"
              f"藏掉了 ✗✗（正是本洞的形状）—— 查 `scripts/ci-push-range.py` 与 ci.yml 的接线")
    return 1 if bad else 0


# ─────────────────────── CLI ───────────────────────

def main(argv=None):
    ap = argparse.ArgumentParser(description="changes job 的推送范围基点（PUSH-RANGE-HIDES-RELEASE）")
    ap.add_argument("--repo", default=".", help="仓库路径（默认当前目录）")
    ap.add_argument("--from", dest="head", default="HEAD", help="本次推送的 sha（默认 HEAD）")
    ap.add_argument("--marker", default=DEFAULT_MARKER, help=f"已验证点标记（默认 {DEFAULT_MARKER}）")
    ap.add_argument("--before", default=None,
                    help="事件里的 before sha（只用来识别**首推**：全 0 ⇒ 交回 dorny ✓）")
    ap.add_argument("--legacy", default=None, metavar="SHA",
                    help="**旧口径**（before..sha）当基点 —— 只给反向验证/负对照用 ✗")
    ap.add_argument("--filters", default=str(DEFAULT_FILTERS), help="过滤器清单来源（默认真 ci.yml）")
    ap.add_argument("--explain", action="store_true", help="把范围里的文件与命中列到 stderr ✓")
    ap.add_argument("--json", action="store_true", help="JSON 到 stdout（不打印 GITHUB_OUTPUT 行）")
    ap.add_argument("--selftest", action="store_true", help="夹具仓库上的判据自检（含正/负对照）")
    ap.add_argument("--check-wiring", action="store_true", help="咬 ci.yml 的接线（判据不漂移）")
    ap.add_argument("--cross-check", action="store_true", help="CI 对账：脚本读数 vs dorny 读数")
    args = ap.parse_args(argv)

    if args.selftest:
        ok, lines = selftest()
        print("推送范围基点判据自检（scripts/ci-push-range.py --selftest）")
        print("\n".join(lines))
        print(("✓ 全部通过" if ok else "✗ 有判据不成立"))
        return 0 if ok else 1
    if args.check_wiring:
        res = check_wiring(args.filters)
        print(f"接线判据（{args.filters}）")
        for good, msg in res:
            print(f"  {'✓' if good else '✗'} {msg}")
        ok = all(g for g, _ in res)
        print("✓ 接线正确" if ok else "✗ 接线漂移")
        return 0 if ok else 1
    if args.cross_check:
        return cross_check()

    filters = load_filters(args.filters)
    head = git(args.repo, "rev-parse", f"{args.head}^{{commit}}").stdout.strip()
    got = evaluate(args.repo, head, args.marker, filters, before=args.before, legacy=args.legacy)
    if args.explain:
        print(f"base={got['base'] or '(空：交回 dorny 事件分支)'}  why={got['why']}", file=sys.stderr)
        print(f"范围里的文件 {len(got['files'])} 个：", file=sys.stderr)
        for f in got["files"][:40]:
            print(f"  {f}", file=sys.stderr)
        for g, hits in got["matched"].items():
            print(f"  [{g}] {len(hits)} 个命中：{hits[:6]}", file=sys.stderr)
    if args.json:
        print(json.dumps({"base": got["base"], "why": got["why"],
                          "flags": got["flags"], "n_files": len(got["files"])},
                         ensure_ascii=False))
        return 0
    print(f"base={got['base']}")
    print(f"why={got['why']}")
    for g in filters:
        print(f"{g}={'true' if got['flags'][g] else 'false'}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except EnvError as exc:
        print(f"✗ {exc}", file=sys.stderr)
        sys.exit(2)
