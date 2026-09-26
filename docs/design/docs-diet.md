# 文档瘦身（docs diet）：三类分治 + 判据守护

> 用户 2026-09-26：「**文档太重了，还没实现多少东西文档先爆炸了**」✗ ——
> **落成机制，不要口号** ✗。本文件是那套机制的**设计 + as-built** ✓。
> **判据**：`python3 scripts/docs-lint.py` ✓ —— **已进** `scripts/soko gate`（含 `--fast` ✓）、
> `scripts/ci-local.sh` 与 CI 的 **`docs-lint` job**（独立、不设 `if:` ⇒ **永远跑** ✓）。
> **反向验证**：`python3 scripts/docs-lint.py --selftest` ⇒ **6/6** ✓（也进 gate ✓）。

## 1. 问题（实测，不是感觉）

| 量 | 2026-09-26 10:05 基线（逐字节核对 ✓） |
|---|---|
| `docs/` | **419 文件 / 8.84 MB / 105,753 行** ✗ |
| 根 `REQUIREMENTS.md` | 264 KB / **3112 行**（其中 §9 = **2989 行**日期日志 ✗） |
| 最大的几个 | `STATUS-ARCHIVE` 585 KB · E1 计划 290 KB · `learning-difficulties` 273 KB · `e2-plan` 260 KB · **一个遗留 `.tmpdir` 260 KB** ✗ · `R1-design-craft` 205 KB · `course-inventory` 165 KB · `course-lean-style` 151 KB · `e2e/ledger` 138 KB · `CI-FAILURES` 130 KB · `TESTING` 107 KB |

**诊断** ✓：三类东西混在一起、要求完全不同 ⇒ **绝不能一刀切** ✗。

## 2. 三类口径（处置规则）

| 类 | 定义 | 处置 |
|---|---|---|
| **活规范** | 当前仍生效的契约：协议 / 架构 / 测试地图 / 现行设计 / **缺口台账与其复现件** / 入口文档 | **保留**（短、准 ✓）；超预算的**只留契约与边界** ✓，过程移出 |
| **过程记录** | 一次性过程与其已收口产物：逐轮日记 / CI 失败台账 / e2e·perf 台账 / 调研笔记 / 已完成批次的计划与审计 | **只留结论 + 指针** ✓：保留最近 N 条，更早进 `docs/archive/`（可 gzip ✓） |
| **垃圾** | 遗留 `.tmpdir` / `.tmp-<pid>` / 编辑器残留 / 生成残留 | **直接删** ✗ + `.gitignore` 防复发 ✓ + lint 判据 ⑤ |

## 3. 判据（六条，全部机械可判 ✓）

| # | 判据 | 阈值（本轮盘点后定） |
|---|---|---|
| ① | **活文档总量**（根 `*.md` + `docs/**`，**不含** `docs/archive/**`） | ≤ **3.0 MB**（实测 2.96 ✓；基线 8.84 ⇒ **33.5%**） |
| ② | **单文件行数**（任何活文档 `.md`） | ≤ **2000** |
| ③ | **入口文件行数**（`AGENTS.md` / `README.md` / `REQUIREMENTS.md` / `ROADMAP.md` / `STATUS.md` / `docs/README.md` / `docs/HANDOVER.md` / `docs/E2-HANDOVER.md` / `docs/design/e2-plan.md`） | ≤ **800** |
| ④ | **设计文档预算**：`docs/design/**` **新增** ≤150 行；**既有**按 `scripts/docs-budget.json` **冻结** | **只许减不许增** ✗（要放宽 ⇒ 手改那份 JSON ⇒ 评审可见 ✓） |
| ⑤ | **禁垃圾**：`docs/**` 不得有 `.tmpdir` / `.tmp` / `.tmp-<pid>` / `.DS_Store` / `*.orig` / `*.rej` / `*~`（**含未跟踪的本地残留** ✗） | 命中数 = **0** |
| ⑥ | **归档可追溯**（红线 ✓）：`docs/archive/**` 每个文件必须在 `docs/archive/README.md` 里**被点名**；归档总量 | 未点名 = **0**；总量 ≤ **2.5 MB**（实测 2.12 ✓） |

> **逃生门用过一次** ✓（评审可见）：本轮把文档预算写进 `AGENTS.md` 收尾义务时，
> 它从 368 → 375 行 ✗ ⇒ **手改 `scripts/docs-budget.json` 抬高那一条** ✓
> （其余 102 条不动 ✓）—— 这正是"要放宽必须手改 JSON"的设计意图 ✓，
> 而不是在脚本里抬阈值 ✗。
>
> **为什么 ④ 是"冻结"而不是"一律 ≤150 行"** ✗：78 份既有设计文档若逐份重写，
> 风险远大于收益（且很容易在"压缩"里丢掉边界 ✗）⇒ 用**棘轮**：
> **既有的只许减不许增** ✓、**新增的按 150 行预算** ✓ —— 压力一样在，但**不返工存量** ✓。

## 4. 处置结果（as-built，2026-09-26）

| 量 | 前 | 后 |
|---|---|---|
| **活文档**（根 `*.md` + `docs/**` 去归档） | 8.84 MB（`docs/` 口径） | **201 个 / 2.96 MB** ✓ |
| **归档**（`docs/archive/**`，gzip + 索引 ✓） | — | **212 个 / 2.12 MB** |
| `docs/` 合计（含归档） | **419 文件 / 8.84 MB** | **408 文件 / 4.98 MB** ✓（**-44%**） |
| 垃圾 | 48 个 / **1.54 MB** ✗ | **0** ✓ |

**最大的 5 个文件的去向** ✓（用户验收项）：

| 文件 | 前 | 去向 |
|---|---|---|
| `docs/STATUS-ARCHIVE.md` | 571 KB / 7392 行 | **活文档 120 KB**（留最近 8 段）· 更早 ⇒ `docs/archive/status-archive-older-rounds.md.gz`（186 KB） |
| `docs/design/vscode-editor-feedback-plan.md`（E1 计划） | 283 KB / 4375 行 | **活文档 19 KB**（收口索引 + 现行结论 + 124 条清单）· 逐字原文 ⇒ `…-full-2026-09-26.md.gz`（114 KB） |
| `docs/notes/settheory-survey/learning-difficulties.md` | 267 KB | **归档** ⇒ `docs/archive/settheory-survey-2026-09-26/learning-difficulties.md.gz`（93 KB） |
| `docs/design/e2-plan.md`（E2 计划） | 254 KB / 3105 行 | **活文档 31 KB / 379 行**（现行契约 + 一行一条收口索引）· 逐字原文 ⇒ `e2-plan-full-2026-09-26.md.gz`（89 KB） |
| `docs/design/site-rebuild/research/R1-design-craft.md` | 200 KB | **归档** ⇒ `docs/archive/site-rebuild-2026-09-26/R1-design-craft.md.gz`（91 KB） |

> 另外两处**同名分量级**的：`REQUIREMENTS.md` **258 KB → 17 KB**（§9 历史逐字进
> `docs/archive/REQUIREMENTS-ARCHIVE.md`，纯文本可 grep ✓）· `docs/e2e/ledger.jsonl`
> **135 KB → 29 KB**（留最近 50 条；更早 203 条 + 其 140 个日志 ⇒ `docs/archive/e2e-2026-09-26/`）。

## 5. 归档布局与可追溯规则（**归档 ≠ 销毁** ✓）

* **布局** ✓：`docs/archive/<批次>-<日期>/<原文件名>.gz`（**保留原相对路径** ⇒ 映射机械可算 ✓）；
  索引进 `docs/archive/README.md`（判据 ⑥ 机械检查 ✓）。
* **全局规则** ✓：仓库里凡出现 `docs/notes/**`、`docs/design/site-rebuild/**`、
  `docs/gaps/WO-*.md`、`docs/E2-PROMPT.md` 的路径 ⇒ **已归档**，到 `docs/archive/` 取
  **同名 + `.gz`**（`gunzip -c … | less` ✓）。
* **不许动的例外** ✗（有**真消费者**，动了就判红）：
  * `docs/gaps/repro/**`（**38 条复现被 `gap.py check` 在 gate + CI 三片矩阵里真跑** ✓，
    其中 G02 与 G06 夹具还被 cargo 测试引用）；
  * 活台账里**被引用**的 `docs/e2e/logs/*.log`（`e2e-merge.py --check` 校验存在 ✓）；
  * `docs/protocol.md`（**4 个测试读它正文并断言**：event type / error code / watch 事件 / 技能词表 ✗）；
  * `docs/perf/ledger.jsonl`（perf-gate 的**基线**）、`docs/gaps/ledger.jsonl`、`docs/courses/ledger.jsonl`。

## 6. 新环节的文档预算（给下一个 agent ✓）

1. **设计先行只写契约不写过程** ✗ —— 过程进 commit message 与 `STATUS.md` ✓；
2. 新设计文档 **≤150 行** ✓；既有文档**只许减不许增** ✓（`scripts/docs-budget.json`）；
3. 要放宽预算 ⇒ **手改那份 JSON**（评审可见 ✓），不要在脚本里抬阈值 ✗；
4. 归档 ⇒ **必须在 `docs/archive/README.md` 点名** ✓，否则 gate 红 ✗；
5. 写大文档前先问：**这是"现行要求"还是"过程"** ✓？过程 ⇒ 写进 `STATUS.md` 或归档 ✗。

## 7. 反向验证（**咬不住的守卫等于没有** ✓）

```bash
python3 scripts/docs-lint.py --selftest      # ⇒ 6/6 条判据咬得住 ✓（exit 0）
```

逐条**故意弄红**（改完自动还原 ✓）：① 灌 800 KB ⇒ 总量红 ✓ · ② 灌 2200 行 ⇒ 单文件红 ✓ ·
③ `REQUIREMENTS.md` +700 行 ⇒ 入口红 ✓ · ④ 撑大冻结文件 ⇒ 预算红 ✓ ·
⑤ 放一个 `.tmp` ⇒ 垃圾红 ✓ · ⑥ 放一个没被索引的归档文件 ⇒ 可追溯红 ✓。

> ⚠ **自检当场咬出一条真洞** ✓：② 的用例最初只灌 1100 行（556+1100 = 1656 < 2000）
> ⇒ 判据**没响** ✗ ⇒ 用例改成 2200 行才咬住 ✓ —— **这正是自检存在的意义** ✓。

## 8. 明确不做（有理由，不是"没时间"）

* **不逐份重写 78 份设计文档** ✗ ⇒ 用**冻结棘轮 + 总量封顶**代替（见 §3 的注 ✓）；
* **不删任何证据** ✗（红线 ✓）：复现件、被引用的 e2e 日志、缺口台账**原路径原字节** ✓；
* **不给"账本类"归档 gzip** ✗：`REQUIREMENTS-ARCHIVE.md` 保持**纯文本** ✓（要求的账本，**可检索性优先** ✓）；
* **不动 `docs/protocol.md` / `docs/architecture.md` / `docs/TESTING.md` 的契约段** ✗（前者有 4 个测试读正文 ✗，后两者是现行地图 ✓）。
