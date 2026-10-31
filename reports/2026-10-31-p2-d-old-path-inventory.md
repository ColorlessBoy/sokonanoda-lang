# P2-D：删旧路径前的盘点（2026-10-31，只读）

分支 `strict/2026-10-29-kernel-app` @ `ff23149a`。F = `crates/front/src`。标「读」= 看过代码，「推断」= 未验证。
主会话复核：四个开关与 `type_oracle` 在 crates/scripts/.github/docs/skills/editor 0 处；`compile/strict/` 与 `scripts/audit-strict-paths.py` 不存在；主回落点 `elab.rs:4368` 属实。

## 结论
- **P3 原计划要删的东西本分支上不存在**：`SOKO_STRICT_TYPED_APP` / `SOKO_STRICT_V2` / `SOKO_STRICT_ALLOW_FALLBACK` / `SOKO_V2_LOCAL_CONVERT`、`type_oracle`、`compile/strict/**` 只在旧分支上有。P3 的「删开关」是空操作。
- **「旧路径」大部分是 loose 也在用的共享代码** ⇒ P2-13 = 砍掉共享代码里的 **strict 分支**，不是删模块。
- `SOKO_STRICT_AT` 是有文档的逃生门（`dialect.rs:119`，CLI/LSP 测试、`docs/protocol.md:289`、两个技能、VS Code e2e 都引用）⇒ **必须留**。

## strict 回落点（读）
| 位置 | 说明 |
|---|---|
| `elab.rs:4368` | `try_implicit_application_with` 调 `kernel_app::try_app`，`None` ⇒ 进旧 typed app。唯一调用方 `elab.rs:5715` |
| `kernel_app.rs:217-223` | `Err(Some)` ⇒ `probe::fallback` ⇒ 旧路径；`Err(None)` ⇒ 旧路径且不计数 |
| `kernel_app.rs:240-275` | 不计数的放弃：闸门关、头不是 Ident（`Eq.{u}`）、局部遮蔽、`implicit_prefix == 0` |
| `kernel_app.rs:315-345` `finish` | 各种非 Solved ⇒ 撤 hover、回落 |
| `elab.rs:1665` / `kernel_app_notation.rs:110` | 记法路由失败 ⇒ 旧记法路径 |
| `elab.rs:1786` | 零元记法（`∅`）`try_hinted_const` 失败 ⇒ 旧诊断 |
| `elab.rs:4360-4366` | `@` 脊、prelude 安装：设计上不进内核路径 |
| `kernel_app_lambda.rs:109` `fill_late` | 域仍带元变量的 λ 按旧办法 elaborate |

## 旧路径的组成（读）
| 部分 | 位置 / 规模 | 共享？ |
|---|---|---|
| `try_implicit_application_with` 旧 typed app | `elab.rs:4337–~5000`（约 660 行） | loose 全用；strict 专属只有 `:4505-4540`、`:4780-4794`、`:4885-4900` |
| `pp_shape.rs` | 316 行 | 共享（`named_arg.rs`、`meta.rs`、`check/meta_exit/p4b.rs`） |
| `operand_type_expr(_explicit)`、`infer_type_text*` | `elab.rs:4050/4114/3516-3870` | 共享（tactic、`#check`、具名实参） |
| `judge::judge_infer*` | `judge.rs`（4748 行） | 共享（tactic、goal 视图、LSP actions/completion） |
| `implicit.rs` / `implicit_msg.rs` | 1218 / 364 行 | 共享；`strict_rejection_message` 等报错文字只给 strict |

## 不止应用：仍走文本往返的 strict 路径（§9.39「零文本」范围，读）
记法层级 `universe_level_text_of_operands`（`elab.rs:2258`）· 记法前导参数 `elab.rs:1680-1701` · 非常量头类型 `elab.rs:2238` · 期望类型层级 `elab.rs:7646/7840` · match/let `elab.rs:6282/6999/7445/7509` · 签名→telescope `implicit.rs:68` · 具名实参 `named_arg.rs:144/259` · tactic `by.rs`、`apply_probe.rs` · goal 视图 `goals.rs:519/564`。

## P2-12/P2-13 风险前五
1. **`=`/`≠` 从不进内核路径**（`Eq.{u}` 头，7,802 次 / 13 文件）⇒ 先去回落会让所有等式失败。→ P2-11e。
2. **strict 报错文字来自旧路径**（`elab.rs:4783/4891`、`named_arg.rs:283`），内核路径只有探测用的无名字符串 ⇒ P2-12 要给点名 binder 的报错；断言 strict 文字的 CLI 测试会变（`cli/tests/implicit_at_alignment.rs:201`、`protocol.rs:391`、`implicit_named_args.rs:147`）。
3. **`@` 脊、具名实参、prelude 安装仍走旧路径**，具名实参依赖 `operand_type_expr_explicit` + `pp_shape::arg_types` ⇒ 删 strict 文本路径前要先移植。
4. **共享辅助函数里按 strict 分支的文字**（`pp_shape.rs:123`、`goals.rs:492`、`suggest.rs:633`、`lsp/src/actions.rs:1212`、`by.rs:344/2866`）不属于应用路径，删了会让 goal、建议、tactic 回退。
5. **hover 与缓存键**：回落时 `finish` 撤 hover、旧路径重记；不回落后报错位置可能没有实参 hover（LSP `render.rs:208`，推断）。去掉 `env_on` 会改 strict 的 `key_tag/key_bit` ⇒ 有意让缓存失效，`tests/kernel_app_keys.rs` 要重写。

## 审计脚本
`scripts/audit-strict-paths.py` 在本分支没有；旧分支 `f151eaba` 上有（152 行，对 `operand_type_expr_explicit(`、`infer_type_text(`、`pp=explicit` 做只降不升的计数，基线 2/6/1）。它的基线注释引用的 `type_oracle`、`level.rs` 本分支没有 ⇒ 摘过来后要按本分支重定基线。
