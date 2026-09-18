# 设计：多文件 `import` 与项目管理（设计 + 计划 I16）

> 状态：**已落地（0.57.0，2026-09-18）**——P0–P6 完成，P7 为 backlog；实现实况与
> 三处偏差见 §5.1「as-built」（ROADMAP **I16**）。
> 触发（用户 2026-09-17）：「我想增加 代码import +project管理，帮我调研一下其他语言
> 都是怎么分别处理单文件，和项目。项目如何维护。sokonanoda如何实现，具体执行方案
> 是什么」。
> 权威顺序：`REQUIREMENTS.md`（要求总账）> 本文（I16 方案）> `ROADMAP.md`/`AGENTS.md`。
> 关联：`docs/design/compile-cache.md`（缓存键今天只含单文件，§4.8 是其延拓）、
> `docs/design/compiler-service-events.md`（`watch --workspace` = 每文件独立 session）、
> `docs/design/course-syllabus.md`（`course/course.json` 是既有的"清单"先例）、
> `docs/architecture.md` §4（一条文件的旅程）、§6（内核改动清单 = 冻结边界）、
> `docs/notes/lsp-notes.md:128/191`（多文件交叉失效曾被明确推迟，本文是那个触发点）。

---

## 0. 一句话结论

**把"编译单元"从「一个文件」升级为「项目闭包（一个入口文件 + 它的 import 闭包）」，
其余一切不变**：`import Foo.Bar` 用真实 Lean 4 的置顶语法，模块名↔路径用 Lean 同款
规则，项目根 = 最近祖先目录里的 `sokonanoda.toml`；内核**一行不改**（跨模块的
声明由 front 在**同一个 arena / 同一个 `EnvBuilder`** 里按拓扑序构造，正好是
`EnvBuilder::add_declar` 已有的能力）；单文件、无 `import` 的文件**行为逐字节不变**
（缓存键、事件流、golden 计数全不动），所以教学画布 `playground.sokonanoda` 与
45 个课程/示例文件今天怎么跑，以后还怎么跑。

一句话验收：**两个文件、一个 `sokonanoda.toml`，`import` 能跨文件看到声明、
报错能指回正确的文件与行列、缓存能区分"依赖变了/没变"**——而且零 cargo、
零网络、golden 事件计数不动。

---

## 1. 触发与目标

### 1.1 用户要什么

| 用户的问题 | 本文的回答 |
|---|---|
| 其他语言怎么**分别**处理单文件与项目？ | §2.1–§2.4：10 个系统的横向调研（Lean/Lake、Coq、Agda、Isabelle、Idris、Rust、Go、Python、JS/TS、Haskell/OCaml）+ 语言服务器与缓存的做法 |
| 项目如何维护？ | §2.5 模式总结 + §4.4/§4.8/§4.9：清单文件、根发现、模块图、缓存的键与失效、`build`/`query`/LSP/`watch` 各自怎么接 |
| sokonanoda 如何实现？ | §3 现状审计（含实测与代码接缝）+ §4 设计（D1–D10，每条带取舍与被否方案） |
| 具体执行方案？ | §5 分阶段计划（P0–P6，每阶段独立可验收）+ §6 验收 A1–A8 |

### 1.2 为什么值得做（本项目特有的三条理由）

不是"别的语言都有所以要有"。用本仓库语料**量出来**的理由（数字与命令见本轮调研
底稿，`docs/STATUS-ARCHIVE.md` 之外的原始测量由 `course/`+`crates/cli/tests/` 复算）：

1. **正确性：同名不同义今天无法表达，只能靠"每个文件各自闭合"回避。**
   45 个语料文件里 **71 个名字有 ≥2 种定义**，其中 **20 个变体从未在同一单元共现**
   ——它们是真正的跨文件歧义，不是练习 vs 答案：
   - `Or`：单元①/④ 与 `examples/` 是 `axiom Or : Prop -> Prop -> Prop`，
     单元⑨/⑩ 是 `inductive Or (A B : Prop)`（真归纳 + `Or.rec`）；
   - `Iff`：单元⑨/⑩ 是 `def Iff (A B : Prop) := And (A -> B) (B -> A)`，
     `examples/` 是 `axiom Iff`；
   - `And.intro/.left/.right`：15 处 `(a : Prop) -> ...`，`fol-basics` 用
     `forall (a : Prop), ...`，`py-fol-core` 用 `{a : Prop} -> ...`——**内核里是不同的类型**。
   > **模块边界是唯一能让"哪个 `Or`？"可回答的机制。** 没有 import，agent 与用户
   > 只能靠文件路径这个隐式约定来区分，而任何一次"把两个例子拼起来看"都会踩雷。
2. **复用：`solutions/` 与 `examples/` 的整段重抄。** 单元1/6/9 的解答文件与画布
   声明骨架**逐一对应**（19/19、19/19、27/27），只有证明体不同；`examples/py-fol-core`
   的 26 个声明里有 18 个是把 `examples/fol-basics` 重写一遍（还换了 binder 风格）。
   最直白的一处：`course/unit6:21-36` 与 `course/unit7:16-31` 是**逐字节相同的 16 行
   `inductive Nat` 块**（diff IDENTICAL），英文镜像再复制两份 = **同一段代码 4 份拷贝**，
   而 unit7 的注释自己写着"same as the previous unit"。
   全语料 **1217/3851 行（31.6%）** 落在"名字在 ≥2 个文件里出现过"的声明块内——
   注意：**这一条只是收益，不是理由**（最大 5 组重复一共只省 122 行，纯为省行数
   不值得动缓存与 LSP），真正值钱的是第 1 条与第 3 条。
3. **教学闭环：老师 agent 的"教材库"可以被 45 个画布复用。**
   教学主循环是"agent 把逻辑连接词从零讲一遍 → 出题 → 用户作答"。今天每开一个新
   画布，agent 都要**重新口述/重写** `And`/`Or`/`Iff` 的骨架（22 个文件里都有
   `And` 家族）——这既是 token 成本，也是"同一概念在不同画布里定义不一致"的根源。
   有了 import，agent 可以维护 `Lesson/Logic.sokonanoda`，每个学生的画布只写
   `import Lesson.Logic` + 本轮练习；而**判卷路径不变**（仍是
   `scripts/soko grade playground.sokonanoda`）。

### 1.3 非目标（v1 明确不做）

- **不做包管理**：没有 registry、没有 `require`/git 依赖、没有版本求解。项目内
  只有"本仓库的模块"；跨项目依赖留 `[deps]` 扩展点（§4.11）。
- **不做 `namespace`/`open`/`section`/`private`/`import all`**：它们是真实 Lean 语法，
  但要各自走"课程 + 测试 + 白名单"三件套；v1 只做 `import Foo.Bar` 一条，
  扁平全局名字 + 冲突教学错误（§4.7 说明为什么这仍然够用）。
- **不重构课程语料**：45 个文件的 golden 计数是硬门禁（`crates/cli/tests/course.rs`），
  v1 只保证"不改它们也能用 import"，语料重构单独一轮（§4.10 / Q5）。
- **不做 decl 级编译产物（`.olean` 等价物）**：v1 用"闭包哈希 + 报告缓存"拿到
  "没变就不重编"；跨进程复用**已检查声明**留 P4（可选"信任台账"，且启用前必须换强哈希）
  与 P7（真正的 decl 级产物，需先推翻内核冻结边界）。
- **不改内核**：任何需要动 `crates/kernel` 的方案直接否掉（§9）。

---

## 2. 调研：其他语言/证明助手怎么分「单文件」与「项目」

> 调研由 3 个并行 subagent 完成（Lean 4/Lake；其他语言与证明助手；语言服务器与缓存），
> 每条结论带官方文档/源码出处（§10）。本节只保留**对设计有约束力**的部分，
> 结论列在 §2.5（采纳 / 不采纳）。

### 2.1 Lean 4 + Lake（教学语法的母体，规则必须对齐）

**单文件模式**（`lean foo.lean`）
- 只做：解析 header → 加载 import 的 `.olean` → elaborate 命令 → 产出环境则 exit 0；
  **默认不写任何产物**（`oleanFileName? := none`，要 `-o Foo.olean` 才写）。`main` 不运行
  （那是 `--run`）。⇒ **"单文件 = 零配置、零产物"是 Lean 的原生行为**，我们的单文件
  路径与它同形（写出 `.sokonanoda` 报告缓存属于我们的扩展，不进用户目录树）。
- **隐式 prelude**：没有 `prelude` token 时自动 `import Init`（外加 meta 版 Init）。
  `prelude` 关键字是 Lean 自己源码用的，官方明确"不供用户使用" ⇒ **我们不做
  `prelude` 关键字**（与现有 `-- sokonanoda:prelude none` 指令不冲突：那是我们的
  注释指令，不是 Lean 语法）。
- **模块名来自路径**：`moduleNameOfFileName fileName opts.rootDir?`；`-R/--root=dir` 的
  官方说明是"set package root directory from which the module name of the input file is
  calculated (**default: current working directory**)"；文件不在 root 下直接报
  `input file '<f>' must be contained in root directory (<root>)`。
  ⇒ 我们的"**无 manifest 时模块根 = 入口文件目录**"（§4.4）与 Lean 的默认
  （cwd）**同构但不完全相同**：我们的更局部、更可预测（编辑器里打开深目录文件
  不会因为 cwd 不同而改变语义）。**这是刻意 divergence，写进 Q2。**
- **缺模块的报错**（可照抄的体验）：`unknown module prefix 'Foo'` +
  `No directory 'Foo' or file 'Foo.olean' in the search path entries: …`（root 分量
  缺失），以及 root 存在但产物不在时的 `object file '<path>' of module Foo.Bar does not exist`。
  ⇒ 我们的 `import-not-found` 也要**列出搜过的根**（§4.2）。
- 搜索路径 = 调用方 roots ++ `LEAN_PATH` ++ 内置 `<sysroot>/lib/lean`。

**模块名 ↔ 路径**
- `import A.B.C` ↔ 某个 root 下的 `A/B/C.lean`（去掉扩展名、`.` 变目录）；**第一个命中的
  root 胜出**，后面的 root 不再看，也**没有**重复模块诊断。
- 分量合法性 = **层级标识符**规则（`import Foo-Bar` 是语法错误）；`«Foo-Bar»` 虽可解析
  但会触发可移植性检查：保留文件名（`CON/PRN/AUX/NUL/COM1-9/LPT1-9`）与
  某些 OS 禁用字符（`< > " | ? * !`）。⇒ 我们的 D2 采纳同一套：**`-` 直接拒绝**
  （教学 hint 指路），`! ?` 等给出可移植性提示（注意：本仓库 lexer **允许** `! ?`
  出现在标识符里，所以这条要在模块名层单独判）。
- **`import` 置顶**是硬规则，官方报错逐字：
  `invalid 'import' command, it must be used in the beginning of the file`。
  ⇒ 我们的 `import-must-precede-declarations` 语义与它一致（文案中文，语义同款）。
- **环**：Lean 核心假定 import 是 DAG（注释 "As imports are a DAG…"），但**专门的自导入
  诊断在 Lake 里**：`module imports itself` 与 `bad import '<M>'`
  （`src/lake/Lake/Build/Module.lean:413/:88`）；裸 `lean foo.lean` 下自 import 只能退化成
  `unknown module prefix`（还没有 olean）或陈旧的
  `import <M> failed, environment already contains '<c>' from <N>`。
  ⇒ 我们做**显式 `import-cycle` + 环路径**（纯前端可判、教学价值高），语义上与
  Lake 的诊断同向、比裸 `lean` 友好。
- **⚠️ 文件自己的目录不在 Lean 的搜索路径里（本设计最重要的一条外部事实）**：
  `initSearchPath` 在 C++ driver 里、**命令行解析之前**只跑一次
  （`src/Lean/Util/Path.lean:113-114`、`src/util/shell.cpp:224-227`），此后只有 LSP
  服务端会再动它；**cwd 只影响"模块名怎么算"，不参与 import 解析**。后果：
  `lean foo.lean` 旁边放着 `Bar.lean`、文件里写 `import Bar`，**会失败**，除非
  `LEAN_PATH` 覆盖该目录。⇒ 我们"无清单时模块根 = 入口文件目录"（§4.4）**是对真实
  Lean 的刻意 divergence**，换来的正是"两文件 demo 零配置"与"编辑器/agent 不依赖
  服务端 cwd"；它是硬规则 3（教学**语法**是 Lean 子集）之外的**语义偏离**，
  必须写进文档、并由 **Q2** 明确拍板。
- `import` vs `import all`：**没有 `module` 头的文件里，普通 import 就是传递可见**
  （`isExported := publicTk.isSome || moduleTk.isNone`）——教学文件正是这种情形。
  ⇒ 我们只做普通 import 的传递可见（§4.7），`public`/`meta`/`import all` 都不做。

**Lake 项目形态**（我们要"小一号"地抄）
- 官方工作区布局：`lean-toolchain`（工具链钉版本，elan 按需装）+ `lakefile.toml` 或
  `lakefile.lean` + `lake-manifest.json`（依赖的精确 revision，"应视为代码的一部分
  并入库"）+ `.lake/`（`lakefile.olean` 缓存、`packages/` 依赖副本、
  `build/{bin,lib,ir}`）。
- `lake new foo` 生成：`lakefile.toml`（TOML 是默认配置语言）+ `Foo.lean` +
  `Foo/Basic.lean` + `Main.lean` + `lean-toolchain` + `.gitignore` + `.lake/`。
- **源码默认就在包根**：`srcDir : FilePath := "."`，官方注释说明它会作为 `lean` 的
  `-R` 参数传下去；`src/` 不是必须的。⇒ 我们默认模块根 = manifest 所在目录，
  `src = "src"` 只是可选（§4.4 一致）。
- `lean_lib` 的 `roots`/`globs` 决定"库名 = 模块前缀、glob 圈定子模块"；`lake build`
  的产物落在 `.lake/build/lib/lean/Foo/Basic.olean`（+`.ilean`/`.trace`），
  C 在 `.lake/build/ir/`，可执行在 `.lake/build/bin/`。
- **Lake 不向上找包（已定论）**：`rootDir : FilePath := "."`（`CLI/Main.lean:48`），
  只由 `-d/--dir` 改（`:234,:362`），`mkLoadConfig` 只做 `resolvePath? opts.rootDir`（`:135`），
  整条 `loadWorkspace → loadWorkspaceRoot → resolveConfigFile` **从不碰父目录**
  ⇒ `lake build` 在包的子目录里**会失败**（除非 `--dir`）；官方文档同口径
  （`--dir=DIR` "Use the provided directory as location of the package instead of the
  current working directory"）。包身份**只看文件名**：优先 `lakefile.lean`，
  两个都在就打日志说用哪个，都没有就报
  `[name]: no configuration file with a supported extension:`（源码逐字，`Load/Package.lean:93-119`）。
- **同一个生态里两条不同的"向上"规则（很有启发）**：**Lake 不向上找包，但 elan 向上找
  `lean-toolchain`**（"walking up through parent directories until a toolchain version is
  found"）——**工具链是继承的，包根不是**。⇒ 我们把这两件事分开：
  **根** = 最近祖先的 `sokonanoda.toml`（§4.4）；**版本钉** = manifest 里的 `requires`
  （v1 只警告），而不是像 `lean-toolchain` 那样独立成向上继承的文件。
  manifest 版本错误也可以照抄它的语气（`invalid version '{ver}'; you may need to update
  your 'lean-toolchain'`）——**"告诉用户改哪个文件"** 是这些文案的共同点。

**编辑器（vscode-lean4）的项目根发现——我们 LSP 的直接参照**
- `findLeanProjectRootInfo`：先从 `.lake/packages` 里"退出来"（依赖自己的 toolchain
  不算项目根），然后**向上走**：见 `lean-toolchain` ⇒ 项目根（带工具链）；见
  `lakefile.lean|toml` ⇒ "有 lakefile 但没 toolchain"的独立状态；一路上到文件系统根，
  再退回 VS Code workspace folder，最后退回**文件自己的目录**。
- ⇒ **"孤立文件也有根、以单文件模式服务"** 是 Lean 编辑器的既有行为，
  我们的三层发现（manifest → workspace → 文件目录）与它同形（§4.9）。
  差异：我们**不引入** `lean-toolchain`-式"根但无工具链"的中间状态，
  而是把版本钉在 `requires`（Q6，v1 只警告）。
- **服务端怎么起（对 DSH/VSCode 接线有参考价值）**：有 lakefile 时
  `lake serve --`，否则 `lean --server`（`leanclient.ts:834-840`）；**cwd = 项目根**；
  扩展**不设** `LEAN_PATH`、**不传** `--root`，只把 `~/.elan/bin` 前置进 PATH
  （设置项是 `lean4.envPathExtensions`/`lean4.serverArgs`，**没有** `lean4.serverEnv`）。
- **"没有项目"的官方 UX 文案（可直接照抄语气）**：`Lean 4 server is operating in
  restricted single file mode.`、`Opened folder does not contain a valid Lean 4 project.`、
  以及"父目录里有合法项目，要不要打开它？"的提示；`有 lakefile 但没有 lean-toolchain`
  是**硬错**。**负向事实**（别张冠李戴）：`No Lean project found` 与 `missing manifest`
  这两个字符串**在 vscode-lean4 里不存在**。
- **设计结论（两个工具链对"我的项目是什么"看法不一致）**：Lake 认为根 = cwd（或 `--dir`），
  elan 与编辑器认为根 = **最近祖先**。我们的选择与**编辑器/elan 同侧**
  （§4.4 的最近祖先 + 硬边界）：只有这条规则能让"在子目录里打开任意文件"工作，
  而判卷/agent 场景的 cwd 恰恰不可控。



### 2.2 其他证明助手（同一问题的不同答案）

| 系统 | 单文件 | 项目标记 / 发现 | 模块身份 | 产物 |
|---|---|---|---|---|
| **Coq/Rocq** | `rocq c foo.v` 可以 | `_RocqProject`（**IDE 向上找最近祖先**），扩展名故意为空；`-Q dir dirpath` 把逻辑路径映射到物理目录，用 `-Q` 时必须 `From X Require Import Y` | 路径 + `-Q`/`-R` 映射推出（**文件名必须是合法标识符**：`to-to.v` 非法） | `.vo` 在源旁边；`rocqchk` 可对产物独立复检 |
| **Agda** | `agda Foo.agda` 可以 | `.agda-lib`（`name/depend/include/flags` **全可选**）；**从模块名推出 root 再向上找**，同一层多于一个库 = 错误，且 root 之下不许再有库文件 | 严格路径推导：`A.B.C` 必须住在 `root/A/B` | 有库时 `_build/VERSION/`，否则 `.agdai` 在源旁边（改名留孤儿） |
| **Isabelle** | **没有单文件模式** | `ROOT` 会话 + `ROOTS` 目录清单 + `-d DIR`；没有"向上找" | 声明式理论名（文件名只是惯例） | heap 镜像/日志在 `ISABELLE_HEAPS`；`imports` 必须无环 |
| **Idris 2** | `idris2 f.idr` 可以（`--check` 也能只做检查） | `.ipkg`（`modules` **必须逐一列出**、`depends`、`sourcedir`）；不向上找 | **声明** `module Foo.Bar` **且**必须与 `sourcedir` 下路径一致 | `build/exec`、`build/ttc/*.ttc`（源变即重生成）；切模式要手删 `build` |

**要点**：证明助手普遍把"逻辑名 ↔ 物理路径"当成**一等公民**（Coq 的映射、Agda 的严格一致、
Lean 的路径推导），而"强制清单"（Isabelle/Dune）会让**第一次运行**变重；Idris 的
"声明 + 校验"是最贵的身份模型（两处要同步）。

### 2.3 通用语言（零配置是默认，清单只为主语料服务）

| 系统 | 裸文件 | 清单 | 模块身份 | 产物位置 / 失效 |
|---|---|---|---|---|
| **Rust** | `rustc single.rs` → `./single` | `Cargo.toml`（`cargo new` 只给 `src/main.rs` + `.gitignore`）；workspace 共享 `target/`+`Cargo.lock` | 声明 `mod` + 固定文件规则（`foo.rs` 或 `foo/mod.rs`，**两者都在 = 错误**） | `target/{debug,release}`；重编**主要看 mtime**（内容哈希 freshness 仅 nightly） |
| **Go** | `go run single.go`（无 `go.mod`） | `go.mod`（`module` 路径即包路径前缀）+ `go.sum` | 目录即包：包路径 = module 路径 + 子目录 | `GOCACHE`；自导入非法、两模块提供一个包 = 错误 |
| **Python** | `python script.py` | **没有清单**；`pyproject.toml` 只在构建/安装时读（恰三个表） | 文件名推导，**从不声明**；`__init__.py` 区分包 | `__pycache__` 在源旁边，按 mtime；循环导入被容忍、**首个命中静默胜出** |
| **Node/TS** | `node script.js` | `package.json`（**最近的祖先**决定 CJS/ESM）；`tsconfig.json` 是 TS 的项目清单（`include/exclude/rootDir/paths/references`） | CJS 无模块身份；ESM 说明符就是路径 | 向上走 `node_modules`；`.tsbuildinfo` 用**内容哈希 + mtime** |
| **Haskell / OCaml** | `ghc hello.hs` / `ocaml foo.ml` | `.cabal`（**必须逐一列出模块**，否则"陈旧的成功"）/ `dune-project`（没有就不干活） | GHC：声明且必须 `A/B/C.hs`；OCaml：**从不声明**，取文件名首字母大写 | `.o/.hi` 在源旁边（GHC）/ `_build/default`（dune）；GHC 用**接口指纹**、OCaml 用 **`.cmi` 摘要** |
| **JVM** | `java Hello.java` 可以 | Maven `pom.xml`（GAV 三元组） | 声明 `package` + 类型名，目录规则**由宿主决定是否强制** | `.class` 在源旁边或 `target/`；Maven 约定 `src/main/java` |

**要点**：**初学者最先遇到的语言，都是"裸文件先跑起来"的语言**（Python/Node/`rustc f.rs`/
`go run f.go`/`java Hello.java`/`ghc hello.hs`/`ocaml foo.ml`）；而文档里抱怨最多的
摩擦点恰好是"必须先有清单/映射"（Dune 没有 `dune-project` 就不干活、Cabal 必须列全模块、
Coq 的 `-Q`+`From`、TS 的配置矩阵）。这直接支持 **Q2 的建议（无清单也允许 import）**。

### 2.4 语言服务器与增量缓存（工程线；决定 §4.4/§4.8/§4.9 的形状）

**LSP 契约（我们必须遵守的部分）**
- `InitializeParams.rootUri` **可以是 null**（"Is null if no folder is open"），
  `workspace/workspaceFolders` 在"只开一个文件"时返回 `null`——**单文件是常态不是错误**，
  协议里**没有** single-file-mode 开关：服务器自己降级。⇒ 我们三层根发现合法且有先例。
- `workspace/didChangeWatchedFiles` + `FileSystemWatcher{globPattern, kind}` 是标准通道；
  编辑器侧已经在 watch `**/*.sokonanoda`，**是我们服务端没接**（§4.9）。
- 诊断由服务器"拥有"、必须自己清理；**"新推送的诊断总是替换旧的，客户端不做合并"**；
  官方明说打开项目时可以"recomputed (**or read from a cache**)"——**LSP 明文允许缓存读诊断**，
  我们 §4.8 的做法有协议背书。

**根发现的各家做法与共同坑**
- rust-analyzer：`rust-project.json` → `.rust-project.json` → `Cargo.toml`，逐级向上；
  都不在则向下看一层；不在任何项目的文件是 **detached file**（只能用 sysroot）；`linkedProjects` 覆盖。
- clangd：`compile_commands.json` 在**源文件的每个祖先目录里找，并且每层还看 `build/` 子目录**；
  找不到就退化成 `clang $FILENAME`——于是出现"一堆假的 `#include` 找不到"；
  覆盖手段 `--compile-commands-dir` / `CompilationDatabase: Ancestors|None|<path>`。
- TypeScript：`configured > external > inferred`；孤立文件拿到 InferredProject；
  在 `node_modules` 里**不向外找**（怕把整个世界拉进来）。
- pyright：**服务端不向上找**（CLI 找），而"流浪的 pyrightconfig/pyproject 会静默胜出"。
- gopls：`go.work` → `go.mod` 向上；都没有则 `AdHocView`（官方原话"no better choice"）。
- ocaml-lsp：`dune-project`/`dune-workspace` 向上；Lean 4 的 VS Code 扩展见 §2.1。
- **共同的坑（我们就按这些设计）**：① 它们**都一路走到文件系统根** ⇒ `$HOME` 里的
  流浪 marker 会捕获无关文件——所以我们的向上搜索必须有**硬边界**（最近 `.git` 或
  workspace 根，§4.4）；② 两个根发布同一个 URI **没有去重**（后发布者胜）⇒
  我们只允许**一个模块根**；③ 错根的典型症状是"假错误 + 什么都不工作"，
  缓解手段是**显式 pin 根**（我们的 `--root` + 编辑器设置镜像）与"报告用了哪个 manifest"。

**失效与产物（我们要抄的部分）**
- Lean/Lake：每个产物旁边一份 `BuildTrace{caption, inputs, hash, mtime}`，`Hash` 是
  **内容哈希**、`mixHash` 链式混入**每个 import 的产物 trace**、`-D` 选项、模块名、包 id；
  trace 不存在时才退回 mtime。**而 `.olean` 自身的版本检查默认关闭**
  （`LEAN_CHECK_OLEAN_VERSION` 默认 OFF）——**正确性其实由 Lake 的 trace 承担**，
  这是"产物头不可信、要另做键"的绝佳反例。
- GHC：接口文件里存**每个声明的 MD5 指纹**，并且记录"编译它时用到的所有指纹"；
  复用条件是源 MD5 未变 + `.o` 比 `.hi` 新，否则**逐指纹比较**（mtime 只是三项条件之一）。
- OCaml：`.cmi` 存**接口摘要**（BLAKE128），一致性按摘要而不是 mtime；
  缺接口报 `Cannot find file mod.cmi`。Coq：`.vo` 里**两个 digest**（前一个覆盖半份文件、
  后一个覆盖全部），import 时 `digest_match` 失败就拒绝加载并报
  "makes inconsistent assumptions over library Y"。
- **64 位哈希被上游自己标为不够**：Lake 的 `Hash = UInt64` 带 TODO
  "Use a secure hash rather than the builtin Lean hash function"——**我们今天的缓存也是
  FNV-1a 64**。⇒ 设计规则：报告缓存沿用 64 位（与现状同一条信任线），
  但**任何"跳过内核重查"的信任台账必须先换强哈希**（§4.8）。
- 原子写：Lean 写 `<name>.tmp.<pid>` 再 `rename`（理由逐字是"neither expose
  partially-written files nor modify possibly memory-mapped files"）——与本仓库现有缓存
  同款；并发方面 **Lake 没有锁文件**（lean4#2445 删掉了），两个 `lake build` 可以竞争，
  这是要避开的（我们：缓存写 best-effort + 原子 rename，不加锁）。

### 2.5 结论：我们采纳什么、明确不采纳什么

| 维度 | 其他系统的做法 | 我们的决定 | 理由 |
|---|---|---|---|
| 单文件默认 | 除 Isabelle 外都能裸文件跑 | **采纳**（不变式 1：无 `import` 逐字节不变） | 初学者路径最短；我们已经是这样 |
| 项目标记与发现 | Agda/Rocq-IDE/Cargo 向上找；Dune/Isabelle 强制清单；**Lean 的搜索路径里没有文件自己的目录**（§2.1） | **采纳"向上找 + 硬边界"**（§4.4）：最近 `sokonanoda.toml`，止于 `.git`/workspace 根，`--root` 覆盖，无清单退化到入口文件目录（**对 Lean 的刻意 divergence，Q2 拍板**） | 编辑器里"打开任意文件就能用"；硬边界避免 `$HOME` 流浪 marker（§2.4 的共同坑）；不用 cwd 做语义 → 判卷/agent 的 cwd 不可控 |
| 模块身份 | 路径推导（OCaml/Python/Coq+映射）vs 声明+校验（Haskell/Idris/Isabelle） | **采纳路径推导**（= Lean 同款） | 一个文件一个规范名，零漂移；声明式要两处同步（Idris 的代价） |
| 清单形态 | Lake 三件套（`lakefile` + `lean-toolchain` + `lake-manifest.json`） | **一个文件 + 可选元数据**（`sokonanoda.toml`；`requires` 对应 `lean-toolchain` 的**意图**，v1 只警告） | 无外部依赖 ⇒ 不需要 manifest/lock 那套；教学只要"标记 + 少量元数据" |
| 产物位置 | 源旁边（Coq/OCaml/GHC）／中央构建目录（Lake/Cargo/dune）／用户缓存（clangd/本仓库） | **保持用户缓存目录**，不引入 `.soko/build/` | 零仓库污染、只读源码树可用、`git status` 干净；代价是不跨机器共享（我们不需要） |
| 缓存键 | GHC/Coq/Lake 都是"内容 + **每个依赖的接口哈希** + 选项/版本" | **采纳**（§4.8 的 iface 链） | 这是"缓存判定结果"安全的三条件之一（§2.4），也是唯一能防"改了依赖却命中"的形状 |
| 哈希强度 | Lake 的 UInt64 被官方标 TODO；Coq/GHC/OCaml 用强摘要 | 报告缓存沿用 FNV-1a 64；**信任台账必须先换强哈希** | 与现状同一信任线；碰撞在"跳过内核重查"下等于判定漏洞 |
| mtime | 几乎所有人都有 mtime 快路径 | **不引入**（键就是内容） | 已有 tmp+rename 原子写与内容键；少一条"时钟/checkout 还原时间"的坑 |
| 环 / 重名 | Go/Rust/Isabelle 硬错；Python 容忍；Node/Python 首命中静默 | **都硬错**，且给环路径 / 两个来源 | 教学价值最高；静默首命中是最坏失败（§2.3 反模式） |
| 包管理 | Rocq `-Q` 双名、Go module path、Cabal 必须列全、`NODE_PATH`/`GHC_PACKAGE_PATH` 式环境根 | **都不做** | v1 没有 registry 需求；环境变量式根是"隐形全局状态"反模式 |
| LSP 侧 | 三层根发现（lean4/gopls/pyright…）；一个文件一个诊断发布者 | **采纳**（§4.9），并接上 `didChangeWatchedFiles` | 与 lean4 扩展同形；避免"两个根抢同一个 URI" |

## 3. 现状审计（今天到底是怎么运作的）

### 3.1 单文件自足：不是口号，是四处代码级假设

| 位置 | 事实（实测/path:line） |
|---|---|
| 语法 | `import Foo.Bar` **今天是语法错误**：`error[parse]: expected a .sokonanoda command, found Token { kind: Ident("import"), … }`（`import` 被 lex 成普通标识符；命令分派在 `crates/front/src/parser.rs:73` `parse_command`） |
| 编译 | `compile_fol` / `check_document` **一个文件 = 一个 `Arena` + 一个 `EnvBuilder`**；prelude 在文件级安装：`crates/front/src/compile/check.rs:429-459`（`PreludeMode::Bare` 什么都不装；`Full` 装 Nat/Eq，且**文件自己有顶层 `inductive Nat`/`Bool` 时让位**） |
| prelude 模式 | 来源是**文件自己**：`-- sokonanoda:prelude none`（`crates/front/src/compile/prelude.rs:36`，`prelude_mode_from_source`）或 CLI `--bare`；45 个语料文件 **0 个**用指令，全部 Full；**9 个文件**自带 `inductive Nat` 显式覆盖 prelude（`course/{unit6,unit7}`、两语镜像、对应解答、`examples/py-nat`） |
| 缓存 | key = `format \| version \| bare? \| text`（`crates/front/src/compile/cache.rs:93`），**只含文件自身**；设计文档原话：「无跨文件依赖（教学文件自给自足，无 import），故 key 只需文件自身」（`docs/design/compile-cache.md:53`），v1 边界明确写「不做与内核 `.olean` 等价的『已编译环境导入』」（同文 §6） |
| Session / 增量 | `front::session` 的 TrustPlan 只在**本文件**内信任已检查前缀（`docs/design/i8-i9.md`）；`DocumentReport`/`DeclState` 没有"声明来自哪个文件"的概念 |
| LSP | 一个文档一个 `Session`、一个 `DocumentReport`；诊断/hover/codeLens/holes 全部按**单文档**下发 |
| 服务 | `sokonanoda watch --workspace <dir>` 已经会递归发现 `*.sokonanoda`，但**每文件一个独立 session、跨文件无全序**，且明文写着「服务是否转播留给后续，v1 不做」（`docs/design/compiler-service-events.md:51-60`；as-built `§8`） |
| 真相层 | `front::query`（I15）的 `QueryDoc` 也是单文件；六个 MCP 工具全部按 `--file` 定位 |
| 团队结论（当时） | `docs/notes/lsp-notes.md:128`：「触发引入 salsa 的条件是：多文件项目、需要交叉文件失效……**到那时再评估**」；`:191`「对我们现阶段是过度设计（单文件课程）」 |

### 3.2 已有种子（这次不用从零造）

- **清单先例**：`course/course.json`（10 个单元的**有序**清单）+ `crates/cli/tests/course.rs`
  已经在遍历四个语料目录——"项目清单"这个概念在仓库里已经存在，只是没有通用化。
- **名字体系**：`Name` 本来就是层级名（`And.intro`、`Nat.rec`），模块名天然可以
  映射成前缀；`PRELUDE_NAMES`（`crates/front/src/compile/mod.rs:25`）已经列出了
  prelude 占用的名字，闭包级冲突检查直接复用。
- **前端已有重复声明错误**：同文件重复 `def x` 报
  `error[elab-duplicate-declaration]: duplicate declaration x`（实测），
  跨模块只需要把"来源文件"带进这条消息。
- **跨文件引用基建**：`crates/front/src/references.rs`（rename/find-references 的种子）
  已经有"从 AST 位置找声明"的形状，跨模块只需让结果带 `ModuleId`。
- **arena 与 builder 的现成能力**：`EnvBuilder` 的 `add_declar`/`declaration_count`/
  `finish`（`crates/kernel/src/builder.rs:238/234/301`）本来就是"按序把声明放进同一个环境"，
  这正是多模块需要的语义——**所以内核不用动**。

### 3.3 一次改造会波及哪里（影响面清单）

| 面 | 现状 | import 之后必须回答的问题 |
|---|---|---|
| parser/AST | `Command` 枚举无 import | 新 `Command::Import`；只能出现在声明之前；错误要教学化 |
| 编译驱动 | 单文件入口 | 谁解析先后顺序？谁决定 prelude 形状？失败模块如何阻断下游？ |
| 报告 | `DocumentReport { decls, events, … }` 单文件 | 报告要按模块分组，且**每个 decl/diagnostic 带 `ModuleId`**（否则诊断指错文件） |
| 缓存 | key 只含自身文本 | 依赖变了必须 miss；依赖没变必须 hit；键怎么算（§4.8） |
| LSP | 单文档 | 项目根从哪来？改动一个库文件后**谁**重编？跨文件跳转/引用/重命名 |
| `query`/MCP | `--file` 单文件 | 查询环境要包含闭包（否则 `state` 看不到 import 来的名字） |
| `watch`/`build`/`course` | 每文件独立 | `build` 要按 DAG 顺序；`course` 的 golden 不能动 |
| 教学面 | 单画布 | 第 11 单元 + 新错误码 hint + `docs/protocol.md` 契约 |

---

## 4. 设计

### 4.1 总览

```
              sokonanoda.toml（项目根标记；可选元数据）
                       │  向上找最近的祖先
   <root>/Playground.sokonanoda
        │  import Lesson.Logic        ← 真实 Lean 4 置顶语法
        ▼
   resolver（模块名 ↔ 路径，Lean 同款）
        │  深度优先 → 拓扑序（依赖在前）+ 环检测
        ▼
   闭包预扫描（收集每个模块的顶层名字）
        │  决定一次 prelude 形状（Nat/Bool/Eq 是否安装、被谁占用）
        ▼
   一个 Arena + 一个 EnvBuilder
        │  for module in topo_order:
        │      parse → elab → try_check_declar（逐个，check-then-add）
        ▼
   ProjectReport { modules: [ModuleReport{module,path,decls,events,diagnostics}], … }
        │
        ├── CLI：人类视图 / `--json`（事件带 module/file）
        ├── 缓存：per-module 报告，键 = 闭包哈希（§4.8）
        └── LSP：每文档诊断照旧，但环境来自闭包；反向后继驱动重编
```

**不变式（写进契约测试）**：
1. 没有 `import` 的文件，编译结果、事件流、缓存键、退出码**逐字节不变**；
2. 每个 `Span` 只属于一个文件（**不做源码拼接**，见 §9.1）；
3. 内核仍然是唯一判定者：跨文件只改变"声明从哪来"，不改变"谁来判"。

### 4.2 D1 —— 语法：`import` 置顶（Lean 4 子集）

```sokonanoda
import Lesson.Logic
import Lesson.Nat

theorem my_lemma : ... := ...
```

- 形态与官方 Lean 完全一致：`import` + 点分模块名，**一行一个**，**必须出现在
  任何声明之前**（Lean 同款规则）。`prelude` 关键字、`import all`、
  `import Foo.Bar.Baz` 之外的形态（别名、路径字面量）**都不做**。
- 词法/关键字：`import` **只在命令位是关键字**，**不进 `is_reserved_command`**
  （`crates/front/src/parser.rs:1221-1237`）——表达式位仍是普通标识符，与 Lean 一致
  （`def import := 1` 合法），也不动既有测试 `command_keyword_inside_expression_is_rejected`
  （`parser.rs:1590`）。实现方式 = `parse_command`（`parser.rs:73-89`）加一个 arm
  （§4.12）。
- 新错误码（都带中文 hint，进 `docs/protocol.md` 的 code 表）：
  | code | 触发 | hint 方向 |
  |---|---|---|
  | `import-must-precede-declarations` | `import` 出现在声明之后 | "Lean 的 import 必须写在文件最上方" |
  | `import-empty` / `import-malformed` | 空、含非法字符、以 `.` 结尾 | 展示合法形态 |
  | `import-not-a-valid-module-name` | 段不是合法标识符（典型：**`-`**，如 `unit1-propositions-proofs`） | "模块名来自路径，Lean 不接受 `-`；改文件名或建一个合法名字的模块" |
  | `import-not-found` | 解析不到文件 | 列出**搜过的模块根**与完整期望路径（Lean 的 unknown module 体验） |
  | `import-cycle` | 环 | 打印环路径 `A → B → A` |
  | `import-name-collision` | 两模块声明同名（见 §4.7） | 给出**两个来源**（文件 + 行列）与"改名或分模块"的建议 |
  | `import-prelude-conflict` | 闭包内 prelude 形状冲突（§4.6） | 说明哪个模块占用/禁用了什么 |
  | `import-dependency-failed` | 依赖模块有解析/检查失败 | 指向**第一个**失败模块的位置（不刷屏） |

### 4.3 D2 —— 模块名 ↔ 路径（Lean 同款，含"dash 非法"教学）

- `import Foo.Bar` ↔ `<module-root>/Foo/Bar.sokonanoda`（大小写敏感；分隔符只用 `.`，
  路径分隔符 `/` 由我们补）。
- 每段必须是合法标识符：字母/`_`/非 ASCII 开头，续字符允许本仓库 lexer 已有的集合
  （`' ! ?` 等），**不允许 `-`、空格、数字开头**。这条不是我们的发明，是 Lean 的
  模块规则；因此 `course/unit1-propositions-proofs.sokonanoda` **不可被 import**——
  这是**特性**：它教用户"文件名就是模块名"，并且编译器给出可操作的建议
  （`import-not-a-valid-module-name`）。
- **只允许一个模块根**（manifest 决定，§4.4），因此不存在"同名模块在两处"的歧义
  （对比 clangd/tsc 的多根歧义，见 §2.4）。
- 平台细节：在大小写不敏感的文件系统（macOS 默认/Windows）上，`Foo` 与 `foo` 不能
  共存——resolver 检测到"期望路径存在但大小写不匹配"时给出**专门提示**，而不是
  报 not-found。

### 4.4 D3 —— 项目根：`sokonanoda.toml` + 最近祖先 + 一条零配置退路

**标记文件**（存在即项目根，内容可以全空；元数据都可选，最小惊讶）：

```toml
# sokonanoda.toml —— 存在即"这个目录是项目根"
name = "my-proofs"        # 可选
requires = "0.57"         # 可选：工具链钉版本（类似 lean-toolchain），不匹配给 warning/error
src = "."                 # 可选：模块根，默认 = 本文件所在目录
```

- **发现规则**：从**入口文件所在目录**向上找最近的 `sokonanoda.toml`；找到就用它，
  找不到就走**零配置退路**：模块根 = 入口文件所在目录（所以"两个文件互相 import"
  不需要任何清单，符合本项目"单文件是默认、项目是 opt-in"的产品性格）。
  > **这是对真实 Lean 的刻意 divergence**（§2.1 实测事实：`lean` 的搜索路径里
  > **没有**文件自己的目录，cwd 只影响模块名的计算，`lean foo.lean` + 旁边的
  > `Bar.lean` + `import Bar` 在官方 Lean 下会直接失败）。理由：教学默认路径要
  > 零配置（§2.3 的结论：初学者最先遇到的语言都能裸文件跑），而且**判卷/教学 agent
  > 的 cwd 不可控**（DSH/编辑器的工作区与进程 cwd 可以不同）——把语义绑在 cwd 上
  > 会制造"同一个文件在不同入口下结果不同"的隐形状态。代价：学生若把同一段
  > `import` 搬进官方 Lake 项目，需要按 Lake 的规则放置文件（这是"知道得更多"，
  > 不是"学到错的"）。**Q2 保留严格对齐 Lean 的选项。**
- **硬边界（调研结论，必须有）**：向上搜索**止于最近的 `.git` 目录或 workspace 根**
  （绝不到文件系统根、更不到 `$HOME`）。理由见 §2.4：clangd/rust-analyzer/pyright/gopls/
  Agda/lean4 **都一路走到 `/`**，于是"`$HOME` 里的流浪 marker 会捕获无关文件"是它们的
  共同坑；我们只在"这个仓库/这个工作区"内认项目。
- **`--root` 与"报告用了哪个 manifest"**：CLI/LSP 都必须能说出**实际生效的根与清单路径**
  （`doctor`/诊断/`soko/project` 里可见）。错根的典型症状是"一堆假错误"（§2.4），
  而**显式 pin 根 + 报告来源**是所有被调研系统共同的缓解手段。
- **覆盖**：CLI `--root <dir>`（显式指定模块根，忽略发现）；LSP 用
  `initialize` 的 `workspaceFolders`/根目录 + 文档路径做同样的向上发现。
- **为什么是"祖先发现"而不是"必须显式"**：这是 cargo/clangd/Agda/gopls 的共同选择
  （§2.4）——编辑器里打开深层文件时不需要用户先告诉工具"项目在哪"，同时
  `--root` 保留确定性；仓库里再加 `sokonanoda.toml` 之后，**本项目自身就是一个项目**
  （`import Lesson.Logic` 立刻可用，教学 agent 的教材库有地方放了）。
- **为什么"空文件也合法"**：标记本身是特性（零学习成本）；TOML 只用来承载可选的
  元数据与将来的 `[deps]`。**这是待拍板 Q1/Q2**（TOML vs JSON vs 纯标记）。

### 4.5 D4 —— 编译模型：一个闭包、一个 arena、拓扑序、失败即阻断

1. **解析闭包**：从入口出发 DFS（按 import 书写顺序），得到拓扑序（依赖在前）；
   重复 import 只算一次；环 → `import-cycle`。
2. **闭包预扫描**：只做语法层扫描，收集每个模块的**顶层声明名**与
   `inductive Nat/Bool` 的存在性。目的：在任何 elaboration 之前就定下 prelude 形状
   （§4.6）与保留名占用，**不碰内核**。
3. **一次 prelude 安装**：`install_prelude`/`install_bool_prelude`/`install_eq_prelude`
   在闭包级调用**一次**（而不是每个文件一次）——这是与今天最大的结构差异，
   也是必须做对的点（否则第二个模块重复安装会撞内核的重复声明 panic）。
4. **逐模块编译**：`for module in topo_order`，每个模块：parse → elab →
   `try_check_declar` 逐个判定 → **check-then-add**（失败声明不进环境，后续声明
   看不到它——语义与今天单文件完全一致）。
5. **开放练习（`sorry`）**：与今天同一条规则——**开放声明的洞不污染环境**，
   下游看不到它；额外在 **import 行**上给一条 info/warning
   （`import-has-open-exercises`：「`Lesson.X` 里还有 N 个未完成的练习，它们对下游不可见」），
   避免"为什么这个名字用不了"的困惑。
6. **失败阻断**：模块 M 有 parse 失败或检查失败时，**所有 import M 的模块不编译**，
   只在它们的 import 行上报 `import-dependency-failed`（指向 M 的第一个错误的
   文件+行列）。理由：不阻断会级联出几十条"未定义标识符"，掩盖真正的第一因。
7. **每模块一个 `ModuleReport`**：`{ module: "Lesson.Logic", path, decls: [DeclState],
   events, diagnostics, open_exercises }`；`ProjectReport { entry, modules, order,
   mode, iface }`。`DeclState`/诊断**新增 `module` 字段**（`Span` 仍是文件内 offset，
   不引入跨文件坐标）。
8. **顺序确定性**：拓扑序 + 稳定 tie-break（同层按 import 书写顺序）写进测试，
   保证事件流可 golden。
9. **与既有两遍语义的接口**（实测细节）：今天的 check-then-add 是"pass 1 先全
   `add_declar`、有失败再用 `skip` 重跑 pass 2 并在 build/add 前短路"
   （`crates/front/src/compile/check.rs:339-359,501-532`）。导入 replay 必须
   **每一遍都按同一顺序重放**（它天然在"本地命令之前"），且 `skip` 键从
   命令下标升级为 **(module, cmd index)**；导入模块自身的失败因此能被精确定位，
   也不会让本地 pass 2 误跳。
10. **可见性索引零改动的理由**：导入声明先入表（索引 `0..k`）⇒ `EnvLimit::ByName`
   的自索引、`#check` 的 `ByIndex(env_before)`、hover 记录的 `env_at` 全部照旧
   正确（`crates/kernel/src/env.rs:210-216,280-287`；`check.rs:465,1134,1652`）。
11. **内核的能力边界（决定了 v1 策略）**：内核里唯一的可见性原语是
   `cutoff: usize`（`crates/kernel/src/env.rs:224-234` 注释自陈
   "control visibility … only making a particular slice of that environment
   available"）——**环境就是一条扁平的声明序**。因此：
   - **可表达**：扁平、拓扑序的 import 闭包（本设计）；
   - **不可表达**（必须由 front 定策略）：跨文件**互递归**、**菱形导入 + 遮蔽**、
     两套并列命名空间。v1 策略 = 扁平 + 重名即错（§4.7）+ 环即错（§4.2）；
     跨文件互递归与 `namespace` 一并留 P7，且**需要先有设计再谈实现**。

### 4.6 D5 —— prelude 与 Bare 的**闭包**语义

今天 prelude 是**文件级**的（`crates/front/src/compile/check.rs:429-459`：文件自带
顶层 `inductive Nat` 就让位）。多文件必须把它提升为**闭包级**：

- **模式由根（入口）文件决定**：`-- sokonanoda:prelude none`（或 `--bare`）在入口
  上 → 整个闭包 Bare；依赖文件自身的模式指令**不参与决策**，与入口不一致时给
  `import-prelude-conflict`（提示"prelude 是整个编译单元的属性"）。
- **让位规则闭包化**：闭包里**任一模块**有顶层 `inductive Nat` → 不装 prelude Nat；
  `Bool` 同理；`Eq` 今天"总是装、减去被占用的顶层名"，改为"减去**闭包并集**里被
  占用的名字"。9 个自带 `inductive Nat` 的语料文件因此语义不变（它们本来就不
  靠 prelude Nat）。
- 这条让"共享 prelude 模块"这个教学模式自然成立：`Lesson/Nat.sokonanoda` 里写
  显式 `inductive Nat`，所有 import 它的画布共享**同一个** Nat（而不是各自装一份）。
- **实现要求（实测的崩溃点）**：三处判定今天都是文件级——`prelude_mode_from_source`
  （`crates/front/src/compile/prelude.rs:39-59`）、`user_top_level_names(file)`
  （`check.rs:322-336`）、`prelude_shape(file, mode)`（`session.rs:371-396`）；
  且安装点的重复声明是靠 `.expect(...)` 崩的（`prelude.rs:120,182,249,265`）。
  闭包化时必须同时做到：**判定用闭包并集**、**安装幂等**（已存在则跳过），
  否则第二个模块就 panic——这是 P2 的第一条"修复前红"测试。

### 4.7 D6 —— 名字可见性：扁平 + 冲突教学错误（`namespace` 留 v2）

- `import Foo` 把 Foo 的声明**全部**放进环境（Lean 的传递可见语义，无 export 列表、
  无 `private`）。跨模块**重名 = 错误**（`import-name-collision`），消息里必须
  同时给出两个来源（文件 + 行列），因为"哪里重了"是初学者唯一能行动的信息。
- **为什么不做 `namespace`**：它是真实 Lean 语法但需要"课程 + 测试 + 白名单"三件套，
  而 v1 的真实需求（消除 20 处跨文件歧义）用"模块边界 + 重名报错"已经满足。
  `namespace` 进 P6/后续 I 项；届时的规则建议（保留给未来）：`namespace X` 只影响
  声明名的前缀，不改变 import 语义。
- PM 与内核交互点：跨模块重名若逃过前端检查会撞内核的重复声明（panic → 
  `kernel-rejected` 的反人类文案），所以**前端必须在入环境前查**（复用
  `PRELUDE_NAMES` + 闭包预扫描的名字表），并为此写"内核不该被惊动"的测试。

### 4.8 D7 —— 缓存：闭包哈希（Merkle），报告按模块存

今天的键 `format|version|bare?|text`（`compile/cache.rs:93`）**必须升级**，否则改一个
被 import 的库文件不会让下游 miss —— 那是"缓存给出错误结论"，违反硬规则 8 的
精神（判定永远走 kernel，缓存只省重复劳动）。设计：

```
iface(module) = H( CACHE_FORMAT,
                  engine_version,          # CARGO_PKG_VERSION
                  closure_prelude_shape,   # Full/Bare + Nat/Bool/Eq 是否安装
                  source_text(module),     # 本模块源文本（含 `--` 注释）
                  [iface(dep) for dep in imports(module)]  # 按 import 书写顺序
                )
```

- **报告缓存键 = `iface(入口文件)`**：无 import 时退化为今天的 `text` 键（**同一个
  哈希函数、同一份目录**，所以既有缓存条目与 warm-cache 测试行为不变）。
- **按模块存**：`<cache_root>/compiled/<iface>.json` 存 `{format, report}`，
  `report` 内含该模块的 `ModuleReport`；`build` 可以预热整个项目的每个模块。
- **失效语义**：改动 `Lesson/Logic.sokonanoda` → 它的 `iface` 变 → 所有 import 它的
  入口 `iface` 全变 → 必然 miss；只改注释也会 miss（与本仓库今天的行为一致，
  简单且可预测；不做"注释无关"的优化）。
- **不做**（v1）：不缓存内核环境（§9.2）、不做 LRU、不做跨机器共享。
- **内容寻址的一个细节**（实测）：今天的键**不含路径**（`compile/cache.rs:83-118`），
  条目里没有环境、只有报告（`:27-31`）。多文件下这仍然 sound——因为闭包内容相同
  ⇒ 编译结果相同；但**报告里不得存绝对路径**（否则同一内容在移动过的项目里会
  命中"旧路径"的报告）。因此 `ModuleReport` 存**规范模块名**，路径在读取时由
  resolver 重新推导；CLI 的人类视图本来就另带 `label`（`crates/cli/src/check.rs:11`）。
- **P4 可选优化（信任台账）**：`<cache_root>/verified/<iface>` 记录"这份 iface 的声明
  曾被内核判过"。编译一个依赖模块时，若其 `iface` 已在台账里，**跳过
  `try_check_declar`**（仍然重新 elaborate，仍然逐声明入环境）。soundness 论证与
  今天的缓存同一条：相同 (版本, prelude 形状, 源文本闭包) 下 elaboration 是纯函数，
  内核判定过就是判定过。**默认开启前必须先用 `SessionUpdate.stats.kernel_checks`
  这类可观测量证明收益**（I8 TrustPlan 的先例）。
- **哈希强度（信任台账的前置条件）**：报告缓存沿用既有的 FNV-1a 64（与现状同一条
  信任线：碰撞最坏是给出陈旧报告）；但**信任台账一旦启用，iface 链必须换成
  128/256 位强哈希**（SHA-256/BLAKE3）——因为那里"碰撞 = 内核不再重查 = 判定漏洞"，
  而调研里 Lake 的 `Hash = UInt64` 正带着官方 TODO "Use a secure hash…"（§2.4）。
  换哈希 = 新依赖，故它被绑在 P4 的可选项上，不拖累 P1–P3。

### 4.9 D8 —— 表面：CLI / LSP / query / MCP / watch / build

| 面 | v1 行为 |
|---|---|
| `sokonanoda <file>` | 有 `import` 时按项目闭包编译（人类视图每条诊断前缀 `path:line:col:`，今天已是这个形状）；**无 import 时逐字节不变** |
| 新旗标 | `--root <dir>`（覆盖模块根）；`--no-project`（强制单文件语义，用于诊断/回归） |
| `--json` | 事件**只增字段**：`module`（+ 既有 `span` 保持文件内坐标）；新增 `project.file`/`project.summary` 仅在项目模式下出现 |
| `build [path…]` | 项目化：发现项目 → 按 DAG 顺序编译每个模块 → 预热缓存；人类行保持 `built K file(s) — H hit, M compiled, F failed` 形状（`K` 的含义在项目模式下 = 模块数）；`--json` 增 `deps`/`module` 字段 |
| `query <op>` | `--file` 的环境包含其闭包（`state`/`goals` 能看到 import 来的名字）；新增 `--root`；**信封与退出码不变**（`soko.query/1`，0/1/2） |
| `watch --workspace` | **语义不动**（每文件独立、跨文件无全序）；项目感知的 watch 由 LSP 承担（v1 不扩 CLI watch，避免协议震动） |
| LSP | ① 项目根发现（manifest 向上 / workspace 根 / 单文件三层；`initialize` 现在**丢弃参数**（`crates/lsp/src/lib.rs:563`），必须捕获 `rootUri`/`workspaceFolders`）；② `Doc` 从**单槽 `Mutex<Doc>`**（`:58-60,152-155`，19 处 `self.doc.lock()`）改为按 URI 的多文档表，`did_close`（`:637` 空实现）负责清理，publish 按 URI；③ 变更一个库文件 → **反向后继**重编（防抖 + 只 publish 已打开文档），并挂 `didChangeWatchedFiles`（客户端已 watch `**/*.sokonanoda`，`editor/vscode/extension.js:1383`，服务端无 handler）；④ 跨文件 `goToDefinition`/`findReferences`/`rename`——结论里的目标要带文件身份（`compile/report.rs:131-133` 今天明写 "same file"），四个处理器不能再把请求 URI 贴到单槽结论上（`:838,1034,1055-1060,1069`）；⑤ `code_action` 路径把文档切片喂 `judge_infer`（`lsp/src/lib.rs:480-495`、`actions.rs:39-41`）⇒ judge 的导入上下文必须一起接；⑥ `soko/goals`/`stateAt`/`hints` 在闭包环境里求值，并真正解析 `textDocument.uri`（`protocol.rs:14` 等今天是死字段）；⑦ 新增只读请求 `soko/project`（根、manifest、模块表、依赖边、每模块状态）供扩展画/导航（可选，P5）；⑧ LSP 是**多线程 tokio**（`:1093`）而内核 `catch_unwind` 会换全局 panic hook ⇒ 子 agent 报告的"抢 hook"风险必须在这层防（编译串行化在 per-document 锁内，不要并行跑多份内核检查） |
| MCP | 六个工具**签名不变**，语义变为"在该文件的闭包环境下回答"；`dsh/README.md` 补一句 |
| `course` | **不动**（golden 表是硬门禁） |

### 4.10 D9 —— 教学面：第 11 单元 + 白名单三件套

- 新课程单元 **11「模块与项目」**（`course/unit11-modules-projects.sokonanoda` + EN 镜像
  + `course.json` + 两处 golden 表 + 解答）：模块名 = 路径、`import` 置顶、
  `sokonanoda.toml` 是什么、为什么 Lean/Lake 这样组织、5–8 个练习（含"修一个
  非法的模块名"与"两个模块重名怎么办"）。
- **白名单三件套**（硬规则 4）：语法点（`import`）→ 课程单元（第 11 单元）→ 测试
  （front 单测 + CLI e2e + 课程语料）+ `docs/protocol.md` 的错误码表。
- 画布纪律（产品规则，写进 teacher 技能）：**单文件仍是默认教学表面**；
  只有当"教材库复用"或"期末项目"出现时才升格为项目，且**判卷命令不变**
  （`scripts/soko grade playground.sokonanoda`）。
- 语料重构（把 `solutions/` 变成 `import UnitN` + 答案）**不在 v1**：它要重钉 4 处
  golden/清单断言，收益（省行数）小于风险。列入 Q5 单独决策。

### 4.11 D10 —— 工程与发布

- **模块布局**（REQUIREMENTS §4：新代码按 ~500 行/文件切，不许再养巨石）：
  `crates/front/src/project/{mod,manifest,resolve,graph,iface,report}.rs`
  （职责：清单解析 / 模块名↔路径 / 闭包与环 / 哈希 / 项目报告）；
  parser 只加 `Command::Import` 与置顶校验；编译驱动加
  `compile_project(...) -> ProjectReport`（**不改** `elab.rs`/`check.rs` 的主体，
  只在既有 per-file 入口旁加一条闭包入口）。
- **依赖**：manifest 需要 TOML 解析。选项：`toml` crate（+serde，与 Lake 风格一致）
  vs `serde_json`（零新依赖但不可注释）vs 纯标记文件（零解析）。**待拍板 Q1**；
  推荐 `toml`（教学友好、Lean 同款），并在 P2 里把它锁进 `Cargo.lock`、走 CI。
- **零工具链不变**：用户路径仍是 Release 二进制/VSIX；TOML 解析在编译期进二进制，
  不引入运行期依赖、不联网。
- **版本**：用户可见的新能力（`import` + 项目）= **minor bump**（0.56.1 → 0.57.0），
  落地那一轮同时 bump `Cargo.toml` 与 `editor/vscode/package.json`，
  并按"收尾义务"同步 `skills/`、VS Code README/CHANGELOG、`site/`（若有用户可见文案）。
- **性能门禁**：项目模式新增 `crates/front` 的 perf 冒烟（10 模块 × 每模块 20 声明的
  闭包编译 + warm cache 命中），进 `docs/PERF.md`；**内核热路径零改动**。

### 4.12 代码接缝与改动清单（全部实测 `path:line`）

> 本节是本轮 subagent 的接缝勘察结论，也是 P1–P5 的**执行底稿**：每一条都能直接
> 定位到函数。**内核不在表内（冻结）**。

**贯穿全篇的三个结构事实**（设计必须围着它们转）：
1. **一次编译 = 一个 arena + 一个 `EnvBuilder`**：`run_pass` 顶部成对创建
   （`crates/front/src/compile/check.rs:424-425`），`builder.finish()` 消费成
   `ExportFile`（`check.rs:1192`）；每条声明的可见性由 `EnvLimit::ByName` 换算成
   **名字上记录的入表索引** 决定（`crates/kernel/src/env.rs:210-216,280-287`）。
2. **内核的名字身份 = 指针身份**：`NamePtr` 的 `Eq/Hash` 就是地址
   （`crates/kernel/src/util.rs:133-142`），`decl_idx` 只由 `EnvBuilder::add_declar`
   写入且是 `pub(crate)`（`builder.rs:238-247`，`name.rs:100`）。⇒ **跨 arena 合并
   环境在今天的 API 下不可能**（§9.3），而"同一 builder 按序 `add_declar`"是唯一
   零内核改动路径。
3. **prelude 是每次编译现场安装的受信任声明**（`compile/prelude.rs:1-2`），
   安装点全部用 `.expect(...)` 落地（`prelude.rs:120,182,249,265`）——重复安装 =
   `add_declar` 返回 `Err` → panic。

**必须改的接缝（按 crate）**：

| 位置 | 改什么 | 关键点 |
|---|---|---|
| `parser.rs:73-89` `parse_command` | 加 `import` arm + `parse_import` | 模块路径可直接吃 `Ident("Foo.Bar")`（`.` 已是标识符续接字符，`token.rs:267-273`）；**不要把 `import` 加进 `is_reserved_command`（`parser.rs:1221-1237`）**——否则 `def import := 1` 这类表达式位用法被禁，且会动既有测试 `command_keyword_inside_expression_is_rejected`（`parser.rs:1590`）。D1 的"仅命令位"由此一条实现 |
| `ast.rs:204-254` | `Command::Import { module, span }` | 顺手补 `Command::span()` 与全部穷尽 match：`compile/check.rs:322,333,1180,1539,1566`、`warning.rs:67`、`semantic.rs:181,471`、`goals.rs:58-130`、`proof.rs:48` |
| `compile/check.rs` `run_pass`（`:338` 起） | **闭包命令序列**替代单文件命令序列 | 导入模块的命令必须在任何本地命令 `add_declar` **之前**入表（索引 `0..k`），本地从 `k` 起——这样 `EnvLimit::ByName`/`ByIndex(env_before)`/hover 的 `env_at`（`:599,609,1134,1167,1422,1652`）**全部无需改动** |
| 同上，两遍语义（`:339-359`） | 导入 replay 必须**逐遍确定性重放** | today：pass 1 把能 elaborate 的声明全 `add_declar`（注释自陈不 sound `:339-343`），有失败就用 `skip` 重跑 pass 2（`:344-359`），pass 2 在 build/add 前短路（`:501-505,522-532,676-686,819-829,921-926`）。导入的 `skip` 键要从"命令下标"升级为 **(module, cmd index)** |
| `compile/prelude.rs:39-59,90-120,132-249` | **幂等化 + 闭包化** | `prelude_mode_from_source`（文件级）、`user_top_level_names(file)`（`check.rs:322-336`，文件级）、`prelude_shape(file,mode)`（`session.rs:371-396`，文件级）三处都要闭包级；安装改为"已存在则跳过"而不是 `.expect` panic |
| `compile/cache.rs:83-118` | key 纳入闭包摘要 + `CACHE_FORMAT +1`（`:23`） | 今天 key = `format+version+build_stamp+prelude_bare+src`（纯内容寻址、**无路径**）；条目只存报告（`:27-31`）⇒ 命中**不能**给下游提供环境 |
| `session.rs:74-78,94-97,260-269` | `TrustPlan` 增"闭包未变"前置；`DeclKey`/`prefix_failures` 增文件身份；`prelude_shape` 项目级 | 模块级不变式（`session.rs:3-5`）要从"本文件命令文本不变 ⇒ 前缀语义不变"升级为"**闭包**不变"；`recompiled_from`/`stats.kernel_checks` 的对外承诺需要新语义（谁被重查） |
| `judge.rs:99-107,137-148,265-270,400-406` | `judge_terms`/`judge_infer`/`judge_hole_fill` 增导入上下文，并进 `judge_cache_key` | **最容易漏的一条**：judge 只吃 `prefix_src: &str` 并在内部重新 `parse_prefix` + 整篇编译（`:546-548`）⇒ 不传导入上下文的话，tactic 判定看不到 import 来的声明，而且**缓存会跨导入变更复用过时结论** |
| `query/mod.rs:40-52,118` | `QueryDoc` 增 root/闭包；`check()` 与 `set_text()` 必须用**同一份**闭包 | `check()` 现在绕开 session 直接重算（`:118`）；两条路径不一致会破 `query check ≡ --json` 计数契约（`crates/cli/tests/query.rs:84`） |
| `cli/src/main.rs:31-65,68-73` | `--root` / `--no-project`（照 `--doc` 模板） | 路径在 `check_source(src,label,…)`（`cli/src/check.rs:11`）就已丢失、`query::load_source` 只回 `String`（`cli/src/query.rs:109-135`）⇒ `--text` 无位置：**`--text` + `import` 必须显式 `--root`**（或明确报错） |
| `lsp/src/lib.rs:58-60,152-155,619-637,825-1078` | `Doc` 单槽 → `HashMap<Url, Doc>`；`initialize` 存 root；`did_close` 清理；按 URI publish；`didChangeWatchedFiles` | 现在是**一个 `Mutex<Doc>` 单槽**（19 处 `self.doc.lock()`）、`initialize` 丢弃参数（`:563`）、`did_close` 空实现（`:637`）、结论处理器把请求 URI 贴到单槽结论上（`:838,1034,1055-1060,1069`）；crate 内非测试代码**零文件 IO**，URI→路径→模块映射要从零建 |
| `compile/report.rs:131-133` | `ResolvedTarget::Declaration` 增文件身份 | 文档注释明写"same file"；跨文件跳转/引用/rename 全靠它 |
| `docs/protocol.md` + `crates/cli/tests/common/mod.rs:12-23` | 新错误码 + 新事件词汇 | 事件词表有封闭断言（`protocol.rs:336`），任何新事件必须**同轮**进两处；错误码有**两处**门禁：`compile/tests.rs:1410 protocol_doc_lists_every_error_code`（35 项数组 + **无通配 match**，新 variant 会让编译失败）与 `compile/tests.rs:753` 的 23 项 `matches!` 列表（**不会自动覆盖，必须手工补**） |
| `semantic.rs` 关键字表 + `editor/vscode/syntaxes/sokonanoda.tmLanguage.json` + LSP legend/补全 | `import` 高亮/补全三处同步 | 关键字是单一真相 + 交叉校验：`crates/cli/tests/extension.rs:1028`（TM grammar 必须跟 `semantic.rs`）、`crates/lsp/src/tests/tokens.rs:4`（legend 全量）、`crates/lsp/src/tests/navigation.rs:53`（补全列表）——**同一提交**改三处，否则 CI 红 |
| `docs/design/{compile-cache,course-status,compiler-service-events}.md`、`docs/HANDOVER.md:184-185`、`ROADMAP.md:374`、`docs/notes/lsp-notes.md:103,128,191` | 显式推翻/更新"单文档是刻意取舍"的记录 | LSP 单文档是**被负向断言保护的架构决策**（`crates/cli/tests/extension.rs:201-204` 断言脚本**不含** `soko/courseStatus`）——P5 不是"接线"而是推翻决策，必须同轮改文档与断言 |

**会**静默**出错（不报错、不失败，但结论错）的三道门**——比测试变红更危险，必须在 P2/P5 显式处理：

| 门 | 症状 | 处理 |
|---|---|---|
| `crates/front/tests/perf.rs:70` `incremental_edit_anywhere_is_fast`（`:108` `kernel_checks <= 1`） | 非编辑区域影响环境时（**被导入文件变了**正是如此），这个断言会失效；它今天**没有依赖概念** | P2 重写语义（"闭包内被编辑的后缀"），并在 `docs/PERF.md` 记账；不要只调阈值 |
| `crates/cli/tests/watch.rs:292-298`（`b's version must not be coupled to a's`） | 把"文件间无耦合"写成契约；imports 在**项目模式**下正好反转它 | v1 保持 `watch` 每文件独立（§4.9）→ 契约不动；项目模式的耦合只在 LSP/`build` 里发生，设计文档显式声明这条边界 |
| `crates/front/src/judge.rs:137-148` + `suggest.rs` 的 25 条测试 | 导入没折进 `prefix_src`/`judge_cache_key` 时**不报错、只是静默没有建议**（`judge.rs:17-19` 已记录过 prefix/periphery 分歧的先例） | judge 三个入口（`judge_terms`/`judge_infer`/`judge_hole_fill`）必须吃导入上下文并进缓存键；用"导入来的名字能被 suggest 命中"写判别性测试 |

**另外三条只有本地才知道的坑（CI 不会替你发现）**：

- **LSP 的"URI 盲"是被测试钉死的现状**：`crates/lsp/src/tests/perf.rs:87-102` 打开
  `file:///test.sokonanoda`，却用 `file:///perf.sokonanoda` 请求并断言有 `decls`
  ——按 URI 建 map 之后这条测试必须改成"未知文档"的期望（它钉的是现状，不是遗漏）。
- **缓存的错误命中在 LSP 层测不出来**：`crates/lsp/src/lib.rs:113-116` 有
  `if cfg!(test) { None } else { cache::load(...) }` ⇒ **跨文件串味只有 CLI 侧测试
  能发现**（`crates/cli/tests/cli.rs:1664,1752` 是落点）。这也是"把闭包折进内容键、
  而不是路径键"的又一个理由（§4.8）。
- **画布/锚点的守卫只在本地**：CI 里**没有** anchor 步骤（`grep -c anchor .github/workflows/ci.yml` = 0），
  唯一执行点是 `scripts/soko gate` → `crates/cli/src/env/mod.rs:278-292`；
  而站点构建 `scripts/gen-site-demos.py:149,462` 也会渲染画布
  （`pages.yml:77-81` 只守"产物新鲜"）。⇒ 如果画布有一天加了 `import`，
  **CI 与 Pages 都不会以"画布不再自包含"报警**——A1 的逐字节对拍必须真的落地，
  不能指望既有流水线。
- 顺带两笔账（P6 文档同步时一起修）：`docs/TESTING.md` §1 指向
  `crates/lsp/src/cache.rs`（**该文件不存在**；真实缓存键测试在
  `crates/front/src/compile/cache.rs:210,232,258`）；fuzz 目标
  `fuzz/fuzz_targets/parse_never_panics.rs` 在 parse 成功后跑完整
  `check_document`，而 **CI 从不跑 fuzz** ⇒ 新增顶层 `import` 命令扩大了这个
  "有 fuzz 目标但无 CI 反馈"的面，P1 顺手跑一次本地 fuzz。

**会挡路的既有测试/断言（先知道，才不会"改完才发现"）**：

| 测试 | 为什么会挡 |
|---|---|
| `crates/cli/tests/watch.rs:255` `watch_workspace_tracks_each_file_with_independent_versions` | 把"编辑 a 不动 b"写成断言——**项目语义与它直接冲突**；v1 保持 `watch` 每文件独立（§4.9）所以这条可以不动，但设计必须显式声明 |
| `crates/lsp/src/testutil.rs:33,218` | 单一硬编码 `URI`，`wait_diagnostics` 以它做就绪屏障；`handshake` 只发空 capabilities 并对 8 个能力等值断言（`:87-123`）⇒ P5 要动这套夹具 |
| `crates/lsp/src/tests/perf.rs:4-116` | 时延门（50ms/10ms）按**单文件**编译设定；多文件闭包会打破 ⇒ 门槛要么按单文件保持、要么显式重设并在 `docs/PERF.md` 记账 |
| `crates/front/src/session.rs` 的 19 条增量测试 | `session_rechecks_only_the_affected_suffix` 等按"命令下标"表述，闭包引入后要加"依赖变更"用例而不是改旧断言 |
| `crates/cli/tests/course.rs:86-97,114`、`course_status.rs:65-77` | 两处 inline `GOLDEN` 表 + `files.len() == GOLDEN.len()` 硬等值：**课程语料不动就是零漂移**（§4.10），这是 A1 的守门人 |
| `crates/front/src/parser.rs:1590` | `command_keyword_inside_expression_is_rejected`：`import` 不进保留字表就不受影响（见表内说明） |
| `crates/cli/tests/query.rs:556,587` | CLI≡LSP 字段级一致性；P5 后必须**在多文件下**同样成立 |

**改动量估算**（不含测试）：`front` 新增 `project/*` 约 400–700 行 + 既有文件挂接
150–300 行；`cli` 75–150 行；`lsp` 150–300 行；测试 400–800 行。
`CompileOptions` 目前 `#[derive(Copy)]`（`compile/prelude.rs:30-34`）且有 12 个构造点
（CLI 6 + LSP 2 + …），加入 root/闭包字段后 **`Copy` 会失效**——这是有意的：
项目上下文本来就不该是 `Copy` 的小值，P1 里一并处理。

---

## 5. 分阶段计划（每阶段独立可交付、可验收）

> 原则与 I15（`agent-query-channel.md`）一致：**先定契约与真相层，再上传输与表面**；
> 每阶段先写"修复前红"的测试；每阶段的验收命令必须能直接粘贴执行。

### P0 —— 设计定稿与契约（本轮，只出文档）
- **交付**：本文（含 §2 调研、§4 设计、§5 计划、§6 验收、§8 待拍板）+ `ROADMAP.md` I16 +
  `REQUIREMENTS.md` §9 + `STATUS.md` 本轮 + `docs/README.md` 设计索引。
- **不做**：一行代码、一次 bump（沿用第八十八轮"设计轮不 bump"先例）。
- **验收**：文档自洽；Q1–Q6 有明确推荐；每条设计决策都能指回一条事实（path:line 或实测）。

### P1 —— 语法与解析（不动编译驱动）
- **交付**：`Command::Import`（AST）+ parser 置顶校验 + 模块名合法性校验 +
  `import-not-a-valid-module-name` / `import-must-precede-declarations` /
  `import-malformed` 三个错误码与 hint；`crates/front/src/project/resolve.rs`
  （模块名 ↔ 相对路径、大小写提示、单根查找，**纯函数**，不读项目）。
- **测试**：front 单测（合法/非法模块名矩阵、置顶规则、`import` 在表达式位的行为）
  + CLI e2e（`import` 报错文案与退出码 1）。
- **硬约束**：**无 import 的文件逐字节不变**（用 `--json` 快照对拍 HEAD）。
- **验收**：`cargo test -p sokonanoda-front -p sokonanoda-cli --locked` 全绿；
  新增测试 ≥12 条；golden 计数零漂移。

### P2 —— 闭包编译（核心）
- **交付**：`project/{manifest,graph,report}.rs` + `compile_project()`：
  清单解析（TOML/JSON 按 Q1 定）、祖先发现、DFS 拓扑序、环检测、闭包预扫描、
  **一次 prelude 安装**、逐模块 check-then-add、`ModuleReport`/`ProjectReport`；
  CLI `sokonanoda <file>` 走闭包入口（无 import 时走原入口）。
- **测试**：两文件/三文件/菱形依赖/环/自环/重复 import/依赖失败阻断/
  开放练习不入环境/闭包 prelude 四种形状（Full、Bare、显式 `inductive Nat`、
  冲突）；每条都同时写 front 单测与 CLI e2e。
- **验收**：`--root` 与零配置退路都能编译 `import`；诊断文件归因正确（A4）；
  `cargo test --workspace --locked` 全绿、45 个语料文件行为不变。

### P3 —— CLI 与协议表面
- **交付**：`--root` / `--no-project`；`build` 项目化（DAG 顺序 + 新 `--json` 字段）；
  `query <op>` 在闭包环境下求值；`docs/protocol.md` 增补模块字段与全部新错误码；
  `docs/TESTING.md` 增补项目层的跑法。
- **测试**：`crates/cli/tests/{query,build,protocol}.rs` 扩展（**只增不改**既有断言）；
  `query check` 计数 ≡ `--json` 事件计数在**多文件项目**下依然成立。
- **验收**：A5/A6 的命令全部通过；六工具 MCP 输出信封不变（`soko.query/1`）。

### P4 —— 缓存与失效
- **交付**：`project/iface.rs`（闭包哈希）+ 缓存键升级（无 import 时与今天同键）
  + per-module 报告落盘 + `build --clean` 覆盖项目条目。
- **测试**：依赖变更必 miss / 无关文件变更不 miss / 版本变化必 miss /
  warm cache `--json` 逐字节一致 / 缓存损坏 best-effort 不失败。
- **可选**：信任台账（跳过内核重查）——**先测收益**（`stats.kernel_checks`），
  收益不显著就不做（避免无谓的信任面）。
- **验收**：A6；`docs/design/compile-cache.md` §2/§4/§6 同步为"闭包键"版本。

### P5 —— LSP 与编辑器
- **交付**：项目根发现（`initialize` 捕获 `rootUri`/`workspaceFolders`）；`Doc` 单槽 →
  `HashMap<Url, Doc>` + `did_close` 清理 + 按 URI publish；反向后继图与重编调度
  （防抖、只 publish 已打开文档）+ `didChangeWatchedFiles` handler；跨文件
  `goToDefinition`/`findReferences`/`rename`（结论带文件身份）；`code_action` 的
  judge 上下文；`soko/*` 开始真正解析 `textDocument.uri`；`soko/project`（可选）；
  VS Code 侧只做必要同步（README/CHANGELOG/package.json 版本；若加状态展示，
  走 `docs/vscode-dev-guide.md` 的测试三层）。
- **测试**：`crates/lsp/src/tests/` 新增 `project.rs`（真实 LSP 会话：跨文件跳转、
  改库刷新下游诊断、环/缺失模块的诊断不崩、`soko/project` 形状）；
  **夹具必须一起改**：`testutil.rs:81` 的 `initialize` 要能带 rootUri、
  `testutil.rs:32/218` 的诊断屏障要按 URI 过滤（今天开第二个文档会**挂起**而不是
  明确失败）、`tests/perf.rs:87-102` 的"URI 盲"断言按新语义改写。
- **验收**：A7；`cargo test -p sokonanoda-lsp --locked` 全绿（117 条里除上述
  URI 盲断言按新语义改写外，其余不动）。

### P6 —— 教学面与发布
- **交付**：第 11 单元（CN/EN + 解答 + `course.json` + 两处 golden 表）+
  `skills/sokonanoda-{teacher,dev}` 的 import/项目章节 + `AGENTS.md` 速记 +
  `dsh/README.md` 一句 + `site/`（如涉及用户可见文案）+ 版本 0.57.0 +
  `docs/HANDOVER.md`/`STATUS.md`/`REQUIREMENTS.md` §9 收尾。
- **验收**：A8 + `scripts/soko gate` PASS + 全量 `cargo test --workspace --locked` 全绿。

### P7 —— 后续（不在本轮承诺范围，登记为 backlog）
- decl 级编译产物（跨进程复用已检查环境，`.olean` 等价物）；
- `namespace`/`open`（真实 Lean 子集，需各自三件套）；`import all`；
- 跨项目依赖（`[deps]` 的 path/git 形态）与 `sokonanoda new` 脚手架；
- 课程语料重构（`solutions/` 改为复用 + golden 重钉）；
- `watch` 的项目模式（协议要不要带 DAG 顺序，见 Q6）。

---

## 5.1 as-built（2026-09-18 落地，版本 0.57.0）

用户 2026-09-18 指示「新产生一个 git 分支，全部按照建议，你给我完整做完一版」——
§8 的 Q1–Q7 全部按推荐执行。**P0–P6 全部落地**，P7 仍是 backlog（本轮未承诺）。

**与本文设计的偏差（三条，都是实现时才发现的事实）**

1. **§4.12/P4 里的 `project/iface.rs` 没有单独成文件**：闭包摘要就是
   `ProjectPlan::digest(&CompileOptions)`（`crates/front/src/project/mod.rs`），
   没必要为 30 行多开一层模块。其余文件名与设计一致。
2. **P5 只做到"多文档 + 跨文件定义"**：`initialize` 捕获 root、
   `Docs{map,order,root,active}`、按 URI publish、`goto_definition` 跨文件都完成；
   **反向后继图 / 重编调度 / `didChangeWatchedFiles` / `soko/project` /
   跨文件 `references`+`rename` 未做**——第一版"依赖变更后重编译其它打开文档"
   会在 tower-lsp 的串行通知 + 客户端 socket 缓冲下挂住（已回退，测试删除）。
   当前语义与余项的补测起点登记在 `docs/TESTING.md` §5.7 与
   `crates/lsp/src/tests/project.rs` 文件头。**这是本轮唯一的能力缺口。**
3. **`import-prelude-conflict` 的判据细化**：没有 prelude 指令的模块视为
   **继承**入口模式，只有"显式指令与闭包决定不一致"才报错（设计 §4.6 只写了
   "闭包内不一致"，实现时需要区分"未声明"与"显式声明"两次预扫描）。
   另有两条实现期决定：`import` 必须加入 `is_reserved_command`（否则行首
   `import` 会被当成应用的实参吞掉）；加载器的后序 `visit` 返回
   `VisitOutcome::Cycle`，保证入口在拓扑序最后。

**交付物（可核对）**

| 层 | 位置 | 规模 |
|---|---|---|
| 语法/解析 | `crates/front/src/parser.rs`、`token.rs`、`ast.rs`（`Command::Import`） | 3 个 parse 期错误码 |
| 闭包编译 | `crates/front/src/project/{mod,module_name,resolve,manifest,graph,report}.rs` | 6 文件 + 18 单测 |
| 编译驱动 | `crates/front/src/compile/check.rs`（`units: &[SourceUnit]`、`split_report`、命令下标归因） | 内核零改动 |
| 缓存 | `ProjectPlan::digest` + `cache::key`（`CACHE_FORMAT` 1→2） | `docs/design/compile-cache.md` §7 |
| CLI/协议 | `--root`/`--no-project`、`build` 项目化、`query` 闭包、`help.rs` 多文件段 | `crates/cli/tests/imports.rs` 12 e2e |
| LSP | 多文档、按 URI publish、跨文件定义 | `crates/lsp/src/tests/project.rs` 3 e2e |
| 教学面 | `course/unit11-modules-projects.sokonanoda`（+EN+solution）、`course/unit11-project/` 可运行两文件项目、`course.json` | golden：画布 (7,6,0)、solution (12,0,0) |
| 文档 | `docs/architecture.md` §4.5/§8.10、`docs/protocol.md`、`docs/TESTING.md` 三行 + §5.7、`docs/HANDOVER.md`、`ROADMAP.md` I16、三个 skills、`AGENTS.md`、`dsh/README.md`、VS Code README/CHANGELOG | 本轮同一 commit 同步 |

**验收（全部实测通过）**：`scripts/soko gate` PASS；`cargo test --workspace --locked`
全绿（front 448 / LSP 120 / CLI 191+）；A1 由
`import_free_files_are_byte_identical_to_the_single_file_path` 守住；零 cargo 的
用户路径仍只走 Release 二进制（新能力不引入任何工具链依赖）。

---

## 6. 验收标准（可执行）

> 命令里的 `$SOKO` = `scripts/soko`（harness 中立启动器，零 cargo）。
> 每条都写出**期望输出**，不接受"看起来对"。

- **A1（零回归，最重要）**：`git stash`-级别的对拍——对 45 个语料文件
  `for f in course/*.sokonanoda examples/*.sokonanoda playground.sokonanoda; do
  $SOKO --json "$f"; done` 的 stdout 与 HEAD **逐字节一致**；
  `cargo test --workspace --locked` 全绿（含 `course.rs`/`course_status.rs`
  两处 golden 表与 756 条既有测试）。
- **A2（零配置两文件）**：临时目录里只放 `A.sokonanoda`（`import B`）与
  `B.sokonanoda`，**没有 manifest**：`$SOKO A.sokonanoda` 退出码 0 且
  `--json` 里能看到 B 的 `decl.checked`（事件带 `module: "B"`）。
- **A3（清单与根发现）**：深层目录 `proj/src/Lesson/Logic.sokonanoda` +
  `proj/sokonanoda.toml`（`src = "src"`）：在 `proj` 下任意子目录执行
  `$SOKO src/Playground.sokonanoda` 都能解析到同一模块；
  `sokonanoda.toml` 写非法 TOML → 明确报错（不 panic、不静默忽略）；
  `requires = "0.56"` 与二进制 0.57.0 → 给出可读的版本提示。
- **A4（归因与阻断）**：`Logic.sokonanoda` 第 12 行有类型错误时，
  ①诊断路径指向 `Logic.sokonanoda:12`（不是入口文件）；
  ②入口文件只在 import 行报 **1 条** `import-dependency-failed`；
  ③退出码 1。
- **A5（闭包 prelude）**：`Lesson/Nat.sokonanoda`（显式 `inductive Nat`）+
  两个下游画布：三者的 `Nat` 是同一份声明（`#check`/hover 文本一致）；
  入口写 `-- sokonanoda:prelude none` 而依赖写 Full → `import-prelude-conflict`；
  反向同理。
- **A6（缓存正确性）**：`$SOKO build proj --json` 冷跑 → 全部 `compiled`；
  再跑 → 全部 `hit`；`touch`（内容不变）→ 仍 `hit`；改 `Lesson/Logic.sokonanoda`
  一行 → 它自己与**所有下游**都 `compiled`，无关模块仍 `hit`；
  任意两次 `--json` warm 输出逐字节一致（既有 warm-cache 契约的多文件版）。
- **A7（LSP 跨文件）**：真实 LSP 会话（沿用 `crates/cli/tests/query.rs` 起进程的做法）：
  `textDocument/definition` 从画布里的 `And.intro` 跳到 `Lesson/Logic.sokonanoda`
  的 URI+range；把库文件改坏 → 打开的入口文档收到新诊断（且诊断 URI 正确）；
  缺失模块/成环时服务器**不崩**、给出诊断。
- **A8（教学与协议）**：第 11 单元进 `course.json` 与两处 golden 表；
  `docs/protocol.md` 列全 9 个新错误码并由契约测试守住（`protocol_document_lists_…`
  现有模式）；`skills/sokonanoda-teacher` 能照文档跑通一轮"教材库 + 画布"教学；
  `scripts/soko gate` PASS。

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| **单文件路径被改坏**（最致命） | 教学主循环与 45 个 golden 全红 | §4.1 不变式 1 写成契约测试：无 import 走**原入口**，`--json` 逐字节对拍（A1）；PR 门禁 = `scripts/soko gate` |
| 闭包 prelude 装错（重复安装/漏装） | 内核 panic（重复声明）或"未定义标识符"雪崩 | 闭包预扫描先定形状（§4.6）；四种形状的判别性测试；9 个自带 `inductive Nat` 的语料文件当活样本 |
| 诊断指错文件 | 教学体验崩坏（最容易被忽略的正确性 bug） | `ModuleReport` 携带 `path`，所有渲染只从 `(module, span)` 生成；A4 + U8 的"穷举对拍"方法（I15 教训：**新测试通过 ≠ 新语义被测试**） |
| 缓存给出**错误结论**（改了依赖却命中） | 违反"内核唯一判定者"的信任边界 | iface 递归包含依赖源文本（§4.8）；专门写"改依赖必 miss"的判别性测试（A6）；信任台账默认不做 |
| LSP 每次编辑重编整个闭包导致卡顿 | 编辑器体验退化 | 报告缓存 + 只重编受影响后缀 + 反向后继只 publish 已打开文档；P5 带 perf 冒烟（`docs/PERF.md` 阈值纪律） |
| 前端巨石再长 | 违反 REQUIREMENTS §4 | 新代码全部落在 `crates/front/src/project/` 新文件；`elab.rs`/`check.rs`/`session.rs` 只加挂接点，不加实现；收尾 `wc -l` 记账 |
| 模块名规则与 Lean 不完全一致（`-`、大小写、非 ASCII） | 破坏"教学语法 = 真实 Lean 4 子集"的承诺 | 规则逐条对齐 Lean 并在 `docs/notes/` 留出处；非法名给**可操作**的 hint；Windows/macOS 大小写不敏感专门测 |
| `requires` 版本钉太松/太严 | 要么形同虚设、要么把用户挡在门外 | v1：不匹配 = **warning + 继续**（`doctor` 里可见）；严格模式留 Q6 |
| 依赖 TOML 解析引入新 crate | 构建时间/供应链面增加 | 先按 Q1 拍板；若选 TOML 则锁 `Cargo.lock`、CI `--locked` 验证；备选 = 纯标记文件（零解析） |
| **跨模块重名没有任何测试经验** | `elab-duplicate-declaration`（`compile/error.rs:32/110`）在测试里被枚举过 4 次，但**从未有测试真正触发它**——而"同名声明到达两次"正是天真 import 实现的第一症状 | P2 第一组测试就要打这条：同名两模块 → 一条 `import-name-collision`（带两个来源），并断言**内核不被惊动**（`kernel_checks` 计数不因此增长） |
| 「静默错误」比测试变红更危险 | 三道门（`front/tests/perf.rs:108` 的 `kernel_checks<=1`、`watch.rs:292-298` 的独立性契约、judge/suggest 的静默无建议）不会报错，只会让结论悄悄错 | 见 §4.12 的专门表；每道门在对应阶段显式重写语义或显式声明边界，不靠调阈值 |
| 导入使每次 update 重新 elaborate 整个闭包 | 编辑器里改一行 → 依赖链上所有模块的 elaborate 重跑（内核重查可用信任台账省，elaborate 省不掉） | 闭包小（教学项目 <20 模块、单文件 <200 行）；P4 加 perf 冒烟与阈值；若实测超标，优先级提到 P7 的 decl 级产物（需内核决策） |
| **CI/Pages 不会发现"画布不再自包含"** | 画布与站点的守卫都不在 CI（anchor 只在本地 `soko gate`；`gen-site-demos.py` 只守产物新鲜） | A1 的逐字节对拍必须真落地成测试（P1 就加）；不要依赖既有流水线报警 |
| **fuzz 面扩大但 CI 不跑 fuzz** | 新顶层 `import` 命令的解析/编译 panic 只在本机 fuzz 时才可能暴露 | P1 在验收清单里显式跑一次 `cargo fuzz`（或把 fuzz 目标纳入 CI 作为独立工作项，另立 ticket） |
| 内核只能表达"扁平前缀环境" | 跨文件互递归／菱形遮蔽／并列命名空间**不可表达**；天真实现会在内核 panic 或静默错 | §4.5 第 11 条把策略写死（重名即错、环即错）；这些形状在 P7 之前不做，遇到就给教学化错误 |

---

## 8. 待拍板（建议已给出，等用户确认）

| # | 决策 | 候选 | 建议 |
|---|---|---|---|
| **Q1** | 清单格式 | (a) `sokonanoda.toml` + `toml` crate (b) `sokonanoda.json` + 已有 `serde_json` (c) 纯标记文件（空文件即项目根，零解析；元数据以后再说） | **(a)**：与 Lake 同风格、可注释、教学友好；代价是 +1 crate。若想零新依赖选 (c)，元数据推迟 |
| **Q2** | 无 manifest 时的 `import` | (a) 允许：模块根 = **入口文件目录**（零配置两文件 demo）(b) 报错，必须建 manifest (c) 严格对齐真实 Lean：模块根 = **进程 cwd**（但官方 Lean 连"文件自己的目录"都不在搜索路径里，见 §2.1） | **(a)**：与"单文件是默认、项目是 opt-in"一致；`--no-project` 可强制单文件语义。**(a) 是刻意 divergence**：(c) 会让"同一个文件在不同 cwd 下结果不同"，而教学 agent/DSH 的 cwd 不可控；代价只是"搬进官方 Lake 项目要按 Lake 规则摆文件" |
| **Q3** | prelude 模式的决策者 | (a) 入口文件 (b) manifest 字段 (c) 每个模块各自 | **(a)** + 冲突报错；将来若要更明确可让 manifest 覆盖（向后兼容地加字段） |
| **Q4** | 依赖库的"已检查"复用 | (a) v1 只做报告缓存（每次重查内核）(b) 同时做信任台账（跳过重查） | **(a)**，并留好观测点；(b) 必须先量收益（I8 先例） |
| **Q5** | 课程语料是否重构（`solutions/` 复用、单元共享库） | (a) 本轮不动 (b) 同轮重构并重钉 golden | **(a)**：golden 重钉应独立一轮，混在一起会让"import 本身有没有回归"无法归因 |
| **Q6** | `watch --workspace` 是否项目化 / `soko/project` 是否 v1 就做 | (a) 都不做（CLI watch 保持"每文件独立"）(b) 都做 | **(a)**：LSP 已覆盖编辑器场景；协议震动留到有真实消费者时（沿用 `compiler-service-events.md` 的节奏） |
| **Q7** | 产物/缓存位置 | (a) 保持**用户缓存目录**（现状：`~/Library/Caches/sokonanoda` 等，内容寻址）(b) 项目内 `.soko/build/`（§2.4 调研推荐的形状，模块路径为键）(c) 两者并存（项目内主、用户缓存辅） | **(a)**：零仓库污染、只读源码树可用、`git status` 干净；代价是不跨机器共享（我们不需要）。若将来要"团队共享已编译产物"，再按 (b)/(c) 走一次设计（`--cache-dir` 已存在，扩展点已留） |

---

## 9. 被否方案（为什么不做）

1. **源码拼接（concatenation）当 import 语义。**
   把闭包拼成一个大字符串喂给现有单文件流水线，看起来最省事（本轮调研底稿也建议过）。
   否掉的理由是**它会制造一个必须到处打补丁的坐标谎言**：诊断/ hover / codeLens /
   holes / inlay / `soko/*` 全部消费 `Span { offset, line, column }`，拼接后所有
   行列都要做"文件内重映射"；`DocumentReport` 会混进别的文件的声明；
   缓存键退化成"整个闭包一份"（任何小改动全量失效）；跨文件跳转仍需要
   `ModuleId`。逐模块解析（每个文件自己的 offset）反而**总工作量更小**，
   且顺手得到 §4.5 的分模块报告。拼接唯一省下的是"闭包预扫描 + 一次 prelude"，
   而那两件事各只有一个函数。
2. **把被 import 的声明"复制"进导入方（copy-paste 语义）。**
   `Or` 在两处含义不同（§1.2）——复制语义会让同一个名字在不同闭包里指向不同
   声明却都"编译通过"，正是今天歧义的放大版。
3. **跨 arena / 跨进程复用内核环境（`.olean` 等价物）。**
   内核 `Declar` 借用同一个 `Arena`，`NamePtr` 的相等就是地址相等
   （`crates/kernel/src/util.rs:133-142`），`decl_idx` 只由 `EnvBuilder::add_declar`
   写且是 `pub(crate)`；`ExportFile.dag`/`mutual_block_sizes` 都以同一 arena 的指针
   为键。⇒ 跨 arena 必须重新 intern，除非给冻结内核加"序列化器"（违反
   "kernel 保持上游快照原样"）。**并且这条已经被文档明确拒绝过一次**：
   `docs/design/compile-cache.md:78`「不做与内核 `.olean` 等价的『已编译环境导入』」；
   旁证 `docs/architecture.md:296,311`（上游 NDJSON 路径原样保留但刻意绕开；
   `Session` 每次 update 开新 arena、**跨 update 只复用渲染快照，不复用内核对象**）。
   v1 用"闭包哈希 + 报告缓存"达到同样的用户可感效果（§4.8）；**本文是显式推翻
   那条 v1 边界**，推翻理由 = 用户要 import，而 import 的最小正确形态不需要它。
   代价写进性能预算：**每次 update 都要从源码重新 elaborate 整个闭包**（P4 的
   信任台账只省内核重查，不省 elaborate）。
4. **一并上 `namespace`/`open`/`private`/`import all`。**
   每条都要"课程 + 测试 + 白名单"三件套；v1 的真实需求（20 处跨文件歧义）
   用"模块边界 + 重名报错"已经解决。一次只加一条语法点（硬规则 4）。
5. **让 LSP 侧自己实现模块解析。**
   I15 的教训：真相必须在 `front`，LSP 只做形状映射（`crates/lsp/src/query_map.rs`）。
   项目解析同理——否则 MCP/CLI/LSP 会长出第二份真相。
6. **一次做完整的依赖管理（registry/git/版本求解）。**
   教学项目没有 registry 需求；先做"单项目内的模块"，把 `[deps]` 的位置留出来
   （§4.11），等有第二个真实项目再谈。
7. **给 `import` 发明更"教学化"的语法（`include "..."`、`use` 等）。**
   违反硬规则 3（教学语法必须是真实 Lean 4 子集）；学生学到的必须是能带走的。

---

## 10. 参考（外部调研底稿与出处）

**本轮调研底稿（仓库内，逐条带 `path:line` 或官方文档 URL）**
- `docs/notes/multifile-prior-art.md` —— 10 个系统（Coq/Rocq、Agda、Isabelle、Idris 2、
  Rust、Go、Python、JS/TS、Haskell/OCaml、JVM）的"单文件 vs 项目"逐系统记录 +
  5 问横向表 + "值得抄的模式/反模式"（每个 URL 都实际抓取核对过）。
- `docs/notes/project-roots-and-incremental-caches.md` —— LSP 契约（`rootUri` 可为 null、
  `didChangeWatchedFiles`、诊断"替换不合并"与"可读缓存"）、方言服务器根发现
  （rust-analyzer/clangd/tsserver/pyright/gopls/ocaml-lsp/Agda/lean4/HLS）与错根症状、
  失效与产物（Lake trace / GHC 指纹 / OCaml `.cmi` 摘要 / Coq `.vo` digest / `.tsbuildinfo`）、
  原子写与并发、以及"缓存判定结果安全吗"的三条规则。
- 代码接缝的全部 `path:line` 依据在本文 §4.12（同轮 subagent 只读勘察，未改动任何文件）。

**关键外部锚点（正文引用的原文）**
- Lean 4 语言参考 §5 Source Files and Modules、§24 Build Tools and Distribution：
  <https://lean-lang.org/doc/reference/latest/>
- Lean 4 源码：`src/Lean/Shell.lean`、`src/Lean/Util/Path.lean`、`src/Lean/Elab/Import.lean`、
  `src/Lean/Parser/Command.lean`、`src/lake/Lake/**`（`github.com/leanprover/lean4`）
- vscode-lean4 项目根发现：`vscode-lean4/src/utils/projectInfo.ts`
  （<https://github.com/leanprover/vscode-lean4>）
- LSP 规范 3.17/3.18：`general/initialize`、`workspace/*`、`language/publishDiagnostics`
  （<https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/>）
- clangd：<https://clangd.llvm.org/design/compile-commands>、<https://clangd.llvm.org/config>
- Coq/Rocq 命令与工具、Agda 包系统与接口文件、Isabelle system/Isar 手册、Idris 2 包与模块文档
  （逐条 URL 见 `docs/notes/multifile-prior-art.md`）

