# WO-007 sokonanoda course 只按单文件编译，不认 import（G-06）

> 台账：`docs/gaps/ledger.jsonl` 的 G-06（`kind: tooling`、`severity: blocker`、`status: open`、
> `wo_planned: WO-007`、`repro: docs/gaps/repro/G06-course-import.sh`）；
> `blocks`：卷 I 的 10 单元共享 `lib/` 的整个方案、课程进度聚合（ledger 原文）。
> 设计出处：`docs/design/teaching-project.md` §6.3（WO 模板）、§8 P1 最后一条（:380）、
> §5 DoD 第 6 条（:248「单元进课程清单后由课程测试跑」）。本 WO 是 P1 的最后一刀。

## 用户可见症状 / 最小复现

- 复现命令（一条，可直接粘贴；夹具与自断言脚本都已入库）：
  `bash docs/gaps/repro/G06-course-import.sh`
  退出码约定见 `docs/gaps/README.md:26-38`：**0 = 缺口仍在**（今天的正常态）· 1 = 行为变了
  （修好后的期望态）· 2 = 环境不满足。
- 今天的实际输出（0.58.0，本轮实跑，逐字摘关键行）：

  ```
  == ① 项目模式判卷（正确路径；同一文件）==
  {"data":{"counts":{"decl_checked":1,…,"exercise_open":0,…},"failed":[],…},"ok":true,"op":"check",…}
     → 1 checked / 0 failed（预期）：yes
  == ② 课程聚合（今天坏掉的路径）==
  {"checked":0,"failed":1,"file":"proj/U4.sokonanoda","title":"G-06 复现单元（import 共享库）",…,"unit":1}
  {"checked":0,"failed":1,"open":0,"type":"course.summary","units":1}
  → 结论：G-06 仍在（项目模式绿、course 聚合红）——与台账一致。（exit 0）
  ```

  同一个文件、同一份文本：`grade`/`query check` 绿，`course` 红。
- 真实课程现场（比夹具更狠；夹具只是最小化）。命令：
  `node scripts/soko course "$PWD/courses/set-theory/course.json" --json`

  ```
  {"checked":0,"failed":2,…,"file":"units/unit01-sets-membership.sokonanoda",…,"open":6,…}
  {"checked":0,"failed":5,…,"file":"units/unit05-pairs-products.sokonanoda",…,"open":7,…}
  {"checked":0,"failed":15,…,"file":"units/unit06-relations.sokonanoda",…,"open":8,…}
  {"checked":0,"failed":3,…,"file":"units/unit12-synthesis.sokonanoda",…,"open":8,…}
  {"checked":1,"failed":63,"open":93,"type":"course.summary","units":12}
  ```

  12 个单元**全红**（今天唯一的 `checked:1` 是 unit08 的偶然残留）。对照同一单元的
  项目模式：`node scripts/soko query check --file "$PWD/courses/set-theory/units/unit01-sets-membership.sokonanoda" --compact`
  → `{"counts":{"decl_checked":2,"exercise_open":6,…},"failed":[],…}`（绿）。
  即：**课程侧已经在用共享库，聚合器却看不见**——`courses/set-theory/units/*.sokonanoda`
  12 个单元全部有 `import`（实测 `grep -rn '^import' courses/set-theory/units/*.sokonanoda`）：
  unit01 `lib.Logic`+`lib.Set`（:9-10）、unit05 `lib.Logic`+`lib.Set`+`lib.Prod`（:25-27）、
  unit06 `lib.Logic`+`lib.Exists`+`lib.Set`+`lib.Rel`（:36-39）……
- 根因（读源码定位，非推测）：`course` 的唯一编译入口是
  `crates/cli/src/course.rs:114-133 count_unit`，它在 :118 调
  `compile_cached(&file, src, &CompileOptions::default())`；而
  `CompileOptions`（`crates/front/src/compile/prelude.rs:31-33`）**只有 `prelude` 一个字段**，
  没有路径/模块根 ⇒ 项目闭包从不加载 ⇒ 每个引用库名字的声明报 unknown identifier。
  另外两条通道都认 import：`grade`（`crates/cli/src/env/mod.rs:181-206` → `check::check_source`）
  在 `crates/cli/src/check.rs:72-106` 有 import 分支；`query` 在
  `crates/cli/src/query.rs:148-176`（`has_imports`）同样。**只有 course 掉队** ⇒ 同一文件
  同一份文本，`grade`/`query check` 说绿、`course` 说红（各单元全是未知标识符），
  课程进度条因此永远是"假红"。
- 为什么一直没被发现：`course/course.json`（第一门课）的 11 个画布**一个 import 都没有**
  （实测 `grep -rn '^import' course/*.sokonanoda` 无输出），所以两处 GOLDEN 一直是绿的；
  第一门课的"import 化"当年被设计显式排除（`docs/design/imports-and-projects.md:609`
  写着「`course` | **不动**（golden 表是硬门禁）」——本 WO 要**显式推翻**这一行，见"文档同步清单"）。

## 期望行为

- 官方 Lean 4 里这段是什么行为：`sokonanoda course` 本身**不是 Lean 语义，是工具链行为**
  （Lean 没有对应子命令；最近的类比是 Lake 脚本/`@[test_driver]`）。但它聚合的**单元**是
  Lean 文件：在 Lean 4 + Lake 下，`import Foo.Bar` 会按模块名去包根下取 `Foo/Bar.lean`
  并让被导入声明对下游可见，模块名 = 相对包根的路径（本仓库已取证的对照见
  `docs/design/imports-and-projects.md:99-120`）。今天 course 的行为等价于"把子目录里的
  文件当孤立脚本类型检查"——在 Lean 里这种形态根本不成立。
- 期望（把 ledger 的 `expected_lean` 落成可验收的三条）：
  1. **有 import 的单元走项目闭包**，与 `grade`/`query check` 用**同一份**闭包、同一个
     模块根、同一个 prelude 模式、同一个摘要键（`crates/cli/src/project_cache.rs:16-49`）；
  2. **计数只取入口模块的事件**（依赖模块的 `decl.checked`/`exercise.open` 不计入单元，
     否则 unit01 会凭空多出 lib 的十几条声明，课程数字失去可比性）；
  3. **`failed` 与 `grade` 的退出码同判**：`failed == 0 ⇔ grade <该单元> exit 0`
     （判据 = 入口无错 **且** 无项目级错误诊断，见 `crates/cli/src/check.rs:162-164`；
     数值 = 闭包内所有模块 `events.errors` 之和，因为项目级错误由
     `crates/front/src/project/report.rs:171-207 attach_diagnostics` 挂进某个模块，
     所以"求和"不丢诊断）。
- 模块根策略（本 WO 定义，必须写进 `docs/protocol.md`）：
  1. 入口目录向上找最近 `sokonanoda.toml`（与 CLI 其他命令完全一致，
     `crates/front/src/project/mod.rs:174-196`）；**单元自己的嵌套清单优先**——
     位于 `course/unit11-project/` 这类子项目里的单元不会被外层覆盖；
  2. 找不到清单时回退到 **`course.json` 所在目录**（"清单目录即默认项目根"，
     即 ledger `expected_lean` 的「最近 sokonanoda.toml / 清单目录」）。理由：
     course 布局天然是 `<课程根>/{course.json,lib/,units/}`，只认入口目录会让
     `import lib.Set` 去找 `<课程根>/units/lib/Set.sokonanoda`；
  3. 不做手工 `--root`（见"不做的事"）。
- 本教学子集的边界（明确不做）：不做 Lake 的包管理/依赖版本/precompiled `.olean`
  语义；不做跨单元共享环境（每单元一次闭包编译，与 `grade` 同形）；不把库模块
  算成"单元的一部分成绩"；不改变"progress is not an error"（有 failed 仍然 exit 0，
  `crates/cli/src/course.rs:5-6,109` 的契约与 `docs/protocol.md:577-579` 一字不改）。

## 范围

- **主改动就一个文件**：`crates/cli/src/course.rs`
  - `:20-46`（`course()` 的入口段：读清单 + 算 `base` + 单元循环头）：把清单路径先绝对化
    （`std::fs::canonicalize`，失败则退回原串；
    错误信息仍用用户给的 `manifest` 原串），`base` 与 `unit_path` 随之绝对化——这一条同时
    把 course 从 G-12 的坑里摘出来（见下"与 G-12 的边界"）；
  - `:114-133`（`count_unit`）：改成 `count_unit(path: &Path, src: &str)`，`parse` 之后按
    `file.commands.iter().any(|c| c.is_import())` 分流（与 `crates/cli/src/check.rs:73`、
    `crates/cli/src/build.rs:126` 同一判据）：
    - **无 import**：一行不改，继续 `compile_cached(&file, src, &options)`（单文件缓存键
      不变 ⇒ 与 `sokonanoda --json` 互相命中 ⇒ 今天的行为逐字节保留）；
    - **有 import**：`project_cache::plan(&entry, Some(src), root_override, &options)` →
      `project_cache::load`（命中直接复用）→ `front::project::compile_plan(plan, &options)` →
      `project_cache::store_if_clean`；计数读 `project.entry_module()` 的事件，
      `failed` 按上面第 3 条求和；
  - `:8` 的 import 行补 `PreludeMode` / `prelude_mode_from_source` / `project_cache`；
  - **顺带对齐（同一函数、同一处改动）**：`options` 从 `CompileOptions::default()` 改为
    `CompileOptions { prelude: prelude_mode_from_source(src) }`，与 `check.rs:65-70`、
    `query.rs:149-151`、`build.rs:109-111` 一致。今天 `courses/**` 与 `course/**` 里
    `-- sokonanoda:prelude` 出现 **0 次**（实测 grep），所以这是**潜在**分歧而非现存 bug；
    但它属于"course 与 grade 同判"这条不变式的一部分，且改动只在同一行。
- **参考实现（照抄形状，不照抄代码）**：`crates/cli/src/build.rs:103-160 build_one`
  的 import 分支（`plan_project` → `plan.digest` → `cache::load` → `compile_plan` →
  verdict `entry_module().events.errors.is_empty() && !project.has_errors()`），
  以及 `crates/cli/src/check.rs:86-105` 用 `project_cache` 的写法。
- **只读、预期不改的 front 代码**（改动会扩大面，别碰）：
  `crates/front/src/project/mod.rs:97-144`（`ProjectPlan::digest`，
  **摘要不含路径** ⇒ course 侧绝对化不会破坏跨命令缓存命中）、`:148-209 plan_project`、
  `:212-321 compile_plan`；`crates/front/src/project/report.rs:137-166,171-207`；
  `crates/front/src/project/manifest.rs:58-76 find_manifest`、`:112-121 module_root`；
  `crates/front/src/compile/prelude.rs:31-33,39-59`。
- **是否动内核：预期否**（`crates/kernel/**` 一行不碰；冻结快照，硬规则 1）。
- **与设计草图的偏差（要写进 as-built）**：`docs/design/teaching-project.md:380` 的草图写
  "`CompileOptions` 补模块根"。as-built 建议**不**改 `CompileOptions`：它是 `Copy` 值、
  直接进单文件缓存键（`crates/cli/src/check.rs:231-244`），把路径塞进去会污染单文件键；
  模块根本来就属于"计划"（`plan_project` 的 `root_override` 参数，`mod.rs:148-154`）。
  本 WO 采用 CLI 侧 `root_override`，并在设计文档里写明这次推翻。
- **与 G-12 的边界（防互相代做）**：course 侧的绝对化只是**调用方**自保
  （`canonicalize(manifest)`），**不碰** `find_manifest`/`module_root` 的通用修复
  ——那是 G-12 = WO-002 的地盘。验收硬条件：修完后
  `bash docs/gaps/repro/G12-relative-entry-ancestor-manifest.sh` 必须**仍然 exit 0**
  （否则等于偷偷代做了 WO-002，台账会互相打架）。反过来，若 WO-002 先落地，本改动仍应保留
  （幂等、防御性）。
- **兼容策略（课程资产零改动）**：
  - 可见行为变化的**只有**"有 import 的单元"：今天它们全部假红，修好后按真实闭包计数；
  - **一个字都不改**：`courses/set-theory/{course.json,sokonanoda.toml,lib/**,units/**}`、
    `course/**`。12 个单元今天已经写好了 `import`，修好后应直接全绿（基线见"验收"）；
  - **双 GOLDEN 不动**：`course/course.json` 的 11 个单元 0 import ⇒ 严格走
    `check.rs:72-73` 的单文件分支 ⇒ `crates/cli/tests/course.rs:86` 起与
    `crates/cli/tests/course_status.rs:68-77` 两张 GOLDEN 表**一字不改**
    （设计 `imports-and-projects.md:721` 称它们是 A1 的守门人）；
  - **与并行 WO 的边界**：G-02（构造子命名空间）会动
    `courses/set-theory/lib/Prod.sokonanoda:45 ctor prod_mk` 与 unit05 的四道
    `prod_mk_*` 题（该文件 :15-25 已写明"裸名降级为别名、`Prod.mk` 转正"的改名计划）；
    G-03/L-01…L-05 也会改 lib。**本 WO 的测试只依赖"入口计数与 grade 同判"，不写死 lib 的
    声明名与条数**，因此与那些 WO 可并行落地、互不阻塞；本 WO 需要同轮改动的文件清单只有：
    `crates/cli/src/course.rs` + 新增 `crates/cli/tests/course_project.rs` + 文档（见下）。
    库改名/换形的文件清单属于各自 WO。

## 不做的事（明确排除，防顺手扩大）

1. **不改课程与库内容**：`courses/**`、`course/**`、`course.json` 一律零改动（课程线资产）。
2. **不修 G-12**：`find_manifest`/`module_root` 的绝对化与"空 parent 视为 `.`"护栏属 WO-002。
3. **不给 `course` 加 `--root` / `--no-project`**：模块根由清单策略决定（`sokonanoda.toml`
   就是那个旋钮）。要显式根的人今天可以用 `grade --root`。
4. **不改协议形状**：`course.unit` / `course.summary` 的键集**不增不减**
   （`docs/protocol.md:570-575`），不提 `warnings`/`modules`/`root`/`deps`；依赖诊断的
   归属仍由 `grade`/`query project` 回答。
5. **不把依赖模块的事件计入单元计数**，也不把 `example.checked` 并进 `checked`
   （`crates/cli/src/course.rs:125-131` 今天把 `ExampleChecked` 归入忽略档：所以 unit08
   修好后仍是 `checked:15` 而不是 16——这是**有意保持**的口径，不在本 WO 扩大）。
6. **不重构 `check.rs`/`query.rs`/`build.rs` 的既有分支**。`has_imports` 今天有两份
   （`check.rs:73` 内联、`query.rs:175-176`），course 会成为第三份：想下沉到 front 请单独
   说明并保证三条通道行为不变，**不得顺手统一**。
7. **不做进度视图的增强**：不加"单元→模块"展开、不加"哪个依赖拖坏了单元"、
   不加 hit/miss 统计（这些是 `query project` 的活）。
8. **不扩 VS Code 扩展的课程发现**：`extension.js:131-134` 只找
   `<root>/course/course.json`，所以 `courses/set-theory` 本来就不在扩展的课程树里
   （它是另一条 backlog，不在本 WO）。
9. **不动内核**；不改 `prelude` 语义；不改单文件路径的任何输出。
10. **不把 `courses/set-theory` 的逐单元计数写进语言仓的硬 GOLDEN**（课程线每加一题都会
    变）；差分不变式用夹具钉（见"验收"）。
11. **不代做** `import-not-found` 的 hint 改进（设计 `imports-and-projects.md:405` 承诺
    "列出搜过的模块根"，`crates/front/src/compile/error.rs:275-277` 今天没列，台账里
    **没有**这一条）——请课程线另记一条缺口，本 WO 不夹带。

## 验收（三层）

- **front 单测**：本 WO **预期零 front 改动** ⇒ 该层用既有回归证明"语义未动"即可：
  `crates/front/tests/perf_project.rs`、`crates/front/src/project/tests.rs` 全绿。
  若实现者选择把 import 判据下沉成 front 的纯函数（上面"不做的事"第 6 条允许但非必需），
  则必须补两条用例并在交付说明里写清：`importless 源 ⇒ false`、含 `import` 的源 ⇒ `true`
  （负例：`import` 出现在声明之后 ⇒ parse 阶段被拒，`crates/front/src/parse` 的既有错误码）。
  **待确认**：这一层最终有没有新增用例，取决于实现者二选一；不允许"改了 front 却不加测"。
- **CLI e2e**（新增 `crates/cli/tests/course_project.rs`；沿用
  `crates/cli/tests/course_status.rs:19-45` 的每进程临时 `SOKONANODA_CACHE_DIR` 与
  "spawn 真实二进制 + 解析 JSON Lines"手法）：
  1. **夹具转绿**：`docs/gaps/repro/G06-course-import/course.json` ⇒ `course.unit` 为
     `failed:0 / checked:1 / open:0`，`course.summary` 为 `failed:0`
     （今天：`checked:0 / open:0 / failed:1`）；
  2. **差分不变式（本 WO 的核心判据）**：对夹具与下面 3 的负例，
     `course.unit.failed == 0 ⇔ grade <该单元绝对路径> exit 0`；同时
     `checked/open/reduced` 与该单元 `query check --compact` 的
     `decl_checked/exercise_open/expr_reduced` 一致；
  3. **负例不假绿**：把夹具的 `proj/Lib2.sokonanoda`（a）删掉、（b）改成有内核错误的文本
     ——两种情况 `course.unit.failed > 0`，**绝不允许** `0/0` 全零绿；
     读不到/解析失败仍走既有 `error` 字段（`crates/cli/src/course.rs:74-89`，行为不变）；
  4. **cwd / 相对路径（G-12 交互）**：在夹具目录里用相对清单路径
     `(cd docs/gaps/repro/G06-course-import && node ../../../../scripts/soko course course.json --json)`
     （本轮实测这条命令今天能跑，只是报红）与在仓库根用相对清单路径两条都必须绿
     （证明 course 侧绝对化生效且不依赖 cwd）；
     同时 `bash docs/gaps/repro/G12-*.sh` 仍 exit 0；
  5. **回归**：`course/course.json` 的 11 个单元计数与行数（11 `course.unit` + 1
     `course.summary`）与修前**逐字节相同**，两张 GOLDEN 表未改；
  6. **缓存共用**：临时 cache 冷跑一次 `course`（或 `grade`）后，
     `build --json <同一单元>` 必须报 `"status":"hit"`——本轮已实测这条判据今天成立
     （`grade` 冷跑 → `build --json` 输出 `{"status":"hit",…}`），修完后 course 也必须
     写进同一份闭包摘要（依赖 `plan.digest` 不含路径：`mod.rs:119-144`）。仅限**有清单**的
     单元（走回退根的单元模块名可能不同，摘要自然不同，属已知边界）。
- **课程用例**：`courses/set-theory/`（12 单元，全部 import）
  - 代表单元：`courses/set-theory/units/unit05-pairs-products.sokonanoda`
    （`import lib.Logic` / `lib.Set` / `lib.Prod`，:25-27；练习 `prod_mk_inj`，
    另有 `prod_fst_mk` / `prod_snd_mk` / `prod_mk_eq_iff`，本轮由
    `grade --json` 实测到的 `exercise.open` 事件名）
  - 证据命令（绝对路径，G-12 纪律）：
    `node scripts/soko course "$PWD/courses/set-theory/course.json" --json`
    修后期望：`{"checked":63,"failed":0,"open":93,"type":"course.summary","units":12}`
    （今天 `{"checked":1,"failed":63,"open":93,…}`）
  - **逐单元基线（修后期望，本轮用 `query check --compact` 逐单元实测所得；
    单位 = checked/open/failed/reduced）**：

    | 单元 | 文件 | 期望 |
    |---|---|---|
    | 01 | unit01-sets-membership | 2 / 6 / 0 / 0 |
    | 02 | unit02-subsets-empty | 1 / 10 / 0 / 0 |
    | 03 | unit03-union-inter-powerset | 3 / 10 / 0 / 0 |
    | 04 | unit04-extensionality-identities | 3 / 8 / 0 / 0 |
    | 05 | unit05-pairs-products | 5 / 7 / 0 / 0 |
    | 06 | unit06-relations | 15 / 8 / 0 / 0 |
    | 07 | unit07-functions | 3 / 9 / 0 / 0 |
    | 08 | unit08-images-preimages | 15 / 9 / 0 / 0（该单元另有 1 个 `example.checked`，按 `course` 口径不计） |
    | 09 | unit09-equinumerosity | 6 / 7 / 0 / 0 |
    | 10 | unit10-cantor | 4 / 6 / 0 / 0 |
    | 11 | unit11-universe-russell | 3 / 5 / 0 / 0 |
    | 12 | unit12-synthesis | 3 / 8 / 0 / 0 |

  - 同时 `python3 courses/set-theory/tools/check.py` 必须仍 exit 0（课程线门禁不因本改动变红）；
    这张表**只作为交付证据**，不进语言仓 Rust 测试（课程每加一题就会变）。
- **影响面（事件计数 / golden）**：
  - 无 import 的单元：事件计数**不变**（单文件分支逐字节保留）⇒ `course/course.json` 的
    两处 GOLDEN（`crates/cli/tests/course.rs` 与 `course_status.rs` 的**双 GOLDEN**）**不需要同步**，
    改了就是回归；
  - 有 import 的单元：计数从"假红"变真值（`courses/set-theory` 无既有 golden，故无 golden 迁移）；
  - 复现脚本：`bash docs/gaps/repro/G06-course-import.sh` 由 exit 0 → **exit 1**
    （这正是 `gap.py close` 的前置；**不要为了让它 exit 0 而改脚本**——
    `scripts/gap.py:179-189` 对 `status: fixed` 的期望就是"行为已变"）；
  - 性能（本轮实测，供实现者对照）：今天 12 单元聚合 0.07 s（假象：所有单元在第一条库名字
    上就早停）；项目模式单单元**冷** 0.245 s、**热** 0.047 s、11 个单元冷跑合计 4.17 s
    ⇒ 修后 `course courses/set-theory/course.json` 冷跑约 4–5 s、热跑 < 1 s。
    若实现把闭包按单元重复编译到不可接受，先别优化（"一次一刀"），把数字写进交付说明。

## 文档同步清单

- `docs/protocol.md:564-579`（Course map）：补三句——① 有 `import` 的单元走项目闭包
  （与 `grade`/`query check` 同一份闭包）；② 模块根策略（最近 `sokonanoda.toml`，
  否则 `course.json` 所在目录）；③ `failed` 口径 = 入口错误 + 闭包级错误，
  与 `grade` 退出码同判。`:584` 的 build 缓存句仍成立（course 继续读同一份缓存，
  现在还会写入闭包条目）。
- `docs/design/imports-and-projects.md`：**:609 的表格行「`course` | **不动**（golden 表是硬门禁）」
  与 :345 的「`watch`/`build`/`course` | 每文件独立」必须改成 as-built**（显式推翻旧决策，
  设计先行；写明"无 import 的单元仍走单文件，golden 依旧不动"）。
- `docs/design/course-status.md:38-41`（写着 `compile_fol_with(unit_src, Full)`）：写成
  as-built——今天是 `compile_cached` + `CompileOptions::default()`，本轮起 import 单元走闭包、
  prelude 按文件指令。
- `docs/design/compile-cache.md:57-59`（"`course` 的 `count_unit` … 都走 `compile_cached`"）：
  补闭包分支与"与 `check`/`build`/`query` 共用 `plan.digest` 摘要键"。
- `docs/architecture.md:228-230`（§4.5 消费方列表：CLI / query / LSP / build）：加 `course`。
- `docs/design/teaching-project.md`：`:380` 的草图与 as-built 偏差（不动 `CompileOptions`）
  记一句；`:243`/`:248` 的"依赖 G-06 的修复进度"改成已落地（DoD 第 6 条自此成立）。
- `skills/`：`skills/sokonanoda-teacher/SKILL.md`（:235 单元地图在 `course/course.json`；
  判卷/进度段）与 `skills/sokonanoda-dev/SKILL.md`（:43 测试层）——若提到 course 的聚合语义
  则同轮更新；`.agents/skills/` 薄入口不用改（`crates/cli/tests/dsh.rs` 守漂移）。
- `editor/vscode/`：**无代码改动**（事件键不变；`extension.js:131-134` 只发现
  `course/course.json`，`:798` 只读固定键；`test-extension-host.js:625` 的 spawn 断言不受影响）。
  CHANGELOG 是否追加一条 Fixed 由发布轮决定（**待确认**：0.58.0 的 CHANGELOG 只记扩展自身
  能力，本改动不涉及扩展代码）。
- `AGENTS.md`（Setup 段的判卷命令，可在 `grade` 例子旁补一条 `course` 聚合例子）、
  `docs/HANDOVER.md`（能力清单/下一步）、`STATUS.md`（本轮）、
  `REQUIREMENTS.md` §9（用户可见行为变化：课程聚合从此认 import，日期一行）。
- 台账：`docs/gaps/ledger.jsonl` 的 G-06 → `status: wo-filed` + `wo: docs/gaps/WO-007-course-import.md`。
  ⚠️ 本轮交付**只写这份 WO 文件**，不动 ledger（多张 WO 并行落地时同一文件会被互相覆盖），
  由语言线接单那一刻回填。

## 门禁

- `scripts/soko gate`（fmt + clippy + test + playground 锚点）；注意：**不要**用
  `cargo fmt --all`（会重排冻结内核），只 fmt 教学 crates 或直接跑 gate。
- 全量 `cargo test --workspace --locked`。
- `bash docs/gaps/repro/G06-course-import.sh` ⇒ **必须 exit 1**（行为已变）。
- `bash docs/gaps/repro/G12-relative-entry-ancestor-manifest.sh` ⇒ **必须仍 exit 0**（不代做 G-12）。
- `python3 courses/set-theory/tools/check.py` ⇒ 0；课程聚合基线见"验收"。
- `python3 scripts/gap.py check`：**关账前允许 G-06 一行红**（status 仍是 `wo-filed`，
  观察值已是"行为已变"）；`close` 之后必须全绿。

## 关账

- 修好后：`python3 scripts/gap.py close G-06 --version 0.59.0`
  （复现脚本已翻成 exit 1，`scripts/gap.py:198-215` 才会放行；
  **待确认**：版本号以修复落地时的实际版本为准——仓库现为 0.58.0，ROADMAP 未点名 0.59.0）。
- 关账后：`python3 scripts/gap.py check` 全绿；G-06 得 `status: fixed` + `fixed_in`；
  `notes` 里追加一句"course 侧入口绝对化（不替代 WO-002）"。
- 课程侧同轮：`courses/set-theory/sokonanoda.toml:4` 的 `requires = "0.58"` 升到新版本钉，
  复跑 `python3 courses/set-theory/tools/check.py`；课程清单/golden 若在课程线另有钉法，
  由课程线按新基线（63 checked / 93 open / 0 failed）同步。
- 回填：`docs/design/teaching-project.md` §4.2 进度列、`STATUS.md` 本轮标题、
  `docs/HANDOVER.md`；并把"course 认 import"写进 `skills/sokonanoda-teacher` 的进度一节，
  让教师 agent 不再依赖 `grade` 逐文件绕行。
