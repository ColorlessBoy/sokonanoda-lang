# 方言（严格隐式参数）做成**可配置参数**：配置面 + 分段实现路径

> 用户 2026-10-20 第三条拍板：「**严格隐式参数**我想还是变成**可配置的参数**，
> project toml、vscode 插件等等地方可以配置」。本文是这条的唯一执行依据；
> 性能那半边在 `docs/design/strict-mode-v2.md`（**两件事互不阻塞**）。

## §0 结论

**一个值、五处来源、一条优先级链**；实现分 **6 段**，每段一条判据、可单独发版。
最要紧的一条不是"能不能配"，是**配置化时别把缓存键搞坏**（见 §3.3）。

## §0bis 2026-11-01 的现状（S6 之后）

**strict 的过渡闸门已删**（`effective = dialect.strict()` / `no_fallback = dialect.strict()`，两道闸门环境变量从代码里消失）⇒ **strict 只由 §2 的五处来源决定**，不再需要任何环境变量。
**默认档没有翻**：`Dialect::default()` 仍是 `Loose`、`DialectSources::resolve()` 五档全空仍落 loose（实测不设环境变量、不传 `--dialect` ⇒ 旧读法）⇒ **翻默认是独立决策**（§5），届时设置项 `default` 与本节同轮改。口径与偏差：`REQUIREMENTS.md` §9.40。

## §1 当初的坑（为什么配置化必须先做缓存键）

1. `implicit_msg::strict_at_enabled()` 当时**每次调用都读 `env::var`**，而它在热路径上 ⇒ 必须**一次解析、进 `CompileOptions`**（§3.2）；`CompileOptions` 当时只有 `prelude`，方言无处可放。
2. ⚠ ⚠ **缓存键的接缝（本文最重要的一条）**：`SOKO_STRICT_AT` 不串味靠 `compile/cache/keys.rs::flags_state()` 把**所有** `SOKO_*` 折进键；方言一旦来自 toml / 编辑器（**不经过环境变量**）就从键里掉出来 ⇒ 两档共用 `<模块根>/.sokonanoda/` 会**互相命中**⇒ 第一步必须**同时**进键（§4 S2）。

## §2 配置面与优先级（细粒度优先）

| # | 来源 | 形态 | 优先级 |
|---|---|---|---|
| 1 | **CLI 旗标** | `sokonanoda grade --dialect strict <file>`（`build`/`query`/`course` 同认） | 1（最高） |
| 2 | **环境变量** | `SOKO_STRICT_AT=1|0`（**唯一存留**：CI、脚本、逃生门；两道过渡闸门已删） | 2 |
| 3 | **编辑器** | VS Code `sokonanoda.dialect.strictImplicitArgs` → LSP `initializationOptions` + `workspace/didChangeConfiguration`（**改设置即时生效**，不重启） | 3 |
| 4 | **文件指令** | `-- sokonanoda:dialect strict`（与 `-- sokonanoda:prelude none` **同形**，`prelude.rs:41` 是现成先例） | 4 |
| 5 | **项目清单** | `sokonanoda.toml` 的 `[dialect] strict_implicit = true`（`project/manifest.rs:19` 的 `Manifest` 加一格） | 5 |
| — | **默认** | **关 = 旧读法（loose）** —— 2026-11-01 仍如此（翻默认见 §0bis / §5） | — |

* 键写成 **`[dialect] strict_implicit`**（不塞顶层）：将来同类开关都进同一个表。
* **不做**用户级全局配置（`~/.config/...`）：先覆盖「项目」与「编辑器」两处，够用 ✓。

## §3 值怎么流

### §3.1 一个值
`CompileOptions` 增加 `dialect: Dialect`（**两态 enum**，不是 `bool` —— 将来加第三态「严格但允许省略前导隐式」时不用改签名）。

### §3.2 只解析一次
`resolve_dialect(cli, env, editor, file_directive, manifest) -> Dialect`：五处来源按 §2 逐级覆盖，**只在这里**读环境/清单/设置；之后全链路只读 `options.dialect`（`strict_at_enabled()` 退化成薄封装）。热路径**不再读环境** ✗（那是 §1 第 1 条的坑）。

### §3.3 进缓存键（否则一定会串味）
* `compile/cache/keys.rs`：`Dialect` 折进 `flags_state()` 的同一处哈希（**顺序无关**的同一套写法）；
* `judge::options_key()`：`prelude` 之外再折 `dialect`；
* **反向验证（必须有）**：同一 `<模块根>` 先 loose 跑、再 strict 跑同一文件 ⇒ 第二次**必须 miss**，且与"清空产物后冷跑"逐字节一致。

### §3.4 LSP 的热切换
`initialize` 按 `initializationOptions` 解析一次；`workspace/didChangeConfiguration` 变了就**重建 options + 作废本会话判定缓存**（不是重启）。判据：改一次设置 ⇒ 下一次按键判定翻转（结构化断言，不看墙钟）。

## §4 分段实现路径（6 段，每段一条判据、可单独发版）

| 段 | 交付 | 判据（可执行） | 可回退 |
|---|---|---|---|
| **S1** | `CompileOptions.dialect` + `strict_at_enabled()` 改读 options | 全语料两档 `--json` **逐字节**与改动前相同 · `front --lib` 1006/1006 | 恢复 env 直读 |
| **S2** | 方言**进缓存键**（§3.3）+ 反向验证 | 同目录两档交替跑 10 次判定稳定；`strict` 首次必 miss | 去掉那一格 |
| **S3** | CLI `--dialect strict|loose` | `--dialect strict` 与 `SOKO_STRICT_AT=1` 输出**逐字节相同** | 删旗标 |
| **S4** | `[dialect]` + 文件指令 `-- sokonanoda:dialect` | 清单一例、指令一例、冲突按 §2；`import` 闭包指令不一致 ⇒ 一条诊断 | 不认该键 |
| **S5** | LSP `initializationOptions` + `didChangeConfiguration` 热切换 | 两次 `initialize` 判定不同 · 一次 config change 后判定翻转 | 只认 `initialize` |
| **S6** | VS Code 设置项 + `docs/protocol.md` wire + `skills/` + `docs/vscode-dev-guide.md` 同轮 | 扩展 stub 宿主测试 + 真 e2e 一条（改设置 ⇒ 判定翻转） | 撤设置项 |

**顺序不能换**：S6 **必须**在 S1–S5 之后 —— 先让用户能配、配了却不生效，比不能配更坏 ✗。**S6 之后（2026-11-01）**：两道过渡闸门已删（§0bis），但**默认档未翻** ⇒ S1–S6 的产物原样成立。

## §5 与性能计划（`strict-mode-v2.md`）的关系

* **互不阻塞**：本文只保证"想用严格档的人能开"，不改默认；v2 保证"开了不慢"。
* S1/S2 顺带把 v2 第 1 棒的读数入口接上（探针按 `options.dialect` 分桶天然成立 ✓）。
* **翻默认仍未做**（2026-11-01）：**独立决策、另行拍板**，单独一笔、独占一轮 CI；届时 §2 的"默认"行、`Dialect::default()`、设置项 `default` 与 `skills/` 说明同一轮改。

## §6 边界（不做）

* 不做**按声明/按文件逐条**的方言（粒度止于"文件指令"）；不做"自动择优"（按内容猜方言 = 判定不可预测 ✗）；
* **两道过渡闸门环境变量已删**（2026-11-01）；`SOKO_STRICT_AT` **不删**（CI 与逃生门要用），但它**不是用户界面**；
* 不在 S1–S2 里顺手改判定语义（那段只搬位置、不改行为）。
