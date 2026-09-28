# rebuild 基线实测：**42 模块 ⇒ 174 次模块编译（4.14×）· 一次 rebuild 222.1s**（2026-09-28）

> 来源：用户 18:11 反馈「一个这么小的项目就这么卡，后面大项目完全吃不消」⇒ P1 只量化。
> 复盘（为什么一直没被发现、守卫、修法风险）：`docs/perf/recompile-waste-retrospective-2026-09-28.md`。
> 缺口：**G-68**（`docs/gaps/ledger.jsonl`）· 复现：`docs/gaps/repro/G68-per-entry-cache-key-recompiles-shared-deps.sh`。
> 量具与原始日志：`/tmp/p1-timed.py`、`/tmp/p1-closure.py`、`/tmp/p1-rebuild.log`、`/tmp/p1-release.log`
> （临时目录；结论已全部落进本文与 G-68，命令见 §5 可重跑）。

## 1. 先确认"rebuild"是哪个命令（不猜）

命令面板里能"重编译"的只有三条（`editor/vscode/package.json` 共 16 条命令，**没有 cargo**）：
`sokonanoda.build` / `sokonanoda.rebuild` / `sokonanoda.clean`。
用户说的 **rebuild = `Sokonanoda: Rebuild (清空编译缓存后重编译)`（`alt+shift+b`）**，
代码路径是**两个 CLI 子进程**：

1. `sokonanoda build --json --clean <模块根>` —— `editor/vscode/extension.js:2058-2064`
2. `sokonanoda build --json <模块根>` —— `editor/vscode/extension.js:2069`

`<模块根>` 取自服务端 `soko/project`、**永远是目录**（E22）⇒ CLI 递归编译该目录下**每一个**
`.sokonanoda`（含 `lib/` 与 `units/solutions/`）。实测口径 = 上面两条 argv。

## 2. 六项数字（靶子：`courses/set-theory`，42 模块 / 7451 行 / 0.5 MB，Darwin arm64）

CLI = 仓库构建 **0.78.0（debug）**（= 本仓 `scripts/soko` 实际解析到的那份）。

| # | 项 | 数字 |
|---|---|---|
| 1 | **总耗时** | **222.1 s** = clean **0.114 s** + build **221.97 s** |
| 2 | **分阶段** | 进程启动+全目录遍历 **5 ms**；逐入口编译 **lib 23.6 s / units 49.5 s / units/solutions 148.9 s**（13 个文件占 **67%**）；其中**内核判定 151.2 s = 墙钟 68%** |
| 3 | **编译几次** | **42 次入口编译**（`files=42 compiled=42 hit=0 failed=0`）+ **174 次模块编译**（Σ闭包）+ **4126 次 `run_pass`** + 266 次文档 pass |
| 4 | **重复编译同一文件** | **是，4.14×**：`lib.Logic` **42** 次 · `lib.Set` **37** 次 · `lib.Exists` **26** 次 · `lib.Fun` 10 次 |
| 5 | **增量缓存是否失效** | 普通 `build` **不失效**（紧接一次 **0.256 s / 41 hit**）；`rebuild` **按定义全失效**——`--clean` 删掉全部 **52** 条项目条目（global 0 · project 52）⇒ 下一条 build 必然 0 hit ⇒ 222 s |
| 6 | **是否全量重扫** | **是**（递归遍历整根 + 每个入口重读重解析整条闭包 = 174 次文件读），**但它不贵**：全扫+算 digest 的代价就是那条 **0.256 s** 热跑 ⇒ 瓶颈不在扫描 |

`SOKO_STAGE_STATS=1` 原始行（冷编）：

```
STAGE_STATS passes=4126 pass_total_ms=615269 by_calls=69085 by_total_ms=194664 \
  judge_ms=151245 hits=66683 misses=266 doc_passes=266 doc_ms=150880
```

**口径**（必须写明，否则数字对不上）：**174 / 42** = `build <目录>` 收集到的 **42 个文件**
各自闭包之和（含唯一一个无 `import` 的 `lib/Logic`，它的闭包是它自己），与 `build.summary`
的 `files=42` 对齐。若只数"含 import 的入口"则是 **173 / 41 入口**——两个数都对，**口径不同**。
闭包大小已用**内核自己的**缓存条目 `project.modules` 对拍：unit05-solution=4 ✓ ·
unit04-solution=3 ✓ · `lib/Demo`=9 ✓。

## 3. 横向对照：**不是"没编 release"**（同一靶子，三条命令各测一遍）

| CLI | clean | 冷 build | 热 build |
|---|---|---|---|
| 0.78.0 **debug**（仓库构建） | 0.114 s | **221.97 s** | 0.256 s |
| 0.77.1 **release**（`target/release`） | 0.085 s | **213.35 s** | 0.248 s |
| 0.76.0 **release**（用户已装 VSIX 自带那份） | 0.082 s | **213.77 s**（41 compiled · 1 failed） | 4.61 s ⚠ |

⇒ **profile 只差 4%**（222 vs 213）⇒ 慢是**算法性**的，不是优化等级。
⚠ 0.76.0 那轮有 1 个 failed（版本不匹配，`requires = "0.78.0"`）⇒ 它的热跑 **4.61 s**：
**非干净条目永不入缓存** ⇒ 它那条闭包每次 build 都重编（次级观察，非本靶子）。

**跨版本（10 个版本一个字没变）**——同一夹具、同一量具（`by_calls` = 共享依赖被 elaborate 的次数）：

| CLI | 0.68.0 | 0.72.0 | 0.73.0 | 0.76.0 | 0.77.1 | 0.78.0 |
|---|---|---|---|---|---|---|
| 3 入口共享 1 依赖 ⇒ `by_calls` | 3 | 3 | 3 | 3 | 3 | 3 |

⇒ **不是回归**：重复功从 0.68.0（更早只有静态计数：`v0.60.0` 课程诞生当天 = **4.11×**）
到 HEAD 一直是同一个常数。

## 4. 时间花在哪：**闭包大小不预测耗时**

对 42 个入口做 `cost ≈ a + b × 闭包大小` 拟合：**r = 0.52、截距为负**（闭包 5 的
`unit06-solution` 花 **16.8 s**，闭包 8 的 `unit12` 只花 **12.9 s**）⇒ 时间是**入口自身**的
elaborate + 判定（`judge_ms` = 墙钟 **68%**），不是"重编依赖"。这条直接决定修法预期：
**4.14× 是工作量上界，不是耗时的收益预期**（详见复盘 §4.4）。

## 5. 复现（一条条可粘贴）

```bash
BIN=./target/debug/sokonanoda ; ROOT=$PWD/courses/set-theory
$BIN build --json --clean "$ROOT"                    # rebuild 第 1 个子进程（0.114s，removed 52）
SOKO_STAGE_STATS=1 $BIN build --json "$ROOT"         # 第 2 个（221.97s；逐行时间戳见 /tmp/p1-timed.py）
$BIN build --json "$ROOT"                            # 热跑对照（0.256s / 41 hit）
python3 /tmp/p1-closure.py "$ROOT"                   # 闭包/重复度（静态，秒级）
python3 scripts/check-recompile-factor.py            # 守卫：测"次数"（G-68 的上界在 scripts/recompile-budget.json）
bash docs/gaps/repro/G68-per-entry-cache-key-recompiles-shared-deps.sh   # 缺口复现（今天 exit 0 = 缺口仍在）
```

## 6. 诚实边界

1. **量测环境**：本会话沙箱禁止工作区外写入 ⇒ **全局单文件缓存静默失效**（`store_in` 是
   best-effort）。只影响唯一无 `import` 的 `lib/Logic`（每次重编 44 ms）；项目产物缓存在
   仓库内、照常工作 ⇒ 上面六个数字不受影响（热跑 41/42 hit 就是证据）。
2. **靶子选择**：用户原话没有点名项目；本表用 `courses/set-theory`（仓库里唯一的真项目、
   42 模块）。单入口的量级供参考：`units/unit05` 冷编 **3.77 s**（闭包 4 个模块）。
3. **耗时口径**：全部是**墙钟**（含进程启动）；`pass_total_ms` 是**趟内累计**（可 > 墙钟，
   因为一趟里多个单元各自计时）。
4. 0.76.0 那轮的 1 个 failed 与 4.61 s 热跑是**版本不匹配**造成的，不是本缺口的证据。

## 7. 收益估算：**"每个模块只编一次"能省多少墙钟**（2026-09-28 实测，切片 1 的输入）

量具 `scripts/measure-rebuild.sh`（**已入库**，口径：独立进程 + 冷缓存 + A/B 同口径 + 每组 2 次）：
A = 以该入口编**整条闭包**（今天 `build <file>`）；B = 把闭包里的 `lib/*` 拷进临时项目、
入口只留"import + 一条平凡定理"⇒ 编的就是**纯依赖**。B/A = 复用它**能砍掉**的墙钟份额。

| 入口 | 闭包 | A 整条闭包（冷, s） | B 只编依赖（冷, s） | **依赖占比** |
|---|---|---|---|---|
| `unit08-solution`（重入口） | 5 | 54.35 / 48.35 | 5.08 / 5.06 | **≈10%** |
| `unit12-solution` | 8 | 32.22 / 32.23 | 8.09 / 8.06 | **≈25%** |
| `unit05-pairs-products`（轻入口） | 4 | 7.06 / 7.06 | 3.05 / 4.06 | **≈50%** |

**结论（和"4.14× ⇒ 快 4 倍"不是一回事，必须写清）**：
* **计数**上确实能到 **174 → 42（4.14× → 1×）**；
* **墙钟**上只能砍掉"依赖那一段"，而它**随入口变重而变小**（10% → 50%）；
  课程总墙钟由**重的解答文件**主导（`units/solutions` 13 个文件占 222s 里的 **148.9s**）
  ⇒ 整轮的真实预期收益落在**低端**（≈10–25%），**不是 4×**。
* ⇒ 切片 1 的价值有两块：① 直接省掉依赖段（10–25%）；② 它是**刀 2（磁盘产物 / P3 可下载）的前提**
  —— 没有模块级产物，跨进程复用与远程缓存都无从谈起。

⚠ 口径陷阱（量具头里也写着）：整轮冷编里**后面的入口会因进程内 `judge_infer` 缓存（按前缀命中）
而变便宜** ⇒ 单文件独立进程的 A 与其在整轮里的那一段**不可比**；上表一律独立进程 + 冷缓存。
