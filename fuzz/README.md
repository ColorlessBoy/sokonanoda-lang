# fuzz：parser/lexer 永不 panic

针对 `sokonanoda-front` 的 cargo-fuzz 基建（cargo-fuzz 标准布局，独立 crate）。
**parser/lexer 永不 panic 是硬要求**：`parse` 返回 Result（任意输入只能是
编译错误），`semantic_tokens` 对任意 UTF-8 输入不 panic。fuzz target
`parse_never_panics` 还会在 parse 成功时走一次完整 `check_document`
（含内核）——**若某种输入能让内核 panic，那是真 bug**。

本 crate 用 `[workspace]` 空表脱离根工作区：根 `Cargo.toml` 的 members
不列它，根工作区的构建/测试不应感知它。CI **不跑** fuzz（时长不可控，
libFuzzer 需要 nightly 工具链），**定期本地跑**。

## 用法

```bash
# 一次性安装（需要 rustup 与 nightly）
cargo install cargo-fuzz

# 在本目录（仓库根的 fuzz/）跑 60 秒
cd fuzz
cargo +nightly fuzz run parse_never_panics -- -max_total_time=60
```

- `-max_total_time=60`：限时 60 秒（可自定，日常建议几分钟到几十分钟）。
- 语料落在 `fuzz/corpus/`（运行时自动生成，**不入库**）。
- 抓到 crash 时，复现输入写在 `fuzz/artifacts/`（**不入库**）。
  复现方式：`cargo +nightly fuzz run parse_never_panics <artifacts 里的文件>`。
- 可选：抓到的 crash 输入手动拷进 `fuzz/corpus/` 留作回归语料（目录不入库，
  输入内容自担）。

## 处置流程：fuzz 抓到 panic = 真 bug

1. **保存复现输入**（`artifacts/` 下的 crash 文件），最小化输入
   （libFuzzer 自带 minimizer：`-minimize_crash=1`）；
2. **按 LESSONS 三层回归修复**：kernel 层 → front 层 → cli/lsp 层各落一条
   回归测试（fuzz 输入转成最小化测试用例），修 bug 之前先把三层测试写成红；
3. **kernel 改动守冻结规则**：若修的是内核，只动冷路径/防御性检查，热路径
   语义零改动，改动带三层回归测试一起落 commit；
4. 修完重跑 fuzz（含该 crash 输入入 corpus）确认不再复现。

## 版本控制边界

`fuzz/.gitignore` 排除 `target/`、`corpus/`、`artifacts/`——三者均不入库；
根工作区不受本目录影响（`cargo test --workspace --locked` 等命令照常全绿）。
