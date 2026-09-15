# 交接文档（HANDOVER）

> 目的：一页看清「现在在哪、还剩什么、怎么继续」。权威仍在
> `REQUIREMENTS.md`（要求总账）、`STATUS.md`（逐轮日志）、`ROADMAP.md`（里程碑）；
> 本文是**汇总与索引**，随轮次更新。
>
> 快照：**v0.39.0**（2026-09-14），最近一轮 **第六十八轮**。仓库根入口 `AGENTS.md`。

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

## 2. 本会话完成的工作（第五十三～六十八轮，全部已发布）

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

> 更早轮次见 `STATUS.md`（最近 3 轮）+ `docs/STATUS-ARCHIVE.md`（第 1–65 轮原文）。

## 3. 剩余 TODO（按建议顺序）

### A. `judge_infer` 往返健壮性（小、影响面广）
- **问题**：`crates/front/src/judge.rs` 的 `judge_infer` 用「内核渲染类型文本 →
  前端再解析」逐层剥 Pi；当类型里含**以箭头结尾的依赖函数** binder 时，
  `proof::render_expr` 在 domain 位不补括号，往返会腐蚀 telescope（依赖类型的
  判定/建议/level 查询都可能受影响）。
- **修法**（二选一）：`judge.rs` 一次性取内核类型并直接剥，不再 render→parse；
  或 `proof::render_expr` 给**箭头/Pi 作为 domain** 的位置加括号。
- **验收**：加回归测试（motive 引用「类型为 `… -> …` 的依赖函数」的 binder 的
  依赖 `match`）；`cargo test -p sokonanoda-front`。设计并入
  `docs/design/match-dependent-motive.md` §7 已知限制。

### B. `match` Phase 2 余项
- **带索引归纳**（`inductive Vec (A : Type) : Nat -> Type`）：需 `num_indices>0`
  的声明 + recursor + match（motive 依赖索引）。设计需先定稿（现 `match.md` §2
  列为不做）。
- **`match` tactic**（`by` 块内用 `match`）：设计与白名单待定。
- **嵌套/守卫/字面量模式**（`| some (some x) =>`、`| 0 =>`）：需模式编译扩展。

### C. I6 剩余
- **prelude `Bool`**：可作为真实可信归纳加（与 Nat 同法）；当前无课程依赖，价值中等。
- **binder 类型推断（非依赖情形）**：让 `fun (x)` 之类能从期望类型推断 binder 类型；
  设计待写（`docs/design/elaborator-let-match.md` 提到过路线）。
- `Nat.succ`/`Nat.add` 边界：`Nat` 现为真实归纳、`Nat.add` 仍原生自引用定义
  （见 `docs/architecture.md` §5.4）；如需可补裸名 `#reduce` 测试。

### D. 远期（L2/L3）
- 协作/多用户、远程；compiler service 的跨文件转播 / `setContent`（v1 未做）。

## 4. 已知限制 / 技术债

- **spine meta 方案 A** 仍有缺口：更深嵌套、`def` 包裹结果类型的 whnf 展开
  （需内核/pp 暴露 whnf，违反冻结）→ 仍走 B′；见 `docs/design/spine-meta-a.md`。
- **参数化归纳 v1**：带索引、宇宙多态参数、互/嵌套递归不做。
- **`match` 依赖 motive**：见 A 的已知限制。
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
