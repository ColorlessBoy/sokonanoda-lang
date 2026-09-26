# `docs/archive/` —— 归档索引（**归档 ≠ 销毁** ✓）

> **这是什么** ✓：`docs/` 的**历史层** —— 已收口的过程记录、已发布批次的计划与调研、
> 逐轮日记、历史要求账本。**它们不进"活文档"预算** ✗（`scripts/docs-lint.py` 判据 ①
> 明确排除本目录 ✓），但**必须可追溯** ✓：**本目录里每个文件都在下面被点名**
> （判据 ⑥ 机械检查 ✗ —— **咬不住等于没有** ✓）。
>
> **怎么读** ✓：`.md.gz` / `.jsonl.gz` / `.log.gz` ⇒ `gunzip -c <文件> | less`
> （或 `zcat`）；纯文本 ⇒ 直接读、直接 `grep` ✓。**原文一字未改** ✓。
>
> **为什么不直接删** ✗：这些是**证据**（判据、复现件、决策依据、发布记录）——
> 红线是「**归档 ≠ 销毁**」✓，用户 2026-09-26 明确：**不许为了让数字变绿而删证据** ✗。
> 判据：`python3 scripts/docs-lint.py` ✓（已进 `scripts/soko gate` 与 CI ✓）。
>
> **总量**：1.85 MB（上限 2 MB ✓；判据 ⑥）。
> **来源**：2026-09-26 文档瘦身（用户：「文档太重了，还没实现多少东西文档先爆炸了」✗）
> —— 设计 `docs/design/docs-diet.md` ✓。

## 索引

### 1. 要求与计划（逐字原文）

| 归档文件 | 原路径 | 内容 | 归档日 |
|---|---|---|---|
| `REQUIREMENTS-ARCHIVE.md` | `REQUIREMENTS.md` §9 | **2026-09-06 → 09-26 的全部追加要求与交付记录**（逐字 **3146 行** + **139 条索引**） | 2026-09-26 |
| `e2-plan-full-2026-09-26.md.gz` | `docs/design/e2-plan.md` | **E2 计划逐字原文**（**3105 行**：50 环节的完整规格 / 判据 / 实测数字 / 交付记录） | 2026-09-26 |
| `vscode-editor-feedback-plan-full-2026-09-26.md.gz` | `docs/design/vscode-editor-feedback-plan.md` | **E1 计划逐字原文**（**4375 行**：124 环节；§13 清单 124/124 已勾完） | 2026-09-26 |

> 活文档侧的去处：`REQUIREMENTS.md` §9.1–§9.4（现行要求）·
> `docs/design/e2-plan.md`（现行契约 + 一行一条的收口索引）·
> `docs/design/vscode-editor-feedback-plan.md`（E1 收口索引 + 关键结论）。

### 2. 逐轮与失败台账（保留最近 N 条，更早的在这里）

| 归档文件 | 原路径 | 内容 | 归档日 |
|---|---|---|---|
| `status-archive-older-rounds.md.gz` | `docs/STATUS-ARCHIVE.md` | **第 1–104 轮**（2026-09-06 → 09-19）与更早散段（活文档只留轮号最大的 12 段） | 2026-09-26 |
| `ci-failures-2026-09-10-to-2026-09-25.md.gz` | `docs/CI-FAILURES.md` | CI 失败台账的**早期 42 条**（活文档只留最近 25 条） | 2026-09-26 |
| `e2e-2026-09-26/ledger-records-older.jsonl.gz` | `docs/e2e/ledger.jsonl` | e2e 台账的**早期 203 条记录**（活文档只留最近 50 条） | 2026-09-26 |
| `e2e-2026-09-26/logs/`（**140 个** `*.log.gz`） | `docs/e2e/logs/` | 与上面那 203 条记录配套的**运行日志**（活文档只留最近 50 条记录引用的 35 个） | 2026-09-26 |

> **e2e 归档的对应关系** ✓：归档记录的 `log` 字段仍写**原路径**
> （`docs/e2e/logs/<名字>.log`）⇒ 到 `docs/archive/e2e-2026-09-26/logs/<名字>.log.gz` 取
> （`gunzip -c` ✓）。判据：`python3 scripts/e2e-merge.py --check` ✓（只校验**活台账**里
> 记录引用的日志必须存在 ⇒ 活文档侧的日志**原路径原字节**未动 ✓）。
>
> 归档日志清单（140 个）：
`2026-09-18-0e53472.log.gz`, `2026-09-18-b0bcba3.log.gz`, `2026-09-18-b5d00c8-vc1.106.0.log.gz`, `2026-09-18-b5d00c8-vc1.138.0.log.gz`, `2026-09-18-ba3d20b.log.gz`, `2026-09-18-da4869a-vc1.106.0.log.gz`, `2026-09-18-da4869a-vc1.138.0.log.gz`, `2026-09-18-e0f9673-vc1.106.0.log.gz`, `2026-09-18-e0f9673-vc1.138.0.log.gz`, `2026-09-18-ed05f0d-vc1.138.0.log.gz`, `2026-09-18-f3b5745-vc1.106.0.log.gz`, `2026-09-18-f62c711.log.gz`, `2026-09-19-084da05-vc1.106.0.log.gz`, `2026-09-19-084da05-vc1.138.0.log.gz`, `2026-09-19-347bd83-vc1.106.0.log.gz`, `2026-09-19-347bd83-vc1.138.0.log.gz`, `2026-09-19-3c145e9-vc1.106.0.log.gz`, `2026-09-19-3c145e9-vc1.138.0.log.gz`, `2026-09-19-84bce52-vc1.106.0.log.gz`, `2026-09-19-84bce52-vc1.138.0.log.gz`, `2026-09-19-952b0f1-vc1.106.0.log.gz`, `2026-09-19-952b0f1-vc1.138.0.log.gz`, `2026-09-19-a948f65-vc1.106.0.log.gz`, `2026-09-19-a948f65-vc1.138.0.log.gz`, `2026-09-19-de75890-vc1.106.0.log.gz`, `2026-09-19-de75890-vc1.138.0.log.gz`, `2026-09-19-f3902b3-vc1.106.0.log.gz`, `2026-09-19-f3902b3-vc1.138.0.log.gz`, `2026-09-20-084da05-vc1.138.0.log.gz`, `2026-09-20-08b6782-vc1.106.0.log.gz`, `2026-09-20-08b6782-vc1.138.0.log.gz`, `2026-09-20-6b76127-vc1.106.0.log.gz`, `2026-09-20-6b76127-vc1.138.0.log.gz`, `2026-09-20-9f5a365-vc1.106.0.log.gz`, `2026-09-20-9f5a365-vc1.138.0.log.gz`, `2026-09-20-a41fd22-vc1.106.0.log.gz`, `2026-09-20-a41fd22-vc1.138.0.log.gz`, `2026-09-20-c41cc96-vc1.106.0.log.gz`, `2026-09-20-c41cc96-vc1.138.0.log.gz`, `2026-09-20-e11fe07-vc1.106.0.log.gz`, `2026-09-20-e11fe07-vc1.138.0.log.gz`, `2026-09-21-00fa895-vc1.106.0.log.gz`, `2026-09-21-00fa895-vc1.138.0.log.gz`, `2026-09-21-2ccf5e1-vc1.106.0.log.gz`, `2026-09-21-2ccf5e1-vc1.138.0.log.gz`, `2026-09-21-4ac2fc5-vc1.138.0.log.gz`, `2026-09-21-67f1c8c-vc1.138.0.log.gz`, `2026-09-21-81af38c-vc1.106.0.log.gz`, `2026-09-21-81af38c-vc1.138.0.log.gz`, `2026-09-21-8899d5a-vc1.138.0.log.gz`, `2026-09-21-b8fc64f-vc1.138.0.log.gz`, `2026-09-21-be63cd3-vc1.138.0.log.gz`, `2026-09-22-3c8c500-vc1.138.0.log.gz`, `2026-09-22-7874dd4-vc1.138.0.log.gz`, `2026-09-22-a66c978-vc1.138.0.log.gz`, `2026-09-23-085d60b-vc1.106.0.log.gz`, `2026-09-23-085d60b-vc1.138.0.log.gz`, `2026-09-23-1b34b64-vc1.106.0.log.gz`, `2026-09-23-1b34b64-vc1.138.0.log.gz`, `2026-09-23-32f06db-vc1.106.0.log.gz`, `2026-09-23-32f06db-vc1.138.0.log.gz`, `2026-09-23-45e3736-vc1.106.0.log.gz`, `2026-09-23-45e3736-vc1.138.0.log.gz`, `2026-09-23-58a0c9a-vc1.106.0.log.gz`, `2026-09-23-58a0c9a-vc1.138.0.log.gz`, `2026-09-23-71e3ee7-vc1.106.0.log.gz`, `2026-09-23-71e3ee7-vc1.138.0.log.gz`, `2026-09-23-78b1449-vc1.138.0.log.gz`, `2026-09-23-851467b-vc1.106.0.log.gz`, `2026-09-23-851467b-vc1.138.0.log.gz`, `2026-09-23-9d8aa9b-vc1.106.0.log.gz`, `2026-09-23-9d8aa9b-vc1.138.0.log.gz`, `2026-09-23-aeb138a-vc1.138.0.log.gz`, `2026-09-23-b53bb11-vc1.106.0.log.gz`, `2026-09-23-b53bb11-vc1.138.0.log.gz`, `2026-09-23-bc2c4f9-vc1.106.0.log.gz`, `2026-09-23-bc2c4f9-vc1.138.0.log.gz`, `2026-09-23-d321f3c-vc1.106.0.log.gz`, `2026-09-23-d321f3c-vc1.138.0.log.gz`, `2026-09-23-eaa7aa7-vc1.138.0.log.gz`, `2026-09-23-eef83f3-vc1.106.0.log.gz`, `2026-09-23-eef83f3-vc1.138.0.log.gz`, `2026-09-24-0d56ef3-vc1.138.0.log.gz`, `2026-09-24-106554e-vc1.138.0.log.gz`, `2026-09-24-12a1e22-vc1.138.0.log.gz`, `2026-09-24-1b4f393-vc1.138.0.log.gz`, `2026-09-24-2152116-vc1.106.0.log.gz`, `2026-09-24-2a0f968-vc1.106.0.log.gz`, `2026-09-24-2a0f968-vc1.138.0.log.gz`, `2026-09-24-3b854a4-vc1.106.0.log.gz`, `2026-09-24-3b854a4-vc1.138.0.log.gz`, `2026-09-24-4cf3aa1-vc1.106.0.log.gz`, `2026-09-24-4cf3aa1-vc1.138.0.log.gz`, `2026-09-24-69a05bb-vc1.106.0.log.gz`, `2026-09-24-69a05bb-vc1.138.0.log.gz`, `2026-09-24-6dd1fe8-vc1.106.0.log.gz`, `2026-09-24-7ca60b9-vc1.106.0.log.gz`, `2026-09-24-85f18da-vc1.106.0.log.gz`, `2026-09-24-85f18da-vc1.138.0.log.gz`, `2026-09-24-9092ea5-vc1.138.0.log.gz`, `2026-09-24-9ec0110-vc1.106.0.log.gz`, `2026-09-24-9ec0110-vc1.138.0.log.gz`, `2026-09-24-b0c35da-vc1.138.0.log.gz`, `2026-09-24-b2e6dac-vc1.106.0.log.gz`, `2026-09-24-b2e6dac-vc1.138.0.log.gz`, `2026-09-24-c3bde87-vc1.138.0.log.gz`, `2026-09-24-e9c3dfb-vc1.138.0.log.gz`, `2026-09-24-ec68f64-vc1.106.0.log.gz`, `2026-09-24-ec68f64-vc1.138.0.log.gz`, `2026-09-25-08f7524-vc1.138.0.log.gz`, `2026-09-25-1bdbf17-vc1.106.0.log.gz`, `2026-09-25-1bdbf17-vc1.138.0.log.gz`, `2026-09-25-2be03e5-vc1.106.0.log.gz`, `2026-09-25-5053579-vc1.106.0.log.gz`, `2026-09-25-5053579-vc1.138.0.log.gz`, `2026-09-25-5260568-vc1.106.0.log.gz`, `2026-09-25-5260568-vc1.138.0.log.gz`, `2026-09-25-5524b01-vc1.106.0.log.gz`, `2026-09-25-5524b01-vc1.138.0.log.gz`, `2026-09-25-5d95894-vc1.106.0.log.gz`, `2026-09-25-5d95894-vc1.138.0.log.gz`, `2026-09-25-5dbe940-vc1.106.0.log.gz`, `2026-09-25-5dbe940-vc1.138.0.log.gz`, `2026-09-25-6024bc5-vc1.106.0.log.gz`, `2026-09-25-6024bc5-vc1.138.0.log.gz`, `2026-09-25-63bb4bf-vc1.106.0.log.gz`, `2026-09-25-6912232-vc1.138.0.log.gz`, `2026-09-25-70e9fbc-vc1.138.0.log.gz`, `2026-09-25-75f5b69-vc1.106.0.log.gz`, `2026-09-25-75f5b69-vc1.138.0.log.gz`, `2026-09-25-d11354b-vc1.106.0.log.gz`, `2026-09-25-d11354b-vc1.138.0.log.gz`, `2026-09-25-db7ce43-vc1.138.0.log.gz`, `2026-09-25-e764edf-vc1.106.0.log.gz`, `2026-09-25-e764edf-vc1.138.0.log.gz`, `2026-09-25-e8c8152-vc1.138.0.log.gz`, `2026-09-25-f15edda-vc1.106.0.log.gz`, `2026-09-25-f15edda-vc1.138.0.log.gz`, `2026-09-25-f7df873-vc1.106.0.log.gz`, `2026-09-25-f7df873-vc1.138.0.log.gz`

### 3. 站点重构（历史存档 · AGENTS.md 明写"**不要照着它们新建页面**"）

> 2026-09-20 的 28 页重构方案，**已被 `docs/design/site-single-page.md` 取代** ✗。
> **仍有效的实测依据**（`STATE.md` §5「实测与文档不符」、`R4-fonts.md` 的字体数据、
> `D1-design-rules.md` 的视觉层参考）**逐字保留在归档里** ✓ ——
> 引用它们的三处已改指本目录：`AGENTS.md`（§站点）、`scripts/gen-site-data.py`（docstring）、
> `site/assets/fonts.css`（注释）。

- `C1-language.md.gz`
- `C2-teaching.md.gz`
- `C3-tooling.md.gz`
- `C4-status-roadmap.md.gz`
- `D1-design-rules.md.gz`
- `D2-information-architecture.md.gz`
- `D3-lab-data.md.gz`
- `D5-css-components.md.gz`
- `D6-editor-components.md.gz`
- `D7-design-audit.md.gz`
- `D9-page-brief.md.gz`
- `R1-design-craft.md.gz`
- `R2-docs-teardown.md.gz`
- `R4-fonts.md.gz`
- `STATE.md.gz`
- `page-template.html.gz`
- `site.md.gz`

### 4. 调研笔记（结论已升格进活文档，这里是现场底稿）

**集合论课程调研**（结论 → `docs/design/set-theory-syllabus.md` / `docs/design/teaching-project.md`）：
- `P1-axiom-params.sokonanoda.gz`
- `P1b-axiom-curried.sokonanoda.gz`
- `chinese-textbooks.md.gz`
- `lean4-sets-functions-prior-art.md.gz`
- `learning-difficulties.md.gz`
- `lib.sokonanoda.gz`
- `lib2.sokonanoda.gz`
- `prior-art-report.md.gz`
- `proof-book-tocs.md.gz`
- `set-theory-teaching-survey.zh.md.gz`

**课程 Lean 化调研**（结论 → `docs/design/course-lean-style.md` / `notation-subset.md` /
`implicit-arguments.md` / `notation-aware-printing.md` / `by-tactics.md`）：
- `C25-delete-skeletons-brief.md.gz`
- `R2-full-rewrite-brief.md.gz`
- `R2-rewrite-brief.md.gz`
- `course-impact-implicit-args.md.gz`
- `course-inventory.md.gz`
- `implicit-args-plan.md.gz`
- `intro-course-constraints.md.gz`
- `notation-audit.md.gz`
- `printback-feasibility.md.gz`
- `tactic-audit.md.gz`
- `tooling-impact.md.gz`

### 5. 缺口工作单（`docs/gaps/ledger.jsonl` 的 `wo` 字段指向它们）

> ⚠ **指针目标** ✓：台账里每条缺口的 `wo` 字段写的是**原路径**（`docs/gaps/WO-0xx-….md`）
> ⇒ 到本目录取同名 `.gz`（`gunzip -c` ✓）。**13 个全部已关账**（fixed）。

- `WO-001-launcher-version-source.md.gz`
- `WO-002-module-root-absolute.md.gz`
- `WO-003-query-check-parse-error.md.gz`
- `WO-004-open-exercise-signature.md.gz`
- `WO-005-ctor-namespace.md.gz`
- `WO-006-prop-type-param-inductive.md.gz`
- `WO-007-course-import.md.gz`
- `WO-008-axiom-binder-params.md.gz`
- `WO-009-single-universe-binder.md.gz`
- `WO-010-kernel-error-span.md.gz`
- `WO-011-notation.md.gz`
- `WO-012-zero-ary-notation-applied.md.gz`
- `WO-013-lsp-drops-rescued-report.md.gz`

### 6. 合并归档（**活文档只留契约，过程进这里** ✓）

> 这几份文档**仍在用** ✓ —— 但**过程部分**（逐轮 as-built / 调研 / 现状审计 /
> 分阶段计划 / 逐条清单 / 已过期的会话快照）已移出 ✓；活文档留的是**契约与边界** ✓。

- `course-lean-style-design-process-2026-09-26.md.gz`
- `duplication-audit-77-items-2026-09-26.md.gz`
- `imports-and-projects-process-2026-09-26.md.gz`
- `testing-round-updates-2026-09-26.md.gz`
- `handover-process-2026-09-26.md.gz`
- `e2-prompt-2026-09-26.md.gz`

### 7. 自述「已废弃」的设计（**作者显式废弃强于引用点** ✓）

> 五篇都**自述**已废弃/历史存档（值位关键字 0.27.0 整体移除、`char_steps` 基建删除、
> 0.62.0 批次的计数已过期）⇒ 归档 ✓。**现行依据**是 `docs/design/remove-funintro.md`
> （被 `crates/cli/tests/cli.rs` 当"已移除"的现行依据 ✓）。

- `design-deprecated-2026-09-26/term-intro.md.gz`
- `design-deprecated-2026-09-26/value-keywords-v2.md.gz`
- `design-deprecated-2026-09-26/real-input-tests.md.gz`
- `design-deprecated-2026-09-26/round14.md.gz`
- `design-deprecated-2026-09-26/lean-style-0.62.md.gz`

### 8. 顶层调研笔记（`docs/notes/*.md`）

> 结论已升格进活文档（`docs/design/*.md`）；这里是**现场底稿** ✓。
> **保留在活文档的**：`notation-rewrite-brief.md` / `R3-rewrite-brief.md` /
> `notation-input-plan.md`（`docs/notes/course-lean-style/`）与 `dsh-project-assets.md`
> —— 它们分别被 `AGENTS.md`、`skills/sokonanoda-teacher`、`crates/front/src/notation_input.rs`
> 与 `crates/cli/tests/dsh.rs`（契约测试）钉住 ✓。

- `cache-key-build-stamp.md.gz`
- `gap-analysis.md.gz`
- `inductive.md.gz`
- `lsp-notes.md.gz`
- `multifile-prior-art.md.gz`
- `project-roots-and-incremental-caches.md.gz`
- `research.md.gz`
- `rust-cross-platform-binary.md.gz`
- `vscode-notes.md.gz`
