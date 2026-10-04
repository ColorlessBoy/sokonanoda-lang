# 内核线交接单（换会话用）

> 覆盖式重写于 2026-10-04 13:33 ✓（值守换会话单 ✓）。**只写当前状态** ✗（历史看 `git log` ✓）。
> 分支 **main**（本地 ✓，**未推送** —— 推送由值守独占 ✓）。（`docs-expiry.json` 登记 2026-11-15 ✓）

## 1. HEAD 与未推提交

* **结束 HEAD** = **`fac807d1`** ✓（记忆表改 `u64` 文本哈希键 + CAP 4096 ✓）。
* **未推 145 笔** ✓（本会话未 push / 未打 tag ✓）；**工作树干净** ✓。
* 判据现状 ✓：LSP `lsp_keystroke_structure` **3 条全绿** ✓ · `gap.py check` 不一致 0 ✓ ·
  front 全量上次收口 **830/0** ✓。

## 2. 手上的 WIP（**没有未提交的** ✓，但有一处「在树里、默认关」）

* **增量身份** ✓：`crates/front/src/compile/check/walk.rs`（累加器 + 预置 ✓）·
  `crates/front/src/judge.rs`（`seed_canonical_prefix` + 哈希表 ✓）·
  `crates/front/src/compile/check/mod.rs`（`canonical_prefix_id_checked` ✓）。
  **默认关** ✗（`SOKO_PREFIX_SEED=1` 才开 ✓）—— 开了会让判据 ① 从 `prefix=0` 变 **5** ✗，
  **机制未解释** ✗（两次修因都被实验否掉 ✓，见 §4）。**正确性优先** ✓：宁可变慢 ✗，不拿错键命中 ✗。
* 自检探针也在树里 ✓：`SOKO_PREFIX_ID_CHECK=1`（逐命令比 `seeded` vs `canonical_prefix_id(prefix_src)` ✓，
  **必须配 `SOKO_LSP_TEST_STDERR=1`** ✗ —— 它打在**服务端子进程** stderr 上 ✓）。

## 3. 下一棒做什么（值守 13:29 已排序 ✓）

1. **12 单元删尺寸闸 + 身份增量构造**（用户点名、最挡路 ✓）。闸**已删** ✓；身份**已证等价** ✓；
   **卡在「5 趟」** ✗ ⇒ 起点 = 开着预置**直接打那 5 趟的 `term` / `binders` / `key`** ✓（别再猜机制 ✗）。
2. **闸计数普查**（甲类 3 + 乙类 4 ⇒ 已登记 **G-88 / G-89 / G-90 / G-91** ✓）。
3. **可配置化 + Lean 对齐**（13:21 + 13:24 **合并成一件** ✓）：① 可配置化（**默认值一个不动** ✓、
   零行为变化 ✓）② 放宽默认值**单独一笔** ✓。**终点取 Lean 数值** ✓：`maxRecDepth 64→3200` ·
   `maxHeartbeats 4096→20000` · `maxSize 64→128` · `maxSynthDepth 8→32` ✓；
   **Lean 没有的**（`PROBE_CAP` ⇒ 判不等 ✗ · `PARSE_LIMIT` ⇒ 退回原文 ✗）**去掉，不许换数字留着** ✗。
   落点用**现成管道** ✓（`CompileOptions` + `sokonanoda.toml [limits]` + CLI `--max-depth=N` ✓），
   **不新造配置系统** ✗。
4. **提速纪律**（13:29 ✓）：中间验证**只认** `scripts/dev-verify.sh`（+ `--granularity` ✓，**0.3 秒** ✓）；
   `cargo test -p sokonanoda-front --lib` 全量与 `target/release/sokonanoda build courses/set-theory`
   （**单进程吃 9 核 · ~4 分钟**）**只在收口各跑一次** ✓；全量任务**一次一个** ✗。

## 4. 每条要带的判据（判红 / 判绿 ✓，以及「现在有没有」）

| 项 | 判据 | 现状 |
|---|---|---|
| 12 单元特性 | `cargo test -p sokonanoda-lsp --test lsp_keystroke_structure` ⇒ 改陈述 `prefix>0` / 改证明体 `prefix==0` | **有 ✓ 全绿** |
| 大前缀不许退回原文 | `judge::tests::a_large_prefix_must_not_fall_back_to_raw_text` | **有 ✓ 绿**（闸在时红过 ✓） |
| ② 不许拿慢换对 | `cargo test -p sokonanoda-lsp --lib perf_course` ⇒ unit12 didOpen **≤ 8.5s** | **有 ✗ 现红**（**14682ms** ✗） |
| 增量身份等价 | `SOKO_PREFIX_SEED=1 SOKO_PREFIX_ID_CHECK=1 SOKO_LSP_TEST_STDERR=1` ⇒ **零分歧** | **有 ✓ 已证等价** |
| 预置的副作用 | 同上 + `prefix` 必须 **0** | **有 ✗ 现红**（`seed=on ⇒ 5` ✗） |
| 可配置化零行为 | 整本课程 `build --json` **剔心跳行逐字节相同**（**改前 vs 改后都要跑** ✓） | **无 ⇒ 先建** |
| 预算耗尽有信号 | `[limits] max_depth=2` ⇒ **必须报错**；`=10000` ⇒ 长证明过；`--json` 见「预算耗尽」事件 | **无 ⇒ 先建** |
| 闸类守卫 | 凡「耗尽 ⇒ 判否」的路径判红 | **无 ⇒ 先建**（甲类 #1/#2/#3 一起 ✓） |
| 开发内环 | `scripts/dev-verify.sh [--granularity]` ⇒ 冷跑 ≤ 2s | **有 ✓**（0.10s ✓ / `marginal=2.20` ✗） |

## 5. 已知的坑（都带实测代价 ✓）

* **`clear()` 型记忆表** ✗：walker 逐命令种 ⇒ 满则整表清空会把**刚种的那条**一起冲掉 ✓。
  已改「插入前淘汰一条」+ **哈希键 + CAP 4096** ✓（仍没治好那 5 趟 ✗，但机制本身是对的 ✓）。
* **子进程 stderr** ✗：服务端探针（`PREFIX_ID_MISMATCH` 等）**看不见** ✗ ⇒ 必须
  `SOKO_LSP_TEST_STDERR=1` ✓（漏了它我误判过一轮 ✓：「没报分歧」是**假象** ✗）。
* **进程级计数器** ✗：`judge::stats` 的计数**全进程共享** ✓ ⇒ 判据要取**差量** ✓；
  课程级断言要放**独立进程**（CLI 侧 ✓）。
* **release 产物会过期** ✗：改源码后只 `cargo test` 只编 debug ✓ ⇒ 量 release LSP 前必须
  `cargo build --release -p sokonanoda-cli -p sokonanoda-lsp` ✓（踩过两次 ✓）。
* **`AGENTS.md` 行数上限 435** ✓（`docs-lint` ⑦ 判红 ✓，而 `pre-commit` **不跑** docs-lint ✗）；
  **上限只许收紧** ✗ ⇒ 加内容要**折进既有行** ✓。
* **bash 3.2**：变量后紧跟**多字节字符**必须写 `${var}` ✗（否则 `unbound variable` ✓）；
  `sed 's/.*passes=/'` **贪婪** ⇒ 会吃到 `doc_passes=` ✓（「`0` 有两种来源」✗）。
* **python heredoc 里别用 ASCII 引号** ✗（我因此写坏过 3 次 ✓）—— 中文串里一律用 `「」` ✓。
* **残留进程** ✗：长跑脚本要自己 `pgrep -f <名字>` 收 ✓（值守清掉过 3 个跑了 8 天的 ✓，
  那不是本会话起的 ✓；本会话收尾自查为空 ✓）。

## 6. 本会话已兑现（可复核 ✓）

* **「只改证明，后面不需要重编」** ✓（用户点名的特性 ✓）：判定缓存键**收敛成唯一一把** ✓ +
  键里放**环境身份** ✓ ⇒ 改证明体 `prefix=0` ✓✓（等长 / 不等长都对 ✓）；**全课程逐字节对拍通过** ✓。
* **G-86 记法粘连** ✓（`lib/Prod` 2 / `lib/Equiv` 16 / `lib/Demo` 26 条诊断 ⇒ **0** ✓；
  全课程 243/6 compiled/failed ✓，**0** 个原先绿的模块变红 ✓）。
* **尺寸闸已删** ✓（用户 13:08 ✓）+ **闸类计数出口** ✓（判据 ③ **先红后绿** ✓）。
* **开发验证内环** ✓：`scripts/dev-verify.sh`（**0.3 秒** ✓，含 `--granularity` ✓）。
* **记账** ✓：老 G-86 并入 **G-30** ✓ · **G-87** ✓ · 闸类 **G-88…G-91** ✓ · Lean 对齐终点落账 ✓。
