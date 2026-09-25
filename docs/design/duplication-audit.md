# 「同一件事多处实现」审计（阶段 U / T-U9；用户 2026-09-25 要求 ✓）

> 用户原话：「排查一下有没有其他模块有一样的问题：**明明是一摸一样的功能，用在不同的地方，
> 但是我们项目分成多个不同的实现，导致各种 bug**」✓
>
> **取证方式**：三个**只读** subagent 分头查（并发 3 ✓，各带规则摘要 + 可执行判据 + 硬边界 ✓）：
> ① `crates/front/**` ② `crates/cli|lsp|query/**` ③ `scripts/**` + `editor/vscode/**` ✓。
> 每条结论都要求带 **file:line 或可复跑命令** ✓；**未证实**的必须自己标出来 ✓。
> 主线按纪律**抽查验证后才并入** ✓（见 §2 ✓）。

## 1. 发现总表（按用户可见风险排序；合并三份报告、去重）

| # | 位置 | 重复了谁 | 风险 | 唯一归属 | 可执行判据 | 处置 |
|---|---|---|---|---|---|---|
| 1 | `crates/cli/src/query.rs:170-176` 自算 `clean` | `ProjectReport::is_clean()`（[report.rs:163](crates/front/src/project/report.rs#L163)）；另 4 处调用者 | **高** —— `query check` 为不干净的项目写缓存 ⇒ **之后所有 `grade`/`check` 静默丢掉依赖模块的警告**（子代理已复现 `1 → 0` ✓） | 判据收进 `store_if_clean*`（**删掉 `is_clean` 形参**，函数内部自取 ✓） | `grep -rn "is_clean()" crates/cli/src crates/lsp/src` → 4 处、**`query.rs` 不在其中** ✓ | ✅ 已修（守卫进 gate ✓ + **反向验证**补上 ✓ —— 实测抹掉 `value_runs` 会被抓到 ✓） |
| 2 ❌**假阳性**（round 80 实测 ✓） | `query/mod.rs:522-578`（`failed` 只由入口事件合成） | `cli/check.rs:156-175`（把**每个非入口模块**的诊断也发出） | **高** —— 同一项目 `grade` 出 2 条诊断、`query check` 的 `warnings` 是 `[]` ⇒ MCP/agent 漏一整层 | **不动** ✗：这是**已文档化的设计** ✓ —— 见下 | `soko --json X \| wc -l` = 2 vs `query check` 的 `"warnings":[]` ✓ | **不做**（设计如此 ✓） |
| 3 ✅**已修**（round 72） | `scripts/audit-wire-fields.py` **没进 gate** ✗ | `AGENTS.md` / `docs/CI-FAILURES.md` 都宣称"已进 gate 与 CI" ✗ | **高** —— 咬 R-1（`value_runs` 漏映射）的**唯一**守卫**从不自动跑** ✓（与 R-3「门禁崩了却不判」同形 ✗） | 接进 `scripts/soko` 步骤表 + CI ✓ | `grep -c audit-wire-fields scripts/soko` ⇒ **0** ✓（已抽查证实 ✓） | **立刻做** |
| 4 | `scripts/kernel-diff.sh:82` 收集器无 `-type f` | `notation-lint.py:364` / `verify-decl-panel.py:63`（后者已补 `is_file()` ✓） | **高** —— 实收 **4 个目录** ⇒ 20 组对拍**恒绿**、**"零差异"覆盖被虚报** ✗ | 收集器唯一化 + `-type f` ✓ | `find courses course examples docs/gaps/repro -name '*.sokonanoda' ! -type f` ⇒ **4 行** ✓（已抽查 ✓） | **立刻做** |
| 5 ⏳**最硬的一处已修**（round 84 ✓）：`references.rs` 的**字节列** ✅（判据已加 ✓）；其余见下 | `query/pos.rs:9`（UTF-16，自称唯一 ✗）、`session.rs:467`、`references.rs:98/154`（**byte** ✗）、`lsp/lib.rs:993`、`lsp/render.rs:381`（与上一份**逐字相同**）+ `lsp/tokens.rs:88` 内联 | **高** —— byte 列**直接喂** LSP `character`（`lsp/render.rs:52`、`project_refs.rs:145`）⇒ 含 `α`/`∈` 的行上高亮/rename 右移 ✓（已立台账 G-36/T-D31，但**重复未消除** ⇒ 修一处不会一起好 ✗） | `front::query::pos` 唯一入口 ✓ | `grep -rn "fn line_col\|fn offset_of" crates/front/src crates/lsp/src` ⇒ 9 命中 ✓；`references.rs:157` 确为字节（已抽查 ✓） | **立刻做** |
| 6 ⏳**一半已修**（round 81 ✓） | `suggest.rs:499 atom_text` ✅ + `:464 eq_refl_candidate` ⏳ | `proof.rs:562 render_atom` + `by.rs:2225 rfl_candidate`（`by.rs:2266` **已改为委托** ✓，suggest 这份**漏了** ✗） | **高** —— 漏 `Notation`/`SetLiteral`/`AnonCtor` ⇒ `rfl` 建议在**记法操作数上静默消失** ✓（同 G-04 第二刀那次 bug ✓） | `proof::render_atom` ✅（`atom_text` 两边都改成**委托** ✓）；`rfl_candidate` 仍待共享 ⏳ | `grep -rn "fn atom_text\|fn eq_refl_candidate\|fn rfl_candidate" crates/front/src` ✓ | **立刻做** |
| 7 ⏳**已核清单、待定规则归属**（round 82 ✓） | 词法符号表：**两条不同的规则 + 四份逐字副本**（见下 ✓） | 互相 | **高** —— 这是 **R-2 的复发通道** ✗（`=` 吃 `=>` ⇒ 整段降级 ✓）；`notation_input.rs:308 known_symbols` 已是统一实现，同文件 4 处各抄一遍 ✗ | **先定哪条规则为准** ✗（见下 ✓），再让其余全部委托它 ✓ | `grep -rn "lexer_builtin_symbols()" crates/front/src` ✓ | **立刻做** |
| 8 ✅**已修**（round 69） | ~~**`display.rs:188 DisplayNotations::render` 与 `:235 render_folded` 函数体逐字等价** ✗~~ ⇒ **`render_folded` 已删除**（无调用者 ✓），接口恢复唯一 ✓；盲区已写进守卫文档 ✓ | 我自己的 T-U2 接口 ✗ | **高（元风险）** —— T-U2 刚立的"唯一接口"**当场分成两个入口**，而且两者**都在 `audit-notation-paths.py` 白名单里**（整文件豁免 ✗）⇒ **无人守** ✗✗ | 只保留 `DisplayNotations::render` ✓，删 `render_folded` ✓ | `grep -n "pub fn render\b\|pub fn render_folded" crates/front/src/display.rs` ✓ | **立刻做（T-U5 一并）** |
| 9 ⏳**最危险的一半已修**（round 85 ✓）：JS 的"兜底=solved" ✗⇒✓；三处词表仍在 | `DeclStatus→可见文字` **三处**硬写（`query/mod.rs:1004-1007`、`lsp/render.rs:411-417`、`extension.js:196-201`） | 互相；wire 只发 stringly-typed `status` ✗ | **高** —— JS 的 `solved` 是**兜底分支** ⇒ **任何新 status 被静默显示成"已解决"** ✗✗ | wire 发 label，或 JS 只做 1:1 映射并**删兜底** ✓ | `grep -n 'status === "open"' editor/vscode/extension.js` ✓ | **立刻做** |
| 10 | 课程文件收集器 **2/5 已修** ✗（`check.py:525/545/563` 的 `glob` 无 `is_file()`；`e2e-merge.py:73-76` 的 `rglob` 无 `is_file()` 且 `except` 不接 `OSError`） | R-3 那一族 ✓ | **高（形状已证、触发未证 ✓）** —— `Path.glob("*.sokonanoda")` **确实**返回 `.sokonanoda` 目录 ✓（Python 3.14 ✓）；`e2e-merge.py` 遇同名目录会 **traceback 而非 `SystemExit`** ✗ | 一个带 `is_file()` + `except OSError` 的共享收集器 ✓ | `grep -n 'glob("' courses/set-theory/tools/check.py` ✓ | **立刻做** |
| 11 | `query::notation_symbols()` 扫文本拿符号 ✗（**第五套**，见 `REQUIREMENTS.md` §9 ㉗） | 记法表 `notation_table` | 中 —— 只能打标签不能折叠 ✗；且 query **够不到**记法表 ✓ | 阶段 U 的显示副本方案（T-U4 ✓） | 已在 §9 ㉗ 记录 ✓ | **并入 T-U4/U5** |
| 12 ✅**已修**（round 86 ✓）：两份逐字相同的 `range_of(Span)` 收成一份 | 彼此 + `query_map.rs:32 range_of_offsets` | 中 —— 同一 wire `Range` 两条换算路（span 列 vs offset）⇒ 非 ASCII 下不一致 ✓ | 已合并到 `render::range_of` ✓（`project_refs` 改调它 ✓）；与 `query_map::range_of_offsets`（按 offset ✓）的**进一步**统一仍待做 ⏳ | `diff` 两份片段 → 无差异 ✓ | 并入 #5 |
| 13 | 事件计数**三份**（`query/mod.rs:519-534`、`cli/course/mod.rs:461-472`、`cli/check.rs:261`+`json_report.rs:27-74`） | 互相 | 中 —— "`checked` 是几 / `failed` 算不算依赖模块"**两套口径** ✓ | `CheckCounts::tally()` 唯一 ✓ | `grep -rn "DeclarationChecked" crates/cli/src crates/front/src/query` → 3 处 ✓ | 并入 #2 |
| 14 | "还剩几个练习"**两套**（事件口径 `query/mod.rs:529` vs 声明口径 `report.rs:109-116`） | 互相，且都进 wire ✗ | 中 —— 两条通道给出两个数，**无断言钉死相等** ✗ | 保留一个口径或加交叉断言 ✓ | `grep -rn "open_exercises\|exercise_open"…` → 生产者 2 个、**无相等断言** ✓ | 立刻做（加断言） |
| 15 ✅**已修**（round 87 ✓）：`redundant` 判据收进真相层、LSP 只调 |（`query/mod.rs:1012/1023` vs `lsp/lib.rs:1082-1095`） | 互相；`query/mod.rs:1021` 自己写着"与 LSP 侧同一条规则" ✓ | 中 —— 分叉时"多余的 `sorry`"两侧给出**相反**的下一步指令 ✗ | ✅ `sokonanoda_front::query::{redundant_hole_spans, hole_is_redundant}` 已公开 ✓，LSP 内联副本已删 ✓ | `grep -rn "hole_is_redundant" crates/front/src/query/mod.rs crates/lsp/src/lib.rs` ✓ | 立刻做 |
| 16 | LSP 合成 `code:"sorry"` 警告 + 全冗余抑制（`lsp/lib.rs:1095-1122`） | `query/mod.rs:579-596`（CLI **无**合成/抑制 ✗） | 中 —— 编辑器比 CLI 多一条警告；"该不该提示"只活在 LSP 一侧 ✗ | 规则下沉到真相层 ✓ | `sed -n '1105,1125p' crates/lsp/src/lib.rs` vs `sed -n '579,596p' crates/front/src/query/mod.rs` ✓ | 立台账（暂不做 ✓） |
| 17 ⏳**已实测、未修**（round 72 ✓） | `audit-wire-fields.py` 消费点表**不扫 `project-tree.js`** ✗ | `project-tree.js:24-28,45-53,62-72,169-197` 读 `soko/project` 大载荷 ✗ | 中 —— 它是"有就渲染"的宽容实现 ⇒ 停发字段**静默降级**、守卫永不响 ✗（R-1 同类、换文件 ✓） | 把 `project-tree.js` + `ProjectView` 加进守卫 ✓ | `grep -n 'collect(' scripts/audit-wire-fields.py`（只有 IV/EX ✓） | 立刻做需**先扩守卫**：直连 `ProjectResponse` 会报 8 个 MISSING ✗，但**实测是假阳性** ✓（`soko query project` 响应 = `{project, reason}` ✓，字段嵌在 front 侧 `ProjectView`/`ModuleView` ✓）⇒ 先让 `fields()` 能读 `crates/front/src/query/types.rs` ✓，在那之前**不接** ✗（常红守卫比没有更糟 ✓） |
| 18 | 版本钉源链 **4 份**（`perf-ledger.sh:39`/`perf-report.sh:12`/`vscode-e2e.sh:102`/`new-course-repo.sh:37` 各自只读 `Cargo.toml` ✗） | `scripts/soko:224-346` 的源链 ✓ | 中（**未证实漂移** ✓：当前四处都 `0.67.0` ✓）—— 风险在**看不见**：一旦加 `sokonanoda-version.txt`，台账仍按 `Cargo.toml` 记版本 ✗ | 唯一归属 `scripts/soko version --json` ✓ | `grep -rn "grep -m1 '\^version' Cargo.toml" scripts/` ✓ | 立台账（暂不做 ✓） |
| 19 | `bump.py:90-97` 要求 `requires` **全等 `x.y.z`** ✗ vs `manifest.rs:135-150`（只比 **major.minor** ✓）与 `soko:300-313`（`x.y` 是合法约束 ✓） | 三处语义相反 ✗ | 中（**未证实**：现生效清单都是 `0.67.0` ✓，`bump.py` 现为绿 ✓） | 以 `manifest.rs` 的语义为唯一判据 ✓ | `python3 scripts/bump.py` ✓ | 立台账（暂不做 ✓） |
| 20 | `gap.py:14` 把**用法错误折进 exit 1** ✗（全仓约定 1 = 有拒绝/漂移 ✓；同一条件在 `notation-lint.py` 是 2、`check-site.py` 是 3 ✗） | 各脚本 docstring 各写一遍退出码 ✗ | 中 —— 调用方把"命令行写错"读成"台账漂移" ✗ | 一份 `scripts/_exit.py` ✓ | `grep -n '退出码' scripts/*.py` ✓ | 立台账（暂不做 ✓） |
| 21 | 状态栏"N 练习"（当前文件 ✗）vs 项目树（整闭包 ✓），都只写"练习" ✗ | `extension.js:391` vs `project-tree.js:28` | 中 —— 同一窗口两个数、用户无法分辨 ✗ | JS 只显示服务端给的数 ✓ | `grep -n openCount editor/vscode/extension.js` ✓ | 立刻做（小改 ✓） |
| 22 | 测试夹具手拼 `file://` URI vs 服务端 `Url::from_file_path` ✗ | 两条 URI 构造路（已咬过人 ✓） | 低 | 夹具统一 `Url::from_file_path(canonicalize(p)?)` ✓ | `grep -rn 'file://{}' crates/lsp/tests` → 3 处 ✓ | 立台账（暂不做 ✓） |
| 23 | `counts.decls`（`project.rs:95`）与 `ProjectModule.decls`（`:110`）同表达式写两遍 | 自身 | 低（今天恒等 ✓，潜伏 ✓） | 求和派生 ✓ | `sed -n '95p;110p' crates/front/src/query/project.rs` ✓ | 立台账（暂不做 ✓） |
| 24 | 首个目标在 wire 上出现**两次**（`protocol.rs:184-191` = `goals[0]` ✓，映射走两条路 ✗） | 自身 | 低（已被 `lsp/tests/state.rs:158,162` 钉住 ✓） | 单值三字段标记 deprecated、由 `goals.first()` 派生 ✓ | `grep -n 'goals\[0\]' crates/lsp/src/tests/state.rs` ✓ | 立台账（暂不做 ✓） |
| 25 | 三份 JSONL 台账的 read/write/schema 各写一遍 ✗ | `e2e-merge.py` / `e2e-summary.py` / `gap.py` | 低 | 一份 `scripts/_ledger.py` ✓ | `grep -n 'ledger.jsonl\|schema' scripts/e2e-merge.py scripts/gap.py` ✓ | 立台账（暂不做 ✓） |

**明确排除（查过，避免下一轮重查 ✓）**：路径↔模块名（`graph.rs:176-193` 唯一 ✓）、
URI↔路径（Rust 侧一律库调用 ✓，重复只在测试夹具 ✓）、cache key 构造（`project/mod.rs:236`
唯一 ✓ —— **分叉不在键，在键右边的"干净"判据** ✓ 即第 1 条 ✓）。

### #2 的实测结论（round 80 ✓）：**不是 bug，是刻意设计且已文档化** ✓✓
我照审计的"归并各模块诊断"做了一版 ✗，被三条独立证据挡回来 ✓：
1. **判据** ✗：`crates/cli/tests/query.rs::query_check_still_reports_a_broken_dependency_in_a_project`
   当场判红 ✓，断言语义是 "only the entry's own diagnostic — the dependency's parse error
   stays in the dependency's **coordinate space**" ✓（left 3 ≠ right 1 ✓）；
2. **坐标空间** ✗：`query/mod.rs` 上方文档写明 "**字节** offset（**坐标空间 = 入口文件**）" ✓
   ⇒ 把依赖模块的诊断并进来，等于**把别的文件的偏移当入口的偏移**发出去 ✗；
3. **协议** ✓：`docs/protocol.md:875` **本来就写着**
   "`query check`'s `failed[]`/`warnings[]` report in the **entry file**'s …" ✓
   ⇒ 审计漏读了这一条 ✗（它自己也留了备选："否则必须把'只算入口'写进 `docs/protocol.md`" ✓ ——
   而**它早就写进去了** ✓）。
**依赖模块那一层该看哪里** ✓：`grade --json`（事件流带 `file`/`module` ✓）与
`query project`（`modules[].warnings/errors` ✓ —— 实测那条 `reserved-declaration-name`
就活在 `modules[Dep].warnings` 里 ✓，而 `project.diagnostics` 是空的 ✗）。
**已回退** ✓ 我的改动，只留一段说明注释 ✓（免得下一个人再走一遍 ✓）。

### #6 的进展（round 81 ✓）
* ✅ **`atom_text` 已同源**：`suggest.rs` 那份原来少列
  `Notation`/`SetLiteral`/`AnonCtor` **三个变体** ✗（7 vs `proof::render_atom` 的 10 ✓）
  ⇒ `rfl` 的实参是记法操作数（`Aᶜ`、`{a, b}`、匿名构造子）时**不加括号** ⇒ 候选解析失败
  ⇒ 该建议**静默消失** ✓（正是 `by.rs:2260` 注释里那次 G-04 第二刀的同一形状 ✓）。
  现在 `by.rs:2266` 与 `suggest.rs:507` **都只是委托** ✓ ⇒ 规则唯一归属
  `proof::render_atom` ✓；判据：`grep -rn "fn atom_text" crates/front/src` 只剩两份**委托** ✓。
* ⏳ **`eq_refl_candidate`（`suggest.rs:464`）与 `rfl_candidate`（`by.rs:2225`）仍是两份** ✗
  —— 签名不同（前者收 `&str` + `&DeclState` ✓、后者收 `&Expr` 并返回 `(String, Expr)` ✓）
  ⇒ 共享需要先定一个共同形状 ✓，下次做 ✓。

### #7 的精确清单（round 82 实测 ✓）—— **不能机械替换** ✗
我按审计的说法去核，发现"两套判据 + 六份装配"要**分开看** ✓：

**(a) 两条规则确实不同** ✗（所以不能简单合并 ✓）：
| 位置 | 规则 |
|---|---|
| `crates/front/src/parser.rs:3187 lexer_builtin_symbols()` | `builtin_notation_symbols()` **只滤掉 `LEXER_NATIVE_SYMBOLS = ["="]`** ✓ |
| `crates/front/src/semantic.rs:313-321` | 滤 `s != "="` **且** `lexer_reserved_symbol_char(s).is_none()` ✓ —— **更严** ✓ |

⇒ 想收口成一处，必须**先决定哪条为准** ✓：
`semantic` 那条更严（多滤"词法保留符号"✓），而 `parser` 那条是给**解析**用的 ✓ ——
两者面对的输入不同（一个喂 `tokenize_with_symbols` 做着色 ✓、一个喂 parser 做解析 ✓）
⇒ **"更严"不一定对解析也成立** ✗。**这是设计问题，不是机械替换** ✓
（先记着 ✓；改之前要证明"解析用更严的表"不改变任何语料的接受/拒绝 ✓）。

**(b) 四份逐字副本 ✅ 已收（round 83 ✓）**：`notation_input.rs` 的 `symbol_at`/`symbol_span_at`/`known_symbols`/`symbol_occurrences` 现在**都调同一个** `merge_known(symbols)` ✓ —— 实测本文件里 `lexer_builtin_symbols()` 从 **4 次 → 1 次** ✓、`merge_known` 被调 **4 次** ✓、front **731 passed** ✓（零行为变化 ✓）
—— 这四份**形状相同** ✓，其中 `known_symbols(doc)` 已是公开入口 ✓
⇒ 可安全收成"一个私有 `assemble(scanned)` + 四个薄壳" ✓（**纯重构、零行为变化** ✓，
判据 = 全语料 `--json` 逐字节对拍 + front 731 ✓）。**下次做** ✓。
**(c) `parser.rs:3206/3231`** 那两处要单独看 ✓（面对的是 token/继承表，不是 `&str` ✓）。

### #5 的进展（round 84 ✓）：**字节列那一处已修，且第一次有了能咬住它的判据**
* **复现判红** ✓：`crates/front/src/references.rs::line_col_of` 原来是
  `column = offset - line_start + 1` ⇒ **字节列** ✗。既有夹具是**纯 ASCII** ✓
  ⇒ 两种口径恒等 ⇒ **一直咬不住** ✓。新判据
  `line_col_counts_utf16_units_not_bytes`（`def α : Process :=` 形状 ✓）修前报红：
  ```
  assertion failed: 列必须是 UTF-16 code unit（LSP character 口径 ✓），不是字节 ✗
    left: 18   right: 17
  ```
* **修法** ✓：改为**委托** `crate::query::line_col_of`（re-export 自 `query::pos` ✓，
  它数 UTF-16 code unit ✓ —— 那份文档自己写着"**只有这里一份实现**" ✓）⇒ 唯一归属 ✓。
* **判据** ✓：新判据绿 ✓ · `cargo test -p sokonanoda-front --lib` ⇒ **732 passed** ✓（731+1 ✓）。
* ⏳ **仍未做**：
  - `lsp/render.rs:52` 与 `lsp/project_refs.rs:145` 两份**逐字相同**的 `range_of(Span)`
    ✗（合并零风险 ✓，下次做 ✓）；
  - `token.rs` 写 `Span.column` 用的是**字符**数 ✗（与 LSP 的 UTF-16 口径不同 ✗）——
    这个影响**所有**经词法产生的 span ✓，改动面大、需要独立判据 ✓（先记 ✓）；
  - `query/mod.rs:1040` 的 `line_col(span)` 直转发 `span.column` ✗（与 CLI `--col` 的
    UTF-16 解释不同单位 ✓）—— 同族 ✓。

### #9 的进展（round 85 ✓）：**"会骗人"的那一半已修**
* **症状** ✗：`editor/vscode/extension.js` 的 `statusLabel`/`statusIcon` 都是
  "**兜底 = solved**" ✗ —— `open`/`failed` 之外**一律**显示 `solved` + `check` 图标 ✓
  ⇒ wire 上多一个新状态（或字段缺失 ✓）时，学习者会看到**"已解决"而其实没解决** ✗✗。
* **修法** ✓（AGENTS 验证纪律规则 4：「有就渲染」是反模式 ⇒ 缺失时给**可见信号** ✓）：
  三个已知态**逐个显式**映射 ✓；未知 ⇒ **原样回显** + **问号图标** + **一次性
  `console.warn`** ✓。词表的唯一来源仍是 `query/mod.rs::status_str` ✓（JS 只做 1:1 ✓）。
* **判据** ✓：`node editor/vscode/test-extension-host.js` ⇒ **34/34 passed** ✓
  （stub 宿主整轮 ✓）。
* ⏳ **仍未做**（审计给的正解 ✓）：让 **wire 带上 label**（或从生成物取常量 ✓），
  这样"三处词表"根本不会存在 ✓ —— `lsp/render.rs::status_label` 用 "solved ✓"/"exercise: open"
  而 JS 用 "solved"/"open"（**字符串本来就不同** ✗）⇒ 要合并得先决定可见文案归谁 ✓；
  另：一条专门断言"未知状态**不**显示成 solved"的判据还没加（stub 宿主未导出该映射 ✓）
  ⇒ 记着 ✓。

### #12（#5 的一部分）✅ 已修（round 86 ✓）
`crates/lsp/src/render.rs:52` 与 `crates/lsp/src/project_refs.rs:145` 的 `range_of(Span)`
**函数体逐字相同** ✓（diff 只差可见性与 doc ✓）⇒ 删掉 `project_refs` 那份 ✓、
改 `use crate::render::range_of;` ✓ ⇒ `span → LSP range` 在 `lsp/` 里只剩**一份实现** ✓。
**判据** ✓：`cargo test -p sokonanoda-lsp` ⇒ **161 passed** ✓（+ 其余套件 3/2 ✓）。
⏳ 仍待做：与 `query_map.rs:32 range_of_offsets`（**按 offset** 换算 ✓）的统一——
那是**另一条路**（不是副本 ✓），合并要先证明两者在非 ASCII 下等价 ✓（同 #5 的剩余项 ✓）。

### #15 ✅ 已修（round 87 ✓）：`redundant` 判据的**唯一归属**回到真相层
* **症状** ✗：`crates/lsp/src/lib.rs:1082-1095` **内联抄了** front 的
  `redundant_hole_spans` + `hole_is_redundant`（`query/mod.rs:1026/1037` ✓）——
  而 `query/mod.rs` 那段注释自己写着"（与 LSP 侧同一条规则）"✓ ⇒ **公认重复** ✓。
  分叉时的后果：同一个"多余的 `sorry`"，一侧算**真缺口**、另一侧算**多写一行**
  ⇒ 学生看到的**下一步指令相反** ✗。
* **修法** ✓：把真相层那两个 helper 改 `pub` ✓（`query` 的公开面 ✓），
  LSP 删掉内联副本 ✓、改调 `sokonanoda_front::query::{redundant_hole_spans,
  hole_is_redundant}` ✓（顺手删掉因此未用的 `use sokonanoda_front::Span;` ✗）。
* **判据** ✓：`cargo test -p sokonanoda-lsp` ⇒ **161 passed** ✓；
  `cargo test -p sokonanoda-front --lib` ⇒ **732 passed** ✓；`check`/`fmt` ✓。

## 2. 主线的抽查验证（纪律：产出**验证后才并入** ✓）

| 抽查项 | 结果 |
|---|---|
| `grep -c audit-wire-fields scripts/soko` | **0** ✓ ⇒ 第 3 条成立 ✓ |
| `kernel-diff.sh:82` 的 `find` 是否带 `-type f` | **没有** ✓ ⇒ 第 4 条成立 ✓ |
| `references.rs:157` 的 `column` 算法 | `offset - before.rfind('\n')…` = **字节** ✓ ⇒ 第 5 条成立 ✓ |
| 第 1 条（缓存污染） | 子代理给出**可复跑脚本**（`1 → 0` ✓）⇒ 采信 ✓（下一轮做的时候会当场复跑 ✓） |

⇒ 三份报告的**风险排序与唯一归属**可以直接采用 ✓；**未证实**的四条（18/19/20/22）
已按要求标注 ✓，处置=**立台账暂不做** ✓。

## 3. 处置（T-U10：或立刻做 / 或立守卫 / 或写台账 ✓）

**立刻做（按顺序，一处一 commit ✓）**：
1. **#8** 删 `render_folded`、只留 `DisplayNotations::render` ✓（**我自己的接口刚分叉了** ✗，
   而且白名单整文件豁免 ⇒ 无用守 ✓ —— 先修这个 ✓，否则 T-U2/T-U3 白做 ✓）
2. **#3 + #17** 把 `audit-wire-fields.py` 接进 gate ✓ 并把 `project-tree.js` 纳进消费点表 ✓
3. **#4 + #10** 收集器唯一化（`kernel-diff.sh` 补 `-type f` ✓；`check.py`/`e2e-merge.py`
   补 `is_file()` + `except OSError` ✓）
4. **#1** `clean` 判据收进 `store_if_clean*`（**已复现**的缓存污染 ✓）
5. **#2 + #13** `query check` 并入各模块诊断、计数口径唯一化 ✓
6. **#6** `suggest.rs` 的两份并入 `proof::render_atom` / 共享 `rfl_candidate` ✓
7. **#7** 词法符号表两处规则 + 六份装配合一 ✓（**R-2 复发通道** ✓）
8. **#5 + #12** offset↔line/col 合一到 `front::query::pos` ✓（G-36/T-D31 的根 ✓）
9. **#9 + #21 + #14 + #15** 词表/计数/判据三处收口（含删 JS 的 `solved` 兜底 ✗）

**立台账暂不做（附理由 ✓）**：#16（规则下沉是大改，先记 ✓）、#18/#19/#20/#22/#23/#24/#25（未证实或低风险 ✓）。

**守卫（T-U10 的硬要求 ✓）**：这一批里凡引入"唯一归属"的，都要配**棘轮化**守卫 ✓
（照 `audit-notation-paths.py` 的先例：基线 + 只拦新增 ✓ + **反向验证** ✓）。
