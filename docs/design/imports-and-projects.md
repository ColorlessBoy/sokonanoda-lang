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
requires = "0.57"         # 可选：工具链约束（类似 lean-toolchain 的意图），不匹配给 warning/error
                          # 真正的"钉"是仓根 sokonanoda-version.txt（完整 x.y.z，启动器按它下载）
src = "."                 # 可选：模块根，默认 = 本文件所在目录
```

- **发现规则**：从**入口文件所在目录**向上找最近的 `sokonanoda.toml`；找到就用它，
  找不到就走**零配置退路**：模块根 = 入口文件所在目录（所以"两个文件互相 import"
  不需要任何清单，符合本项目"单文件是默认、项目是 opt-in"的产品性格）。
  - **入口路径先绝对化（2026-09-18 补，G-12；`plan_project_with_overlay` 的第一步）**：
    `cwd` 只在"把入口变绝对"这一步用**一次**，此后不参与任何判定；`entry_dir` 与
    `--root` 覆盖值（`root_override`）都由绝对化后的结果派生——**两者必须同轮绝对化**，
    否则 `module_name_of_path` 的 `path.strip_prefix(root)` 失配、模块名退化成裸
    `file_stem`、闭包摘要（缓存键）跟着漂。绝对化用**词法**手段（`current_dir()` 拼接，
    不碰文件系统）：stdin + `--root` 会合成一个磁盘上不存在的入口路径，
    `canonicalize` 必然失败并回落成相对路径，反而制造"绝对 entry + 相对 root"的混搭；
    `canonicalize` 是**视图层**的职责（`crates/front/src/query/project.rs` 的
    `absolute()`，`docs/protocol.md` 的 "Paths are absolute" 由它实现）。
  - 为什么必须有这条：相对入口目录向上走到空分量 `""` 时
    `Path::new("").join("sokonanoda.toml")` = `sokonanoda.toml`，等于"去问 CWD 要清单"；
    清单路径没有目录分量 ⇒ 模块根退化成**空路径** ⇒ `fs::read_dir("")` ENOENT ⇒
    每个 `import` 都 `import-not-found`。更糟的是它让"入口写相对还是绝对"改变解析
    结果——正是本节要消灭的 cwd 隐状态。护栏两条：`find_manifest` 不把空目录分量
    当上溯的一站；`module_root` 把空 `parent()` 当 `.`。
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

### 4.4b 自动区分：触发条件是 `import`，不是清单（2026-09-18 明确）

| 情形 | 走哪条路 | 会不会读 `sokonanoda.toml` |
|---|---|---|
| 文件里**没有任何 `import`** | 单文件流水线（今天的路径，逐字节不变） | **从不读**（同目录/祖先目录里的清单再坏也无关；`--root` / `--no-project` 是空操作） |
| 文件里有 `import` | 项目闭包 | 从**入口目录**向上找最近清单（止于 `.git`/HOME）；找不到就零配置（模块根 = 入口目录）。入口目录 = 入口路径**词法绝对化**后的目录，`cwd` 只参与绝对化那一步 |
| 被 import 的依赖模块 | 闭包的一部分 | **它自己的清单永远不参与**（模块根只由入口决定；`import Sub.Lib` 直接解析 `Sub/Lib.sokonanoda`） |
| stdin（`-`） | 无 `import` 时与文件等价 | 不适用；**带 `import` 时明确报错**并提示 `--root`（没有路径就没有模块根） |

所以"单文件像脚本一样直接跑"是**契约**，由
`crates/cli/tests/single_file_vs_project.rs` 四条测试钉住；编辑器侧 LSP 用同一套
规则（`initialize` 的工作区根**不**当模块根用——那会跳过清单发现，让嵌套项目在
编辑器里报 `import-not-found` 而 CLI 正常；回归见
`crates/lsp/src/tests/project.rs::a_nested_project_resolves_against_its_own_manifest`）。

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
| `course` | **as-built（WO-007）**：有 `import` 的单元走项目闭包（与 `grade`/`query check`/`build` 同一份闭包、同一个模块根与 `ProjectPlan::digest` 摘要键），计数只取入口模块、`failed` 与 `grade` 退出码同判；**无 `import` 的单元仍逐字节走单文件**（golden 表因此依旧不动）。模块根 = 入口最近的 `sokonanoda.toml`，否则 `course.json` 所在目录 |

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

## 5.1 as-built（2026-09-18 落地，版本 0.57.0）

用户 2026-09-18 指示「新产生一个 git 分支，全部按照建议，你给我完整做完一版」——
§8 的 Q1–Q7 全部按推荐执行。**P0–P6 全部落地**，P7 仍是 backlog（本轮未承诺）。

**与本文设计的偏差（三条，都是实现时才发现的事实）**

1. **§4.12/P4 里的 `project/iface.rs` 没有单独成文件**：闭包摘要就是
   `ProjectPlan::digest(&CompileOptions)`（`crates/front/src/project/mod.rs`），
   没必要为 30 行多开一层模块。其余文件名与设计一致。
2. **P5 全部落地（含第一版"挂住"的真相，以及后来修掉的模块根错位）**：
   `initialize` 只记会话（**不再把工作区根当模块根**——那会跳过清单发现，让工作区里
   嵌套的项目在编辑器里报 `import-not-found` 而 CLI 正常；现在编辑器与 CLI 同一套
   发现规则，见 §4.4b）、
   `Docs{map,order,root,active}`、按 URI publish、跨文件 `definition`/`references`/
   `rename`、**改依赖自动重编译下游**都完成。两条与设计不同的实现选择：
   ① 下游重编译不是"反向后继图调度"，而是**每次通知把所有打开文档当覆盖重新编译
   需要的那一份**（教学项目规模下更简单、也更正确：未落盘的依赖编辑必须可见）；
   ② 诊断**只在真的变化时才发**（`Doc::published` 比对）。第一版"会挂住"的根因
   不是实现而是**测试写法**：服务端一次通知可能连发多条诊断，而测试先等通知结束
   再读 socket ⇒ 死锁；修法是 `testutil::notify_with_drain`（边处理边排空）。
   两项后来都落地了：`didChangeWatchedFiles` 在 0.57.0（批次 1）、`soko/project`
   在 0.58.0（批次 4，`docs/design/project-view.md`）。细节见 `docs/TESTING.md` §5.7。
3. **`import-prelude-conflict` 的判据细化**：没有 prelude 指令的模块视为
   **继承**入口模式，只有"显式指令与闭包决定不一致"才报错（设计 §4.6 只写了
   "闭包内不一致"，实现时需要区分"未声明"与"显式声明"两次预扫描）。
   另有两条实现期决定：`import` 必须加入 `is_reserved_command`（否则行首
   `import` 会被当成应用的实参吞掉）；加载器的后序 `visit` 返回
   `VisitOutcome::Cycle`，保证入口在拓扑序最后。

4. **§6 A2 的事件契约按实现改口径（第四条偏差）**：设计原写"`--json` 里能看到 B 的
   `decl.checked`（事件带 `module: "B"`）"，实现选择的是**只输出入口模块的事件**：
   依赖的已通过声明不刷屏（一个 12 声明的依赖会把入口的答题流淹没），依赖的**问题**
   仍以诊断形式带 `file`/`module` 归因；想要某模块自己的事件，把它当入口跑一遍即可
   （`query check --file <任何模块>` 同样成立）。`build --json` 另给逐文件状态。
   验收 A2 的正文已按此改写，`docs/protocol.md` 写明。
5. **新增一笔结构债（已登记，第九十五轮已还清）**：`crates/front/src/compile/check.rs`
   从 1717 行涨到 1918 行（`run_pass` 单函数 ≈1174 行）——多 unit 泛化加在这里，
   拆分留到独立一轮（事件流/增量语义不能漂）；**2026-09-18 第九十五轮拆完**：
   `check/{mod,walk,kernel_phase}.rs` + `compile/units.rs`，只动位置不动语义。

**交付物（可核对）**

| 层 | 位置 | 规模 |
|---|---|---|
| 语法/解析 | `crates/front/src/parser.rs`、`token.rs`、`ast.rs`（`Command::Import`） | 3 个 parse 期错误码 |
| 闭包编译 | `crates/front/src/project/{mod,module_name,resolve,manifest,graph,report}.rs` | 6 文件 + 18 单测 |
| 编译驱动 | `crates/front/src/compile/check.rs`（`units: &[SourceUnit]`、`split_report`、命令下标归因） | 内核零改动 |
| 缓存 | `ProjectPlan::digest` + `cache::key`（`CACHE_FORMAT` 1→2） | `docs/design/compile-cache.md` §7 |
| CLI/协议 | `--root`/`--no-project`、`build` 项目化、`query` 闭包、`help.rs` 多文件段 | `crates/cli/tests/imports.rs` 12 e2e |
| LSP | 多文档、按 URI publish、跨文件定义/引用/改名、改依赖自动刷新下游、内存覆盖（未落盘编辑） | `crates/lsp/src/tests/project.rs` 8 e2e + `project_refs.rs` 2 单测 |
| 教学面 | `course/unit11-modules-projects.sokonanoda`（+EN+solution）、`course/unit11-project/` 可运行两文件项目、`course.json` | golden：画布 (7,6,0)、solution (12,0,0) |
| 文档 | `docs/architecture.md` §4.5/§8.10、`docs/protocol.md`、`docs/TESTING.md` 三行 + §5.7、`docs/HANDOVER.md`、`ROADMAP.md` I16、三个 skills、`AGENTS.md`、`dsh/README.md`、VS Code README/CHANGELOG | 本轮同一 commit 同步 |

**验收（全部实测通过）**：`scripts/soko gate` PASS；`cargo test --workspace --locked`
全绿（front 449 / LSP 127 / CLI 191+）；A1 由
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
  `B.sokonanoda`，**没有 manifest**：`$SOKO A.sokonanoda` 退出码 0，A 的
  `decl.checked` 正常输出；B 里已通过的声明**不进**事件流（避免依赖的几十条
  声明淹没入口），B 的问题以**诊断**形式带 `file`/`module` 出现（见 §5.1 偏差④；
  要看某个模块自己的事件就把它当入口：`$SOKO B.sokonanoda`）。
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



---

> **过程部分已归档** ✓（7 节：触发与目标 / 外部调研 / 现状审计 / 分阶段计划 / 风险 / 待拍板 / 参考）
> ⇒ `docs/archive/imports-and-projects-process-2026-09-26.md.gz`（`gunzip -c … | less` ✓）。**归档 ≠ 销毁** ✓；本文件只留**设计契约 + as-built + 验收 + 被否方案** ✓。
