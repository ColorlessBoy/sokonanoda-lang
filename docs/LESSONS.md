# 开发经验台账（LESSONS）

> 每条一行结论 + 出处/守护位置。新 agent 接手先读 `AGENTS.md` 的文档顺序，
> 踩坑后把新经验**追加到本文件**（一条一行，注明在哪被守护——代码注释/
> 契约测试/文档小节），不要让它只活在某个会话的记忆里。

## 内核交互（kernel）

- 内核拒绝 = panic → `try_check_declar` catch_unwind；**任何新的内核交互点
  （#check/#reduce/hover/judge）都必须包 `quiet_catch`/同款 catch_unwind**，
  否则一条 `#check (Type) 3` 崩掉整个编译/LSP 进程（第九轮实修）。
- panic hook 是**进程全局**的（`take_hook`/`set_hook`/恢复）——quiet_catch
  与 `resolve_hovers` 同模式；**不可嵌套调用**，嵌套会互相吞 hook。
- conv 的 body-expr 快路径有闭包语义前提（eval 闭包 vs infer 闭包对同一
  interned `Var 0` 表示不同值）——动 conv 前必读 `docs/architecture.md` §6
  的 soundness 修复记录；回归测试在 `tests/memory_api.rs`。
- def_eq 的 `def_eq mismatch expected: … | actual: …` 是 front 的解析契约；
  新增拒绝消息统一走 `got:` 渲染增强（`render_value_for_def_eq_error`），
  别发明第二种格式。
- `CheckError::Rejected` 会给 payload 包 `rejected: ` 前缀——消息匹配必须
  先剥前缀（`refine_kernel_kind` 已处理，新增匹配别忘）。
- arena 生命周期：每次 `run_pass` 新 arena；跨 update 只复用渲染产物
  （Session 快照），内核对象绝不出界（跨不出 arena 的 `ExprPtr` 只能
  `with_pp` 渲染成 String 带出，见 `resolve_hovers`）。

## 前端与判定（front）

- **判定永远走 kernel**（judge = 合成完整声明走标准流水线）；任何"建议"
  （exact/refine/intro）都只是结构生成，判定与"期望/实际"反馈全由内核出
  （`front::judge`）。
- **归纳块双契约**：内核自算 `is_recursive`（构造子 telescope binder 类型
  是否提到归纳名——注意 `ctor base : (b : Bad) -> Bad` 的字段在 result 的
  箭头链里，parser 的 binders 是空的）并断言 front 传入一致；内核还要求每
  块注册 Recursor（每构造子一条 iota 规则）。front 必须镜像扫描 result 箭头
  链（守护：`non_recursive_inductive_block_compiles_and_reduces` +
  architecture §8 0b）；**字段数（num_fields/ctor_telescope_size_wo_params）
  同样按完整 Pi 望远镜计**（第十四轮契约修正）；无 rec 块由 front 自动派生
  recursor + iota 规则（合成 AST 复用显式 rec 路径，形状锚点 = py-nat 手写
  版；守护：`auto_derived_recursor_*` 三测）。
- 依赖 binder 折叠**必须折成单个 Forall 望远镜**（嵌套独立 Arrow 的 domain
  会在空作用域里 elaborate 而报 unknown）——`judge::fold_declared`。
- 宇宙多态：裸名默认 u=0，跨宇宙判定必须显式 `id.{u}`（judge 测试踩过）。
- prelude 决策（explicit-Nat 探测 / Eq all-or-nothing / 防遮蔽）**依赖整份
  文件内容**——Session 增量正确性的根基（`docs/design/i8-i9.md` §1）。
- 多洞走查：**值头是构造子名、目标头是族名**（`And.intro` vs `And`），对齐
  要经模板的 `name` 字段校验，不能直接比头（多洞实现踩过）。
- 洞位/子目标永远由 server 端 walk 产出（`DeclState.holes/sub_goals`）；
  客户端禁止文本扫洞——已编码为契约测试（`extension.rs`）。

## 工程

- **front 降低/判定路径禁止 per-keystroke 的全文档重编译**（O(n²)，funapply
  因此整体移除；by 块 tactic 的 judge_infer 同源，已加指纹缓存封顶 128 条）。
  任何需要内核信息的特性：要么缓存、要么只在显式请求（hover/code action）
  时计算——半截表达式的 goal-state hover 即此模式。
- **演示资产例行化**：官网 GIF/图由 `scripts/gen-site-demos.py` 渲染；改了
  编辑器交互/文案必须重跑并提交（pages workflow 的 `--check` 步骤会在漂移
  时红掉）。演示画面与实现一一对应，不是假截图。
- **tag 已存在 → 推送不会发布**（auto-tag 幂等跳过）。0.23.0 的 sorry
  fallback 修复（a181102）在 tag 切走之后才推上去，静默漏发——用户装到的
  marketplace 0.23.0 不含该修复，报错"依旧存在"实为旧二进制。铁律：
  **任何要发布到用户手里的改动（crates/editor），落 commit 就必须同步
  bump 版本**（Cargo.toml + package.json 双处，auto-tag 会校验一致）；
  「先修后补发版」不存在，只有「下一版」。docs-only 改动除外。
## 工程流程（subagent / 门禁 / 测试）

- **并行 subagent 必须文件集互斥**：任务书里写死"允许修改的文件清单 +
  验收命令（test/clippy/fmt）"，主会话独占公共文件的接线点（先接好接口
  再派发）——两轮实践验证。
- **subagent 断网/失败 ≠ 工作丢失**：先 `git status` + 跑测试核实残局，
  完整且全绿就直接收尾，不要盲目重跑（第十轮实况）。
- 新错误码 = 三件套：`ErrorKind` 变体 + `docs/protocol.md` 条目 + 中文
  hint；def_eq 之外的内核消息分类进 `refine_kernel_kind`（消息形态见
  审计报告的"分桶"节）。
- golden 是契约：课程/协议 golden 变更必须"刻意"并在提交说明里给出新旧
  计数（course.rs 顶部注释）。
- lint/fmt 门禁形态：教学 crates 各自 `[lints.rust] warnings = "deny"`
  注入严格度；kernel 冻结快照保持 warning 级；fmt 门禁只覆盖教学 crates
  （kernel 的 rustfmt.toml 需要 nightly）。
- 内核冷路径改动（panic 消息、`got:` 渲染）允许，但每处都要三层回归
  （kernel 单测 + CLI e2e + 语料）并在 `docs/architecture.md` §6 记账。
- criterion 多 target 包跑基准必须带 `--bench pipeline` 选择器——`cargo
  bench -p sokonanoda-front` 会先跑 lib unittest target 并拒绝 criterion
  旗标（守护： benches/pipeline.rs 顶部注释）。
- **外部调研文档（vscode-notes 等笔记类）也会过期**——落地与调研结论冲突
  时必须回头改笔记，否则文档带人跳坑（node_modules 一事故）。

## 编辑器与打包（lsp / vscode）

- vsce 打包：把依赖移进 `dependencies` ≠ 能打包；**node_modules 绝不能进
  `.vscodeignore`**（vsce 靠它把生产依赖装进 VSIX）——两处契约测试分别
  守护（`runtime_dependency_is_packaged` / 打包元数据完整性）。
- LSP stdout 纪律：stdio 是协议通道，服务器任何日志只进 output channel
  （stderr）；`--json` 视图才是 stdout 的合法内容（CLI 侧）。
- 坐标系：front span 是字节 offset + 1-based 行列；LSP 是 0-based 行 +
  UTF-16 列——换算集中在 `encode_semantic_tokens`/`offset_to_line_col`，
  别在业务代码里散落换算（增补平面字符踩过）。
- 洞位置、nextHole 逻辑放 server 端（ocaml-lsp 教训），客户端零位置推导。
- **发布后核对 Marketplace 要认「索引延迟」**：`extensionquery` API（POST
  `/_apis/public/gallery/extensionquery`，`filterType:7` = 扩展全名，
  `flags:3` 带版本）在 `vsce publish` 成功后**还要几分钟**才收录新版本；别
  据此判失败。也**别**用 `/_apis/public/gallery/publishers/<p>/vsextensions/<n>/<v>`
  探测——那个路由根本不存在，一律 404，最容易被误读成「版本没上架」。判据
  看 `lastUpdated` 是否推进 + 轮询 `versions` 里出现目标版本号（本轮实测
  延迟约 2 分钟）。步骤日志要鉴权（未登录拉 `/actions/runs/<id>/logs` 是 403），
  public 仓库也一样，所以「job 步骤 success」是能拿到的最强信号（v0.17.0）。

## 教学（course / playground）

- **中文文风硬约束**：拒绝翻译腔/AI 味/抖音味/小红书味，规范在
  `skills/sokonanoda-teacher/references/zh-style.md`；写文案的 subagent
  任务书必须引用它；自查法 = 通读圈四类（物理思考动词/"X很Y："起手/
  抽象名词主语句/残留英文），圈出即重写（不是修补）。（第一课实修：
  "被一张票'居住'了"这类翻译腔直接被学习者点出。）

- 课程排序哲学：**逻辑先行**（REQUIREMENTS §6）——先证明命题，Sort 等
  "函数类型的类型"问题自然出现再揭晓；`Eq.{1}` 先当机械规则、悬念后置。
- kernel 只判类型不判意图：`double := fun n => n` 也能过——语义要求用
  证明形状表达（`two_def` 闭环模式，teaching-session §2）。
- 解答钥匙必须经完整内核验证（course/solutions + golden）；本次流程自己
  就靠它抓出过 Eq.symm 钥匙缺实参（被自家 LSP 实时抓出）。

## prelude 新增内建类型必须带「显式声明让位」闸（2026-09-15，0.41.0 Bool）

- **教训**：把 `Bool` 加进 prelude 时，若不做让步判断，任何自带
  `inductive Bool` 的教学文件都会在 `builder.add_inductive` 的重复声明检查处
  **panic**（不是返回错误码）。仓库里已有两个这样的用例（`tt`/`ff`）。
- **规矩**：每个 prelude 内建类型在 `check.rs::run_pass` 都要有
  `explicit_<name>` 判断（文件自行声明即不装），并同步
  `session.rs::PreludeShape`（否则增量会话不会因新增/删除该声明而失效）。
- 参考：`install_prelude`/`install_bool_prelude`（`crates/front/src/compile/prelude.rs`）、
  `explicit_nat`/`explicit_bool`（`check.rs`）。

## 模式编译器用「源到源 canonical 化」而非手写内核项（2026-09-15，0.42.0）

- **教训**：把「每构造子一条 arm」扩成嵌套/字面量/守卫，若直接手写
  `Ind.rec` 应用 + lambda，会掉进 de Bruijn/作用域泥潭（本项目 dependent motive
  就修过一次索引 bug）。
- **做法**：编写器把用户模式**源到源**编译成「每构造子一条 arm、参数全是绑定」
  的 `Expr::Match` 树（嵌套匹配作为 arm body），再交回既有 lowering 逐层处理；
  守卫直接生成 prelude `Bool` 的 `match`。编译器只需处理**名字**（确定性字段名
  保证幂等、撞构造子名则换新鲜名），完全不碰索引。
- **代价**：多一趟 canonical 化（幂等，不重复改名）；嵌套/守卫下子目标类型
  退回常量（保守，sound）。
- 参考：`crates/front/src/compile/elab.rs`（`compile_pattern_body`/`guard_chain`）、
  `docs/design/match-patterns.md` §4/§10。

## 视图 provider 必须在任何 await 之前注册（2026-09-15，0.49.0 Infoview）

- **教训**：`activate()` 先 `await resolveServerForStart()`（读盘、可能下载）再
  `registerWebviewViewProvider`，这段窗口里 webview 视图**没有 provider** →
  用户看到空白/"点几次侧栏才突然出现"。视图贡献是声明式的，但**内容**要靠注册。
- **规矩**：视图/树/provider 一律在 `activate` **最前面同步注册**；慢工作（解析、
  下载、启动服务）放到之后异步续段，并用 `status` 消息把进度反馈给面板。
- 另：扩展容器 `hideIfEmpty: true`，视图带 `when` 会在条件不满足时把**整个容器**
  隐藏 → 面板"弹不出来"。要么别加 `when`，要么保证容器非空。
- 参考：`editor/vscode/extension.js::activate`、`docs/design/webview-infoview.md` §11。

## 高亮收拢：一个分类源，两张颜色表（2026-09-15，0.49.0）

- **教训**：hover（markdown 围栏）只能 TextMate 着色，Infoview（webview）用 CSS 类；
  若两处各自定义 scope/class，必然漂移（用户："样式不一样"）。
- **做法**：`front::semantic` 导出 `SemanticKind::{ALL, as_str, tm_scope}` 与
  `runs_to_text`/`goal_runs` —— **分类与文本一个出处**；两张颜色表都从 `ALL` 派生，
  配**穷尽测试**（TM 语法必须含每个 `tm_scope()`、CSS 必须含每个 `.tok-<kind>`）。
- **接受**：markdown 只能 scope 级近似（无名字解析），颜色永不 100% 相同；
  详见 `docs/design/highlighting.md`。

## 绝不 `cargo fmt --all`（冻结内核会被重排）（2026-09-16，0.49.0 收尾）

- **教训**：收尾时手滑跑了 `cargo fmt --all`，它按仓库 `rustfmt.toml` 重排了
  `crates/kernel/**`（20+ 文件）。虽是纯格式、语义中立，但**违反「kernel 冻结快照」硬规则**，
  并污染提交历史（发现于提交后、推送前，已还原重做）。
- **规矩**：只 fmt 教学 crates（`-p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp`）
  或直接 `sokonanoda gate`（它的 fmt 步骤本就只覆盖这三个 crate）。
- **自检**：提交前 `git status --short | grep kernel` 必须为空。

## 平台没给的颜色别硬造：先问"代价"（2026-09-16，0.50.0）

- **背景**：Infoview（webview）拿不到主题 token 色，用户要求"和主题对齐"。
- **尝试**：实现了完整的主题解析器（读活动主题 JSON/内置主题、展开 `include`、
  `tokenColors`+`semanticTokenColors`+`editor.tokenColorCustomizations`、TextMate
  特异性匹配、`colors` 消息、18 项单测）。功能可用，但**复刻 VS Code 主题解析**
  的边角（`.tmTheme`、选择器、主题定向覆盖、HC）脆弱且昂贵。
- **结论（用户拍板）**：**不解析**，Infoview 用**自研固定调色板**（按 off/light/HC
  各一套），保证"必有着色"即可；并把决策与不可对齐的边界写进设计文档。
- **教训**：平台未暴露的能力，先评估"复刻成本 vs 收益"再动手；能用固定方案替代时，
  优先固定方案 + 明确边界说明。

## 换行当分隔符 = 表达式必须"知道在哪停"（2026-09-16，0.51.0）

- **教训**：给 `by` 块加"换行也能分隔 tactic"看似只改分隔符，实际难点在**表达式贪婪**：
  `exact f` 换行 `apply g` 会被读成应用 `f apply g`（换行只是空白）。
- **做法**：在 tactic 上下文里，若下一 token 在**更晚的行**且是 **tactic 关键字**，则当前
  表达式结束（`starts_atom` 返回 false + `parse_by_block` 据此继续）。不引入缩进敏感。
- **代价/边界**：续行以 tactic 关键字开头的多行项会被切开；同行不写 `;` 不分隔。写进设计
  文档 §11 与 `CHANGELOG`。
- **普适**：任何"用换行/缩进做分隔"的语法，都要先定清"什么情况下换行**不**结束当前项"。

## `gate` 的 anchor 用的是"运行中二进制"的内嵌编译器（2026-09-16，0.51.0 发现的坑）

- **现象**：`sokonanoda gate` 报 playground `unknown identifier 'intro'`（新语法解析失败），
  但 `cargo run -q -p sokonanoda-cli --bin sokonanoda -- playground.sokonanoda` exit 0。
- **根因**：PATH 上的 `sokonanoda` 是**下载缓存里的旧版**（v0.27.0），而 `gate` 的 anchor
  在**进程内**用该二进制的内嵌编译器检查 playground —— 旧解析器当然不认新语法。
  cargo 三步（fmt/clippy/test）用的是本地源码，所以"测试全绿但 anchor 失败"。
- **修复**：`gate` 启动时比对自身 `CARGO_PKG_VERSION` 与仓库 `Cargo.toml`
  `[workspace.package] version`，不一致就**拒绝运行**（exit 3）并提示
  `sokonanoda update` 或直接 `cargo run …`。
- **贡献者自检**：`sokonanoda version`（bin）与仓库版本一致，再信 `gate` 的 anchor；
  否则用 `cargo run` 那条。

## 用户可见改动 = VS Code + skills 同一轮一起改（2026-09-16，用户要求）

- **教训**：0.40–0.51 连续 12 个版本做了大量用户可见改动（Infoview 落位/反馈/色板、
  `match` 全形态、`Vec`、`build`、换行 tactic、gate 守卫…），`skills/` 三个技能却
  没有同步——agent 读到的操作手册落后于产品，正是"门面漂移"的老毛病在 agent 侧的翻版。
- **规矩**：任何用户可见改动（命令/键位/视图/反馈/语法/协议/发布形态）**同一轮**更新
  `editor/vscode/`（README/CHANGELOG/package.json）**与** `skills/` 三个技能 +
  `AGENTS.md` + `docs/vscode-dev-guide.md`；skills 是符号链接到仓库，改仓库即同步。
- **code agent 适配是一等公民**：计划先问「agent 怎么用/怎么验证」——`--json` 输出、
  能力写进 skill、命令可直接执行、`HANDOVER` 同步。
- **skill 写法**：**确切可执行的一条命令** > "建议/可以考虑…"式散文；少 token。

## STATUS 归档脚本：断言必须放在写文件之前（2026-09-16，第二次踩）

- **教训**：轮次归档的脚本先 `status.write_text(去掉最旧轮)`，再 `assert` 归档文件的
  锚点——锚点文案一错（本轮是归档里轮 78 的日期是 09-15，我写成 09-16），脚本在写完之后
  才失败 → **最旧那轮从 STATUS 消失且从未进归档**（第一次坑掉轮 69，这次坑掉轮 79）。
- **规矩**：先 `assert` 所有锚点（STATUS 与 ARCHIVE 两处）再动 `write_text`；
  或者用 `git show HEAD:STATUS.md` 兜底恢复。
- **自检**：改完立刻 `grep -c '^## 本轮进度' STATUS.md`（应为 3）并确认归档首条 = 被移出的轮次。

## STATUS 插入轮次要按"轮号排序"，不要"插在上一轮锚点前"（2026-09-16，第三次踩）

- **教训**：三轮都用"把新轮插在上一轮 heading 之前"的脚本——当上一轮本身是**早先插到
  更低位置**的（轮 82 在 81 之前、轮 83 又插到 81 前），结果顺序变成 82,83,81；
  轮号不再单调，读者/网站"最新一轮"取值会错。同类第 3 次（轮 69/79 丢失、这次错序）。
- **规矩**：归档/插入一律**解析出所有轮次 → 按轮号（中文数字转数值）降序 `sort` →
  取前 3 留 STATUS、其余并入 ARCHIVE**；脚本里先 `assert` 无重复轮号、再写文件。
- **自检**：`grep -n '^## 本轮进度' STATUS.md` 的轮号必须**严格递减**，且与快照行一致。

## 本地 `gate` 报 cargo 101：先怀疑 perf 哨兵，别改内核（2026-09-16，0.54.0 收尾）

- **教训**：`sokonanoda gate` 报 `cargo exit exit status: 101`，我按"clippy 失败"去查
  （kernel 一堆 warning 干扰视线），实际是**满载时 `incremental_edit_anywhere_is_fast`
  这类 perf 哨兵误报**；机器空转重跑即 PASS。且我先前用 `grep -E "^\+ cargo|error"`
  过滤门禁输出，**掩膜了真实失败步骤**（同 CI 的"禁止 grep 掩膜退出码"规矩）。
- **规矩**：门禁红先逐条单跑并看**真实退出码**——
  `cargo fmt -p … --check; echo $?` / `cargo clippy --workspace --all-targets; echo $?` /
  `cargo test --workspace --locked; echo $?`（各自 `$?`），空载重跑 `sokonanoda gate` 复核；
  只有单跑仍红且指向教学 crate 才动手（内核只读）。

## DSH 的 `edit` 被拒（`file changed since it was read`）＝自己刚改过它（2026-09-17）

- **现象**：DeepSeek Harness 里 `edit` 抛
  `cannot edit "…": file changed since it was read`（错误码 `FS_STALE_VERSION`），
  用户会以为工具坏了。
- **机制**（不是 bug）：`read` 会记下该文件的版本，`edit`/`write` 必须基于同一版本
  （`deepseek-harness/packages/fs/fs-local/src/index.ts:193,243`）。文件在读完与写入
  之间被任何东西改动，守卫就拒绝——防止按过期内容覆盖别人的修改。
- **本次真实原因**：我在 `edit` 之前刚跑过 **`cargo fmt -p sokonanoda-cli`**，
  rustfmt 重排了同一个文件（`const DSH_REJECTED_KEYS` 折行、`assert!` 换行）；
  于是"读→fmt→edit"必然触发。守卫还顺带让我看见了自己写错的多余右括号。
- **规矩**：① 报错就**重新 `read` 再 `edit`**（同一轮内立刻做，中间别再跑任何会
  写文件的命令）；② 要连续编辑同一个文件，就**先把编辑做完，最后统一跑 `fmt`**；
  ③ `fmt`/生成脚本/任何写文件的命令之后，之前读过的文件都视为"已过期"。


## 抽层/去重时：**删除旧实现前，先做"全输入对拍"**（2026-09-17，H6-A 真相层）

- **背景**：把 LSP 的 `soko/*` 语义平移进 `front::query` 再删旧实现（"单一真相"）。
  平移后 LSP 套件 **117/117 全绿**，但真出了**语义漂移**：无 `by` 块的声明被
  当成"根状态"，已证声明凭空多一个目标、半成品证明丢掉上下文。
- **为什么测试没红**：LSP 唯一覆盖该分支的用例，其画布**没有 lambda 前缀**，
  于是"剩余目标"与"声明类型"取值恰好相同——**新语义没有被测试**，
  只是"新测试通过"。
- **抓出它的方法（值得复用）**：删旧实现之前，写一个**临时探针**，对同一份输入
  **穷举每一个输入位置**（这里是 5 个画布 × 每个光标 offset 0..=len，共 709 次）
  逐字段比较新旧两份实现，再把不一致**按分支分组计数**（本次 424 处全落在同一分支，
  定位只需一眼）。
- **规矩**：① 语义函数搬家/去重，**先对拍再删除**，不要只靠既有测试套件"全绿"；
  ② 一致性契约测试必须覆盖**判别性输入**（能让两条候选语义取不同值的输入），
  否则等于没测；③ 适配器一致性测试要跑**真实构件**（这里是
  `target/<profile>/sokonanoda-lsp`）——好处是陈旧构件会当场暴露，代价是改了 front
  后只跑单个 crate 的测试会拿旧二进制对拍（先 `cargo build --workspace`）。
- **守护位置**：`crates/front/src/query/tests.rs`（no-`by` 两条红先单测）、
  `crates/cli/tests/query.rs::query_state_and_lsp_agree_without_a_by_block`
  （端到端跑真实 LSP）、`docs/design/agent-query-channel.md` §4 as-built 3 / §12。

## 镜像内核谓词：逐字翻译 + 给"判别性输入"写测试（2026-09-17，H6-C 的 `is_k`）

- **背景**：front 要替内核算一些**元数据**（`is_k`、`is_recursive`、`num_fields`…），
  内核会断言两边一致（`assert_eq!(rd.is_k, st.k_target)`），不一致就**整个块被拒**。
  这类谓词是"镜像"，不是"独立设计"。
- **踩的坑**：把 `init_k_target`（`kernel/src/inductive.rs:1268-1276`）近似成
  "`Prop` + 单构造子 + 无索引 + 字段数 == 参数数"。内核的真判据是
  `pi_telescope_size(ctor.ty) == local_params.len()`，而 ctor 的内核类型是
  `forall (params ++ fields), result` → 真判据 ⟺ **构造子没有自己的字段**。
  两个能让两个版本取不同值的形状立刻把它照出来：
  ① `Both (A B : Prop)` + `mk (a : A) (b : B)`（字段数恰好等于参数数）；
  ② `Q : Nat -> Prop` + `q : Q 0`（有索引但无字段，内核要 `is_k: true`）。
  两条都在**内核那一侧**被拒，front 单测当时全绿。
- **规矩**：① 镜像谓词要**逐字**翻译内核那几行（把内核的算式抄下来，别用自己的
  直觉改写）；② 对它写测试时先问"**哪个输入会让我的版本和内核对不上**"，那种输入
  才是判别性的（本次是"字段数 == 参数数"和"无字段的索引族"）；③ 这类 bug 常常
  front 单测抓不到，**CLI e2e 层（真跑一遍内核）是必需的**——H6-C 就是被
  `cli_inductive_accepts_multi_name_binder_groups` 抓住的。
- **守护位置**：`crates/front/src/compile/elab.rs::is_k_target`（注释里写了两个反例）、
  `crates/front/src/compile/tests.rs` 的两条 `*_k_target` 单测、
  `crates/cli/tests/cli.rs::cli_inductive_accepts_multi_name_binder_groups`。

## 改验收标准之前，先把被验收的东西**量一遍**（2026-09-17，LSP ≤1200 行）

- **踩的坑**：H6-A 的验收写着"`crates/lsp/src/lib.rs` 降到 ≤1200 行"。删完重复实现后
  文件是 3988 行，我就**凭印象**判定"这个指标是拍脑袋的、剩下的都是协议服务代码"，
  并在设计文档/STATUS/REQUIREMENTS/ROADMAP 四处把口径改成"无重复实现即可，不追行数"。
- **真相**：量一下就知道——`lib.rs` 3988 行里 **2575 行是 `#[cfg(test)] mod tests`、
  63 行是另一个测试模块**，非测试代码只有 ~1350 行。把测试模块移出文件、再顺手把
  wire 类型与 semantic-token 辅助各抽一个模块，**≤1200 随手就达标**。
  我"改正"验收标准的依据（"剩下的都是协议服务代码"）是错的，而错的根源是**没量**。
- **规矩**：① 认为某条验收"不合理/做不到"时，先跑 `wc -l`、`grep -c`、拆一下构成，
  再决定是改标准还是改代码；**凭印象改标准比凭印象写代码更危险**——它会永久留在
  文档里，让后人以为"做不到"；② 一条行数类验收要问清"算不算测试、算不算生成代码"，
  否则争论的不是同一个数；③ 就算确实要改口径，也要在文档里写**实测构成**
  （test/非 test 各多少行），而不是形容词。
- **守护位置**：`docs/design/agent-query-channel.md` §3.2/§11 A5（最终数字）、
  `docs/TESTING.md` 的 LSP 行（测试文件位置）、本条目。

## 测试里别写死"当前版本"（2026-09-18，0.57.0 bump 当天变红）

- **踩的坑**：`crates/front/src/project/manifest.rs::requires_compares_major_minor_only`
  为了测 `requires` 只比 major.minor，写了 `requires = "0.57"` 并注释"0.57 != 0.56
  at this point in history"。它作为 I16 的新测试**在 0.56.1 上全绿**；bump 到
  0.57.0 的同一轮，这条断言自己变成"匹配"，全量测试 447/448。
- **为什么 CI 没提前抓**：这类测试只在"版本恰好跨过断点"的那一刻红——门禁全绿、
  分阶段 commit 也全绿，bump 是**唯一**触发点。它是版本 bump 的隐藏耦合项。
- **规矩**：① 测"版本不匹配"的用例必须**从 `env!("CARGO_PKG_VERSION")` 推出**一个
  必然不同的版本（major 或 minor ±1），不能写死字面量；② 反过来说，测"匹配"的
  用例可以直接用 `env!("CARGO_PKG_VERSION")`——它跟着版本走；③ **bump 版本号时
  把全量 `cargo test --workspace --locked` 当必跑项**，不要只跑改动 crate
  （本次正是靠全量跑才在提交前抓住）。
- **守护位置**：`crates/front/src/project/manifest.rs`（注释里写明为什么从运行版本
  推导）、本条目。

## 大函数"只动位置"的搬移：用**二进制对拍**验收，不要只信测试全绿（2026-09-18，批次 3）

- **背景**：把 `run_pass`（≈1174 行单函数）拆成 `walk.rs`（命令走查，每命令一个
  方法）+ `kernel_phase.rs`（内核阶段 + 报告装配）。三刀都是"只动位置不动语义"，
  每刀后 `cargo test --workspace --locked` 全绿（862 条）。
- **为什么光靠测试不够**：契约测试只覆盖它**断言过**的那些面（事件计数、golden
  锚点、课程输出）；一个 arm 里被漏掉的 `push_error`、少一个 `cmd_hovers` 条目、
  条件写反，可能没有任何测试扫到。
- **做法（值得复用）**：`git worktree add /tmp/base HEAD` 拿改动前的树，构建出
  旧 CLI；对**同一批输入**同时跑新旧两个二进制，stdout 逐字节比对。本次输入 =
  全部 58 个 `.sokonanoda`（`git ls-files '*.sokonanoda'`）+ `--root` /
  `--no-project` / stdin / `query check|goals|holes`，`fail=0`。（改 LSP 时同理，
  对拍对象换成 `target/<profile>/sokonanoda-lsp` 的 JSON-RPC 应答。）
- **两个坑**：① `cargo fmt` 会把搬过去的代码重排（换行/尾逗号），所以**文本级**
  对拍要先归一化空白（`"".join(text.split())`），否则满屏假差异；② **两个 worktree
  别共用 `CARGO_TARGET_DIR`**——cargo 的 fresh 判定按包路径分键、但输出文件名相同，
  后建的那棵树会**静默覆盖**前者的二进制（本次 `cargo build` 报 "Finished in 0.07s"
  却把基线二进制留在 `target/debug/`）。要么各自 target dir，要么 `touch` 源文件
  强制重建后再比。
- **守护位置**：`docs/TESTING.md`「二进制对拍」小节、`docs/HANDOVER.md` §4 结构债、
  本条目。

## 性能数字先问采样口径：同进程并行跑会把单次成本放大 3–4×（2026-09-18，台账复盘）

- **踩的坑**：项目层 perf 套件（`crates/front/tests/perf_project.rs` 等）在**同一个
  测试二进制里并行**跑，6 个重活用例互相抢 CPU/内存带宽/分配器。于是"4×20 项目编译"
  被记成 90–152ms 记了好几轮，而它**真实值 32–38ms**：同一份代码，
  ① 只跑这一个用例 → 32.4ms；② `--test-threads=1` 跑全套 → 33–38ms；
  ③ 默认并行跑全套 → 118–152ms（同一提交连测两次还能差 28%）。
  LSP 侧同款：didOpen/按键并行 64/50ms，串行 12/12ms。
- **为什么危险**：这些数字进了 `docs/perf/ledger.jsonl` 与文档基线表，会被当成
  "当前性能"复述给用户/写进里程碑；夸大 3–4× 会让人去优化不存在的问题，也会让
  真正的回归淹没在噪声里。
- **规矩**：① 性能套件的**采集**一律串行（`--test-threads=1`，
  `scripts/perf-ledger.sh` / `perf-report.sh` 已默认带）；② 用例内部 best-of-N 取
  最小，别用单次采样；③ 换采样口径的那次要写清楚——**跨口径的绝对值不可比**，
  比较前先看是否落在 ±25% 内，超出再复测；④ 想确认"某个数是不是真的"，先用
  "只跑这一个用例"复测（本次正是这样定位的）。
- **守护位置**：`docs/PERF.md`「噪声地板与采样口径」（含三行对照表）、
  `scripts/perf-ledger.sh`（`--test-threads=1`）、`crates/front/tests/perf_project.rs`
  的 `measure_best`、本条目。

## stub 宿主必须忠实真实 API：构造函数不是装饰（2026-09-18，批次 4 项目树）

- **踩的坑**：`test-extension-host.js` 的 `MarkdownString` stub 写成
  `constructor() { this.value = ""; }`——把构造参数**丢掉**了。于是"状态栏 tooltip
  带项目行"的断言永远读到空串：不是扩展没写 tooltip，是测试看不见它写的内容。
  同一次还发现 stub 的 `createStatusBarItem` 不返回实例，测试根本拿不到 item。
- **为什么危险**：stub 的偏差会**伪装成功能缺陷**（我去查了两遍 `updateStatusBar`
  的调用顺序），也会伪装成**通过**——如果断言恰好是"为空则跳过"。
- **规矩**：① stub 的最小实现也要尊重真实签名（构造参数、返回值、`event()` 返回
  disposable）；② 新增断言前先在脑子里过一遍"这个 stub 会不会把我要断言的信息吃掉"；
  ③ 发现 stub 不忠实时**先修 stub 再改产品代码**——反过来会把产品改成迎合 stub。
- **守护位置**：`editor/vscode/test-extension-host.js` 的 `fake vscode` 段、
  `crates/cli/tests/extension.rs::unit_test_script_covers_every_node_layer`（清单）、本条目。
