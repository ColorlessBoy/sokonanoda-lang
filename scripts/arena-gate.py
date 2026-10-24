#!/usr/bin/env python3
"""发版前的 Lean Kernel Arena 门禁（2026-10-24 用户要求）。

## 为什么要有它

Lean Kernel Arena 是内核的**外部**判定语料。2026-10 的更新里出现了
`bugs/nested-rec-rule-nfields`（嵌套归纳的 iota 规则谎报字段数 ⇒ 接受一条
`False` 的证明）。这类问题本地单测/课程语料都碰不到：必须在**发版本号之前**
拿**最新**的外部语料跑一遍，否则内核改了、tag 照打。

## 契约（硬规则）

* **语料 = 最新 master**：每次运行都查 `lean-kernel-arena` 的
  `Build and Deploy Site` workflow 最近一次 **master push** 的成功 run，
  下载它的 `test-tarball` artifact（`good/` 接受、`bad/` 拒绝；≤10MB 的
  计分用例）。缓存只按 run id + sha 复用；取不到就 **exit 3**，绝不静默跳过。
* **可选全量**：`LEAN_KERNEL_ARENA` 或 `--full <dir>` 额外跑 arena checkout
  的全量语料（含 >10MB 用例；仍受 `LEAN_KERNEL_ARENA_MAX_BYTES` 的 64MB 上限，
  跳过数会打印）。不设就明确打印 skipped。
* **已知不支持**：`crates/kernel/tests/arena_known_unsupported.txt` 里的条目
  （当前只有 unsafe/partial 两条）必须是 **declined 或 rejected，绝不能是
  accepted**；名单里有一条不在语料里 ⇒ 判负（过期白名单）。
* **自检**：每次先跑一个 2 条用例的临时语料（good + bad），再把 good 放进
  `bad/` 反向验证 —— 反向必须判负，否则说明 gate 咬不住。
* **不调用官方 Lean 工具链**：只下载 lean4export **已经导出好**的 ndjson。

## 用法

    python3 scripts/arena-gate.py                  # 最新 master 语料（release 测试）
    python3 scripts/arena-gate.py --selftest       # 只跑 bite/自检
    python3 scripts/arena-gate.py --corpus <dir>   # 本地 good/bad 语料（调试/离线）
    python3 scripts/arena-gate.py --full <dir>     # 额外跑 LEAN_KERNEL_ARENA 全量
    python3 scripts/arena-gate.py --offline        # 用缓存（打印 NOT LATEST；发版禁用）

## 退出码

* 0 = 自检 + 语料全过（全量未设时明确 skipped）
* 1 = 语料/内核判负（mismatch / 已知项被接受 / 白名单过期）
* 2 = 用法错误
* 3 = 取不到语料 / 构建失败 / 自检没咬住 / 环境缺失
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from pathlib import PurePosixPath

ARENA_REPO = "leanprover/lean-kernel-arena"
ARENA_WORKFLOW = "Build and Deploy Site"
ARTIFACT = "test-tarball"
MAX_EXTRACT_BYTES = 256 * 1024 * 1024
CARGO_TIMEOUT_S = 1800
GH_TIMEOUT_S = 600

# Must stay in sync with the summary line printed by
# `crates/kernel/tests/arena.rs::arena_tarball_tier`.
TARBALL_SUMMARY_RE = re.compile(
    r"arena_tarball_tier: (?P<cases>\d+) cases "
    r"\(accepted (?P<accepted>\d+) · rejected (?P<rejected>\d+) · "
    r"declined (?P<declined>\d+) · timeouts (?P<timeouts>\d+)\) · "
    r"known unsupported (?P<known_unsupported>\d+) · "
    r"known slow (?P<known_slow>\d+) \(timeouts (?P<slow_timeouts>\d+)\)"
)


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def die(code: int, message: str) -> None:
    print(f"arena-gate: ✗ {message}", file=sys.stderr)
    raise SystemExit(code)


def run(
    cmd: list[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    capture: bool = True,
    timeout: int | None = None,
) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            cmd,
            cwd=str(cwd) if cwd else None,
            env=env,
            capture_output=capture,
            text=True,
            timeout=timeout,
        )
    except FileNotFoundError:
        die(3, f"找不到命令 `{cmd[0]}`（先装好 gh / cargo）")
    except subprocess.TimeoutExpired:
        die(3, f"命令超时（{timeout}s）：{' '.join(cmd)}")


def tail(text: str, lines: int = 60) -> str:
    rows = (text or "").strip().splitlines()
    return "\n".join(rows[-lines:])


def build_identity(repo: Path) -> str:
    sha = run(["git", "rev-parse", "--short", "HEAD"], cwd=repo).stdout.strip()
    dirty = bool(run(["git", "status", "--porcelain"], cwd=repo).stdout.strip())
    return f"{sha} ({'dirty' if dirty else 'clean'})"


def latest_run() -> dict:
    if shutil.which("gh") is None:
        die(
            3,
            "找不到 `gh`。发版门禁要求查 lean-kernel-arena 的最新 master run：\n"
            "  brew install gh && gh auth login\n"
            "离线调试用 --corpus <dir>（会打印本地语料来源；发版路径不要用它）。",
        )
    proc = run(
        [
            "gh",
            "run",
            "list",
            "-R",
            ARENA_REPO,
            "--workflow",
            ARENA_WORKFLOW,
            "--branch",
            "master",
            "--event",
            "push",
            "--status",
            "success",
            "--limit",
            "1",
            "--json",
            "databaseId,headSha,createdAt,displayTitle",
        ],
        timeout=GH_TIMEOUT_S,
    )
    if proc.returncode != 0:
        die(3, f"查 {ARENA_REPO} 最新 master run 失败：\n{tail(proc.stderr)}")
    data = json.loads(proc.stdout or "[]")
    if not data:
        die(3, f"{ARENA_REPO} 没有成功的 master push run（workflow={ARENA_WORKFLOW}）")
    return data[0]


def safe_extract(tarball: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    try:
        tf = tarfile.open(tarball, "r:gz")
    except (OSError, tarfile.TarError) as exc:
        die(3, f"打不开语料 tarball {tarball}：{exc}")
    with tf:
        total = 0
        for member in tf.getmembers():
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts:
                die(3, f"语料 tarball 含非法路径：{member.name}")
            if not (member.isfile() or member.isdir()):
                die(3, f"语料 tarball 含链接/特殊文件：{member.name}")
            if not path.parts or path.parts[0] not in ("good", "bad"):
                die(3, f"语料 tarball 含意外顶层条目：{member.name}")
            total += member.size
        if total > MAX_EXTRACT_BYTES:
            die(3, f"语料解压后 {total} 字节，超过 {MAX_EXTRACT_BYTES} 上限")
        try:
            tf.extractall(dest, filter="data")
        except TypeError:  # Python < 3.12
            tf.extractall(dest)


def count_cases(corpus: Path) -> tuple[int, int]:
    good = len(list((corpus / "good").rglob("*.ndjson")))
    bad = len(list((corpus / "bad").rglob("*.ndjson")))
    if good == 0 or bad == 0:
        die(3, f"语料 {corpus} 缺 good/ 或 bad/ 的 .ndjson（good={good}, bad={bad}）")
    return good, bad


def fetch_corpus(run_info: dict, cache: Path, refresh: bool) -> dict:
    run_id = str(run_info["databaseId"])
    sha = run_info["headSha"]
    meta_path = cache / "identity.json"
    if not refresh and meta_path.exists():
        try:
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            meta = {}
        if str(meta.get("run_id")) == run_id and meta.get("sha") == sha:
            return meta

    cache.parent.mkdir(parents=True, exist_ok=True)
    work = cache.parent / f"{cache.name}.work"
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    try:
        proc = run(
            ["gh", "run", "download", run_id, "-R", ARENA_REPO, "-n", ARTIFACT, "-D", str(work)],
            timeout=GH_TIMEOUT_S,
        )
        if proc.returncode != 0:
            die(3, f"下载 arena {ARTIFACT}（run {run_id}）失败：\n{tail(proc.stderr)}")
        tarballs = sorted(work.glob("*.tar.gz"))
        if len(tarballs) != 1:
            die(3, f"artifact {ARTIFACT} 里应有且只有一个 .tar.gz，实际 {[t.name for t in tarballs]}")
        corpus = work / "corpus"
        safe_extract(tarballs[0], corpus)
        good, bad = count_cases(corpus)
        meta = {
            "source": f"{ARENA_REPO} master",
            "workflow": ARENA_WORKFLOW,
            "run_id": run_id,
            "sha": sha,
            "created_at": run_info.get("createdAt"),
            "fetched_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
            "good": good,
            "bad": bad,
        }
        (corpus / "identity.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
        shutil.rmtree(cache, ignore_errors=True)
        corpus.rename(cache)
        return meta
    finally:
        shutil.rmtree(work, ignore_errors=True)


def cargo_test(repo: Path, env_extra: dict[str, str], filter_name: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env.update(env_extra)
    return run(
        [
            "cargo",
            "test",
            "--release",
            "-p",
            "sokonanoda",
            "--test",
            "arena",
            "--locked",
            "--",
            filter_name,
            "--nocapture",
        ],
        cwd=repo,
        env=env,
        timeout=CARGO_TIMEOUT_S,
    )


def selftest(repo: Path) -> None:
    tmp = Path(tempfile.mkdtemp(prefix="soko-arena-selftest-"))
    try:
        corpus = tmp / "corpus"
        (corpus / "good").mkdir(parents=True)
        (corpus / "bad").mkdir()
        empty = repo / "crates/kernel/test_resources/Empty/export"
        forged = repo / "crates/kernel/test_resources/RuleDomainMismatch/export"
        shutil.copyfile(empty, corpus / "good" / "empty.ndjson")
        shutil.copyfile(forged, corpus / "bad" / "forged-rule.ndjson")
        allow = tmp / "known.txt"
        allow.write_text("", encoding="utf-8")
        slow = tmp / "slow.txt"
        slow.write_text("", encoding="utf-8")
        env = {
            "LEAN_KERNEL_ARENA_TARBALL": str(corpus),
            "LEAN_KERNEL_ARENA_KNOWN_UNSUPPORTED": str(allow),
            "LEAN_KERNEL_ARENA_KNOWN_SLOW": str(slow),
            "LEAN_KERNEL_ARENA_TIMEOUT": "60",
        }
        proc = cargo_test(repo, env, "arena_tarball_tier")
        if proc.returncode != 0:
            die(3, "自检正臂没过（good/bad 两条夹具本身跑不通）\n" + tail(proc.stdout + proc.stderr))
        # 咬住证明：把 good 用例放进 bad/ ⇒ 期望 accept、实际 accept？不，期望
        # reject、实际 accept ⇒ 必须判负。若仍通过 ⇒ gate 是安慰剂。
        shutil.copyfile(corpus / "good" / "empty.ndjson", corpus / "bad" / "empty.ndjson")
        bitten = cargo_test(repo, env, "arena_tarball_tier")
        if bitten.returncode == 0:
            die(
                3,
                "自检反臂**没咬住**：把 good 用例放进 bad/ 之后仍然通过 ⇒ 门禁是安慰剂",
            )
        print("arena-gate: selftest PASS（good/bad 正常；反向语料被咬住）")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def run_tarball_tier(repo: Path, corpus: Path) -> dict[str, int]:
    timeout = os.environ.get("SOKO_ARENA_TIMEOUT", "120")
    env = {"LEAN_KERNEL_ARENA_TARBALL": str(corpus), "LEAN_KERNEL_ARENA_TIMEOUT": timeout}
    # 本地 `--corpus` 常常是子集：允许用 SOKO_* 显式给它自己的名单（默认仍是仓库里的
    # 正式名单；官方最新语料路径不要设这两个，否则会绕过过期名单检查）。
    for public, internal in (
        ("SOKO_ARENA_KNOWN_UNSUPPORTED", "LEAN_KERNEL_ARENA_KNOWN_UNSUPPORTED"),
        ("SOKO_ARENA_KNOWN_SLOW", "LEAN_KERNEL_ARENA_KNOWN_SLOW"),
    ):
        value = os.environ.get(public)
        if value:
            env[internal] = value
    proc = cargo_test(repo, env, "arena_tarball_tier")
    output = proc.stdout + proc.stderr
    summary = [line for line in output.splitlines() if line.startswith("arena_tarball_tier:")]
    for line in summary:
        print(f"arena-gate: {line}")
    if proc.returncode != 0:
        die(1, "tarball tier 判负\n" + tail(output, 100))
    if not summary:
        die(3, "tarball tier 没有打印汇总行（测试可能没跑到）\n" + tail(output))
    counts = None
    for line in summary:
        match = TARBALL_SUMMARY_RE.search(line)
        if match:
            counts = {key: int(value) for key, value in match.groupdict().items()}
    if counts is None:
        die(3, f"tarball tier 的汇总行形状不认识（与 Rust 侧不同步？）：\n{tail(output)}")
    return counts


def run_full_tier(repo: Path, full: Path) -> None:
    if not (full / "tests").is_dir() or not (full / "_build/tests").is_dir():
        die(2, f"--full 需要 lean-kernel-arena checkout（含 tests/ 与 _build/tests）：{full}")
    proc = cargo_test(repo, {"LEAN_KERNEL_ARENA": str(full)}, "arena_fast_tier")
    output = proc.stdout + proc.stderr
    summary = [line for line in output.splitlines() if line.startswith("arena_fast_tier:")]
    for line in summary:
        print(f"arena-gate: {line}")
    if proc.returncode != 0:
        die(1, "全量 tier 判负\n" + tail(output, 100))
    if not summary:
        die(3, "全量 tier 没有打印汇总行（测试可能没跑到）\n" + tail(output))


def write_result(repo: Path, meta: dict, counts: dict[str, int]) -> None:
    """Record the official latest-corpus pass for README/site generation.

    Only called for a **downloaded latest** corpus (never `--corpus`/`--offline`),
    and only after both tiers pass. The file is tracked: it is the single source
    of the public credibility block.
    """
    data = {
        "schema": "soko.arena/1",
        "source": meta.get("source", f"{ARENA_REPO} master"),
        "workflow": meta.get("workflow", ARENA_WORKFLOW),
        "revision": meta["sha"],
        "run_id": str(meta["run_id"]),
        "created_at": meta.get("created_at"),
        "fetched_at": meta.get("fetched_at"),
        "good": int(meta["good"]),
        "bad": int(meta["bad"]),
        "cases": counts["cases"],
        "accepted": counts["accepted"],
        "rejected": counts["rejected"],
        "declined": counts["declined"],
        "timeouts": counts["timeouts"],
        "known_unsupported": counts["known_unsupported"],
        "known_slow": counts["known_slow"],
        "slow_timeouts": counts["slow_timeouts"],
        "passed": True,
    }
    path = repo / "site" / "data" / "arena.json"
    text = json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    if path.exists() and path.read_text(encoding="utf-8") == text:
        print(f"arena-gate: {path.relative_to(repo)} unchanged")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    print(f"arena-gate: wrote {path.relative_to(repo)}")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="发版前跑 Lean Kernel Arena 最新语料（不调用官方 Lean 工具链）"
    )
    parser.add_argument("--corpus", type=Path, help="本地 good/bad 语料目录（调试/离线；发版路径不用）")
    parser.add_argument("--full", type=Path, help="额外跑 LEAN_KERNEL_ARENA 全量 checkout")
    parser.add_argument("--offline", action="store_true", help="用缓存语料，不查最新（打印 NOT LATEST）")
    parser.add_argument("--refresh", action="store_true", help="强制重新下载最新 test-tarball")
    parser.add_argument("--selftest", action="store_true", help="只跑自检/咬住证明")
    parser.add_argument(
        "--write-result",
        action="store_true",
        help="通过后写 site/data/arena.json（README/官网的可信度来源；仅最新语料）",
    )
    args = parser.parse_args()

    repo = repo_root()
    print(f"arena-gate: build {build_identity(repo)}")
    selftest(repo)
    if args.selftest:
        return 0

    if args.corpus:
        corpus = args.corpus.resolve()
        if not corpus.is_dir():
            die(2, f"--corpus 不是目录：{corpus}")
        good, bad = count_cases(corpus)
        print(f"arena-gate: corpus local:{corpus} · {good} good / {bad} bad")
    else:
        cache = repo / "target" / "arena" / "cache"
        if args.offline:
            meta_path = cache / "identity.json"
            if not meta_path.exists():
                die(3, "offline 但没有缓存语料（先联网跑一次，或用 --corpus）")
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
            print(
                "arena-gate: ⚠ offline：用缓存语料，**不保证最新** "
                f"(master@{meta.get('sha', '?')[:9]} run {meta.get('run_id')})"
            )
            corpus = cache
        else:
            latest = latest_run()
            meta = fetch_corpus(latest, cache, args.refresh)
            print(
                f"arena-gate: corpus master@{meta['sha'][:9]} run {meta['run_id']} "
                f"({meta.get('created_at', '?')}) · {meta['good']} good / {meta['bad']} bad"
            )
            corpus = cache

    counts = run_tarball_tier(repo, corpus)

    full = args.full
    env_full = os.environ.get("LEAN_KERNEL_ARENA")
    if full is None and env_full:
        full = Path(env_full)
    if full is not None:
        run_full_tier(repo, full.resolve())
    else:
        print(
            "arena-gate: full tier skipped（设 LEAN_KERNEL_ARENA 或 --full 跑 >10MB 的大用例；"
            "tarball 只含 ≤10MB 的计分用例）"
        )

    write_requested = args.write_result or os.environ.get("SOKO_ARENA_WRITE_RESULT") == "1"
    if write_requested:
        if args.corpus or args.offline:
            print(
                "arena-gate: --write-result ignored（本地语料/offline 不是官方最新；"
                "README/官网保留上一次 official 结果）"
            )
        else:
            write_result(repo, meta, counts)

    print("arena-gate: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
