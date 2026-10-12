# 初学者体验缺口台账（2026-10-12）

四条探针线（语言/反馈 · LSP 服务面 · 真 VS Code 全流程 · vim）的**可执行索引**：每条缺口一行，
带复现命令、归属与复核状态。探针已入库 **`scripts/ux-probes/`**（原先只在 gitignore 的
`.cache/ux/**` ⇒ clone 就没了 ✗）；本文件是它们的**唯一索引**。

## 1. 构建身份与读数来源（跨轮比较前先读这一节）

| 项 | 值 |
|---|---|
| HEAD | **`30030b7a`**（2026-10-12 07:31，`fix(cli): grade --root 认模块根`） |
| 工作树 | **脏**：`crates/front/src/compile/{elab,implicit,meta,tests}.rs`（C 线隐式实参修复**在途**）· `editor/**`、`courses/set-theory/units/I.1/unit01-*.sokonanoda`、`docs/e2e/*`、`skills/*` |
| 语言线读数 | `.cache/ux/lang/REPORT.md`（HEAD `cb646c04`，`target/debug/sokonanoda` 22:58:29） |
| LSP 线读数 | `.cache/ux/lsp/REPORT.md`（HEAD `30030b7a`，LSP 07:38 重编） |
| 编辑器线读数 | `.cache/ux/editor/REPORT.md`（HEAD `cb646c04`，扩展自带 CLI/LSP `0.90.0`，真 VS Code 1.140） |
| vim 线读数 | `.cache/ux/vim/REPORT.md`（HEAD `cb646c04`，`extension.js` 138843 B @ 22:49:05，真 VSCodeVim 1.32.4） |
| 主会话自采 | `.cache/ux/main-findings.md`（M-01…M-07）· 修复计划骨架 `.cache/ux/fix-plan.md` |
| 独立复核 | `.cache/ux/verify/VERDICT.md` = **一致 48 · 误读 1（r02）· 已变 0**（只覆盖语言 23 + LSP 26） |
| 本次汇总复跑 | 语言 **25/25 跑过**（23 条 `r*.sh` + 新增 `m03`/`m07`）· LSP **26/26 跑过**（22 缺口 + 4 对照全绿）· 编辑器/vim **未复跑**（需真 VS Code，见 §6.6） |
| 复跑二进制 | `target/debug/sokonanoda` 2026-10-12 08:39 · sha256 前 16 = **`aa7d77b36f7ce818`**（含 C 线在途改动；VERDICT 用的是 07:47 的 release ⇒ 差异见 §6.2） |

**怎么读复现列**（一律从仓库根执行；脚本抬头各自写明判据）：
`lang/*.sh`：**exit 0 = 缺口仍在** · 1 = 已不复现。`*-probe.mjs --case <id>`：**exit 1 = 缺口仍在**
（0 = 观测与用例期望的好行为一致 · 2 = 用法 · 3 = 前置缺失）。`vim-probe.mjs --pair <id>`：
**exit 0 = 「无 vim 绿 + 有 vim 红」成立**（= 冲突是 vim 特有）。编辑器/vim 需要真 VS Code。

## 2. 🔴 blocker / high

| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核 |
|---|---|---|---|---|---|---|
| r01 | 语言 | `kernel-rejected` 报错是内核 de Bruijn 黑话（`第 N 个绑元`/`Set.[]`） | 高·blocker | `bash scripts/ux-probes/lang/r01-kernel-gibberish.sh` | front `compile/error.rs`+`report.rs` | 一致 |
| r03 | 语言 | `Eq.subst` 嵌进实参位被静默错误 elaborate（顶层同项却判绿） | 高·blocker | `bash scripts/ux-probes/lang/r03-nested-eqsubst.sh` | front `compile/elab.rs` | 一致 |
| r04 | 语言 | `apply` 不校验结论 ⇒ 错行无信号、留下不可证子目标 | 高·blocker | `bash scripts/ux-probes/lang/r04-apply-accepts-mismatch.sh` | front `compile/by.rs` | 一致（主会话独立复现） |
| r05 | 语言 | 隐式实参反解无规律（`Eq.symm`/`Eq.trans` 缺参、`congrArg` 掉黑话） | 高·blocker | `bash scripts/ux-probes/lang/r05-implicit-inconsistent.sh` | front `compile/implicit.rs` | 一致（**复跑转 exit 1**，见 §6.2） |
| r06 | 语言 | 入口自身 parse 错被标 `import-module-invalid` + 甩锅 hint | 高·blocker | `bash scripts/ux-probes/lang/r06-parse-blames-import.sh` | front `project/graph.rs:396-413`·`compile/error.rs:390-392` | 一致（对照描述已更正） |
| r07 | 语言 | parse 诊断漏 Rust `Token{…}`/`Span{…}` 内部结构 | 高·blocker | `bash scripts/ux-probes/lang/r07-parse-leaks-rust-debug.sh` | front `token.rs` / parse 诊断 | 一致 |
| r08 | 语言 | `congrArg A h` 报错含空名 Pi 绑定（`Pi ( : …)`） | 高 | `bash scripts/ux-probes/lang/r08-congrarg-empty-binder.sh` | front `compile/elab.rs` | 一致（**仍 exit 0**） |
| A1 | LSP | `by` 块洞的 quick fix 插入**项**而非 tactic ⇒ 应用即判红 | blocker | `node scripts/ux-probes/lsp-probe.mjs --case A1-codeaction-by-hole-breaks` | `crates/lsp` code action | 一致（复核独立复现） |
| H1 | LSP | 记法 hover 末块给的是**常量**类型（`a ∈ A : ∀ …`） | 高 | `node scripts/ux-probes/lsp-probe.mjs --case H1-notation-hover-type` | `crates/lsp/src/lib.rs` | 一致 |
| H2 | LSP | 同一 `def` 四处 hover 给四种信息（tactic 里的名字没有 `:=` 值块） | 高 | `node scripts/ux-probes/lsp-probe.mjs --case H2-hover-def-value-drift` | `crates/lsp` | 一致 |
| H3 | LSP | 同一行 `#check Iff.refl`：hover 与 Infoview 输出互相矛盾 | 高 | `node scripts/ux-probes/lsp-probe.mjs --case H3-hover-panel-drift` | `crates/lsp` | 一致 |
| C1 | LSP | 补全只在光标落在**已有 token** 上时给本域假设（行末不给） | 高 | `node scripts/ux-probes/lsp-probe.mjs --case C1-completion-binders-at-caret` | `crates/lsp` completion | 一致 |
| C2 | LSP | 补全**从不**给 import 进来的库名（`Set.*` 候选 0 条） | 高 | `node scripts/ux-probes/lsp-probe.mjs --case C2-completion-imported-names` | `crates/lsp` completion | 一致 |
| C5 | LSP | 服务面无 `\in` 输入法；字面 `\in` 被当集合差报错（vim 用户入口） | 高 | `node scripts/ux-probes/lsp-probe.mjs --case C5-completion-backslash-input` | `crates/lsp` + `editor/vscode` 客户端改写器 | 一致 |
| D1 | LSP | `by` 里 tactic 拼错 ⇒ 编辑器指到签名里的 `∈`（真错在下一行） | 高 | `node scripts/ux-probes/lsp-probe.mjs --case D1-diag-blames-wrong-line` | front 归因（同 r06）+ `crates/lsp` | 一致 |
| N2 | LSP | 一条没写完的声明 ⇒ 整份文件 hover/F12/大纲/灯泡全灭 | 高 | `node scripts/ux-probes/lsp-probe.mjs --case N2-nav-one-bad-decl-kills-file` | `crates/lsp` | 一致 |
| F1 | 编辑器 | 一个 parse 错 ⇒ 前面**正确**行的 `⊆` 被画红并劝补记法 | 高·blocker | `node scripts/ux-probes/editor-probe.mjs --case diag-parse-cascade` | front 级联 + `crates/lsp` | 未复核 |
| F2 | 编辑器 | 「打字即红」是常态：中间态级联误报每敲一键都出现 | 高·blocker | `node scripts/ux-probes/editor-probe.mjs --case diag-timing-typing` | 同 F1 | 未复核 |
| F3 | 编辑器 | 隐参诊断一侧记法一侧显式形式，且不说缺几个实参 | 高 | `node scripts/ux-probes/editor-probe.mjs --case diag-implicit-arg` | front 消息层 | 未复核 |
| F4 | 编辑器 | 诊断泄漏内核占位符（`Set.[]`/`第 1 个绑元`） | 高 | `node scripts/ux-probes/editor-probe.mjs --case diag-notation-fail` | 同 r01 | 未复核 |
| F5 | 编辑器 | 补全没有 `lib.*`、`.` 不开候选、binder 只在能编译时进候选 | 高 | `node scripts/ux-probes/editor-probe.mjs --case completion-word-vs-dot`（+`completion-palette`） | `crates/lsp`（= C2 的编辑器面） | 未复核 |
| vim-A | vim | 替换后光标右移一位 ⇒ 每个 `\word` 后多一个 `)`（用户那段证明错乱） | 高 | `node scripts/ux-probes/vim-probe.mjs --pair diag-caret-shift` | `editor/vscode/src/abbreviation-rewriter.js:115-135` | 未复核（修复在途） |
| vim-B | vim | normal 模式 `o`/`p` 把上一行待定缩写偷偷补刀成符号 | 高 | `node scripts/ux-probes/vim-probe.mjs --pair vim-nm-o-prefix` | 同文件 `:187-200` | 未复核 |
| vim-C | vim | `\` 撞 vim leader（NORMAL 起手时整串被吃） | 高 | `node scripts/ux-probes/vim-probe.mjs --pair pair-alpha-space` | 同文件 `:30` | 未复核 |

## 3. 🟠 medium

| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核 |
|---|---|---|---|---|---|---|
| r09 | 语言 | 缺隐参诊断的「示范写法」是字面 `Eq.symm …` 占位符 | 中 | `bash scripts/ux-probes/lang/r09-unsolved-msg-placeholder.sh` | front `elab.rs` | 一致（**复跑转 exit 1**，见 §6.2） |
| r10 | 语言 | 画布 hint 与画布自己写好的答案不是同一条路 | 中 | `bash scripts/ux-probes/lang/r10-hint-vs-answer-mismatch.sh` | `courses/set-theory`（课程内容） | 一致（依赖脏画布） |
| r11 | 语言 | hint 用点名 `A x`，`query state` 用记法 `x ∈ A` | 中 | `bash scripts/ux-probes/lang/r11-hint-goal-notation-mismatch.sh` | 课程内容 | 一致 |
| r12 | 语言 | 同一条报错一半记法一半点名，且定位晚 2 行（r04 的可见后果） | 中 | `bash scripts/ux-probes/lang/r12-mixed-notation-in-error.sh` | front 消息层 | 一致 |
| r13 | 语言 | `query holes` 的 `ty` 恒 null；与 `query goals` 形状不一致 | 中 | `bash scripts/ux-probes/lang/r13-holes-no-type.sh` | `crates/cli/src/query` | 一致 |
| r14 | 语言 | `query state` 被高亮 runs 撑爆（`--compact` 只去空白不删 runs） | 中 | `bash scripts/ux-probes/lang/r14-state-runs-bloat.sh` | `crates/cli` | 一致 |
| r15 | 语言 | `kernel-rejected` 的 span 覆盖整条声明，无法定位 | 中 | `bash scripts/ux-probes/lang/r15-kernel-span-whole-decl.sh` | front（同 r01） | 一致 |
| r16 | 语言 | 一处 parse 错把全部计数清零（学习者失去进度信号） | 中 | `bash scripts/ux-probes/lang/r16-parse-zeroes-all-counts.sh` | front parse 路径 | 一致 |
| r17 | 语言 | `intros`/`simp`/`rw` 被拒时不说「本子集不支持 + 替代」 | 中 | `bash scripts/ux-probes/lang/r17-common-lean-syntax.sh` | front parse 诊断 | 一致 |
| r22 | 语言 | 一条声明只报第一个错（修一处要重跑一次） | 中 | `bash scripts/ux-probes/lang/r22-one-diagnostic-per-decl.sh` | front 诊断聚合 | 一致 |
| r23 | 语言 | unit01 逼学习者手写底层 `Eq.subst`，包装在够不到的 `lib.Rel` | 低→中 | `bash scripts/ux-probes/lang/r23-no-subst-wrapper-in-unit01.sh` | 课程内容（import 集 / hint） | 一致（依赖脏画布） |
| M-03…M-06 | 主会话 | 目标写成 `Eq` 变体时报错是误导/废话（层级错、`???` 占位） | 中 | `bash scripts/ux-probes/lang/m03-eq-target-variants.sh` | front `elab.rs` + pp | 主会话实测（**在途**，见 §6.2） |
| M-07 | 主会话 | 缓存/模块产物键不含内建 prelude 摘要 ⇒ 改 prelude 后旧产物答旧行为 | 中 | `bash scripts/ux-probes/lang/m07-prelude-digest.sh`（键构成不变量；行为判据见 §6.5） | front `compile/cache.rs` | 主会话实测 |
| C3 | LSP | 补全与位置无关：四处给逐字节相同的 110 条 | 中 | `node scripts/ux-probes/lsp-probe.mjs --case C3-completion-position-blind` | `crates/lsp` | 一致 |
| C4 | LSP | prelude 名在补全里没有签名/实参个数 | 中 | `node scripts/ux-probes/lsp-probe.mjs --case C4-completion-prelude-arity` | `crates/lsp` | 一致 |
| A3 | LSP | 灯泡梯子很窄：四处全 0 条（`rfl`/构造子都没有） | 中 | `node scripts/ux-probes/lsp-probe.mjs --case A3-codeaction-ladder-thin` | `crates/lsp` actions | 一致 |
| A4 | LSP | 判红声明的 quick fix 只有 `Iff` 那半，`Eq` 分支缺失 | 中 | `node scripts/ux-probes/lsp-probe.mjs --case A4-codeaction-eq-refl-missing` | `crates/lsp` actions | 一致 |
| D2 | LSP | 类型不匹配消息一半折叠一半点名 | 中 | `node scripts/ux-probes/lsp-probe.mjs --case D2-diag-half-folded` | front 消息层 | 一致 |
| D5 | LSP | 消息漏出占位符 `???` 与无关的「请写 sorry」 | 中 | `node scripts/ux-probes/lsp-probe.mjs --case D5-diag-placeholder-leak` | front `token.rs` | 一致 |
| N1 | LSP | F12 落点的 `file://` URI 未规范化（`/crates/front/../../prelude`） | 中 | `node scripts/ux-probes/lsp-probe.mjs --case N1-nav-uri-unnormalized` | `crates/lsp` | 一致 |
| F6 | 编辑器 | 同一 Infoview 两种打印：声明列表 `Type 0` vs 目标卡片 `Type` | 中 | `node scripts/ux-probes/editor-probe.mjs --case goal-on-sorry` | `crates/lsp` + `editor/vscode` | 未复核 |
| F8 | 编辑器 | 状态栏「本文件 N」= 未完成练习数 ⇒ 写坏时 2→0（像做完了） | 中 | `node scripts/ux-probes/editor-probe.mjs --case solve-ex5-full` | `editor/vscode/extension.js:1459` | 未复核 |
| F9 | 编辑器 | `sorry` 警告挂在声明行（90）不是洞（92），点它选中整条声明 | 中 | `node scripts/ux-probes/editor-probe.mjs --case problems-jump-sorry` | `crates/lsp` 诊断 span | 未复核 |

## 4. 🟡 low

| id | 面 | 现象 | 严重度 | 复现命令 | 归属 | 复核 |
|---|---|---|---|---|---|---|
| r18 | 语言 | prelude 里没有 `id` | 低 | `bash scripts/ux-probes/lang/r18-id-not-defined.sh` | `prelude/` | 一致 |
| r19 | 语言 | 变量名打错不给候选建议，且消息中英混排 | 低 | `bash scripts/ux-probes/lang/r19-no-typo-suggestion.sh` | front 消息层 | 一致 |
| r20 | 语言 | 目标渲染多一层括号（`({a}) ⊆ A`） | 低 | `bash scripts/ux-probes/lang/r20-goal-extra-parens.sh` | front pp | 一致 |
| r21 | 语言 | `ty_runs` 把绑定变量标成 `unknown_ident`（63 处） | 低 | `bash scripts/ux-probes/lang/r21-tyruns-unknown-ident.sh` | front `semantic.rs` | 一致 |
| r24/E1 | 语言·LSP | `soko version --json` 把缓存 marker 当仓库构建身份（`0.83.0`/`0.73.0`） | 低 | `node scripts/ux-probes/lsp-probe.mjs --case E1-version-marker-misleading` | `scripts/soko` | 一致（读数随缓存变） |
| A5 | LSP | 同一判红声明给两条逐字节相同的 quick fix | 低 | `node scripts/ux-probes/lsp-probe.mjs --case A5-codeaction-duplicate` | `crates/lsp` actions | 一致 |
| D3 | LSP | `sorry` 警告是英文、覆盖整条声明；空 `by` 也谎称 uses sorry | 低 | `node scripts/ux-probes/lsp-probe.mjs --case D3-diag-sorry-english` | `crates/lsp` | 一致 |
| G1 | LSP | 目标/inlayHint 显示层加多余括号 | 低 | `node scripts/ux-probes/lsp-probe.mjs --case G1-goal-redundant-parens` | `crates/lsp` | 一致 |
| N4 | LSP | `documentSymbol.selectionRange` == 整条声明（8/8） | 低 | `node scripts/ux-probes/lsp-probe.mjs --case N4-nav-selection-range` | `crates/lsp` | 一致 |
| F10 | 编辑器 | `alt+shift+n` 不环绕（103→92→92），洞没有独立装饰 | 低-中 | `node scripts/ux-probes/editor-probe.mjs --case nexthole,prevhole` | `editor/vscode` | 未复核 |
| F11 | 编辑器 | hint 只在 toast 里、一次一条，没有可点入口 | 低 | `node scripts/ux-probes/editor-probe.mjs --case hints-visibility` | `editor/vscode` | 未复核 |
| F12 | 编辑器 | `sorry` 上没有 quick-fix / 灯泡候选 | 低 | `node scripts/ux-probes/editor-probe.mjs --case hole-lightbulb` | `editor/vscode` + `crates/lsp` | 未复核 |
| F13 | 编辑器 | 「这题过了 / 坏了」都没有显式信号（判卷数也不降） | 低 | `node scripts/ux-probes/editor-probe.mjs --case hole-after-fill,all-sorry-filled` | `editor/vscode` | 未复核 |
| F16 | 编辑器 | 「课程」「项目」视图在本次工作区是空的（是否设计如此未核对） | 低-中 | `node scripts/ux-probes/editor-probe.mjs --case course-map` | `editor/vscode` | 未复核 |
| F17 | 编辑器 | Infoview 自己建议 Rebuild 而版本明明一致（截图像素读数） | 低-中 | `node scripts/ux-probes/editor-probe.mjs --case infoview-project-block` | `editor/vscode` | 未复核 |
| F18 | 编辑器 | parse 失败时 Infoview 说「光标不在任何声明内」 | 低 | `node scripts/ux-probes/editor-probe.mjs --case infoview-project-block`（截图 `shots/completion-palette-suggest-palette.png`） | `editor/vscode` | 未复核 |

## 5. ✅ 已排除 / 设计如此（别当缺口修）

* **r02 `set_option pp.explicit true`「静默空操作」= 误读**（复核判「误读」，与主会话结论一致）：
  裸常量 `#check Eq.subst` **有意**不受该选项影响（走签名源文本捷径）——
  `reference/commands/set_option.md` §边界已写明，`crates/front/src/compile/tests.rs` 也钉着这条。
  实测 `#print Set.singleton` / `#check (fun x => Iff.refl x)` 开/关**都变**。
  ⇒ 真问题是**画布内容**：unit01 第 37 行挑了 `#check Iff.refl`（裸常量）这条看不见效果的演示。
* **编辑器「补全弹窗打不开」= 探针自纠，撤回**：原判据 `offsetParent !== null` 对 **fixed** 定位的
  `.suggest-widget` 恒为 null；行为判据（打 `exa` + Enter ⇒ 文本变 `exact`）证明弹窗工作。
  真问题是**候选内容**（F5），不是弹窗。
* **M-01 `grade --root` 被静默丢掉**：HEAD `30030b7a` 已修（复核实测 `query check --root` ⇒ exit 0 /
  `decl_checked: 1`）⇒ 不再是缺口。
* **r06 的「唯一变量 = 有没有 `∈`」对照不成立**（`r06-nomem` 第一条错是 `/\`，不是 `·` 子弹）；
  真判别量 = 单文件预解析是否先撞 `notation-unknown-symbol`。**现象本身仍是真 bug**（code 归因 + hint）。
* **LSP 侧不报的边界**：声明名位置 F12 = `null`（G-53 已知诚实行为）· `Nat.add`/`Nat.zero` F12 = `null`
  （内核按名字安装 ⇒ 如实没有落点，不许编位置）· `textDocument/diagnostic`（pull）= `-32601`
  （能力表未声明 `diagnosticProvider` ⇒ 只推不拉是合法形态）· `·` 焦点点不支持属**语言**缺口（另计 r07）。
* **vim 侧不算 bug 的**：`vim-undo-one-step`（一次替换 = 一个 undo 单元）是标准 vim 语义；
  `alt+n`/`alt+b` 键位本身是通的（第一版读到「不动」是探针没做焦点仿真）；`dd`/`gg`/`:w`/缩进全 PASS；
  **project 模式本身没有引入打字异常**（`proj-nota-space` vs `proj-unit-nota-space` 两条都对）。
* **正面（值得保留）**：`⟨h,h⟩` 用错类型的诊断 · 缺隐参诊断的正文 · `apply` 结论不匹配的诊断 ·
  `elab-unknown-identifier` 的 span 精确度 · `sorry`/开放练习状态清楚 · hover 签名（编辑器 F7）·
  Build/Clean 的反馈（F14）· 撤销/重做与保存（F15）· 编辑器 F1 的因果对照 `notation-clean-control`。

## 6. 🔄 复跑现状与仍未收口项（2026-10-12 15:5x 主会话复跑，**干净缓存**）

**已修好（对应探针由「缺口仍在」转「已变 / 一致」）**：
- **第一批（09:2x）**：`r04`（`apply` 结论校验 —— 负例现在报「`apply` 的目标不匹配…」并落在写错那行；两条正确
  用法仍绿）· `r05`/`r08`/`r09`（隐式实参第二候选 + `congrArg` 空名 Pi）· `r12`。独立夹具
  `.cache/ux/main-verify/c-probe.sokonanoda` **9/9 checked、0 诊断**（裸写 `Eq.symm hx` · `exact Eq.subst
  (Eq.symm hx) hp` · `apply Eq.subst (Eq.symm hx)` · 5 个既有绿例 · 用户原来的位置填隐参写法）。
- **第二批（长尾 ①–⑤）**：`A1`（`by` 块灯泡插入的是 **tactic** 文本 —— 洞带**位置模式**，`REPORT_SHAPE` 7→8）·
  `D1`+`r06`（入口自身的 parse 错归因到**自己** = 新码 `entry-parse-failed`；`query check` 与 `grade --json`
  诊断集**逐条相等**）· `r01`+`r15`（内核拒绝**源级渲染** + span 收到**出错子项**；判定不变：40 文件 × 2 臂
  对拍 0 差异）· `C1`–`C5`（补全给 **`import` 进来的库名 / prelude 签名 / 行末 binder / 位置分档 / `\in`
  输入法**）· `m07`（缓存键折进**内建 prelude 摘要** ⇒ 改了 prelude 不再答旧行为）。LSP 探针 `--all`：
  **8 一致 / 20 不一致 → 13 / 15**（只有 `A1`/`D1`/`C1`–`C5` 转一致，**零回归**）。

**⚠ vim 探针的期望已陈旧（16:5x 实测 7 FAIL，**不是**缺口）**：`diag-caret-shift` 期望 `(α x)` —— 那个 `)` 是
`autoClosingBrackets` 自动补的，而本批已按设计给 `[sokonanoda]` 钉 `"never"`；`vim-nm-o-prefix`/`vim-nm-enter-prefix`
期望 `["\\an",""]`，而规范判据 `scripts/vscode-input-vim-e2e.mjs --case normal-o-keeps-pending-prefix`（`\\an · <Esc> · o · Z`
⇒ `["\\an","Z"]`）**PASS** ✓；`ctl-enter-indent [off]` 是**无 vim 对照臂**却 FAIL（签名 = 掉了一个 `<CR>`）⇒ 判为**探针侧**。
**⚠ 探针自己的坑（本批修掉，判据随之修正）**：① **干净缓存纪律** —— 25 条语言探针原先都没隔离缓存 ⇒ 语言
改了以后会读 M-07 的旧条目**假红**（`r04` 实测：修好之后探针仍报 exit 0）⇒ 现已全部加 `mktemp` 缓存 + 关
两处产物 + `trap` 清理；② **`r02` 从「缺口」改成「对照臂」**（裸常量 `#check` 有意不受 `pp.explicit` 影响）；
③ **`m07` 判据原先歧义**（分不清"prelude 摘要已折进"与"设了 `SOKO_BUILD_LABEL`" ⇒ 会**假报已修** ✗）⇒ 改成
三侧可分辨：旧式 ⇒ exit 0 / 新式 ⇒ exit 1 / 两者都不是（设了 label）⇒ **exit 2 UNDECIDABLE**。
## 7. 🔁 复核方法（怎么复跑一条）

```bash
cd <repo>                                                     # 一律从仓库根跑
bash scripts/ux-probes/lang/r01-kernel-gibberish.sh; echo $?  # exit 0 = 缺口仍在（判据在脚本抬头）
node scripts/ux-probes/lsp-probe.mjs --list                   # 26 条用例清单
node scripts/ux-probes/lsp-probe.mjs --case A1-codeaction-by-hole-breaks --json   # exit 1 = 缺口仍在
node scripts/ux-probes/lsp-probe.mjs --all --json > /tmp/lsp.json                 # 全量
node scripts/ux-probes/editor-probe.mjs --case diag-parse-cascade --json          # 真 VS Code（慢）
node scripts/ux-probes/vim-probe.mjs --pair diag-caret-shift                      # exit 0 = 对照臂成立
```

* **退出码约定**：`lang/*.sh` = 0 缺口仍在 / 1 已不复现；`lsp-probe.mjs` = 0 与用例 `expect` 一致 /
  1 不一致（缺口仍在）/ 2 环境缺失；`editor-probe.mjs` = 0 与期望一致 / 1 不一致 / 2 用法 / 3 前置缺失
  （⚠ `--case completion-behavior` 的 1 语义相反：它判「Enter 该换行」，不成立 = 补全在工作）；
  `vim-probe.mjs --pair` = 0 表示「无 vim 绿 + 有 vim 红」成立。
* **探针自检**：LSP 的 4 条对照（`A2`/`D4`/`G2`/`N3`）与编辑器 `notation-clean-control`、
  vim 的 `novim` 臂**必须绿** —— 它们红了说明探针本身坏了，读数作废。
* **干净缓存纪律**（M-07 的教训：旧产物会答旧行为）：复核读数一律
  `SOKONANODA_CACHE_DIR=$(mktemp -d)` + `SOKONANODA_NO_PROJECT_ARTIFACTS=1` +
  `SOKONANODA_NO_MODULE_ARTIFACTS=1`。⚠ 探针会重写 `.cache/ux/**` 下自己的夹具与工作区（gitignored）
  —— 那是探针状态，不是编辑仓库文件。
* **构建身份**（跨轮比较前先钉）：`git rev-parse --short HEAD` + 二进制 mtime/sha256。
  ⚠ **别信** `scripts/soko version --json` 的 `cli.marker`/`lsp.marker`（缓存 marker，见 r24/E1）；
  LSP 用 `soko/version` 自述。本次复跑身份 = HEAD `30030b7a` + debug 08:39 sha256 前 16 `aa7d77b36f7ce818`。
* **真 VS Code 的纪律**：独立 profile + 独立 CDP 端口（editor 9601 / vim 9501–9502），profile 放
  `$TMPDIR`（路径 >103 字符 ⇒ 起窗即退），跑完 `pkill -f user-data-dir=<profile>`；残留窗口会占端口
  ⇒ 新窗口绑不上、读数连到旧窗口 ⇒ 脚本必须**拒绝跑**而不是硬来。没有别的 VS Code 探针在跑时才跑。
* **本文件的过期日 = 2026-11-12**（`scripts/docs-expiry.json`，`process` 档）：到期三选一 ——
  续期（写**特定**理由）· 删除 · 归档（`docs/archive/README.md` 点名）。
