# 课程门禁接进 CI / gate —— 设计（卷 I《集合论》）

> 状态：**as-built（2026-09-19）——S1/S2/S3 已落地，S0 未做（见 §9）**。
> 本文 §1–§8 是 2026-09-18 的设计原文（保留，便于对照取舍）；落地记录、验证与
> 与设计的偏差见 **§9**。设计当时的范围声明是"**只写设计**，本文件不修改
> `.github/workflows/ci.yml`、`scripts/soko`、`courses/set-theory/tools/check.py`"——
> 这三处现已按 §7 的 S1–S3 改过。
> 上游：`docs/design/teaching-project.md` §P-C6、`courses/set-theory/AGENTS.md`（判卷三纪律）、
> 台账 `docs/gaps/ledger.jsonl` 的 G-10/G-12/G-15（**as-built 2026-09-18/0.59.0：三条都已修**：
> G-10 = WO-003（`query check` 带 parse 诊断 + exit 1）；G-12 = WO-002（模块根绝对化）；
> G-15 = WO-010（诊断坐标自带行列，量具缺陷已修）——§4.3 里"span 不可信"的措辞随之作废，
> 门禁**做法**（只认退出码 + `--bisect`）不变）。

## 0 结论（TL;DR）

| 层 | 结论 | 一句话理由 |
|---|---|---|
| 判据的**唯一真相** | `courses/set-theory/tools/check.py`（python3，零 cargo） | 课程线的判卷路径本来就是 `scripts/soko grade`（node + 二进制）；硬规则 6 要求作者/agent 路径零 cargo，判据不能长在 Rust 里 |
| CI | **先加在 `test` job 里当新 step**（不是新 job） | 那一步的二进制是**当轮 cargo 刚编出来的**（版本天然一致、零下载）；代价 +10～40s；`auto-tag` 的 `needs` 已经包含 `test`，发布自动被挡 |
| 本地 | 阶段 3 挂到 `scripts/soko` 的 `gate`（**启动器层**，不动 Rust） | 「本地 = CI」纪律；不动 CLI 就不用 bump 版本/等发版；缺 python3 时 exit 3（无法判定≠绿） |
| Rust 集成测试 | **不做**（`crates/cli/tests/course.rs` 的 golden 只继续服务旧 `course/`） | 判据双实现必然漂移；课程要能按 §3.3 抽成独立仓，判据必须跟着课程走 |
| 升级触发器 | CI 里该 step 实测 > 90s，或课程抽出成独立仓 | 前者升级为独立 job（并行），后者整包搬走（`new-course-repo.sh` 的 CI 模板就是终点形态） |

## 1 现状与实测证据

| 事实 | 证据（本轮实测，macOS arm64，`target/` 已构建） |
|---|---|
| 判卷规模 | 34 个目标 = 8 个 lib 模块 + Demo（重复计一次）+ 12 单元 + 12 解答 |
| 判卷结果 | `python3 courses/set-theory/tools/check.py` → **296 checked · 93 open · 0 判负**，exit 0 |
| 单次总耗时 | 同一条命令 **real 10.3s**（34 次 grade ⇒ 平均 ~0.30s/目标） |
| 单目标耗时 | `node scripts/soko grade <unit12 绝对路径>` 3 次 1.72s（~0.57s/次）；直接跑 release 二进制 3 次 0.73s（~0.24s/次）⇒ **差的一半是启动器每轮重解析**（`node -e ''` 仅 0.036s，大头是 `--version` 探针 + 项目编译） |
| CI 参照（run 35337443573） | `lint` 23s · **`test` 4m46s** · e2e 各腿 1m50s～3m13s · `auto-tag` 5s |
| 前置缺口 | `git ls-files courses/set-theory` = **0** ⇒ `courses/` 目前**未入库**（`git status` 是 `?? courses/`）；门禁接 CI 前必须先落库 |

判据今天**不完整**、且这一点是设计动因：`check.py` 只看退出码，而 `grade` 对「合法 open」返回 0
（实测：单元①画布 exit 0 且 6 个 `exercise.open`）——**一份原样保留 `sorry` 的"解答"今天能过门禁**。

## 2 放哪一层：三方案的成本 / 收益 / 失败模式

### 2.1 方案 A：塞进 `scripts/soko gate`（= CLI 的 `gate()`，`crates/cli/src/env/mod.rs:244`）

- **成本**：改 Rust ⇒ 要 bump 版本 + 发版，`scripts/soko gate` 的版本护栏（二进制版本 ≠ 仓库
  版本即 exit 3）才会放行新逻辑；gate 从「只要 cargo」变成「cargo + python3 + node」，
  `scripts/soko` 是**跨平台**零依赖启动器，而 Windows 上通常没有 `python3`（要探 `py -3`/`python`）。
- **收益**：一条命令覆盖全部；本地跑绿 ≈ CI 绿的纪律最直白。
- **失败模式**：① 无 python 的机器上 gate 从「绿」变「跑不动」，若实现成静默跳过就是**假绿**；
  ② 语言线贡献者只改 Rust 也要等课程判卷；③ gate 用**缓存里的旧二进制**跑课程判卷会给出过期结论
  （AGENTS.md 点名的"历史上最常见的故障源"）。

### 2.2 方案 B：CI 独立 job（`course:`）

- **成本**：必须自己搞到二进制。`scripts/soko setup`（下载 release）在版本 bump 的 main push / PR 上
  **必然 404**——CI 在 `auto-tag` 之前跑，tag 还不存在（G-11 同族的版本源问题）；改成 job 内
  `cargo build -p sokonanoda-cli` 则新增一份 Rust 构建（安装工具链 + rust-cache，参考 `test` job 的
  4m46s）；用 artifact 从 `test` job 传二进制则踩 CI-FAILURES 里记过的 artifact 布局坑。
- **收益**：失败归因干净（课程内容 vs 语言回归分开）；可与 `test` 并行；课程抽出成独立仓时**整包搬走**。
- **失败模式**：① **忘记把新 job 加进 `auto-tag` 的 `needs`** ⇒ 课程红了照样发版（假绿发布）；
  ② 下载路径把 CI 绑到网络与已发布产物（PR 上不可判）；③ 多一个 job 就多一份"轮子"要维护。

### 2.3 方案 C：Rust 集成测试（照 `crates/cli/tests/course.rs`）

- **成本**：把 `check.py` 的发现逻辑（`course.json`、`lib/*.glob`、`solutions/*`）在 Rust 里重写一遍；
  课程每加一个单元就要动 Rust 测试文件（或目录扫描）。
- **收益**：`CARGO_BIN_EXE_sokonanoda` 直接给二进制 ⇒ 天然免疫启动器/下载/版本漂移；
  `Output.status.success()` 就是 G-10 安全的退出码；`concat!(env!("CARGO_MANIFEST_DIR"), …)` 天然绝对路径
  （G-12 安全）。**这是唯一一个"三条坑自动不成立"的方案**。
- **失败模式**：① **判据双份**（python 一份、Rust 一份）→ 迟早分叉，正撞仓库"文档与实现不分叉"的纪律；
  ② 课程作者/agent 要 cargo 才能自判，违反硬规则 6；③ 课程抽出成独立仓时这套测试原地失效；
  ④ `course.rs` 的 **golden 计数**正是本设计要避开的东西，放在同一目录里会诱导后来者照抄。

### 2.4 采纳：分层（B 的落点简化版 + A 的启动器挂载 + C 的零漂移思想）

1. **判据**留在 `check.py`（唯一真相，课程自己拥有，可随课程抽出）；
2. **CI**：在 `test` job 的 "Course layer is guarded" 之后加一个 step
   （`timeout-minutes: 5`，显式 `actions/setup-node@v5` 钉 node 22，
   `SOKONANODA_BIN="$PWD/target/debug/sokonanoda"`，`python3 "$GITHUB_WORKSPACE/courses/set-theory/tools/check.py"`）
   ——**不新建 job、不下载、不复用 artifact**，用当轮构建的二进制；
3. **本地**：阶段 3 在 `scripts/soko` 的 `gate` 分支里追加同一条命令（启动器是 node，天然跨平台；
   探不到 python3 ⇒ **exit 3 并说明**，绝不静默跳过）；
4. 因为 CI 是兜底，**回退本地挂载不会留下门禁空洞**（见 §7 的 S3 行回退列）。

## 3 判据：只判"0 判负 + 解答 0 open"，**不锁计数**

课程还在长（现在 93 道练习），任何 `checked == 296` / `len(targets) == 34` / golden 表都是负资产：
加一道题就得改门禁，门禁就从"质量闸"退化成"记账本"。判据固定为下面五条，**全部与规模无关**：

| # | 判据 | 为什么是它 | 防空转 |
|---|---|---|---|
| G1 | 每个目标 `grade` **退出码 0** | 唯一稳定契约（G-10：不认 `query check`） | — |
| G2 | 目标**存在**（`course.json` 条目 / lib / 解答文件都在） | 缺失今天只记 `missing`；必须算判负 | 防止"删文件当修好" |
| G3 | **解答**：`exercise.open == 0`（且 `decl.checked > 0`） | 退出码对合法 open 返回 0 ⇒ 只靠 G1 抓不到没填的洞 | 防止"整份删掉"（配合 G4） |
| G4 | **解答覆盖**：画布每个具名 `exercise.open.name` 都能在对应解答的 `decl.checked.name` 里找到 | 就是旧 `course.rs::solution_covers_every_canvas_exercise` 的无计数版 | 防止"删题代替填洞" |
| G5 | **lib + Demo**：`exercise.open == 0` | 库里有 `sorry` 会让所有引用它的单元判卷失真 | — |

- **计数只进报告与台账**：CI step summary 打印 `34 目标 · 296 checked · 93 open · 0 判负`，
  完整 `--json` 传 artifact；累积趋势追加进 **`docs/courses/ledger.jsonl`**（P6 已预定的路径，
  格式照 `docs/perf/ledger.jsonl`：`{"schema":"soko.course-ledger/1","version":…,"commit":…,"date":…,
  "targets":34,"checked":296,"open":93,"rejected":0,"solutions_open":0}` + 逐目标行）。
  趋势只出 `::warning`（例如 open 突然掉一半），**永不判红**。
- **`diagnostic` 计数**照旧进报告，但不作为判红条件（诊段家族会变，例如 `reserved-decl-warning`）；
  红只挂在退出码与 `open` 这两条稳定契约上。
- **禁止写法**（review 硬规则）：门禁代码里除 `== 0` / `> 0` 外，不得出现与计数比较的整数字面量；
  不得引入 golden 表或"N 个单元"清单长度断言。

## 4 三条坑的工程化（每条都配一个"自检"，让坑复发时门禁自己红）

### 4.1 G-10 —— `query check` 假绿：**只认 `grade` 退出码**

现场（本轮复验）：`query check --text 'infix:50 " e " => mem'` ⇒ `ok:true`、`counts` 全 0、**exit 0**；
同一文本 `grade` ⇒ `unexpected-token`、**exit 1**。G-10 在 `docs/gaps/ledger.jsonl` 仍是 `status:open`
（`wo_planned: WO-003`）。
**as-built（0.59.0，WO-003 已落地）**：`query check` 现在也报 parse 诊断（`failed[]`）并
exit 1，与 `grade` 同口径——本节"只认 `grade` 退出码"的判据**保持不变**（它是课程门禁的
契约，换判据要单独一轮），但 §4.1 的自检不再是换用 `query check` 的前提。
**自检（mutation）**：`check.py --selftest` 在临时目录里造一份**故意坏**的单元（合法前缀 + 一个
`exact bogus_name`），走**与真实目标完全相同的判卷函数**，要求它必须非 0；若它绿了 ⇒ 门禁自己
exit 1 并报"判据通道失效（G-10 类）"。这条自检是将来（WO-003 ≥0.59.0 之后）敢不敢换
`query check` 的唯一前提。

### 4.2 G-12 —— 相对路径把模块根打空：**一律绝对路径**

现场：相对路径入口 + 祖先清单 ⇒ 模块根退化成空路径 ⇒ 所有 `import lib.*` 报找不到。
**做法**：`check.py` 用 `Path(__file__).resolve().parent.parent` 定位课程，给 `grade` 的入口一律
`str(path.resolve())`；CI 里连脚本路径也写绝对（`"$GITHUB_WORKSPACE/courses/…"`）；
**不得**出现 `cd courses/set-theory && python3 tools/check.py` 这类依赖 cwd 的配方。
**自检**：`--selftest` 从**另一个 cwd**（临时目录）再判一个带 `import lib.*` 的目标，必须同样 exit 0；
相对路径回归时这条先红。

### 4.3 G-15 —— 内核错误的 span 不准：**失败输出不指行**

现场（`bash docs/gaps/repro/G15-query-check-bare-offsets.sh`，本轮复跑仍复现）：带 import 时坏声明在
3–4 行、诊断归因 3–6 行（**末端溢进下一个声明**）；无 import 时坏声明 8–9 行、诊断归因 6–9 行
（**起点提前**）。课程每个单元都 `import lib`，命中面 100%——**把 span 当权威就会让作者改错题**。
**做法**（失败信息的三段式）：

1. **权威段**：`标签 + 绝对路径 + exit 码`（这三样不可疑）；
2. **参考段**：诊断的 `stage/code/message` 原样给出，**显式标注"span 不可信，见 G-15"**，
   CI 注解只带 `file=`、**不带 `line=`**（`::error file=<相对路径>::…`）；
3. **定位段（根治）**：给出可复制的二分命令 `python3 courses/set-theory/tools/check.py --only <标签> --bisect`。
   `--bisect` 的语义：按顶层声明边界把文件切成前缀，逐次判卷，输出**最后一个仍为绿的前缀长度**与
   **第一个让 grade 变红的声明名/行号**——这个结论**不依赖诊断 span**，正是 G-15 台账里人工用的办法。

### 4.4 顺带堵一个头号故障源：**判卷二进制必须与仓库同版本**

AGENTS.md 点名「缓存过期是历史上最常见的故障源」，`gate` 也已为它留了 exit 3 的护栏。课程门禁同样要：
`check.py` 前置解析判卷命令——给了 `--bin`/`SOKONANODA_BIN` 时**自己比对**其 `--version` 与语言仓
`Cargo.toml` 的版本（不一致 ⇒ exit 2，说明"结果不可信"）；否则用 `scripts/soko version --json`，
并要求 `cli.source` **不是** `cache(STALE…)/cache(unknown…)`（`new-course-repo.sh` 的模板已这么写）。
解析不到任何可信二进制 ⇒ exit 2（前置缺失），**不判绿**。CI 里因为显式给了当轮构建的
`target/debug/sokonanoda`，这条前置恒定成立。

## 5 失败时的可读输出与 CI 界面

- **人读**（本地/日志）：保留今天的表，判负行后追加缩进块：
  `✗ 单元 unit05  exit=1  stage=kernel  code=…  message=…`（+ `span 不可信（G-15）` + `--bisect` 命令）。
  结尾仍是单行汇总 `34 个目标 —— 296 checked · 93 open · N 个被判负`。
- **注解**：CI 失败时对该 step 的日志逐条 `::error file=…::<标签> 判负（exit 1）…`（无行号），
  沿用 `test` job 已有 "Surface failing tests as annotations" 的口径（无 token 也能公开读）。
- **留档**：`$GITHUB_STEP_SUMMARY` 打表格（每目标 status/checked/open）；`--json` 全量报告上传
  artifact（名字如 `course-gate-report`，参照 `cargo-test-log` / `perf-report`）。
- **台账**：`--ledger` 追加 `docs/courses/ledger.jsonl`；阶段 1 只本地/手动跑，阶段 4 再考虑
  main-only 回提交（照 `e2e-ledger` 的 concurrency + rebase 重试 + 补 status 三件套，见 §7 S4）。

## 6 CI 时间预算

| 项 | 数值 |
|---|---|
| 实测（本地，暖） | **10.3s** / 34 目标；`SOKONANODA_BIN` 指向二进制后 10.2s（本轮同量级） |
| 外推（ubuntu-latest，冷二进制、无 target 复用） | **15～45s** |
| 建议上限 | step `timeout-minutes: 5`；**预算线 = 90s**（`test` job 现 4m46s，占比 < 1/3） |
| 扩展性 | 成本对目标数**线性**（~0.3s/目标本地）：34→10s；满编 ~120 目标 → ~35–40s 本地 / ~1.5min CI ⇒ 触到 90s 线 |
| 超线对策 | 先 `SOKONANODA_BIN` 直接给二进制去掉每轮探针；仍超线则升级为独立 job（并行，不进关键路径）或按项目闭包批量判卷（**前提**：§4.1 的假绿自检已绿） |

## 7 落地步骤（每步都能独立过当轮 gate）与回退

| # | 改动 | 验证（当轮必跑） | 回退 |
|---|---|---|---|
| **S0** | 把 `courses/set-theory/`（现为 `?? courses/`）**入库** | `git ls-files courses/set-theory \| wc -l` > 0；`python3 courses/set-theory/tools/check.py` exit 0 | `git revert` 该提交 |
| **S1** | `check.py` 加判据 G2–G5 + `--only/--bisect/--selftest/--ledger`（**只加判据，不加"关判据"的开关**） | `python3 courses/set-theory/tools/check.py` exit 0（计数不变 296/93/0）；`--selftest` exit 0；故意坏一个临时副本 ⇒ exit 1 | 逐个提交，`git revert` 单个即回到上一判据集 |
| **S2** | `ci.yml` 的 `test` job 加 step（§2.4 第 2 条；含 setup-node、注解、summary、artifact） | 推**临时预检分支**：一次故意坏课程文件 ⇒ 只有该 step 红、注解指向正确文件；还原后全绿（CI-FAILURES 2026-09-18 的预检纪律） | 删掉这一个 step（一个 hunk）即回到今天 |
| **S3** | `scripts/soko` 的 `gate` 分支挂载同一命令（探不到 python3 ⇒ exit 3 并打印装法；把解析到的 `SOKONANODA_BIN` 透传给子进程，避免 34 轮重解析） | `scripts/soko gate` 全绿；`PATH` 里去掉 python3 时 exit 3（不是 0）；`git ls-files -s scripts/soko` 模式位仍是 100755 | `git revert` 启动器提交；**CI step 仍在 ⇒ 门禁不出现空洞** |
| **S4**（可选） | main-only `course-ledger` job：把 `--json` 报告合并成一条 `docs/courses/ledger.jsonl` 提交 | 重跑该 job 幂等（无新条目不提交）；`auto-tag` 的 `needs` 含它 | 删 job；报告仍留在 step summary/artifact 里 |

**收尾义务**（每步同轮）：`STATUS.md` 记一轮；`REQUIREMENTS.md` §9 追加日期条目；
`courses/set-theory/README.md`/`AGENTS.md` 与 `skills/sokonanoda-dev`、`skills/sokonanoda-ci`
同步"一条命令"的口径；若首轮 CI 红，按纪律写 `docs/CI-FAILURES.md`。

## 8 明确不做 / 未决

- **不做**：给新课程加 golden 计数（与 §3 冲突）；把 `course/`（卷 0）的 `crates/cli/tests/course.rs`
  golden 改动或合并；为了跑门禁在 CI 里 `scripts/soko setup` 下载二进制（版本 bump 时必红）；
  把课程判据写进 Rust（§2.3）。
- **未决**：① `--bisect` 的声明边界用"行首关键字扫描"是否够（`namespace`/`section` 的嵌套边界）；
  ② 阶段 4 台账回提交是否值得它的复杂度（也可只在人工收尾时跑一次 `--ledger`）；
  ③ 卷 I 抽出成独立仓时，本文件 §2.4 的 CI 配方与 `scripts/new-course-repo.sh` 生成的
  `ci.yml`（`setup → check-course.py`）合并成一份的时机。

## 9 as-built（2026-09-19，S1–S3）

### 9.1 落地清单

| 文件 | 改动 |
|---|---|
| `courses/set-theory/tools/check.py` | 判据 **G1–G5**（§3 原文，全部与规模无关）+ `--selftest` / `--bisect` / `--only` / `--json` / `--report` / `--summary` / `--annotations` / `--ledger [路径]` / `--bin`；退出码 0/1/2 按 §3/§4.4 |
| `scripts/soko` | 新增 `case 'gate'`（§7 S3）：python3 探针在**最前面**（探不到 ⇒ exit 3 + 装法，绝不静默跳过）→ 原样跑 CLI 的 cargo 门禁 → 绿了再跑课程门禁，并把**解析到的**二进制经 `SOKONANODA_BIN` 透传；`refuseUntrusted()` 从 default 分支提取出来复用（行为逐字不变）；**第四步（主线收尾，0.59.0）= `python3 scripts/gap.py selftest` + `check`**（缺口台账契约，~3 s；课程被抽走时前三步跳过它仍跑） |
| `.github/workflows/ci.yml` | `test` job 里 `Course layer is guarded` 之后新增 `actions/setup-node@v5`（node 22）+ `Course gate (set-theory, G1–G5)`（`timeout-minutes: 5`，`SOKONANODA_BIN=${{ github.workspace }}/target/debug/sokonanoda`，先 `--selftest` 再 `--annotations --report /tmp/course-gate.json --summary "$GITHUB_STEP_SUMMARY"`）+ `Upload course gate report (always)`（artifact `course-gate-report`）。**不新建 job**（§2.4），`auto-tag` 的 `needs` 不动（`test` 本来就在里面 ⇒ 课程红就挡住发布）；同 job 再下一步 `Gap ledger is consistent (docs/gaps)`（`gap.py selftest` + `check`，`timeout-minutes: 3`，`SOKONANODA_BIN` 同上）——台账是契约，红了说明语言变了而台账没跟上 |

### 9.2 与设计的偏差 / 实现细节（reviewer 看这里）

1. **span 的措辞（G-15 勘误）**：§4.3 的"内核错误 span 漂移"现场已被台账 G-15 的勘误推翻
   ——那是复现脚本把**字节 offset 当字符下标**切片造成的假象，`grade --json` 的
   `span.start/end`（带 line/column）是**精确的**；G-15 今天剩下的是 `query check` 的
   `failed[]` 只给裸字节 offset。设计**做法不变**（门禁不以 span 为权威、注解不带 `line=`、
   定位给 `--bisect`），但输出措辞写成"**span 只作参考**（G-15）——定位一律用 `--bisect`"，
   不再断言 span 不可信。**0.59.0 收尾（WO-010）再进一步**：`query check` 的 `failed[]` 现在
   自带 1 基行列，G-15 关账 ⇒ 输出措辞改成「诊断 span 自带行列、可信；要看『第一个判红的
   声明』仍用 `--bisect`」。
2. **`--bisect` 的前缀文件写在目标同目录**：`grade` **不支持 `--root`**（`crates/cli/src/env/mod.rs`
   的 `env::grade` 里 `root: None` 是写死的），所以前缀不能放进临时目录（模块根会退化成
   临时目录、`import lib.*` 全断）。前缀落成目标同目录的 `.soko-bisect-<pid>-<n>.tmp`
   （**不带 `.sokonanoda` 后缀**，任何 glob 都不会捡到；`finally` 删除），模块根照旧由祖先
   清单发现——正是 G-12 的安全形状。
3. **`--bisect` 只对 `grade` 判红的文件适用**：G1 之外的判据（G3/G4/G5）在 `grade` 上是
   **退出码 0**，二分不出东西；这类判负直接点名声明/练习（`--bisect` 会打印"整份文件判绿，
   无需二分"）。判负块因此分两种尾巴：exit≠0 ⇒ 给 `--bisect` 命令；exit=0 ⇒ 说明二分不适用。
4. **解答与画布的配对按文件名里的 `unitNN`**（不是 `<画布名>-solution`）：缺失 ⇒ 该行
   `missing`（G2 判负）、重名 ⇒ 进 G2 的 `课程结构` 行；`course.json` 里没有对应画布的
   解答仍会被判卷（G1/G3），只是不做 G4。
5. **`--only` 先精确后子串**：门禁自己打印的 `--only "单元 5"` 必须只选中一个（纯子串会把
   单元 1/10/11/12 一网打尽）。
6. **本地 gate 的课程缺失分支**：`courses/set-theory/tools/check.py` 不在检出里时（课程抽成
   独立仓是 §3.3 的既定方向）打印一行提示并跳过——不是静默；**python3 缺失仍是 exit 3**。
7. **§4.4 的前置判卷命令**已实现：`--bin`/`SOKONANODA_BIN` 时比对二进制自报版本与
   `sokonanoda-version.txt`（优先）/`Cargo.toml`，不一致 ⇒ exit 2；走启动器时读
   `scripts/soko version --json`，`cli.source` 含 `STALE`/`unknown`/`unverified` ⇒ exit 2。
8. **计数不锁**：当轮实测 **34 个目标 · 315 checked · 96 open · 0 判负**（课程内容与
   另一条线在并行增长——正是"不锁计数"的理由）。`--ledger` 默认写
   `docs/courses/ledger.jsonl`（`soko.course-ledger/1`，字段照 §3），本轮没有跑它，
   所以仓库里没有这个文件。

### 9.3 当轮验证（命令 → 结果）

| 验证 | 结果 |
|---|---|
| `python3 courses/set-theory/tools/check.py --selftest` | exit 0（正控制 + 故意坏单元被拒 + 二分点名，0.7s） |
| `python3 courses/set-theory/tools/check.py` | exit 0（34 目标 · 315 checked · 96 open · 0 判负；本地 11–13s 冷 / 1.4s 温） |
| CI 形状（`--selftest` + `--annotations --report … --summary …`） | exit 0；report `soko.course.check/2`、summary 表格、注解（全绿时无） |
| 变异：解答塞 `sorry` / 画布多一道练习 / 删解答 / lib 塞 `sorry` / 解答混坏声明 | 分别 G3 / G4 / G2 / G5 / G1 判负，exit 1 |
| 变异文件的 `--only "<标签>" --bisect` | 点名 `theorem injected_bad`（第 126 行）与"最后全绿 = 前 8 个声明（到第 125 行）"——**不依赖诊断 span** |
| `--bin /tmp/nope`、`--bin` 版本不符（假二进制报 0.1.0） | 都是 exit 2（不判绿） |
| `scripts/soko gate`（cargo 用 stub 替身，避开慢路径） | 走到课程门禁 exit 0；把 `SOKONANODA_BIN` 换成"判负的假二进制" ⇒ gate exit 1（判负正确传播） |
| `PATH` 去掉 python3 后 `scripts/soko gate` | exit 3 + 装法提示（没有跑任何 cargo） |
| `python3 scripts/gap.py selftest`（14 条判据）/ `check`（24 条缺口） | selftest exit 0；check exit 0（全绿），~3 s |
| 变异：把某条 `fixed` 改回 `open`（不改复现件） | `gap.py check` exit 1 并点名该条（**契约真的在判**） |
| `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` / `bash -n`（step 的 run 块） | 都通过 |

### 9.4 未做 / 交给下一轮

- **S0 仍是前提（0.59.0 收尾轮复述）**：`git ls-files courses/set-theory` 到本收尾轮
  仍是 **0**（`courses/` 整个目录还是 `??` 未入库；`site/set-theory.html` 同样）。
  在干净检出上这个 CI step 会因找不到 `check.py` 而红——**落 commit 时必须 `git add
  courses/`（含 `tools/check.py`）与 `site/set-theory.html`、`scripts/gap.py`、
  `docs/gaps/`，否则这一版一发出去 CI 就先红在课程门禁 / 台账门禁上**。
- **S4**（main-only 台账回提交）未做；`--ledger` 已实现，只等人手动跑。
- ~~**skills 未同步**~~ ✅ **已同步（第一百〇六轮 / 0.59.0 收尾）**：
  `skills/sokonanoda-dev`（§3 门禁清单加 `check.py --selftest` 一行、§4 setup 写明
  `scripts/soko gate` 含课程门禁且 python3 探不到即 exit 3）与 `skills/sokonanoda-ci`
  （§0 同口径 + §1 陷阱表新增「课程门禁在 `test` job 的 step 里」一条）。
- §8 未决①②③ 不变（`--bisect` 边界只扫行首关键字；台账复杂度；与
  `scripts/new-course-repo.sh` 模板合并的时机）。

