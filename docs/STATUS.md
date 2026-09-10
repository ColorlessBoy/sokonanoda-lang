# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-10（第二十轮：soko/stateAt 光标处 goal 视图 + 扩展 0.6.0）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `docs/REQUIREMENTS.md`（先读）**；
> 设计 = `docs/design-by-tactics.md`（§6 as-built）/ `docs/design-hover-refactor.md` /
> `docs/design-course-bilingual.md` /
> `docs/design-hover-brackets.md` /
> `docs/design-round14.md`（含本轮 B′ 决议）/
> `docs/design-kernel-taxonomy.md` / `docs/design-course-status.md` /
> `docs/design-hints-suggestions.md` / `docs/design-rename-inlay.md` /
> `docs/design-goal-refine.md` / `docs/design-i8-i9.md` /
> `docs/design-infrastructure.md`；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 差距审计 = `docs/gap-analysis.md`；**经验台账 = `docs/LESSONS.md`**；
> 发布 = `docs/RELEASE.md`；**CI 失败台账 = `docs/CI-FAILURES.md`**；
> agent 入口 = `AGENTS.md` + `skills/`；LSP/VS Code 调研 = `docs/lsp-notes.md` / `docs/vscode-notes.md`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-10，第二十轮：光标处 goal 视图 Phase 2 落地）

> 设计先行：`docs/design-by-tactics.md` §6 修订为 as-built。承接第十九轮
> 「实现留后续轮次」的 Phase 2：front 产出 per-tactic 状态，LSP 按光标
> 选取，VS Code 练习树渲染。

1. **front：`by_steps` 产出（kernel 一行未动）**：`by::ByStep.goal` 改
   `Option<String>`（`None` = 全闭合）；`DeclState.by_steps: Vec<ByStepState>`
   （`{span, goal, binders}`）贯通 Def/Theorem/Example 的 Open 与 Checked
   分流（`lower_by_val` 返回降级值 + 步状态）；I8 session 的 `remap_prefix`
   同步平移 `by_steps` 的 span（注释级编辑零重编译后坐标不漂）。新测试 3：
   partial by 的逐步 goal/上下文/span（含 render 的应用括号形状）、checked
   by 尾步 `goal=None`、session 注释编辑重映射。
2. **LSP：`soko/stateAt`（选择全在服务端）**：请求
   `{textDocument, position}` → `{version, decl?, goal, binders, span, step,
   total}`。选取语义定为 **Lean `goalsAt?`**（比初稿「执行后」更贴合学习者）：
   光标在某 tactic span 内 → 该 tactic 的**执行前**状态（`steps[i-1]`；首条 =
   根状态，goal 用内核渲染的完整声明类型 `ty_text`）；否则取终点 ≤ 光标的
   最后一步执行后状态。无 by 块的声明退回剩余 goal/上下文。5 个协议级测试
   （进入态/末步态/根态/无 by 回退/声明外为空）；wire 词汇表 +1
   （`common/mod.rs`）；`docs/protocol.md` 新小节。
3. **VS Code：练习树「当前光标处」组**（subagent 实现，主会话验证）：
   目标（点击 `revealRange` 跳 tactic）+ 假设 + `by 进度 k/n`；选区变化
   去抖 200ms 请求 `soko/stateAt`，请求序号 + 活动文档守卫丢弃过期响应
   （响应带 version）；诊断刷新后重取。静态契约测试 +2（客户端必须消费
   `soko/stateAt`、必须挂选区监听）；**版本 0.5.2 → 0.6.0**（minor：新学习
   能力），CHANGELOG/README/工作区 Cargo.toml 同步。
4. **顺带修复**：`playground.sokonanoda:7` 的 `???`→`sorry` 迁移残留
   （原文案成了「sorry 也可以写成 sorry」的同义反复）改写为自然说明。
5. **验收**：测试总量 **437 + 8 ignored**（front 229 / lsp 89 / cli 71 /
   kernel 48）全绿；fmt/clippy 干净；playground 锚点
   `decl.checked=20 / exercise.open=9 / 0 诊断`不变；course 汇总
   `32 checked · 25 open · 0 failed` 不变；`node --check` + 扩展静态契约
   套件（7 测试）通过。commit `396adee` 已 push main 且 CI（lint + test）
   全绿；**未打 tag**（0.6.0 的发布留给用户触发）。
6. **opencode 项目配置适配（2026-09-10 用户要求，配置-only）**：
   `opencode.json` 增 `skills.paths: ["./skills"]`（三个 skill 自动加载、
   免软链）、Lean 工具链 bash deny、watcher 忽略 `target/node_modules/
   .vscode-test/learner` 等产物、cargofmt/rustfmt 自动格式化关闭（护 kernel
   冻结快照）；新增 `.opencode/command/{gate,check,round}.md` 与
   `.opencode/agent/teacher.md`（主 agent，画布老师角色）；`skills/README.md`
   与 `AGENTS.md` 同步。**配置改动需重启 opencode 才生效**；不触碰 Rust
   测试面，CI 不受影响。另修 VS Code 报错「Unable to load schema from
   https://opencode.ai/config.json … is untrusted」：工作区新增
   `.vscode/settings.json`，`json.schemaDownload.trustedDomains` 补
   `https://opencode.ai` 与 VS Code 默认域名（修后 schema 校验/补全恢复）。

## 本轮进度（2026-09-09，第十九轮：by-tactic 块 + VSCode goal-state 设计）

> ⚠️ 发布后修复：0.5.1 修「扩展自动下载的 LSP 缓存不校验版本（升级后仍跑旧
> 服务器）→ 按扩展版本号版本追踪；axiom 连接词语义 token → TYPE」。
> 0.5.2 修「`by` 块 span 终点取下一个 token 起点 → 注释被吞进警告范围；
> 尾部 Hole span=offset 0 → 止于 `sorry`」。Lean 确认 `sorry` 术语+tactic 双栖，
> `by sorry` 与 Lean 对齐、与值位 `:= sorry` 无冲突。

> 设计先行：`docs/design-by-tactics.md`。触发：用户要求「实现一些基础 tactic，
> 跟 Lean 4 一样用 `by` 开始」（补 assumption / rfl），并调研设计 VSCode 前端
> 显示 goal state。首期五个 tactic：**intro / exact / apply / assumption / rfl**，另加 `by sorry` 占位（目标保持开放，与值位 sorry 同语义）。

1. **`by` 语法 + 引擎（front 层，kernel 一行未动）**：`theorem t : T := by <tactic>; <tactic>; …`
   （`;` 分隔，教学子集不引入缩进敏感）。新 `Expr::By`/`Tactic` AST、`Semicolon`
   词法、`FolFile.src`（`parse` 存原文，`run_pass` 按声明起点切片当前缀源码）。
   引擎 `crates/front/src/by.rs`：目标树（apply 多子目标）+ 父指针收集上下文；
   逐 tactic 判定复用 `judge_terms`（kernel 唯一裁判），`apply` 用新
   `judge::judge_infer` 推断被应用函数类型 + 位置 spine 合一（codomain 中出现的
   命名 binder = 类型参数、其余 = 子目标）。`by` 没写完整 = 尾部 `sorry` →
   既有 `open_goal` 分流成 Open 练习（「部分作答」同语义）。
2. **内核类型文本可回读**：pp 把 `(a : T) -> (b : T)` 折叠成 `forall (a b : T), …`——
   parser 新增多名字 binder 组 `(a b : T)`（Lean 对齐，仅类型箭头位），
   `proof::render_expr`/`judge::judge_infer` 补 `+` 括号与逐 binder 剥层，
   引擎读回内核类型不再失真。
3. **课程三件套**：`course/` 新增单元⑥「by 写法」（中文 + `en/` 英文镜像 +
   `solutions/` 解答钥匙，全经内核验证）；`playground` 追加 2 道 by 练习题
   （open=7→9）；`course.json`、`course.rs`/`course_status.rs` golden 计数
   更新（unit6 13 checked / 5 open）。
4. **测试**：front 13 新（parse by 块/白名单拒绝未知 tactic/intro+exact/
   assumption/apply+rfl/部分 by→Open/错误 exact→`elab-tactic-failed`/多名字
binder 组/空 by→Open/intro 非函数目标/assumption 无匹配/rfl 非 Eq/apply
    目标不匹配/`by sorry` 占位→Open）+ CLI 2 新（by 端到端、部分 by→open）。
    测试总量 **426**（front 226 / lsp 81 / cli+kernel 119）；fmt/clippy 干净。
5. **VSCode goal-state（Phase 2 设计，协议先行）**：调研 vscode-lean4 Infoview /
   coq-lsp `proof/goals`——共识 = server 端按光标位置从编译期信息树取 tactic
   前后状态。设计：front 产出 `DeclState.by_steps`（每 tactic 执行后 goal+binders）
   + 新请求 `soko/stateAt`（位置感知，返回 version 供丢弃过期）+ VSCode「练习」
   树顶部「当前光标处」goal 组（方案 A，零 webview）。实现留后续轮次，
   `docs/protocol.md` 待落地时补。

## 本轮进度（2026-09-09，第十八轮：hover 重构——良构表达式 + 高亮范围）

> 设计先行：`docs/design-hover-refactor.md`。触发：用户反馈「括号 hover 内容
> 乱七八糟、有些是包含括号的外部表达式」「`(Not a)` 与 `(And.right a (Not a) h)`
> 左右括号内容对不上」，并要求——逐字符评估所有 hover、把正确行为设计成单测、
> 最终**能看到 hover 内容对应的表达式范围（高亮）**。
>
> ⚠️ 本轮与「课程双语化」并发推进；双语 agent 曾 stash 我未完成的
> `lib.rs/tests.rs`（stash@{0}）隔离验证。本轮收尾时已在工作树重建全部
> hover 改动（lib.rs 逐字节一致、front 两个 binder 测试从 stash 还原），
> 丢弃了已过期的 stash，并丢弃其中一条非本轮的 `playground two := 2`
> 实验改动（画布保持 `sorry` 未作答）。

1. **逐字符盘点（16 个 *.sokonanoda 文件，11229 行 dump）**：三类不合理——
   (a) **lambda/Pi 的 binder 名整段溢出**（977 处）：hover `fun (a : Prop) => …`
   的 binder `a` 时最小 span 是整段 lambda，把「表达式 + 整段类型」全吐出来；
   (b) **括号组切片截断**（AST span 不含括号）：`(h : And a (Not a))` 显示
   `And a (Not a : Prop`（缺右括号）；(c) **hover 不返回 range**：`range: None`，
   编辑器无法高亮「这个 hover 在说哪个表达式」。
2. **front binder 行（冷路径）**：`elab.rs` 为每个 Lambda/Forall binder 记一条
   `binder: true` 的声明行（span = 完整标注 `(a : Prop)`，expr = binder 类型，
   scope = push 前）；`HoverType.binder` 字段贯通 `report.rs`/`check.rs`
   （binder 行跳过 infer、text 置空，渲染用源码切片）。hover 到 binder 名 →
   `a : Prop`，不再整段溢出。
3. **LSP 渲染层重构（`render.rs`）**：`HoverResolved{range, content}`；
   `balanced_span` 把截断切片补成良构表达式（跳过 `--` 注释）；`expr_hover`
   统一渲染 + 平衡 span；`bracket_hover` 按「binder 标注组（span==整组）→
   组内最大表达式行」解析，高亮整组（含括号，保证覆盖光标）。
   `hover()` 所有分支（括号/精确/邻近/声明）都返回 `range`。
4. **测试**：front 2 新（lambda binder 行、Pi binder 行）+ LSP 5 新（binder 名
   不溢出、`h` binder 声明、`(h : …)` 括号显示声明不截断、`(a : Prop)` 括号
   显示声明、range 覆盖光标）+ 旧断言对齐（`hover_map_covers_subexpressions`
   允许 binder 行空 text）。测试总量 **410**（front 212 / lsp 81 / cli+kernel 117）
   全绿；fmt/clippy 干净；playground 锚点 `decl.checked=20 / exercise.open=7 /
   0 诊断` 不变。
5. **stash 协调收尾**：并发双语 agent 遗留的 stash@{0}（含我的旧 lib.rs/tests.rs
   与一条 playground 实验）已丢弃——lib.rs 工作树与 stash 逐字节一致、两个
   front 测试已从 stash 还原到工作树、playground 实验改动（`two := 2`）不保留。
   STATUS 原「并发的 hover 重构编译不过」注记已过时，本行为其解决记录。

## 本轮进度（2026-09-09，第十八轮：课程双语化）

> 设计先行：`docs/design-course-bilingual.md`。触发：用户要求 tutorial 等
> 教程文档提供中文与英文两种版本（范围 = `course/` 单元课程为主；形态 =
> 中文/英文各一份独立文件）。

1. **英文镜像（按语义重构）**：新增 `course/en/`（5 单元画布 +
   `solutions/` 解答钥匙）。英文注释是**重新写就的自然教学文案**，按英语
   语感重组句子与段落、不以中文行号/行数为准（用户修订原则：按语义重构、
   不按字节翻译）；**代码与中文逐字节一致**，`soko:hint` 阶梯条数与顺序
   同构（思路 / 目标形态 / 关键件）。中文文件原样不动，作为权威源。
2. **课程清单**：`course.json` 每条目增 `title_en`（英文标题），`file`/`unit`
   与现有 `course.rs` 断言完全兼容。
3. **CI 守卫**：`crates/cli/tests/course.rs` 新增
   `en_mirrors_match_chinese_event_counts`——`course/en/` 与 `course/` 文件
   同名一一对应，两版 `--json` 事件计数（decl.checked / exercise.open /
   expr.reduced / diagnostic）逐项相等；英文钥匙 0 诊断、0 洞。判定走 kernel
   （事件计数），禁文本比对（注释本来就允许不同）。
4. **验证**：EN 5 画布事件计数与中文 golden 表完全一致（如 unit1
   decl.checked=12/exercise.open=5/diagnostics=0），EN 钥匙全 0 诊断 0 洞；
   code 逐字节一致（脚本核对全部 10 个镜像文件）。`en_mirrors…` 守卫在
   隔离外来改动时通过（`cargo test -p sokonanoda-cli --test course` 4 测试
   全绿），我的改动 fmt/clippy 干净。
5. **边界**：不改 `course/` 中文文件、不译 `docs/` 开发者文档与根画布
   `playground.sokonanoda`（留待后续按需扩展）；不做运行时 i18n 机制。
注：仓库里曾有**并发的 hover 重构**（非本轮产物），验证时已被隔离并
    stash；该 hover 重构现已由对方收尾完成（见上方「第十八轮：hover 重构」
    条目），并发期间的 stash 已清理，`sokonanoda-lsp` 恢复编译通过。

## 本轮进度（2026-09-09，第十七轮：括号 hover + 真名还原）

> 设计先行：`docs/design-hover-brackets.md`。触发：用户反馈 VS Code hover
> 内容完全混乱 + `(表达式)` 悬停要求 + `$N` 索引必须还原真名。

1. **根因三连（全部实验证实）**：(a) pp 的 `binder_names` 从空开始 +
   `parse_binders` 对 telescope 域 lift → `name_loose_bvars` 的文本映射
   永不成立（同一变量打印 `$2`/`$3`/`$4`）；(b) `infer_under_binders` 的
   `force_all` 把 `Not a` 展开成 `a -> False`；(c) `hover_type_at` 的
   「起点 ±2」回退在 `)` 上命中右侧邻居（`And.left : forall …` 漏进括号
   悬停），且与 hover() 的 TOLERANCE 回退语义打架。
2. **kernel 显示层（冷路径，architecture §6 记档）**：pp 新增
   `seed_binder_names` + `TypeChecker::with_pp_scoped`——scope 名字预置
   `binder_names`，松散变量在**所有位置**（含 lift 域）精确还原真名
   （代数验证 `S-1-j` 恒成立）；`infer_under_binders` 去 `force_all`
   （`Not a` 保持折叠，与 `#check` 展示一致）。热路径零改动。
3. **LSP 括号组匹配**：`render::bracket_hover_at`（文本扫描配对、跳
   `--` 注释、组内最大行 = 括号包住的表达式；反向扫描扫到光标之前）；
   hover 优先级 = 关键字静默 → **括号** → 精确 → ±2 邻近 → 声明；
   删除错误的邻近回退（goto-def/高亮/补全同步受益）；
   `hover_content` 统一「空 text 或含 `$` → 只显示源码切片」。
4. **测试**：front 2 新（and_not_absurd 全语料零 `$` + 用户两条指定
   样例 + 部分应用真名；and_swap `And.intro b a : b -> a -> And b a`）+
   LSP 5 新（两组括号正反面、`)` 不漏邻居签名回归、`((p))` 四括号透明、
   注释内括号不张冠李戴）+ 旧断言对齐（`(a : Prop)` 的 `(` → 组内最大行）。
5. 测试总量 **402**（front 210 / lsp 76 / cli+kernel 116…）全绿；
   fmt/clippy 干净（kernel warning 级不变）；playground 锚点
   `decl.checked=20 / exercise.open=7 / 0 诊断`。版本 0.4.0 → **0.4.1**
   （patch：改进非新能力；CHANGELOG 已记）。
6. **CI 两连红（v0.4.1 首推）+ 修复**：(a) lint——新代码
   `int_plus_one` 触发 `-D warnings`，本地验证被 grep 掩膜+管道退出码
   双重污染造成假绿（教训入 CI-FAILURES.md，预防=跑与 CI 完全一致的
   命令）；(b) release 的 github-release——`download-artifact` v4 目录
   布局与 upload 路径不符（`vsix/` 路径从未存在；v0.4.0 同因），
   修为 `sokonanoda-vsix/sokonanoda.vsix`。marketplace-publish 本轮
   **成功**（Azure 超时确认为间歇性）。

## 本轮进度（2026-09-07，第十六轮：???→sorry 迁移 + VS Code 集成测试 + hover 纪律）

1. **???→sorry 迁移完成**：lexer 遇 ? 报教学引导错误；全仓清扫 24 文件
   （playground/course/examples/tests/docs）；协议词表不变。
2. **hover 纪律**：显示「表达式 : 类型」+ 声明名显示完整内核签名
   （ty_text）+ 关键字悬停静默 + 松散变量 $N→binder 名字。
3. **VS Code 集成测试**：@vscode/test-electron 4 用例 + CI xvfb。
4. **Lean 4 调研确认**：点分名原子/sorry warning/hover 签名——设计与
   官方对齐。
5. 测试总量 **388 + 4 VS Code 集成测试**。

## 本轮进度（2026-09-07，第十六轮：Lean 4 对齐 + VS Code 集成测试）

1. **sorry → warning 诊断分级**（Lean 4 对齐）：含 sorry 的开放练习产出
   WARNING 级诊断（code `sorry`，消息 `declaration 'X' uses 'sorry'`），
   与 kernel-rejected 的 ERROR 分离——黄色波浪线表示"编译但有缺口"，
   不再与真错误混淆。LSP 2 个新测试（warning 存在 / 非 sorry 不稀释）。
2. **VS Code 集成测试框架**（subagent 搭建）：@vscode/test-electron 4 用例
   （扩展激活/干净 0 诊断/kernel-rejected/sorry hover）；CI Linux 加
   xvfb-run；.vscode-test.mjs 配置 trust 跳过与 60s timeout。
3. **Lean 4 调研确认**（subagent）：点分名原子（我们的设计与官方一致）、
   sorry severity=warning（一致）、hover 三段式（签名+docstring+import，
   可借鉴）、错误优先原则（可借鉴：同声明已有 error 时不发 sorry warning）。
4. 测试总量 **390**（+2 sorry 测试）；全绿；clippy/fmt 干净。

## 本轮进度（2026-09-07，第十五轮：开发清单清零，2 subagent 并行）

1. **spine meta 方案 B′（S1）**：`field_type_text` 升级为深度 AST 替换
   （`substitute_names`：模板 binder 名 → goal 实参 AST 全量替换，innermost
   wins 遮蔽守卫，命中节点 span 回填）——复合字段类型（`And a b`）现在正确
   实例化为学生上下文（`And True False`），不再原样渲染模板名；裸 Ident
   路径与失败语义不变（旧断言零改动）。kernel 渲染版（方案 A）留作远期。
2. **失败声明建议升级（S2）**：三条建议梯子——
   - **kernel 验证 rfl 替换**（新 `judge_value_replace`：值位整体替换合成
     声明交完整 kernel 裁决；Eq 形状声明验证通过才呈现，`verified: true`）；
   - **Reset**（保留已写 lambda 前缀、只重置主体为 `???`——学生类型标注
     工作保留，剩余目标由 goal 视图接管；保守形态识别：仅括号/花括号
     binder 的 `fun x =>` 链）；
   - **Restart**（整值骨架，既有）。
   首条 `is_preferred`；lib.rs 锚点全绿。
3. 测试总量 **380**；全绿；fmt/clippy 干净；playground（12 open / 0 诊断）与
   course（19/20/0）锚点不变。
4. **至此 gap-analysis Top 10 + 附加小项 + 分类学余项全部清零**；剩余仅
   运营项（release 首跑需打 tag、教学回环需真实学习者）与远期设计项
   （spine meta 方案 A）。

## 本轮进度（2026-09-07，第十四轮：hole_id + auto-derivation，2 subagent 并行）

> 设计先行：`docs/design-round14.md`（含 spine meta 的 A/B/C 方案取舍——
> 推荐方案 B 为下一轮实施项）。

1. **稳定 hole_id（P）**：`soko/goals` 的 `holes` 变 `[{range, id}]`，
   id = `<声明名>:<洞序号>`（匿名 example 用 `example@<行>`，与
   `render::decl_name` 一致）——同一版本内稳定、声明名不变时跨版本稳定，
   外部工具可引用；`soko/nextHole` 保持裸 Range；VS Code 点击统一走
   `holes[0].range`（P 核实客户端历史上只消费 `decl.hole`，无行为变化）；
   契约测试双向钉死（服务端出 id、客户端不当裸 Range 用）。
2. **归纳块 recursor 自动派生（Q）**：无显式 `rec` 的 `inductive` 块自动
   合成 recursor + iota 规则（与 py-nat 手写版同构、内核 def_eq 比对通过）：
   - 递归块（Nat 无 rec + add 闭环）、非递归块（Unit）、多构造子
     （Bool：`not tt ⇒ ff`）全部工作；
   - Prop 块退化为无宇宙参数的小消除 recursor；
   - **字段望远镜契约修正**：`num_fields`/`ctor_telescope_size_wo_params`
     改按完整 Pi 望远镜计（result 箭头链的 domain 也是字段——内核
     `check_declared_metadata` 的要求；py-nat 等既有块数值不变）；
   - 第十三轮的 `elab-missing-inductive-rec` 守卫/变体/文档条目移除
     （被本功能取代）；显式 rec 优先，py-nat/课程块零变化。
   - 教学定位：rec 块仍是单元⑤正课，auto-derivation 是其后的便利层。
3. 测试总量 **360**（front 183 / lsp 61 / cli 65 / kernel 45…）；全绿；
   fmt/clippy 干净；playground（0 诊断）/course（19/20/0）锚点不变。

## 本轮进度（2026-09-07，第十三轮：内核分类学收尾 + 基建，4 subagent 并行）

> 设计先行：`docs/design-kernel-taxonomy.md`。K（内核冷路径分诊）/
> L1（失败声明建议）/ M（criterion 基准）/ N（fuzz harness）并行，主会话
> 合并期修复 K 发现的功能性 bug（非递归归纳块）。

1. **内核错误分类学余项（K，冷路径三层回归）**：全内核 `assert_eq!` 清点
   分诊——3 处学习者可触发站点改稳定消息（`is_prop_type` 带 `got:` 渲染、
   iota 规则顺序/数量两处裸断言）；新错误家族 `kernel-rec-rule-mismatch`
   （`ErrorKind` + code + hint + protocol.md + 穷尽清单）；8+ 处 front 已
   拦截的 backstop 与真内部不变量保留 assert（internal）。热循环零改动。
2. **非递归归纳块修复（主会话，K 发现的功能 bug）**：内核按构造子 telescope
   自算 `is_recursive` 并断言一致——front 恒传 `true` 导致
   `inductive Unit/Bool` 崩溃。修复：`elab.rs` 从源码 AST 同规则镜像（含
   result 箭头链的 domain）；缺 `rec` 的块在**入环境前**报干净教学错误
   `elab-missing-inductive-rec`（check-then-add 保持；rec 块本就是白名单
   内容，auto-derivation 留作课程轮设计）。测试三层（kernel 语义 +
   front 2 + CLI 2）。
3. **失败声明建议（L1）**：`SuggestionKind::Restart`——kernel-rejected
   声明按自身类型形状生成重启骨架 `fun (x : A) => ???`（tokenize 定位
   值位，≤3 层剥 Pi，binder 防撞改名），LSP code action 整体替换值位；
   骨架落回后内核重查回到 Open（测试验证闭环）。
4. **criterion 基准（M）**：`crates/front/benches/pipeline.rs`（黑盒公开
   API）：native_bigint_reduce ~41ms / iota_deep_reduce ~14ms /
   session_suffix_recheck ~500µs；语料校验 `OnceLock` 先行。本地跑：
   `cargo bench -p sokonanoda-front --bench pipeline`（多 target 需带
   `--bench pipeline` 选择器）。
5. **fuzz harness（N）**：`fuzz/`（独立 crate，脱离 workspace，cargo-fuzz
   标准布局）——`parse_never_panics`：parse/semantic_tokens/prelude 指令/
   完整 check_document 永不 panic；`cd fuzz && cargo check`（stable）过；
   CI 不跑，用法见 `fuzz/README.md`。
6. 测试总量 **356**；全绿；fmt/clippy 干净；playground（0 诊断）与
   course（19/20/0）锚点不变。

## 本轮进度（2026-09-07，第十二轮：课程地图 + 小项，4 subagent 并行）

1. **`sokonanoda course <course.json>`（课程地图，gap #8 后端）**：聚合
   course.json 全部单元的 `decl.checked / exercise.open / failed /
   expr.reduced` 计数，JSON 视图 = 封闭新事件 `course.unit`（坏单元带
   `error` 字段）+ `course.summary`（进 protocol.md + 词汇表 + 4 个 e2e）；
   人类视图逐单元一行；**进度不是错误**（open/failed 也 exit 0）。
2. **VS Code「课程」树（gap #8 前端）**：`sokonanoda.courseMap` 视图 +
   `sokonanoda.courseRefresh` 命令——客户端跑 CLI 子进程解析 JSON Lines
   （10s 超时、并发去重、找不到 course.json 静默空树）；节点按
   open/failed 着色、点击打开单元文件；**聚合归 CLI，服务器保持单文档**
   （负断言：客户端不得引用 soko/courseStatus）。
3. **REPL 命令历史持久化（小项）**：`$HOME/.sokonanoda_history`（截尾
   1000 行；HOME 缺失静默禁用；不做行编辑——超范围另立项）；测试注入
   临时 HOME，既有 repl 测试不再污染真实家目录。
4. **course/ 五单元提示阶梯内容**：20 个 open 练习 × 3 条（共 60 条
   `-- soko:hint`：思路→目标形态→关键件，答案不进提示）；golden 逐单元
   计数不变、solutions 零诊断（注释级改动不产事件——playground 同机制）。
5. 测试总量 **335**（front 171 / lsp 56 / cli 63 / kernel 45…）；全绿；
   fmt/clippy 干净。playground 锚点不变（checked=14 / open=12 / 0 诊断）；
   course 锚点 19 checked · 20 open · 0 failed。

## 本轮进度（2026-09-07，第十一轮：教学辅助四件套，3 subagent 并行 + 2 调研）

> 设计先行：`docs/design-hints-suggestions.md`（提示阶梯/下一步建议）与
> `docs/design-rename-inlay.md`（rename/references/inlay/lsp 子命令）；主会话
> 预接线（协议、能力注册、桩、front 种子）后 4 个实现 subagent 文件集互斥并行。

1. **提示阶梯 `soko/hints`**：画布指令 `-- soko:hint <text>`（独占一行、挂到
   下一条声明，机制 `front::compile::hints` + `DeclState.hints`；注释级编辑走
   Session 零重编译路径并刷新阶梯）；LSP 自定义请求 `soko/hints`（无状态，
   揭示进度归客户端）；VS Code 练习树「提示」节点 + `sokonanoda.revealHint`
   逐条揭示（不预告剩余条数——WPI 实证）；playground 12 题全部挂上
   思路→目标形态→关键件三级阶梯（**答案绝不进提示**，遵守 teaching-session 规则）。
2. **下一步建议（按目标形状）**：`front::suggest`（每请求 ≤3 条、首条
   `is_preferred`）——exact（kernel 判定）、`Eq.refl` rfl 候选（kernel 验证后
   才呈现）、refine（模板）、intro（形状）；`front::judge::judge_hole_fill`
   把洞替换候选后整份交 kernel 终审。**顺带修复多洞错位 bug**：spine 状态下
   「匹配外层 goal 的假设」不再被塞进子洞（逐洞按 `sub_goals[i].ty` 判定）。
3. **rename + find-references**：全语义集（`resolve_at` + `references_for` +
   tokenize 精确名字 token，零文本扫描；注释/字符串天然不误伤）；prepareRename
   返回名字子 span + placeholder；rename 产出**版本化 documentChanges**，
   非法名/不可解析 → ResponseError（不返回空 edit，LSP 3.17 规范）；shadowing
   内层胜出有回归测试。
4. **inlay hints**：每个开放练习的洞尾标注期望类型（`: T`，子洞类型来自
   server 端 walk；单主洞显示剩余目标）+ markdown tooltip（目标 + 假设）；
   只读信息，无 textEdits。
5. **`sokonanoda lsp` 子命令**（单二进制分发，gleam 模式）：`crates/lsp` lib 化
   （`sokonanoda_lsp::run()`），`sokonanoda` 二进制 `lsp` 子命令拉起 stdio 服务器
   （tty 时 stderr 提示）；`sokonanoda-lsp` 二进制保留，VS Code 端不受影响。
6. 测试总量 **327**（front 171 / lsp 56 / cli 55 / kernel 45…）；全绿；
   fmt/clippy 干净（教学 crates 零警告）。playground 锚点不变：
   checked=14 / open=12 / diagnostics=0。

## 本轮进度（2026-09-07，第十轮：gap-analysis 第一批落地）

1. **行业基线 LSP 三件**（主会话）：completions（关键字/宇宙/prelude 名/
   文档声明，单源 `front::semantic::keywords()`；内部名不外泄）、folding
   range（仅多行声明）、`--version`（cli）+ workspace `rust-version = 1.96`
   （MSRV 声明，rust-analyzer 教训）。
2. **导航基线（subagent A，断网后核实其工作已完整落盘）**：go-to-definition
   （elab 记录 use→def 解析映射：局部 binder→binder span、顶层名→声明
   span；shadowing 正确——内层 `x` 解析到内层 binder）、document highlight
   （同定义全部使用点）、binder 补全（光标处 name_scopes 在域名字）。
   新 API：`ResolvedTarget`、`DocumentReport.definitions/name_scopes`、
   `HoverType.scope_names`。
3. **REPL undo（subagent B）**：`ProofState` 快照栈 + `undo`（`u`/`#undo`）；
   intro/exact 成功前入栈、失败不动；cli e2e + 4 单测。lean4game/Isabelle
   的教学基线能力。
4. 测试总量 **273**（front 141 / lsp 33 / cli 35 / kernel 45…）；全绿。

## 本轮进度（2026-09-07，第九轮：subagent 并行 ×3，主会话多洞/refine）

1. **多洞 + refine（I9 第二段，设计 `docs/design-goal-refine.md`）**：
   构造子 spine 走查——`And.intro ??? ???` 等多洞是合法 Open 状态（不再
   hole-misplaced）；子洞期望类型从文档自身的 axiom/ctor 形状实例化
   （参数位=目标自己的实参，证明位=实例化后的字段类型）；
   `DeclState.holes/sub_goals/refine_template` 贯通 LSP——**refine 建议**
   （`And.intro a b ??? ???`：参数自动填充、证明字段留洞，结构来自文档、
   kernel 终审）、`soko/goals` 携带 holes/sub_goals、nextHole 跨子洞环绕。
   front 4 + LSP 3 个新测试。
2. **内核错误分类学（审计 subagent 报告 → 实现 subagent 落地）**：8 个新
   kernel 错误码（`kernel-expected-sort` / `expected-pi` / `theorem-not-prop`
   / `inductive-non-positive` / `ctor-result-mismatch` / `ctor-arg-invalid-app`
   / `ctor-arg-not-type` / `ctor-arg-too-large`），各带中文教学提示；内核
   冷路径 5 处消息增强（两处无消息 assert 加消息、三处 `got:` 渲染）；
   分类器 `refine_kernel_kind`（含 `rejected:` 前缀剥离与 internal 兜底）。
3. **关键稳定性修复**：`#check`/`#reduce` 直通内核求值路径此前无 panic
   保护——`#check (Type) 3` 会**崩掉整个编译/LSP 进程**；现在经
   `quiet_catch` 降级为分类诊断（并接通分类器）。VSIX 真因修复：
   `.vscodeignore` 排除了 node_modules（上轮只移了 dependencies）——实测
   VSIX 从 9 文件/13KB 变为 324 文件/470KB 且含 vscode-languageclient；
   契约测试封死两处回归。
4. **发布流水线（subagent 实现）**：`release.yml`（tag 触发 + dispatch
   dry-run；Rust 双二进制 + VSIX 同 Release）+ `docs/RELEASE.md` 发布手册。
5. **业内标准差距审计（调研 subagent）**：`docs/gap-analysis.md`——Top 10
   补全清单（completions/go-to-def/folding/提示分级/undo/rename/inlay/
   章节地图/下一步建议/--version+MSRV）与反标配清单；已并入 ROADMAP L2/L3。
6. 测试总量 **245**（front 133 / lsp 26 / cli 35 / kernel 45）；全绿；
   clippy/fmt 干净。

## 本轮进度（2026-09-07，第八轮：goal 面板与跳洞）

1. **VS Code goal 面板（I9 收尾，消费 `soko/goals`）**：资源管理器新增
   "练习" 树——每个声明显示 kind·状态，开放练习展开为「目标 + 已引入
   假设」，点击直达洞位；状态栏显示未完成练习数（点击聚焦面板）；诊断
   更新即自动刷新。**`alt+n` / `alt+shift+n` 跳下一个/上一个洞**（环绕；
   位置计算全部在 server 端 `soko/nextHole`——客户端禁止文本扫洞，
   ocaml-lsp 教训落入代码约束）。
2. **客户端契约测试**（`crates/cli/tests/extension.rs`，4 个）：package.json
   声明的命令必须在 extension.js 注册、键位只指向已声明命令、客户端必须
   消费 soko/goals+nextHole 且禁止自算洞位、运行时依赖必须在 dependencies
   （VSIX P0 回归守护）、打包元数据齐全。无需 Electron 即可 CI 守护客户端。
3. **AGENTS.md**（项目指令入口）：opencode/Claude Code 等原生读取——接手
   阅读顺序、角色技能（skills/）、硬规则速记、命令清单、收尾义务。
4. 测试总量 **239**（+4 扩展契约套件）；clippy/fmt 全绿。

## 本轮进度（2026-09-07，第七轮：逻辑先行课程落地 + I9 宇宙携带）

1. **课程重排（用户课程排序哲学落地，REQUIREMENTS §6）**：course/ 5 单元与
   playground 全部重排为逻辑先行——
   ①命题与证明项（先证明命题，全程不谈 Sort）→ ②等式与 rfl（先认识数字，
   `Eq.{1}` 作为机械规则并埋下"为什么是 1"的悬念）→ ③函数与箭头（结尾埋
   "函数类型的类型？"悬念）→ ④宇宙（Sort 由悬念揭晓，回收 Prop=Sort 0）
   → ⑤归纳与递归（不变）。旧 unit1/2/3/4 文件更名重写，题目与钥匙全部
   复用既有 kernel 验证结论（Eq.symm 钥匙修正了一处缺实参的错误——被
   本仓库 LSP 实时抓出，opencode 接线的第一次实战验证）。
   同步：course.json（新文件名/标题）、course.rs golden（新计数 12,5,1 /
   2,5,2 / 1,4,1 / 0,3,0 / 4,3,1）、course/README、skill 的 curriculum.md、
   teaching-session.md §3（12 题新编号）、playground（12 练习逻辑先行为主：
   checked=14 open=12 diagnostics=0）。
2. **I9 余项——开放声明携带宇宙参数**：`DeclState.universe` 贯通
   （OpenExercise op → 报告 → LSP exact_binder → judge 合成声明），
   带 `{u}` 的开放练习（如毕业题 Eq.symm）现在能获得 exact 建议；
   上一轮记录的已知限制清除。测试：front 2 个（记录 + judge 端到端）。
3. 测试总量 **235**（+2 宇宙携带）；golden 更新为刻意变更；clippy/fmt 全绿。

## 本轮进度（2026-09-07，第六轮：agent skills）

1. **skills/ 目录（用户需求：为 code agent 设计 skill 部分）**：Agent Skill
   格式（SKILL.md frontmatter + references）：
   - `sokonanoda-teacher`：判卷接口（--json 事件读法）、3 步教学循环、
     事件决策表、出题规范（含逻辑先行哲学）、解答钥匙守则、编辑器能力
     清单、硬规则；references/events.md（事件形状 + 增量语义）与
     references/curriculum.md（题池地图 + 适配规则）；
   - `sokonanoda-dev`：接手清单（REQUIREMENTS→STATUS→ROADMAP→architecture）、
     硬规则、TDD 三层 + 文档先行 + subagent 工作流、CI 门禁形态。
   - 安装方式见 `skills/README.md`（软链到 harness 的 skills 目录）。
2. **conformance 守护**：`crates/cli/tests/skill.rs`（4 测试）——frontmatter
   合法且 name=目录名、引用的仓库路径必须存在、事件/方法词汇封闭且必须被
   `docs/protocol.md` 记载（词汇表抽到 `crates/cli/tests/common/mod.rs`，
   protocol.rs 与 skill.rs 共用）；course.json 的单元/钥匙孪生存在性校验。
   protocol.md 补上 watch 流（Session delta）词汇一节。
3. **opencode LSP 接线**：仓库根 `opencode.json` 把 `sokonanoda-lsp` 挂到
   `.sokonanoda` 扩展名（`cargo run` 启动，无预构建要求）——opencode 等
   agent 打开教学文件即自动消费 kernel 判定诊断；README 增设
   "For code agents" 一节。
4. 测试总量 **233**（+4 skill 套件）；clippy/fmt 门禁维持全绿。

## 本轮进度（2026-09-07，第五轮：I8 真增量 + I9 + 内核修复 + 工程达标）

本轮按"先调研后动手"执行（4 个并行 subagent：代码审计 / LSP 增量业界实践 /
goal 视图 UX / VSCode+CI 标准），设计文档 `docs/design-i8-i9.md`，全程 TDD。

1. **I8 真增量（front，零内核改动）**：学 Lean4/coq-lsp 的"前缀精确复用 +
   变化点后保守重算"。`run_pass` 增加 `TrustPlan`：信任前缀照常 elaborate +
   入环境但**跳过内核重查**（内核检查是贵的那一半）；失败声明不入环境
   （check-then-add 语义保持）。`Session` 快照升级为逐命令
   `{state, hovers, events, errors}`，文本不变 → 零重编译且**修复了注释/空白
   编辑导致的 span 漂移 bug**（重映射坐标）；`first_diff` 之后才重查。
   `SessionUpdate.stats.kernel_checks` 让"改第 i 个声明只重查后缀"可验证
   （测试：5 声明改第 4 → kernel_checks == 2）。LSP Backend 切换到 Session
   （此前每次编辑 2×2 遍流水线，现在前缀零内核重查）。prelude 指令变化时
   整体重建（决策依赖整文件内容，语义与全量严格等价）。
2. **I9 tactic 判定 kernel 化（`front::judge`，零 kernel 原语）**：
   合成完整声明 `def _soko_judge_k : forall binders, 剩余目标 := …术语…`
   走标准流水线，kernel 是唯一裁判。LSP `exact` 与 REPL
   `exact/apply/assumption` 全部接入；`proof.rs` 文本比对删除
   （REQUIREMENTS §2.8 清账）。defeq-但-不同文本的假设（`a -> False` vs
   `Not a`）现在能被识别。goal 视图协议：`soko/goals`（结构化多洞 goal 列表）
   与 `soko/nextHole`（server 端位置计算，ocaml-lsp 教训）两个自定义请求。
3. **内核 soundness 修复（上游 bug，本轮最重要发现）**：judge 端到端测试暴露
   conv `unify_direct` 的 Pi/Lam body-expr 快路径把 **eval 闭包与 infer 闭包**
   按体表达式指针判等（同一 `Var 0` 在两种闭包下是 `$0` vs `Sort 1`），
   `(A : Sort 1) -> A` 这种不可居住类型被身份 lambda 通过（官方 Lean 拒绝）。
   修复 = 快路径增加闭包语义守卫（`closure_ctxs_compatible`，热路径仅一个
   判别分支），回归测试三层（kernel 2 + CLI 2 + 全量语料）。
4. **工程达标（业内标准）**：CI 增加 lint job（fmt + clippy）；`actions/cache`
   → `Swatinem/rust-cache@v2`；`--locked`。lint 门禁形态：教学 crates 在各自
   `Cargo.toml` 用 `[lints.rust] warnings = "deny"` 注入严格度，kernel 冻结
   快照不参与（其 `lib.rs` 的 `deny(cast_possible_truncation)` 降为 warn，
   上游代码自身未过该 lint）。fmt 门禁只覆盖教学 crates（kernel rustfmt.toml
   需要 nightly）。VS Code 打包 P0：`vscode-languageclient` 移到 dependencies
   （此前打出的 VSIX 装上即坏）、补 repository/LICENSE/CHANGELOG/.vscodeignore、
   `vsce package` 冒烟通过。
5. **课程哲学修正（用户插话，已记录 REQUIREMENTS §6）**：逻辑先行——先讲
   True/False/And/Or/Iff/Forall/Exists 让学生在"证明命题"里建立直觉，Sort 等
   到"函数类型的类型是什么"这一自然问题出现时再引入；course/ 与 playground
   按此重排（**下一轮任务**）。
6. 测试总量 **229**（kernel 45 / front 121 / cli 40 / lsp 23），
   全绿；`cargo clippy --workspace` exit-0，教学 crates 0 警告。

## 本轮进度（2026-09-07，接手 agent 第 1–3 轮）

1. **模块化重构（用户要求：不得单文件巨石）**：`front/lib.rs`→
   `span/token/ast/diagnostic/parser + compile/{mod,error,event,report,elab,prelude,check}`；
   `cli`→`main/check/json_report/repl/help`；`lsp`→`main/render/actions`；
   公开 API 全部 re-export 保持稳定；**kernel 一行未动**（性能原则）。
2. **全流水线测试资产（157 tests 全绿，见 `docs/TESTING.md` 地图）**：
   kernel 41+2 / arena 1 / memory 1；front 49→**74**（lexer 10 / parser 10 /
   compile 51，含 ErrorKind 矩阵、DocumentReport 状态机、doc-conformance、perf 冒烟）；
   cli 21→**21+8**（新增 protocol golden：封闭事件词表、lesson-01/02 金字、
   协议文档防漂移）；**lsp 0→10**（内存内 LspService 协议级集成测试）。
3. **测试揪出并修复的真实缺陷**：
   - LSP `intro` quick-fix 行列 +1 偏移（actions.rs 1-based→LSP 0-based）；
   - publishDiagnostics 补 `version`；didChange 改取最后一个 change（FULL sync 语义）；
   - `--json` 的 elab/kernel diagnostic 补 `hint` 字段（protocol.md 本就承诺）；
   - `docs/protocol.md` 补齐 6 个缺失 elab 错误码（doc-conformance 测试守护）。
4. **文档**：新增 `docs/REQUIREMENTS.md`（用户全部要求的权威总账）、
   `docs/TESTING.md`（测试资产地图）、`docs/lsp-notes.md`、`docs/vscode-notes.md`。

## 本轮进度（2026-09-07 第四轮：I8 + I9 后半 + 语义高亮 + watch）

1. **语义高亮（F8，用户要求）**：front 新增 `semantic.rs`（keyword/sort/number/
   hole/声明名/构造子/binder 分类，声明点优先）；LSP 实现
   `textDocument/semanticTokens`（UTF-16 编码正确处理增补平面字符——修掉了
   `span.column` 是字符计数的错位隐患）；VS Code 端 vscode-languageclient
   自动注册，零配置生效。
2. **I9 kernel 显式错误**：6 处 def_eq 失败点（def-like/App 实参/let 体/
   inductive 参数/表达式）在 panic 前用 debug printer 渲染两端（≤200 字符截断），
   稳定格式 `def_eq mismatch expected: <E> | actual: <A>`；front 解析为
   「类型不匹配：期望 `E`，实际是 `A`」并填充 `CompileError.expected/actual`；
   热路径零改动，内核 41 测试全绿。
3. **I8a check-then-add（双趟）**：kernel 拒绝的声明不再占用名字——第二趟在
   干净环境中重算，依赖者得到真正的 unknown-identifier 诊断；归纳块首次纳入
   kernel 判定（`EnvBuilder` 新增 `begin/end_inductive_block` +
   `mutual_block_sizes` 记账，对齐上游 parser；`add_inductive` 返回构建的
   Declar）。由此暴露并修正了 py-nat 系 iota 规则的两处非标准写法：
   规则值必须是 `ms n (Rec.{u} motive mz ms n)`（`.{u}` 不能省）。
4. **I8b Session**（front::session）：版本号、delta 事件
   （exercise.opened/solved/failed、decl.checked/failed）、`recompiled_from`
   日志；内容未变（仅注释/空白）零重编译。
5. **I8c watch**：`sokonanoda watch <file>` 常驻监控 → `file.changed` +
   delta + 诊断的 JSON Lines 流（L1 服务层的 CLI 形态）。
6. 测试总量 **203**（kernel 43 / cli 38 / front 103 / lsp 20... 计 CLI 子套件见
   docs/TESTING.md）。

## I6 落地（2026-09-07 第二轮）

全部四件套已实现并通过 178 个测试（kernel 43 / cli 35 / front 87 / lsp 13）：

1. **prelude 可选化（用户要求）**：`CompileOptions{prelude: PreludeMode::{Full,Bare}}`；
   API `compile_fol_with/check_document_with`；CLI `--bare`；文件级注释指令
   `-- sokonanoda:prelude none`（front::prelude_mode_from_source，CLI/LSP 都认）；
   Bare 模式下 `+` 不再产生悬空 Nat.add 常量（改报 elab-unknown-identifier）。
2. **Eq 三件套 prelude**：`Eq`/`Eq.refl`/`Eq.subst` 以 `.sokonanoda` 源语法书写
   （签名与官方 Lean 一致），受信任安装；文件自带 Eq 系列则整体跳过
   （all-or-nothing，与显式 Nat 块一致）。
3. **binder 类型推断**：`elab_expr` 下传 expected（声明类型逐层剥 Pi），
   `fun n => n + 1` 免写 `(n : Nat)`；依赖情形（`forall (α : Sort u), α -> α`）
   因 de Bruijn 对齐天然支持；声明类型耗尽仍报 `elab-untyped-binder`。
4. **partial hole（部分作答）**：`???` 允许出现在 lambda 体尾部；声明保持
   Open 且 `DeclState.goal` = 剥掉已写 binders 后的剩余目标；洞在非尾部位置
   仍报 `elab-hole-misplaced`。LSP intro quick-fix 的「替换 ??? →
   fun (x : T) => ???」循环第一次真正闭环。
5. **开课**：根目录 `playground.sokonanoda`（12 练习初始全 open、0 诊断、
   exit 0）；`docs/teaching-session.md` = 开课手册 + 事件决策表 + 全部解答钥匙
   （12/12 经完整内核验证，含 `two_def` 闭环）。
6. 新增裸名 `#reduce Nat.add/Nat.succ` 边界测试（I6 验收项，防 delta 循环）。

## 本轮进度（2026-09-07 第三轮：I7 课程层 + I9 goal 视图第一段 + VS Code 修复）

1. **I7 课程层**（用户定位：course/ = agent 路线图，执行层由 agent 按用户灵活
   适配——已写入 REQUIREMENTS §6 与 teaching-session §0）：`course/` 5 单元
   画布 + `course.json` 顺序清单 + `solutions/` 解答钥匙（全部经完整内核验证
   可解）+ `crates/cli/tests/course.rs` golden（每单元 decl.checked/exercise.open/
   expr.reduced 计数钉死）+ CI 步骤。
2. **I9 goal 视图第一段**：开放声明现在携带已引入假设清单
   （`DeclState.binders: Vec<GoalBinder{name, ty}>`，goal_under_binders 同步
   记录）；LSP hover 在 `???` 上显示「目标 + 已引入假设」；code action 新增
   **`exact <假设>`**（类型与目标匹配时自动提议，洞替换为该假设名），
   与既有 `intro` 并存；LSP 测试 13→16。
3. **VS Code 薄壳修复**（依 docs/vscode-notes.md）：修复 `client.start()` 未作为
   disposable 注册的真实 bug（改为正确 start/stop 生命周期）；`sokonanoda.serverPath`
   设置 + 自动发现（workspace target/debug|release → PATH）；`alt+s` 状态命令
   （documentSymbol → 快速选择面板）；codeLens 的 `sokonanoda.status` 命令
   补了客户端 handler（此前点击报 command not found）；新增 F5 启动配置。
4. 测试总量 186（kernel 43 / cli 38 / front 89 / lsp 16）。

## 下一步（交接快照，2026-09-07 第十六轮后；依据 = docs/gap-analysis.md）

> gap-analysis Top 10 已全部清零。以下为运营验证与精选改进。

### 运营验证（需要真实使用）
- **release.yml 首跑**：`git tag v0.1.0 && git push --tags` 后核对产物
  （双二进制 + VSIX；taiki-e 多 bin 映射、xvfb-run 集成测试需首次实测）
- **教学回环实战**：逻辑先行画布已就绪——找真实学习者走完 12 题
  （skills/sokonanoda-teacher 循环），回收提示分层与事件决策表的打磨需求
- **opencode LSP 实战**：已确认一次（抓出 Eq.symm 钥匙缺实参），
  后续在教学过程中持续观察

### 精选改进（gap-analysis 余项 + 教学反馈）
- **"错误优先"原则**：同声明已有 error 时抑制 sorry warning
  （Lean AddDecl.lean 的 `!(← MonadLog.hasErrors)` 模式）
- **正向完成信号**：全部练习解出时给绿色装饰（vscode-lean4 双勾✓✓ 模式）
- **completions**：关键字 + 作用域内名字（gap #1，Deduce 实证第一痛点）
- **go-to-definition 增强**：点分名 `And.intro` 整体跳转（已实现），
  可评估 `And` 段跳 `And`（rust-analyzer 段级导航，成本 M）
- **folding range**：声明体折叠（gap #3）

### 远期（L2/L3）
- spine meta 方案 A（kernel 渲染子洞类型）
- VS Code 扩展 marketplace 发布
- L1 service 事件流（watch 已是 CLI 形态）
## 下一批候选（按投入产出比排序）

1. ~~**提示分级 `soko/hints`**~~ ✅（第十一轮）：画布 `-- soko:hint` 指令 +
   `soko/hints` 请求 + VS Code 逐条揭示；playground 12 题已挂阶梯。
   余项：course/ 五个单元的内容阶梯（教学轮补）。
2. ~~**失败洞的"下一步建议"**~~ ✅（第十一轮）：`front::suggest` 按目标形状
   （exact/rfl/refine/intro，kernel 验证优先 + is_preferred）；顺带修复多洞
   错位 bug。余项：失败声明（kernel-rejected）的针对性建议。
3. ~~**`soko/courseStatus` + VS Code 章节地图**~~ ✅（第十二轮）：聚合归
   `sokonanoda course` CLI 子命令（服务器保持单文档），VS Code「课程」树
   消费子进程 JSON Lines；学习者进度=画布自身状态（声明式文件即存储）。
4. ~~**rename + find-references**~~ ✅（第十一轮）：语义集 + 版本化
   documentChanges + ResponseError；shadowing 有回归测试。
5. ~~**inlay hints**~~ ✅（第十一轮）：洞期望类型 + tooltip；只读无 textEdits。
6. **内核错误分类学余项**：~~`conv.rs` 与 `infer.rs` 同名消息区分~~ ✅
   （第十三轮：统一 `got:` 形状、措辞区分站点）；~~`assert_eq!` 灰色地带~~ ✅
   （第十三轮全量清点分诊）；refine 子洞的 kernel 级 expected type
   （elaborator spine meta，M–L）**仍为余项**——需专门设计轮。
   另：~~归纳块 auto-derivation~~ ✅（第十四轮）。~~refine 子洞的 kernel 级
   expected type~~ ✅（第十五轮方案 B′：深度 AST 替换；方案 A 记为远期）。
7. **小项打包**：全部 ✅（lsp 子命令 / REPL 历史 / criterion / fuzz /
   hole_id）。**小项全部清零。**

### 运营/验证类

- **release.yml 首跑验证**：`git tag v0.1.0 && git push --tags` 后核对
  Release 产物（双二进制 + VSIX；见 docs/RELEASE.md §6 风险清单——
  taiki-e 多 bin 映射、softprops、gh release upload 均需首次实测）；
  `npx @vscode/vsce` 建议钉版本。
- **教学回环实战**：逻辑先行画布已就绪（course/ + playground，12 题）——
  找真实学习者走一遍 `skills/sokonanoda-teacher` 循环，回收提示分层与
  事件决策表的打磨需求。
- **opencode.json 实战核查**：LSP 经 opencode 消费的体验（已有一次实战：
  抓出 Eq.symm 钥匙错误）。

### 更远（L2/L3）

VS Code 扩展集成测试（@vscode/test-electron）、发布 marketplace、
L1 service 事件流（watch 已是 CLI 形态）、KernelError 显式化完整推进
（`CheckError::Internal` 目前无人构造）。

## 已确认的决策（用户 2026-09-06）

1. 命名练习：`def name : T` / `theorem name : T`，匿名用 `example`（官方 Lean 的
   `example` 不能带名字；不发明非 Lean 的 `example name : T`）。
2. LSP 框架：用现成 tower-lsp。
3. 范围：I1–I9 全部实现；goal 视图也进第一期。
4. 反馈目标：**足够细致、足够详细**的 LSP（能力清单见 design doc F1–F8）。

## 已完成（按 commit）

| 范围 | 内容 | 位置/commit |
|---|---|---|
| I0 地基 | `--json` 事件、错误 stage/code、语料 CI、examples 语料测试 | `2926347` |
| 文档 | architecture / research / design v1 三件套 | `bf3441b` |
| 设计 v2 | LSP-first、文件无 `#`、练习=带洞声明 | `c1db151` |
| I1 逐声明状态 | `DocumentReport`/`check_document`：open/checked/failed，练习带名字与目标；开放/失败声明不影响后续 | `f23ca71` |
| I2 错误细分 | `ErrorKind` 稳定 code（`elab-*`/`kernel-rejected`）+ 教学 hint（CLI/JSON/LSP 三处） | `f23ca71` |
| I3 类型图 v1 | elaboration 记录每个子表达式 (span, 内核项, binder 作用域)；kernel 新增 `infer_under_binders` → hover 表 | `f23ca71` |
| I4/I5 第一段 | `crates/lsp`（tower-lsp）：diagnostics/hover(类型+目标)/documentSymbol/codeLens/quick-fix `intro`；`editor/vscode` 薄壳 | `f23ca71` |
| 细节 | 事件按源码顺序输出（open 练习排队处理） | `bacb1ee` |

里程碑对照（ROADMAP 第 5 节）：M0 ✅、M1 ✅、M2 大部分（判定细节/提示在 I2 完成；
"期望目标类型 vs 实际" 的 kernel 比对待接）、M3 协议文本+JSON 已实现（service 增量待接）、
M4 课程内容 ❌、M5+（L1 service / L2 完整编辑器 / L3 agent）未开始。

## 测试现状

`cargo test --workspace` 全绿：kernel lib 41（2 ignored：缺 fixture）、arena 1、
memory_api 1、front 49、cli 21、examples 语料 1。LSP 服务器做了手动 JSON-RPC 冒烟
（initialize/didOpen 诊断 0/hover `x: Prop` 与 `???` 目标/documentSymbol/intro
quick-fix/坏声明 `kernel-rejected`）。

## 怎么跑 / 验证

```bash
cd /Users/penglingwei/Documents/lean/sokonanoda/sokonanoda-lang
cargo test --workspace
cargo run -q -p sokonanoda-cli --bin sokonanoda -- examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json examples/lesson-01.sokonanoda
cargo run -q -p sokonanoda-cli --bin sokonanoda repl          # #check/#reduce/#prove 调试
cargo run -q -p sokonanoda-lsp --bin sokonanoda-lsp           # LSP（editor/vscode 使用）
```

## 待办（按依赖排序，全部已入 ROADMAP §10）

- **I6 prelude 对齐 + elaborator 推进**：Bool/Eq/rfl 等受信任基元；核对 Nat.succ/Nat.add
  占位自引用体；binder 类型推断 → `let` → `match`。验收：每个语法点 TDD 三件套。
- **I7 第一门课（M4）**：5 单元（表达式与类型 / 函数与箭头 / 命题与证明项 / 等式与 rfl /
  归纳与 match）× 3–8 练习，`course/` 目录 + golden 事件；CI 全绿。
- **I8 真正增量**：check-then-add（失败的声明不进环境），编辑一行只重查受影响后缀；
  事件带版本。
- **I9 kernel 显式错误 + goal 视图**：panic→`KernelError`（conv 差异给两端项）；
  `#prove` 逻辑入库，LSP 多洞 goal/refine/code action。
- **L2/L3（后续）**：VS Code 扩展打包（语法+进度树+goal 面板）、L1 service 事件流、
  讲课 agent 接入同一文档状态。

## 给接手 agent 的提醒

- 判定永远走 kernel，不做文本比对（`proof.rs::assumption` 的文本比对是草案，待替换）。
- 新增语法 = 课程 + 测试 + 白名单；`???` 只允许在声明值位。
- kernel 拒绝目前仍是 panic→`Result`（`try_check_declar`）；细粒度 kernel 错误是 I9。
- arena 生命周期：`EnvBuilder`/`ExportFile` 挂同一 `stumpalo::Arena`，必须活得比检查会话久。
