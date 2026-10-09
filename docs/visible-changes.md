# 「看得见的变化」清单（批次 N 收尾 · 2026-09-26 专项）

> **这是什么** ✓：用户 2026-09-26 专项（记法 × 隐式参数 × 产品交互）的**验收面**——
> 每条 = **用户报的什么 → 屏幕上多/少什么 → 哪一层的哪条断言咬住它**。
> 规格登记在 `REQUIREMENTS.md` §9.5；缺口台账 `docs/gaps/ledger.jsonl`
> （G-43 / G-69 / G-70 = 隐式参数那三条前置，均已 `fixed` ✓）。
> ⚠ **够不到的面不假装** ✓：终端 stdout、状态栏、webview DOM 不是真宿主 e2e 能断言的 ⇒
> 钉「**载荷到达**」那一层（stub 宿主 / CLI / 前端），并在下表写明它**不在** e2e 层。

| 用户报的 | 屏幕上多/少什么 | 判据（断言名 · 层） |
|---|---|---|
| ① 终端每秒刷 `build.tick` · 「build 没有反应」 | **机器通道不变**：`--json` 的 stdout 心跳契约一字不动（终端不发、管道发）✓；**人看的通道改走 stderr**：**逐文件**一行 `… 42/240 · <相对路径>`（C1，旧的是每 10% ⇒ 每 24 个才一条 ✗）+ 人话心跳 `… still building (12s)`（≥1s，真事件顶掉）；`SOKO_BUILD_TICK_MS=1` 强制、`SOKO_BUILD_NO_TICK=1` 逃生门 | `cli_build_heartbeat_is_off_unless_asked_for`（`crates/cli/tests/cli.rs` · 四态含反向）+ **`scripts/check-progress-cadence.py`**（N=120 ⇒ 人看 ≥120 行；拿 C1 之前的二进制跑**必须判红** = 10 行 ✓，`--selftest` 反向） |
| ② Rebuild 面板恒 `0%` · 无参 rebuild 是假重编 | 进度**边编边走**（`build.progress` 每编完一个文件就报；`build.file` 的确定性不变）；**C2**：无参 `clean`/`rebuild` 也按**当前目录**解模块根、**两处都清**（旧的无参只清全局 ⇒ 紧接着的预热全 `hit` ✗） | e2e「Rebuild shows a non-zero percent while it is still running (②)」（**真宿主**）+ `cli_build_reports_file_progress_while_it_compiles`（CLI）+ **`a_no_argument_rebuild_also_clears_the_module_root_artifacts`**（`crates/cli/tests/artifacts.rs` · 反向验证已实跑：撤掉默认 ⇒ `clean.project == 0` 判红 ✓） |
| ③ Infoview 版本戳陈旧 | 版本戳**每次写产物刷新**，裸版本号补标签（「由编译器 X 写入」/「服务器」） | `editor/vscode/test-project-first-screen.js`（stub 宿主，已进 `npm run test:unit`）+ 前端判据（两层都反向验证过） |
| Q1 大项目编译超时 | `sokonanoda.build.timeoutMs`（默认 300000，**0 = 不限制**）+ 超时消息写明去哪改 | `editor/vscode/test-extension-host.js`（配置读取 + 传参 · stub 宿主） |
| Q2 没装 CLI | `sokonanoda.installCli`（**自带即装**、离线、不校 SHA256） | `test-extension-host.js`（命令注册）+ **值**判据：跑 `--version` 与插件版本比 |
| K1 改一行等很久 | 编辑后热编译变快（**前缀复用默认开**） | e2e「rewriting an unchanged project unit does not recompile」·「editing a dependency refreshes the open unit once」（**真宿主**）+ `crates/cli/tests/judge_env_reuse.rs`（两态等价） |
| **隐式参数**（B2/B3） | 课标库的前导类型参数**不用再手写**：`a ∈ A` · `f '' A` · `A ⊆ B` · `A ∩ B` · `(∅) x` | front 单测 **6 条**（`crates/front/src/compile/tests.rs`，逐条**反向验证** ✓）+ 课程门禁 **43 目标 · 377 checked · 99 open · 0 判负** |
| 记法铺面（A1–A5）· prelude 能不能改 | `->` 折成 `→` · 集合字面量显示 `{a}` · F12 跳 `{a}` / prelude / 记法声明 · 目标行不漏折；**E1（2026-10-08）**：prelude 的真相搬到 `prelude/{Eq,L1,Quot}.sokonanoda`（Rust 里只剩 `include_str!`）⇒ **改文件真的会改行为** ✓、F12 落点是**仓库里那份**（不再是缓存副本 ✗） | e2e「Infoview 的 ⊢ 用记法箭头 →」「Infoview 里集合字面量显示成 {a}」「go to definition on a `{a}` set literal lands on `Set.singleton`」「go to definition on a prelude name lands in the prelude source」「hover 的类型文本折成记法（T-U12 面 #3）」（**真宿主 36/36** ✓）+ E1：`the_three_source_files_are_the_truth_and_rust_holds_no_prelude_text`（三段拼接 == 视图 == `prelude_source()`；Rust 里哨兵 0 命中）+ **反向验证**（把 `prelude/L1.sokonanoda` 的 `True` 改名 ⇒ `True.intro` 探针判红 ✓）+ `tests::navigation::goto_definition_on_a_builtin_notation_lands_in_the_prelude` | **B2（2026-10-08）**：光标**严格在**某条 tactic 之内 ⇒ 显示它**作用后**的目标（与 Lean 的 `useAfter` 一致；**恰在起点**仍是"进入态"），证明**在末条 tactic 闭合** ⇒ Infoview 与树项报 **🎉 恭喜，证完了（Q.E.D.）**（`open`/`failed`/无 tactic 的声明仍是中性的「已无目标 ✓」）。判据：`state_at_inside_a_tactic_shows_the_state_after_it` + `state_at_the_start_of_a_tactic_still_shows_the_entering_state`（front · 真相）· `state_at_inside_the_last_tactic_of_a_checked_proof_has_no_goals`（LSP · wire）· `test-webview.js`「a proof closed at its last tactic congratulates (Q.E.D.)」+ `test-extension-host.js`「the cursor tree congratulates…」（两个渲染点都钉）· **反向验证**：`SOKO_STATE_AFTER=0` ⇒ 前三条判红 ✓；e2e 用例「B2：光标在末条 tactic 之内…」（真宿主，**批次收尾跑**） **E3（2026-10-08）**：prelude 现在可以**运行时覆盖**（`SOKO_PRELUDE_DIR=<目录>`，目录里放`Eq.sokonanoda`/`L1.sokonanoda`/`Quot.sokonanoda` 中想改的那几份，缺的用内置）——学生用 release 包（没有 cargo）也能"改 prelude 就见效"；覆盖内容**进缓存键**、畸形覆盖 ⇒ **退出码 2 + stderr 原因**（不 panic）。判据：`a_runtime_prelude_override_changes_behaviour` + `a_malformed_override_exits_two_without_panicking`（CLI 子进程 · e2e 级）· `e3_tests::*`（front · 装载层）· `the_prelude_override_fingerprint_enters_the_key`（键）· 226 文件 `--json` 逐字节 ✓ · 反向验证两条 ✓ **C3（2026-10-08）**：`#check`/`#print` 的输出进 **Infoview** 的「命令输出」块（`#check` ⇒ `表达式 : 类型`、`#print` ⇒ 定义文本；按**光标所在行**取，与 Lean 的`getInteractiveDiagnostics{lineRange?}` 同口径；`#check` 的 inlay hint 照旧并存）——改前 `#print` 的事件**根本没进报告**、`#check` 只有内联灰字 ✗（用户 P5）。判据：`session_keeps_print_results_on_zero_recompile`（front · 真相）· `state_at_carries_the_command_outputs_on_the_caret_line`（LSP · wire，含"光标在命令行上= 不在任何声明里"那条路）· `test-webview.js`「command outputs … render as a messages block」（渲染，含"不许被占位符盖过"与"无输出不画空壳"）· `audit-wire-fields.py` ⇒ MISSING NONE ✓ · **反向验证**：改死渲染条件 ⇒ webview 判红 ✓；e2e 用例「C3：`#check`/`#print` 的输出出现在Infoview…」（真宿主，**批次收尾跑**） **C3 补口（2026-10-09 用户实测）**：`#check` 缺 notation/高亮 · `#print` 在带 `import` 的文件里「完全没有反应」。**屏幕上多什么**：① `#check` 的**类型那一半折记法**（`->` ⇒ `→`，与目标行同一观感 ✓）＋**整条命令输出按 `tok-*` 上色**（与目标/条件/声明卡片**同一个 `codeBlock`**，样式零硬编码 ✓）；② `#print` 在**项目模式**（课程文件都有 `import`）里**真的出现**，并打印出定义的 λ 本体（`fun (n : Nat) => …`）。**根因三条**（都是「真相对、显示没带上」的接缝）：① `run_pass_with` 的报告组装只映射 `TypeChecked` ⇒ 项目模式 `report.prints` **恒空**；② `splice_entry_report` 漏拼 `fresh.prints` ⇒ 编辑一次又没了；③ `messages[].text` 只有纯文本、没有分段 ⇒ webview 只能画一色 `<pre>`。**缓存纪律**：`REPORT_SHAPE` **3 → 4**（`prints` 的语义变了 ⇒ 旧条目必须整库不命中，否则旧缓存静默给「没反应」的旧答案 ✗）。**判据**：front（真相）`project_documents_keep_print_results_on_the_caret_line`（含增量拼接路 + `trusted_prefix_len() > 0` 的结构前提）· `command_outputs_carry_notation_and_highlight_runs`（runs 逐字节拼回 `text` + 折过记法 + 有 `kind`）· LSP（wire）`state_at_keeps_command_outputs_in_a_project_entry`（**真项目夹具**）· `state_at_carries_the_command_outputs_on_the_caret_line`（字段存在性）· webview（渲染）「command outputs render through the shared runs renderer」（`tok-*` span + 无 runs 的兜底）· `audit-wire-fields.py` 新接 `m.runs` · e2e「C3（项目模式）：带 import 的文件里 `#print` 的输出也要到 Infoview」（真宿主，**批次收尾跑**）。**反向验证三条**：撤 `fresh.prints` ⇒ front 判红 ✓ · 撤 webview 的 `codeBlock` ⇒ webview 判红 ✓ · 抹 `StateMessageInfo.runs` ⇒ wire-fields 判红 ✓。**边界（如实记）**：`#print` 的文本是内核的声明 pp（与 `decl.printed` 事件逐字节同源）⇒ **只上色、不折记法**；`#print` 打**开放练习**（`sorry` 体）仍报 `unknown identifier`（它不在环境里，与今天一致） |

## 怎么跑（每一列一条命令）

```bash
SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh   # 真宿主那一列 → docs/e2e/ledger.jsonl
node editor/vscode/test-extension-host.js                # stub 宿主（Q1/Q2/③）
cargo test -p sokonanoda-cli -p sokonanoda-front         # CLI / front 那一列
python3 courses/set-theory/tools/check.py                # 课程门禁（隐式参数那一行）
```

**遗留（如实记）**：③ 与 Q1/Q2 只有 **stub 宿主 + 值**判据（webview DOM 与状态栏在真宿主里
够不到 ⇒ 按 §9.2 的边界钉"载荷到达" ✓）；隐式参数那行的「**数量级下降**」只做到
「**不再必须写**」——点名叫法仍有一批带 `soko:notation-ok` 标记的调用点，逐站点迁移是后续工作；
`by` 块里对**隐式化之后的课标库**做 `cases`（`f '' (f ⁻¹' C)` 一类派生的假设）会撞**判定缝**
⇒ 台账 **G-71**（复现件 `docs/gaps/repro/G71-tactic-context-pp-form-not-rereadable.sokonanoda`，
自包含、判红 = 缺口仍在 ✓）。三条都详见 `docs/design/e2-plan.md` 的 T-N13 as-built「遗留」。
