# 交接文档（HANDOVER）

> 目的：一页看清「现在在哪、还剩什么、怎么继续」。权威仍在
> `REQUIREMENTS.md`（要求总账）、`STATUS.md`（逐轮日志）、`ROADMAP.md`（里程碑）；
> 本文是**汇总与索引**，随轮次更新。
>
> 快照：**v0.40.0**（2026-09-15），最近一轮 **第七十轮**。仓库根入口 `AGENTS.md`。

## 1. 30 秒接手

```bash
sokonanoda setup        # 用户/agent：版本锁定下载 CLI+LSP（零 cargo，幂等）
sokonanoda doctor       # 0=就绪 3=未就绪
sokonanoda gate         # 贡献者门禁：fmt + clippy + test + playground 锚点（需要 cargo）
cargo test --workspace --locked   # 全量 13 套件
```

- **用户/agent 路径零工具链**：执行只用 `sokonanoda` 子命令 / Release 二进制 / VSIX；
  **禁止**要求安装 Rust/cargo（REQUIREMENTS §2 第 9 条）。
- **贡献者**才需要 cargo；CI 与本地命令一致。
- 判定永远走 kernel，**禁止文本比对**（REQUIREMENTS §2 第 4 条）。

## 2. 本会话完成的工作（第五十三～七十七轮，全部已发布）

| 轮 | 版本 | 内容 | 设计 / 证据 |
|---|---|---|---|
| 53 | — | TODO 清账：穷尽守卫修复、codeLens/quick-fix 测试、内核 2 个 ignored fixture 重建解禁、`install.sh`+`.devcontainer`、4 份大项设计 | 见各 `docs/design/*` |
| 54 | 0.28.0 | 值位 `let`（Phase 1） | `docs/design/elaborator-let-match.md` |
| 55 | 0.29.0 | 编译器服务事件流：`file.didChange` + `service.hello` + `--doc/--workspace` | `docs/design/compiler-service-events.md` |
| 56 | 0.30.0 | VS Code webview Infoview 目标面板 | `docs/design/webview-infoview.md` |
| 57 | 0.31.0 | 扩展**强制内置 LSP**（`serverOverride` opt-in）+ `sokonanoda: doctor` 自检 | `docs/design/extension-server-policy.md` |
| 58 | 0.32.0 | spine meta 方案 A（请求期内核探针补子洞期望类型） | `docs/design/spine-meta-a.md` |
| 59 | 0.32.1 | I8 early-cutoff 依赖精确化 + arena 基准立项 | `docs/design/early-cutoff.md` |
| 60 | 0.33.0 | `match` v1（非递归源内归纳） | `docs/design/match.md` |
| 61 | 0.33.1 | tactic hover 呈现升级（代码围栏 + 高亮 + 排版） | `docs/design/tactic-hover.md` §5 |
| 62 | 0.34.0 | 无注解 `let`（内核推断绑定类型） | `docs/design/elaborator-let-match.md` §13 |
| 63 | 0.35.0 | `match` 递归归纳（自动插入 IH） | `docs/design/match.md` §10 |
| 64 | 0.35.1 | 发布加固：`SHA256SUMS` + SLSA provenance | `docs/RELEASE.md` §6 |
| 65 | 0.36.0 | `match` 支持 prelude `Nat`（内置 Nat 改真实可信归纳） | `docs/design/match.md` §10 |
| 66 | 0.37.0 | watch stdin 客户端命令（subscribe/unsubscribe/ping） | `docs/design/compiler-service-events.md` §9 |
| 67 | 0.38.0 | 参数化归纳声明（非带索引）+ `match` | `docs/design/parameterized-inductives.md` |
| 68 | 0.39.0 | `match` 依赖 motive（归纳法形状可用） | `docs/design/match-dependent-motive.md` |
| 69 | 0.39.1 | `judge_infer` 类型往返健壮性（Arrow domain 补括号） | `docs/design/match-dependent-motive.md` §8 |
| 70 | 0.40.0 | 统一 goal 呈现（`front::semantic` runs）+ Infoview 落右侧 + engine ^1.106 + 市场简介护栏 | `docs/design/goal-rendering.md` |
| 71 | 0.41.0 | prelude `Bool`（非递归真实可信归纳） | `docs/design/match.md` §10 Phase 5 |
| 72 | 0.42.0 | `match` 模式编译器 v1（嵌套/字面量/通配/守卫） | `docs/design/match-patterns.md` |
| 73 | 0.43.0 | 呈现面高亮统一（`sokonanoda` 围栏全量） | `docs/design/goal-rendering.md` §7 |
| 74 | 0.44.0 | Infoview 声明类型提示 + 点击跳转 | `docs/protocol.md` / `webview-infoview.md` |
| 75 | 0.45.0 | 应用位置 binder 类型推断（I6 最后一项） | `docs/design/elaborator-let-match.md` as-built |
| 76 | 0.46.0 | `match` 作为 tactic（`by` 块内）+ judge 前缀修复 | `docs/design/by-tactics.md` §2 |
| 77 | 0.47.0 | 带索引归纳（`Vec A n` 声明 + 派生 recursor + match） | `docs/design/indexed-inductives.md` |

> 更早轮次见 `STATUS.md`（最近 3 轮）+ `docs/STATUS-ARCHIVE.md`（第 1–71 轮原文）。

## 3. 剩余 TODO（按建议顺序）

> **状态（2026-09-15）**：§3 A/A′/A″/B/C 的**可执行项已全部完成**（0.40.0–0.47.0）；
> 仅剩 §3 D「远期 L2/L3」（协作/远程、compiler service 跨文件转播）与 §4 的
> 已文档化技术债（多数需内核/pp 变更，违反 kernel 冻结）。

### A′. ~~统一 goal 呈现 + Infoview 落右侧~~ ✅ 已完成（0.40.0）
- 单一分类源 `front::semantic`（`tag_runs`/`tag_expr`/`declaration_kinds` +
  `SemanticKind::{ALL, as_str}`）；`soko/stateAt`/`soko/goals` 下发
  `goal_runs`/`ty_runs`（旧字符串字段保留）；Infoview 渲染 `tok-<kind>`
  （主题变量单一映射）；视图移入右侧 `secondarySidebar` 容器，engine
  `^1.106.0`；删握手 + 「暂时不可用」，静默回退树组；TM 语法与
  `front::semantic` 由测试锁死。见 `docs/design/goal-rendering.md`。

### A. 已完成
- **`judge_infer` 往返健壮性** ✅（0.39.1）：`proof::render_expr` 的 Arrow domain 位
  补括号；依赖 `match`/suggest/hover/level 均受益；证据 `render_expr_round_trips`
  + `match_dependent_motive_with_function_typed_binder_round_trips_safely`。

### A″. ~~呈现面高亮统一~~ ✅ 已完成（0.43.0）
> 用户 2026-09-15 追问后补做。
- 统一手段：LSP `code_block`/`CODE_LANG`、扩展 `codeMarkdown`——凡渲染
  `.sokonanoda` 文本一律 ` ```sokonanoda ` 围栏（着色只来自 `front::semantic`
  的 TM 语法/语义 token）。
- 覆盖：表达式/签名 hover（原 ` ```text `）、声明 hover（签名 + 目标态代码块）、
  补全 documentation 签名、练习树 tooltip（`appendCodeblock(…, "sokonanoda")`）。
- 刻意保持纯文本（VS Code 不渲染 markdown/无法着色）：诊断消息、inlay hint、
  TreeItem.description、CodeAction 标题；见 `docs/design/goal-rendering.md §7`。
- 契约：`code_fences_always_use_the_sokonanoda_language`、
  `rendered_language_text_uses_the_sokonanoda_fence`。

### B. `match` Phase 2 余项
- ~~**带索引归纳**~~ ✅ 已完成（0.47.0）：`num_indices>0` 声明 + index-aware
  派生 recursor + `match`（常量结果类型）；结果类型依赖索引不在 v1。见
  `docs/design/indexed-inductives.md`。
- ~~**`match` tactic**（`by` 块内用 `match`）~~ ✅ 已完成（0.46.0）：`by` 白名单加
  `match`（臂体是项，等价 `exact (match …)`）；顺带修 `judge_terms` 前缀（使
  `by exact match …` 可用）。臂体内再写一串 tactic 为后续可选扩展。
- ~~**嵌套/守卫/字面量模式**~~ ✅ 已完成（0.42.0）：有序 arm + 列式模式编译；
  通配 `_`、嵌套 `some (succ k)`、Nat 字面量（脱糖 `succ^k zero`）、Bool 守卫
  `if`；`docs/design/match-patterns.md`。剩余：`as`/or 模式、多 scrutinee、
  `if/then/else` 表达式。

### C. I6 剩余
- ~~**prelude `Bool`**~~ ✅ 已完成（0.41.0）：`install_bool_prelude`（非递归真实可信归纳，
  `Bool.true`/`Bool.false` + `Bool.rec`）；`match` 可用；文件自带 `inductive Bool`
  时让位（`explicit_bool` 闸 + `PreludeShape` 四元组）。见 `docs/design/match.md` §10 Phase 5。
- ~~**binder 类型推断（非依赖情形）**~~ ✅ 已完成（0.45.0）：有期望望远镜时可推断
  （既有）；无期望的**应用位置**从实参类型推断（`annotate_application_lambda`，
  支持柯里化），实参不足仍报 `elab-untyped-binder`。见
  `docs/design/elaborator-let-match.md` as-built。
- ~~`Nat.succ`/`Nat.add` 边界裸名 `#reduce` 测试~~ ✅ 已完成（0.42.0）：
  `bare_prelude_nat_names_stay_terminating`（裸 `#reduce Nat.add` 终止为常量）。

### D. 远期（L2/L3）
- 协作/多用户、远程；compiler service 的跨文件转播 / `setContent`（v1 未做）。

## 4. 已知限制 / 技术债

- **spine meta 方案 A** 仍有缺口：更深嵌套、`def` 包裹结果类型的 whnf 展开
  （需内核/pp 暴露 whnf，违反冻结）→ 仍走 B′；见 `docs/design/spine-meta-a.md`。
- **参数化归纳 v1**：带索引、宇宙多态参数、互/嵌套递归不做。
- **`match` 依赖 motive**：`judge_infer` 往返限制已修（0.39.1，见 §3 A）；其余同 B。
- **prelude `Nat` 经 `Nat.rec` 归约**：结果可能显示为不合并一元链（与 numeral
  def-eq），已文档化并钉测试；`#reduce 1 + 1 => 2`、`three => 3` 正常。
- **perf 套件是哨兵**：外部基准（Lean Kernel Arena）已立项但 opt-in
  （`scripts/perf-arena.sh`）；见 `docs/PERF.md`。
- **`TESTING.md §5` 盲区**：编辑器 codeLens/quick-fix 已补进程内 rpc；VS Code
  Electron 集成走 `editor/vscode/src/test/extension.test.js`。

## 5. 环境 / Gotchas（本会话实打实踩过）

- **编辑器 LSP 与发布版本可能不一致**：
  - VS Code 扩展**默认强制用内置 LSP**（0.31.0 起）；`sokonanoda.serverPath` /
    `SOKONANODA_LSP_BIN` / 工作区构建**默认被忽略**，要覆盖需开
    `sokonanoda.serverOverride`（贡献者）。用 `sokonanoda: doctor` 看解析来源
    （`source=`）与版本；`restart server` 回执带 `source=`。
  - opencode 插件**优先用仓库构建** `target/{debug,release}/sokonanoda-lsp`；
    改 Rust 后需重启（`cargo build -p sokonanoda-lsp` 或 `sokonanoda gate` 会刷新
    debug 构建；注意 `cargo test`/`clippy` **不一定**重编该 bin）。
- **旧扩展版本**：VS Code 在**下次完整启动**时清理 `.obsolete` 旧版本；长时间不整退
  会堆积（只占磁盘，不影响解析——扩展始终用运行中那份的内置 bin）。
- **`sokonanoda version` 报的是下载缓存**，不是编辑器在跑的服务器；要看真实运行版本
  用 LSP 的 `soko/version`。
- **CI：marketplace-publish 间歇 Azure gallery 超时**（已多次记录）：`build`/
  `package-vsix`/`github-release` 会先成功（Release 25+1 资产齐全），仅发布步骤失败；
  探活后 `gh run rerun --failed` 即可。台账 `docs/CI-FAILURES.md`。
- **发布资产现为 26 个**（8 lsp + 8 cli + 9 vsix + `SHA256SUMS`），另附 SLSA
  provenance；校验见 `docs/RELEASE.md` §6。
- **本地 perf 哨兵在机器重载时会误报**（`incremental_edit_anywhere_is_fast` 受
  CPU 争用）；单跑通过即环境问题，重跑 `gate`。

## 6. 关键文档索引

- 要求总账：`REQUIREMENTS.md`（§2 硬规则、§9 追加日志）
- 进度：`STATUS.md` / 归档 `docs/STATUS-ARCHIVE.md`
- 里程碑/待办：`ROADMAP.md §10`
- 架构/gotchas：`docs/architecture.md`（§6 内核改动清单、§8 gotchas）
- 协议：`docs/protocol.md`；测试地图：`docs/TESTING.md`
- 发布：`docs/RELEASE.md`；CI 台账：`docs/CI-FAILURES.md`
- VS Code 规范：`docs/vscode-dev-guide.md`
- 设计：`docs/design/`（新增见 `docs/README.md` 列表）
- 技能：`skills/sokonanoda-{dev,teacher,ci}`
