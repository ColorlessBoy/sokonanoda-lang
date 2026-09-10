---
name: sokonanoda-dev
description: Develop and extend the sokonanoda-lang teaching compiler stack (Rust workspace kernel/front/cli/lsp) safely - the frozen kernel, TDD three-layer testing, modularization limits, docs-first workflow and CI gates. Use when taking over development, adding syntax or compiler features, or touching CI/docs in this repo.
---

# sokonanoda-dev：接手 sokonanoda-lang 开发

## 0. 接手清单（按序读完再动手）

1. `docs/REQUIREMENTS.md` —— 用户全部要求的权威总账（含硬规则）；
2. `docs/STATUS.md` —— 当前进度日志（最新一轮在最上）；
3. `ROADMAP.md` —— 里程碑与 §10 待办（I 系列编号）；
4. `docs/architecture.md` —— 流水线、内核机制与 §8 gotchas；
5. `docs/design-i8-i9.md` 等设计文档 —— 已确认方案的 as-built 记录。

## 1. 不可动摇的硬规则（REQUIREMENTS §2，违者返工）

1. kernel 冻结快照：不改语义、不加间接层、不动热路径；允许的改动清单在
   `docs/architecture.md` §6（bugfix 须带三层回归测试）；
2. 无官方 Lean 工具链依赖（lean/lake/lean4export 一律不调用）；
3. 教学语法是真实 Lean 4 的子集；新增语法 = 课程 + 测试 + 白名单三件套；
4. 判定永远走 kernel：新增功能禁止文本比对（`front::judge` 是合成声明走
   完整流水线的范例）；
5. 反馈即功能：类型/化简/打印/错误都要结构化输出，人和模型都能无文档驱动。

## 2. 工作流（TDD 三层 + 文档先行）

- **先写设计**：新功能先出设计方案落 `docs/`（含取舍与验收标准），再动手；
- **测试三层**：front 单元测试（`crates/front/src/compile/tests.rs` 等模块内
  `#[cfg(test)]`）→ CLI e2e（`crates/cli/tests/`）→ 语料/协议/golden 守护
  （`examples.rs` / `protocol.rs` / `course.rs` / `skill.rs`）；
- **多用 subagent**：探索/调研/机械重构派出去并行，主会话做核心设计编码，
  产出后主会话验证（编译 + 全量测试）;
- **模块化**：任何文件接近 ~500 行即拆分；公开 API 用 re-export 保持稳定；
- **交接友好**：落 commit 前先更新 `docs/STATUS.md`；用户新要求追加进
  `docs/REQUIREMENTS.md` §9 并注明日期，冲突时以该文件为准。

## 3. 质量门禁（CI 与本地一致）

```bash
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
cargo clippy --workspace --all-targets      # 教学 crates 经 [lints] deny；kernel 只 warning
cargo test --workspace --locked             # 全部 13 个套件
```

- 教学 crates 的严格度来自各自 `Cargo.toml` 的 `[lints.rust] warnings = "deny"`；
- kernel 是冻结快照：其 lint 保持 warning 级，fmt 门禁不覆盖（rustfmt.toml
  需要 nightly）；
- 协议防漂移：改事件/输出格式必须同步 `docs/protocol.md`
  （`protocol.rs` / `skill.rs` conformance 测试会抓漂移）。

## 4. 环境搭建（新机器）

```bash
git clone https://github.com/ColorlessBoy/sokonanoda-lang.git && cd sokonanoda-lang

# 方式 ①（agent / headless，与 VS Code 解耦）：一条命令
bash scripts/soko.sh setup     # 版本锁定下载 CLI + LSP（幂等；零 cargo）
bash scripts/soko.sh doctor    # 0=就绪 3=未就绪；--json 机器可读

# 方式 ②（开发必需）：源码编译（CLI + LSP + 全量测试）
cargo build --release --locked -p sokonanoda-cli -p sokonanoda-lsp
export PATH="$PWD/target/release:$PATH"

# 门禁（= CI：fmt + clippy + test + playground 锚点）
bash scripts/soko.sh gate
```

Release 资产：每个平台的 `sokonanoda-lsp-<rust-triple>.tar.gz` 与
`sokonanoda-cli-<rust-triple>.tar.gz`（各 8 个；后者就是可直接执行的 CLI），
加上 VSIX ×9（8 平台包内嵌两者 + universal 回退包）。不要用
`releases/latest`——下载 URL 按仓库版本锁定，版本错配是明确要避免的故障。
完整入门设计见 `docs/design-onboarding.md`。

## 5. VS Code 扩展开发规范

`editor/vscode/` 的改动有独立开发规范：`docs/vscode-dev-guide.md`。
版本纪律（feature→minor / fix→patch）、测试三层（静态契约→集成→手动）、
常见坑（node_modules 打包/didOpen 通知/LSP 帧格式/代理）全在里面。

## 6. 常用命令

```bash
cargo test -p sokonanoda-front compile::tests::       # 前端单测
cargo run -q -p sokonanoda-cli --bin sokonanoda -- examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda
cargo run -q -p sokonanoda-lsp                        # 编辑器反馈通道
```

内核/前端机制细节（arena 生命周期、prelude 与原生 Nat 技巧、conv 缓存、
EnvBuilder 语义）见 `docs/architecture.md`——不要凭直觉猜内核行为。
