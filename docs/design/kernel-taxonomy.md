# 设计：内核错误分类学余项 + 失败声明建议 + 基准/fuzz 基建

> 状态：设计定稿（2026-09-07，第十三轮实施）。依据：`docs/notes/gap-analysis.md`
> 余项、`STATUS.md` 第十一/十二轮遗留、内核冷路径改动规则
> （REQUIREMENTS §3、LESSONS「内核冷路径改动」、architecture §6 记账）。

## 0. 头脑风暴与取舍

1. **同名消息「区分」的落点**：`conv.rs::is_prop_type`（`.expect("expected a
   sort")`，无 `got:` 渲染）与 `infer.rs::ensure_sort_v`（带 `got:`）共享
   `kernel-expected-sort` 一个码。教学上这是**同一种学习者错误**（需要类型处
   给了项），拆成两个码没有教学价值；正确解法是**统一消息形状**（两处都渲染
   `got:`），并以**不同措辞区分站点**（conversion 路径 vs 类型推断路径）供
   调试定位——分类器前缀不变，一个码保留。
2. **assert_eq! 灰色地带**：内核用 `assert_eq!` 写的「输入条件拒绝」会被
   分类器归入 `assertion failed:` → `kernel-internal`（「永不是学习者的错」
   ——但其实是）。本轮做**全量清点 + 分诊**：学习者可触发的站点改成带稳定
   消息的 `panic!`（归入既有家族或新增家族，分类器同步）；真内部不变量保留
   `assert_eq!`（归 internal，注释注明）。清单与分诊结论落报告。
3. **refine 子洞的 kernel 级 expected type**（M–L）：spine meta 需要内核/
   elaborator 深度配合，**本轮不做**（已记入 STATUS 余项，等专门设计轮）。
4. **失败声明（kernel-rejected）的针对性建议**：失败声明没有洞，编辑目标
   是**整个答案**——按声明的**类型形状**生成「重启骨架」（剥 Pi：
   `fun (x : A) => ???`）作为 code action 替换整个值位；结构生成、kernel 在
   下次编辑终审（`verified: false`）。不做「自动修错」——方向性/参数序错误
   属于教学时刻，骨架只是重启点。
5. **criterion 基准**：防 I8 增量静默劣化（本地跑，不进 CI 时长预算）；
   三个目标：原生大数归约、iota 链、Session 后缀重查（kernel_checks 语义
   已有测试锚定，基准钉时序）。
6. **fuzz harness**：parser/lexer 永不 panic 是硬要求（parse 返回 Result，
   semantic_tokens 不 panic）；独立 crate（`[workspace]` 置空表脱离工作区，
   cargo-fuzz 标准布局），CI 不跑，本地/定期跑。

## 1. 内核冷路径改动（K）

允许清单（只许冷路径，热循环零改动）：
- `conv.rs::is_prop_type`：`.expect("expected a sort")` →
  `panic!("expected a sort in conversion, got: {}", self.render_value_for_def_eq_error(depth, t))`；
- 全内核 `assert_eq!`/`assert!` 清点（`rg "assert" crates/kernel/src`）：
  分诊表（站点/条件/学习者可否触发/处理）；可触发的改稳定 `panic!` 消息，
  新家族需同时改 `front::error::refine_kernel_kind` 与 `docs/protocol.md`
  的 kernel 码清单（本轮 protocol.md 归 K 独占）。

## 2. 失败声明建议（L1）

- `front::suggest`：`SuggestionKind::Restart { skeleton: String }`——仅对
  `DeclStatus::Failed` 且**无洞**的声明生成：从声明类型（命令文本 parse）
  剥 Pi 得 `fun (x : A) => ???`（层数 ≤3，binder 名防撞已有假设）；
- lsp `actions.rs`：Failed 分支的编辑 = 替换**值位**（从命令文本定位
  `:=` 后的表达式 span——tokenize 精确，不扫文本；值位 span 可用
  DeclState？Failed 声明无 holes——需要从命令文本用 lexer 找 `:=` 与值
  首尾，落点在 actions 内部私有函数，全部用 tokenize）；
- 标题：`用目标形态重启：fun (x : A) => ???（先搭骨架，内核逐层判）`；
- 排序：Failed 声明的建议只有 Restart（无 exact/refine 竞争）。

## 3. 基准与 fuzz（M / N）

- **M（criterion）**：`crates/front/benches/pipeline.rs`（[[bench]] harness=false；
  criterion dev-dep）——(a) 原生大数：39 位 +1 归约；(b) iota：`add two two`
  深归约；(c) Session：5 声明改第 4 个的后缀重查。`cargo bench -p
  sokonanoda-front` 本地跑；CI 不跑基准。
- **N（fuzz）**：`fuzz/`（独立 crate，`[workspace]` 空表；不在 members）——
  target `parse_never_panics`：UTF-8 切片 → `front::parse`（Result）+
  `semantic::semantic_tokens`（不 panic）+ `compile::prelude_mode_from_source`；
  验收 `cd fuzz && cargo check`（stable）通过；用法文档进 `fuzz/README.md`
  （`cargo +nightly fuzz run parse_never_panics`）。

## 4. 文件分工（互斥清单）

| owner | 允许修改 |
|---|---|
| 主会话（已完成预接） | 本设计文档 |
| K（内核分类学） | `crates/kernel/src/{conv.rs, infer.rs, inductive.rs, env.rs, …}`（仅冷路径消息）、`crates/front/src/compile/error.rs`、`crates/front/src/compile/tests.rs`、`crates/kernel/tests/memory_api.rs`、`crates/cli/tests/cli.rs`、`docs/protocol.md`（仅 kernel 码清单） |
| L1（失败声明建议） | `crates/front/src/suggest.rs`、`crates/lsp/src/actions.rs` |
| M（criterion） | `crates/front/Cargo.toml`、`crates/front/benches/**` |
| N（fuzz） | `fuzz/**`（新） |
| 主会话（合并期） | `docs/architecture.md` §6 记账、`STATUS.md`、`docs/TESTING.md`、`docs/notes/gap-analysis.md` |

冲突警戒：K 与 L1 都不碰 `suggest.rs`/`error.rs` 交叉面（K 只改 error.rs 的
分类器，L1 只消费既有 SuggestionKind 扩展——suggest.rs 归 L1 独占）。
