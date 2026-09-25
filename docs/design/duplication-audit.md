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
| 14 ✅**已加交叉断言**（round 88 ✓）：两套计数不再"无约束并存" | "还剩几个练习"**两套**（事件口径 `query/mod.rs:529` vs 声明口径 `report.rs:109-116`） | 互相，且都进 wire ✗ | 中 —— 两条通道给出两个数，**无断言钉死相等** ✗ | ✅ 加了**下界**断言 ✓（`声明侧 >= 事件侧` ✓）—— **相等是错的** ✗（G-01：签名坏 ⇒ 不发事件而声明仍 Open ✓） | `grep -rn "open_exercises\|exercise_open"…` → 生产者 2 个、**无相等断言** ✓ | 立刻做（加断言） |
| 15 ✅**已修**（round 87 ✓）：`redundant` 判据收进真相层、LSP 只调 |（`query/mod.rs:1012/1023` vs `lsp/lib.rs:1082-1095`） | 互相；`query/mod.rs:1021` 自己写着"与 LSP 侧同一条规则" ✓ | 中 —— 分叉时"多余的 `sorry`"两侧给出**相反**的下一步指令 ✗ | ✅ `sokonanoda_front::query::{redundant_hole_spans, hole_is_redundant}` 已公开 ✓，LSP 内联副本已删 ✓ | `grep -rn "hole_is_redundant" crates/front/src/query/mod.rs crates/lsp/src/lib.rs` ✓ | 立刻做 |
| 16 | LSP 合成 `code:"sorry"` 警告 + 全冗余抑制（`lsp/lib.rs:1095-1122`） | `query/mod.rs:579-596`（CLI **无**合成/抑制 ✗） | 中 —— 编辑器比 CLI 多一条警告；"该不该提示"只活在 LSP 一侧 ✗ | 规则下沉到真相层 ✓ | `sed -n '1105,1125p' crates/lsp/src/lib.rs` vs `sed -n '579,596p' crates/front/src/query/mod.rs` ✓ | 立台账（暂不做 ✓） |
| 17 ⏳**已实测、未修**（round 72 ✓） | `audit-wire-fields.py` 消费点表**不扫 `project-tree.js`** ✗ | `project-tree.js:24-28,45-53,62-72,169-197` 读 `soko/project` 大载荷 ✗ | 中 —— 它是"有就渲染"的宽容实现 ⇒ 停发字段**静默降级**、守卫永不响 ✗（R-1 同类、换文件 ✓） | 把 `project-tree.js` + `ProjectView` 加进守卫 ✓ | `grep -n 'collect(' scripts/audit-wire-fields.py`（只有 IV/EX ✓） | 立刻做需**先扩守卫**：直连 `ProjectResponse` 会报 8 个 MISSING ✗，但**实测是假阳性** ✓（`soko query project` 响应 = `{project, reason}` ✓，字段嵌在 front 侧 `ProjectView`/`ModuleView` ✓）⇒ 先让 `fields()` 能读 `crates/front/src/query/types.rs` ✓，在那之前**不接** ✗（常红守卫比没有更糟 ✓） |
| 18 | 版本钉源链 **4 份**（`perf-ledger.sh:39`/`perf-report.sh:12`/`vscode-e2e.sh:102`/`new-course-repo.sh:37` 各自只读 `Cargo.toml` ✗） | `scripts/soko:224-346` 的源链 ✓ | 中（**未证实漂移** ✓：当前四处都 `0.67.0` ✓）—— 风险在**看不见**：一旦加 `sokonanoda-version.txt`，台账仍按 `Cargo.toml` 记版本 ✗ | 唯一归属 `scripts/soko version --json` ✓ | `grep -rn "grep -m1 '\^version' Cargo.toml" scripts/` ✓ | 立台账（暂不做 ✓） |
| 19 | `bump.py:90-97` 要求 `requires` **全等 `x.y.z`** ✗ vs `manifest.rs:135-150`（只比 **major.minor** ✓）与 `soko:300-313`（`x.y` 是合法约束 ✓） | 三处语义相反 ✗ | 中（**未证实**：现生效清单都是 `0.67.0` ✓，`bump.py` 现为绿 ✓） | 以 `manifest.rs` 的语义为唯一判据 ✓ | `python3 scripts/bump.py` ✓ | 立台账（暂不做 ✓） |
| 20 | `gap.py:14` 把**用法错误折进 exit 1** ✗（全仓约定 1 = 有拒绝/漂移 ✓；同一条件在 `notation-lint.py` 是 2、`check-site.py` 是 3 ✗） | 各脚本 docstring 各写一遍退出码 ✗ | 中 —— 调用方把"命令行写错"读成"台账漂移" ✗ | 一份 `scripts/_exit.py` ✓ | `grep -n '退出码' scripts/*.py` ✓ | 立台账（暂不做 ✓） |
| 21 ✅**已修**（round 89 ✓）：状态栏说出范围（"本文件 N" ✓）+ tooltip 点明对比 | `extension.js:391` vs `project-tree.js:28` | 中 —— 同一窗口两个数、用户无法分辨 ✗ | ✅ 状态栏文本改为 `本文件 N` ✓、tooltip 注明"下面那行是**整个项目**的" ✓ —— 数**本身**没改（当前文件那个数是对的 ✓），改的是**用户能不能分辨** ✓ | `grep -n openCount editor/vscode/extension.js` ✓ | 立刻做（小改 ✓） |
| 22 ✅**已修**（round 98 ✓）：夹具改用 `canonicalize` 后的路径拼 URI | 两条 URI 构造路（已咬过人 ✓） | 低 | ✅ 三处都改成 `file_uri()`（`canonicalize` 后拼 ✓，不引新依赖 ✓）| `grep -rn 'file://{}' crates/lsp/tests` → 3 处 ✓ | 立台账（暂不做 ✓） |
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

### #14 ✅ 已加交叉断言（round 88 ✓）—— 且**刻意不加等号**
`crates/front/src/query/tests.rs::open_exercise_counts_never_contradict_each_other` ✓：
* 两个计数**判据本就不同** ✓（`open_exercises()` 数 `DeclStatus::Open` **声明** ✓ vs
  `counts.exercise_open` 数 `ExerciseOpen` **事件** ✓）—— G-01 那类"**签名坏 ⇒ 不发
  `exercise.open`**"会让声明仍 Open 而事件缺失 ✓ ⇒ **`assert_eq!` 是错的** ✗。
* 所以断言的是**下界** ✓：**每个事件都对应一条 Open 声明** ⇒ `声明侧 >= 事件侧` ✓
  （抓"声明状态与事件流打架" ✗：例如给非 Open 声明发了 open 事件 ✓）。
* ⚠ **我第一版判据假红过一次** ✗：用 `project_report_ref()` 取声明侧，而单文件夹具里
  它是 `None` ✓ ⇒ `unwrap_or(0)` 得到 0 ⇒ 报"0 < 1"✗（看着像产品 bug ✓）。
  改用 **query 自己的答案**（`goals()` 的 `status` ✓，与 wire 同源 ✓）后通过 ✓。
  这段教训写进了测试注释 ✓（免得下一个人重犯 ✓）。

### #21 ✅ 已修（round 89 ✓）：两个"N 练习"各自说出范围
* **症状** ✗：状态栏 `extension.js:1154` 原来只显示**裸数字** `$(circle-outline) N`
  （= **当前文件** `decls.filter(status === "open")` ✓），tooltip 也只说"N 个练习未完成" ✗；
  而项目树/`projectStatusLine` 那个数 = **整个闭包** `counts.open_exercises` ✓
  ⇒ 同一窗口两个数、**用户无法分辨** ✓。
* **修法** ✓：状态栏文本改为 `$(circle-outline) 本文件 N` ✓；
  tooltip 首行点明"**本文件**还有 N 个…"✓，并在附上项目那行时明确写
  "下面这行是**整个项目**的：" ✓ —— **范围写进文本**，不只写进 tooltip ✓
  （状态栏本来就只有一瞥的时间 ✓）。
  **数本身没动** ✗：当前文件那个数是**对的**（它就是本文件的 ✓），错的是"没说清是谁的" ✓。
* **判据** ✓：`node editor/vscode/test-extension-host.js` ⇒ **34/34 passed** ✓ · 语法自检 ✓
* ⏳ 仍未动：项目树那行的文案（`project-tree.js:28` 的 `${counts.open_exercises} 练习` ✓）——
  它坐在"`N 模块 · M 失败 · K 练习`"这种**项目级**行里 ✓、范围已由上下文隐含 ✓，
  且 e2e 可能断言这行文本 ✗ ⇒ 先不动 ✓（真要改，连同 e2e 一起 ✓）。

### #22 ✅ 已修（round 98 ✓）：夹具不再手拼 `file://`
三处（`tests/common/mod.rs:130` 的 `rootUri` ✓、`tests/lsp_cache.rs:53`、
`tests/lsp_edit_concurrency.rs:54` ✓）原来是 `format!("file://{}", path.display())` ✗
⇒ 手拼出的可能带 `..` ✓，而服务端发的是 `Url::from_file_path` 规范化后的 ✓
⇒ **两边字符串不同、谁也认不出谁** ✗ —— 这个形状**已经咬过人** ✓
（`perf_course.rs` 的注释记着它 ✓；2026-09-25 的 ubuntu e2e flake 是**同族的 JS 版** ✗，
那处已在 e2e watcher 上用 `realpathSync` 规范化键修掉 ✓）。
修法 ✓：新增 `file_uri()`（`std::fs::canonicalize` 后拼 ✓，不引新依赖 ✓；
`common/mod.rs` 里因落在 `impl` 内 ⇒ 用 `Self::file_uri` ✓）。
**判据** ✓：`cargo test -p sokonanoda-lsp` ⇒ **161 passed / 0 failed** ✓（+ 附带 3 / 2 ✓）。

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

---

## 3. **T-U10 收口：77 处绕过的逐条处置**（用户 2026-09-25 要求 ✓，见 `REQUIREMENTS.md` §9 ㉟）

**硬规则（用户原话）**：**凡产出用户可见文本的（LSP / hover / 诊断消息 / 状态栏 / 项目树）
必须走 ① 或 ②** ✓（① 迁移到唯一接口 / ② 补"必须折叠"的判据 ✓）；**不许拿 ③ 糊过去** ✗。

### §3 的分组表**已按当前基线（89）刷新** ✓（2026-09-25 round 111 ✓）
> **口径**：以 `scripts/notation-paths-baseline.txt` 的**当前 89 条**为准 ✓
> （用户 2026-09-25 要求 ✓）。与下面那张**旧 77 表**的差别有**两处原因** ✓：
> ① 守卫修准（`(?<![\w:.])` ⇒ `(?<!\w)` ✓）**多抓 15 处** ✗（旧数是**低估** ✓）；
> ② `round 106` 迁掉 `kernel_phase.rs` 3 处 ✓（-3 ✓）⇒ 77 + 15 − 3 = **89** ✓。

| 组 | 处数（**89 版** ✓） | 位置 | 产出什么 | 处置 |
|---|---|---|---|---|
| **A** | **5** | `crates/lsp/src/lib.rs`（全在 `half_expression_goals_hover` :1332-1376 ✓） | **hover 文本** ✗ | **① 优先** ✓（需报告结构小扩展 ⇒ 见下 ✓） |
| **B** | **20** | `crates/front/src/compile/elab.rs` | 诊断消息（**只有 4 处是给人看的** ✓，其余 17 处是判卷/解析输入 ✗） | **① 4 处** ✓ / **③ 17 处** ✓（§3 的 B 组逐条表 ✓） |
| **C** | **22** | `crates/front/src/by.rs` | tactic 候选/错误文本（混判定输入 ✗） | 逐条判 ⏳ |
| **D** | **17** | `crates/front/src/compile/goals.rs` | 目标/候选文本（混判定输入 ✗） | 逐条判 ⏳ |
| **E** | **8** | `crates/front/src/compile/check/walk.rs`（**守卫修准后从 1 → 8** ✗） | 目标生产（**喂 judge** ✗） | **③ 不做** ✓（折它 = `suggest::*` 判红 ✓，见 §9 ㉜ ✓） |
| **F** | **6** | `crates/front/src/semantic.rs`（`tag_runs_with_notations` ✓） | 分段标签 | **②** ✓（接缝守卫已钉 ✓） |
| **G** | **3** | `crates/front/src/compile/prelude.rs`（**守卫修准后新可见** ✗） | prelude 的类型文本 | 逐条判 ⏳（多为**内建声明**的源级文本 ✓） |
| **H** | **2 + 1** | `crates/front/src/compile/check/mod.rs`（`render_expr`×2 + **`print_back`×1** ✓） | 显示副本生产（**这类正是"该迁移"的** ✓） | **①** ✓（`print_back` 那处与 `kernel_phase` 同款 ✓） |
| **I** | **3** | `crates/front/src/compile/tests.rs` | **测试期望串** | **③ 不做** ✓ |
| **J** | **1** | `crates/front/src/judge.rs` | **判定量** | **③ 不做** ✓（内核红线 ✓） |
| **K** | **1 + 1** | `crates/front/src/suggest.rs`(1) · `crates/front/src/query/mod.rs`(1，`tag_runs` ✓) | 候选文本 / wire runs | 逐条判 ⏳ / **②** ✓ |
| — | **0** ✅ | ~~`kernel_phase.rs`~~ | ~~`ty_text`/`val_text`~~ | **✅ round 106 已迁** ✓（基线 -3 ✓） |

**下一步顺序**（round 107 定 ✓）：**H 组的 `print_back` 那处**与 **B 组 4 处消息**最容易 ✓
（都在 front 内部 ✓）；**A 组**需要先做报告结构扩展 ✓。

---

**历史（旧 77 版，保留对照 ✓）**：**基线全貌**（`scripts/notation-paths-baseline.txt`，共 **77** 处 ✓，按 文件×调用 归组 ✓）：

| 组 | 处数 | 位置 | 产出什么 | 处置 | 理由 |
|---|---|---|---|---|---|
| **A** | **5** | `crates/lsp/src/lib.rs`（全在 `half_expression_goals_hover` ✓ :1332-1376 ✓） | **hover 文本** ✗ | **① 优先** ✓ | **用户可见** ⇒ 硬规则强制 ①/② ✓。5 处逐条见下表 ✓ |
| **B** | **19** | `crates/front/src/compile/elab.rs` | **诊断消息**片段 ✓（含拼消息 ✗） | **① 优先** ✓ | **用户可见**（诊断会显示给学习者 ✓）⇒ 硬规则 ✓ |
| **C** | **17** | `crates/front/src/compile/goals.rs` | **开放练习的候选/目标文本** ✓ | 逐条判 ✗ | 有些是**判卷输入** ✗（红线 ✓：不许折 ✓）⇒ 只迁**显示副本** ✓ |
| **D** | **22** | `crates/front/src/by.rs` | tactic 引擎的**候选/错误文本** ✓ | 逐条判 ✗ | 同上：判定量不许折 ✗；显示量走显示副本 ✓ |
| **E** | **6** | `crates/front/src/semantic.rs`（`tag_runs_with_notations` ✗） | **分段标签** ✓ | **②**（已有 ✓） | 它靠 `runs` 那层 ✓：接缝守卫已钉"`runs` 拼接 == `text`" ✓；**再补**"输入必须已折过" ✓ |
| **F** | **1** | `crates/front/src/query/mod.rs`（`tag_runs_with_notations` ✓） | **wire 的 runs** ✓ | **②**（已有 ✓） | 同上 ✓；`text` 侧已改显示副本 ✓（T-U4 ✓） |
| **G** | **3** | `crates/front/src/compile/tests.rs` | **测试夹具** ✓ | **③ 不做** ✓ | 测试自己的期望串 ✓，不是用户可见文本 ✓ |
| **H** | **1** | `crates/front/src/compile/check/mod.rs` | 编译期诊断拼串 ✓ | 并入 **B** ✓ | 与 B 同类 ✓ |
| **I** | **1** | `crates/front/src/compile/check/walk.rs` | 目标生产 ✓ | **③ 不做** ✓ | **它是判卷输入** ✗（红线 ✓：折它 = `suggest::*` 当场判红 ✓；见 §9 ㉜ ✓） |
| **J** | **1** | `crates/front/src/judge.rs` | 判定用文本 ✓ | **③ 不做** ✓ | **判定量** ✗（折它就会改变接受/拒绝 ✓ —— 内核红线 ✓） |
| **K** | **1** | `crates/front/src/suggest.rs` | 候选文本 ✓ | 逐条判 ✗ | 候选要**能解析**✓ ⇒ 折与不折都可能对 ✓；**② 补判据**为主 ✓ |

**A 组 5 处逐条**（用户点名 ✓）：
| # | 位置 | 产出 | 处置 |
|---|---|---|---|
| A1 | `lsp/lib.rs:1332` `term_text = render_expr(body)` | hover 里的**项**文本 ✗ | **①**：走 `render_text`（用报告里的 `notations` ✓） |
| A2 | `lsp/lib.rs:1340` `goal_text = render_expr(&goal_ty)` | hover 里的**目标**文本 ✗ | **①**（同上 ✓） |
| A3 | `lsp/lib.rs:1359` `goals.push(render_expr(domain))` | 部分应用的目标列表 ✗ | **①**（同上 ✓） |
| A4 | `lsp/lib.rs:1364` `goals.push(render_expr(…))` | 同上 ✗ | **①**（同上 ✓） |
| A5 | `lsp/lib.rs:1376` `codomain = render_expr(cur)` | 同上 ✗ | **①**（同上 ✓） |

**落地要点（A/B 两组 ✓）**：front 已有 `DisplayNotations`（`print_back` 那一族 ✓），
而 `ProjectReport` 里带着 `notations: Vec<NotationDecl>` ✓（`report.rs:137` ✓）
⇒ 需要一个**公开的**"按报告折一段文本"入口 ✓（例如
`query::fold_text_with(report, text) -> String` ✓），LSP/hover 与 elab 的诊断拼串都走它 ✓
—— **本次未实现** ✗（上下文不足 ✓），但落点已精确到行 ✓。

### A/B 组的**落地设计**（round 102 实测定的 ✓ —— 不是简单委托 ✗）
**实测**：`DisplayNotations { table: Vec<NotationDecl>, arity: HashMap<String,usize> }`
（`display.rs:108-116` ✓）⇒ 建表**必须有 ARITY 表** ✗，而 arity 是从**声明的类型**算出来的
（`arities_in_commands` ✓，`display.rs:641` ✓）。`ProjectReport` 里带的
`notations: Vec<NotationDecl>`（`report.rs:137` ✓）**只有声明点** ✓ ⇒
**LSP / hover 这一侧拿不到 arity** ✗ ⇒ **折不了** ✗（本 session 第三次撞到同一堵墙 ✓：
`query::runs` 那次、`goal_display` 那次、这次 ✓ —— 每次都是"**表不在这一层**" ✓）。

⇒ **正解（与 T-U4 同款 ✓）**：**显示副本在 front 的生产/查询层折好** ✓，
LSP 只**搬运** ✓。具体：给 hover/诊断要用的文本在 **front 侧**产出**折过的副本** ✓
（照 `ty_text` / `val_text` / T-U4 的 `goal_display` 的样板 ✓），
A 组 5 处与 B 组 19 处改成读那份副本 ✓ —— **而不是**在 `lsp/lib.rs` 里调 `render_expr` ✗。
**这条设计结论取代**本文件 §3 里"加一个公开 `fold_text_with(report, text)`"的初稿 ✗
（那需要 front 侧把 arity 也放进报告 ✓，改动面更大且没必要 ✓）。

**因此 T-U11 的第一步是**：找出 A/B 两组各自**该由哪个 front 生产者**产出显示副本 ✓
（`half_expression_goals_hover` 的输入从哪来 ✓、`elab.rs` 的诊断消息在哪一步拼 ✓），
**先定这个再动代码** ✗（否则就是又一次"在错的层上修" ✓）。

### ✅ **T-U11 的第一次实际迁移**（round 106 ✓）：`kernel_phase.rs` 的 `ty_text` 走唯一接口
* **迁移前**：`crates/front/src/compile/check/kernel_phase.rs:104/123/321` 直接调
  `crate::display::print_back(&text, display).as_display_str().to_string()` ✗
  （正是 T-U12 副发现指出的那条路 ✓ —— `ty_text`/`val_text` 与开放练习的类型副本 ✓）。
* **迁移后**：`display.fold(&text)` ✓ —— **零行为变化** ✓（`fold` 就是
  `print_back(text, self).as_display_str().to_string()` ✓，见设计 §2 ✓）。
* **判据** ✓：`cargo check` ✓ · `cargo test -p sokonanoda-front --lib` ⇒ **735 passed** ✓
  （含 T-U12 的面级判据 `a_kernel_pp_display_surface_must_be_folded` ✓）
  · 守卫：**92 → 89** ✓（`--rebless` ✓）· `audit-notation-paths.py` ⇒ 无新增绕过 ✓。
* ⚠ **基线数字更新** ✓：上一轮修好守卫后是 **92** ✓（不是 77 ✓ —— 旧数字漏检了
  **路径限定**调用 ✗，见守卫自己的注释 ✓）；本轮的 **89** 是新基准 ✓。
  §3 的分组表**仍按旧 77 编** ✗ ⇒ 待按 **89** 刷新 ⏳（新抓的那些集中在
  `walk.rs`(8) / `kernel_phase.rs`(3，本轮已迁 ✓) / `check/mod.rs` 等 ✓）。

### A 组的**精确落点**（round 107 实测 ✓）
`crates/lsp/src/lib.rs:1283 half_expression_goals_hover(report, text, offset, decls)` ✓
自己 `parse_expr_text` + `peel_pi_layers` + `render_expr` ✓ ⇒ 它渲染的是**源码 AST**
与**剥出来的 Pi 层** ✓。**点形式从哪里漏** ✗：剥出来的 `domain`/`codomain`
是**内核侧**的类型 ✓（不是源码里的记法 ✓）⇒ 5 处 `render_expr` 里，
真正需要折叠的是这些**内核侧子项** ✓。

**好消息（少走一步 ✓）**：报告里的 `DeclState` **已经带折过的副本** ✓：
`ty_text` / `val_text`（`report.rs:109/114` ✓）—— 它们就是**整个类型/值**的折叠结果 ✓
⇒ A 组的**整类型**那一类（`goal_text = render_expr(&goal_ty)` ✓）可以直接改读它 ✓。

**卡点（与 round 102 同一条墙 ✓）**：`domain`/`codomain` 是**子项** ✗，
报告里**没有**对应的折叠副本 ✓，而 LSP **拿不到 arity 表** ✗（`DisplayNotations` 需要它 ✓）
⇒ 折不了 ✓。**正解（下一步 ✓，设计与 T-U4 同款 ✓）**：让 **front 把"折叠能力"带进报告** ✓
—— 最小改法是在 `ProjectReport` 里带上**记法表 + arity**（或直接带一个
`Vec<NotationDecl>` + `HashMap<String,usize>` ✓，即 `DisplayNotations` 的字段 ✓），
并给 front 一个**公开的** `fold_text(&self, text) -> String` ✓；LSP 只调它 ✓。
**不许**在 LSP 侧重建 arity ✗（那会变成第五套实现 ✓ —— 守卫会抓 ✓）。

**本轮结论** ✓：A 组**不能**用"简单替换"完成 ✗（需要一次 wire/报告结构的小扩展 ✓）；
**先做 B 组更容易** ✓（`elab.rs` 在 front 内部 ✓ —— 那里**本来就有**表 ✓，
可以就地走 `fold` ✓，零结构改动 ✓）。⇒ **顺序调整：B 组优先** ✓。

### B 组逐条处置（round 108 实测 ✓，21 处 = 基线 20 + 1 ✓）
**判据**：看**这一处的产出是不是"给人看的消息"** ✓。是 ⇒ **①**（折 ✓）；
是**判卷/解析输入** ⇒ **③ 不做** ✓（**折它 = 改判定** ✗，内核红线 ✓，见 §9 ㉜ ✓）。

| # | 位置 | 产出什么 | 结论 |
|---|---|---|---|
| 1 | `elab.rs:1513` | binder 记法 guard **报错消息**里的 guard 文本 ✓ | **① 折** ✓ |
| 2 | `elab.rs:1754` | ``` `{candidate}` 的结果类型是 `{}` ``` **消息** ✓ | **① 折** ✓ |
| 3 | `elab.rs:2158` | ``` `{}` 的签名 `{}` 里有 N 个隐式参数… ``` **消息** ✓ | **① 折** ✓ |
| 4 | `elab.rs:2161` | 同上消息里的 `ty_text` ✓（**本来就是折过的** ✓） | **已 ①** ✓ |
| 5 | `elab.rs:2434` | ``` `⟨…⟩` 的期望类型 `{}` … ``` **消息** ✓ | **① 折** ✓ |
| 6 | `:2003 · :3042 · :3742 · :4192` | `judge_infer(…, &render_expr(x))` ✗ **判定输入** | **③ 不做** ✓（折它 = 改判定 ✗） |
| 7 | `:1556 · :1611 · :1692` | `render_expr(&Expr::Ident{…})` 供 `resolve_known` **名字解析** ✗ | **③ 不做** ✓（解析要**点名**形式 ✓） |
| 8 | `:515 · :557 · :736 · :755 · :889` | `GoalBinderSpec.ty` / 归纳/递归子的 `signature` ✓ | **③ 不做** ✓（**喂 judge** ✗；显示侧由 T-U4/T-U5 的 `runs` 负责 ✓ —— `notation_fold.rs` 第 2 组的行程开关钉着这条 ✓） |
| 9 | `:1356 · :4221 · :4656` | 内部/期望类型文本（参与推断 ✓） | **③ 不做** ✓（同上 ✓） |

⇒ **B 组的实际工作面只有 ~4 处**（1–5 里除 4 ✓）✓ —— 而不是"20 处一起折" ✗。
**下一步** ✓：确认 `elab.rs` 那几处**能不能拿到记法表** ✓（若 `ElabCtx` 没有 ✓，
就沿用 A 组的正解：**由 front 的生产侧产显示副本** ✓，而不是在 `elab.rs` 里重建表 ✗）。

### 迁移与基线流水（T-U11，逐条可核 ✓）
| round | 动作 | 基线 |
|---|---|---|
| 105 | 守卫修准（漏检**路径限定调用** ✗⇒✓） | 77 → **92** ✓ |
| 106 | 迁 `kernel_phase.rs:104/123/321`（`ty_text`/`val_text` ✓） | 92 → **89** ✓ |
| 111 | 迁 `check/mod.rs:392`（`by_step_states` 的 `fold` 闭包 ✓） | 89 → **88** ✓ |
| 111 | 守卫精修：**跳过定义/再导出签名行** ✓（`check/mod.rs:1243` 的 `pub fn render_expr` 与
其体内再导出**不是绕过** ✗ ⇒ 去掉 2 条假条目 ✓） | 88 → **86** ✓ |
**判据** ✓：每次迁移后 `cargo test -p sokonanoda-front --lib` 全绿 ✓（735 ✓，含 T-U12 面级判据 ✓）；
每次 `--rebless` 后 `audit-notation-paths.py` ⇒ 无新增绕过 ✓；`--self-test` ⇒ **仍咬得住** ✓。

### A 组与 B 组的**共同卡点**（round 111 实测 ✓）⇒ 需要**同一次**结构扩展
* **A 组**（LSP hover ✓）：`half_expression_goals_hover(report, …)` 里 **LSP 拿不到 arity 表** ✗
  （报告只有 `notations: Vec<NotationDecl>` ✓，`DisplayNotations` 需要 arity ✓）。
* **B 组**（`elab.rs` 的 4 处消息 ✓）：`ElabCtx`（`elab.rs:445` ✓，字段 `prefix_src`/`options`/
  `inductives`/`ns`/`defs` ✓）**完全没有** `DisplayNotations` ✗（`grep DisplayNotations elab.rs`
  ⇒ **零命中** ✓）⇒ 那 4 处**就地折不了** ✗。
⇒ **两个组卡在同一条**：**"表不在这一层"** ✓（本 session 第 **4** 次 ✓：`query::runs` ✓、
`goal_display` ✓、A 组 ✓、B 组 ✓）。
**一次扩展同时解两组** ✓（下一步的设计 ✓）：
1. `ElabCtx` 加 `notations: &'b DisplayNotations` ✗ ⇒ B 组 4 处走 `ctx.notations.fold(text)` ✓；
2. `ProjectReport` 带上**表 + arity**（或一个公开的 `fold_text(&self, text)` ✓）⇒ A 组 5 处走它 ✓。
**红线** ✓：**不许**在任何一层"重建" arity ✗（= 第五套实现 ✓，守卫会抓 ✓）；
**也不许**把 `ElabCtx` 的表用在**判定路径**上 ✗（B 组那 17 处 ③ 的理由不变 ✓）。
**顺序** ✓：先 1（B 组 ✓，改动面小、判据现成 ✓）后 2（A 组 ✓）。

### `ElabCtx` 扩展的侦察结果（round 112 ✓）—— **不是"加个字段"** ✗
| | 实测 |
|---|---|
| 构造点 | **9 处** ✗：`walk.rs:420/651/865/986/1267/1313` ✓、`elab.rs:662` ✓、`prelude.rs:446/557` ✓ |
| 记法表在哪可得 ✓ | **`walk.rs` 的 6 处**（`self.display` ✓，`check/mod.rs:830` 建的 ✓） |
| 记法表在哪**不存在** ✗ | **3 处**：`elab.rs:662` ✓、`prelude.rs:446/557` ✓（**都不在 check 阶段** ✓） |
⇒ 直接加 `notations: &DisplayNotations` ✗ 会逼着往上游传表 ✓（改动面**不止 9 处** ✓）。
**可行设计（待做 ✓）**：加 **`Option<&'b DisplayNotations>`** ✓ ——
`walk.rs` 的 6 处填 `Some(&self.display)` ✓、其余 3 处填 `None` ✓；
4 处消息在 `Some` 时走 `fold` ✓、`None` 时按原样 ✓（"**没有表的地方 ≠ 显示上下文**" ✓，
与 `prelude.rs` 注释里那句"没有表的地方传空表（不影响）"同一种约定 ✓）。
⚠ **不许**把 `None` 当默许去**重建**表 ✗（第五套实现 ✓，守卫会抓 ✓）。
**本轮未动** ✗（9 处 + 语义分叉 ⇒ 上下文不足 ⇒ 不留半成品 ✓）。

### ⚠ `ElabCtx` 扩展的**硬约束**（round 128 实测 ✓，下次别踩 ✗）
**`sokonanoda-front` 把 `dead_code` 当错误** ✗：只加字段、不读它 ⇒
```
error: field `notations` is never read
  --> crates/front/src/compile/elab.rs:456:9
```
⇒ **"加字段"与"至少一处真实使用"必须落在同一个提交** ✓（分两步做会编译不过 ✗）。

**下次的正确做法（本轮已把前半做完并验证 ✓，只是回退了 ✓）**：
1. 加字段 ✓（`elab.rs` 的 `ElabCtx` ✓）—— 已实测**编译器会精确点名全部 9 处** ✓：
   `walk.rs:420/651/865/986/1267/1313` ✓（填 `Some(&self.display)` ✓）、
   `elab.rs:668` ✓、`prelude.rs:446/557` ✓（填 `None` ✓）；
2. **同一提交里**改一处消息 ✓ ⇒ 字段被读 ✓ ⇒ 编译通过 ✓。
**机械填法（本轮验证过 ✓，别用正则 ✗）**：正则只匹配到 2/6 ✗ —— 要用**编译器给的行号**
**从后往前**插（行号不漂 ✓），循环到 `rc==0` ✓。
**4 处消息的真实签名（本轮已读出 ✓）**：
| 位置 | 函数 | 有没有 `ctx` |
|---|---|---|
| `elab.rs:1513` | `binder_notation_operand` | **有** ✓（首选 ⇒ 第一处就改它 ✓） |
| `elab.rs:1754` | `candidate_list(candidates: &[&str])` | **没有** ✗（纯格式化 ⇒ 要给它加参数 ✗） |
| `elab.rs:2158` | `try_implicit_application` | 有 ✓ |
| `elab.rs:2434` | `anon_ctor_target` | 有 ✓ |
⇒ 第一处改 `binder_notation_operand` 的**消息文本** ✓（折它 ✓）；其余三处随后 ✓
（`candidate_list` 要么加参数 ✓、要么不折 ✓ —— 它是**消息** ✓ ⇒ 按硬规则该折 ✓）。

### ✅ **T-U11 第二次迁移**（round 129 ✓）：`ElabCtx` 拿到记法表 + 3 处诊断消息走唯一接口
* **字段** ✓：`ElabCtx.notations: Option<&'b DisplayNotations>`（`elab.rs:445` ✓）——
  **只许消息路径用** ✓，判定/解析路径一律不折 ✗（内核红线 ✓）。
* **9 个构造点全填** ✓（用**编译器行号、从后往前**插 ✓，一次成功 ✓）：
  `walk.rs` ×6 ⇒ `Some(&self.display)` ✓；`elab.rs:668`、`prelude.rs:446/557` ⇒ `None` ✓
  （`None` = "这里不是显示上下文" ✓，沿用 `prelude.rs` 既有约定 ✓）。
* **助手 `render_msg(ctx, expr)`** ✓：`Some` ⇒ `n.render(expr)` ✓（= 唯一接口 ✓）；
  `None` ⇒ `render_expr(expr)` ✓（原样 ✓）。
* **迁移的 3 处消息** ✓：`binder_notation_operand`（`render_expr(guard)` ✓）、
  `try_implicit_application`（`render_expr(head)` ✓）、`anon_ctor_target`（`render_expr(expected)` ✓）
  —— 都是**给人看的诊断文本** ✓（按用户硬规则必须折 ✓）。
* ⏳ **剩 1 处**：`candidate_list(candidates: &[&str])`（`:1754`）**没有 `ctx`** ✗
  ⇒ 要么给它加参数 ✓、要么不折 ✓（它是消息 ⇒ 按硬规则该折 ✓）。
* **判据** ✓：`cargo check` **rc=0** ✓（`dead_code` 那条错误消失 ⇒ 字段确实被读了 ✓）·
  `cargo test -p sokonanoda-front --lib` ⇒ **735 passed / 0 failed** ✓（行为不变 ✓）·
  `audit-notation-paths.py` ⇒ **无新增** ✓ · 基线 **86 → 84** ✓（`--rebless` ✓）· fmt ✓

### ✅ **B 组（`elab.rs` 诊断消息）** 全部迁完 ✓（round 130 ✓）—— 4/4
* 第 4 处 = `describe_candidate_results` ✓（产出"`候选` 的结果类型是 `…`" ✓ —— **给学习者看的消息** ✓）：
  给它加了 `ctx: &ElabCtx<'_, '_>` 参数 ✓、`render_expr(result)` ⇒ `render_msg(ctx, result)` ✓、
  调用点（`elab.rs:1738` ✓）同步补 `ctx` ✓。
* **判据** ✓：`cargo check` 干净 ✓ · `cargo test -p sokonanoda-front --lib` ⇒ **735 passed / 0 failed** ✓ ·
  `audit-notation-paths.py` ⇒ **无新增** ✓ · `--self-test` ⇒ **仍咬得住** ✓（`walk.rs` 8 处 ✓）·
  基线 **84 → 83** ✓（`--rebless` ✓）· fmt ✓
* **B 组小结** ✓：4 处**用户可见诊断**全部走唯一接口 ✓（`render_msg` ✓）；
  其余 17 处按 ③ **不做** ✓（判卷输入 ✗ / 名字解析 ✗ / 喂 judge ✗ —— 折它们会改判定 ✓）。
* ⏳ **下一步 = A 组**（LSP hover 5 处 ✓）：需要 `ProjectReport` 带表 + arity ✓
  （或公开 `fold_text` ✓）；红线 ✓：**不许**任何一层**重建 arity** ✗（第五套实现 ✓，守卫会抓 ✓）。

### 🔴 **新缺陷（round 156 实测 ✓）**：`kernel-rejected` 诊断漏出**原始内核 pp** ✗
**证据** ✓（探针原文 ✓，折叠**开与关完全相同** ✗）：
```
code = kernel-rejected
message = 类型不匹配：期望 `Pi (A : (Set.[] Nat.[])), Pi (B : (Set.[] Nat.[])),
          (((Set.subset.[] Nat.[]) $1) $0)`，实际是 `Pi (…), Sort(0)`
```
⇒ 这是**给学习者看**的诊断 ✓（"类型不匹配" ✓），却把内核 pp 原样吐出来 ✗ ——
`Set.[]` / `Set.subset.[]` / `$1` / `Sort(0)` ✓ **既不是记法、也不是干净的点形式** ✓。
**用户硬规则** ✓（`REQUIREMENTS.md` §9 ㉟）：「凡产出用户可见文本的（… 诊断消息 …）
必须走 ① 或 ②」✗ ⇒ 它**当前两条都没走** ✓。
**归属** ✓：这条**不在** T-U11 的 79 条基线里 ✓（基线只扫 `render_expr`/`print_back`/
`tag_runs_with_notations` ✓）⇒ 它是**守卫的盲区** ✓ —— 消息由**内核拒绝**那条路拼出来 ✓
（`kernel-rejected` ✓，源头在 judge/内核→诊断的转换处 ✓）。
**下一步（先定位再改 ✓）**：
① `git grep '"类型不匹配'`（或 `类型不匹配：` ✓）找拼串点 ✓；
② 看它有没有记法表可用 ✓（若有 ⇒ 直接走 `fold` ✓；若没有 ⇒ 与 A/B 组同款"把表带进去" ✓）；
③ 判据 ✓：该消息在 `SOKO_NO_NOTATION_FOLD=1` 下**必须变**（能咬 ✓ —— 因为它是**内核 pp** ✓，
不是源级渲染 ✓）；若它现在**两态相同** ✗ ⇒ 判据就是"**两态必须不同**" ✓
（这比"无点形式"更贴切 ✓：先把折叠接上 ✓，再谈记法 ✓）。

### 🔴🔴 **更正（round 158 实测 ✓）**：round 157 那个"修复"**无效** ✗ —— 但它换来一条**真正的诊断** ✓
**证据** ✓（判据在两态下**都红** ✓、消息**一字不差** ✓）：
```
折叠开：… 期望 `Pi (A : (Set.[] Nat.[])), …, (((Set.subset.[] Nat.[]) $1) $0)` …
折叠关：… 完全相同 ✗
```
⇒ `display.fold(&expected)` **什么也没做** ✓。**真因** ✓：
那段文本是**内核的 debug 形式** ✗（`Set.[]` / `$1` / `Sort(0)` / `Pi (…)` ✓），
**不是**前面那套可折叠的渲染形式 ✓ ⇒ **折叠器解析不了它** ✓ ⇒ 折与不折同值 ✓。
（对照 ✓：`ty_text` 那条路折的是 `render_expr` 出的**源级形状** ✓ ⇒ 能折 ✓；这条不是 ✓。）

**⇒ 真正的修法方向（不是"折一下" ✗）** ✓：**换渲染器** ✓ ——
`expected` / `actual` 是在 `kernel_phase.rs` 里由 `format!("{e}")` 从**内核错误**取来的 ✓
（`:160` / `:373` ✓ 的 `let msg = format!("{e}")` ✓）⇒ 要让**内核/诊断转换处**用
**可读的类型打印**（前端的 `render_expr` 形状 ✓，或内核的 surface printer ✓），
**而不是** `Debug`/`Display` 的 `Set.[]` 形状 ✓。折记法是**第二步** ✓；第一步是**先给出能折的文本** ✓。

**方法论收获（值得单独记 ✓）** ✓：这一轮的判据**红了两次** ✓ —— 但它证明的不是"判据写错了"✗，
而是"**修复没生效**"✗。**把"弄红"的纪律用在修复上，同样有效** ✓：
如果只看 `cargo check`/`cargo test` 全绿 ✓ 就宣布修好 ✓，这个缺陷会**原样留在**用户面前 ✗。

### 🔴🔴 真修法的**两条路线**（round 159 侦察 ✓）
`Set.[]` **不是内核里的字面量** ✗ —— 它是 printer **拼出来**的 ✓
（`名字 + ".[]" + 宇宙层` ✓，`crates/kernel/src/debug_printer.rs` 那族 `fmt` ✓）。
⇒ 修法有两条，**代价差很远** ✓：

| 路线 | 做法 | 代价 |
|---|---|---|
| **A. 改内核 printer** ✗ | 让它输出可读类型 ✓ | **侵入内核** ✗ ⇒ 触发红线流程（三层回归 + 语料逐字节对拍 ✓）；而且 `. []` 这种形状可能是**别处依赖**的 ✓ ⇒ **不首选** ✗ |
| **B. front 侧带出结构化类型** ✓✅ | 在**内核→诊断**的构造点 ✓，让错误**一并带出类型**（`Value`/`Expr` 结构化 ✓）；`kernel_phase.rs` 用它 + front 的可读渲染器打印 ✓ | **最小侵入** ✓：内核只**多带一个字段** ✗（不改既有判定 ✓）；front 侧改动可控 ✓ |

**⇒ 首选 B** ✓。**下一步（第 ① 步 ✓）**：读**内核错误类型** ✓（`crates/kernel` 里那个被
`format!("{e}")` 打印的东西 ✓）—— 看它**是否已经**带类型字段 ✓：
* **已带** ✓ ⇒ 直接在 `kernel_phase.rs` 用 front 渲染器打印 ✓（改一处 ✓）；
* **没带** ✗ ⇒ 在内核侧**加字段** ✓（结构化 ✓、不动判定 ✓，仍走三层回归 ✓）。

**判据已经就绪** ✓✓：round 158 写的那条（**两态必须不同** + 折叠后出现 `⊆` ✓）
**已经证明能咬** ✓（它红过两次 ✓）⇒ 渲染一改好它就会**转绿并从此锁住** ✓。

### **G 组（`prelude.rs`，3 条）逐条处置** ✅（round 169 ✓）
三条**完全同形** ✓：`:470` · `:492` · `:591`，都是
```rust
signature: Some(crate::proof::render_expr(ty)),
```
⇒ 它们造的是 **`GoalBinderSpec.signature`** ✓ —— 与 **B 组**里 `elab.rs:515/557/736/755/889`
那些**同款** ✓（round 108 已判过 ✓）。
| 位置 | 产出 | 结论 |
|---|---|---|
| `prelude.rs:470` · `:492` · `:591` | **`GoalBinderSpec` 的 signature** ✓（内建/prelude 声明的 binder 规格 ✓） | **③ 不做** ✓ |
**理由** ✓（与 B 组那 5 处**同一条** ✓）：`GoalBinderSpec` 是**喂 judge 的输入** ✗（
`judge_binders` ✓）—— **折它 = 改判定** ✗（内核红线 ✓）；而**显示侧**由 T-U4/T-U5 的
`runs` 负责 ✓（`notation_fold.rs` 第 2 组的**行程开关**钉着这条 ✓）。
⇒ G 组 **0 条迁移 / 3 条 ③** ✓ —— 它们此前**不在**旧基线里 ✓（守卫修准后才露出来 ✓，
`round 105` ✓），所以是**新可见、但不需要动**的一批 ✓。
---

### **I 组（`compile/tests.rs`，3 条）· J 组（`judge.rs`，1 条）逐条处置** ✅（round 171 ✓）
**I 组** —— 三条都在**同一个往返测试**里 ✓（`tests.rs:1501-1505` ✓）：
```rust
assert_eq!(render_expr(&expr), expected, "source: {source}");        // :1501
let back = parse_expr_text(&render_expr(&expr))                      // :1503
assert_eq!(render_expr(&back), expected, "round-trip: {source}");    // :1505
```
⇒ 它们验的是**渲染器的往返性质** ✓（源 → AST → 文本 → AST → 文本 ✓）⇒ **测试内部的期望串** ✗
⇒ **③ 不做** ✓（在那里折记法就**换了一个被测对象** ✗ —— 测的就不再是"往返"了 ✓）。
**J 组** —— `judge.rs:1113` ✓：
```rust
_ => render_expr(expr),          // 判定侧渲染器的**回退分支**
```
⇒ 它是**判定量** ✗（`judge` 模块 ✓）⇒ **折它 = 改判定** ✗（内核红线 ✓）⇒ **③ 不做** ✓。
⇒ **两组共 4 条：0 迁移 / 4 条 ③** ✓ —— 与组级结论一致 ✓、理由各自具体 ✓。
---

### **F 组（`semantic.rs`，5 条）· K2 组（`query/mod.rs`，1 条）逐条处置** ✅（round 173 ✓）
`tag_runs_with_notations` 这一族**不是**"绕过接口"✗ —— 它是 **`runs` 的**生产机制**** ✓
（T-U4/T-U5 的"显示副本"正建立在它上面 ✓）。
| 位置 | 产出 | 结论 |
|---|---|---|
| `semantic.rs:241` | **真实生产调用** ✓（`text, decls, binders, &[]` ✓ 由编译侧喂入 ✓） | **② 已有** ✓（T-U5 的**接缝守卫**钉着 `runs` 拼接 == `text` ✓） |
| `semantic.rs:1201/1217/1483/1505` | **测试内部**的调用 ✓（断言 `runs` 的分段 ✓） | **③** ✓（测试自己的期望 ✓） |
| `query/mod.rs:495` | **wire 的 `runs` 生产者** ✓（`semantic::tag_runs_with_notations(text, decls, binders, notations)` ✓） | **② 已有** ✓（同上 ✓） |
**K 组（`suggest.rs`）现在是 0 条** ✓ —— 该文件的 `render_expr` 已被 `render_msg` 包装 ✓
（守卫的"已折写法不算绕过"规则 ✓，round 133 ✓）⇒ **无事可办** ✓。
---

### **E 组（`check/walk.rs`，8 条）逐条处置** ✅（round 174 ✓）—— 只有**两种形状** ✓
| 形状 | 位置 | 产出 | 结论 |
|---|---|---|---|
| `signature: Some(render_expr(ty))` ×6 | `:503` `def` ✓ · `:598` `def` ✓ · `:709` `theorem` ✓ · `:822` `theorem` ✓ · `:896` `axiom` ✓ · `:943` `axiom` ✓ | **`GoalBinderSpec.signature`** ✓ | **③** ✓ —— 与 **G 组完全同因** ✓（喂 judge ✗，折它 = 改判定 ✓ 内核红线 ✓） |
| `goal: render_expr(ty)` ×2 | `:737` `theorem` ✓ · `:1055` `example` ✓ | **walk 累积的"开放练习目标"** ✓ | **③** ✓ —— **有实测证据** ✓：T-U4 那轮我折了它 ⇒ **5 条 `suggest::*` 当场红** ✗（§9 ㉜ 的行程开关 ✓），已回退 ✓ |
⇒ **8 条全 ③** ✓，理由分两类 ✓（6 条同 G 组 ✓、2 条有**历史实测**背书 ✓）。
⚠ **为什么不放进白名单** ✗：`--self-test` **正是拿这个文件当靶子** ✓
（"应当抓到 walk.rs 里的绕过" ✓）⇒ 排除它 ⇒ **自检瞎掉** ✗（round 171 实测 ✓）
⇒ **这 8 条留在基线里** ✓ —— **守卫的可验证性 > 数字好看** ✓。
---

### **C 组（`by.rs`，22 条）逐条处置** ✅（round 175 ✓）—— **全部在判定侧** ✓
按**所属函数**归类后一目了然 ✓（**一个显示面的都没有** ✗）：

| 所属函数 | 行 | 性质 | 结论 |
|---|---|---|---|
| `level_hint_of` | `:60` `:63` | `infer(&render_expr(…))` ✓ 宇宙层推断 | **③** ✓ |
| `restore_universe_levels` | `:257` | `judge_infer(…)` ✓ | **③** ✓ |
| `canonical_goal_type` | `:513` | `judge_render_type(…)` ✓ | **③** ✓ |
| `canonical_goal_with_spec` | `:563` | `judge_render_type(…)` ✓ | **③** ✓ |
| `run_tactics` | `:953` `:954` `:965` `:1164` | `spec.ty` / `term` / 余式 —— **tactic 引擎的判定输入** ✗ | **③** ✓ |
| `solve_type_params` | `:1240` | `probe` ✓ 合一求解 | **③** ✓ |
| `apply_tactic` | `:1277` `:1349` `:1350` `:1351` | `term` / 候选 / `codomain` / `goal` ✓ | **③** ✓ |
| `cases_tactic` | `:1486` `:1550` | `goal_ty` / `scrutinee_ty` ✓ | **③** ✓ |
| `spec_of_for_judge` | `:1892` `:1904` | **函数名就写着** `for_judge` ✓✓ | **③** ✓ |
| `exact_tactic` | `:2015` | `nodes[cur].ty` 喂判卷 ✓ | **③** ✓ |
⇒ **22 条全 ③ / 0 迁移** ✓ —— 与 round 108 的**组级**结论一致 ✓，
但现在**逐条**落到了**所属函数**上 ✓：**这一组没有任何用户可见面** ✗
（它们全部服务 `front::judge` 与 tactic 引擎 ✓ —— **折它们 = 改判定** ✗ 内核红线 ✓）。
⚠ **留在基线里** ✓（46 条待迁移的账目要真 ✓；白名单只收"不会再被守卫自检用到"的文件 ✓
—— `by.rs` 不在其中 ✓）。
---
