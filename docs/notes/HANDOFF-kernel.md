# 内核线交接单（换会话用）

> 覆盖式重写于 2026-10-03 22:0x ✓（上一版是 **08:45** 的，已过时 ✗）。
> 分支 **main**（本地 ✓，**未推送** —— 推送由值守独占 ✓）。
> **本文件只写当前状态** ✗（历史看 `git log` ✓）。
> （`scripts/docs-expiry.json` 登记 2026-11-15 过期 ✓）

## 1. HEAD 与未推提交

- **本会话结束 HEAD** = `96c0ed29` ✓；**远端** `origin/main` = `b8a2a4d1` ✓（= 本批基点 ✓）。
- ⚠ **起始 HEAD 本会话内没有记录** ✗（旧交接单写于 08:45 ✓，其起点是 `e1591d01` ✓）。
- **未推 11 笔** ✓，其中 **我的 7 笔** ✓（另 4 笔是**课程线**的 ✓，别算错 ✗）：

| 笔 | 内容 |
|---|---|
| `96c0ed29` ✓ | **G-62/G-63 停手结论**（剩余那一半**归 B2** ✓） |
| `700b5f32` ✓ | G-62 说清卡在哪 ✓ + G-63 判定 = u1/b1 两条红本身 ✓ |
| `9834ab1a` ✓ | `diag(elab)`：外层站点探针（**未完成的探针** ✗，env 门控 ✓） |
| `ea1f4100` ✓ | `cargo fmt` 修 diag 的格式（消 CI `lint-fmt` 红 ✓） |
| `b8a2a4d1` ✓ | pre-push 逃生门**按项粒度**（跳过时仍跑 fmt + clippy ✓） |
| `1bbf4181` ✓ | `diag(elab)`：求解步三值打印（**未完成** ✗，门控 ✓） |
| 更早 6 笔 ✓ | L-08 保真度 ✓ · G-27 收口 ✓ · 心跳判据结构性化 ✓ · G-62 area 重写 ✓ · 求解器路线③（**防御性，无实测效果** ✗）· `gap.py` 支持 `repro_root` ✓ |

## 2. 手上的 WIP

**无** ✓ —— `git status --short` = **空** ✓（探针与试改**全部已撤或已如实提交** ✓）。
`crates/front/src/compile/{elab,implicit}.rs` 都**干净** ✓。

## 3. 下一棒做什么（值守已排序 ✓）

**进"编辑慢族"（用户反复报的痛点 ✓，5 个 blocker 里的 2 个 ✓）**：**G-31 / G-68** → G-29：

1. **判据先立、先让它红** ✓ —— `docs/design/module-artifacts.md` **§5** 的唯一验收口径是
   「**复用路径 vs 逐入口路径的报告/事件逐字节相同**」✓，而 `crates/cli/tests/imports.rs`
   里**没有这条** ✗（只有冷/热一致 ✓、单文件路径一致 ✓）。
   ⚠ **这就是"半天白干"的根因** ✓（18:30 收口 → 19:30 发现报告静默丢项 → 19:34 全撤 ✗）：
   **没有这条尺子，任何"修好了"都不可信** ✗。
2. **基线已测（当前包 ✓，不是台账里那个旧的 4.14× / 222.1s ✗）**：
   ```
   $ SOKONANODA_BIN=…/target/release/sokonanoda python3 scripts/check-recompile-factor.py
     Σ闭包 = 6 次模块编译，去重后 4 个模块 ⇒ 重复 **1.50×** ✓
     ✓ **by_calls = 3** ≤ 上界 3 ✓   ← 缺口形状：3 个入口共享 1 个依赖 ⇒ 该编 1 次，实编 3 次 ✗
   ```
   修好后实测应为 **1** ✓，并把 `scripts/recompile-budget.json` 的 `max_by_calls` 从 3 收紧到 1 ✓。
3. 结构路线已定 ✓（**无结构未知** ✗）：`module-artifacts.md` **§9** 的**纯前端**路线 ✓，
   接口 09-28 定 ✓，三个切片 09-29 已落 ✓（`27ebca0e` / `58c7b239` / `b3104132` ✓）
   ⇒ **只剩"用起来"** ✓。

## 4. 已知的坑（都带实测代价 ✓）

1. **语言类长尾（G-62 / G-63 / G-73 / G-61）的正解在架构改造里** ✗，**局部补丁会打断
   `lib/Exists` 这类文件** ✗ —— 实测：往 `implicit.rs` 的 route ① 塞"用操作数本身"的 guard
   ⇒ `lib/Exists` 立刻报 `kernel-expected-sort` ✗ ⇒ 只能撤回 ✓。
   台账已注明：G-62 剩余那一半**归 B2 切片**（元变量进项 + zonk + `syntheticOpaque` ✓），
   **B2 排在 K1 之后** ⇒ 本线不做 ✓。**别再试 guard、别再开探针** ✗。
2. **改 `.rs` 用 bash + 内联 python 字符串替换的代价** ✗：最近 60 轮里 **44 轮**这么干 ✓
   ⇒ **7 次编译失败 + 3 次整文件回退重来** ✗（含括号错 → 回退 → 类型错 → 回退 ✓）。
   ⇒ **一律用编辑工具** ✓；commit message 也**写文件 + `git commit -F`** ✓ —— 用 `-m "…"`
   会被 shell 展开反引号 ✗（本会话就中过一次 ✓，commit 直接失败 ✓）。
3. **`cargo fmt --all` 与真门禁结论相反** ✗：真门禁是 `scripts/ci-local.sh --fast` 里那三包
   （`cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check` ✓）
   —— **别被 `--all` 的假红带着跑** ✗（kernel 的 `rustfmt.toml` 需要 nightly ✓）。
4. **报绿必须带"逐 job 结论"** ✓：`gh run view --log` 在本仓**返回空** ✗（实测 3 次 ✓）
   ⇒ 用 `gh run download <run-id> --dir /tmp/x` ✓（`cargo-test-log/cargo-test.log` ✓ ·
   `e2e-*/latest.json` 的 `failing_cases` ✓）。**"不在失败列表" ≠ 绿** ✗
   （job 可能压根没跑 ✓ —— 我在 G-84 上就犯过这个错 ✓）。
5. **远端 main 现有 2 条红** ✓（`b8a2a4d1` 那轮 ✓）：`test (sokonanoda-cli, tests)`
   （G-84 ✗ 安装器族 **非内核线** ✓ · **u1/b1 ✗ 归 B2** ✓）· `e2e (ubuntu · VS Code 1.138.0)`
   39/1 ✗（用例 `edit responsiveness … A/B flicker with the show delay off vs on` ✓
   ⇒ **G-29/G-31 族** ✓，**非 flaky** ✗）。

## 5. 本会话已兑现（可复核 ✓）

`G-32` ✓ · `G-27` ✓ · `G-80` ✓ · `G-36` ✓ · `G-53` ✓ · `G-65` ✓ · `G-66` ✓ ·
`G-84`（**曾误报绿 ✗，实为红 ✓**）· `G-49` 部分 ✓ · `G-63` 隐式半 ✓ ·
台账三修（L-08 ✓ / G-27 ✓ / G-61 ✓）**CI 全绿** ✓ ·
`G-68` 全撤（**性能项让位给正确性红线** ✓，结论已留档 ✓）。
