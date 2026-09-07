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
- 依赖 binder 折叠**必须折成单个 Forall 望远镜**（嵌套独立 Arrow 的 domain
  会在空作用域里 elaborate 而报 unknown）——`judge::fold_declared`。
- 宇宙多态：裸名默认 u=0，跨宇宙判定必须显式 `id.{u}`（judge 测试踩过）。
- prelude 决策（explicit-Nat 探测 / Eq all-or-nothing / 防遮蔽）**依赖整份
  文件内容**——Session 增量正确性的根基（`docs/design-i8-i9.md` §1）。
- 多洞走查：**值头是构造子名、目标头是族名**（`And.intro` vs `And`），对齐
  要经模板的 `name` 字段校验，不能直接比头（多洞实现踩过）。
- 洞位/子目标永远由 server 端 walk 产出（`DeclState.holes/sub_goals`）；
  客户端禁止文本扫洞——已编码为契约测试（`extension.rs`）。

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

## 教学（course / playground）

- 课程排序哲学：**逻辑先行**（REQUIREMENTS §6）——先证明命题，Sort 等
  "函数类型的类型"问题自然出现再揭晓；`Eq.{1}` 先当机械规则、悬念后置。
- kernel 只判类型不判意图：`double := fun n => n` 也能过——语义要求用
  证明形状表达（`two_def` 闭环模式，teaching-session §2）。
- 解答钥匙必须经完整内核验证（course/solutions + golden）；本次流程自己
  就靠它抓出过 Eq.symm 钥匙缺实参（被自家 LSP 实时抓出）。
