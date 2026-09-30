# E19 基线（刀 0）：甲案（元变量）开工前冻结的读数

> **一句话**：E19 甲案（给记法求解器加元变量）**开工前**把"今天判成什么样"钉成数字 ——
> 每条都带**可复跑的命令**；刀1/刀2 只许在**红线**（§1）上逐字节不变，课程计数（§2）
> 会**有意**变化（接受面变宽），变化**逐条记账** ✓。
> 评估与三刀计划 ⇒ `docs/design/e19-evaluation.md` §4；缺口本体 ⇒ 台账 **G-48**。

## 0. 冻结对象

| 项 | 值 |
|---|---|
| 冻结日期 | 2026-09-30 |
| `git rev-parse HEAD` | `2c20be09e6dbb9ef339f5c2457550634dedde05c` |
| 二进制 | `target/release/sokonanoda`（`cargo build --release -p sokonanoda-cli --bin sokonanoda`，**1m32s**） |
| 二进制 sha256 | `7ae7782e7abc86fc771e12cb2a0ad6642dd5023aa9765ea6e57f8474b74bb963` |
| 副本 | `/tmp/e19/sokonanoda-baseline`（**易失** ⇒ 丢了就按上两行重建：`git worktree add` 到该 commit + 上面那条 build ✓） |

## 1. 红线（刀1/刀2 **必须**逐字节不变）

| # | 判据 | 命令 | 基线读数 |
|---|---|---|---|
| **R1** | 非课程语料逐字节对拍 | `bash scripts/kernel-diff.sh --non-course <前> <后>` | **172 组 0 差异**（86 文件 × 2 op：`grade --json` + `query check`）· **19.27s** |
| **R2** | 非课程 `--json` 摘要 | `bash scripts/kernel-diff.sh --digest <bin>` | `43581e0650e217f69a81f6dbe928f4c82b583ba144c7eb344649ecadf68ae5e5` |
| **R3** | 课程 `--json` 摘要 | 同上 | `d0375577b5999b5b60ceb8606e16c5945115d35e02e47ed33e751eaee064454d` |
| **R4** | 全语料 `--json` 摘要 | 同上 | `0231dcc44fbc795882287d9872083c24e1fd12c1bdf69bdd99fc399e806ff68e` |

* **口径**：`--non-course` = `course/` + `examples/` + `docs/gaps/repro/`（**86 文件**）。
  ⚠ **画布 `playground.sokonanoda` 不算非课程**（它是"当前练习"、随教学变 ⇒ 不进冻结判据；
  算进去就是 87×2=174，与台账里写的 172 对不上 ✗）—— 它只进 R4 的全量口径（129 文件）。
* **摘要**= 逐文件 `grade --json` + `query check` 的 stdout（含 stderr 与退出码）拼起来取
  sha256，**心跳（`build.tick`/`build.progress`）不进口径** ✓（`grade --json` 今天本就不发它们）。
* 刀1 的**开关关**态：R1–R4 **四条全等**；**开**态：R1/R2 **仍必须全等**（非课程一个字不许动），
  **R3/R4 允许变**（接受面变宽就是甲案的目的 ✓）⇒ 变了几条要**逐条记账**。

## 2. 三层判据（真相 / 契约 / 课程）

| 层 | 命令 | 基线读数 | 耗时（同机） |
|---|---|---|---|
| 真相（front） | `cargo test -p sokonanoda-front` | **769 passed / 0 failed**（lib）· 全部 15 个 target 合计 **802/0**（2 ignored） | 执行段 **1.53s**（lib）· 墙钟 72.18s（**含编译**） |
| 契约（notation） | `cargo test -p sokonanoda-cli --test notation` | **49 passed / 0 failed** | 执行段 **6.46s** · 墙钟 11.44s（含编译） |
| 课程门禁 | `SOKONANODA_BIN=<bin> python3 courses/set-theory/tools/check.py --json` | **43 目标 · 377 checked · 99 open · 0 判负**（canvas_open 96 · lib_open 0 · solutions_open 0） | **69.70s** |

## 3. 目标缺口（G-48：本刀存在的理由）

| 复现件 | 今天 | 刀1 之后 |
|---|---|---|
| `docs/gaps/repro/G48-notation-nullary-sugar-operands.sokonanoda` | **exit 1**（`elab-notation-argument-unsolved`：`∅ ≈ {b}` 两侧都补不出论域） | **exit 0**（两条声明都 checked）⇒ `python3 scripts/gap.py check` 会当场要求把台账 G-48 改成 `fixed` ✓ |

同轮复核：`python3 scripts/gap.py check` ⇒ **全部与台账一致** ✓（G-48 今天 `open` + "仍有失败" ✓）。

## 4. 性能基线（**先计数后墙钟**；墙钟只兜数量级）

| 读数 | 基线 | 说明 |
|---|---|---|
| 全课程冷 `build` 墙钟 | **56.67s** | `SOKONANODA_BUILD_JOBS=1 SOKO_STAGE_STATS=1 <bin> build --json courses/set-theory`（先 `build --clean` 清缓存）· `compiled 42 / failed 0 / hit 0` ✓ |
| `JUDGE_PREFIX runs` / `bytes` | **905** / **47,866,553**（52,891 B/run） | 结构计数（噪声免疫）⇒ **退化先看这两个数** |
| `passes` / `by_calls` / `judge_ms` | **1362** / 21,551 / **23,826** | 同上（`pass_total_ms=114,109` · `by_total_ms=37,084`） |
| `JUDGE_INFER` | calls **87,507** · hits/misses **86,602/905** | 记法/隐式求解要问类型的次数 ⇒ 元变量那一刀的直接监视器 |
| front lib / notation / 门禁 / 172 组对拍 | 1.53s / 6.46s / 69.70s / 19.27s | 见 §2/§1 |

⚠ **与 `docs/ONBOARDING.md` §1 的 47.8s / `runs=791` 不同**：那组读数在第 518–519 轮
（T-N13 迁移 + `diag_helper`）**之前** ⇒ 语料变过。E19 的对照口径**只认本文件**（同 HEAD、同二进制）✓。
⚠ **不许用绝对毫秒当判据**（本机同序列波动可达数倍）——退化先用结构计数拦，墙钟只抓数量级 ✓。

## 5. 记法面守卫（刀1 会碰 `elab.rs` ⇒ 同轮复核）

| 守卫 | 基线 |
|---|---|
| `python3 scripts/notation-lint.py` | OK —— 91 文件零旧写法（**587 处显式豁免**，全带标记） |
| `python3 scripts/audit-notation-paths.py` | OK —— 无新增绕过（命中 50 处；**基线 46 条指纹是地板**） |

## 6. 刀1 预登记的开关与回退（写在这里，实现时照抄）

* 开关 `SOKO_NOTATION_METAVAR=1` 开、**默认关**（关 = 逐字节等于 §1 的 R1–R4 ✓）；
* 判据两条**同时**成立：G-48 复现件 **exit 0** ✓ **且** R1 **172 组 0 差异** ✓；
* 回退 = 删开关分支（或默认值改回关）⇒ 回到本文件的四条摘要 ✓。

## 7. 复跑（三条命令）

```bash
cargo build --release -p sokonanoda-cli --bin sokonanoda
bash scripts/kernel-diff.sh --digest ./target/release/sokonanoda
bash scripts/kernel-diff.sh --non-course ./target/release/sokonanoda ./target/release/sokonanoda
SOKONANODA_BIN=./target/release/sokonanoda python3 courses/set-theory/tools/check.py --json
```
