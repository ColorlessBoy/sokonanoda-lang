# 项目要求总账（REQUIREMENTS）

> 这是用户全部要求的**权威记录**。任何 agent 接手任何任务前先读本文，
> 再读 `ROADMAP.md`（里程碑）与 `docs/STATUS.md`（当前进度）。
> 新要求出现时追加到本文末尾并注明日期；冲突时以本文为准。

## 1. 产品愿景（不变）

用户与 code agent 共同看着同一个 `*.sokonanoda` 文件（共享画布）：
agent 从零讲课、出题；用户作答；我们自己的编译器实时给出反馈，
同一份反馈既给用户也给 agent。反馈通道以 LSP 为中心（LSP-first）。

## 2. 不可动摇的硬规则（来自 ROADMAP §1，长期有效）

1. **kernel 保持完整**，不因教学裁剪；
2. **无官方 Lean 工具依赖**（lean/lake/lean4export/leanc/elan 一律不调用）；
3. **教学语法是真实 Lean 4 的子集**，填完洞的声明放进官方 Lean 依然合法；
4. **语法白名单即课程**：parser 只认课程引入过的语法点；
5. **分层推进**：L0 编译器 → L1 服务 → L2 编辑器 → L3 agent 协作；
6. **TDD 与重复测试**：front 单测 + CLI 端到端 + 课程语料三层覆盖；
7. **反馈即功能**：类型/化简/打印/错误都结构化输出，人与模型都能无文档驱动；
8. **判定永远走 kernel**，不做文本比对（`proof.rs::assumption` 的文本比对是待替换草案）。

## 3. 内核性能是产品优势（2026-09-06，用户要求）

- sokonanoda 内核的快（arena + hash-consing + 闭包求值，相对官方内核 10–100x）
  是本项目立身之本：**任何重构/新功能都不得触碰 `crates/kernel` 热路径**；
- 前端只消费内核公开 API；教学功能（prelude、elaborator）不得给内核加间接层；
- 守护手段：保留内核全量测试 + 新增前端 perf 冒烟测试（大数字原生归约、
  iota 深归约链），CI 跑通，防止无意中把快路径弄慢。

## 4. 工程标准：模块化、单文件不许越长越大（2026-09-06，用户要求）

- **现状不合格**：`front/lib.rs` ~1200 行、`front/compile.rs` ~2100 行，单文件巨石难维护；
- **目标结构**（公开 API 用 re-export 保持稳定，调用方不改）：
  - `crates/front/src/`：`span.rs` / `token.rs` / `ast.rs` / `diagnostic.rs` /
    `parser.rs` / `compile/{mod,error,event,report,elab,prelude,check}.rs` / `proof.rs`；
  - `crates/cli/src/`：`main.rs`（入口与参数）+ `check.rs` / `json_report.rs` /
    `repl.rs` / `help.rs`；
  - `crates/lsp/src/`：`main.rs` + `backend.rs` / `render.rs` / `actions.rs`；
  - **`crates/kernel` 保持上游快照原样**（只允许此前已注明的适配清单，见
    `docs/architecture.md` §6）；
- 新代码一律按此结构放；任何文件接近 ~500 行即考虑拆分；
- 每个模块开头一段 doc comment 说明职责；交接靠文档不靠读巨石文件。

## 5. prelude 可选：既能全加载，也能完全裸跑（2026-09-06，用户要求）

- 教学需要两种模式：**Full**（内置 Nat/Eq 等受信任基元）与 **Bare**（完全不加载
  任何 prelude，学生/课程从零构造一切，例如用显式 `inductive Nat` 块）；
- API：`compile_fol/check_document` 接受编译选项（`PreludeMode::{Full, Bare}`）；
- CLI：`--bare` 旗标（未来可扩展 `--prelude <file>` 自定义基元文件）；
- LSP：文件级注释指令 + 工作区设置（默认 Full）；
- Bare 模式下文件必须仍能通过完整内核检查（例如纯逻辑公理练习）；
- prelude 内容本身用 `.sokonanoda` 源语法书写再安装（签名与官方 Lean 一致，
  与真实 Lean 兼容；Eq/Eq.refl/Eq.subst 为 I6 第一批）。

## 6. 教学工作流（2026-09-06 确认并执行中）

- **课程层与执行层的关系（2026-09-07 用户明确）**：`course/` 只是**给大模型的
  路线图/素材库**（知识点、顺序、题池、可解性守护）；**具体执行层必须由大模型
  按每个用户灵活适配**——根据用户的错误历史、节奏、兴趣实时调整出题与讲解，
  动态画布（如 playground.sokonanoda）才是用户真正面对的表面。禁止把课程文件
  当成"用户直接消费的固定课程"。
- 画布 = 仓库根 `playground.sokonanoda`（纯声明式，无 `#` 命令，`--` 中文讲解）；
- agent（本会话的我）即老师：写定义/出题 → 用户作答 → agent 跑
  `cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda`
  读结构化事件（decl.checked / exercise.open / diagnostic+code+hint）决定下一步；
- 第一课内容：①表达式与类型 ②函数与箭头 ③命题与证明项 ④等式与 rfl；
  练习判定只走 kernel；`???` 洞（含 lambda 体内的部分作答）是合法状态；
- 课程细节与解答钥匙见 `docs/teaching-session.md`。

## 7. 开发过程要求（2026-09-06，用户要求）

- **多用 subagent**：探索/调研/机械重构/文档起草都派 subagent 并行做；
  主会话专注核心设计与编码；产出后主会话验证（编译+测试）；
- **持续头脑风暴**：新功能先出设计方案（写进 docs），再动手；
- **文档先行、交接友好**：`docs/REQUIREMENTS.md`（本文）、`docs/STATUS.md`（进度日志）、
  `docs/architecture.md`（架构事实）、`docs/lsp-notes.md` / `docs/vscode-notes.md`
  （外部标准调研）、`docs/teaching-session.md`（教学循环）、`docs/protocol.md`（事件协议）；
  每轮进度落 commit 前先更新 STATUS；
- LSP/VS Code 集成遵循业界标准做法（tower-lsp[-server]、vscode-languageclient、
  版本化诊断、FULL sync、自定义请求承载 goal 视图——见两份 notes 文档）。

## 8. 路线对齐

- 当前执行：I6（Eq prelude + binder 推断 + partial hole + prelude 可选化）
  → playground 开课 → I7 第一门课（course/ + golden）→ I8 增量 → I9 goal 视图
  → L2/L3（编辑器打包、service 事件流、讲课 agent 深化）。
- 详细验收标准以 `ROADMAP.md` §10 为准。

## 9. 要求追加日志

- 2026-09-06（会话 1）：接手项目，先 I6 后推进至终极形态；用户要尽快在
  playground.sokonanoda 开课，agent 当老师。
- 2026-09-06（会话 1 续）：多用 subagent；多写 docs 方便交接。
- 2026-09-06（会话 1 再续）：持续调研 Rust LSP/VS Code 标准实践；
- 2026-09-06（会话 1 补）：代码必须模块化、专业化（不得单文件巨石）；
  prelude 必须可选（Full / 完全 Bare）；多用 subagent；保持内核性能优势；
  本文档即为"记住所有要求"的机制。
- 2026-09-07：**充分测试是大规模合作与多次大规模重构的重要资产**——每个环节
  （lex/parse/elab/kernel/事件/CLI/JSON 协议/LSP/语料/文档一致性/perf）都要有
  自动化测试守护；落实为 157 个测试 + `docs/TESTING.md` 测试地图（本轮完成）。
- 2026-09-07（续）：I6 落地（prelude 可选 Full/Bare + `--bare` + 注释指令；
  Eq 三件套 prelude；binder 类型推断；partial hole）→ 178 个测试全绿；
  `playground.sokonanoda` 开课（12 练习 + 教学循环文档）。
- 2026-09-07（再续）：**课程层只是给大模型提供路线图，具体执行层需要大模型
  适配各个用户、灵活调整**——course/ 是 agent 素材库，不是用户直接消费的
  固定课程；I9 goal 视图第一段（假设列表 hover + exact/intro code action）。
