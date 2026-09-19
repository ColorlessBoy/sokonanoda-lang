# WO-002 相对路径入口 + 祖先清单让模块根退化成空路径（G-12）

> 台账：`docs/gaps/ledger.jsonl` G-12（`kind: language` · `severity: blocker` · `status: open` ·
> `wo_planned: WO-002`）；复现：`docs/gaps/repro/G12-relative-entry-ancestor-manifest.sh`。
> 本文件是**工作单**，不是实现：语言线领单后先修订设计（`docs/design/imports-and-projects.md`
> §4.4 的"发现规则"），再按 TDD 三层动手——见 `docs/design/teaching-project.md` §6.3/§7。

## 用户可见症状 / 最小复现

- 复现命令（一条，从仓库根直接粘贴；退出码 0 = 缺口仍在，1 = 行为已变）：

```bash
bash docs/gaps/repro/G12-relative-entry-ancestor-manifest.sh
```

- 今天的实际输出（2026-09-18 实测，只贴关键行；长 JSON 用 `…` 截断）：

```
== ① 相对路径入口（文档写法）==
{"code":"import-not-found",…,"message":"找不到模块 `lib.Lib`（期望 lib/Lib.sokonanoda）",…}
   → import-not-found（预期）：yes
== ② 绝对路径入口（同一文件同一内容）==
{"human":"checked declaration t","name":"t","type":"decl.checked"}
   → 判卷通过（预期）：yes
== ③ 项目视图里的模块根（预期为空串；正确值应为 /var/folders/…/tmp.YlTMXQ8nho）==
   root=''
结论：G-12 仍在（相对路径坏、绝对路径好、模块根为空）——与台账一致。   # exit 0
```

- **形状①（台账 `today` 描述的）**：入口目录向上走到空分量 `""` 时，
  `Path::new("").join("sokonanoda.toml")` = `sokonanoda.toml`，于是"CWD 里的清单"被当成
  祖先清单命中；清单路径没有目录分量 ⇒ `manifest.rs:112 module_root` 拿 `parent() = Some("")`
  ⇒ 模块根 = 空 `PathBuf` ⇒ `resolve.rs:63 exact_lookup` 的 `fs::read_dir("")` 直接 ENOENT
  ⇒ 每个 `import` 都 `import-not-found`，`query project` 的 `root` 渲染成 `''`。
- **形状②（本轮实测补充，台账 `today` 里没有——请同轮补进台账）**：入口用**裸文件名**、
  cwd 在清单的**子目录**里时，上溯还没走到清单就先在 `""` 处断了（`Path::new("units").parent()
  == Some("")`、`Some("").parent() == None`），零配置退路抢跑 ⇒ 模块根 = 入口目录：

```bash
cd courses/set-theory/units && ../../../scripts/soko grade unit05-pairs-products.sokonanoda
# 3 × {"code":"import-not-found",…,"message":"找不到模块 `lib.Logic`（期望 ./lib/Logic.sokonanoda）"}
# query project: root='/…/courses/set-theory/units'，modules=[('unit05-pairs-products','load-failed')]
```

- **同一个文件，只因 cwd 不同就绿/红**（第三个读数，实测）：从仓库根跑
  `scripts/soko grade courses/set-theory/units/unit05-pairs-products.sokonanoda` 是**绿**的
  （相对路径里还留着 `courses/set-theory/` 这一段目录分量，上溯时直接命中清单；退出码 0，
  末尾事件是 `checked declaration demo_mem_pair` + `exercise open … kura_degenerate`）。
  ⇒ 症状的本质不是"相对路径不行"，而是**模块根是什么取决于入口怎么写 + cwd 在哪**，而
  `docs/design/imports-and-projects.md` §4.4（:437-447）明令 cwd 不参与语义：
  "把语义绑在 cwd 上会制造'同一个文件在不同入口下结果不同'的隐形状态"。

## 期望行为

- **非 Lean 语义，是工具链（模块根发现）行为。** 官方 Lean 4 的对照读数（本仓库自己的调研，
  `docs/design/imports-and-projects.md` §2.1）：`-R/--root` 默认 = **cwd**（:110-113）；
  搜索路径 = 调用方 roots ++ `LEAN_PATH` ++ `<sysroot>/lib/lean`（:121）；**文件自己的目录
  不在搜索路径里**（:141-149）。⇒ 在官方 Lean 下，文件实参写相对还是绝对**不改变** import
  的解析结果（只影响 `moduleNameOfFileName` 的输入与"必须落在 root 内"的检查）。
- 本项目修好后必须满足三条（这是验收判据，抄自台账 `expected_lean` + 设计 §4.4/§4.4b）：
  1. 入口路径先**绝对化**再上溯；cwd 只在"把入口变绝对"这一步用一次，此后不参与任何判定；
  2. 同一文件用相对 / 绝对写法 ⇒ 模块根、清单来源、模块名、事件流**逐字一致**（路径文本本身
     除外）；
  3. 模块根**永不为空**；`Path::new("")` 既不能当 root，也不能当上溯的一站。
- 本教学子集的边界（哪些不做）：
  - 只有**单一模块根**（`--root` > 最近祖先清单 > 入口目录），不做 Lean 的多根/`LEAN_PATH`
    搜索列表、不做"第一个命中的 root 胜出"那套；
  - 不改模块名 ↔ 路径规则：入口在 `units/` 下，模块名仍叫 `units.unit05-pairs-products`
    （`-` 非法那条规则也照旧，见 §4.3 / `crates/front/src/project/module_name.rs`）；
  - 不做符号链接与 `..` 的规范化**下沉**：视图层已经 `canonicalize`（见"范围"）。

## 范围

- 主修全在 **front 项目层**（`crates/front/src/project/`），三处，按重要性排序：

| 位置 | 改什么 | 依据 |
|---|---|---|
| `mod.rs:157-209 plan_project_with_overlay` | 函数入口处把 `entry_path` **绝对化一次**；`entry_dir` 由它派生（保留 `:163-167` 空 parent → `.` 的既有护栏）；`root_override` **同轮**绝对化 | 上溯的起点必须是绝对目录，否则形状①②都会复现 |
| `manifest.rs:58-76 find_manifest` | 上溯循环把**空目录分量**当终点：`dir.as_os_str().is_empty()` ⇒ 返回 `None`（护栏） | `Path::new("").join("sokonanoda.toml")` = `sokonanoda.toml`，语义上等于"去问 CWD" |
| `manifest.rs:112-121 module_root` | `path.parent()` 为 `Some("")` 时视为 `.`（现在只兜了 `None`） | 台账 G-12 `notes` 明确点名的护栏；空 root 还会让 `absolute()`（`query/project.rs:15-20`）的 `canonicalize("")` 失败、原样输出 `''` |

- **关键陷阱（防"修一半"，务必同轮处理）**：`graph.rs:147-164 module_name_of_path` 用
  `path.strip_prefix(root)` 推模块名。若只绝对化 entry、不绝对化 root（例如 `--root src`
  或 `--no-project` 的 `check.rs:78-82`、`build.rs:127-131` 传下来的相对 parent），
  `strip_prefix` 失配 ⇒ 模块名退化成**裸 file_stem**（`units.unit05-pairs-products` →
  `unit05-pairs-products`）⇒ `ProjectPlan::digest`（`mod.rs:119-144`）变 ⇒ 缓存键变、
  项目视图与重名检查全动。所以绝对化必须是"entry 与 root 一起"，并加不变量单测（见验收）。
- 绝对化用**词法**手段（`std::path::absolute`，1.79 稳定，`Cargo.toml:10` 的
  `rust-version = "1.96"` 够用；或 `current_dir()` 拼接——语义等价且完全不碰文件系统）。
  **不要**在内层用 `canonicalize`：stdin + `--root` 会合成一个**磁盘上不存在**的入口路径
  （`check.rs:118-123` 的 `root.join("Main.sokonanoda")`），`canonicalize` 失败后回落成相对
  路径，反而制造"绝对 entry + 相对 root"的混搭；canonicalize 是**视图层**的职责
  （`query/project.rs:15-20`，`docs/protocol.md:412-413` 的"Paths are absolute"就是它实现的）。
  - **待确认**：`std::path::absolute` 对 `..` 的词法处理以当前工具链实测为准
    （本机 `rustc 1.98.1`）；入口路径不需要 `..` 解析时直接用 `current_dir()` 拼接更可控。
- 预期**不动**的相邻文件（读源码确认过，作为边界的证据）：
  - `resolve.rs:40-82`：保持纯查找（空 root 在 `plan_project` 归一为 `.`，`--root ''` 也在那里
    归一；`resolve.rs:63` 的 `read_dir("")` ENOENT 是**症状**不是病因）；
  - `crates/cli/src/check.rs` / `build.rs`：相对入口、`--no-project`、stdin + `--root` 的入口
    都由 `plan_project` 统一绝对化，CLI 侧不改；
  - `crates/lsp/src/lib.rs:275-277`：LSP 入口来自文档 URI（已绝对），只回归不改；
  - `crates/cli/src/course.rs`：见"不做的事"。
- **是否动内核：预期否**（内核是冻结快照）。本缺口全在 front 的 IO/路径层，连 `resolve.rs`
  的查找算法与内核热路径都不碰。

## 不做的事

- 不做多搜索根 / `LEAN_PATH` 语义（设计 §4.4 明令不做）；本 WO 只修"单根算错"。
- 不改 `find_manifest` 的 `.git` / `$HOME` 硬边界（设计 §4.4 :448-451），**不**改成
  "一路走到 `/`"（那是被调研系统 clangd/rust-analyzer/pyright/gopls/Agda/lean4 的共同坑）。
- 不改 `--root` / `--no-project` 的**语义**（只把结果变绝对）；不新增 flag、不改协议字段、
  不改退出码。
- 不改模块名规则与 `-` 非法规则；不顺手做端口/大小写提示那条 `resolve.rs` 的逻辑。
- **不修 G-06**（`course` 走项目闭包）：`crates/cli/src/course.rs:114-118 count_unit` 今天根本
  不走清单发现（单文件 `compile_cached`），所以 G-12 的修法碰不到它。**但**登记一条提醒：
  G-06 修的时候 `course.rs:34` 的 `base = manifest_path.parent()`（可能是相对路径）必须同轮
  绝对化，否则 G-12 会在课程聚合路径上复发——这条**不进**本 WO 验收。
- 不改课程仓文件：`courses/set-theory/tools/check.py:6/:33` 的绝对路径补丁、`AGENTS.md:21-22`
  的说明、台账 status 由课程线/派单方在关账同轮跟进。
- 不碰 G-02 的构造子命名空间（另一张 WO；它会动 `courses/set-theory/lib/Prod.sokonanoda` 的
  `prod_mk`）。本 WO 不碰构造子/命名空间语义，两张 WO 可并行，但**不要同轮混改同一文件**。

## 验收（三层）

### front 单测（`crates/front/src/project/manifest.rs` 的 `#[cfg(test)]` + `tests.rs`）

1. `find_manifest(Path::new(""))` 必须 `None`；且上溯**不会**把没有目录分量的
   `sokonanoda.toml` 当成祖先清单（护栏，直接钉住本缺口的病根）。
2. `module_root(Path::new("sokonanoda.toml"), &Manifest::default())` 必须等于 `.`
   （空 parent 归一；现在会得到空 `PathBuf`）。
3. 绝对化助手（若落在 front）：相对路径 `units/u.sokonanoda` ⇒ 绝对且尾部不变；已绝对则原样。
4. **不变量（防"修一半"）**：同一次 `plan_project` 里必须 `entry.is_absolute() ==
   root.is_absolute()` 且 `entry.strip_prefix(&root).is_ok()`——否则模块名退化（见"范围"的陷阱）。
5. 既有测试必须原样绿：`tests.rs:72-94 manifest_moves_the_module_root_and_nested_names_resolve`、
   `manifest.rs:181-189 an_empty_manifest_is_a_valid_project_root`、
   `manifest.rs:192-204 src_moves_the_module_root_and_must_stay_inside`。

### CLI e2e（新增到 `crates/cli/tests/imports.rs`）

复用该文件的 `run(dir, args, stdin)` 助手（`:36-63`：`current_dir(dir)` + `SOKONANODA_CACHE_DIR`
隔离缓存；`tests/common/` 里没有这个助手）：

1. 夹具：`sokonanoda.toml` + `lib/Lib.sokonanoda` + `units/u.sokonanoda`（含 `import lib.Lib`）。
   `run(dir, ["--json","units/u.sokonanoda"])` 与 `run(dir, ["--json", <dir>/units/u.sokonanoda])`
   的**事件流逐行一致**（type+name 序列相同、退出码同为 0、0 条 `diagnostic`）——这正是台账
   `today` 里"同文件换绝对路径即绿"的反面。
2. **形状②**：`run(<dir>/units, ["--json","u.sokonanoda"])` 同断言（最近祖先清单必须被找到，
   不许退化成入口目录）。
3. `query project`（`--compact`）两种写法的 `root` **逐字相等**且都等于 `canonicalize(dir)`
   （经 `query/project.rs:15-20`），`entry` 模块名相同。注意入口在根的子目录下，名字是
   `units.u`——**不要**按 `u` 写 golden。
4. 回归（既有测试不改也绿）：`--no-project` + 相对入口（模块根 = 入口目录）、
   stdin + 相对 `--root src`（`imports.rs:172-188`）、无 import 文件的 A1 对拍
   （`imports.rs:96-116`）、零配置退路（无清单时 root = 入口目录）。
5. 负向护栏：把祖先清单删掉后，`import lib.Lib` 仍必须 `import-not-found`（证明"修好"不是
   靠"到处都能找到清单"）。

### 课程用例

- 真实文件与练习：**`courses/set-theory/units/unit05-pairs-products.sokonanoda`**——第 25-27 行
  `import lib.Logic` / `import lib.Set` / `import lib.Prod`；练习名 `prod_fst_mk`（:60）、
  `prod_snd_mk`（:68）、`prod_mk_inj`（:76）、`prod_mk_eq_iff`（:85）、`mem_prod_iff`（:107）。
  卷 I **12 个单元全部** `import lib.*`（实测 `grep -l '^import ' courses/set-theory/units/*.sokonanoda | wc -l` = 12；
  `courses/set-theory/course.json` 12 条），所以这一条代表整门课。
- 命令与判据：

```bash
cd courses/set-theory
../../scripts/soko grade units/unit05-pairs-products.sokonanoda --json
# 今天：3 × import-not-found、0 checked / 0 open、exit 1
# 修后：exit 0；decl.checked=5、exercise.open=7、0 diagnostic，
#       且与绝对写法（"$PWD/units/unit05-pairs-products.sokonanoda"）**逐个事件一致**
../../scripts/soko query project --file units/unit05-pairs-products.sokonanoda --compact
# 今天：root=''、modules=[('units.unit05-pairs-products','load-failed')]、counts.errors=3
# 修后：root='/…/courses/set-theory'、modules=lib.Logic/lib.Set/lib.Prod/units.unit05-pairs-products
#       全 'compiled'、counts={modules:4, compiled:4, decls:67, errors:0, open_exercises:7}
```

- 课程侧收尾（课程线跟进，语言线本轮不改课程仓）：`tools/check.py` 的 `str(path.resolve())`
  绝对路径补丁可撤，`AGENTS.md:21-22` 的"原因见 G-12/G-10"改成只留 G-10。

### 影响面：事件计数是否变 → 是否要同步 golden

- 事件流**不含路径**（`decl.checked` / `exercise.open` / `expr.*` / `diagnostic` 是 type+name+
  span+code），编译结果不变 ⇒ `crates/cli/tests/course.rs:86-97` 与
  `crates/cli/tests/course_status.rs:68-80` 的**双 GOLDEN 预期不变**；
  `ProjectPlan::digest`（`mod.rs:119-144`）只用模块名/源码/imports ⇒ 缓存键不变、不失效。
- 但**路径文本**会变，两条既有断言必须同轮改（不改就 `cargo test` 红；这不是"golden 漂移"，
  是断言本来就写死了相对路径）：
  - `crates/cli/tests/imports.rs:124`：`text.contains("\"file\":\"Logic.sokonanoda\"")`
    → 依赖诊断的 `file` 变绝对（`check.rs:167-173` 只 trim 前导 `./`，不会绝对化）；
  - `crates/cli/tests/project_features.rs:250`：`assert_eq!(parse_error["file"], "Bad.sokonanoda")`
    → 改成 `ends_with("Bad.sokonanoda")`（同文件 `:404` 已是这种写法）。
- 其余路径断言是子串/后缀，不受影响：`imports.rs:164/:278`、`project_features.rs:288/:316`、
  `crates/front/src/query/tests.rs:525`；`project_features.rs:584-590` 的 root 断言本就接受
  非空绝对路径。
- 人类视图不变：入口错误的 `path:line:col` 用的是 CLI 参数 `label`（`check.rs:51-58`），
  不随绝对化改变。

## 文档同步清单

- `AGENTS.md:21-22`（`check.py` 用绝对路径 + G-12/G-10 的说明）→ 关账后删 G-12、只留 G-10；
  Setup 表里的 `scripts/soko grade <相对路径>` 用法（`:29-32`）恢复为无条件成立，并补一句
  "相对/绝对写法等价"。
- `docs/protocol.md` §`soko/project`（:412-413 已承诺 "Paths are absolute"）→ 修好后契约才
  真正成立；建议同轮把"`root` 是绝对路径、**永不空**"写进契约句。
- `docs/design/project-view.md:46`（`root: String // 模块根（绝对路径）`）同轮核一句；
  `docs/design/imports-and-projects.md` §4.4（:437-439 发现规则）补"入口路径先绝对化，
  cwd 只在绝对化那一步参与"——**设计先行**：语言线领单后先改这段设计再动代码。
- `docs/gaps/ledger.jsonl` G-12：`status: open → wo-filed`、`wo: docs/gaps/WO-002-module-root-absolute.md`
  （**派单方补**；本文件不代改台账）。修好后 `fixed_in` 必填。
- `docs/gaps/repro/G12-relative-entry-ancestor-manifest.sh`：修好后按设计翻成 exit 1（这正是
  `gap.py close` 的判据）。若要留成长期回归锚点，建议同轮把它改成"两种写法必须一致"的
  正向断言，并把**形状②**（裸文件名 + cwd 在子目录）补进去——这是语言线的设计决定。
- `docs/gaps/README.md:48`（手工复跑清单里 G-12 的那一行）同轮改文案。
- `docs/HANDOVER.md`（缺口/WO 段）与 `STATUS.md`（收尾义务：只留最近 3 轮）同轮记一轮；
  `REQUIREMENTS.md` §9（:1436-1440 已记 G-12）标注修复版本——**用户可见改动必须同轮**。
- `skills/`：已 grep `skills/*/SKILL.md`，**无** G-12 / "绝对路径"措辞命中 ⇒ 预期不动；若语言线
  在技能正文里补"相对路径判卷也可"，同轮确认 `.agents/skills/` 薄入口仍指向正文
  （`crates/cli/tests/dsh.rs` 会挡漂移）。
- `editor/vscode/`：扩展侧无 API/命令/字段变化（`editor/vscode/README.md:61/:69` 的模块根
  说明与修后行为一致）⇒ 预期不动；按仓库硬规则仍要在同轮**核对** README/CHANGELOG/
  package.json，无改动就在 `STATUS.md` 写一句"核对过、无需 bump"。

## 门禁

- `scripts/soko gate`（= fmt + clippy + test + playground 锚点）。注意 gate 的 anchor 用
  **运行中二进制**的内嵌编译器，版本与仓库不一致会直接 exit 3 ——先 `scripts/soko update`。
- 全量 `cargo test --workspace --locked`（上面那两条必须同轮改的断言会在这里先红给你看）。
- 手工三连（课程用例，见验收第三层）+ `cd courses/set-theory && python3 tools/check.py`
  （绝对路径那套应保持绿）。
- `python3 scripts/gap.py check`：修好但未关账时 G-12 一行会红（复现 exit 1）——**预期**，
  `close` 后转绿。
- 纪律：不用 grep 掩膜退出码；`cargo fmt` 只对教学 crates（禁 `cargo fmt --all`，会重排冻结内核）。

## 关账

- 修好后：`python3 scripts/gap.py close G-12 --version <新版本>`（脚本会**先复跑复现**，仍复现
  则拒绝关账；`fixed_in` 必填）。版本取 bump 后的实际值：当前 `Cargo.toml:6` = `0.58.0`
  且 0.58.0 已发布 ⇒ 预期 `0.59.0`，以 `docs/RELEASE.md` 的 bump 为准。
- 关账后：`python3 scripts/gap.py check` 全绿；`courses/set-theory/tools/check.py` 的绝对路径
  补丁可撤（课程线跟进）；`AGENTS.md:21-22`、`docs/gaps/README.md:48`、
  `REQUIREMENTS.md:1436-1440` 同步。
- 若语言线决定**只修形状①、不修形状②**（或反过来），必须在台账 `notes` 里写清剩下的形状
  与复现，**不要**让缺口以"半修"状态关账。
