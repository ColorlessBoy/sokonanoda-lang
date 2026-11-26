#!/usr/bin/env python3
"""`source` 可复查的共用检查（SCHEMA.md §2.5）。

`source` 里每段 `文件:行号` 都必须：文件存在（相对 `courses/set-theory`）、
行号在文件范围内、该行非空。**语义核验**（答案能否由该行推出）仍靠人。

被 `verify_batch.py`（草稿批次）与 `validate.py`（全库）共用 —— 同一份口径，
避免两处实现漂移（2026-11-24 审计：全库曾有 6 道题 source 不合规，批次检查
却只覆盖当批）。
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent
COURSE_ROOT = ROOT.parent  # courses/set-theory（source 里的 lib/、units/ 相对它）
SOURCE_RE = re.compile(r"([A-Za-z0-9_][A-Za-z0-9_./-]*\.sokonanoda):(\d+)")


def check_source(raw_source, problems, qid):
    """把不合规项追加到 problems；返回是否全部可核验。"""
    text = str(raw_source or "")
    matches = SOURCE_RE.findall(text)
    if not matches:
        problems.append(qid + ": source 缺少 `文件:行号` 格式：" + repr(text[:120]))
        return False
    ok = True
    for rel, line_text in matches:
        candidate = (COURSE_ROOT / rel).resolve()
        if not candidate.exists():
            problems.append(qid + ": source 文件不存在：" + rel)
            ok = False
            continue
        try:
            lines = candidate.read_text(encoding="utf-8").splitlines()
        except Exception as error:  # noqa: BLE001 - 只报问题，不中断检查
            problems.append(qid + ": source 文件读不了：" + rel + "（" + str(error) + "）")
            ok = False
            continue
        line_no = int(line_text)
        if line_no < 1 or line_no > len(lines):
            problems.append(qid + ": source 行号越界：" + rel + ":" + str(line_no)
                            + "（文件共 " + str(len(lines)) + " 行）")
            ok = False
            continue
        if not lines[line_no - 1].strip():
            problems.append(qid + ": source 指向空行：" + rel + ":" + str(line_no))
            ok = False
    return ok
