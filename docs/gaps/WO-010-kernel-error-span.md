# WO-010 内核拒绝的诊断 span 与出错声明范围不一致（G-15）

> 台账行：`docs/gaps/ledger.jsonl` 第 2 行（`id=G-15`、`kind=tooling`、`severity=painful`、
> `status=open`、`wo=null`、`wo_planned=null`）。本 WO 落地后应把该行 `wo` 指向本文件、
> `status` 改 `wo-filed`（见文末「关账」）。
> 写这一轮**没有改任何源码、复现件或课程文件**，只创建了本文件；下文每个技术判断后面都标了
> 依据（跑过的命令 / 读到的文件:行号）。
>
> **一句话结论（先读这条）**：0.58.0 实测逐字节复核后，内核拒绝的 span **精确等于**出错声明的
> 范围；台账记的两种「溢出」形状与「更严重的现场」，都能用**复现脚本自己的度量口径**
> （把**字节** offset 当**字符**索引）逐字算出来。⇒ 本 WO 要修的不是「搬 span」，而是
> ①把坐标契约钉死（`query check` 的 `failed[]` 今天只给两个裸整数）、②把量具修好、
> ③加上守护。否则课程线与 agent 会继续照着错的结论二分定位。

## 用户可见症状 / 最小复现

- 复现命令（一条，可直接粘贴）：`bash docs/gaps/repro/G15-query-check-bare-offsets.sh`
- 今天的实际输出（2026-09-18 实跑；`node scripts/soko version` = `0.58.0` / `repo-build`，
  `./target/release/sokonanoda --version` = `sokonanoda 0.58.0`，且 `git status --short crates/`
  为空 ⇒ 这台构建与仓库源码一致；`exit 0` = 按脚本判据「缺口仍在」）：

  ```text
  == ① 带 import（课程单元的形状）==
     坏声明 3-4 行；诊断归因 3-6 行 → **溢出**
     → 落在坏声明内？NO（末端溢出）

  == ② 无 import（对照）==
     坏声明 8-9 行；诊断归因 6-9 行 → **溢出**
     → 落在坏声明内？NO（起点提前）

  结论：G-15 仍在（内核拒绝的 span 与出错声明范围不一致）——与台账一致。
  ```

- **复核：同一次运行的原始事件里，span 是精确的。** 把复现脚本造的那两个文件原样交给同一台
  二进制（`node scripts/soko grade <file>`，只读它自己吐的 JSON Lines）：

  | 形状 | 事件自带 `span.start/end` | `offset` 区间 | 按**字节**重算的行号 | 脚本印出的行号 |
  |---|---|---|---|---|
  | ① `WithImport.sokonanoda`（坏声明 3-4 行） | `3:1 → 4:19` | `12..104` | `3 → 4`（= 声明范围） | `3 → 6`（报「溢出」） |
  | ② `NoImport.sokonanoda`（坏声明 **6-7** 行） | `6:1 → 7:19` | `201..293` | `6 → 7`（= 声明范围） | `6 → 9`（报「溢出」） |

  两个 `offset` 区间按字节切出来的文本**逐字等于**坏声明本身
  （`theorem bad (α : Type) (A : Set α) : Set.subset α (Set.empty α) A :=\n  fun (x : α) => x`）。

- 根因在**度量口径**，不在编译器（三条证据）：
  1. `span.offset` 是**字节**偏移、`column` 按**字符**计数：`crates/front/src/token.rs:84-89`
     （`self.offset += c.len_utf8();` 与 `self.line += 1 / self.column += 1`）；
  2. 复现脚本用 Python 的**字符切片**去消费这个字节偏移：
     `docs/gaps/repro/G15-query-check-bare-offsets.sh:62`（`src = open(path).read()`）与 `:68`
     （`sl = src[:s].count("\n") + 1`）；
  3. 两个文件里都有 `α`（UTF-8 两字节）⇒ 「取第 104 个**字符**」比「取第 104 个**字节**」
     多走一截：① 里 3 个 `α` 刚好跨过第 5 行空行、落进第 6 行的 `theorem next`；② 里 6 个 `α`
     跨到第 9 行。
- 形状 ② 的**标签本身也错**：脚本 `:82` 硬编码 `probe NoImport.sokonanoda 8 9`，但同一脚本
  的 heredoc 里坏声明在 **6-7** 行（1-4 行四条定义、5 行空行、6-7 行 `theorem bad`、8 行空行、
  9-10 行 `theorem next`）。台账「无 import 时坏声明 8-9 行…起点提前两行」正是从这个错标签
  推出来的：真实起点 6 行与坏声明起点一致，**漂的是末端**（6→9，而不是「起点提前」）。
- **课程规模复核（决定性）**：把错证明塞进真课程文件
  `courses/set-theory/units/solutions/unit02-solution.sokonanoda`（第 1-3 行是中文注释、
  第 7 行起是 `theorem subset_refl …`，全文件 84 行 / 4316 字符 / 4609 字节）后判卷，
  事件字段是 `7:1 → 8:37`、按字节重算是 `7 → 8`（= 声明范围）；**把同样的 offset 当字符索引**
  算出来是 `10 → 12`——与台账「更严重的现场：第 7 行写错、诊断归因到第 10-11 行（下一个
  theorem）」同形。原始 spike 文件（108 行版）今天已不在仓库
  （`docs/gaps/spike/` 只剩 `README.md`），无法逐字节复算 ⇒ 这一条记为「同源强相关」，
  标**待确认**（见文末）。

### 附：形状实测（同一台 0.58.0 二进制，临时项目 `/tmp/g15shapes/proj`）

| # | 形状 | 诊断 | `span` 字段 | 按字节重算 | 判定 |
|---|---|---|---|---|---|
| A | 中间坏 `theorem` + `import Lib` | `kernel-rejected`/kernel | `3:1 → 4:19`（off `12..104`） | `3 → 4` | = 声明范围 ✅ |
| B | 末尾坏 `theorem` + import | `kernel-rejected`/kernel | `6:1 → 7:19` | `6 → 7` | ✅ |
| — | 无 import、中间坏（复现 ② 的文件） | `kernel-rejected`/kernel | `6:1 → 7:19`（off `201..293`） | `6 → 7` | ✅ |
| D | `by` tactic 里写错（走判据前缀） | `elab-tactic-failed`/elab | `4:3 → 4:10`（=`intro x` 两个 token） | `4 → 4` | ✅ 前缀坐标没漏出来 |
| F | 坏声明在**依赖** `Broken.sokonanoda` 里 | `kernel-rejected`/kernel，**带 `file`/`module`** | `6:1 → 7:19` | `6 → 7`（依赖文件坐标） | ✅ |
| G | 中文注释头 + import + 中间坏 | `kernel-rejected`/kernel | `5:1 → 6:19` | `5 → 6` | ✅ |
| F′ | F 的入口走 `query check` | `import-dependency-failed` | `start=0/end=13`（入口第 1 行 `import Broken`） | `1 → 1` | ✅ 依赖错误不进 `failed[]` |
| C | 归纳块（我的形状自身写错，parse 就报 `unexpected-token` @`4:1`） | parse | — | — | **未覆盖**内核归纳块（待确认 3） |
| E | `#check (Set.empty 3)` | 无诊断 | — | — | **未覆盖** `#check` 失败路径（待确认 3） |

> 量具教训（写 WO 时我自己踩了一次，与 G-15 同型）：F 形状第一次复核时我拿**入口文件**的文本
> 去折算依赖的 offset，得到「`5:5`、切片为空」，差点报成缺陷；正确做法是读事件里的
> `file`/`module` 字段（`grade --json` 已带）。**span 不可信之前，先怀疑自己的坐标空间。**

## 期望行为

- **官方 Lean 4**：命令级诊断的 range 覆盖**整条命令**（`Syntax` 的 range 就是该命令的
  source range），且 LSP `character` 是 **UTF-16** 码元。这不是 Lean 语义而是工具链契约：
  编辑器高亮、`query check` 的 `failed[].start/end`、agent 的自动修复循环都挂在它上面
  ——台账 `expected_lean` 字段原话（`docs/gaps/ledger.jsonl` G-15 行）。
- **本教学子集的边界**（本 WO 只钉这三条）：
  1. 内核拒绝（`stage=kernel`）的 span == 出错命令的源码范围（起点 = 命令首 token，末端 = 值
     表达式末 token，即 `parser.rs:175/195/213` 的 `Span::new(start, val.span().end)`）；
  2. 每个对外通道里的位置字段**必须自带单位与坐标空间**，消费者不需要自己猜/自己换算；
  3. `by` tactic / `match` / 无注解 binder 走判据前缀（`docs/architecture.md` §4.5）判出来的
     错误，落回**真实文件坐标**。
- **明确不做**（不是本 WO 的边界）：不追求 Lean 的 `Info`/`Syntax` 结构、不做多诊断合并、
  不做「错误最小化」（Lean 的 `Info` tree 那套）。

## 真实缺口（本 WO 的修法对象）

> 台账记的现象不成立（见上），但「课程线与 agent 定位不可信」这个**后果**是真的。缺口的落点
> 如下四条，前三条必修、第四条是本轮新增的守护。

### R1 `query check` 的 `failed[]` / `warnings[]` 只有两个裸整数

- 形状：`{code,message,start,end}`——没有单位、没有行列、没有坐标空间说明，也没有 `file`/`module`。
  证据：`crates/front/src/query/types.rs:159-174`（`FailedDecl` / `WarningInfo` 各只有
  `start: usize, end: usize`）、`crates/front/src/query/mod.rs:288-315`
  （直接填 `e.span.start.offset` / `w.span.start.offset`）、`docs/protocol.md:650`（对外形状）。
- 现状实测（`node scripts/soko query check --file /tmp/g15def.sokonanoda --compact`，该文件
  内容就是下文 front 单测里那三条 `def`）：
  `"failed":[{"code":"kernel-rejected", …, "start":47,"end":92}]`——这两个数就是字节 offset，
  但**协议没写**：`docs/protocol.md:676-677` 只说了「`line`/`col` 是 UTF-16、`--offset` 是字节」，
  没说 `failed[].start/end` 是哪种。
- 后果（已在仓库里发生）：复现脚本（`:62,68`）与课程线的定位习惯都把它当字符索引
  ⇒ 「span 溢出到别的声明」这个假象，以及台账 `notes` 里记的「我最初三版复现都因为 span
  不可信而误判」。这正是 `blocks` 里「课程作者与学习者的定位」「agent 自动修复循环」两条。
- 另有一条**坐标空间**事实要写进协议：`query check` 只看**入口文件**的报告
  （`crates/front/src/query/mod.rs:119-135`：先由入口 report 装配，项目模式下再取
  `entry_module().events`；`docs/design/project-view.md` 的「只读派生」同义）⇒ `failed[]` 的
  offset 坐标空间 = 入口文件；依赖模块的错误只以 `import-dependency-failed` 出现（F′ 实测），
  要看依赖的病得走 `query project`（模块行 `errors: N`）或 `grade --json`（带 `file`/`module`）。

### R2 复现脚本的度量口径错（量具缺陷）

- `docs/gaps/repro/G15-query-check-bare-offsets.sh:62`（读文本）+ `:68`（`src[:s]` / `src[:e]`）
  把**字节** offset 当**字符**索引；`:82` 的期望行区间（`8 9`）与该 heredoc 里坏声明的真实
  区间（`6 7`）不符。
- 「缺口即测试」的机制（`docs/design/teaching-project.md` §6.4、`scripts/gap.py:61-78`）完全
  依赖复现脚本的退出码；量错的脚本会让 `gap.py check` 与 `gap.py close` 一起说谎。

### R3 没有守护「内核拒绝的 span == 出错命令范围」

- 现存唯一相关测试是 `crates/front/src/compile/tests.rs:87-92`
  （`reports_kernel_rejection_with_span`），断言只有 `out.errors[0].span.start.line >= 1`——
  等于没断言。`docs/TESTING.md` 的「kernel 检查」行也引用了它。
- 所以今天任何真的 span 漂移都不会被测出来：**G-15 值得修的一半是这条守护**。

### R4 与 0.58.0 修过的 warning 归因是不是同一处代码？（点名问题的答案）

| 关注点 | error 侧 | warning 侧 |
|---|---|---|
| 「这条诊断属于哪个文件/哪条命令」 | `CompileOutput.error_cmds` 平行数组：字段 `crates/front/src/compile/event.rs:33`，入口 `push_error` `:55-58`，切分 `crates/front/src/compile/units.rs:85-89` | `CompileOutput.warning_cmds`：字段 `event.rs:39`，入口 `push_warning` `:61-64`，切分 `units.rs:97-106`（**同一条 `split_report`**） |
| span 从哪来 | `PendingOp::{Decl,InductiveBlock,Check,Reduce,Print}.span`（`crates/front/src/compile/check/walk.rs:312/502/602/764`）← parser 的 `Command::*.span`（`crates/front/src/parser.rs:175/195/213`）；装配在 `crates/front/src/compile/check/kernel_phase.rs:223/251/318/341/360` | 语法级：`crates/front/src/compile/warning.rs` 自算；`redundant-sorry`：hole span（`kernel_phase.rs:134-143` 的 `hole_span`） |

**结论：归因同族（同一条 `split_report` + 同款平行数组），span 不同源。**
⇒ 0.58.0 修 `warning_cmds` 不会顺带修 G-15；反过来，本 WO **不碰** `split_report` 与那三个
平行数组（它们刚被 0.58.0 的守护 `warnings_are_attributed_to_the_unit_that_produced_them`
钉住，见 `docs/TESTING.md` 表末行）。
**同族隐患（建议顺手写清，不改行为）**：`crates/front/src/query/mod.rs:119-127` 手工装配
`CompileOutput { events, errors, warnings, ..Default::default() }`，绕过 `push_*` 入口 ⇒
`event_cmds`/`error_cmds`/`warning_cmds` 在这里是**空数组**，与 `units.rs:80-84` 的
`debug_assert_eq!` 不变量（「每条错误必须带命令下标」）在字面上冲突。今天无害（`check()` 只读
`errors`/`warnings`），但谁哪天用这里的 `error_cmds` 就会静默失配——加一句注释或改用 `push_*`。

## 范围

| 文件 | 位置 | 改什么 |
|---|---|---|
| `crates/front/src/query/types.rs` | `:159-174` | `FailedDecl`/`WarningInfo` **新增** `start_line/start_col/end_line/end_col`（`u32`，与事件 `span` 同口径）；`start/end` 保留（字节） |
| `crates/front/src/query/mod.rs` | `:288-315` | 填新字段（`e.span.start.line/column` 等）；`R4` 的空平行数组加注释说明 |
| `docs/protocol.md` | `:650`、`:676-681` | `failed[]`/`warnings[]` 形状更新；**写死**「`start/end` 是字节 offset、坐标空间 = 入口文件」「行列 1 基」；`ok:false`/退出码不动 |
| `docs/gaps/repro/G15-query-check-bare-offsets.sh` | `:57-74`（`probe()`）、`:77`/`:82`（调用） | 度量改字节正确（首选**直接用事件自带的 `span.start.line/end.line`**，并**顺带断言**字段与字节重算一致）；`:82` 期望区间改 `6 7`；注释里的「0.58.0 实测」结论改写 |
| `crates/front/src/compile/tests.rs` | `:87-92` 收紧紧 + 新增一条 | 见「验收」 |
| `crates/cli/tests/query.rs` | 新增测试 | `failed[]` 带行列、且与 `grade --json` 的 `diagnostic.span` 一致（现有 `:125`、`:306` 只断言长度/非空，不受加字段影响） |
| `docs/TESTING.md` | 「kernel 检查」行附近 | 加守护行（诊断坐标） |
| `courses/set-theory/AGENTS.md` | `:22-23` | 判卷纪律改写：`span.offset` 是**字节**；用事件自带 `line/column`；二分法保留为「解析失败/多条错误时」的手段 |

- **是否动内核：否。** 内核是冻结快照（`REQUIREMENTS.md` §2 第 1 条）；span 全程由 front 装配
  （`kernel_phase.rs` 的三处 `CompileError::kernel(…, span)`），`crates/kernel/` 一行不动。
- **是否动课程内容：否。** 课程线 12 个单元依赖的是 `grade` 的退出码与事件计数
  （`courses/set-theory/tools/check.py:31-47` 只数 `decl.checked`/`exercise.open` + 退出码，
  **不消费任何 offset**——读 `:31-47` 确认），所以 R1/R2/R3 对课程内容零影响。

## 兼容策略（含「同轮必须一起改」的文件清单）

1. **只加字段，不改语义**：`docs/protocol.md:681` 明写「Fields are **additive only**；renaming
   one is a breaking change」⇒ 新增 `*_line/*_col`，**不动** `start/end`（动手改成行列 = 破坏性，
   直接违反协议；也不要把 `start/end` 重命名成 `offset`）。`soko.query/1` 的 schema 号不动。
2. **零课程改名/零构造子改动**：本 WO 与 G-02（构造子命名空间）无交集——不需要碰
   `courses/set-theory/lib/Prod.sokonanoda` 的 `prod_mk`、不需要碰任何 `lib/` 或
   `units/*.sokonanoda`（那是 WO-005 的地盘，见 `docs/design/teaching-project.md` §8 的 P1 排序）。
   课程侧唯一改动是**手册里一条判卷纪律**（`courses/set-theory/AGENTS.md:22-23`）——今天它把
   G-15 当成产品事实教给课程 agent，不修它会继续传播错结论。
3. **golden 不动**：事件种类与计数不变 ⇒ `crates/cli/tests/course.rs:86` 与
   `crates/cli/tests/course_status.rs:68` 的**双 GOLDEN** 不需要同步（改 golden 的范围 = 仅
   `query.rs` 的新断言）。
4. **同轮改动清单（全部，缺一条都不算完成）**：
   `crates/front/src/query/types.rs`、`crates/front/src/query/mod.rs`、`docs/protocol.md`、
   `docs/gaps/repro/G15-query-check-bare-offsets.sh`、`crates/front/src/compile/tests.rs`、
   `crates/cli/tests/query.rs`、`docs/TESTING.md`、`courses/set-theory/AGENTS.md`、
   `docs/HANDOVER.md`、`STATUS.md`、`docs/gaps/ledger.jsonl`（G-15 行）。

## 不做的事（明确排除，防顺手扩大）

- **不动内核**（`crates/kernel/**`），不改 `EnvLimit`/`try_check_declar` 的任何行为。
- **不动 `split_report` 与三个平行数组**（`crates/front/src/compile/units.rs:44-108`）：0.58.0 刚
  修好，`docs/TESTING.md` 有守护。
- **不改 `span.offset` 的字节语义**：`--offset`（`docs/protocol.md:677`）、缓存摘要、LSP 语义
  着色（`crates/lsp/src/tokens.rs:88-89` 自己按 UTF-16 编码）全依赖它。
- **不把 `query check` 改成只给行列**（破坏性），也不新增 op/事件类型。
- **不顺手修 UTF-16 列**：`docs/protocol.md:676` 说 `line/col` 是 UTF-16，实现按 **char** 计数
  （`token.rs:88`），LSP 诊断 range 直接把 `span.column` 当 UTF-16 `character`
  （`crates/lsp/src/render.rs:52` 的 `range_of`）⇒ emoji/增补平面字符会让高亮右移
  （中文是 BMP、课程撞不到）：这是**另一条缺口**，先有 repro 再开 WO。同理
  `crates/lsp/src/render.rs:13-21` 的 `pos_at` 用**字节**数列（只被 `:216` 的括号 hover 用）也另开。
- **不做 G-01/G-02/G-03/G-06**：与本文无关。
- **不给 `diagnostic` 事件加 `file` 字段**：多文件时已带 `file`/`module`（F 形状实测），无需改。
- **不改课程内容**（`courses/set-theory/**` 除 `AGENTS.md` 的一条纪律外）。

## 验收（三层）

- **front 单测**（`crates/front/src/compile/tests.rs`）：
  - 收紧紧 `reports_kernel_rejection_with_span`（`:87-92`）：断言 span 切片等于坏声明文本，
    而不只是 `line >= 1`；
  - 新增（下面这段输入我已用 0.58.0 实跑：切片逐字相等、`grade` exit 1）：

    ```rust
    #[test]
    fn kernel_rejection_span_is_the_failing_command_range() {
        // 三条声明、坏的夹在中间：span 不得溢到 good/after 上（G-15 的真实判据）。
        let src = "def good : Prop -> Prop := fun (p : Prop) => p\n\
                   def bad : Prop -> Type := fun (x : Prop) => x\n\
                   def after : Prop -> Prop := fun (p : Prop) => p\n";
        let out = compile_fol(&parse(src).unwrap());
        let e = &out.errors[0];
        assert_eq!(e.code(), "kernel-rejected");
        assert_eq!(
            &src[e.span.start.offset..e.span.end.offset],
            "def bad : Prop -> Type := fun (x : Prop) => x"
        );
    }
    ```
  - 带 `import` 的那一形状（复现 ① ）进 front 的项目测试（`crates/front/src/project/tests.rs`
    的既有夹具）或 CLI e2e，避免只保住单文件路径。
- **CLI e2e**（`crates/cli/tests/query.rs`）：同一文件同时跑 `grade --json` 与 `query check`，
  断言 `failed[0]` 的 `start_line/start_col/end_line/end_col` == 该 `diagnostic` 的
  `span.start/end` 的行列，且 `start/end` 仍是字节 offset；再断言依赖坏时 `failed[]` **只有**
  入口的 `import-dependency-failed`（F′ 形状，坐标空间 = 入口）。可选：`protocol.rs` 的
  `kernel_rejection_diagnostic_shape` 加一条「`span.start.line` == 坏声明所在行」。
- **课程用例**（真文件 + 真练习名）：
  `courses/set-theory/units/unit02-subsets-empty.sokonanoda` 的**练习 2 `subset_trans`**
  （声明在 `:28-30`；对应解答 `courses/set-theory/units/solutions/unit02-solution.sokonanoda:7-8`
  的同名 `theorem`）。做法：把该单元复制到临时项目（连同 `courses/set-theory/lib/` 与
  `sokonanoda.toml`），把 `:30` 的 `sorry` 换成错证明
  `fun (x : α) => fun (hx : A x) => h1 x hx`——**已在 /tmp 副本上实跑**：判卷
  `kernel-rejected`、`span = 28:1 → 30:43`、按字节 `28 → 30`、`exit 1`；修完后
  `query check` 的 `failed[0]` 必须给出 `start_line: 28`。
  课程文件本身**保持绿**（`python3 courses/set-theory/tools/check.py` 必须仍然全绿）——
  上面只是验收素材，答案不许入库。
- **影响面**：事件种类与计数**不变**（行列是诊断字段，不是新事件）⇒ **不改双 GOLDEN**
  （`crates/cli/tests/course.rs:86`、`crates/cli/tests/course_status.rs:68`）；改动带来的
  唯一 golden 面是 `crates/cli/tests/query.rs` 新增断言。

## 文档同步清单

- `docs/protocol.md:650`（`failed[]`/`warnings[]` 加字段 + `start/end` 单位 = 字节 + 坐标空间 =
  入口文件）、`:676-681`（UTF-16 说法与实现的边界，见「待确认 1」）。
- `docs/TESTING.md`：分层守护表加一行（「诊断坐标」：span == 出错命令范围；
  `query check` 行列与 `grade --json` 一致；跨文件归因仍按 `error_cmds`）。
- `docs/design/teaching-project.md` §6.4/§8：G-15 的关账状态（P1 排序里 G-15 原不在 P1 名单，
  若本轮顺带修完请在进度段写明）。
- `courses/set-theory/AGENTS.md:22-23`：**必须**改（判卷纪律）。
- `skills/`：写 WO 时全仓 grep `"failed"`：`skills/` 下只命中 `sokonanoda-ci/SKILL.md` 的
  `gh run rerun --failed`（CI 排错，与本协议无关）⇒ **预期不改**；实现者请再 grep 一次
  `failed` 确认（`crates/cli/tests/skill.rs` 会挡漂移）。
- `editor/vscode/`：编辑器走 LSP 诊断，`editor/vscode/src` grep `"failed"` 无命中 ⇒
  **预期不改**；若实现者顺手动了 LSP 诊断列（不在本 WO 范围），则按
  `docs/vscode-dev-guide.md` 同步 + bump。
- `AGENTS.md`（仓库根）：入口命令未变 ⇒ 预期不改；若 skill 正文改了按硬规则同轮同步
  `.agents/skills/` 入口（`crates/cli/tests/dsh.rs` 挡漂移）。
- `docs/HANDOVER.md`、`STATUS.md`（`STATUS.md` 只留最近 3 轮；旧轮归档
  `docs/STATUS-ARCHIVE.md`）。
- `REQUIREMENTS.md` §9：本次不是用户新要求 ⇒ 不追加；若把「诊断坐标契约」升级成用户要求，
  在 §9 追加一条并注明日期。
- `docs/gaps/ledger.jsonl` G-15 行（见「关账」）。
- 版本 bump 按 `docs/RELEASE.md` 纪律（`query` 输出加字段 = 用户可见）。

## 门禁

```bash
bash docs/gaps/repro/G15-query-check-bare-offsets.sh   # 修好后应 exit 1（= 行为变了）
python3 scripts/gap.py check                          # 台账一致性（脚本改了它才可能绿）
scripts/soko gate                                     # fmt + clippy + test + playground 锚点
cargo test --workspace --locked                        # 贡献者路径（唯一需要 cargo 的一步）
```

> `gate` 的 anchor 用**运行中二进制**的内嵌编译器；它与仓库版本不一致会直接 exit 3
> （`AGENTS.md` 的注意事项）——先 `scripts/soko update`，或用
> `cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda`。
> **禁止** `cargo fmt --all`（会重排冻结内核）：只 fmt 教学 crates，或直接 `scripts/soko gate`。

## 待确认（不许在实现里当事实用）

1. **UTF-16 列**：`docs/protocol.md:676` 承诺 UTF-16，`crates/front/src/token.rs:88` 按 char 计数，
   `crates/lsp/src/render.rs:52` 又把 `span.column` 当 UTF-16 用 ⇒ 增补平面字符（emoji 等）
   会让列/高亮偏 1。需要一条**独立 repro**（同一行放一个 emoji + 后面跟一个报错 token）才能定性；
   中文（BMP）不受影响，所以 12 个单元从没撞到。
2. **会话/缓存路径**：`crates/front/src/session.rs:418`（`span_from_offsets`）与 `:436`
   （`remap_span`）在编辑后按「旧命令内相对偏移」平移 span——本轮只测了**磁盘重编译**
   （CLI `grade`），没有测 LSP 增量编辑；若要在编辑器路径上也声称「span 精确」，需要一条
   session 级测试。
3. **未实测的两条内核路径**：归纳块的内核拒绝（`kernel_phase.rs:240-299`）与
   `#check`/`#reduce` 失败（`:301-348`）——我的 C 形状先在 parse 就错了、E 形状没报错，
   所以「这两种 shape 里 span 也精确」目前**是推断不是实测**。
4. **台账「更严重的现场」**（spike 02 解答第 7 行 → 归因 10-11 行）：原始 108 行文件不在仓库，
   只能给「同源强相关」（我在现行课程文件上用同一口径复算出 `10 → 12`）；标待确认。

## 关账

- 修好后：`python3 scripts/gap.py close G-15 --version <新版本>`。`close` 会先复跑复现，
  `exit 0`（仍复现）会被**拒绝**关账（`scripts/gap.py:198-215` 读源码确认）。
- **顺序注意**：本 WO 的「修好」包含**把量具修对**——脚本改对后它会翻成 `exit 1`
  （「行为变了」），这正是 `close` 期望的信号。脚本重写必须与产品改动**同一轮**，否则
  `gap.py check` 会在「台账说 open / 复现说精确」之间来回红。
- 手改 `docs/gaps/ledger.jsonl` G-15 行（`gap.py close` 只写 `status/fixed_in/note`）：
  - `wo` → `docs/gaps/WO-010-kernel-error-span.md`、`status` → `wo-filed`（**本 WO 落地即可做**）；
  - 关账时重写 `today`（现在写的「带 import 时坏声明 3-4 行、诊断归因 3-6 行…」与 0.58.0 实测
    不符）与 `expected_lean`（把判据写成「span == 出错命令范围 + 通道自带行列/单位」）；
  - `notes` 里保留教训：**offset 是字节，别再拿字符切片量 span**（这就是 G-15 的全部现场）。
- 收尾义务：`STATUS.md` 更新一轮；`docs/HANDOVER.md` 同步。
