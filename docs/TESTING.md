# TESTING.md — 测试资产地图（交接文档）

测试是本项目的**协作与重构资产**，不是附属品：多个 agent / 多个人会并行改 `front`、`kernel`、`cli`、`lsp`，并且会反复做大规模重构。因此每条流水线层（lex → parse → elab → kernel 检查 → 事件与报告 → CLI/LSP 渲染）都有**独立可跑**的守护测试——改哪一层就只跑哪一层的测试，不用跑全仓库就能确认自己没破坏别人。反过来说，规则是 **TDD 三件套**：给前端加任何语法点 / 错误模式，必须凑齐三件——

1. **front 单测**：`crates/front/src/**` 的 `#[cfg(test)]`，token / parser / compile 各层独立守护 AST 形状、错误码与 kernel 判定；
2. **CLI e2e**：`crates/cli/tests/`，通过真实二进制 + stdin/stdout 守护人类视图与 `--json` 机器视图；
3. **语料**：`examples/*.sokonanoda`，每份教学文件必须整文件通过（corpus 测试自动纳入新文件）。

三件齐全，这个语法点才算"进了课程"。另附两条铁律（详见 `docs/architecture.md` §7）：

- **判定练习靠 kernel，不靠文本比对**（唯一例外：`proof.rs::assumption` 的草案级文本比对，待替换，见盲区 4）；
- **错误码 + 教学提示是协议的一部分**：每个 `ErrorKind` 必须有稳定 code（`elab-*` / `kernel-*` 前缀）与中文 hint，且要写进 `docs/protocol.md`——这两条由 meta 测试强制执行（见 §2）。

## 1. 分层守护表（改哪层，跑哪层）

| 流水线层 | 守护什么 | 守护测试位置（文件 : 测试名模式） | 怎么跑 |
|---|---|---|---|
| lex | 行注释跳过、`???`→Hole、`forall` 关键字 vs 标识符、`->`/`=>`/`:`/`:=`/`@`、宇宙切片 `id.{u}` 的 token 边界、数字、非 ASCII 标识符、`=` 缺 `>` 的错误行列、`#check` 等命令 token | `crates/front/src/token.rs` :: `line_comment_is_skipped`、`triple_question_lexes_as_hole`、`forall_is_a_keyword_but_prefixed_idents_are_not`、`punctuators_lex_in_order`、`universe_application_slice_lexes_as_plain_tokens`、`number_lexes_as_num_token`、`non_ascii_identifiers_lex_as_ident`、`lone_equals_is_an_error_with_position`、`hash_command_lexes_as_ident` | `cargo test -p sokonanoda-front token::` |
| parse | 命名箭头 `(x : A) -> B` = 带 binder 的 Forall、隐式 `{x : A} -> B`、`forall (...), body` 逗号、宇宙参数 `{u, v}`、重复宇宙参数报错、inductive 块（ctor/rec 字段）、表达式内禁命令关键字、`???` 落在 val、诊断行列 | `crates/front/src/parser.rs` :: `named_arrow_parses_as_forall_with_binder`、`implicit_named_arrow_parses_as_forall_with_implicit_binder`、`forall_with_comma_parses_binders_and_body`、`def_parses_two_universe_params`、`duplicate_universe_param_is_rejected`、`inductive_block_parses_ctors_and_recursor`、`command_keyword_inside_expression_is_rejected`、`example_keeps_hole_in_value_position`、`reports_diagnostic_with_span` | `cargo test -p sokonanoda-front parser::` |
| elab | 未知标识符/常量、宇宙层级未声明/个数不符、binder 类型要求、`@id.{0}` 显式宇宙应用、隐式 binder 风格、双宇宙参数 axiom、`ErrorKind` 穷尽性与 code/hint 形状 | `crates/front/src/compile/tests.rs` :: `checks_universe_*`、`rejects_undeclared_universe_variable`、`rejects_wrong_number_of_universe_arguments`、`checks_at_marker_and_implicit_binders_in_py_fol_style`、`parses_implicit_binder_styles`、`checks_axiom_with_two_universe_params`、`checks_explicit_universe_application`、`every_error_kind_has_stable_code_and_hint` | `cargo test -p sokonanoda-front compile::tests::` |
| kernel 检查 | 整文件过完整 kernel、拒绝带 span、拒绝消息形状（不是 panic 栈）、py-fol / py-nat 移植用例正反两面、显式 inductive / Nat 块、`#check`/`#reduce`/`#print`、原生大整数路径 | `crates/front/src/compile/tests.rs` :: `checks_a_valid_file_end_to_end`、`reports_kernel_rejection_with_span`、`kernel_rejection_message_is_not_a_panic_trace`、`checks_from_scratch_fol_proofs`、`checks_ported_py_fol_core`、`py_core_checks*` / `py_rejects_*` 系列、`explicit_inductive_block_compiles`、`explicit_nat_block_overrides_builtin_prelude`、`ported_nat_fol_add_two_two_reduces`、`checks_nat_literals_and_reduces_addition`；kernel 自身：`crates/kernel/src/tests/*` + `crates/kernel/tests/memory_api.rs` | `cargo test -p sokonanoda-front compile::tests::`；`cargo test -p sokonanoda` |
| 事件与报告 | `CheckEvent` 流（checked/open/typed/reduced/printed）、`DocumentReport` 状态机（Checked/Open/Failed 按源码序、Failed 带 error、errors 计数）、open 练习不污染 env、hover 类型图、`render_expr` 往返 | `crates/front/src/compile/tests.rs` :: `checks_dependent_forall_with_lambda`、`accepts_open_exercise`、`document_report_tracks_open_checked_failed_decls`、`document_report_states_in_source_order`、`open_exercise_does_not_pollute_env`、`document_report_produces_hover_types_for_subexpressions`、`hover_map_covers_subexpressions`、`render_expr_round_trips` | `cargo test -p sokonanoda-front compile::tests::` |
| CLI 人类视图 | stdin/文件批处理、`line:col: error[code]: message` 格式、REPL 声明累积 / `#env` / `#help` / `#prove` | `crates/cli/tests/cli.rs` :: `cli_checks_a_valid_file_via_stdin`、`cli_rejects_a_bad_declaration`、`cli_reports_parse_errors_with_positions`、`cli_prints_*`、`repl_*`、`human_errors_carry_the_pipeline_stage`、`cli_help_is_self_documenting` | `cargo test -p sokonanoda-cli --test cli` |
| CLI JSON 协议 | `--json` 词汇封闭（7 种事件）、diagnostic 形状（stage/code/hint/span）、`protocol.md` 列出全部事件类型、exercise.open 是成功态 | `crates/cli/tests/protocol.rs` :: `every_example_stream_is_closed_vocabulary`、`kernel_rejection_diagnostic_shape`、`protocol_document_lists_every_emitted_type`、`elab_unknown_identifier_diagnostic`、`open_exercise_is_success_state`；另有 `cli.rs` :: `json_mode_*` | `cargo test -p sokonanoda-cli` |
| LSP 协议 | **无自动化**（见盲区 3）：publishDiagnostics / hover / documentSymbol / codeLens / quick-fix 只能手工验证 | （缺位——`crates/lsp/` 目前没有 tests 目录） | `cargo run -p sokonanoda-lsp` 手测 |
| 语料 | 每份 `examples/*.sokonanoda` 必须被真实 CLI 整文件通过 | `crates/cli/tests/examples.rs` :: `every_example_lesson_is_a_valid_sokonanoda_file` | `cargo test -p sokonanoda-cli --test examples` |
| 文档一致性 | `docs/protocol.md` 必须列出每个 `ErrorKind` 的 code；parse 两个 code 也在文档里 | `crates/front/src/compile/tests.rs` :: `protocol_doc_lists_every_error_code`（自带不带通配的穷尽清单） | `cargo test -p sokonanoda-front protocol_doc` |
| perf 冒烟 | 原生大整数路径（39 位大数 +1 归约）与 iota 链（`add two two` = 4 层 succ）不退化；30s canary 挡 debug 构建下的意外爆炸 | `crates/front/src/compile/tests.rs` :: `perf_smoke_native_and_iota_reduce` | `cargo test -p sokonanoda-front perf_smoke` |
| proof/tactic | `#prove` 的 intro/exact/lambda 搭建；生成的 lambda 必须被完整 kernel 接受 | `crates/front/src/proof.rs` :: `intro_builds_lambda_text`、`exact_fills_the_hole`、`generated_lambda_passes_the_kernel` | `cargo test -p sokonanoda-front proof::` |

## 2. 两个 meta 测试的机制（新 agent 最容易踩）

- `every_error_kind_has_stable_code_and_hint`：测试内部有一段**不带通配分支**的 `matches!` 穷尽清单——给 `ErrorKind` 加新 variant 时，这段代码会**编译失败**，逼你把新 variant 加进清单，并补齐 code（前缀必须与 stage 一致：elab kind → `elab-*`，kernel kind → `kernel-*`）与非空中文 hint。这是把"忘补 code/hint"从运行时错误提前到编译错误。
- `protocol_doc_lists_every_error_code`：把每个 code 断言出现在 `docs/protocol.md` 里。目前 6 个 elab code 还没写进文档，用 `known_missing` 允许表临时放行（见盲区 1）。**允许表是自毁的**：一旦文档补齐了这 6 个 code，测试反而会失败，失败消息会明确提示你"从 `known_missing` 删除它们"。

## 3. 失败时的排查顺序

测试红了先定位层，再定位归属：

1. `parser::` / `token::` 红了 → front 的词法/语法层，看 `DiagnosticKind` 与 span；
2. `compile::tests::` 里 `checks_*` 红了 → 要么 elab 语义回归，要么 kernel 误拒；先用 `#check`/`#reduce` 在 REPL 复现，区分 front 与 kernel 的责任；
3. `cli::` / `protocol.rs` 红了 → 输出格式或事件词汇变了，**先怀疑协议破坏**（`docs/protocol.md` 是契约）；
4. `examples.rs` 红了 → 语料与前端能力漂移，绝不许"改语料让它过"，要修前端或明确废弃该语法点；
5. `protocol_doc_lists_every_error_code` / `every_error_kind_*` 红了 → 错误码体系或文档契约变更，按 §2 处理。

## 4. 怎么加一个新语法点（TDD 三件套 checklist）

按顺序走，每一步先红后绿：

1. **lex 层**（若引入新 token/符号）：在 `crates/front/src/token.rs::tests` 加 token 级测试，含错误定位断言（`line`/`column`）。跑 `cargo test -p sokonanoda-front token::`。
2. **parse 层**：在 `crates/front/src/parser.rs::tests` 加 AST 形状断言——新 `Command`/`Expr` 的字段逐一断言（含 binder 风格、span），同时加一个错误路径测试。跑 `cargo test -p sokonanoda-front parser::`。
3. **elab/kernel 层**：在 `crates/front/src/compile/tests.rs` 同时加一对——`checks_<新语法点>`（合法路径：`out.errors == vec![]` + 期望事件精确值）与 `rejects_<误用>`（非法路径：断言错误 `code()` 的精确值，不是只断言"有错"）。跑 `cargo test -p sokonanoda-front compile::tests::`。
4. **新增 ErrorKind 时**：`error.rs` 补 variant + code + 中文 hint → `every_error_kind_has_stable_code_and_hint` 的穷尽清单与 `protocol_doc_lists_every_error_code` 会逼你同步两处测试并把 code 写进 `docs/protocol.md`。
5. **CLI e2e**：在 `crates/cli/tests/cli.rs` 加 stdin 端到端（断言一行人类输出），必要时在 `crates/cli/tests/protocol.rs` 加 JSON 断言。跑 `cargo test -p sokonanoda-cli`。
6. **语料**：在 `examples/` 加或扩充教学文件；corpus 测试自动纳入。跑 `cargo test -p sokonanoda-cli --test examples`。
7. **收尾**：`cargo test -p sokonanoda-front && cargo test -p sokonanoda-cli && cargo test -p sokonanoda` 全绿；更新 `docs/architecture.md` §4 的语法白名单（课程 = 语法白名单）与 `docs/protocol.md` 的事件/错误码清单。

## 5. 已知盲区（下一步接手的人从这里开始）

1. **`docs/protocol.md` 的 elab 错误码清单不完整**：缺 `elab-unknown-constant`、`elab-too-many-binders`、`elab-nat-literal-disabled`、`elab-invalid-nat-literal`、`elab-too-many-ctor-fields`、`elab-unknown-ctor-for-iota`。补齐文档后删除 `protocol_doc_lists_every_error_code` 里的 `known_missing` 允许表（测试失败消息会引导你）。
2. **kernel 内部 2 个 ignored fixture 测试**：`crates/kernel/src/tests/util.rs` 的 `reject_rec_rule_with_forged_lambda_domains` 与 `reject_unlisted_recursor`——上游提交了测试但没提交 `test_resources/` 的 NDJSON fixture，重建 fixture 是独立任务。另有 `crates/kernel/tests/arena.rs` 需要设 `LEAN_KERNEL_ARENA` 才运行，未设时自动跳过。
3. **编辑器端零自动化**：`sokonanoda-lsp` 没有任何测试；publishDiagnostics、hover、documentSymbol、codeLens、quick-fix 全靠手工验证。
4. **goal 视图未测——留给 I9**：`#prove` 只有 `proof.rs` 的 3 个单测（intro/exact/lambda），REPL 集成路径、goal 渲染、`assumption` 的文本比对草案（待替换为 kernel 判定）都没有 e2e 覆盖。
5. **hover 文本的语义要读懂再用**：hover 显示的是"该子表达式的类型"。类型位置的 `Nat` 的 hover 是 `Type 0`（Nat 的类型），值位置的 `n + 1` 才是 `Nat`——`hover_map_covers_subexpressions` 固定了这两个真实值，编辑器呈现时别想当然。
6. **perf 只有 canary 不是基准**：30s 阈值只挡 debug 构建下的"意外爆炸"，真正的性能回归基准（Lean Kernel Arena 对比）尚未立项。

## 6. 当前规模（截至本文写作）

- `cargo test -p sokonanoda-front`：**74** 个单测全绿（token 10 / parser 10 / proof 3 / compile 51）；
- `cargo test -p sokonanoda-cli`：**30** 个 e2e 全绿（cli 21 / protocol 8 / examples 1）；
- `cargo test -p sokonanoda`（kernel）：41 过 + 2 ignored（盲区 2）+ memory_api 1 过。

## 2026-09-07 更新：新增层与总量

- **semantic tokens 层**：`crates/front/src/semantic.rs`（分类单测 10 个）+
  `crates/lsp`（capability/编码/端到端 4 个，UTF-16 增补平面用例）。
- **session/delta 层**：`crates/front/src/session.rs`（solved/failed/opened、
  零重编译、parse 错误恢复，2 个）。
- **check-then-add**：`kernel_failed_declaration_frees_its_name`（双趟语义）。
- **watch（L1 CLI 形态）**：手动三版本冒烟（file.changed → exercise.solved →
  零重编译）；自动化等 service 化后补。
- 测试总量（2026-09-07 第四轮）：**203**（kernel 43 / front 103 / cli 38 / lsp 20，
  cli 含 26+3+1+8 四个套件）。
- 内核 iota 规则教学注意：`inductive` 块现在被 kernel 判定，iota 规则的递归
  调用必须写 `Rec.{u}`（宇宙参数显式），规则值必须是 `ms n (Rec.{u} ...)`。

## 2026-09-07 更新（第五轮：I8 真增量 + I9 judge + 内核 soundness）

- **I8 增量层**（`crates/front/src/session.rs`，5 个新测试）：
  `session_rechecks_only_the_affected_suffix`（改第 4/5 个声明 →
  `stats.kernel_checks == 2/1`，插入/删除对齐）、
  `session_zero_recompile_keeps_prefix_results`（注释编辑零重编译 + hover
  span 平移 + 事件缓存）、`session_whitespace_between_commands_zero_recompiles_with_remap`、
  `session_prefix_failure_keeps_name_free_for_suffix`、
  `session_prelude_directive_change_rebuilds`。
- **I9 judge 层**（`crates/front/src/judge.rs`，8 个）：kernel 判定匹配/不匹配
  （含期望与实际）、依赖 binder、binder 缺类型标注报错、defeq-不同文本
  （`a -> False` ≡ `Not a`）、elab 错误透传、check-then-add 下前缀失败声明的
  名字释放、Bare 模式、宇宙参数。`proof.rs` 5 个（exact_kernel /
  assumption_kernel 正反两面）。
- **goal 视图协议层**（`crates/lsp`，3 个新测试）：`soko/goals` 结构与 hole
  range、`soko/nextHole` 三向导航、defeq-exact 端到端（文本比对给不出、
  kernel 判定能给出）。LSP 全部测试切换到带自定义方法注册的 `test_service`。
- **内核 conv soundness**（`crates/kernel/tests/memory_api.rs`，2 个）：
  `dependent_codomain_is_not_inhabited_by_identity_lambda`（修复前通过、
  修复后拒绝）+ `identity_over_sort_still_checks`（对照不过度拒绝）；
  CLI e2e 两条（`cli_rejects_uninhabited_dependent_codomain` /
  `cli_accepts_identity_over_sort`）。
- **lint 门禁即测试**：教学 crates `[lints.rust] warnings = "deny"`（clippy
  违规 = 编译失败），CI fmt 门禁覆盖教学 crates（kernel rustfmt.toml 需
  nightly，见 ci.yml 注释）。
- 测试总量（2026-09-07 第五轮）：**229**（kernel 43+2 新回归 / front 121 /
  cli 40（28 cli + 8 protocol + 3 course + 1 examples）/ lsp 23）。
  `cargo clippy --workspace` exit-0（kernel 冻结快照保持 warning 级）。

## 2026-09-07 更新（第六/七轮：skills + 逻辑先行课程）

- **skill 套件**（`crates/cli/tests/skill.rs`，4 个）：frontmatter（name=目录、
  description 长度）、引用路径存在性、事件/方法词汇封闭且 protocol.md 记载
  （词汇表移至 `tests/common/mod.rs` 与 protocol.rs 共用）、course.json 单元/
  钥匙孪生存在性。
- **宇宙参数携带**（第七轮）：`open_exercise_carries_universe_params`、
  `judge_uses_carried_universe_for_sort_u_goals`（front）。
- **课程 golden 重测**（course.rs，刻意变更）：新单元序 ①命题与证明
  (12,5,1) / ②等式与 rfl (2,5,2) / ③函数与箭头 (1,4,1) / ④宇宙 (0,3,0) /
  ⑤归纳 (4,3,1)；playground 重排后 checked=14 / open=12 / diagnostics=0。
- 测试总量（2026-09-07 第七轮）：**235**。

## 2026-09-07 更新（第八轮：goal 面板 + 客户端契约）

- **扩展契约套件**（`crates/cli/tests/extension.rs`，4 个）：命令注册一致性
  （package.json ↔ extension.js）、goal 视图协议消费（soko/goals + nextHole）
  且客户端禁止文本扫洞、运行时依赖在 dependencies（VSIX P0 回归守护）、
  打包元数据齐全。CI 无需 Electron 即可守护客户端。
- editor/vscode：练习树（soko/goals）+ 状态栏计数 + alt+n/alt+shift+n 跳洞
  （环绕）；AGENTS.md 作为 agent 项目指令入口（薄层，细节在 skills/）。
- 测试总量（2026-09-07 第八轮）：**239**。
