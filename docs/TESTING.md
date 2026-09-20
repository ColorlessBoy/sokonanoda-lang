# TESTING.md — 测试资产地图（交接文档）

测试是本项目的**协作与重构资产**，不是附属品：多个 agent / 多个人会并行改 `front`、`kernel`、`cli`、`lsp`，并且会反复做大规模重构。因此每条流水线层（lex → parse → elab → kernel 检查 → 事件与报告 → CLI/LSP 渲染）都有**独立可跑**的守护测试——改哪一层就只跑哪一层的测试，不用跑全仓库就能确认自己没破坏别人。反过来说，规则是 **TDD 三件套**：给前端加任何语法点 / 错误模式，必须凑齐三件——

1. **front 单测**：`crates/front/src/**` 的 `#[cfg(test)]`，token / parser / compile 各层独立守护 AST 形状、错误码与 kernel 判定；
2. **CLI e2e**：`crates/cli/tests/`，通过真实二进制 + stdin/stdout 守护人类视图与 `--json` 机器视图；
3. **语料**：`examples/*.sokonanoda`，每份教学文件必须整文件通过（corpus 测试自动纳入新文件）。

三件齐全，这个语法点才算"进了课程"。另附两条铁律（详见 `docs/architecture.md` §7）：

- **判定练习靠 kernel，不靠文本比对**（历史上的 `assumption` 文本比对草案已于 2026-09-07 删除）；
- **错误码 + 教学提示是协议的一部分**：每个 `ErrorKind` 必须有稳定 code（`elab-*` / `kernel-*` 前缀）与中文 hint，且要写进 `docs/protocol.md`——这两条由 meta 测试强制执行（见 §2）。

## 1. 分层守护表（改哪层，跑哪层）

| 流水线层 | 守护什么 | 守护测试位置（文件 : 测试名模式） | 怎么跑 |
|---|---|---|---|
| lex | 行注释跳过、`sorry`→Hole、`forall` 关键字 vs 标识符、`->`/`=>`/`:`/`:=`/`@`、宇宙切片 `id.{u}` 的 token 边界、数字、非 ASCII 标识符、`=` 缺 `>` 的错误行列、`#check` 等命令 token | `crates/front/src/token.rs` :: `line_comment_is_skipped`、`sorry_lexes_as_ident_and_parser_treats_as_hole`、`forall_is_a_keyword_but_prefixed_idents_are_not`、`punctuators_lex_in_order`、`universe_application_slice_lexes_as_plain_tokens`、`number_lexes_as_num_token`、`non_ascii_identifiers_lex_as_ident`、`lone_equals_is_an_error_with_position`、`hash_command_lexes_as_ident` | `cargo test -p sokonanoda-front token::` |
| parse | 命名箭头 `(x : A) -> B` = 带 binder 的 Forall、隐式 `{x : A} -> B`、`forall (...), body` 逗号、宇宙参数 `{u, v}` / `{u v}` / `{u} {v}`、重复宇宙参数报错（**跨组也算**）、`axiom` 的声明 binder（G-13）、`example` 拒宇宙参数、声明名不许以 `.` 结尾（G-18）、有冒号的花括号仍是 binder、inductive 块（ctor/rec 字段）、表达式内禁命令关键字、`sorry` 落在 val、诊断行列 | `crates/front/src/parser.rs` :: `named_arrow_parses_as_forall_with_binder`、`implicit_named_arrow_parses_as_forall_with_implicit_binder`、`forall_with_comma_parses_binders_and_body`、`def_parses_two_universe_params`、`duplicate_universe_param_is_rejected`、`universe_params_accept_space_separated_names`、`universe_params_accept_repeated_brace_groups`、`duplicate_universe_param_across_groups_is_rejected`、`named_group_with_colon_is_still_a_binder`、`axiom_with_decl_binders_parses`、`axiom_decl_binders_match_arrow_style_ast`、`axiom_untyped_decl_binder_still_errors`、`example_cannot_declare_universe_params`、`def_cannot_end_with_a_dot`、`inductive_block_parses_ctors_and_recursor`、`command_keyword_inside_expression_is_rejected`、`example_keeps_hole_in_value_position`、`reports_diagnostic_with_span` | `cargo test -p sokonanoda-front parser::` |
| elab | 未知标识符/常量、宇宙层级未声明/个数不符、binder 类型要求、`@id.{0}` 显式宇宙应用、隐式 binder 风格、双宇宙参数 axiom、**axiom 的 binder 糖与箭头写法同判**（G-13）、`{u v}`/`{u} {v}`（G-14）、`ErrorKind` 穷尽性与 code/hint 形状 | `crates/front/src/compile/tests.rs` :: `checks_universe_*`、`rejects_undeclared_universe_variable`、`rejects_wrong_number_of_universe_arguments`、`checks_at_marker_and_implicit_binders_in_py_fol_style`、`parses_implicit_binder_styles`、`checks_axiom_with_two_universe_params`、`checks_axiom_with_decl_binders`、`axiom_decl_binders_match_arrow_style_outcomes`、`checks_space_separated_and_split_universe_params`、`checks_explicit_universe_application`、`every_error_kind_has_stable_code_and_hint` | `cargo test -p sokonanoda-front compile::tests::` |
| L1 prelude（0.59.0；设计 `docs/design/prelude-l1-proposal.md`） | Full 下 30 个名字可用（B1–B7 各用一次、0 errors、20 checked）；`Or`/`And` 是**真归纳块**（`match` 的点号模式与裸模式都过）；让位 = **族粒度 + 依赖闭包**（声明 `And` ⇒ `Iff.*` 消失、`Or`/`Eq` 仍在；只声明 `And.left` ⇒ 整族让位；`ctor Or.inl` 也进 `taken`）；让位**闭包级**（依赖模块声明 `And` ⇒ 入口没有 prelude 的 `And.elim`）；Bare 下没有 L1；`PRELUDE_NAMES` ↔ 实际安装防漂移；建议材料层（`sub_goals` 期望类型）也吃 L1 | `crates/front/src/compile/tests.rs` :: `l1_*`（7 条）、`prelude_names_match_installs`、`eq_symm_is_installed`、`bare_mode_has_no_l1`；`crates/cli/tests/cli.rs` :: `l1_prelude_*`（3 条）、`l1_query_check_counts_match_the_event_stream`；复现件 `docs/gaps/repro/L01-*.sh`、`L02-*.sh` | `cargo test -p sokonanoda-front l1_` && `cargo test -p sokonanoda-cli l1_` |
| kernel 检查 | 整文件过完整 kernel、拒绝带 span、拒绝消息形状（不是 panic 栈）、py-fol / py-nat 移植用例正反两面、显式 inductive / Nat 块、`#check`/`#reduce`/`#print`、原生大整数路径 | `crates/front/src/compile/tests.rs` :: `checks_a_valid_file_end_to_end`、`reports_kernel_rejection_with_span`、`kernel_rejection_message_is_not_a_panic_trace`、`checks_from_scratch_fol_proofs`、`checks_ported_py_fol_core`、`py_core_checks*` / `py_rejects_*` 系列、`explicit_inductive_block_compiles`、`explicit_nat_block_overrides_builtin_prelude`、`ported_nat_fol_add_two_two_reduces`、`checks_nat_literals_and_reduces_addition`；kernel 自身：`crates/kernel/src/tests/*` + `crates/kernel/tests/memory_api.rs` | `cargo test -p sokonanoda-front compile::tests::`；`cargo test -p sokonanoda` |
| 诊断坐标（G-15 / WO-010） | 内核拒绝（`stage=kernel`）的 span == **出错命令**的源码范围（不是 `line >= 1` 那种空断言）；`query check` 的 `failed[]`/`warnings[]` **自带** 1 基 `start_line`/`start_col`/`end_line`/`end_col`，与 `grade --json` 同一诊断的 `span` 逐字段一致；`start`/`end` 仍是**字节** offset（坐标空间 = 入口文件，只加不改）；依赖模块的病不进 `failed[]`（只以入口 `import` 行的 `import-dependency-failed` 出现）；复现脚本的量具只用事件自带行列或 `src.encode()` 的**字节**切片（拿字节 offset 当字符下标 = G-15 的假缺口） | `crates/front/src/compile/tests.rs` :: `reports_kernel_rejection_with_span`（已收紧）、`kernel_rejection_span_is_the_failing_command_range`；`crates/cli/tests/query.rs` :: `query_check_failure_positions_are_typed_and_match_grade`、`query_check_still_reports_a_broken_dependency_in_a_project`；复现件 `docs/gaps/repro/G15-query-check-bare-offsets.sh` | `cargo test -p sokonanoda-front kernel_rejection` && `cargo test -p sokonanoda-cli --test query` |
| 事件与报告 | `CheckEvent` 流（checked/open/typed/reduced/printed）、`DocumentReport` 状态机（Checked/Open/Failed 按源码序、Failed 带 error、errors 计数）、open 练习不污染 env、hover 类型图、`render_expr` 往返 | `crates/front/src/compile/tests.rs` :: `checks_dependent_forall_with_lambda`、`accepts_open_exercise`、`document_report_tracks_open_checked_failed_decls`、`document_report_states_in_source_order`、`open_exercise_does_not_pollute_env`、`document_report_produces_hover_types_for_subexpressions`、`hover_map_covers_subexpressions`、`render_expr_round_trips` | `cargo test -p sokonanoda-front compile::tests::` |
| 「多余的 `sorry`」（`redundant-sorry`） | 候选 = 实参超出函数望远镜且结果展不开箭头；**终审走完整 kernel**（删掉该实参后整条声明能过才报）。守护：正例报 warning 且 span 收窄到 `sorry` token、真缺口/项不对/前瞻引用**不得**误报；探针**不入环境** ⇒ 终审必须显式给可见前缀 `EnvLimit::ByIndex(env_before)`（`ByName(探针名)` 会取 0 ⇒ 空环境 ⇒ 假 `unknown const`）；会话快照按命令缓存**内核终审过的** warning（增量编辑后仍在、span 随前文平移）；洞级 `redundant` 标记在 `query goals`/`holes` 与 LSP `soko/goals` **两视图一致**（agent 据此说"删掉这一行"） | kernel：`crates/kernel/tests/memory_api.rs` :: `synthetic_declaration_needs_an_explicit_environment_limit`；front：`crates/front/src/compile/tests.rs` :: `a_leftover_sorry_*`（正 2 + 反 3）；session：`crates/front/src/session.rs` :: `session_keeps_kernel_verified_warnings_across_edits`；CLI：`crates/cli/tests/protocol.rs` :: `redundant_sorry_warns_and_the_declaration_stays_open`、`crates/cli/tests/cli.rs` :: `cli_redundant_sorry_warns_on_stderr_and_stays_successful`；LSP：`crates/lsp/src/by_sorry_range_tests.rs` :: `redundant_sorry_replaces_the_not_yet_solved_warning`；真相层：`crates/front/src/query/tests.rs` :: `holes_carry_the_redundant_sorry_mark`；两视图：`crates/cli/tests/query.rs` :: `query_and_the_lsp_agree_on_the_redundant_mark` | `cargo test -p sokonanoda-front leftover_sorry` && `cargo test -p sokonanoda-cli redundant` && `cargo test -p sokonanoda-lsp redundant`（`docs/design/redundant-sorry.md`） |
| 输出通道的跨模块归因（0.58.0 合并轮） | 每命令的**平行数组**不变量：`events`↔`event_cmds`、`errors`↔`error_cmds`、**`warnings`↔`warning_cmds`**（内核终审的 warning 是 pass 2 现算的，`split_report` 必须按**命令下标**把它放回产生它的模块——按单元重算语法级 warning 会把它丢掉；语法级 warning 钉在所属单元的命令区间上，不猜 span）。单文件恒等（逐字节不变） | `crates/front/src/compile/tests.rs` :: `warnings_are_attributed_to_the_unit_that_produced_them` | `cargo test -p sokonanoda-front warnings_are_attributed` |
| CLI 人类视图 | stdin/文件批处理、`line:col: error[code]: message` 格式、REPL 声明累积 / `#env` / `#help` / `#prove` | `crates/cli/tests/cli.rs` :: `cli_checks_a_valid_file_via_stdin`、`cli_rejects_a_bad_declaration`、`cli_reports_parse_errors_with_positions`、`cli_prints_*`、`repl_*`、`human_errors_carry_the_pipeline_stage`、`cli_help_is_self_documenting` | `cargo test -p sokonanoda-cli --test cli` |
| CLI JSON 协议 | `--json` 词汇封闭（8 种事件，含 `warning`）、diagnostic 形状（stage/code/hint/span）、`protocol.md` 列出全部事件类型、exercise.open 是成功态、warning 不改退出码 | `crates/cli/tests/protocol.rs` :: `every_example_stream_is_closed_vocabulary`、`kernel_rejection_diagnostic_shape`、`protocol_document_lists_every_emitted_type`、`elab_unknown_identifier_diagnostic`、`open_exercise_is_success_state`、`reserved_declaration_name_warns_but_stays_successful`；另有 `cli.rs` :: `json_mode_*` | `cargo test -p sokonanoda-cli` |
| by tactic | `by` 块白名单（intro/exact/apply/assumption/rfl/**match**/sorry）：`match` 作为 tactic 以当前目标为期望类型判定；`judge_terms` 合成文件保留真实前缀（`match` 宇宙查询可用） | `crates/front/src/parser.rs` :: `match_is_a_tactic_in_a_by_block`；`crates/front/src/compile/tests.rs` :: `by_block_with_match_tactic_checks`、`by_block_with_exact_match_checks`；`crates/cli/tests/cli.rs` :: `cli_by_match_tactic_checks_via_kernel` | `cargo test -p sokonanoda-front by_block_with_match` |
| LSP 协议 | publishDiagnostics / hover / completion / inlay / 自定义请求（goals/nextHole/hints/stateAt/version）/ codeLens / quick-fix 全部有进程内 rpc 测试（`test_service` 走完整 tower-lsp 栈）。codeLens：`code_lens_reflects_exercise_status`（标题）、`code_lens_ranges_match_each_declaration`（range + `sokonanoda.status`）。quick-fix 四族：exact（`code_action_offers_exact_for_matching_hypothesis`、`code_action_spine_hole_exact_targets_its_own_hole`、`code_action_exact_uses_kernel_defeq_not_text_match`）、refine（`code_action_offers_kernel_shaped_refine_skeleton`、`code_action_refine_edit_targets_the_hole`）、intro（`code_action_offers_intro_on_open_exercise`、`code_action_intro_still_offered_without_matching_hypothesis`）、restart/reset（`code_action_restarts_failed_decl_over_the_whole_value_span`、`code_action_restart_replaces_multiline_values_as_a_whole`、`code_action_non_lambda_answer_gets_only_restart`、`code_action_failed_eq_decl_offers_kernel_verified_rfl_first`）；无候选回退：`code_action_open_goal_without_a_next_step_offers_none`、`code_action_failed_decl_without_peelable_type_gets_none` | `crates/lsp/src/tests/`（0.56.1 按特性拆分：`hover` / `hover_brackets` / `lenses` / `navigation` / `state` / `goals` / `lifecycle` / `tokens` / `perf`；共享夹具与再导出在 `tests/mod.rs`，**断言与测试名一字未改**）、`by_sorry_range_tests.rs`、`crates/lsp/src/inlay.rs`、`actions.rs`；进程内 rpc 的 `test_service` 等 helper 在 `crates/lsp/src/testutil.rs` | `cargo test -p sokonanoda-lsp --lib` |
| 多文件项目（import 闭包） | **内存覆盖**（打开文档的未落盘文本进闭包，且进摘要：`an_overlay_makes_unsaved_dependency_edits_visible`）/ **无清单也能 import**（模块根 = 入口目录）/ 清单把根上移（嵌套模块名 `Foo.Bar`）/ 缺模块报期望路径 + `-` hint / **大小写不符也报错**（大小写不敏感文件系统上不能静默命中）/ 环报出环路径 / **依赖失败阻断导入者且只报一条** / 闭包级重名与 prelude 冲突 / 依赖里的开放练习变成 warning / `--bare` 传染整个闭包 / 开放练习不进导入者环境 / 清单损坏是项目错误不是 panic / 闭包摘要稳定且对依赖敏感 | `crates/front/src/project/tests.rs` :: `imports_work_without_any_manifest`、`manifest_moves_the_module_root_and_nested_names_resolve`、`missing_module_reports_the_path_and_dash_hint`、`case_mismatch_is_an_error_even_on_case_insensitive_filesystems`、`cycles_are_reported_with_the_cycle_path`、`a_failed_dependency_blocks_its_importers_with_one_diagnostic`、`a_shared_prelude_module_suppresses_the_builtin_nat_for_the_closure`、`a_dependency_with_a_conflicting_prelude_directive_is_reported`、`a_duplicate_name_across_modules_is_a_collision_not_a_kernel_panic`、`open_exercises_in_a_dependency_warn_on_the_import_line`、`bare_entry_makes_the_whole_closure_bare`、`open_exercises_do_not_leak_into_the_importer`、`a_broken_manifest_is_a_project_error_not_a_panic`、`closure_digest_is_stable_and_dependency_sensitive` | `cargo test -p sokonanoda-front project::` |
| 单文件 vs 项目（自动区分） | 无 `import` 的文件**从不读清单**（同目录坏清单、`--root`、`--no-project` 全是空操作）；同一个坏清单在有 `import` 的文件上必须报 `manifest-invalid` 且仍以入口目录编完；**依赖自己的清单永不参与**；零配置能 import；stdin 与文件逐字节一致、stdin 带 `import` 给 `--root` 提示 | `crates/cli/tests/single_file_vs_project.rs` :: `a_single_file_never_reads_a_manifest`、`the_same_manifest_does_bite_once_the_file_has_an_import`、`a_dependency_module_never_contributes_its_own_manifest`、`zero_config_imports_still_work_and_stdin_matches_a_file` | `cargo test -p sokonanoda-cli --test single_file_vs_project` |
| 多文件 CLI 端到端 | 两文件项目真跑通；**A1：无 import 的文件与单文件路径逐字节一致**；JSON 里依赖诊断带 `file`/`module`；失败依赖只阻断一次；缺模块在 import 行报期望路径；`--root` 从别处指定模块根；stdin + import 无 root 是清晰错误；`--no-project` 忽略清单、根回入口目录；清单 `requires` 不符只 warning；`build` 走整项目并按文件报状态；`query` 在闭包里解析导入名；**项目缓存命中 + 依赖改动必 miss** | `crates/cli/tests/imports.rs` :: `cli_compiles_a_two_file_project_without_any_manifest`、`import_free_files_are_byte_identical_to_the_single_file_path`、`dependency_diagnostics_carry_the_file_path_and_module_in_json`、`a_failed_dependency_blocks_the_entry_with_one_diagnostic`、`missing_module_reports_the_expected_path_on_the_import_line`、`root_flag_points_at_the_module_root_from_elsewhere`、`stdin_with_imports_without_root_is_a_clear_error`、`no_project_ignores_the_manifest_and_uses_the_entry_directory`、`manifest_requires_mismatch_is_a_warning_not_an_error`、`build_walks_a_project_and_reports_per_file_status`、`query_resolves_imported_names_in_the_closure`、`a_project_cache_hits_and_a_dependency_change_invalidates_it` | `cargo test -p sokonanoda-cli --test imports` |
| 多文件 LSP（P5） | **项目入口有 quick-fix**（导入的构造子参与 refine/exact：`code_actions_work_in_a_project_entry`）；**编辑器外改动自动刷新**（`workspace/didChangeWatchedFiles`：`an_external_change_to_a_dependency_refreshes_the_open_entry`）；**工作区里嵌套的项目按自己的清单解析**（工作区根不当模块根）；被 import 的声明对入口可见（无诊断）；缺失 import 是诊断不是崩溃；`textDocument/definition` 能跳进被导入模块，**入口自己声明的名字仍留在入口**（项目模式不吞本地跳转）；**跨文件 `references`**（定义排最前、模块按拓扑序）；**跨文件 `rename`**（两份文件各一份 `TextDocumentEdit`，打开的带版本、未打开的为 null；改成项目里已有的名字先被拦下）；**改依赖 ⇒ 下游入口自动重编译重发**（未落盘的依赖编辑通过内存覆盖可见，改回来能恢复）。测试要用 `testutil::notify_with_drain`（见 §5.7） | `crates/lsp/src/tests/project.rs` :: `an_imported_module_is_visible_to_the_entry`、`a_missing_import_is_a_diagnostic_not_a_crash`、`definition_jumps_into_the_imported_module`、`definition_stays_in_the_entry_for_local_names`、`references_cross_files_include_the_declaring_module`、`rename_rewrites_the_dependency_and_the_entry`、`rename_rejects_a_name_that_already_exists_in_the_project`、`editing_a_dependency_refreshes_the_open_entry`、`a_nested_project_resolves_against_its_own_manifest`、`project_request_describes_the_closure_of_the_requested_document`、`project_request_follows_the_unsaved_buffer_and_reports_failures`；纯逻辑：`crates/lsp/src/project_refs.rs` :: `references_span_modules_in_closure_order`、`rename_edits_cover_every_module_that_uses_the_name` | `cargo test -p sokonanoda-lsp --lib project` |
| 项目状态视图（0.58.0 批次 4） | `QueryDoc::project_view()` 是唯一真相：闭包模块表（拓扑序、入口标记、`import` 边、声明/错误/练习计数）+ **状态三分**（`compiled` / `load-failed` = 根因 / `blocked` = 被上游拖住）+ 项目级诊断 + 根与清单来源；单文件是**另一种合法状态**（`null` + `no-imports`/`no-path`/`parse-error`）；路径 canonicalize 成绝对路径；只读派生（不重跑内核）。CLI `query project`（信封 `soko.query/1`、恒退出 0）与 LSP `soko/project`（回显 uri/version）同源；VS Code 项目树是它的渲染（含"答案指名别的文档 ⇒ 丢弃"） | `crates/front/src/query/tests.rs` :: `project_view_lists_the_closure_with_statuses`、`project_view_reports_the_manifest_when_one_exists`、`project_view_separates_load_failure_from_being_blocked`、`project_view_is_none_for_single_files_with_a_reason`；`crates/cli/tests/project_features.rs` :: `query_project_describes_root_manifest_and_module_statuses`、`query_project_separates_the_broken_module_from_the_blocked_ones`、`query_project_answers_null_with_a_reason_for_single_files`；`crates/lsp/src/tests/project.rs` :: `project_request_*`；`editor/vscode/test-extension-host.js` :: `the project tree renders the closure the server describes` 等 3 例 | `cargo test -p sokonanoda-front --lib project_view` && `cargo test -p sokonanoda-cli --test project_features query_project` && `cargo test -p sokonanoda-lsp --lib project_request` && `node editor/vscode/test-extension-host.js` |
| 课程共享库（`course/shared/`） | 三个规范模块（And / Or / Nat）可单独编译；`Demo.sokonanoda` 走真闭包（跨模块公理、**跨模块 match**、跨模块递归 + `#reduce`）；**双向漂移守护**：规范文本与 24/12/8 份副本逐字一致（少了=有副本没跟上，多了=新副本没登记）；单元画布不得出现 `import`（自给自足是教学属性） | `crates/cli/tests/course_shared.rs` :: `shared_modules_compile_standalone`、`shared_demo_compiles_through_the_import_closure`、`every_unit_copy_matches_the_canonical_module`、`the_shared_library_replaces_rather_than_duplicates_the_canvas_purpose` | `cargo test -p sokonanoda-cli --test course_shared` |
| 语料 | 每份 `examples/*.sokonanoda` 必须被真实 CLI 整文件通过 | `crates/cli/tests/examples.rs` :: `every_example_lesson_is_a_valid_sokonanoda_file` | `cargo test -p sokonanoda-cli --test examples` |
| 内核真相查询（`front::query`） | **唯一语义源**（LSP / CLI / MCP 都调它）：`state` 的 `goalsAt?` 选择（tactic 命中用**半开区间**；根状态 = 声明类型 + 空 binders；**无 `by` 的声明退回声明级目标 + 上下文**，已闭合则空）、洞 `id` 的稳定性与唯一性、`QueryError` 与「正常的没有」的区分（不可解析 / 不在声明内 / 越界）、坐标换算（UTF-16 列） | `crates/front/src/query/tests.rs` :: `state_at_root_before_any_tactic`、`state_at_inside_a_tactic_shows_the_entering_state`、`state_at_after_apply_lists_every_sub_goal`、`state_at_open_declaration_without_a_by_block_keeps_its_context`、`state_at_closed_declaration_without_a_by_block_has_no_goal`、`holes_are_addressable_and_stably_identified`、`check_counts_match_the_event_stream`、`next_hole_walks_forward_and_backward`、`probe_fills_sub_goal_types_that_the_walk_cannot_determine` | `cargo test -p sokonanoda-front --lib query::` |
| `query` 子命令 + 两视图一致性 | 单 JSON 对象信封（`soko.query/1`）、退出码（0 答上了 / 1 内核拒绝 / 2 用法）、`ok:false` **不是**空结果、`--text` 支持未落盘中间态；**`query check` 计数 ≡ `--json` 事件计数**；**`query state` ≡ 真实 LSP 二进制的 `soko/stateAt`**（逐字段：step/total/goal/binders/goals），覆盖根状态 / tactic 之内 / tactic 之后 / **无 `by` 的开放与闭合** | `crates/cli/tests/query.rs` :: `query_check_counts_match_the_json_event_stream`、`query_state_agrees_with_the_lsp_state_at_request`、`query_state_and_lsp_agree_without_a_by_block`、`query_state_outside_a_declaration_is_a_structured_error`、`query_holes_lists_stable_ids_and_navigates`、`query_usage_errors_exit_two_without_a_payload` | `cargo test -p sokonanoda-cli --test query`（**先 `cargo build --workspace`**：它 spawn `target/<profile>/sokonanoda-lsp`，旧构件会对拍出假红） |
| MCP 桥（DSH） | `dsh/mcp/server.js` 七工具与 `query` 七个 op 一一对应；`server/discover` **立刻**回 `-32601`（沉默 = 60 s 超时）；`initialize` 回显 legacy 版本并 advertise `capabilities.tools`；未知方法/工具 `-32601`；工具失败 `isError:true`；patch 与 `scripts/soko mcp` 接线 | `crates/cli/tests/dsh.rs` :: `dsh_mcp_server_forwards_every_query_op` | `cargo test -p sokonanoda-cli --test dsh` |
| K 目标判据（镜像内核） | 单构造子 `Prop` / 索引 / 字段数的**判别性形状**都要与内核 `init_k_target` 逐字一致（字段数 == 参数数不是 K 目标；无字段的常量索引族**是** K 目标）；箭头写法索引字段能派生递归子并真归约（`match` + `#reduce`） | `crates/front/src/compile/tests.rs` :: `single_constructor_prop_derives_recursor`、`single_constructor_prop_with_fields_is_not_a_k_target`、`single_constructor_indexed_prop_without_fields_is_a_k_target`、`indexed_inductive_with_arrow_style_field_derives_recursor`、`indexed_inductive_with_named_field_derives_recursor`、`arrow_style_indexed_recursor_reduces`；`crates/cli/tests/cli.rs` :: `cli_inductive_accepts_multi_name_binder_groups`、`cli_arrow_style_indexed_field_derives_and_reduces`、`cli_single_constructor_prop_derives_recursor` | `cargo test -p sokonanoda-front k_target` && `cargo test -p sokonanoda-cli --test cli -- inductive` |
| 派生 recursor 的**宇宙参数**判据（镜像内核 `large_elim_test`，G-03 / 0.59.0） | 无 `rec` 的块派生的 recursor 必须带**内核算出的**宇宙参数个数：`inductive Bar (A : Type) : Prop` + `ctor mk (a : A) : Bar A` ⇒ 0 个（motive 落 `Prop`）；判别性成对形状 **P3 vs P9**（字段是不是结果的索引）一个 0 一个 1；**P10/P11/P13/P14**（字段类型是 `P -> Q` / `forall (x : Nat), P` / 具名 `def … : Prop` / 具名 Prop 定义的应用）都必须保持 1 个——"字段类型语法上像不像 Prop"的近似会把它们翻面；整份 WO-006 语料 16/16 `decl.checked` | `crates/front/src/compile/tests.rs` :: `prop_type_param_single_ctor_derives_a_zero_universe_recursor`、`derived_recursor_universe_mirrors_the_kernel_large_elim_test`（表驱动 P1–P14）、`discriminative_pairs_pin_the_large_elim_test_to_the_kernel`、`zero_universe_prop_recursor_computes_through_match`；`crates/cli/tests/cli.rs` :: `cli_accepts_prop_inductive_with_type_parameter`、`cli_prop_recursor_without_universe_parameter_computes`、`cli_rejects_universe_argument_on_a_zero_universe_recursor` | `cargo test -p sokonanoda-front large_elim` && `cargo test -p sokonanoda-front prop_type_param` && `cargo test -p sokonanoda-cli --test cli -- prop_` |
| **用户自定义记法（0.59.0；设计 `docs/design/notation-subset.md`，台账 G-04 第一刀）** | 符号是**独立 token**（`Sym` 码点类 `U+2200–22FF`/`U+2A00–2AFF`/`\`，最大咬合；`∀` 仍是 `Forall`）；字符串字面量只用于声明里的符号文本，未闭合报 `unterminated-string`（span 在开引号）；四条命令 `infix:N`/`infixl:N`/`infixr:N`/`notation` 的优先级梯子（`p`/`p+1`、`p`/`p+1` 左结合、`p+1`/`p`）与非结合同级链报错；符号文本去空白后不能全是标识符字符（`notation-shape`）、未声明符号报 `notation-unknown-symbol`、重复声明同一符号报错；**两种写法判卷一致**（点名 vs 记法：退出码 + 五元计数）；**护城河**：点名省 `α`（`Set.mem a A`）仍被内核拒绝（`kernel-rejected`/stage kernel）；前导类型参数补全（先操作数类型、再期望类型；`#check ∅` 报 `elab-notation-argument-unsolved`）；记法**不产生事件**、不进声明表、不进 goal 视图；`SemanticKind::ALL`/`tm_scope` **逐字节不变**（符号分类为 `Keyword`，未声明符号不产生 run）；**课程两版同判**（真单元②：画布是记法版、夹具造点名版，退出码 + 五元计数相等；R2 课程 Lean 化之后契约方向翻转，见 `docs/design/notation-subset.md` §15） | front：`crates/front/src/token.rs` :: `math_symbol_is_its_own_token` 等 7 条；`crates/front/src/parser.rs` :: `infix_*`/`notation_*`（14 条）；`crates/front/src/compile/tests.rs` :: `notation_*`（8 条）；`crates/front/src/semantic.rs` :: 记法 3 条；CLI：`crates/cli/tests/notation.rs` :: `notation_and_pointful_canvases_grade_identically`、`notation_emits_no_new_event_kinds`、`the_pointful_spelling_keeps_working_and_the_moat_holds`、`undeclared_symbol_is_a_parse_diagnostic_with_a_teaching_hint`、`notation_command_alone_is_a_clean_grade_with_checked_declarations`、`notation_nullary_without_an_expected_type_reports_the_unsolved_code`、`stdin_and_a_file_agree_on_a_notation_canvas`、`a_real_course_unit_grades_identically_in_either_spelling`、`the_shipped_course_uses_the_library_notation`、`inequality_delta_unfolding_carries_the_right_universe_level`、`a_broken_declaration_does_not_poison_the_next_notation`、`anonymous_constructors_pick_the_constructor_from_the_expected_type`、`an_anonymous_constructor_without_an_expected_type_reports_its_own_code`、`intro_renames_the_bound_variable_in_the_rest_of_the_goal`、`goals_whose_head_is_a_def_unfold_through_several_layers`；`crates/cli/tests/protocol.rs` :: `notation_diagnostics_stage_as_parse_and_elab` | `cargo test -p sokonanoda-front --lib notation` && `cargo test -p sokonanoda-cli --test notation` |
| **记法第二刀（0.60.0；设计 `docs/design/notation-subset.md` §10–§12，台账 G-04）** | `prefix:N` / `postfix:N` 两条一元命令（N 必填 1–1000，与 `infix` 族同诊断）；**优先级**：`prefix:N` 的操作数按 `parse_operators(N)` 解析、`postfix:N` 在爬升里 `N >= min_precedence` 才吸收 ⇒ **N 越大绑得越紧**（`postfix:100` 的 `A ∪ Bᶜ` = `A ∪ (Bᶜ)`、`postfix:50` 的同一个式子 = `(A ∪ B)ᶜ`）；**声明驱动的词法**：`scan_notation_symbols`（跳过注释/字符串，与 parser 共用同一份符号合法性判据）+ `tokenize_with_symbols` 最长匹配，`𝒫`/`ᶜ`/`⁻¹'`/`×ˢ` 这些标识符字符与 `''`（`'` 单独出现给 `Sym`）都读得出来，标识符内部也断开（`Aᶜ` = `Ident(A)+Sym(ᶜ)`）；**跨 `import` 传播**：闭包加载器先收 import 边（`scan_import_lines`，词法级）、先访问依赖，再把依赖**导出**的记法表当继承表解析自己；记法表按 import 边合并（没 import 的不泄漏）；重声明继承来的符号是 parse 错误；`project::is_project_source` 让「入口单独 parse 失败但写了 import」也走闭包（只认 `notation-unknown-symbol` 一条码——放宽会把别的解析诊断退化成 `import-module-invalid`）；`by` 块目标里的记法现在判得动（判卷通道从前缀 `FolFile` 收记法声明）；一元记法在实参位要加括号（`f (𝒫 A)`）；五个符号判卷两种写法计数逐一相等；课程库声明五个符号、单元③⑧ 各一条 `example` 演示（**课程计数逐项不变**） | front：`crates/front/src/token.rs` :: `scan_finds_every_declared_symbol`、`scan_skips_comments_and_strings_that_are_not_notation_symbols`、`declared_symbols_lex_as_sym_with_longest_match`、`tokenize_without_symbols_is_byte_identical_to_the_first_cut`；`crates/front/src/parser.rs` :: `prefix_and_postfix_commands_parse_into_one_ast_node_each`、`prefix_and_postfix_require_a_precedence`、`declared_symbols_are_reserved_for_the_whole_file`、`a_prefix_symbol_in_operator_position_is_a_teaching_error`；`crates/front/src/compile/tests.rs` :: `prefix_postfix_and_pointful_spellings_compile_identically`、`prefix_precedence_decides_where_the_operand_stops`、`postfix_precedence_decides_where_it_binds`、`a_loose_postfix_binds_outside_the_binary_operator`、`unary_notation_emits_no_events_and_is_not_a_declaration`、`unary_notation_with_two_leading_parameters_is_completed_from_the_operands`、`rfl_on_a_notation_goal_keeps_its_grouping`、`a_by_block_whose_goal_carries_notation_still_judges`；`crates/front/src/project/tests.rs` :: `a_dependency_notation_is_usable_in_the_entry`、`a_notation_from_a_module_that_was_not_imported_is_not_visible`、`redeclaring_an_inherited_notation_is_a_dedicated_error`；CLI：`crates/cli/tests/notation.rs` :: `the_five_second_cut_symbols_grade_clean_in_both_spellings`、`a_library_notation_works_in_the_entry_through_import`、`the_shipped_course_library_declares_the_five_symbols`、`the_shipped_course_uses_the_library_notation_in_a_demo` | `cargo test -p sokonanoda-front --lib -- prefix postfix notation` && `cargo test -p sokonanoda-cli --test notation` |
| **namespace / open（0.60.0 第一刀 + 0.60.x 第二刀；设计 `docs/design/namespace-open.md`，台账 G-05）** | 三条命令 `namespace <name>` / `end <name>` / `open <name>`（可点分）；**声明名在 parser 里加前缀**（`namespace A` 里 `def mem` ⇒ 全局名 `A.mem`；`def Set.mem` ⇒ `A.Set.mem` 拼接）；引用候选顺序 = ① 命名空间链从内到外（`A.B.x` → `A.x`）② 精确名 ③ `open` 前缀（按 open 顺序），命中即止；`open` 是集合不是快照（之后/之前声明的都享受）；**三条命令零事件、不进声明表、不进 `top_level_def_spans`**；`end` 错配 / 无对应 `namespace` / 裸 `end` 报 `parse-namespace-mismatch` / `parse-namespace-shape`，文件尾未闭合报 `parse-namespace-unclosed`（span 指回那条 `namespace`）；作用域**文件内**：`open` 不跨 `import`（依赖里的 `open` 必须**不**泄漏进入口），但被导入模块的**全局名**本来就可见 ⇒ 入口 `open Set` 对依赖声明的 `Set.mem` 有效；判卷合成走 `parse_fragment`（容忍片段里未闭合的 `namespace`），用了 namespace/open 的文件里 `by` 块根目标先过内核 pp（`judge_render_type`）⇒ `exact` 与 `apply` 在命名空间里判定一致；未用命名空间的文件零额外开销；课程 `lib/Set.sokonanoda` 已包进 `namespace Set`（全局名零改动、外部引用零改动、门禁计数不变）；**第二刀**：`open Foo (a b)`（only）/ `open Foo hiding a b` / `open Foo renaming a => b` 三条**互斥**子句（先过滤后改名、改名是替换），`open Foo … in <命令>` 局部（限叶子命令，`Walk` mark/rollback 撤销，合成前缀补一行源码原文的 open 头），`export Foo [<子句>]` 文件内与 `open` 逐字相同 + **跨 `import`**（单元切换重放导出表；`open` 不跨）；遮蔽给 `open-shadowed-name` **warning**（语法级、只看本文件、不改退出码）；`open … in <声明>` 包住的声明由 `ast::effective_commands` 展开给所有声明级 pass；`section`/`variable` 实测**做不动**（应用逐位显式、无隐式参数插入，设计 §N9）；`namespace` 跨文件传播**已澄清无需做** | front：`crates/front/src/parser.rs` :: `namespace_end_open_are_commands`、`nested_namespaces_accumulate_the_prefix`、`a_dotted_namespace_prefixes_like_two_nested_ones`、`a_dotted_declaration_name_is_concatenated_inside_a_namespace`、`end_mismatch_is_a_dedicated_parse_error`、`end_without_an_open_namespace_is_a_dedicated_parse_error`、`an_unclosed_namespace_is_reported_at_eof`、`namespace_shape_errors_are_dedicated`、`parse_fragment_tolerates_an_unclosed_namespace`、`namespace_and_open_are_reserved_inside_expressions`；`crates/front/src/compile/scope.rs` :: `nested_and_dotted_namespaces_agree`、`candidates_follow_the_documented_order`、`open_is_a_set_and_reset_clears_both_halves`；`crates/front/src/compile/tests.rs` :: `namespace_short_names_and_pointful_names_grade_identically`、`namespace_resolution_prefers_the_longest_namespace_prefix`、`namespace_resolution_falls_back_to_the_exact_name`、`open_makes_a_prefix_omissible_and_sees_later_declarations`、`open_order_decides_between_two_candidates`、`a_namespace_reference_that_is_nowhere_is_the_ordinary_unknown_identifier`、`namespace_coexists_with_ctor_namespaces`、`a_by_block_inside_a_namespace_resolves_short_names`、`namespace_commands_emit_no_events_and_are_not_declarations`、`print_check_and_reduce_inside_a_namespace_resolve_short_names`、`hover_resolution_inside_a_namespace_points_at_the_global_declaration`、`open_only_hiding_and_renaming_resolve_the_right_names`、`open_only_keeps_the_other_short_names_out`、`open_renaming_takes_the_original_short_name_away`、`open_in_is_local_to_that_one_command`、`open_in_body_is_still_a_real_declaration_for_templates_and_hover`、`export_short_names_are_visible_to_later_declarations`、`open_shadowed_names_warn_but_do_not_fail`、`a_short_name_colliding_with_the_root_declaration_warns`、`a_non_colliding_open_produces_no_warning`、`open_and_export_commands_stay_event_free`；`crates/front/src/parser.rs`（第二刀）:: `open_clauses_parse_into_the_filter`、`open_clause_names_may_collide_with_command_keywords_in_other_positions`、`open_in_wraps_the_next_command_and_records_the_header`、`nested_open_in_chains_unwrap`、`export_parses_like_open_without_in`、`open_and_export_clause_shape_errors_are_dedicated`；`crates/front/src/compile/scope.rs`（第二刀）:: `only_hiding_and_renaming_filter_the_candidates`、`local_open_rolls_back_and_keeps_an_already_open_prefix`；CLI：`crates/cli/tests/namespace.rs` :: `open_reaches_a_declaration_from_an_imported_module`、`open_does_not_leak_out_of_the_module_that_wrote_it`、`namespace_commands_are_not_declarations`、`end_mismatch_is_a_dedicated_parse_error_with_exit_1`、`an_unclosed_namespace_is_a_dedicated_parse_error_with_exit_1`、`open_in_is_local_to_one_command`、`open_only_hiding_and_renaming_grade_like_the_pointful_names`、`export_reaches_the_importing_file_while_open_does_not`、`open_shadowed_names_warn_but_still_exit_0`、`open_clause_shape_errors_are_dedicated_with_exit_1`；复现：`docs/gaps/repro/G05-namespace-open.sokonanoda` | `cargo test -p sokonanoda-front --lib namespace` && `cargo test -p sokonanoda-front --lib open_` && `cargo test -p sokonanoda-cli --test namespace` |
| **`abbrev`（G-08 / 0.60.0；设计 `docs/design/abbrev.md`）** | `abbrev` 与 `def` **同语义**（真 Lean 子集兼容：真 Lean 的 `abbrev` 行可原样编）：同 AST（`Command::Def`，`abbrev` 不是新节点）、同宇宙参数/声明级 binder/`namespace` 前缀、同 `sorry` ⇒ `exercise.open`；**实测结论**：本语言里 `def` 与 Lean 的 `abbrev` **没有可观察差异**（类型位/项位透明、`#reduce` 展开、hover/`#check` 保留别名名、项层 `rfl` 判等、递归两边都不支持；唯一差异 reducibility hint 在本语言只有一个透明度层级 ⇒ 不可观察）；`abbrev` 在**表达式位**是 parse 诊断（`unexpected-token`，名字位与 `def theorem …` 一样宽松——既有语义）；CLI help 自文档化 | front：`crates/front/src/parser.rs` :: `abbrev_and_def_produce_the_same_command`、`abbrev_is_a_reserved_command_not_an_identifier`、`abbrev_inside_a_namespace_gets_the_prefix`；`crates/front/src/compile/tests.rs` :: `abbrev_and_def_compile_identically`、`abbrev_is_transparent_in_type_positions`、`abbrev_unfolds_under_reduce_and_by_rfl`、`abbrev_with_a_sorry_value_is_an_open_exercise_like_def`；CLI：`crates/cli/tests/cli.rs` :: `cli_abbrev_and_def_grade_identically`、`cli_abbrev_canvas_is_a_clean_human_grade`、`cli_abbrev_in_an_expression_is_a_parse_error`、`cli_help_is_self_documenting`（加了 `abbrev <name>` 断言）；复现：`docs/gaps/repro/G08-abbrev.sokonanoda` | `cargo test -p sokonanoda-front --lib abbrev` && `cargo test -p sokonanoda-cli --test cli abbrev` |
| 文档一致性 | `docs/protocol.md` 必须列出每个 `ErrorKind` 的 code；parse 两个 code 也在文档里 | `crates/front/src/compile/tests.rs` :: `protocol_doc_lists_every_error_code`（自带不带通配的穷尽清单） | `cargo test -p sokonanoda-front protocol_doc` || perf 冒烟 | 原生大整数路径（39 位大数 +1 归约）与 iota 链（`add two two` = 4 层 succ）不退化；30s canary 挡 debug 构建下的意外爆炸 | `crates/front/src/compile/tests.rs` :: `perf_smoke_native_and_iota_reduce` | `cargo test -p sokonanoda-front perf_smoke` |
| proof/tactic | `#prove` 的 intro/exact/lambda 搭建；生成的 lambda 必须被完整 kernel 接受 | `crates/front/src/proof.rs` :: `intro_builds_lambda_text`、`exact_fills_the_hole`、`generated_lambda_passes_the_kernel` | `cargo test -p sokonanoda-front proof::` |
| 值位关键字移除 | 0.22.0 移除 `funapply`（性能）、0.27.0 移除 `funintro`（语义冗余）：值位只保留普通表达式与 `by` 块；`by` 块 tactic `intro`/`apply` 不受影响；`elab-intro-not-a-function` 错误码随之删除；`funintro` 现为未定义标识符 | `docs/design/remove-funintro.md`；`crates/cli/tests/cli.rs` :: `cli_value_funintro_is_no_longer_a_keyword` | `cargo test -p sokonanoda-cli value_funintro` |
| lambda 尾关键字 | `fun … => by …`：lambda 体尾部按值位解析 `by`（parser-only，降低沿链下降零改动）；拆完 binder 直接用 tactic 继续 | `crates/front/src/parser.rs` :: `parse_lambda`；`crates/front/src/compile/tests.rs` :: `by_in_a_lambda_tail_*` | `cargo test -p sokonanoda-front by_in_a_lambda_tail` |
| by-tactic 块 | `theorem t : T := by <tactic>; …` 的解析/引擎/kernel 判定：intro+exact / assumption / apply（单、多子目标）/ rfl / `by sorry` 占位 → Open；错误路径（未知 tactic、intro 非函数、assumption 无匹配、rfl 非 Eq、apply 头不匹配）→ `elab-tactic-failed`；多名字 binder 组回读 | `crates/front/src/compile/tests.rs` :: `*by_*`、`parses_multi_name_binder_group`；`crates/cli/tests/cli.rs` :: `cli_checks_by_tactic_blocks`、`cli_by_tactic_partial_block_is_open_exercise` | `cargo test -p sokonanoda-front by_` && `cargo test -p sokonanoda-cli by_tactic` |
| 服务器自述 | `soko/version` 返回 `{version, pid}`，供扩展的重启命令在重启前后各问一次——旧 pid 消失 + 新 pid 出现 + 版本变化，把「旧进程退出、新进程是新版」变成可见事实（扩展更新后跑旧服务器是最常见的困惑） | `crates/lsp/src/lib.rs` :: `Backend::version`；`crates/lsp/src/tests/lifecycle.rs` :: `version_request_reports_version_and_pid`；`editor/vscode/extension.js` :: `restartServer` 的重启前/后回执 | `cargo test -p sokonanoda-lsp --lib -- version_request` |
| 声明级 binder | `theorem f (a : A) (h : B a) : C := v` 的解析/降级（Forall 类型 + Lambda 值）、`:= sorry` 直出 codomain 目标与上下文、无 fun 闭合、`by` 初始上下文、宇宙参数/隐式 binder 消歧、无类型 binder 教学报错 | `crates/front/src/parser.rs` :: `decl_binders_*`、`untyped_decl_binder_*`、`universe_params_before_*`、`example_with_decl_binders_*`；`crates/front/src/compile/tests.rs` :: `decl_binders_*`；`crates/cli/tests/cli.rs` :: `cli_decl_binders_*`、`cli_untyped_decl_binder_*`；`crates/lsp/src/inlay.rs` :: `decl_binder_hole_shows_the_codomain_goal` | `cargo test -p sokonanoda-front decl_binder` && `cargo test -p sokonanoda-cli decl_binder` |
| `axiom` 的 binder 参数表（G-13）＋宇宙参数组（G-14）＋声明名尾点（G-18） | `axiom` 与 def/theorem 同序吃 binder（类型侧折算，无 Lambda）、binder 与箭头写法同 AST/同判定、无类型 binder 仍教学报错；`{u v}`（空格）与 `{u} {v}`（连排）等价于 `{u, v}`、跨组重名报 `duplicate universe parameter`、有冒号的花括号仍是 binder、`example` 拒宇宙参数（明确文案）；`def f.{u}` 是 parse 错误（不再静默吃成 `f.`），点分限定名照常 | `crates/front/src/parser.rs` :: `axiom_with_decl_binders_parses`、`axiom_decl_binders_match_arrow_style_ast`、`axiom_untyped_decl_binder_still_errors`、`universe_params_accept_space_separated_names`、`universe_params_accept_repeated_brace_groups`、`duplicate_universe_param_across_groups_is_rejected`、`named_group_with_colon_is_still_a_binder`、`example_cannot_declare_universe_params`、`def_cannot_end_with_a_dot`；`crates/front/src/compile/tests.rs` :: `checks_axiom_with_decl_binders`、`axiom_decl_binders_match_arrow_style_outcomes`、`checks_space_separated_and_split_universe_params`；`crates/cli/tests/cli.rs` :: `cli_axiom_decl_binders_compile`、`cli_axiom_untyped_decl_binder_is_a_parse_error`、`cli_axiom_curried_forms_still_compile`、`cli_space_separated_and_split_universe_params_compile`、`cli_universe_param_name_cannot_end_with_a_dot`；复现：`docs/gaps/repro/G13-axiom-binder-params.sh`、`docs/gaps/repro/G14-single-universe-binder.sokonanoda` | `cargo test -p sokonanoda-front axiom` && `cargo test -p sokonanoda-front universe_params` && `cargo test -p sokonanoda-cli --test cli -- axiom` |
| 值位 `let` | `let x : T := v; body` 的 parse（binder/ty/val/body/span、右结合嵌套、`fun` body / 括号内、丢分号/丢 `:=`/丢名字报错、`let` 不让路成应用实参）与 elab（binder scope/shadow、期望类型推理、`#check`/`#reduce`、值位/body 洞子目标、zeta 契约与 beta 展开 status/goal 一致）；缺注解 `let x := v` 报 `elab-untyped-binder` + `let` 写法 hint | `crates/front/src/parser.rs` :: `let_*`；`crates/front/src/compile/tests.rs` :: `let_*`；`crates/cli/tests/cli.rs` :: `json_mode_let_*` | `cargo test -p sokonanoda-front let_` && `cargo test -p sokonanoda-cli let_` |
| `sokonanoda build` | build 冷/热/clean/目录递归；`course`/`--json` 走缓存后结果不变（冷热逐字节一致） | `crates/cli/tests/cli.rs` :: `cli_build_warms_and_reuses_cache`、`cli_build_clean_removes_entries`、`cli_course_is_stable_with_a_warm_cache` | `cargo test -p sokonanoda-cli build` |
| 高亮单一起源 | `SemanticKind::tm_scope` 穷尽；TM 语法含每个 scope；CSS 含每个 `.tok-*`；hover 文本 == stateAt runs 投影 | `crates/front/src/semantic.rs` :: `tm_scope_is_total`、`runs_to_text_round_trips`、`goal_runs_projects_*`；`crates/lsp/src/tests/{state.rs :: hover_goal_text_equals_the_state_at_run_projection, tokens.rs :: every_semantic_kind_maps_to_a_legend_entry}`；`crates/cli/tests/extension.rs` :: `tm_grammar_declares_every_semantic_scope` | `cargo test -p sokonanoda-front tm_scope` && `cargo test -p sokonanoda-cli --test extension tm_grammar` |
| Infoview webview（Node） | 骨架/`status`/`⊢ `/`tok-*`/声明行号/无跳转/服务行：纯 Node DOM shim 行为测试 | `editor/vscode/test-webview.js`（`npm run test:unit`） | `node editor/vscode/test-webview.js` |
| 扩展宿主接线（Node，stub host） | 诊断事件只理 `.sokonanoda`（别的扩展不触发分析）/ 连发事件合并成一次刷新 / 并发 `soko/goals` 只发一次 / **切文件时丢弃过期答案**（树上不能出现"名字是 A、点击跳 B"的行）/ Infoview 相同 `decls` 不发第二遍 / 光标移动只问 `stateAt` / 课程树缓存一次 CLI 运行（显式刷新仍重跑）/ **项目树三例**：渲染闭包（根 + 拓扑序 + 入口/依赖标签 + 点击开模块 + 状态栏 tooltip 带项目行）、单文件一条占位行、答的是别的文档 ⇒ 丢弃 | `editor/vscode/test-extension-host.js`（`npm run test:unit` 的第 4 个文件；`crates/cli/tests/extension.rs::unit_test_script_covers_every_node_layer` 守住它在册） | `node editor/vscode/test-extension-host.js` |
| 项目层性能（三层） | front 分阶段（plan/digest/compile/keystroke/覆盖）与线性缩放、CLI 冷/热/依赖改动必 miss、LSP 项目 didOpen/按键（**每次按键 1 份诊断**）/改依赖刷新下游/请求延迟；每例打印 `PERF`（人读）+ `PERFJSON`（台账） | `crates/front/tests/perf_project.rs`、`crates/cli/tests/perf_project.rs`、`crates/lsp/src/tests/perf.rs`（`perf_project_*`） | `scripts/perf-ledger.sh`（→ `docs/perf/ledger.jsonl`）；口径见 `docs/PERF.md` |
| 编译结果缓存 | key 稳定/内容敏感；store→load 往返；`format` 不匹配 miss；缓存命中与重编诊断一致（`report_diagnostics` 共用） | `crates/lsp/src/cache.rs` :: `key_is_stable_and_content_sensitive`、`store_then_load_round_trips_and_is_key_scoped`、`format_mismatch_is_a_miss` | `cargo test -p sokonanoda-lsp cache` |
| by 分隔符 | `;` 或**换行**分隔 tactic（可混用）；换行边界 = 下一 token 在更晚行且是 tactic 关键字 → 当前表达式结束（`exact f` 换行 `apply g` 不合并）；多行项续行照常拼接；`;` 兼容 | `crates/front/src/parser.rs` :: `by_block_newlines_separate_tactics`、`by_block_newline_boundary_beats_application`、`by_block_multiline_application_is_one_tactic`、`by_block_semicolons_still_work_and_mix_with_newlines`、`by_block_does_not_consume_the_next_command`；`crates/cli/tests/cli.rs` :: `cli_by_newline_separated_tactics_check_via_kernel` | `cargo test -p sokonanoda-front by_block` |
| binder 类型推断（I6） | 无期望类型的应用位置 `(fun x => …) arg`：从实参类型（`judge_infer`）推断 binder；柯里化多参；实参不足仍报 `elab-untyped-binder` | `crates/front/src/compile/tests.rs` :: `untyped_binder_is_inferred_from_the_application_argument`、`curried_untyped_binders_are_inferred_from_the_arguments`、`untyped_binder_still_errors_with_too_few_arguments`；`crates/cli/tests/cli.rs` :: `cli_untyped_lambda_binder_is_inferred_from_the_argument` | `cargo test -p sokonanoda-front untyped` && `cargo test -p sokonanoda-cli cli_untyped` |
| 值位 `match` | `match e with \| Ctor binder... => body` 的 parse（arm/binder/嵌套/`\|` 缺失/重复 ctor）与 elab（非递归枚举 swap、结构体单臂、`Prop` 结果、arms 乱序重排、branch 里 `sorry`、**递归归纳：递归字段后自动插入 IH `ih`/`ih2`…、branch 引用 IH**、**prelude `Nat`/`Bool`：受信任归纳块登记进 `InductiveTable`，分支用点号名（`Nat.zero`/`Nat.succ`、`Bool.true`/`Bool.false`），降低/归约走 `.rec`**、**依赖 motive：结果类型 `R` 含裸局部变量 `x`（如 `P n`）时 motive = `fun t => R[x:=t]`，分支期望 = `R[x:=<ctor 项>]`、IH 类型 = `R[x:=<field>]`**、**模式编译器：通配 `_`、嵌套构造子（同一 ctor 多条 arm、首个匹配者胜）、Nat 字面量（脱糖 `succ^k zero`）、Bool 守卫 `if`（假则落下一 arm）**、未覆盖/未知/重复/期望类型未知报错）；降低为显式 `<Ind>.rec.{level}`（level 由结果类型 Sort 推出）且与手写 recursor 判定一致；错误码 `elab-match-bad-arm`/`-not-inductive`/`-no-expected-type`/`-recursive-unsupported`（Phase 2 起不再触发）/`-non-exhaustive` | `crates/front/src/compile/tests.rs` :: `match_*`（含 `match_recursive_inductive_uses_the_induction_hypothesis`、`match_prelude_nat_*`、`match_prelude_bool_*`、`match_wildcard_*`、`match_nested_*`、`match_nat_literals_*`、`match_guard_*`、`prelude_bool_*`、`explicit_bool_block_yields_to_the_source_declaration`、`match_dependent_*`（`nat_induction` 声明 binder 形式））；`crates/cli/tests/cli.rs` :: `cli_match_*`（含 `cli_match_recursive_*`、`cli_match_dependent_motive_checks_via_kernel`） | `cargo test -p sokonanoda-front match_` && `cargo test -p sokonanoda-cli match_` |
| 带索引归纳 | `inductive Vec (A : Type) : Nat -> Type`：索引 = `ty` 在 params 之外的 Pi 望远镜；派生 recursor（motive 先绑索引、major 在索引之后，iota 自调用带索引实参）；`match` 从 scrutinee 书写类型取索引实参、字段改名后 elaborate；常量 motive 可归约 | `crates/front/src/compile/tests.rs` :: `indexed_vec_checks_and_derives_recursor`、`match_on_indexed_vec_computes_with_a_constant_motive`、`match_field_types_follow_the_user_binder_names`；`crates/cli/tests/cli.rs` :: `cli_indexed_vec_checks_and_reduces` | `cargo test -p sokonanoda-front indexed` |
| 参数化归纳（非带索引） | `inductive Option (A : Type)` 的 parse（params + ctor 字段不含 params + 隐式参数）与 elab：派生递归子 `Option.rec`（参数最外层、内核 `def_eq` 重建 + 元数据断言）、显式 `rec`/`iota`（错位/漏参数被内核拒绝）、`match` on `Option A`（参数实例从 scrutinee 的**书写源类型**代入字段类型，`some a` 的 `a : A`）、递归 + 参数（`List A`，递归字段自动 IH）、scrutinee 非参数化局部量 → `elab-match-parameterized-unsupported` | `crates/front/src/parser.rs` :: `inductive_block_parses_parameters`、`inductive_block_parses_implicit_parameter`；`crates/front/src/compile/tests.rs` :: `parameterized_*`、`match_on_parameterized_option_instantiates_field_type`、`match_parameterized_without_written_params_reports_code`；`crates/cli/tests/cli.rs` :: `cli_match_parameterized_*` | `cargo test -p sokonanoda-front parameterized` && `cargo test -p sokonanoda-cli match_parameterized` |
| **课程门禁（卷 I《集合论》· 教学内容层）** | 判据 **G1–G6** 与课程规模无关：G1 每目标 `grade` 退出码 0（G-10：只认退出码）；G2 目标存在（`course.json` 条目 / `lib/` / 每个画布的解答文件）；G3 解答 `exercise.open == 0` 且 `decl.checked > 0`；G4 解答覆盖画布每个具名 `exercise.open.name`；G5 `lib`+Demo 无 `sorry`；**G6 清单自洽**（v2：volume/chapter id 唯一且非空、每个 unit 恰好属于一个 chapter、`prereqs` 指向存在的 chapter id）。计数与**配额差额**只进报告/台账、**从不判红**；`--selftest` 是**判据通道**的变异自检（故意坏的单元必须被判负 + 从另一个 cwd 判带 `import lib.*` 的目标 + 二分点名坏声明 + G6 的三类坏清单判负/一份合法 v2 判绿），`--bisect` 按顶层声明边界二分、结论不依赖诊断 span（G-15） | `courses/set-theory/tools/check.py`（唯一真相，python3、零 cargo）；清单 v2 展平与 G6 的纯清单单测 `courses/set-theory/tools/test_manifest_v2.py` | `python3 courses/set-theory/tools/check.py --selftest && python3 courses/set-theory/tools/check.py && python3 courses/set-theory/tools/test_manifest_v2.py`（判负时 `--only "<标签>" --bisect`；设计 `docs/design/course-gate-in-ci.md`、`docs/design/course-manifest-v2.md`） |
| **课程清单 v2（`soko.course/2`，台账 G-07）** | 清单两种形状都读：v1 扁平数组（`course/course.json`）与 v2 对象（`volumes[].chapters[].units[]`）；v2 的 `course.unit` **只加** `volume`/`chapter`/`tags`、`course.summary` 只加 `volumes`/`chapters`——**v1 事件一个键都不多**（additive-only）；未知 `schema` 直接拒读不猜；VS Code 课程树 v2 按卷→章→单元分组、v1 平铺保留；站点卷 I 页面按卷/章分组渲染（计数仍实测自门禁） | `crates/cli/src/course/manifest.rs`（解析/展平单测）、`crates/cli/tests/course_manifest.rs`（e2e：v1 兼容 / v2 增量 / 未知 schema / 卷 I 清单形状 / 入门课仍是 v1）、`crates/cli/tests/extension.rs`（脚本契约）、`editor/vscode/test-extension-host.js`（分组渲染 / v1 平铺 / 兜底分组） | `cargo test -p sokonanoda-cli --test course_manifest --test extension` && `node editor/vscode/test-extension-host.js` |
| **课程多清单聚合（`course <path>… [--all]`）** | 一次报多份清单：>1 份时 `course.unit` 才多出 `manifest`、`course.summary` 多出 `manifests`（单清单一个键都不多）；`--all` 递归发现 `course.json`（字典序、跳过隐藏目录/`target`/`node_modules`、同一清单去重）；一批里有一份读不了 ⇒ 整体 exit 1 且**零事件**；`--all` 一个都没找到是错误（不是空报告成功）；`--all` 只对 `course` 合法 | `crates/cli/src/course/mod.rs`（`discover_sources`/`walk_manifests`）、`crates/cli/tests/course_manifest.rs` :: `several_manifests_aggregate_in_order_and_tag_each_unit`、`all_discovers_every_manifest_under_a_root`、`a_bad_manifest_in_the_batch_fails_the_whole_run`；契约 `docs/protocol.md`（Course map） | `cargo test -p sokonanoda-cli --test course_manifest` |
| **课程成本台账（`docs/courses/ledger.jsonl`）** | `--ledger [路径]` 追加一条 `soko.course-ledger/1`：日期/课程名/目标数/checked/open/判负/用时 ms/版本（+ `commit`/`solutions_open`/逐目标 `rows`）；**默认关闭**（CI 不往仓库里写文件）；字段表 `LEDGER_FIELDS` 由 `--selftest` 与纯清单单测双重守护；已提交的台账逐行判「合法 JSON + 字段齐全 + `date`/`version` 形状对」（`version` 只判 `x.y.z`——台账是历史，版本 bump 后旧条目带着旧版本号，拿它比当前钉会假红） | `courses/set-theory/tools/check.py`（`LEDGER_FIELDS`/`ledger_entry`/`selftest` ⑤）、`courses/set-theory/tools/test_manifest_v2.py` :: `case_cost_ledger_fields_and_committed_file` | `python3 courses/set-theory/tools/check.py --selftest && python3 courses/set-theory/tools/test_manifest_v2.py` |

## 1.1 课程门禁：语言层之上的教学内容守护层（2026-09-19）

语言层的 TDD 三件套守"编译器没坏"，`courses/set-theory/tools/check.py` 守"**课程内容没坏**"
（卷 I 的 12 个单元 + 解答 + 课程标准库 + `Demo` 自检入口，共 34 个目标）。它是这一层的
**唯一真相**：判据是 python3 脚本、零 cargo（硬规则 6：课程作者/agent 的路径不装工具链），
课程抽成独立仓时整包搬走；**不在 Rust 里重写一份**（判据双实现必然漂移，设计
`docs/design/course-gate-in-ci.md` §2.3）。

判据只有五条（G1 退出码 / G2 目标存在 / G3 解答 0 open 且 checked>0 / G4 解答覆盖画布
每个具名练习 / G5 lib+Demo 无 sorry），**全部与规模无关**：`checked == N`、`len(units) == 12`
这类 golden 一律不写——课程还在长，锁计数会让门禁从质量闸退化成记账本（计数只进
`--json` 报告、`$GITHUB_STEP_SUMMARY` 与 `--ledger` 台账）。

三条已知坑各有一个工程化对策，且都由 `--selftest` 自己守着：

1. **G-10（`query check` 假绿）**：判据只认 `grade` 的**退出码**；`--selftest` 把一份
   "合法前缀 + `exact bogus_name`"的坏单元走**同一个判卷函数**，它必须被判负——通道
   失效时门禁自己红（这也是将来敢不敢换 `query check` 的前提）；
2. **G-12（相对路径打空模块根）**：入口一律 `str(path.resolve())`；`--selftest` 从另一个
   cwd（临时目录）再判一个带 `import lib.*` 的目标，必须同样 exit 0；
3. **G-15（失败位置的可读性）**——**已修**（WO-010 / 0.59.0：`query check` 的 `failed[]`
   自带 1 基行列，复现件也重写了）：失败信息仍是三段式——权威段（标签 + 绝对路径 + 退出码）、
   参考段（诊断 `stage/code/message` 原样，span 自带行列、**可信**）、定位段
   `python3 …/check.py --only "<标签>" --bisect`（按顶层声明边界二分，输出最后一个全绿
   前缀与第一个判红的声明，解析失败/多条错误时最稳）。CI 注解仍只带 `file=`、不带 `line=`。

接线：`scripts/soko gate`（贡献者门禁 = cargo 门禁 + 课程门禁 **+ 缺口台账门禁**；探不到
python3 ⇒ **exit 3**，绝不静默跳过；课程那一跑把解析到的二进制经 `SOKONANODA_BIN` 透传，
36 个目标不重复解析启动器；**台账那一跑刻意不透传**——G-11/G-16 测的就是启动器的解析链，
覆盖会短路夹具，`gap.py` 自己也剔除该变量）与
`.github/workflows/ci.yml` 的 `test` job 里的两个 step（`timeout-minutes: 5` 的课程门禁用
**当轮 cargo 刚编出来的** `target/debug/sokonanoda`——版本天然一致、零下载；失败打 `::error
file=…::` 注解、`--report` 进 `course-gate-report` artifact、表格进 step summary；紧随其后
的 `Gap ledger is consistent (docs/gaps)` 跑 `gap.py selftest` + `check`）。
为什么不建独立 job：见设计 §2.1/§2.2。退出码：0 全绿 / 1 有判负 / 2 前置缺失
（二进制与仓库版本不一致等，**无法判定 ≠ 绿**）。

## 2. 两个 meta 测试的机制（新 agent 最容易踩）

- `every_error_kind_has_stable_code_and_hint`：测试内部有一段**不带通配分支**的 `matches!` 穷尽清单——给 `ErrorKind` 加新 variant 时，这段代码会**编译失败**，逼你把新 variant 加进清单，并补齐 code（前缀必须与 stage 一致：elab kind → `elab-*`，kernel kind → `kernel-*`）与非空中文 hint。这是把"忘补 code/hint"从运行时错误提前到编译错误。
- `protocol_doc_lists_every_error_code`：把每个 code 断言出现在 `docs/protocol.md` 里。（历史注：曾有 6 个 elab code 靠 `known_missing` 允许表放行，2026-09-12 文档补齐后允许表已删除——该机制已闭环。）

## 3. 失败时的排查顺序

测试红了先定位层，再定位归属：

1. `parser::` / `token::` 红了 → front 的词法/语法层，看 `DiagnosticKind` 与 span；
2. `compile::tests::` 里 `checks_*` 红了 → 要么 elab 语义回归，要么 kernel 误拒；先用 `#check`/`#reduce` 在 REPL 复现，区分 front 与 kernel 的责任；
3. `cli::` / `protocol.rs` 红了 → 输出格式或事件词汇变了，**先怀疑协议破坏**（`docs/protocol.md` 是契约）；
4. `examples.rs` 红了 → 语料与前端能力漂移，绝不许"改语料让它过"，要修前端或明确废弃该语法点；
5. `protocol_doc_lists_every_error_code` / `every_error_kind_*` 红了 → 错误码体系或文档契约变更，按 §2 处理。
6. `query::` / `--test query` 红了 → 先分**语义**还是**两个视图漂移**：
   - 只有 `query_state*_agree_with_the_lsp*` 红 ⇒ 真相层大概是对的、某个适配器没跟上；
     **但先跑 `cargo build --workspace` 排除旧构件**（该测试 spawn
     `target/<profile>/sokonanoda-lsp`，改了 front 只跑单 crate 会拿旧二进制对拍）；
   - `front/src/query/tests.rs` 红 ⇒ 语义本身变了：**先逐字对照
     `docs/protocol.md` §`soko/stateAt` 的原文**（半开区间、根状态、无 `by` 分支），
     别照直觉"修"测试；两处都有的话，先修语义再重跑一致性契约。
7. `k_target` / `is_k` 相关红 ⇒ **去读内核那几行**（`kernel/src/inductive.rs:1268-1276`
   的 `init_k_target`）并逐字镜像，不要写"看起来等价"的判据；为每个能让两个版本
   取不同值的输入补一条测试（H6-C 的教训，见 `docs/LESSONS.md`）。
8. 派生 recursor 的**宇宙参数个数**红（`rejected: assertion 'left == right' failed`，
   `left`/`right` 是 0/1）⇒ 去读 `kernel/src/inductive.rs:1164-1262` 的
   `large_elim_test` / `large_elim_test_aux` / `mk_elim_level` 并逐字镜像
   （front 侧 `large_elim_test_mirror` / `large_elim_test_aux_mirror`），
   设计见 `docs/design/prop-large-elim-mirror.md`。**别用源码近似**：字段类型
   是不是 Prop 值是语义问题（`P -> Q`、`forall (x : Nat), P`、具名 `def … : Prop`
   都是 Prop 值），要看内核给的排序，不是源码里有没有 `Prop` 字样（G-03 的教训）。

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

1. **（已闭环，留档）`docs/protocol.md` 的错误码清单**：曾有 6 个 elab code 靠 `known_missing` 允许表放行，2026-09-12 文档补齐后允许表已删除。现在 `protocol_doc_lists_every_error_code` 用 `all: [ErrorKind; N]` 穷尽数组 + 一个**不带通配分支的 `match kind {}`** 双重守护：前者逐个断言 code 出现在文档里，后者在新增 `ErrorKind` variant 时**编译失败**（`matches!` 会隐式落 `_ => false`，守护不了），逼你同步数组、code 与 `docs/protocol.md`。
2. **kernel 内部 2 个 ignored fixture 测试**：`crates/kernel/src/tests/util.rs` 的 `reject_rec_rule_with_forged_lambda_domains` 与 `reject_unlisted_recursor`——上游提交了测试但没提交 `test_resources/` 的 NDJSON fixture，重建 fixture 是独立任务。另有 `crates/kernel/tests/arena.rs` 需要设 `LEAN_KERNEL_ARENA` 才运行，未设时自动跳过。
3. **（已闭环，留档）编辑器端 LSP 部分手工**：codeLens / quick-fix 曾有自动化缺口（2026-09-14 补齐：`code_lens_ranges_match_each_declaration`、`code_action_refine_edit_targets_the_hole`、`code_action_open_goal_without_a_next_step_offers_none`，四族 quick-fix 与 codeLens 全走进程内 rpc；见 §1 的 LSP 行）。真正仍需手工/真实编辑器的只剩 VS Code Electron 集成（已由 `editor/vscode/src/test/extension.test.js` 覆盖激活→LSP→诊断/hover 运行时行为，见下节）。
4. **（已闭环，留档）** goal 视图曾「只有 proof.rs 3 个单测、无 e2e」——I9 起已有 `soko/goals` 等 5 个自定义请求的进程内测试与 `crates/cli/tests` 的 goal 视图路径。
5. **hover 文本的语义要读懂再用**：hover 显示的是"该子表达式的类型"。类型位置的 `Nat` 的 hover 是 `Type 0`（Nat 的类型），值位置的 `n + 1` 才是 `Nat`——`hover_map_covers_subexpressions` 固定了这两个真实值，编辑器呈现时别想当然。
6. **perf 只有 canary 不是基准**：30s 阈值只挡 debug 构建下的"意外爆炸"。外部
   基准（对照 Lean Kernel Arena）**已立项为 opt-in**：`scripts/perf-arena.sh` +
   `crates/kernel/tests/arena.rs`（`LEAN_KERNEL_ARENA` 门控，未设自跳过，CI 不依赖）；
   语料属外部仓库不 vendor，见 `docs/PERF.md`「External baseline」。
7b. **（已闭环，留档）项目入口里没有 quick-fix**：`front::suggest`（code action 的
   exact/refine/intro 建议）与子洞探针只拿**入口文本**判据，看不到被导入的名字——
   同一个文件放进单文件给得出 `refine And.intro a b sorry sorry`，放进项目入口是
   `null`（2026-09-18 真 LSP 探针复现）。根因有三层，现在都修了：
   ① `judge_*` 合成文件只有文档前缀 ⇒ 新增 `extra_prefix`（闭包上下文），
   `QueryDoc::judge_prefix(offset)` 是真相层入口；
   ② `suggest` / `probe_sub_goal_types` 没接这条前缀 ⇒ 新增 `suggest_with` /
   `probe_sub_goal_types_with`，LSP 的 code action 与 goal/inlay/hover 都传它；
   ③ **建议材料表**（`GoalTemplates`：refine/intro 的构造子索引）`run_pass` 里按
   *单个单元* 构建 ⇒ 项目模式下入口看不见 `And.intro`，refine 建议凭空消失；
   现在按"拓扑序前缀 + 本单元"的命令表构建。
   守护：`code_actions_work_in_a_project_entry`（LSP）、
   `project_documents_expose_a_judge_prefix_and_probe_sub_goals`（front）。
7. **（已闭环，留档）多文件 LSP 的跨文件失效 + 测试死锁**：I16 P5 第一版
   "改依赖 ⇒ 重编译下游" 曾在两条文档的测试里挂死，当时的结论是"实现会挂"，
   真相是**测试写法与服务端发布节奏撞车**：服务端在一次通知里可能连发多条
   `publishDiagnostics`（改动的文档 + 下游），而测试先 `await` 通知处理完再去读
   socket——服务端的 send 等测试读、测试又等通知结束 ⇒ 死锁（tower-lsp 的串行
   通知把这一点放大成"必然挂"）。两条修法都在 0.57.0 落地：
   ① 服务端**只在诊断真的变了时才发**（`Doc::published` 比对），一次通知通常
   只发一份；② 测试用 `testutil::notify_with_drain`（`tokio::select!` 边处理边
   排空，处理完再取走队列里剩下的），或用 `did_open_at_drained` /
   `did_change_at_drained`。**写多文档测试一律用排空版**——`did_open_at` +
   `wait_diagnostics_for` 只对"一次通知最多一条诊断"的单文档场景安全。
   这两项后来都做完了：`didChangeWatchedFiles` 在批次 1（0.57.0），`soko/project`
   在批次 4（0.58.0，见 §1「项目状态视图」行与 `docs/design/project-view.md`）。

## 6. 初始规模快照（历史，最新数字以 STATUS.md 各轮为准）

- `cargo test -p sokonanoda-front`：**74** 个单测全绿（token 10 / parser 10 / proof 3 / compile 51）；
- `cargo test -p sokonanoda-cli`：**30** 个 e2e 全绿（cli 21 / protocol 8 / examples 1）；
- `cargo test -p sokonanoda`（kernel）：41 过 + 2 ignored（盲区 2）+ memory_api 1 过。

## 2026-09-07 更新：新增层与总量

- **semantic tokens 层**：`crates/front/src/semantic.rs`（分类单测 10 个）+
  `crates/lsp`（capability/编码/端到端 4 个，UTF-16 增补平面用例）。
- **session/delta 层**：`crates/front/src/session.rs`（solved/failed/opened、
  零重编译、parse 错误恢复，2 个）。
- **check-then-add**：`kernel_failed_declaration_frees_its_name`（双趟语义）。
- **watch（L1 CLI 形态）**：初始手动三版本冒烟（file.changed → exercise.solved →
  零重编译）；现已有自动化集成套件 `crates/cli/tests/watch.rs`（握手
  `service.hello`、规范名 `file.didChange`、`--doc` 别名、`--workspace`
  每文件独立版本与 `file` 字段、闭词汇与 protocol.md 列名；client→service
  命令集：`ping`→`pong`、`subscribe` 过滤 workspace 文件、`unsubscribe`
  停止事件、坏命令回 `error` 且流不崩；等待用 `recv_timeout` 轮询，无固定
  sleep；同文件单测守护订阅白名单语义）。
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

## 2026-09-07 更新（第九轮：多洞/refine + 内核错误分类学 + 发布流水线）

- **多洞/refine**（front 4 + LSP 3）：spine 走查 Open 状态、子目标实例化
  （参数位=目标实参、证明位=字段类型）、refine 骨架（参数自动填充）、
  无模板回退、混合实参；LSP refine action、soko/goals holes/sub_goals、
  nextHole 跨子洞。
- **内核错误分类学**（front 5 + cli 3 e2e + kernel 消息增强）：8 个新
  kernel 错误码各有实测触发样例（见 subagent 报告/protocol.md）。
- **稳定性**：#check/#reduce panic 守卫（`cli_classifies_check_apply_to_non_function`）、
  .vscodeignore 回归守护（`runtime_dependency_is_packaged`）。
- 测试总量（2026-09-07 第九轮）：**245**。

## 2026-09-07 更新（第十轮：导航基线 + undo）

- **completions/folding**（lsp 2）：关键词/宇宙/prelude/声明全集（内部名
  不外泄）、多行声明折叠；`cli_prints_its_version`（cli）。
- **go-to-definition/documentHighlight**（lsp 3 + front 解析映射测试）：
  use→binder/use→声明（shadowing 用例）、同定义高亮、binder 补全。
- **REPL undo**（front 4 + cli e2e）：快照栈、失败不入栈、恢复全等。
- 测试总量（2026-09-07 第十轮）：**273**。

## 2026-09-07 更新（第十一轮：提示阶梯 + 建议 + rename/references + inlay）

- **提示阶梯**：front `compile/hints.rs`（指令解析/挂接 6 测试：独占行、
  挂下一条声明、跳过非声明命令、最后声明后丢弃）+ session 集成
  （`session_attaches_hints_and_refreshes_on_comment_edit`：hint 编辑零重编译
  但阶梯刷新）+ lsp `soko/hints`（阶梯返回/声明外为空）。
- **下一步建议**（front judge 5 + suggest 9 + lsp actions 4）：judge_hole_fill
  正反两面与 parse 失败不 panic；suggest 排序（exact→refine→intro）、
  rfl 仅 Eq goal 且 kernel 验证、多洞逐洞 exact 回归
  （`spine_holes_get_per_hole_exact_not_outer_goal_matches`）、
  is_preferred 恰一条、≤3 条截断。
- **rename/references**（front references 5 + lsp render 12）：references
  四向（use/含声明/def 点/无 target）、prepare 三态、rename 成功（版本化
  documentChanges、shadowing 内层、注释不误伤）、非法名/无 target
  ResponseError（测试内联原始调用助手断言 error）。
- **inlay**（lsp 5）：单洞/多洞子类型/checked 无/partial tooltip/无洞空。
- **VS Code 契约**（cli extension.rs）：`sokonanoda.revealHint` 声明/注册/
  workspaceState/禁「还剩 N 条」；`common/mod.rs` 词汇 + soko/hints（skill
  conformance 同步）。
- **lsp lib 化**：`cargo test -p sokonanoda-lsp` 语义不变；`sokonanoda lsp`
  子命令编译期覆盖（cli e2e 无 Electron 冒烟，stdio 交互不进 CI）。
- 测试总量（2026-09-07 第十一轮）：**327**（front 171 / lsp 56 / cli 55 /
  kernel 45）。

## 2026-09-07 更新（第十二轮：课程地图 + REPL 历史 + course 阶梯）

- **course 聚合**（`crates/cli/tests/course_status.rs`，4 个）：manifest 聚合
  逐单元 pinned（12/2/1/0/4 · 5/5/4/3/3，summary 19/20/0）、人类视图片段、
  坏单元带 error 且 exit 0、manifest 缺失 exit 非零；词汇表 +2
  （`course.unit`/`course.summary`，protocol.rs 与 skill.rs 共用同一封闭表）。
- **课程树契约**（extension.rs，+1）：package.json 声明视图/命令 ↔
  extension.js 注册、CLI 子进程调用 + `course.unit` 解析、**负断言**禁止
  客户端引用 soko/courseStatus（服务器单文档是方向性取舍）、子进程超时
  纪律（setTimeout + kill）。
- **REPL 历史**（cli e2e，3 个）：临时 HOME 注入（既有 repl 测试从此不污染
  真实家目录）、非空输入追加、HOME 缺失静默禁用、跨会话累积。
- **course 阶梯内容**：5 单元 × 20 练习 × 3 条 `-- soko:hint`（60 条）；
  golden 逐单元计数不变 + solutions 零诊断（注释不产事件）。
- 测试总量（2026-09-07 第十二轮）：**335**（front 171 / lsp 56 / cli 63 /
  kernel 45）。

## 2026-09-07 更新（第十三轮：内核分类学 + 建议 + 基准/fuzz）

- **内核冷路径分诊**（kernel memory_api 3 新 + front 分类 5 + cli e2e 2）：
  `accepts_iota_rules_in_constructor_order`（对照）/`rejects_iota_rules_out_of_constructor_order`/
  `rejects_iota_rule_count_short_of_constructors`；front
  `pipeline_classifies_iota_rules_out_of_order`/`pipeline_classifies_missing_iota_rule`
  （断言精确 code + hint）；`refine_kernel_kind` 家族样例 +9 组；
  CLI `cli_classifies_iota_rules_out_of_order`/`cli_classifies_missing_iota_rule`。
- **非递归归纳块**（front 2 + cli 2）：`non_recursive_inductive_block_compiles_and_reduces`
  （kernel 语义层）、`inductive_block_without_rec_is_a_clean_elab_error`、
  `cli_accepts_non_recursive_inductive_block`、`cli_classifies_missing_inductive_rec`。
- **失败声明 Restart 建议**（front suggest 6 + lsp actions 4）：骨架形状/
  重启闭环（落回后 kernel 重查回 Open）/binder 防撞/≤3 层/隐式风格/非 Pi 无建议；
  LSP 值位整体替换/多行/摘要截断。
- **criterion 基准**（不进 cargo test 计数）：`cargo bench -p
  sokonanoda-front --bench pipeline`（多 target 必须 `--bench pipeline`）；
  语料校验 OnceLock 先行。
- **fuzz**：`fuzz/` 独立 crate（脱离 workspace）；`cd fuzz && cargo check`
  守编译；CI 不跑。
- 测试总量（2026-09-07 第十三轮）：**356**。

## 2026-09-07 更新（第十四轮：hole_id + auto-derivation）

- **hole_id wire**（lsp 2 + extension 契约 1）：`goals_request_ids_holes_by_decl_and_order`
  （命名 `<name>:0` / 匿名 `example@<行>:0`）、sub_holes 测试补对象形状与
  区间对齐；`goals_holes_carry_ids`（客户端读 `hole.range`、负断言禁止裸
  Range 传参）；nextHole 测试不动（仍裸 Range）。
- **recursor 自动派生**（front 3 + cli 2 改写）：
  `auto_derived_recursor_non_recursive_compiles`（Unit→`1`）、
  `auto_derived_recursor_bool_computes`（`not tt⇒ff`/`not ff⇒tt`）、
  `auto_derived_recursor_recursive_nat_adds`（无 rec 的 Nat 块 + add 闭环）、
  `cli_accepts_bool_with_auto_derived_recursor`；显式 rec 回归锚点
  （py-nat/course5/memory_api iota）全部不动。
- **字段望远镜契约修正**：`num_fields`/`ctor_telescope_size_wo_params` 按
  完整 Pi 望远镜计（result 链字段计入）——内核 `check_declared_metadata`
  一致性要求；既有块数值不变（括号字段构造子的两种计法相同）。
- 测试总量（2026-09-07 第十四轮）：**360**。

## 2026-09-07 更新（第十五轮：spine meta B′ + 失败声明建议升级）

- **深度替换**（front 2）：`sub_goal_field_types_substitute_compound_binders`
  （复合字段 `And a b` → goal 实参实例化）、
  `sub_goal_field_types_respect_binder_shadowing`（innermost wins）；
  裸 Ident 路径旧断言零改动。
- **失败声明建议梯子**（front judge 4 + suggest 10 + lsp 4）：
  judge_value_replace 正反两面（rfl 接受/拒绝丢弃/宇宙携带/解析失败不
  panic）；rfl 验证项 → Reset（lambda 前缀保留）→ Restart 排序与
  is_preferred 恰一；保守形态识别（多 binder 单 fun 不识别）；两个既有
  重启锚点更新到新梯子，lib.rs 锚点不动。
- 测试总量（2026-09-07 第十五轮）：**380**。

## VS Code 集成测试（@vscode/test-electron，2026-09-08 新增；0.58.0 起例行化）

> **例行化入口**：`scripts/vscode-e2e.sh`（构建 release → stage 到 `bin/<target>/`
> → `npm test` → 记 `docs/e2e/ledger.jsonl` + 裁剪日志；默认钉 VS Code 1.138.0）。
> 手册 = **`docs/E2E.md`**；0.58.0 起 14 条用例，含 `.sokonanoda` 语言 id 与
> **项目树**三条（真 `soko/project` 答案渲染的行：闭包 / 单文件占位 / 缺模块根因）。
> **CI 也跑同一条命令**（独立 `e2e` job，3 条腿：ubuntu × VS Code 1.138.0 /
> **1.106.0（声明的最低版本，2026-09-18 本地 14/14 验过）**，macOS × 1.138.0 只在
> push 到 main 时跑；结果进 job summary 与 artifact，main 上再由 `e2e-ledger` job
> 用 `scripts/e2e-merge.py` **合并成一条提交推回仓库**；`auto-tag` 的 `needs` 含
> `e2e` ⇒ e2e 红了不发版）。

扩展在**真实 VS Code**（Electron）里跑测试，补上此前只有静态契约
（`crates/cli/tests/extension.rs`）与手测的缺口。框架为官方推荐组合：
`@vscode/test-cli`（`vscode-test` 命令）+ `@vscode/test-electron`，
配置在 `editor/vscode/.vscode-test.mjs`，测试是**纯 JS**（与扩展一致，
无 TS 构建步骤）。

**位置与运行**：

```bash
cargo build -p sokonanoda-lsp        # 前置：测试绝不自己构建服务器
cd editor/vscode && npm ci && npm test
```

- 测试文件：`editor/vscode/src/test/extension.test.js`；
- 夹具工作区：`editor/vscode/src/test/fixtures/workspace`（最小工作区，
  不放 `.sokonanoda`——测试文档运行时写进系统临时目录再打开）；
- CI：`ci.yml` 的 test job 末尾——先 `cargo build -p sokonanoda-lsp`，再
  node 22 + `npm ci` + `xvfb-run -a npm test`（Linux 上 Electron 需要显示器，
  xvfb 在 ubuntu-latest 镜像预装）。

**覆盖内容**（4 个用例，全部走真实 kernel，不复刻任何前端逻辑）：

- **前置（suiteSetup，不计用例）**：扩展激活——
  `getExtension('sokonanoda-lang.sokonanoda')` → `activate()` → `isActive`
  （隐含客户端 start 成功——服务器找不到时 activate 只弹警告不启动，
  会在这里暴露）。服务器二进制不存在时整组 **skip 不是 fail**：激活能过
  但服务器起不来，诊断/hover 只会等超时，那不是被测代码的回归；
1. **干净文件 0 诊断**：内联干净教学样例（**不含** `sorry`——见用例 3 的
   语义）→ 等 hover 非空作为「服务器已编译完本文档」的 ready 信号（服务器
   `refresh()` 先发诊断再返回，hover 在其后，见 `crates/lsp/src/lib.rs`）
   → 断言诊断为空；
2. **kernel 拒绝带码**：`def bad : Prop -> Type := fun (x : Prop) => x`
   → 断言诊断含 `kernel-rejected` code、`source == "sokonanoda"`、severity
   为 Error（与 front 的 Failed 用例同族，走的是同一 kernel 判定）；
3. **开放练习带 sorry warning**：`theorem t : True := sorry` → 断言诊断含
   code `sorry`、`source == "sokonanoda"`、severity 为 Warning 且全文件
   无 Error（Lean 4 对齐：文件编译通过但带缺口——缺口要可见，但不该
   算编译失败）；
4. **开放练习 hover 非空**：同一形态的光标落在 `sorry` 上断言 hover
   markup 非空（洞的目标/引导信息）。

**与静态契约测试的分工**：`extension.rs` 不需要 Electron，守护清单/入口
脚本/打包元数据（快、进 `cargo test` 门禁）；本节测试守护**运行时行为**
（激活→LSP 起进程→诊断/hover 端到端）。两层都在时，扩展的
manifest 变更与行为回归分别有人管。

**版本注意**：

- `@types/vscode` 必须钉在 `1.85.0`（engines.vscode 的**最小**版本，vsce
  打包要求 types 不高于 engines）；`@vscode/test-electron` 用 3.x——2.5.2
  会去找 `MacOS/Electron`，而 VS Code ≥1.136 的二进制已改名 `MacOS/Code`，
  spawn 直接 ENOENT；3.x 要求 node ≥22（CI 的 setup-node 钉 22）；
- 首次运行会下载 ~300MB VS Code 到 `editor/vscode/.vscode-test/`（已进
  .gitignore）；网络不通时会在此失败。

**已知风险**：测试跑的是 VS Code stable（会随上游漂移，xfail 策略是红
了先看 VS Code 更新日志）；每次 CI 运行都重新下载 VS Code（未加缓存，
VS Code 下载地址按版本变化，缓存收益低）；Linux CI 首跑验证仍待实际
workflow 触发确认；用例 3/4 依赖 LSP 的 sorry-warning 行为（`sorry` 发
WARNING 而非静默），与 `crates/lsp/src/lib.rs` 的对应改动是同一契约的
两端，需同轮落地否则测试会真实变红（这是期望的守护行为，不是误报）。

## 2026-09-10 更新（第二十轮：soko/stateAt 光标处 goal 视图）

- **front `by_steps` 层**（compile/tests.rs + session.rs，3 个）：
  `partial_by_block_records_per_step_states`（逐步 goal/上下文/源码 span）、
  `checked_by_block_records_closed_final_step`（尾步 `goal=None`）、
  `session_remaps_by_step_spans_on_comment_edit`（零重编译后 span 平移）。
- **LSP 协议层**（lib.rs，5 个）：`state_at_inside_a_tactic_shows_the_entering_state`
  （Lean `goalsAt?`：光标在 tactic 上显示执行前状态）、
  `state_at_after_the_last_tactic_shows_the_remaining_goal`、
  `state_at_on_the_by_keyword_returns_the_root_goal`、
  `state_at_without_by_steps_returns_the_declaration_goal`、
  `state_at_outside_any_declaration_is_empty`。
- **词汇表**：`common/mod.rs::LSP_CUSTOM_METHODS` +`soko/stateAt`（skill
  conformance 与 protocol.md 三向一致）。
- **VS Code 契约**（extension.rs）：`entry_script_speaks_the_goal_view_protocol`
  增两断言（消费 `soko/stateAt`、挂 `onDidChangeTextEditorSelection`）。
- 测试总量（第二十轮）：**437 + 8 ignored**（front 229 / lsp 89 / cli 71 /
  kernel 48）；playground 锚点 20/9/0、course 32/25/0 不变。

## 2026-09-10 更新（第二十一轮：插件自带 LSP——bundled VSIX）

- **纯 Node 单测**（`editor/vscode/test-server.js`，15 个）：平台→target 映射、
  bundled 路径/exec 位修复（缺位 chmod、只读降级）、解析顺序（setting/env/
  bundled/workspace/缓存）、**下载 URL 锁定 `v${version}` 且不含 `/latest/`**、
  不支持平台报错；`test-download.js` 改为 require `server.js` 真实现（去重）。
- **打包冒烟**（ci.yml `Package host VSIX`）：每次 CI 对 host 打平台包并断言
  `bin/linux-x64/sokonanoda-lsp` 大小 >1MB、exec 位、`TargetPlatform`。
- **静态契约**（extension.rs，+5）：bundled 解析/latest 禁令/版本锁定 URL、
  scripts 齐全 + `.vscodeignore` 不排 bin、`Cargo.toml` ↔ `package.json`
  版本一致、release.yml per-target 打包 + universal、ci.yml test:unit + stage。
- **集成测试**：CI 先 `stage-lsp.js --profile debug` 把服务器放进
  `bin/linux-x64/`，集成测试因此走的正是用户安装平台包后的 bundled 路径。
- **本机验收**：`npm run package:host` → 1.96MB VSIX（zip mode 755、
  `TargetPlatform="darwin-arm64"`）；`package:universal` → 0.47MB 无 bin。
- 总量：workspace **440 passed + 8 ignored**；node 单测 22。

## 2026-09-10 更新（第二十二轮：平台矩阵扩展 4 → 8）

- **node 单测**（test-server.js，16 个）：新增 linux-arm64 / win32-arm64 /
  alpine-x64 / alpine-arm64 的 `platformTarget`/`rustTarget`/`bundledServerPath`
  映射、musl 下载 URL、`isAlpineLinux`（`/etc/alpine-release`，仅 linux）。
- **静态契约**（extension.rs，12 个）：server.js 必须含 8 个 target 与 Alpine
  检测；release.yml 必须含 4 个新 rust target + `cargo zigbuild` + `.2.28`。
- **release dry-run**：8 平台构建（Linux 为 Zig 交叉：gnu 目标 readelf 断言
  GLIBC ≤ 2.28、musl 断言静态；win32-arm64 原生）→ 9 个 VSIX zipfile 冒烟
  （8 平台 + universal）→ GitHub Release 17 资产（8 tarball + 9 VSIX）。
- **版本** 0.7.0 → 0.8.0；扩展 README 平台列表、CHANGELOG、RELEASE.md、
  vscode-dev-guide（zigbuild/glibc 坑）、ci skill 同步。

## 2026-09-10 更新（第二十二轮续：opencode launcher）

- **opencode 配置契约**（`crates/cli/tests/opencode.rs`，4 个）：
  1）`opencode.json` 必须启动仓库 launcher（`bash -c exec …` 单行），禁止
  `cargo run`；2）无 cargo 的 PATH 下命中 **VS Code 扩展自带 bin**；
  3）无 VS Code、无 cargo 时用 fake `curl` 断言 **按仓库版本锁定**的
  Release 下载（URL 含 `/download/v9.9.9/`、不含 `/latest/`）并运行解出的
  二进制；4）离线（`SOKONANODA_LSP_OFFLINE=1`）且一无所有时给可行动错误。
  launcher 解析顺序：`SOKONANODA_LSP_BIN` → 仓库 target → VS Code 扩展
  bin → 下载缓存 → 版本锁定下载 → `cargo build`。

## 2026-09-10 更新（第二十二轮续：CLI 零工具链化）

- **CLI 解析**（test-server.js，+2）：`resolveCliCommand` 的 bundled →
  workspace → PATH 顺序、Windows `.exe` 名称。
- **静态契约**（extension.rs）：server.js 必须含 `resolveCliCommand` 与
  `sokonanoda.exe`；release.yml 必须含 `--cli-binary` 与
  `sokonanoda-cli-` 资产名。
- **打包冒烟**：CI host VSIX 与 release 的 8 平台包均断言 **LSP + CLI 两个
  二进制**入包、大小 >1MB、linux/darwin exec 位；Release 资产 = 8 LSP
  tarball + 8 CLI tarball + 9 VSIX = 25。

## 2026-09-10 更新（第二十三轮：环境配置单一入口）

- **opencode/onboarding 契约**（`crates/cli/tests/opencode.rs`，8 个；含插件直连原生二进制、不许 `"bash"`、opencode.json 不写 lsp 的契约）：
  `sokonanoda doctor --json` 退出码契约（空缓存 3 → 伪造 marker 后
  0）、`setup` 离线可行动（exit 3 + 提示）、`grade` 直 exec 缓存 CLI、
  launcher 命中 VS Code 扩展自带 bin（无 cargo）、fake-curl 版本锁定下载
  （URL 含 `/download/v9.9.9/`、无 `/latest/`）、一无所有 exit 3、
  命名空间命令/插件/shim 契约（命令禁源码构建命令）。
- **发布资产属性断言**：release job tar 前 chmod + `tar tzvf | grep '^-rwx'`
  （v0.8/v0.9 曾发布 0644 二进制；见 CI-FAILURES 2026-09-10）。
- **本机冒烟**：`sokonanoda setup && sokonanoda doctor`（READY）+
  `grade playground.sokonanoda`（事件流）+ opencode LSP 诊断。

## 2026-09-11 更新（第二十八轮：内核已定义名字的声明 warning）

- **触发**：画布 `axiom Prop : Sort 1`——内核接受但名字永不被引用；用户
  要在 VS Code 里给出 warning。设计见 `docs/design/reserved-decl-warning.md`。
- **front**（`compile::warning`）：`collect_warnings` 纯语法扫描顶层声明名，
  命中 `Prop`/`Sort`/`Type` 产出 `reserved-declaration-name`（span 用
  `decl_name_span` 收窄到名字 token）；`CompileOutput.warnings` 与
  `DocumentReport.warnings` 双通道；会话零重编译路径现算。
  测试：`reserved_declaration_name_produces_a_warning`、
  `ordinary_declaration_names_have_no_warning`、
  `session_keeps_warnings_on_zero_recompile`（front 243，+3）。
- **CLI**：新事件 `warning`（`{type, human, code, message, hint, span}`），
  人类视图 stderr `line:col: warning[code]: message`；不改退出码。
  测试：`reserved_declaration_name_warns_but_stays_successful`（CLI protocol 9，+1）。
- **LSP**：`update.report.warnings` → `DiagnosticSeverity::WARNING`。
  测试：`reserved_declaration_name_is_a_warning_not_an_error`（LSP 93，+1）。
- **锚点**：`playground.sokonanoda` 现为 checked=14 / open=5 / warning=1 /
  0 诊断（第 79 行的 `axiom Prop : Sort 1`）。
- **`Type n` 记法**（同轮，用户要求）：`Type n` = `Sort (n + 1)` 解析糖。
  测试：parser `type_with_level_parses_as_sort_succ`、front
  `type_with_level_is_lean_sort_succ`、CLI `cli_accepts_type_with_level_as_sort_succ`；
  课程单元④ zh/en 各加一条 `#check (Type 0)`（事件计数仍相等）。

## 2026-09-11 更新（第二十九轮：环境能力进二进制，删除 `soko.sh`）

- **触发**：用户要求环境能力做成二进制 CLI，拒绝 `scripts/soko.sh`（Windows
  不可用、维护面大）。设计见 `docs/design/binary-cli.md`。
- **CLI 子命令**（`crates/cli/src/env/`）：`version`/`doctor`/`setup`/
  `update`/`grade`/`gate`；`setup`/`update` 用**内嵌下载器**
  （`ureq`(rustls/ring) + `flate2` + `tar`，无 shell/外部工具）按
  `build.rs` 钉死的 `TARGET` 拉取 `<pkg>-<triple>.tar.gz`；
  `SOKONANODA_RELEASE_BASE` 供测试/自托管覆盖；标记与 VSIX/插件一致
  （`<version> <vsce-target>`）。
- **去脚本**：删除 `scripts/soko.sh`；`.opencode/command/sokonanoda/*` 改调
  `sokonanoda <sub>`；插件 `findRepoRoot` 改用官方目录
  `.opencode/plugins/sokonanoda.ts` 作仓库标记；LSP shim 改为解析二进制并
  exec `sokonanoda lsp`。
- **测试**（`crates/cli/tests/opencode.rs`，8 个，重写）：`version` 三态标记、
  `doctor` 退出码、`setup` 离线可行动、`update` 用本地 HTTP 服务器验证
  版本锁定下载（无 `/latest/`）、shim 解析/失败可行动、插件/命令契约。
- **发布注意**：二进制新增 TLS（rustls/ring）+ tar/gzip 依赖，需 release
  `workflow_dispatch` 干跑验证 8 平台交叉构建。

## 2026-09-17 更新（第八十九轮：内核真相查询通道 H6-A/H6-B/H6-C，0.56.0）

- **`redundant-sorry`（第九十一轮续）**：分层见 §1 新行。设计与那 5 分钟实验
  （真根因 = `EnvLimit::ByName(探针名)` → `NO_DECL` → cutoff 0 → 空环境，不是
  `NamePtr` 身份）见 `docs/design/redundant-sorry.md` §8。
- **新增真相层 + 两条通道 + 一致性契约**：`front::query`（唯一语义源）→
  `sokonanoda query <op>`（单 JSON 对象）→ `dsh/mcp/server.js`（七工具）。
  分层守护见 §1 的三行新条目（内核真相查询 / `query` 子命令 + 两视图一致性 /
  MCP 桥），排查入口见 §3 第 6–7 条。
- **两条一致性契约**（防两套真相，本轮的核心资产）：
  `crates/cli/tests/query.rs::query_check_counts_match_the_json_event_stream`
  （`query check` 计数 ≡ `--json` 事件计数）与
  `query_state_agrees_with_the_lsp_state_at_request` /
  `query_state_and_lsp_agree_without_a_by_block`（**spawn 真实
  `target/<profile>/sokonanoda-lsp`** 逐字段对拍 `soko/stateAt`，含无 `by` 两分支）。
  跑法：`cargo build --workspace && cargo test -p sokonanoda-cli --test query`。
- **H6-C 的三层**：front 单测（箭头字段派生 / 具名孪生 / 真 iota 归约 / 单构造子
  `Prop` / K 目标两个判别性反例）+ CLI e2e（3 条）+ 课程语料 9 个文件简化后
  `examples`/`course`/`course_status` 全绿（golden 计数不变）。
- **LSP 结构债清零：`lib.rs` 4256 → 3988（删重复）→ 1105 行**（0.56.0：两个测试模块
  移出文件 + 抽出 `protocol.rs` 159 行 wire 类型、`tokens.rs` 107 行 semantic token
  辅助；0.56.1：`tests.rs` 2567 行拆成 `tests/` 一目录，最大文件 399 行）。
  每一步都零断言改动（220 条 assert 记账相等、95/95 测试名一致、规范化行流只差
  plumbing），117/117 全程保持全绿。测试文件位置见 §1 的 LSP 行。
- **总量（2026-09-17，两条并行线各自的快照）**：合并前本线 **756**、0.56.2 线
  **772**（多了 `redundant-sorry` 的 8 条、摘掉 2 条 `#[ignore]`）；合并后的真数以
  本文件第九十八轮与 `STATUS.md` 为准。两边共同的底数：kernel 43 + arena 1 +
  memory_api = 51；front lib 406 + perf 3 = 409；cli 单元 5 + 集成 190；lsp lib 117；
  doc-tests 6 ignored（内核既有）。

## 2026-09-18 更新（第九十五轮：批次 3 拆分 `run_pass` + 「二进制对拍」验收手段）

- **拆分落点**（只动位置不动语义，三刀三次提交）：`front/src/compile/check.rs`
  → 目录模块 `check/{mod,walk,kernel_phase}.rs`，加上先前的 `compile/units.rs`。
  现状 `check/mod.rs` 791 + `walk.rs` 951 + `kernel_phase.rs` 413（原 1918 行单文件、
  ≈1174 行单函数）。`run_pass` 现在只剩闭包装配 + 前缀合成 + 两段调用。
- **二进制对拍（位置搬移类改动的首选验收，比 golden 覆盖面大）**：
  ```bash
  git worktree add --detach /tmp/soko-base HEAD      # 改动前的树
  (cd /tmp/soko-base && cargo build -p sokonanoda-cli --bin sokonanoda --locked)
  cargo build -p sokonanoda-cli --bin sokonanoda --locked
  # 对全部语料 + 项目/模式开关 + stdin + query 逐字节比对 stdout
  ```
  本次输入 = `git ls-files '*.sokonanoda'`（58 个）+ `--root course/shared` +
  `--no-project` + `playground` 走 stdin + `query check|goals|holes`；`fail=0`。
  三个注意点：① `cargo fmt` 会重排搬过去的代码，**文本级**对拍先归一化空白；
  ② **两个 worktree 不要共用 `CARGO_TARGET_DIR`**（cargo 按包路径分键做 fresh 判定
  但输出同名，后建的树会静默覆盖前者的二进制 → `touch` 源文件强制重建再比）；
  ③ 顺手删死代码（如只写状态 `built_inductives`、推空 `CmdHover` 的空操作）同样
  过一遍对拍——这类改动**没有**测试会红。
- **总量（2026-09-18，`cargo test --workspace --locked`，全绿）**：**862** 个测试 ——
  kernel 51（lib 43 + arena 1 + memory_api 7）；front 462（lib 453 + perf 3 +
  perf_project 6）；cli 214（单元 5 + 集成 16 个目标 209：cli 80 / extension 33 /
  imports 13 / query 12 / project_features 11 / watch 10 / protocol 9 / dsh 8 /
  opencode 8 / course 6 / course_shared 4 / course_status 4 / skill 4 /
  single_file_vs_project 4 / perf_project 2 / examples 1）；lsp 135（lib）。
  测试目标 26 个（+ doc-tests 3 个目标，6 ignored 属内核既有）。

## 2026-09-18 更新（第九十六轮：项目状态视图 —— `query project` / `soko/project` / 项目树）

- **新增一层只读视图**（批次 4）：`front::query::project_view()` 从**已编译的**
  闭包报告派生 `ProjectView`；`project::ModuleReport::status`
  （`compiled`/`load-failed`/`blocked`）让"根因"与"受害者"可区分。三个传输同一
  份真相：CLI `query project`、MCP `project`、LSP `soko/project`（回显 uri/version）。
  §1 新增「项目状态视图」一行；扩展宿主层补项目树三例（§1 的宿主行）。
- **测试构成（2026-09-18，`cargo test --workspace --locked`，全绿）**：**871** 个测试
  —— kernel 51（lib 43 + arena 1 + memory_api 7）；front 466（lib 457 + perf 3 +
  perf_project 6）；cli 217（单元 5 + 集成 16 个目标 212：cli 80 / extension 33 /
  project_features 14 / imports 13 / query 12 / watch 10 / protocol 9 / dsh 8 /
  opencode 8 / course 6 / course_shared 4 / course_status 4 / skill 4 /
  single_file_vs_project 4 / perf_project 2 / examples 1）；lsp 137（lib）。
  另有四个纯 Node 套件：server 18 / download 7 / webview 10 / extension-host 11。
- **版本 0.58.0**（Rust 与扩展同步 bump；`cargo_and_extension_versions_match` 守着）。

## 2026-09-18 更新（第九十八轮：合并 0.56.2 线 + 跨模块 warning 归因修复）

- **合并**：`origin/main`（0.56.2 = `redundant-sorry`）与本线（I16 项目管理 +
  批次 1–4，0.58.0）在 scratch worktree 里合并，19 个文件冲突手心合并；0.56.2
  写在旧 `check.rs` 的探针代码手工搬进本线的 `check/{walk,kernel_phase}.rs`。
  §1 的「多余的 `sorry`」一行仍然成立（正 2 + 反 3、会话快照、两视图一致）。
- **合并暴露并修掉一个真 bug**：内核终审的 warning 在**项目模式**下会丢归因
  （`split_report` 原先只按单元重算语法级 warning）。修法 = `CompileOutput` 增
  平行数组 `warning_cmds` + `push_warning(cmd, w)`，`split_report` 按命令下标归因；
  §1 新增一行守护（`warnings_are_attributed_to_the_unit_that_produced_them`）。
- **测试构成（2026-09-18 合并树，`cargo test --workspace --locked`，全绿）**：
  **888** 个测试 / 0 failed / 6 ignored（内核既有）—— kernel 52（lib 43 + arena 1 +
  memory_api 8）；front 475（lib 466 + perf 3 + perf_project 6）；cli 223（单元 5 +
  集成 17 个目标 218：cli 81 / extension 33 / project_features 14 / imports 13 /
  query 13 / protocol 10 / watch 10 / dsh 8 / opencode 8 / course 6 / course_shared 4 /
  course_status 4 / skill 4 / single_file_vs_project 4 / launcher 3 / perf_project 2 /
  examples 1）；lsp 138（lib）。测试目标 27 个 + 3 个 doc-test 目标。
  另有四个纯 Node 套件：server 18 / download 7 / webview 10 / extension-host 11；
  真 VS Code 例行化（`scripts/vscode-e2e.sh`）在 1.138.0 与 1.106.0 上各 14/14
  （台账 `docs/e2e/ledger.jsonl`）。
- **版本 0.58.0**：合并后 push `main` → auto-tag `v0.58.0` → release（0.56.2 的
  `v0.56.2` 与其功能都在历史里；CHANGELOG 的 0.58.0 条目补记 `redundant-sorry`）。
