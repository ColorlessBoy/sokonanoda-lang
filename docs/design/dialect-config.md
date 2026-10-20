# 方言（严格隐式参数）做成**可配置参数**：配置面 + 分段实现路径

> 用户 2026-10-20 第三条拍板：「**严格隐式参数**我想还是变成**可配置的参数**，
> project toml、vscode 插件等等地方可以配置」。本文是这条的唯一执行依据；
> 性能那半边在 `docs/design/strict-mode-v2.md`（**两件事互不阻塞**）。

## §0 结论

**一个值、五处来源、一条优先级链**；实现分 **6 段**，每段一条判据、可单独发版。
最要紧的一条不是"能不能配"，是**配置化时别把缓存键搞坏**（见 §3.3）。

## §1 现状（为什么现在直接暴露成配置会踩坑）

1. `implicit_msg::strict_at_enabled()` **每次调用都读一次 `env::var`**，而它在热路径上
   （`elab::operand_type_expr`、`at_prefix`、`refl_text`）。做成配置后**不许**再读环境，
   必须**一次解析、进 `CompileOptions`**，否则每处都要改调用链。
2. `CompileOptions` 目前只有 `prelude: PreludeMode`（`crates/front/src/compile/prelude.rs:35`）
   ⇒ 方言**没有**位置可放，也进不了 `judge::options_key()`（`judge.rs:203`，现在只折 `prelude`）。
3. ⚠ ⚠ **缓存键的接缝（本文最重要的一条）**：现在 `SOKO_STRICT_AT` 之所以不会串味，
   是因为 `compile/cache/keys.rs::flags_state()` 把**所有** `SOKO_*` 环境变量排序后折进
   编译缓存键。**一旦方言改成 `sokonanoda.toml` / 编辑器设置（不经过环境变量），它就从
   缓存键里掉出来** ⇒ 两档共用 `<模块根>/.sokonanoda/` 会**互相命中**，判定与冷跑不一致。
   `keys.rs` 的注释里已经为 `SOKO_ARG_EXPECTED`/`SOKO_JUDGE_INPLACE` 记过**同族事故**。
   ⇒ 配置化的第一步必须**同时**把方言塞进键里（§4 S2）。

## §2 配置面与优先级（细粒度优先）

| # | 来源 | 形态 | 优先级 |
|---|---|---|---|
| 1 | **CLI 旗标** | `sokonanoda grade --dialect strict <file>`（`build`/`query`/`course` 同认） | 1（最高） |
| 2 | **环境变量** | `SOKO_STRICT_AT=1|0`（**保留**：CI、脚本、逃生门） | 2 |
| 3 | **编辑器** | VS Code `sokonanoda.dialect.strictImplicitArgs` → LSP `initializationOptions` + `workspace/didChangeConfiguration`（**改设置即时生效**，不重启） | 3 |
| 4 | **文件指令** | `-- sokonanoda:dialect strict`（与既有的 `-- sokonanoda:prelude none` **同形**，`prelude.rs:41` 是现成先例） | 4 |
| 5 | **项目清单** | `sokonanoda.toml` 的 `[dialect] strict_implicit = true`（`project/manifest.rs:19` 的 `Manifest` 加一格） | 5 |
| — | **默认** | **关**（= 旧读法；与 0.95.0 一致，翻默认是 v2 第 4 棒的事） | — |

* 键写成 **`[dialect] strict_implicit`**（而不是塞进顶层）：将来同类开关（是否允许省略 `@`、
  宇宙层是否隐式……）都进同一个表，避免顶层键无限增殖。
* **不做**用户级全局配置（`~/.config/...`）：先覆盖「项目」与「编辑器」两处，够用 ✓。

## §3 值怎么流

### §3.1 一个值
`CompileOptions` 增加 `dialect: Dialect`（**两态 enum**，不是 `bool` —— 将来要加第三态
「严格但允许省略前导隐式」时不用再改签名）。

### §3.2 只解析一次
`resolve_dialect(cli, env, editor, file_directive, manifest) -> Dialect`：五处来源按 §2 逐级覆盖，
**只在这里**读环境/清单/设置；之后全链路只读 `options.dialect`。
`strict_at_enabled()` 保留为**薄封装**（读 options），调用点一个不改，迁移成本最低。

### §3.3 进缓存键（否则一定会串味）
* `compile/cache/keys.rs`：`Dialect` 折进 `flags_state()` 的同一处哈希（**顺序无关**的同一套写法）；
* `judge::options_key()`：从 `format!("{:?}", prelude)` 扩成 `prelude + dialect`；
* **反向验证（必须有）**：同一 `<模块根>` 先 loose 跑一遍、再 strict 跑同一文件 ⇒
  第二次**必须 miss**，且判定与"清空产物后冷跑"逐字节一致。

### §3.4 LSP 的热切换
`initialize` 时按 `initializationOptions` 解析一次；`workspace/didChangeConfiguration`
变了就**重建 options + 作废本会话的判定缓存**（不是重启服务器）。判据：改一次设置 ⇒
下一次按键的判定翻转（结构化断言，不看墙钟）。

## §4 分段实现路径（6 段，每段一条判据、可单独发版）

| 段 | 交付 | 判据（可执行） | 可回退 |
|---|---|---|---|
| **S1** | `CompileOptions.dialect` + `strict_at_enabled()` 改读 options；**环境变量仍是唯一来源** | 全语料 356 两档 `--json` **逐字节**与改动前相同 · `front --lib` 1006/1006 | 恢复 env 直读（1 处） |
| **S2** | 方言**进缓存键**（§3.3）+ 反向验证 | 同目录两档交替跑 10 次，判定稳定；`strict` 首次必 miss | 去掉那一格 |
| **S3** | CLI `--dialect strict|loose`（`grade`/`build`/`query`/`course`） | `--dialect strict` 与 `SOKO_STRICT_AT=1` 的输出**逐字节相同** | 删旗标 |
| **S4** | `sokonanoda.toml` 的 `[dialect]` + 文件指令 `-- sokonanoda:dialect` | 清单一例、指令一例、两者冲突按 §2；`import` 闭包里指令不一致 ⇒ 一条诊断（照 `import-prelude-conflict` 同形） | 不认该键（回到 default） |
| **S5** | LSP: `initializationOptions` + `didChangeConfiguration` 热切换 | LSP 单测：两次 `initialize` 判定不同 · 一次 config change 后判定翻转 | 只认 `initialize` |
| **S6** | VS Code 设置项 + `docs/protocol.md` wire + `skills/` + `docs/vscode-dev-guide.md` 同轮 | 扩展 stub 宿主测试 + 真 e2e 一条（改设置 ⇒ 判定翻转） | 撤设置项 |

**顺序不能换**：S6 **必须**在 S1–S5 之后 —— 先让用户能配、配了却不生效，比不能配更坏 ✗。

## §5 与性能计划（`strict-mode-v2.md`）的关系

* **互不阻塞**：本文只保证"想用严格档的人能开"，不改默认；v2 保证"开了不慢"。
* S1/S2 顺带把 v2 第 1 棒要的读数入口接上（方言一旦是 `options` 里的值，
  探针按 `options.dialect` 分桶就天然成立 ✓）。
* **翻默认**仍排在 v2 第 4 棒之后，且**单独一笔、独占一轮 CI**。

## §6 边界（不做）

* 不做**按声明/按文件逐条**的方言（粒度止于"文件指令"）；
* 不做"自动择优"（按内容猜方言 = 判定不可预测 ✗）；
* 环境变量**不删**（CI 与逃生门要用），但它**不是用户界面**；
* 不在 S1–S2 里顺手改判定语义（那段只搬位置、不改行为）。
