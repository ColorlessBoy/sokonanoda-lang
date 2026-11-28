# 会话预检 lesson-preflight（开始做题前自动跑，issue #151）

出题前的内容门禁：**不发坏题**。fast path 的 `开始做题 / 下一题` 由
`dsh/plugins/sokonanoda-dsh/lib/quiz-proxy.mjs` 接本地引擎出题；出题前它会对
「即将发出的这道题」跑 `scripts/lesson-preflight.py`，blocker 的题直接跳过
（记为 `skipped`）并写内部日志 `preflight-blocked <qid>: <原因>`；连续 5 道都不过
⇒ **暂停出题**，给学习者一行可读提示（不崩、不发坏题）。

## 判定口径

| 检查 | 来源 | 级别 |
| --- | --- | --- |
| X11 自造术语（`落点` / `原序对` / `全称命题`，词表可配置） | `text_lint.py` 的 question 模式（#149） | **blocker** |
| X6 段首代词 / X7 空洞评价 / X12 隐喻 / X13 选项中文 >24 字 | 同上 | warning |
| 选项里点名的引理没出现在**实际推送卡**里 | `card-lint.py`（#146）的 `(qid,name)` 判据 | **blocker** |
| 无卡兜底缺题目 `why` / 缺选项点名的引理名 | `engine.explain_for` 的 fallback-card | **blocker** |

**blocker = 内容错，必须先修**；warning 只提示（按 `text_lint` 自己的基线只减不增）。
预检**只读**：state 一律 deepcopy 后走 `engine.next_step`，绝不写回学习者的 state。

## 用法

```bash
# learner path：看学习者接下来 3 题（默认 limit=3）
python3 scripts/lesson-preflight.py
python3 scripts/lesson-preflight.py --limit 22 --json   # 看接下来 22 题 + 机读输出
python3 scripts/lesson-preflight.py --state S.json --chapter I.9

# content 模式：全库扫描（不依赖 state）
python3 scripts/lesson-preflight.py --all
python3 scripts/lesson-preflight.py --all --no-baseline          # 列出全部 blocker
python3 scripts/lesson-preflight.py --all --update-baseline      # 收紧基线

# 红/绿对照（改判据必跑；gate/CI 接线后也跑它）
python3 scripts/lesson-preflight.py --selftest
```

- **state 路径**（learner 模式）：优先 `--state`，其次 `$LESSON_PREFLIGHT_STATE`，
  再 `~/.dsh/state/sokonanoda-dsh/quiz/default.json`（DSH fast path 的真实 state），
  最后 `<repo>/.cache/quiz-proxy/state/default.json`；都找不到 ⇒ 自动转 content 模式。
- `--data-dir DIR` 换题库；`--card-lint PATH` 指定 `card-lint.py`（不给就自动发现
  `scripts/card-lint.py`；**不存在时用内置同口径兼容覆盖检查**，输出里
  `card_lint: "compat"` 并给降级提示，不静默跳过）。
- `--json` 输出 `{mode, checked, blockers, warnings, questions[{qid, findings[{code,
  level, field, name, note}]}], baseline{new,stale}, degraded}`。

退出码：`0` = 0 blocker（content 模式 = 无基线外新增/过期）；`1` = 有 blocker 或
基线漂移；`2` = 用法错误；`3` = 前提缺失（题库/引擎/text_lint 读不出，或显式给的
`--state` / `--card-lint` 不存在）——**不假装判过**。

## content 模式基线（只减不增）

存量欠账冻结在 `scripts/lesson-preflight-baseline.json`（键 = `qid:code:定位`，
双向棘轮）：基线外新增判红；已修条目没同步收紧也判红。修完一批内容后：

```bash
python3 scripts/lesson-preflight.py --all --update-baseline
```

## 证据命令（复核用）

```bash
python3 scripts/lesson-preflight.py --selftest                    # 红/绿 fixtures
python3 scripts/lesson-preflight.py --limit 22 --json             # learner path
python3 scripts/lesson-preflight.py --all                          # content（按基线）
python3 scripts/lesson-preflight.py --all --no-baseline --json     # 全量 blocker
```

## 已知依赖 / 限制

- 需要 `python3` + 课程 `engine.py`/`data/`（与 fast path 同一套，缺了 exit 3）。
- X11/X12/X13 与 X6/X7 复用 `courses/set-theory/knowledge/text_lint.py` 的 question
  模式（#149）；模块没有该模式时用内置兼容规则并标 `degraded`。
- 卡片覆盖优先子进程 `scripts/card-lint.py --json-out`（#146）；脚本未合并时用
  内置兼容实现（与 card-lint v1 同口径：`theorem` / 结果为 `Prop` 的 `def`）。
- 兼容覆盖检查与 `card-lint.py` 的 findings 应完全一致；不一致时以 `card-lint.py`
  为准（content 模式默认用它）。
- 无卡兜底的文本匹配走**卡片原始文本单元**（`definition.code` / `oneLiner` /
  `sections[].text/items/rows` / `types`），**不拿 `json.dumps` 后的字符串做子串**
  ——否则 why 里的反斜杠会被转义成 `\\`，造成「卡片里明明有 why」的假红
  （2026-11-29 修 `I.1.sdiff-4` / `I.1.sdiff-algebra-4`）。
- **content 模式依赖 #146 的卡片数据**：本分支合并 #146（补 10 张专用卡 + 扩 5 张，
  清 67 处未覆盖）**之前**，`--all` 会报 84 条真欠账（67 card + 17 无卡兜底缺引理名），
  这些由 #146 清掉、**不是**本脚本的基线债；基线故意留 **0 条**，合并后再跑
  `--all` 应为 0 blocker（可用 #146 分支的 `cards.json` 配 `--data-dir` 预演）。
  剩余已知欠账只有 X12/X13 warning（92 条，由 `text_lint.py` 基线管）。
