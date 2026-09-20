# 课程大改写（Lean 4 符号 + tactic 模式）——工具 / 门禁 / 文档影响审计

> **只读调研**（2026-09-19，仓库 v0.61.0，工作树干净）。本文件是**新建的笔记**，
> 没有修改任何现有文件；课程树一个字节未动。
> 触发：`courses/set-theory/`（35 个 `.sokonanoda`）即将大规模改写 ——
> ① 联结词从点名形式改成 Lean 4 符号（`∧ ∨ ↔ ∃ ∀ ¬ →`）；② 所有证明改成 tactic 模式（`by` 块）。
>
> **本次快照的实测基线**（`python3 courses/set-theory/tools/check.py --json`，exit 0）：
> **36 目标 · 329 checked · 99 open · 0 判负**；`summary`：`canvas_open` 96 /
> `solutions_open` 0 / `lib_open` 0。
> 按 kind 拆：`lib` 10 目标 / 84 checked / 0 open · `unit` 12 / 65 / **96** ·
> `solution` 13 / 162 / 0 · `page` 1 / 18 / **3**（99 = 96 + 3）。
> 复跑前置：`scripts/soko doctor --json` ⇒ `ready: true`（`target/debug` 二进制自报 0.61.0）。

---

## 0. 结论速览

| # | 问题 | 结论 |
|---|---|---|
| 1 | G1–G6 判什么 | 只判**形状**：退出码 0 / 目标存在 / 解答 0 open 且 checked>0 / 解答按**名字**覆盖画布练习 / lib 无 `sorry` / 清单自洽。**计数与配额差额从不判红** |
| 2 | 门禁里有硬编码期望计数吗 | **没有**（判据代码里与计数比较的整数字面量只有 `0`）。硬编码在**文档/站点/台账**（§2），以及**三处测试/复现件**（§6、§8） |
| 3 | 配额今天是什么 | `I.1=35 · I.2=24 · I.3=23 · I.4=14`（合计 96 = 实测 `canvas_open`）。**差额只报告、绝不判红**（`check.py:20-21`） |
| 4 | 站点计数从哪来 | `gen-site-data.py` 跑门禁 `--json` 取 `summary`（`:220`、`:255-260`）；**CI 里那一步测不了**（离线 ⇒ doctor exit 3 ⇒ 沿用上次数字）。必须**本地**重生成 |
| 5 | 成本台账 | `docs/courses/ledger.jsonl`，`check.py --ledger`（默认关）人工追加；`soko.course-ledger/1`，11 个必填字段 + `rows` |
| 6 | 缺口台账 | 24 条：22 `fixed` + **2 `workaround`（L-04 / L-06）**。改写**不会**把任何一条翻成 `fixed`（纯课程内容）。但 **G-07 的复现件把真课程钉死**（§6.3） |
| 7 | 必须同轮更新的文档 | §7 的清单（含 4 个"红"项：`G07-course-manifest-v2.sh`、`notation.rs`、`query.rs`、`course_manifest.rs`） |
| 8 | 会挡改写的测试 | **8 个**：`notation.rs` 6 个 + `query.rs` 1 个 + `course_manifest.rs` 1 个（条件性）。**两处 GOLDEN 与 `courses/set-theory` 无关**（钉的是入门课 `course/`） |

---

## 1. 课程门禁 `courses/set-theory/tools/check.py`（1141 行）

### 1.1 判据 G1–G6 的确切定义

判红的**只有**这六条，全部在 `evaluate()`（`check.py:563-598`）与 `flatten_manifest()`
（`check.py:315-430`）里产出；计数**从不参与判红**（`check.py:7`、`check.py:20-21`）。

| 判据 | 判什么 | 代码位置 | 精确条件 |
|---|---|---|---|
| **G1** | 每个目标 `grade` 退出码 0 | `check.py:569-570` | `if row["exit"] != 0: reasons.append("G1：…")`。**只认 `grade` 的退出码**，不认 `query check`（G-10；`check.py:9`） |
| **G2** | 目标存在 | `check.py:1108-1113`（文件缺失 ⇒ `status="missing"` + `"G2：目标文件不存在"`）+ `check.py:1124-1132`（`problems` 通道补 `G2：` 前缀） | 目标集合由 `discover()`（`check.py:484-557`）算出：`lib/*.sokonanoda`（`:513-517`）、`lib 自检`（Demo 再判一次，`:518`）、`course.json` 的每个单元（`:522-530`）、每个单元的解答（`:542-548`）、`units/` 下**不在清单里的页面**+其解答（`:551-556`） |
| **G3** | 解答：0 open 且 checked>0 | `check.py:571-575` | 只对 `kind == "solution"` 的行：`row["open"] > 0` ⇒ 红；`row["checked"] == 0` ⇒ 红 |
| **G4** | 解答**按名字**覆盖画布的每个具名练习 | `check.py:579-594` | `covered = set(row["checked_names"])`；`missing = [n for n in canvas["open_names"] if n not in covered]`（`:591-592`）。**按名字，不按位置** |
| **G5** | `lib` + Demo 无 `sorry` | `check.py:576-577` | 只对 `kind == "lib"` 的行：`row["open"] > 0` ⇒ 红 |
| **G6** | 清单自洽（v2） | `check.py:349-430`（结构非法）+ `check.py:426-429`（prereqs） | volume/chapter id 唯一且非空（`:359-365`、`:376-382`）；每个 unit **恰好**属于一个 chapter（同一 `file` 出现两次 ⇒ 红，`:399-405`）；`prereqs` 指向存在的 chapter id（`:426-429`，允许前向引用）；单元条目必须带 `file` 与 `unit`（`:393-398`）。未知 `schema` ⇒ **前置错误 exit 2**，不猜（`:334-339`） |

**`quota` 不在判据里**：`quota_notes()`（`check.py:433-461`）只产出报告行，
`run()` 里它的异常被吞掉且不判负（`check.py:968-974`）。

### 1.2 计数从哪来（事件层）——**`example.checked` 是盲区**

`grade()` 只解析三种事件（`check.py:233-243`）：

| 事件 | 计入 | 是否要求有 `name` |
|---|---|---|
| `decl.checked` | `result.checked` / `checked_names` | 有名字才进 `checked_names`（`:236-237`） |
| `exercise.open` | `result.open` / `open_names` | 有名字才进 `open_names`（`:240-241`） |
| `diagnostic` | `result.diagnostics` | — |

**实测（本次探针，`/tmp` 临时文件，课程树零改动）**：

```
theorem named_proved (P : Prop) (h : P) : P := by exact h   → decl.checked    name=named_proved
example   (P : Prop) (h : P) : P := by exact h              → example.checked name=(无)
theorem named_open  (P : Prop) (h : P) : P := by sorry      → exercise.open   name=named_open
example   (P : Prop) (h : P) : P := by sorry                → exercise.open   name=(无)
```

* **匿名 `example` 走独立的 `example.checked` 事件，`check.py` 完全不看它** ——
  所以把已证演示 `theorem` 改成 `example`，门禁的 `checked` 会**下降**；
  反过来会**上升**。实测佐证：单元③ `query check --compact` =
  `decl_checked 3 / example_checked 1`，门禁报的 `checked` 就是 **3**。
* **匿名 `example` 的 `exercise.open` 没有 `name` 字段**（内核把匿名 example 内部命名为
  `_example_N`，见 `crates/front/src/compile/check/walk.rs:830`、`:910`；测试注释
  `crates/cli/tests/course.rs:56-59` 明说 "Events without a `name` (notably an anonymous
  `example`'s `exercise.open`) are skipped"）⇒ **它不进 `open_names`，G4 就不检查它**。

### 1.3 硬编码的期望计数：**没有**

* 判据代码里与计数比较的整数字面量只有 `0`：`check.py:569`（`!= 0`）、`:572`（`> 0`）、
  `:574`（`== 0`）、`:576`（`> 0`）、`:589`（`!= 0`）、`:1133`（`unit_count == 0`）。
* 设计硬规则：`docs/design/course-gate-in-ci.md:102-103`「门禁代码里除 `== 0` / `> 0`
  外，不得出现与计数比较的整数字面量；不得引入 golden 表或"N 个单元"清单长度断言」。
* `GRADE_TIMEOUT = 180`（`check.py:85`）是超时，不是计数。
* `SELFTEST_G6_GOOD` 里的 `"quota": {"exercises": 99}`（`check.py:722`）与
  `check.py:765` 的 `if not any("99" in note …)` 是**自检夹具**，与"99 open"无关。
* 唯一与目标数有关的注释在启动器里：`scripts/soko:912`「34 targets must not each
  re-resolve the launcher」—— **过期注释**（今天 36），不是断言。

### 1.4 各开关的确切行为

| 开关 | 位置 | 行为 |
|---|---|---|
| `--selftest` | `check.py:771-830`（分发 `:1073-1074`） | 五段：① 正控制（从**另一个 cwd** 判 `lib/Demo.sokonanoda`，G-12，`:775-784`）；② 变异（`exact bogus_name` 必须判负，G-10，`:786-791`）；③ 二分（3 个声明点名第 3 个 + 全绿前缀 = 2，`:793-800`）；④ G6 自检（三类坏清单判负 + 一份合法 v2 判绿 + 配额 99 只报告，`:802-803` → `:741-768`）；⑤ 成本台账字段齐全（`:805-817`）。任一失败 ⇒ 打印 `--selftest FAIL` 并 **exit 1** |
| `--json` | `check.py:1034-1036` | 把 `report`（`schema: "soko.course.check/2"`，`:977`）缩进打到 stdout；**exit 1 if `summary["rejected"]`** |
| `--only <标签>`（可重复） | `check.py:1082-1096` | **先精确后子串**：`target.label == needle` 优先，否则子串（`:1086-1087`）。去重（`:1088-1091`）。**没匹配到 ⇒ 打印可用标签并 exit 2**（`:1092-1095`）。注意标签来自 `course.json` 的 `unit` 号（`单元 N`）与文件 stem，**改文件名会改标签** |
| `--bisect` | `check.py:997-1008` → `bisect()` `:623-658` | 只对「有文件 **且** `grade` 退出码非 0」的行生效（`:1000-1001`）；全量跑时只二分判负目标，`--only` 时二分被选中的（`:1002-1003`）。按**行首关键字**扫顶层声明边界（`DECL_KEYWORDS = ("theorem","def","axiom","inductive","example","namespace","section")`，`:88`），逐次判前缀，输出「最后全绿的前缀」+「第一个判红的声明」。前缀文件写在**目标同目录**（`.soko-bisect-<pid>-<n>.tmp`，无 `.sokonanoda` 后缀，`finally` 删除，`:254-267`）。**不依赖诊断 span**（G-15） |
| `--ledger [路径]` | `check.py:1023-1032`；记录构造 `ledger_entry()` `:889-913` | **默认关闭**；`nargs="?"`、`const=LEDGER_DEFAULT`（`:935-937`）。相对路径按**仓库根**解析（`:1026-1028`），父目录自动建（`:1029`），`open(..., "a")` 追加一行 `json.dumps(entry, ensure_ascii=False)`（`:1030-1032`） |
| `--annotations` | `check.py:989-995` | 对每个非 ok 行打 `::error file=<相对路径>::<标签> 判负（exit N）<理由>`，**不带行号**（G-15）；`--json` 时走 stderr，否则 stdout |
| `--report <路径>` | `check.py:1011-1016` | 把**与 `--json` 同一份** report（含 bisect）以 `indent=2` 落盘；写不了只 warning 不判负 |
| `--summary <路径>` | `check.py:1017-1022` | 追加 markdown 表（`summary_markdown()` `:864-877`）到文件（CI 传 `$GITHUB_STEP_SUMMARY`） |
| `--bin <路径>` | `check.py:938-939` → `resolve_channel()` `:148-196` | 显式二进制；会**自比版本**（与 `sokonanoda-version.txt` → `Cargo.toml` 的钉），不一致 ⇒ **exit 2**（`:159-164`）。否则走 `scripts/soko version --json`，`cli.source` 含 `STALE`/`unknown`/`unverified` ⇒ exit 2（`:191-195`） |
| 退出码 | docstring `:54`；`main()` `:1064-1137` | **0** 全绿 / **1** 有目标被判负 / **2** 前置缺失或用法错误（找不到仓根 `:1069`、前置 `:1079`、`--only` 无匹配 `:1094`、无单元 `:1134`） |

### 1.5 改写后哪些判据可能变红（逐条 + 机制）

#### G1 —— 最可能的红（新符号没声明 / `by` 步不被接受）

* `∧ ∨ ↔ ¬` **不是本语言内建语法**，必须用 `infix`/`notation` 自己声明（G-04 已修，两刀+三刀都在）。
  三条硬边界（`courses/set-theory/README.md:178-186`）：
  1. **同一个符号全课程只能声明一次**（重复声明是 parse 错误）—— 库声明过的，单元不能再声明；
  2. **目标名在使用点解析** —— 用到符号时那个名字必须在作用域里，否则
     `elab-notation-unknown-target`；
  3. 一元记法在实参位要加括号。
* `→` 是内建箭头；`∀`/`∃` 走 **binder 记法**（`binder_notation`，第三刀），
  今天**只在单元⑧ 本地声明**（`courses/set-theory/units/unit08-images-preimages.sokonanoda:141`
  `binder_notation "∃" => Exists`）。要全课程统一用 `∃`，就得把声明搬到一个**所有画布都 import
  的库模块**，并**删掉单元⑧ 的本地声明**（否则重复声明 = parse 错误 ⇒ G1 红）。
* `lib/Logic.sokonanoda` 今天是**只有注释的空壳（0 条声明）**，且 `README.md:202` 明说
  「**别往这里加声明** —— 它是入口，不是库」。所以联结词记法的落点要么是 `lib/Set`
  （已声明 5 个集合符号，`:155-159`），要么是**新的 `lib/*.sokonanoda` 模块**。
  ⚠️ 新增库模块会被 `lib/*.sokonanoda` 的 glob 捡成**新目标**（`check.py:513-517`）
  ⇒ `targets` 36 → 37，且新模块必须过 G5（0 open）。
* 每个目标都是 `grade <绝对路径>`（`check.py:215`），任何解析/elaborate/内核拒绝 ⇒ G1 红。

#### G3 —— 解答留洞 / 解答退化成 `example`

* 改写把某份解答的某个 `sorry` 忘填 ⇒ `open > 0` ⇒ G3 红（`check.py:572-573`）。
* 把解答里的具名声明改成匿名 `example` ⇒ `decl.checked` 不再计入（§1.2）。
  若某份解答的具名声明**全部**变成 `example`，`checked == 0` ⇒ G3 红（`check.py:574-575`）。
* `solutions_open` 今天是 0，所以任何新洞都会立刻显形。

#### G4 —— **按名字**判定；`example` ↔ `theorem` 是最大的语义陷阱

判定代码（`check.py:580-594`）：

```python
by_path = {row["path"]: row for row in rows if row.get("path")}
for row in rows:
    if row["kind"] != "solution" or row["canvas"] is None or row["exit"] is None: continue
    canvas = by_path.get(row["canvas"])
    if canvas is None: canvas = {"exit": judge(Path(row["canvas"])).code, ...}   # --only 时补判
    if canvas["exit"] != 0: continue                # 画布自己判负 ⇒ 覆盖度无从谈起
    covered = set(row["checked_names"])
    missing = [name for name in canvas["open_names"] if name not in covered]
    if missing: row["reasons"].append("G4：解答没有覆盖画布的练习 " + "、".join(missing))
```

**结论：G4 是纯名字匹配，不是位置匹配。** 具体后果：

| 改写动作 | G4 后果 |
|---|---|
| 画布上**重命名**具名练习（`theorem foo` → `theorem bar`），解答不同步 | **红**：`open_names=["bar"]`，解答的 `checked_names` 里只有 `foo` ⇒ `missing=["bar"]` |
| `theorem foo … := by sorry` → `example … := by sorry` | **不红，但门禁变弱**：匿名 `example` 的 `exercise.open` 无 `name` ⇒ 该题**静默退出 G4 的覆盖要求**（"删题代替填洞"的防线被绕过） |
| `example … := by sorry` → `theorem foo … := by sorry` | **红（除非解答补上）**：新名字进了 `open_names`，解答必须有同名 `decl.checked` |
| 画布**新增**一道具名练习，解答不补 | **红**（这正是 G4 的设计意图） |
| 画布**删掉**一道练习（连 `sorry` 一起删） | 不红（G4 只查画布现存的名字）——但 `open` 计数下降，需同步文档/站点 |
| 改名**画布文件**但不改 `course.json` 的 `file` | 该单元行变 `missing` ⇒ **G2 红**；同时它的解答因为 `by_path.get(canvas)` 为 `None` 且补判失败（`exit != 0`）⇒ **G4 静默跳过**（`:584-590`） |
| 改名**解答文件**使 stem 不再以 `unitNN` 开头 | 配对靠 `solution_unit()` 的 `re.match(r"unit(\d+)", path.stem)`（`check.py:474-476`）与 `solutions_dir / f"unit{number:02d}-solution.sokonanoda"`（`:544`）⇒ 预期路径不存在 ⇒ **G2 红**（`design §9.2` 第 4 条明说"配对按文件名里的 `unitNN`，不是 `<画布名>-solution`"） |

> ⚠️ **判据本身不会因为 `example` 化而变红**，但它会**悄悄失去防护**。
> 改写时如果把"练习"从 `theorem name := sorry` 改成 `example := sorry`，
> 请在评审里显式确认这是有意的（否则 G4 从"覆盖检查"退化成"什么都不查"）。

#### G5 —— lib 里加 `sorry`

`lib/` 9 个文件今天全 0 open。改写若往 `lib/` 里放演示用的 `sorry` ⇒ **红**（`check.py:576-577`）。

#### G6 —— 清单结构

* 改 `chapter.id`（例如 `I.1` → `1.1`）**必须同步所有引用它的 `prereqs`**
  （今天：`course.json:46` `I.2 → ["I.1"]`、`:73` `I.3 → ["I.2"]`、`:100` `I.4 → ["I.1","I.3"]`），
  否则 **G6 红**（`check.py:426-429`）。
* 增删/合并/拆分层 ⇒ chapter id 重复或 unit 重复 ⇒ **G6 红**（`:363-364`、`:380-381`、`:399-405`）。
* 单元条目缺 `file`/`unit` ⇒ **红**（`:393-398`）。
* `schema` 改成别的值 ⇒ **exit 2**（`:334-339`）。

#### 配额（`quota.exercises`）——**永不判红**

* 今天 `course.json:15` `I.1=35`、`:48` `I.2=24`、`:75` `I.3=23`、`:102` `I.4=14`（合计 **96**）。
* 实测 `canvas_open` = 96 ⇒ 四章差额**全为 0**（本次 `--json` 的 `quota_notes` 逐条印证：
  `chapter I.1 计划练习 35 · 画布实测 35（差额 0）` …）。
* 改写增删练习后差额会变（可能为负），**只出现在报告与 `--summary` 里**，退出码不变
  （`check.py:20-21`、`:433-461`、`:968-974`、`:1051-1054`）。
* ⚠️ **但 `quota` 会被两处硬断言碰到**：`crates/cli/tests/course_manifest.rs:320-323`
  要求每章 `quota.exercises > 0`；`docs/gaps/repro/G07-course-manifest-v2.sh:59-68` 要求
  `quotas=4`（四章都有 quota）。**把 quota 删掉或改成 0 ⇒ 两处都红**。

---

## 2. 硬编码的期望计数：代码里没有，文档/站点/台账里有一堆

### 2.1 会被门禁/CI **强制**的（真正的硬契约）

| 位置 | 钉住什么 |
|---|---|
| `docs/gaps/repro/G07-course-manifest-v2.sh:68` | `schema=soko.course/2 volumes=1 chapters=4 units=12 prereqs=4 tags=4 quotas=4` —— **对真课程 `course.json` 的逐字断言**（`:31 COURSE_JSON="$PWD/courses/set-theory/course.json"`） |
| 同上 `:84-85` | `"volumes":1`、`"chapters":4`（真课程 CLI 事件） |
| 同上 `:146` | 门禁报告 `volumes=1 chapters=4 units=12 rejected=0` |
| `crates/cli/tests/course_manifest.rs:338` | `assert_eq!(units, 12, "卷 I 的 12 个单元一个都不能丢")` |
| `crates/cli/tests/course_manifest.rs:339` | `assert!(chapters >= 4, …)` |
| `crates/cli/tests/course_manifest.rs:316-323` | 每章 `quota.exercises > 0` |
| `crates/cli/tests/query.rs:569-570` | 单元⑤ `decl_checked == 5`、`exercise_open == 7` |
| `crates/cli/tests/notation.rs:308/312` | 单元② 两行签名的**逐字文本** |
| `crates/cli/tests/notation.rs:384` | 单元② `open >= 8` |
| `crates/cli/tests/notation.rs:532-544` | `lib/Set.sokonanoda` 里 5 行记法声明的**逐字文本** |
| `crates/cli/tests/notation.rs:556/563` | 单元③ 含 `"𝒫 "`、单元⑧ 含 `"'' "` |
| `crates/cli/tests/notation.rs:776/780` | 单元⑧ 含 `"binder_notation"` 与 `"∃ ("` |
| `crates/cli/tests/notation.rs:582` | 单元② 含 `"Set.subset α A C"`，且**不得**有以 `infix`/`notation` 开头的行 |

### 2.2 只是散文（会变成假话，但没有东西会红）

**课程自身**

* `courses/set-theory/README.md:126`、`:133` —— `36 个目标 · 329 checked · 99 open · 0 判负`
* `courses/set-theory/README.md:140-143` —— 四章的 `计划练习 / 画布实测` 表（35/35、24/24、23/23、14/14）
* `courses/set-theory/README.md:145-158` —— 12 个单元的「练习 / 解答（checked）」表
* `courses/set-theory/README.md:163` —— 记法对照页 `18 checked + 3 练习 / 解答 21 checked`
* `courses/set-theory/README.md:192-195` —— `lib/` 逐模块计数
* `courses/set-theory/lib/Set.sokonanoda:140` —— `-- 36 目标 / 329 checked / 99 open 逐项不变）。`

**仓库根**

* `STATUS.md:51`、`:56`、`:123-124`、`:132`、`:236`、`:255`（329/99/36）；`:189`（**355 checked**，已过期）
* `docs/HANDOVER.md:7`、`:440`（329/99/36）；`:403`（**355**，已过期）
* `REQUIREMENTS.md:1670`、`:1687`、`:1751`（329/99/36）；`:1727`（**355**，已过期）

**设计文档**

* `docs/design/course-gate-in-ci.md:28-29`（**34 目标 · 296 checked · 93 open**，最旧）、`:95`、`:156`、`:168`、`:171`、`:236`（**34 · 315 · 96**）、`:246`、`:266`（36/329/99/20024ms/v0.60.0）
* `docs/design/course-stdlib.md:63`、`:292-293`、`:349`
* `docs/design/teaching-project.md:457`（**355**）、`:463`、`:520`、`:527`
* `docs/design/notation-subset.md:409`、`:498`、`:507`、`:675`、`:692`
* `docs/design/namespace-open.md:413`、`:419`、`:501`
* `docs/design/eq-type-level-rewriting.md:228`
* `docs/design/course-manifest-v2.md:241`、`:243`、`:260`、`:263`、`:315`、`:318-319`、`:325`、`:348`、`:351`、`:362`
* `docs/design/prelude-l1-proposal.md:44`、`:338`
* `docs/design/site.md:81`（`12 单元表`）
* `docs/TESTING.md:100`（`36 个目标不重复解析启动器` —— 与 `scripts/soko:912` 的 `34 targets` 同族，都是**过期注释**）
* `docs/design/course-manifest-v2.md:231`、`:245`、`:275`、`:318`、`:355`（`test_manifest_v2.py` 的复跑记录）

**站点**

* `site/data/site.json:13`（`counts_source: "previous-run"`）、`:16`（`measured_at`）、
  `:18`（`"checked": 329`）、`:19`（`"failed": 0`）、`:20`（`"open": 99`）、`:21`（`"targets": 36`）、
  `:139/:190/:233/:277`（四章 `quota` 35/24/23/14）
* `site/set-theory.html:60`（**写死了单元①的文件名** `units/unit01-sets-membership.sokonanoda`）、
  `:75`（目录布局）、`:80-81`（手写的章节序列）、`:91`、`:117`、`:134-138`
* `site/course.html:32`、`:34`、`:35`（写死"单元⑦ / 单元①"）
* `site/index.html:70`、`site/get-started.html:50`、`site/en/index.html:71`（`12 道练习` —— 讲的是
  `playground.sokonanoda`，不是卷 I，但也**不生成、不校验**）
* **好消息**：`site/set-theory.html` 的**所有数字**都是运行时从 `data/site.json` 填的
  （`:274-278`），页面自己不写死任何课程计数（`:143`「官网永不手写计数」）。

**成本台账**

* `docs/courses/ledger.jsonl:1` —— `targets 36 / checked 329 / open 99 / rejected 0`（历史行，**只判形状**）

---

## 3. `courses/set-theory/course.json`（清单 v2）与 `tools/test_manifest_v2.py`

### 3.1 清单 v2 的结构

```
{ "schema": "soko.course/2", "name": "set-theory", "title": "卷 I《集合论》",
  "volumes": [ { "id": "I", "title": "卷 I 集合论", "chapters": [
    { "id": "I.1", "title": …, "prereqs": [], "tags": [...],
      "quota": { "exercises": 35 },
      "units": [ { "file": …, "title": …, "title_en": …, "unit": 1 }, … ] }, … ] } ] }
```

* `course.json:1-121`；1 卷 / 4 章 / 12 单元。
* 章 id 与先修：`I.1`（`:11-15`，prereqs `[]`）、`I.2`（`:44-48`，`["I.1"]`）、
  `I.3`（`:71-75`，`["I.2"]`）、`I.4`（`:98-102`，`["I.1","I.3"]`）。
* 标签：`membership/subset/powerset`、`ordered-pair/product/relation/function`、
  `image/preimage/cardinality/cantor`、`universe/russell/synthesis`。
* **`quota` 今天 = 35 / 24 / 23 / 14**（`:15`、`:48`、`:75`、`:102`）。
* 语义与判红关系见 `courses/set-theory/README.md:111-120` 的表
  （`quota.exercises` 一行写的是「**否，永不**：差额只报告」）。

### 3.2 `tools/test_manifest_v2.py`（387 行）

* 用 `importlib` 直接加载 `check.py` 的模块对象（`:40-45`），喂**临时课程目录** + 假通道，
  所以**秒级、零工具链**（docstring `:10-14`）。
* **12 个用例**（`CASES`，`:353-366`）：展平与 v1 一致（`:142`）、v2 保持 v1 形状（`:160`）、
  重复 chapter id（`:176`）、重复 volume id（`:190`）、unit 挂两章（`:205`）、
  prereqs 悬空（`:218`）、缺 file/unit（`:231`）、**配额差额只报告**（`:247`）、空章合法（`:271`）、
  v1 无 G6（`:283`）、未知 schema 走 Prerequisite（`:292`）、**成本台账字段 + 已提交台账逐行合法**（`:304`）。
* **关键：除了最后一个用例，全部用临时夹具，不读真课程** ⇒ **课程改写不会让这个单测变红**。
  唯一碰仓库的是 `case_cost_ledger_fields_and_committed_file`（`:328-350`）：
  它读 `docs/courses/ledger.jsonl`，逐行判「合法 JSON + `LEDGER_FIELDS` 齐全 + `schema` 对 +
  `date` 形状 + `version` 形状（`\d+\.\d+\.\d+`，**故意不与当前版本钉比较**）+ `elapsed_ms` 非负」。
* ⚠️ **这个单测没有接进 `scripts/soko gate`，也没有接进 CI**（全仓 grep：只有
  `check.py:82` 的注释、`docs/TESTING.md:67/70`、`docs/design/*` 提到它）。
  它是**手动**跑的（`courses/set-theory/README.md:18`）。所以**它红了不会挡住任何人** ——
  改写收尾请显式跑一次。

### 3.3 配额差额：判红还是只报告？

**只报告。** 三处证据：`check.py:20-21`（设计原则）、`check.py:433-461`（`quota_notes()`
返回 `notes` 列表）、`check.py:968-974`（异常吞掉，`quota` 只进 `report["quota_notes"]`
与人读输出 `:1051-1054`）。`--selftest` ④ 与 `test_manifest_v2.py::case_quota_difference_is_reported_not_rejected`
（`:247-268`）专门钉这条：配额 99 vs 实测 0 ⇒ **不判红，但必须出现在报告里**。

---

## 4. 站点数据

### 4.1 `site/data/site.json` 的 `set_theory` 块怎么生成

`scripts/gen-site-data.py`（473 行）：

* 读：`Cargo.toml`（版本，`:66-73`）、`course/course.json`（入门课，`:164`）、
  `courses/set-theory/course.json`（卷 I 清单，`:287`，`SET_THEORY_DIR` = `:47`）、
  **课程门禁 stdout**（`:220` `run([python3, gate, "--json"])`，`GATE_TIMEOUT_S = 300` `:52`）、
  `scripts/soko doctor --json`（前置探针 `:211`）、`STATUS.md`（轮次标题 `:406-421`）、
  `examples/*.sokonanoda`（`:433-434`）、上一版 `site/data/site.json`（沿用分支 `:278`）。
* 写：**只有 `site/data/site.json`**（`:462-469`，`indent=2, sort_keys=True`）。
* `totals` **实测**自门禁 `summary`（`:255-260`）：`targets/checked/open` 直取，
  `failed` 取 `summary["rejected"]`；没有 `summary` 时按门禁 `targets` 列表求和（`:263-266`，
  注释 `:246-249` 说明**故意不用去重的 `rows`**，因为门禁把 `lib/Demo` 判两次）。
* 逐单元 `status/checked/open` 实测（`:238-245` → `:327-329`）。
* `volumes[].chapters[].quota` **不是实测**，逐字来自 `course.json`（`:133-134`、`:347`），
  页面标为「计划练习」（`site/set-theory.html:244`）。
* 三条分支：`:295` `counts_source="gate"`（实测）/ `:311` `"previous-run"`（沿用上次）/
  `:320` `"none"`（不带计数）。`:270-271` 还有一道护栏：全部判负且零 checked ⇒ 拒绝发布。
* **当前状态**：`site/data/site.json:13` 是 **`"previous-run"`** —— 上一次提交时**没测成**
  （CI 里 `gen-site-data.py:200` 强制 `SOKONANODA_OFFLINE=1`，`:211-218` 前置探针失败就放弃实测；
  pages runner 没有 cargo 构建也没有缓存，`doctor` 必然 exit 3 ⇒ 走沿用分支）。
  数字本身仍准确（与最后一次真测的 `a948f65` 相同），但**来源已降级**。

### 4.2 `python3 scripts/check-site.py` 检查什么

> ⚠️ 任务里写的 `site/check-site.py` **不存在**；真实路径是 **`scripts/check-site.py`**（114 行）。

它只做**卫生检查**：

1. `site/**/*.html` 能被 HTML 解析（`:46-47`、`:78-84`）；
2. 站内 `href`/`src` 都能解析到真实文件（`:40-43`、`:50-65`、`:86-89`）；
3. 站点 HTML 里**不许出现写死的版本号** `0.x.y`（`:27` `VERSION_RE`，`:92-97`）；
4. `site/data/site.json` **存在**（`:99-101`）。

**它从不读 `site.json` 的内容、也从不重算任何课程计数** ⇒ **它无法发现过期的
`set_theory` 块**。而且它只在 `pages.yml:85-87` 里跑（且 `if: steps.gate.outputs.enabled == 'true'`），
**不在 `ci.yml`、不在 `release.yml`、不在 `scripts/soko gate`**。

历史教训可对照：`docs/LESSONS.md:485-498`（「网站 round 静默停在 97，生成器 exit 0、
`check-site.py` 也全绿」）。

### 4.3 改写后必须跑哪条命令重新生成

**顺序（缺一不可）**：

```bash
python3 courses/set-theory/tools/check.py --selftest     # 1 判据通道自检
python3 courses/set-theory/tools/check.py                # 2 必须 exit 0；站点计数的唯一来源
python3 scripts/soko doctor --json                       # 3 必须 "ready": true（否则第 4 步静默降级！）
python3 scripts/gen-site-data.py                         # 4 控制台必须打印「门禁实测」，不是「沿用上次实测的计数」
python3 scripts/check-site.py                            # 5 必须 "site hygiene: ok"
git add site/data/site.json && git commit                # 6 提交（CI 只会重新发布这一份）
```

**跳过第 3/4 步的真实后果**：`gen-site-data.py` 会**欢快地 exit 0** 并打印
`wrote site/data/site.json`，但块变成 `counts_source: "previous-run"`、`totals` 沿用旧值，
**新/改名的单元文件完全没有计数字段** ⇒ `site/set-theory.html` 对它们渲染 `—`
（`:210-213`）且**不把它们算进 `stat-exercises`**（`:205-209`）⇒
**标题数字讲旧课程、表格列新课程**，而 `check-site.py` 依然全绿。

**另有一个触发器缺口**：`pages.yml:23-31` 的 `paths` 只有
`site/**`、`scripts/gen-site-data.py`、`scripts/gen-site-demos.py`、`scripts/check-site.py`、
`STATUS.md`、`course/course.json`、`Cargo.toml`、`.github/workflows/pages.yml` ——
**没有 `courses/set-theory/**`**（连 `courses/set-theory/course.json` 也没有）。
⇒ 只改课程**不会触发任何部署**。建议同轮给 `pages.yml` 的 `paths` 补
`"courses/set-theory/**"`（否则只能靠 `workflow_dispatch` 手动触发）。

---

## 5. `docs/courses/ledger.jsonl`（成本台账）

* **是什么**：每次门禁跑完的**机器可读成本记录**，一行一条。设计
  `docs/design/course-manifest-v2.md` §4.6；schema `soko.course-ledger/1`（`check.py:78`）。
* **谁生成**：`check.py --ledger [路径]`（默认 `docs/courses/ledger.jsonl`，`check.py:79`）。
  **默认关闭** —— CI 不往仓库里写文件（`check.py:23-27`、`:935-937`），只有**人工收尾**跑一次。
* **字段**（`LEDGER_FIELDS`，`check.py:83-84`；构造 `ledger_entry()` `:889-913`，键顺序固定）：

  ```json
  {"schema":"soko.course-ledger/1","version":"0.60.0","commit":"<git HEAD>",
   "date":"2026-09-19T10:18:26Z","course":"set-theory",
   "targets":36,"checked":329,"open":99,"rejected":0,"elapsed_ms":20024,
   "solutions_open":0,
   "rows":[{"label":"lib Demo","status":"ok","checked":10,"open":0}, …]}
  ```

  * `version` = 仓库版本钉（`repo_pin()` `:128-145`：`sokonanoda-version.txt` → `Cargo.toml`）；
  * `commit` = `git rev-parse HEAD`（`:880-886`）；
  * `date` = UTC ISO-8601 秒级（`:902`）；
  * `course` = v2 清单的 `name`，否则课程目录名（`:1025`）；
  * `elapsed_ms` **只量判卷那一段**（`:1075-1076`、`:1136`），不含通道解析/版本探针；
  * `rows` 有 `label/status/checked/open`，但**不在 `LEDGER_FIELDS` 里**（缺了也能过校验）；
  * `lib_open` / `canvas_open` 算了但**不序列化**（`:954-955`）。
* **格式**：`open(path,"a")` 追加一行 `json.dumps(entry, ensure_ascii=False)` + `\n`
  （`:1030-1032`）；相对路径按仓库根解析（`:1026-1028`），父目录自动建（`:1029`）。
  **不要求日期单调/排序**（与 `docs/e2e/ledger.jsonl` 不同，后者 `scripts/e2e-merge.py:123-125`
  强制升序）。
* **谁校验**：**只有** `courses/set-theory/tools/test_manifest_v2.py:304-350`
  （逐行 JSON/字段/`schema`/`date` 形状/`version` 形状/`elapsed_ms`）。
  `check.py --selftest` ⑤ 只校验**合成记录**（`:805-817`），不读文件。
  **CI 不校验它**（`ci.yml:190-194` 跑的是 `--selftest` + 全量门禁，**没有 `--ledger`**）。
* **要不要在改写后追加一条**：**要**（这是本仓库的收尾惯例：发版/里程碑时人工跑一次，
  把那一行提交进仓库当趋势点 —— `courses/set-theory/README.md:39-41`、
  `docs/design/course-gate-in-ci.md:264-267`）。命令：

  ```bash
  python3 courses/set-theory/tools/check.py --ledger
  python3 courses/set-theory/tools/test_manifest_v2.py   # 逐行合法性（手动，CI 不跑）
  ```
* 现有唯一一行：`docs/courses/ledger.jsonl:1`（`36/329/99/0 · 20024ms · v0.60.0 · commit 952b0f1`）。
  追加后**旧行保持原样**（历史就是历史；`version` 只判形状，允许与当前钉不同）。

---

## 6. 缺口台账（`docs/gaps/ledger.jsonl`）与 `scripts/gap.py`

### 6.1 24 条的现状

**22 `fixed` + 2 `workaround`**（`L-04`、`L-06`）。与课程/记法/tactic 相关的条目：

| id | status | fixed_in | 与本次改写的关系 |
|---|---|---|---|
| **G-04** | fixed | 0.59.0 | 没有 notation/infix。三刀全落地（`infix`/`infixl`/`infixr`/`notation`、`prefix`/`postfix`、binder 记法/重载/`scoped`/集合字面量）。**`∧ ∨ ↔ ∃ ∀ ¬ →` 全靠它** |
| **G-05** | fixed | 0.60.0 | `namespace`/`open`（含课程 `lib/Set` 迁移）。notes 里明确记录「课程门禁改前/改后同计数（36 目标 / 329 checked / 99 open / 0 判负）」 |
| **G-07** | fixed | 0.60.0 | 清单 v2。**它的复现件把真课程钉死**（§6.3） |
| **G-06** | fixed | 0.59.0 | `course` 认 `import`。复现夹具自包含（`docs/gaps/repro/G06-course-import/`） |
| **G-12** | fixed | 0.59.0 | 相对路径 + 祖先清单。notes 提到 `courses/set-theory/course.json` |
| **L-01 / L-02** | fixed | 0.59.0 | prelude 逻辑骨架 / Eq 引理。notes 记录课程 `lib/Logic` 空壳化 + 65 处 `inl`/`inr` → `Or.inl`/`Or.inr`，**改前/改后 329/99 不变** |
| **L-05** | fixed | 0.59.0 | 「内容型」引理误放进库（16 条）——应当是练习。**这条是"L2 库 vs L3 练习"的分界纪律，改写时最容易再犯** |
| **L-04** | **workaround** | — | 课程标准库缺 `Set` 的「定义展开」引理。绕法 = `lib/Set.sokonanoda` 自带。**没有复现文件 ⇒ `gap.py check` 跳过它** |
| **L-06** | **workaround** | — | 语言没有累积性 + `Exists.elim` 的 `Q` 只能是 Prop。**没有复现文件 ⇒ 跳过** |
| G-01 | fixed | 0.59.0 | 开练习签名受检。带 `repro_expect: "rejected"` |
| G-02 | fixed | 0.59.0 | 构造子命名空间。**`lib/Prod` 的构造子仍是裸名 `prod_mk`**（唯一残留） |
| G-03 | fixed | 0.59.0 | `Exists` 已是真归纳 |
| G-14 | fixed | 0.59.0 | 单宇宙 binder（`congrArg`/复合/像的跨宇宙版本写不出来） |
| G-18 | fixed | 0.59.0 | `def f.{u}` 被静默解析成 `f.` |

**`tactic` / `demo` / `gate` 在 24 条里零命中** —— 今天没有任何一条缺口与 tactic 模式有关。

### 6.2 改写会不会把某条从 `workaround` 翻成 `fixed`（或反过来）？

**不会。** 本次改写是**纯课程内容**（符号拼写 + 证明写法），不动语言。`L-04`/`L-06` 描述的是
**语言**的缺失（库缺展开引理、内核无累积性），改写既不修语言也不删库，所以：

* 两条 `workaround` **保持 `workaround`**；
* 22 条 `fixed` **保持 `fixed`**；
* ⚠️ **但反过来要小心**：如果改写过程中撞到**新的语言边界**（例如某个 `∧`/`∃` 组合在 `by`
  块里 elaborat 不了、tactic 对记法版目标的 `apply` 文本对齐问题 —— 后者 `G-05` 的 notes
  已经记过「`by` 引擎的 `apply` 按文本对齐，源里短名与内核 pp 全名不同」），
  那就**必须新登记一条缺口**（`gaps/` 记一条 + 收编进 `docs/gaps/ledger.jsonl` +
  最小复现件），否则违反 `courses/set-theory/AGENTS.md:13-14` 的写作循环第 5 步。

### 6.3 ⚠️ `docs/gaps/repro/G07-course-manifest-v2.sh` 把真课程钉死了

这是**最容易踩的一脚**。该脚本（158 行）的语义是**反的**：
`exit 1` = 缺口已修（正常），`exit 0` = 「缺口回来了」。它逐字断言真课程：

* `:31` `COURSE_JSON="$PWD/courses/set-theory/course.json"`
* `:68` `*"schema=soko.course/2 volumes=1 chapters=4 units=12 prereqs=4 tags=4 quotas=4"*)`
* `:84-85` `grep -q '"volumes":1'` / `grep -q '"chapters":4'`
* `:146` `grep -q 'volumes=1 chapters=4 units=12 rejected=0'`

⇒ **只要改写动了 `course.json` 的单元数（≠12）、章数（≠4）、先修/标签/quota 的个数（≠4），
或让门禁 `rejected != 0`，脚本就 exit 0，`gap.py check` 报「台账写的是『行为已变』」并
exit 1 ⇒ `scripts/soko gate` 与 CI 的 `Gap ledger is consistent` step 一起红。**

**处置**：若改写只动单元**内容**（不动清单结构），无需动它；若动了结构，
**同轮更新该脚本的四个断言**（并把注释 `:22`、`:41`、`:120`、`:149` 的措辞一起改）。

### 6.4 `scripts/gap.py check` 的契约

* `check`（`scripts/gap.py:224-246`）**不校验必填键、不校验 status 枚举、不查重复/排序**。
  逐条：没有 `repro` 字段 ⇒ 跳过（`:230-232`）；否则跑复现（`run_repro()` `:81-105`）再判（`judge()` `:108-141`）。
  * `.sh` ⇒ `bash <path>`，`cwd = 仓库根`，环境经 `clean_env()` **剔除 `SOKONANODA_BIN`/`SOKONANODA_LSP_BIN`**（`:90-92`、`:66-79`）；
  * `.sokonanoda` ⇒ `scripts/soko grade <绝对路径>`，判据 `clean = exit 0 and 有 "type":"decl.checked" and 无 "type":"diagnostic"`（`:93-102`）；
  * `missing`/`dir`/`none`/`unknown` ⇒ 跳过，**不算失败**（`:103-105`、`:234-236`）。
* `judge()` 的期望**默认由 `status` 推出**：`fixed` ⇒ `.sokonanoda` 应「已判卷通过」、
  `.sh` 应「行为已变」（`code != 0`）；非 `fixed` 反之。**`repro_expect` 显式覆盖**：
  `clean`/`rejected`（只对 `.sokonanoda`）、`exit0`/`nonzero`（只对 `.sh`）；
  取值与类型不匹配 ⇒ 直接判不一致并印出原因（`:126-127`、`:138-139`）。
* 退出码：**0** 全部一致（`:245-246`）/ **1** 有偏差（`:241-244`）/ **2** 台账读不了或非 JSON 行
  （`load()` `:41-43`、`:50-52`）或 argparse 用法错误。**`gap.py` 没有 exit 3**。
* 锐边：`e['id']` 无保护取值（`:231`、`:235`、`:239`）⇒ 缺 `id` 会 KeyError；
  `status` 用 `.get("status","?")` ⇒ **拼错的 status 会被当成"非 fixed"**，于是要求复现仍然失败。
* `selftest`（`:273-328`）：14 条硬编码 judge() 用例 + `clean_env()` 剔除覆盖，**不碰台账、不判卷**。
* `close <id> --version <v> [--note …]`（`:249-270`）：先按「假如已 fixed」预判复现（`:256`），
  不一致就**拒绝**（`:257-261`）；然后写 `status="fixed"`、`fixed_in=<v>`，并把 `--note`
  以 `｜` 追加进 `notes`（`:262-265`），最后**整份重写**（`save()` `:56-58`，紧凑分隔符）。
* **强制执行**：`scripts/soko gate` 第四步跑 **`check`（不跑 `selftest`）**（`scripts/soko:933-943`）；
  CI `.github/workflows/ci.yml:207-217` 跑 **`selftest` + `check`** 两步。
  ⚠️ 文档漂移：`docs/design/course-gate-in-ci.md:205` 说 gate 第四步是「`gap.py selftest` + `check`」，
  但代码只跑 `check`（`scripts/soko:935`）。
* **改 `status` 时要一起改什么**（若真发生）：
  `status: "fixed"` + `fixed_in: "<version>"`，且**复现件必须翻成"修后形状"**
  （`.sokonanoda` 要判绿；`.sh` 要 exit≠0，脚本自己的断言也要改成修后应有的形状）。
  「修好 = 必须判红」的情况写显式 `repro_expect: "rejected"`（G-01 就是这么办的）。
  若复现件无法表达修后状态，**撤掉 `repro` 字段**（G-09 先例）。正确做法是用
  `python3 scripts/gap.py close <id> --version <v> --note "…"` 而不是手改。

---

## 7. 必须同轮更新的文档 / 元数据清单

| # | 文件 | 要改什么 | 怎么验证 |
|---|---|---|---|
| 1 | `courses/set-theory/units/*.sokonanoda`（13）+ `units/solutions/*`（13）+ `lib/*`（9） | 改写本体（符号 + `by`） | `python3 courses/set-theory/tools/check.py` ⇒ exit 0 |
| 2 | `courses/set-theory/course.json` | 仅在**改结构**时动（单元名/章 id/prereqs/quota）。⚠️ 保持 1 卷 / 4 章 / 12 单元 / 4 prereqs / 4 tags / 4 quotas，否则 §6.3 + §8 两处红 | `python3 courses/set-theory/tools/check.py --json`（看 `course`、`quota_notes`） |
| 3 | `courses/set-theory/README.md` | `:126`、`:133`（计数）、`:140-143`（配额/实测表）、`:145-158`（逐单元表）、`:163`（记法页计数）、`:192-195`（lib 逐模块计数）；若符号/`by` 写法变了，§「记法（G-04 第二刀）」`:165-190` 也要改 | 与 `--json` 的 `summary`/`targets` 逐项对照 |
| 4 | `courses/set-theory/AGENTS.md` | 写作循环第 2 步「演示（已证）+ 练习（`sorry`）」的措辞；若规定 tactic 白名单要加一条 | 人读；`crates/cli/tests/skill.rs` **不检查** `courses/` 前缀（`PATH_PREFIXES` `:67-74` 只有 `docs/ course/ crates/ examples/ skills/ scripts/`） |
| 5 | `courses/set-theory/lib/Set.sokonanoda:140` | 注释里的 `36 目标 / 329 checked / 99 open` | 人读 |
| 6 | **`crates/cli/tests/notation.rs`**（6 个测试） | 见 §8。**这是最硬的挡路石** | `cargo test -p sokonanoda-cli --test notation --locked` |
| 7 | **`crates/cli/tests/query.rs:569-570`** | 单元⑤ `decl_checked 5` / `exercise_open 7`（若计数变了） | `cargo test -p sokonanoda-cli --test query --locked` |
| 8 | `crates/cli/tests/course_manifest.rs:338-339`、`:316-323` | 仅在动清单结构时（12 单元 / ≥4 章 / quota>0） | `cargo test -p sokonanoda-cli --test course_manifest --locked` |
| 9 | **`docs/gaps/repro/G07-course-manifest-v2.sh`** | 仅在动清单结构时（`:68`、`:84-85`、`:146` 的 12/4/4/4/4） | `python3 scripts/gap.py check` ⇒ exit 0 |
| 10 | `site/data/site.json` | 重新生成（§4.3） | `python3 scripts/soko doctor --json` ⇒ ready，然后 `python3 scripts/gen-site-data.py` 控制台必须出现「门禁实测」 |
| 11 | `site/set-theory.html:60` | 若单元① 改名，这行命令示例要跟着改 | 人读 + `python3 scripts/check-site.py` |
| 12 | `site/set-theory.html:80-81`、`site/course.html:35` | 手写的章节/单元序列（若大纲顺序变了） | 人读 |
| 13 | `.github/workflows/pages.yml:23-31` | **建议补 `"courses/set-theory/**"`**（否则改课程不触发部署） | `python3 -c "import yaml;yaml.safe_load(open('.github/workflows/pages.yml'))"` |
| 14 | `docs/courses/ledger.jsonl` | `python3 courses/set-theory/tools/check.py --ledger` 追加一条并提交 | `python3 courses/set-theory/tools/test_manifest_v2.py` ⇒ exit 0 |
| 15 | `STATUS.md` | 最新一轮（只留最近 3 轮）：`:51`、`:56`、`:123-124`、`:132`、`:236`、`:255` 的计数；`:189` 的 355 是**旧轮归档**（移到 `docs/STATUS-ARCHIVE.md`） | `python3 scripts/gen-site-data.py` 会读它的标题（`:406-421`） |
| 16 | `REQUIREMENTS.md` §9 | 追加本轮用户要求条目（AGENTS.md 收尾义务）；`:1670`、`:1687`、`:1751` 的计数 | 人读 |
| 17 | `docs/HANDOVER.md` | `:7`（快照行）、`:440` 的计数 | 人读 |
| 18 | `docs/design/course-stdlib.md` | `:63`、`:292-293`、`:349` 的计数；若 L2/L3 分界有新判断，§1/§3 也要动 | 人读 |
| 19 | `docs/design/teaching-project.md` | `:457`、`:463`、`:520`、`:527` 的计数；§5 DoD 第 2/5 条 | 人读 |
| 20 | `docs/design/course-gate-in-ci.md` | `:266` 的台账条目；`:173`/`:186` 的 **G1–G5 → G1–G6** 漂移（见 §9） | 人读 |
| 21 | `docs/design/notation-subset.md` | `:409`、`:498`、`:507`、`:675`、`:692` 的「课程零改动」契约 —— **本次改写正是它说的"后续单独一轮的对照实验"**，§12/§13 需要 as-built 追加 | 人读 |
| 22 | `docs/design/course-manifest-v2.md` | `:241`、`:243`、`:260`、`:263`、`:315`、`:318-319`、`:325`、`:348`、`:351`、`:362` | 人读 |
| 23 | `docs/design/site.md:81` | `12 单元表` | 人读 |
| 24 | `docs/TESTING.md:67`、`:70`、`:100` | 计数与 `36 个目标` | 人读 |
| 25 | `.github/workflows/ci.yml:173`、`:186` | **G1–G5 → G1–G6**（同 §9 的漂移） | `python3 -c "import yaml;yaml.safe_load(open('.github/workflows/ci.yml'))"` |
| 26 | `scripts/soko:912` | 注释 `34 targets` → 实际目标数 | 人读 |

> 若本轮**首次**出现 CI 红，按 `AGENTS.md`「CI 失败记录」在 `docs/CI-FAILURES.md` 追加一条。

---

## 8. 会挡住改写的自动化测试（逐条）

### 8.1 先澄清最大的误会：**两处 GOLDEN 与 `courses/set-theory` 无关**

| GOLDEN | 位置 | 钉的是哪个 fixture |
|---|---|---|
| `course.rs::GOLDEN` | `crates/cli/tests/course.rs:86-98`（11 行 `(checked, open, reduced)`） | **入门课 `course/`**：`course.rs:14 const COURSE_DIR = concat!(env!("CARGO_MANIFEST_DIR"), "/../../course")` |
| `course_status.rs::GOLDEN` | `crates/cli/tests/course_status.rs:68-80` + 汇总 `:117-120` | **入门课 `course/course.json`**：`course_status.rs:13` |
| 第三处（"双 GOLDEN"之外） | `crates/cli/tests/cli.rs:2097-2098`（`checked 86` / `open 65`） | **入门课**：`cli.rs:8` |

`docs/gaps/WO-009-single-universe-binder.md:157` 已经写明：
「**`courses/set-theory` 根本没进这两个 GOLDEN**」。`course/`（单数，入门课 11 单元）
与 `courses/`（复数，卷 I 12 单元）是**两棵不同的树** —— 这是本次审计最容易踩的坑。
⇒ **只改 `courses/set-theory/`，这三处 GOLDEN 全部不受影响。**

### 8.2 会被改写打红的测试（**8 个**）

| # | 测试 | 钉住什么（`file:line`） | 会不会红 |
|---|---|---|---|
| 1 | `notation.rs::a_real_course_unit_grades_identically_in_notation`（`:357`） | 读单元②（`:280`、`:360-361`）；`notation_variant()`（`:287-324`）要求两行签名**各恰好出现一次**：`:308` `"    (h1 : Set.subset α A B) (h2 : Set.subset α B C) : Set.subset α A C :="`、`:312` `"theorem empty_subset (α : Type) (A : Set α) : Set.subset α (Set.empty α) A :="`；`:384` `assert!(open >= 8)` | **红** —— 签名改写成 `⊆`/`∅` 或重排（`by` 块）都会让 `assert_eq!(count, 1)` 失败 |
| 2 | `notation.rs::the_shipped_course_still_uses_the_pointful_spelling`（`:567`） | 单元②**不得**有以 `infix`/`notation` 开头的行（`:573-580`）；`:582` `unit.contains("Set.subset α A C")` | **红** —— 它的注释 `:568-570` 自己说「真要改课程时，请连同本测试一起更新，并同时改 `docs/design/notation-subset.md`」 |
| 3 | `notation.rs::the_shipped_course_library_declares_the_five_symbols`（`:527`） | `lib/Set.sokonanoda` 必须含 5 行**逐字**声明（`:532-544`），即 `courses/set-theory/lib/Set.sokonanoda:155-159` 的 `prefix:100 " 𝒫 " => Set.powerset` / `postfix:100 " ᶜ " => Set.compl` / `infixr:80 " '' " => Set.image` / `infixr:80 " ⁻¹' " => Set.preimage` / `infixr:80 " ×ˢ " => Set.prod` | **红**（若记法块搬家或改形） |
| 4 | `notation.rs::the_shipped_course_uses_the_library_notation_in_a_demo`（`:548`） | 单元③ 含 `"𝒫 "`（`:556`，单元③:60-62）、单元⑧ 含 `"'' "`（`:563`，单元⑧:129-131） | **红**（若这两条 `example` 演示被改掉） |
| 5 | `notation.rs::the_shipped_course_demos_the_binder_notation`（`:768`） | 单元⑧ 含 `"binder_notation"`（`:776`）与 `"∃ ("`（`:780`）—— 对应单元⑧:141 `binder_notation "∃" => Exists`、`:143-145` `∃ (x : α), …` | **红** —— **`∃` 符号化与这条正面冲突**：若把 `binder_notation` 搬到库里并删掉单元⑧ 的本地声明，这条必红 |
| 6 | `notation.rs::a_library_notation_works_in_the_entry_through_import`（`:451`） | `stage_course_unit()` 把真树 `courses/set-theory/{lib,sokonanoda.toml}` 拷到 `/tmp`（`:328-341`，`:329`），入口只 `import lib.Set` 就用 `𝒫 A` / `Aᶜ`（`:456-464`） | **红**（若 `lib/Set` 不再导出这些符号） |
| 7 | `query.rs::query_check_matches_grade_on_a_real_course_unit`（`:559`） | 读单元⑤（`:562`）；`:569` `decl_checked == 5`、`:570` `exercise_open == 7`、exit 0（`:565-568`）、`failed == []`（`:571-575`）；随后 `remove_first_code_assign()`（`:378-389`，`:388` panic `"no \`:=\` on a code line"`）删掉**代码行里第一个 `:=`**，要求 exit 1（`:584`）、`:603` `q["code"] == "unexpected-token"`、query/grade span 相等（`:604-611`） | **红** —— 若单元⑤ 的 5/7 计数变了；**也红**若"第一个 `:=`"的变异不再产出 `unexpected-token`（tactic 改写会重排 `:=` 的位置） |
| 8 | `course_manifest.rs::set_theory_manifest_is_v2_and_fully_grouped`（`:277`） | 读真清单（`:278`）；schema（`:281-284`）、name（`:285-289`）、≥1 卷（`:294`）、每卷 ≥1 章（`:302-305`）、章 `id`（`:308-311`）、`prereqs` 键存在（`:312-315`）、`quota.exercises > 0`（`:316-323`）、单元 `file`+`unit`（`:330-334`）、**`units == 12`**（`:338`）、`chapters >= 4`（`:339`） | **纯内容改写不红**（它只解 JSON，从不判卷）；**改清单结构才红** |

### 8.3 明确**不会**红的（逐条澄清，避免误判）

| 文件 | 为什么安全 |
|---|---|
| `crates/cli/tests/course_shared.rs`（4 测试） | `:98 let root = repo_root().join("course")` —— **入门课**，钉 `course/shared/{And,Or,Nat}.sokonanoda` |
| `crates/cli/tests/course_project.rs`（8 测试） | 夹具是 `docs/gaps/repro/G06-course-import`（`:31`）+ 各自 tempdir |
| `crates/cli/tests/course.rs`（6 测试） | `:14` `COURSE_DIR = …/course` —— 入门课 |
| `crates/cli/tests/course_status.rs`（4 测试） | `:13` `COURSE_MANIFEST = …/course/course.json` —— 入门课 |
| `crates/cli/tests/skill.rs`（4 测试） | 只检查 `skills/**` 与 `course/course.json`。`PATH_PREFIXES`（`:67-74`）= `docs/ course/ crates/ examples/ skills/ scripts/` —— **`courses/` 不在其中**，所以技能正文里出现的 `courses/set-theory/...` 路径**不被校验** |
| `crates/cli/tests/protocol.rs` | 读 `examples/`（`:109`、`:152`、`:182`）与 `docs/protocol.md`，无课程派生计数 |
| `crates/cli/tests/notation.rs` 其余 13 个测试 | 全用自带临时夹具（`LIB` `:76-81`、`SECOND_CUT_LIB` `:394-407`、`THIRD_LIB` `:593-601`） |
| `courses/set-theory/tools/test_manifest_v2.py`（12 用例） | 除台账用例外全用临时夹具（§3.2） |

### 8.4 这些测试被谁跑

* `scripts/soko gate` → `sokonanoda gate` → `crates/cli/src/env/mod.rs:269 &["test","--workspace","--locked"]`
  ⇒ **全部 `crates/cli/tests/` 都跑**；随后课程门禁（`scripts/soko:902`）与 `gap.py check`（`:933-943`）。
* CI：`ci.yml:93` `cargo test --workspace --locked --no-fail-fast`；`:163` 单独再跑 `--test course`；
  `:190-194` 课程门禁；`:207-217` 缺口台账。
* **没有**任何地方跑 `test_manifest_v2.py`（§3.2）—— 手动。

---

## 9. 顺带发现的文档/代码漂移（不改也能过，但会误导）

| 位置 | 漂移 |
|---|---|
| `.github/workflows/ci.yml:173`、`:186` | 说「判据 G1–**G5**」，实际是 **G1–G6**（G6 是 0.60.0 加的） |
| `docs/design/course-gate-in-ci.md:204` | 表格里写「判据 **G1–G5**（… **0.60.0 起另加 G6**）」—— 同一句自相矛盾 |
| `scripts/soko:912` | 注释 `34 targets`（今天 36） |
| `docs/TESTING.md:100` | `36 个目标不重复解析启动器`（目标数会随 lib 模块增减） |
| `scripts/soko:933-943` vs `docs/design/course-gate-in-ci.md:205` | 文档说 gate 第四步跑 `gap.py selftest` + `check`，代码**只跑 `check`** |
| `docs/design/set-theory-syllabus.md` | **有两个 `## 4.`**（`:188` 与 `:206`），标题重复 |
| `docs/design/course-gate-in-ci.md:28-30`、`:95-98`、`:156` | 仍以 **34 目标 / 296 checked / 93 open** 为"实测"，早已过期（属历史记录，但读者会误当现状） |
| `docs/design/teaching-project.md:457`、`STATUS.md:189`、`docs/HANDOVER.md:403`、`REQUIREMENTS.md:1727` | 仍写 **355 checked**（P4 前的值） |
| `docs/gaps/ledger.jsonl` L-04/L-06 | 唯一两条 `workaround`；**没有复现件** ⇒ `gap.py check` 直接跳过（`:230-232`） |

---

## 10. 推荐的安全改写顺序（每步怎么验证）

> 原则：**先立护栏，再动内容；先动一个单元，再铺开；每步都能独立过 `gate`。**
> 顺序按「依赖关系 + 爆炸半径」排：记法声明（影响全部 12 单元）→ 单单元试点 → 铺开 → 收尾。

| 步 | 做什么 | 验证命令 | 预期 |
|---|---|---|---|
| **S0** | **先跑基线并留档**：`python3 courses/set-theory/tools/check.py --report /tmp/before.json --json`；记下 `summary` 与每个 target 的 `checked_names`/`open_names` | 见左 | 36/329/99/0；保存 `before.json` 供逐目标 diff |
| **S1** | **决定记法落点**：`∧ ∨ ↔ ¬`（+ 若统一 `∃`/`∀` 的 binder 记法）写在**哪个库模块**、谁 import。⚠️ 同符号全课程只能声明一次；`lib/Logic` 是空壳入口，`README.md:202` 禁止往那里加声明；新增 `lib/*.sokonanoda` 会 +1 目标 | 只加库文件、**不改任何单元**，跑 `check.py` | exit 0；`targets` 36→37（若新增模块）；新模块 `open == 0`（G5） |
| **S2** | **改一个单元试点**（建议**单元①**：它最小、只有 2 checked / 6 open，且**不在** `notation.rs` 的硬断言里）。画布 + 解答**同时**改（名字保持一致） | `python3 courses/set-theory/tools/check.py --only "单元 1" --only "解答 unit01"` | exit 0；G4 无 `missing` |
| **S3** | **试点单元跑 `--bisect` 与 `--selftest`**，确认新语法下定位手段仍可用 | `check.py --only "单元 1" --bisect`；`check.py --selftest` | 二分报「整份文件判绿」；selftest PASS |
| **S4** | **铺开到其余 11 个单元 + 记法对照页**，但**避开 `notation.rs` 硬断言的 4 个接触点**（单元② 的两行签名与「不得有 infix/notation 行」、`lib/Set` 的 5 行记法、单元③ 的 `𝒫 `、单元⑧ 的 `'' `/`binder_notation`/`∃ (`、单元⑤ 的 5/7 计数）—— 这 5 处**留到 S6 一起改** | `check.py --json` 逐目标对照 `/tmp/before.json` | 只允许**有意**的计数变化；G1–G6 全绿 |
| **S5** | **改 `lib/`**（若 S1 决定搬记法块） | `check.py` + `node scripts/soko grade <lib 绝对路径>` | `lib_open == 0`（G5）；每个 lib 目标 exit 0 |
| **S6** | **同轮改测试 + 复现件**（一次性、原子提交）：`notation.rs`（6 个）、`query.rs:569-570`、必要时 `course_manifest.rs:338-339` 与 `docs/gaps/repro/G07-course-manifest-v2.sh` | `cargo test -p sokonanoda-cli --test notation --locked`；`--test query`；`--test course_manifest`；`python3 scripts/gap.py check` | 全绿。**注意**：`notation.rs` 的 `the_shipped_course_still_uses_the_pointful_spelling` 就是为"这一轮"写的，注释 `:568-570` 明说连同 `docs/design/notation-subset.md` 一起更新 |
| **S7** | **收尾元数据**：`course.json`（若动结构）、`courses/set-theory/README.md`、`lib/Set.sokonanoda:140`、`STATUS.md`、`REQUIREMENTS.md` §9、`docs/HANDOVER.md`、`docs/TESTING.md`、相关 `docs/design/*`（§7 清单） | 人读 + `python3 -c "import yaml;yaml.safe_load(open('.github/workflows/ci.yml'))"` | 数字与 `--json` 一致 |
| **S8** | **追加成本台账** | `python3 courses/set-theory/tools/check.py --ledger` 然后 `python3 courses/set-theory/tools/test_manifest_v2.py` | 新行追加成功；12/12 passed |
| **S9** | **重生成站点数据**（前置：`scripts/soko doctor --json` 必须 `ready: true`） | `python3 scripts/gen-site-data.py` 然后 `python3 scripts/check-site.py` | 控制台必须出现 **「set_theory: 门禁实测 37 目标 · … 」**（**不是**「未实测…沿用上次实测的计数」）；`site hygiene: ok` |
| **S10** | **全量门禁** | `scripts/soko gate` | exit 0（cargo fmt/clippy/test + 课程门禁 + `gap.py check`）。⚠️ `test_manifest_v2.py` **不在 gate 里**，S8 已单独跑过 |
| **S11** | **提交 + 触发部署**：`git add` 课程 + 测试 + 文档 + `site/data/site.json`；因 `pages.yml:23-31` 不含 `courses/**`，若本轮没碰 `site/**`/`STATUS.md`/`Cargo.toml` 则需 `workflow_dispatch` 或同轮补 `paths` | `gh run list --workflow=pages.yml` | pages 真的跑了；`ci.yml` 的 `Course gate` step 与 `Gap ledger is consistent` step 全绿 |

**最容易翻车的三个点（按概率排序）**：

1. **`∃` 的搬家** —— 单元⑧ 本地 `binder_notation` 与 `notation.rs:768` 正面冲突（S6 必须同轮）；
2. **`example` ↔ `theorem` 的误用** —— 不判红，但会让 G4 静默失效或让 `checked` 计数意外漂移（§1.5）；
3. **`gen-site-data.py` 的静默降级** —— 跳过 `doctor` 前置就会把旧数字原样重新发布，而 `check-site.py` 永远绿（§4.3）。

---

## 附录 A：本次审计的复跑命令（只读）

```bash
scripts/soko doctor --json                                     # ready:true 才继续
python3 courses/set-theory/tools/check.py --json               # 36/329/99/0 基线
python3 courses/set-theory/tools/check.py --selftest           # 判据通道自检
python3 courses/set-theory/tools/test_manifest_v2.py           # 12/12（手动，CI 不跑）
node scripts/soko course "$PWD/courses/set-theory/course.json" --json   # checked 65 / open 96（只算单元）
python3 scripts/gap.py check                                   # 24 条与台账一致
python3 scripts/gap.py selftest                                # 14 条判据
python3 scripts/check-site.py                                  # 站点卫生（不查计数！）
git ls-files courses/set-theory | wc -l                        # 42（其中 35 个 .sokonanoda）
```

**本次没有做的事**（有意）：没有改任何现有文件；没有跑 `gen-site-data.py`（它会重写
`site/data/site.json`）；没有跑 `--ledger`（它会追加台账）；`example`/`theorem` 的事件行为
用 `/tmp/soko-probe/probe.sokonanoda` 的临时探针实测（仓库零改动）。
