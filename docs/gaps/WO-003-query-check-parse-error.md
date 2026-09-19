# WO-003 query check 对解析失败返回全零 + ok:true + exit 0（G-10）

> 台账：`docs/gaps/ledger.jsonl` 第 11 行（`id=G-10`、`severity=blocker`、`kind=tooling`、
> `status=open`、`wo_planned=WO-003`、`repro=docs/gaps/repro/G10-query-check-parse-error.sh`）。
> 模板出处：`docs/design/teaching-project.md` §6.3；排序出处：同文件 `:376`（P1 第三张）。
> 设计依据：`docs/design/agent-query-channel.md` §4.2（`check` 应带诊断）与 §5.2（退出码表）。
>
> 一句话：`query check` 把"这份文本根本解析不了"当成"文件里什么都没有"——agent 的
> 主判卷通道因此**假绿**，而同一份文本走 `grade` 是正确的。

## 用户可见症状 / 最小复现

- 复现命令（一条，可直接粘贴）：`bash docs/gaps/repro/G10-query-check-parse-error.sh`
- 本次实跑（2026-09-18；仓库 `Cargo.toml:6` = 0.58.0，darwin-arm64，
  `scripts/soko version --json` 自述 0.58.0）。脚本 **exit 0 = 缺口仍在且与台账一致**：

  通道 1 —— `node scripts/soko query check --text 'infix:50 " e " => mem' --compact`：

  ```json
  {"data":{"counts":{"decl_checked":0,"decl_printed":0,"example_checked":0,
   "exercise_open":0,"expr_reduced":0,"expr_typed":0},"failed":[],"version":1,"warnings":[]},
   "ok":true,"op":"check","schema":"soko.query/1","version":1}
  ```

  → `counts` 全 0、`failed: []`、`ok:true`、**exit 0**。

  通道 2 —— 同一份文本落盘后 `node scripts/soko grade --json /tmp/soko-g10-repro.sokonanoda`：

  ```json
  {"code":"unexpected-token","stage":"parse","type":"diagnostic",
   "span":{"start":{"line":1,"column":10,"offset":9},"end":{"line":1,"column":10,"offset":9}}}
  ```

  → **exit 1**。（脚本的完整输出见脚本自身；这里只贴判据行。）

- 补充实测（本 WO 为写单而跑，不是抄台账）：
  1. **`--file` 与 `--text` 一样假绿**：`query check --file /tmp/g10-bad.sokonanoda --compact`
     返回同样的全零 + `ok:true` + exit 0。台账只记了 `--text`，但两条输入通道在
     `crates/cli/src/query.rs:148-172` 合流到同一处 `set_text`。
  2. **同一个洞还漏到另外两个 op**：同一份坏文本上 `query goals` 给 `data: []`、
     `query holes` 给 `{"holes":[],"navigated":null}`，都是 exit 0——形状上等于"这份
     画布没有声明、没有洞"。而 `query state` 给 `ok:false` + `error.code:
     "outside-declarations"`（不是 `not-parsable`）。三个 op 对同一份坏文本三种口径。
  3. **MCP / DSH 侧同样假绿**：`mcp__sokonanoda__check(text='infix:50 " e " => mem')`
     本次实测返回与通道 1 **逐字节相同**的 JSON（`dsh/mcp/server.js:182-193` 转发
     `scripts/soko query check`，转发契约由 `crates/cli/tests/dsh.rs:243-255` 钉住）。
  4. **边界（决定范围，很重要）**：如果坏的是**被 `import` 的依赖**而不是入口自己，
     `query check` 今天是**对的**——实测夹具 `/tmp/g10proj/{Main,Bad}.sokonanoda`
     （`Main.sokonanoda` = `import Bad` + 一条正常定理）得到
     `{"failed":[{"code":"import-dependency-failed","start":0,"end":10}]}` 且 **exit 1**；
     同一形状有既有回归测试 `crates/cli/tests/project_features.rs:222-253`，归因机制见
     `docs/architecture.md:212-219`。所以缺口**不是**"项目诊断丢了"，而恰好是
     **入口文件自己解析不了**那条路径。

- 根因（读源码，非推测）：`crates/front/src/query/mod.rs:272-316` 的 `check()` 只读
  `self.output`，counts / failed / warnings 全部由它派生；而 `self.parse_error`
  （同文件 `:50-51`）**没有任何读者**——它只在 `:118`（`set_text`）与 `:206`
  （`set_cached_entry`，写 `None`）被写过。`crates/front/src/query/` 内没有一处读它；
  全仓仅有的两个消费者是 `crates/lsp/src/lib.rs:79`（LSP 诊断，已正确）与
  `crates/cli/src/watch.rs:271`（watch 流）。而 `parse_or_empty`（`:594-600`）的注释
  写着"失败时给一个空文件（**`check` 会带着 parse 诊断返回**）"——注释与实现对不上，
  `docs/design/teaching-project.md:264-265` 正把这条列为台账机制的四个真实教训之一
  （"文档与实现分叉"）。

## 期望行为

- **这是工具链行为，不是 Lean 语义**：官方 Lean 4 没有 `query` 通道。对应物是
  `lean Foo.lean` 遇语法错误时**打印带位置的 error 并以非零退出码结束**，不会输出
  "0 errors"（本仓硬规则 2 禁止调用官方工具链，故此处只作语义参照，**不写成验收命令**；
  未在本仓实测）。
- 本 WO 的权威判据是台账 `expected_lean`：**"解析失败必须作为诊断出现；agent 查询
  通道的 `ok:true` 只表示「问出来了」，不能吞掉 parse 错误"**。
- 落地契约（修好后写进 `docs/protocol.md`）：
  1. `check.data.failed` 至少含一条 parse 诊断：`code` ∈ {`unexpected-token`、
     `unexpected-eof`、`import-malformed`、`import-not-a-valid-module-name`、
     `import-must-precede-declarations`}（`crates/front/src/diagnostic.rs:44-51`），
     `message` 原样，`start`/`end` = 诊断 span 的字节 offset（本次实测 9/9）。
  2. `query check` 退出码 **1**（今天 0）：`docs/design/agent-query-channel.md:252-257`
     的表里 "1 = 有内核拒绝的声明"扩成"有拒绝（内核**或 parse**）"。
  3. **`ok` 保持 `true`**：`docs/protocol.md:669-672` 把 `ok:false` 定义成"问不出来"；
     这里问出来了（答案就是"这份文本解析不了"）。塞进 `ok:false` 会破坏既有语义
     （`ok:false` 只带 `error.code`，消费者按"问不出来"处理）。
  4. `counts` 保持全 0（诚实：确实一条声明都没验过）；**不加**新计数键——`CheckCounts`
     的键与事件流逐项对应（`crates/front/src/query/types.rs:147-156`），加键等于给
     同一个事实两套口径。
- 本教学子集的边界（本 WO 不做）：
  - parser **首错即停**，只承诺**一条** parse 诊断，不做多诊断聚合 / error recovery；
  - 不承诺诊断 `message`/`hint` 的文案稳定（判据是 `code` + span + 退出码）；
  - 不要求摘要层的 parse 条目与 `--json` 事件流的 `diagnostic` 行**逐字段相等**
    （事件流带 `stage`/`hint`，摘要层的 `FailedDecl` 今天没有这两个字段，见"范围"）。

## 范围

全部行号来自本次读源码；**是否动内核：预期否**——内核是冻结快照（硬规则 1），
本缺口整条链在 `front::query` 及其传输层之上，`crates/kernel/**` 一行不碰。

| 位置 | 现状 | 本 WO 要做的事 |
|---|---|---|
| `crates/front/src/query/mod.rs:272-316` | `check()` 只读 `self.output` | 把 `self.parse_error` 合成进 `failed`（唯一真相点） |
| `crates/front/src/query/mod.rs:50-51`、`:118`、`:206` | `parse_error` 只写不读；`:206` 置 `None` | 不改写入口，补"缓存命中不得静默"的不变量（见下） |
| `crates/front/src/query/types.rs:137-145`、`:159-165` | `CheckSummary.failed: Vec<FailedDecl>`；`FailedDecl{name,code,message,start,end}` | 默认**不改字段**（修法 A）；选 B 才加 `stage`/`hint` |
| `crates/cli/src/query.rs:267-275` | `exit = if failed == 0 {0} else {1}` | **一行不改**：failed 非空后退出码自动变 1 |
| `crates/cli/src/query.rs:148-172` | 装填文档：无 `import` → `set_text` | 不改；注意 `has_imports`（`:175-179`）对解析失败的文本返回 `false`（`unwrap_or(false)`）⇒ 坏入口**必然**走 `set_text`、`parse_error` 必然被写 |
| `crates/cli/src/project_cache.rs:32-41` | `store_if_clean` 只缓存 clean 产物 | 不改；它正是 `set_cached_entry` 能安全置 `None` 的理由 |

### 修法（两选一，推荐 A；实现者可依设计文档取舍）

- **A（最小，推荐）**：`check()` 构造 `failed` 时先 `output.errors`、再（若有）
  `self.parse_error` → `FailedDecl { name: None, code: diag.code(), message,
  start/end = span offsets }`。约 10 行，wire 形状只是"多一条元素"，符合协议
  "字段只增不改"（`docs/protocol.md:681-682`）。
- **B（对齐设计）**：`docs/design/agent-query-channel.md:209` 写明 `check` 的负载应含
  "诊断 `{stage,code,message,span,hint}`"。今天 `FailedDecl` 缺 `stage`/`hint`，
  agent 只能靠 code 猜"parse 还是内核"。做法：给 `FailedDecl` 加
  `#[serde(default, skip_serializing_if = "Option::is_none")]` 的 `stage`/`hint`，
  内核失败条目回填 `stage:"kernel"`。**代价**：动 `docs/protocol.md:650` 的字段表 +
  前端契约测试。它是**加法**，本 WO 不强制。
- 无论 A/B：parse 诊断的 `failed[].name` 保持 `None`——解析失败时没有可信的声明名，
  硬凑一个会误导 agent（字段本就是 `Option<String>`）。

### 缓存路径的不变量（必做，防回归）

`set_cached_entry`（`mod.rs:197-212`）显式把 `parse_error` 置 `None`。今天这**安全**，
三条合起来：① 只有带 `import` 的文档才走缓存（`crates/cli/src/query.rs:152`）；
② `has_imports` 对解析失败的文本返回 `false`（`:175-179`）⇒ 坏文本永远走 `set_text`；
③ 缓存**只存 clean 产物**（`crates/cli/src/project_cache.rs:32-41`）。但这是**隐式**
  不变量：将来任何一条改动（例如"把带诊断的结果也缓存"）都会让 G-10 复活。要求：
  加一条测试把它钉住（见"验收"的第三条 front 单测）。

### 兼容策略（课程侧与各消费者；本 WO 不改名、不改事件流）

- **课程侧（`courses/set-theory/`，12 单元）不会因此变红**：
  `courses/set-theory/tools/check.py:1-11` 现在**刻意不用** `query check`，判据只用
  `grade` 的退出码与事件计数（`check.py:31-47`）。本 WO 不碰 `grade`、不碰事件流，
  所以课程门禁理论上零变化——**零变化正是验收要求**（受影响就是改错了层）。
  修好后 `check.py` **可以**（不强制同轮）加一路 `query check` 交叉校验；同轮**必须**
  改的只是**文档里的绕行说明**，否则下一轮 agent 会照旧纪律继续绕。
- **退出码 0→1 是有意的契约变更**：任何写成 `query check … && 继续` 的脚本遇到坏文件
  会开始 fail-fast——这正是本缺口要的。按 `docs/RELEASE.md:12-14`、`:77-79` 的版本
  纪律，本次是 minor（0.59.0），并在 `editor/vscode/CHANGELOG.md` 记一条（消费者可见）。
- **MCP / DeepSeek Harness**：`dsh/mcp/server.js` 的**转发逻辑不需要改**——
  `:133-138` 已经把 exit 1 当"有可用负载"继续转发，只有 exit ∉ {0,1} 才抛错；
  同轮只改 `:182-193` 的工具描述措辞（"kernel rejections" → "parse/kernel rejections"）。
  `crates/cli/tests/dsh.rs:243-310` 只钉 op 名、MCP 握手事实与"server.js 里不得有
  判卷逻辑"，改描述不会破它。
- **LSP 不需要改**：`crates/lsp/src/lib.rs:77-88` 已经是"parse 诊断优先"的正确形态
  （`parse_error` 有值就只发它）。验收里只加一条"LSP 诊断不回归"。
- **VS Code 扩展**：不消费 `query check` 摘要（走 `soko/*` 请求 + LSP 诊断），预期只需
  按版本纪律 bump + CHANGELOG。**待确认**：若实现者在 `editor/vscode/` 里发现 shell out
  到 `query check` 的地方，同轮改。
- **不牵动 G-02 式的改名**：本 WO 不新增/重命名任何语法或课程库名字，因此
  `courses/set-theory/lib/Prod.sokonanoda` 的 `prod_mk` 等**不在同轮改动清单里**
  （"裸名保留为别名 + 新增 `Type.ctor` 形式"是 WO-005/G-02 的兼容策略问题，与本单正交）。
- **同轮改动文件清单**：
  1. `crates/front/src/query/mod.rs`（check 合成 parse 诊断）
  2. `crates/front/src/query/tests.rs`（红先单测）
  3. `crates/cli/tests/query.rs`（CLI e2e：exit 1 + payload）
  4. `docs/protocol.md`（`check` 行 + 退出码）
  5. `skills/sokonanoda-teacher/SKILL.md`（判卷命令旁一句话）
  6. `AGENTS.md:22`、`courses/set-theory/AGENTS.md:20`、`courses/set-theory/README.md:19`
     （绕行说明）
  7. `docs/design/agent-query-channel.md`、`docs/design/teaching-project.md`（as-built）
  8. `dsh/mcp/server.js`（描述一行）、`docs/HANDOVER.md`、`STATUS.md`、
     `REQUIREMENTS.md` §9、`Cargo.toml` + `editor/vscode/package.json`（0.59.0）、
     `editor/vscode/CHANGELOG.md`

## 不做的事（明确排除，防顺手扩大）

1. **不修 `goals` / `holes` 的同款假绿**（实测第 2 条）。它们要动的是"数组负载 vs 错误
   信封"的形状选择——`data: []` 是协议里"正常的没有"的既有语义
   （`docs/protocol.md:669-672`），属于**另一张单**：请课程线另开一条缺口（建议 `G-16`
   "query goals/holes 对 parse 失败返回空数组"）。理由：一次一刀
   （`docs/design/teaching-project.md:327-329`）。
2. **不改 `query state` 的 `outside-declarations`**：它至少是 `ok:false`，不假绿；
   要不要改成 `not-parsable` 归上面那张单。
3. **不做 parser 的 error recovery / 多诊断**：`parse` 首错即停，parse code 只有 5 个
   （`crates/front/src/diagnostic.rs:44-51`）。
4. **不动 `grade` / `--json` 事件流**：事件流里的 parse 诊断今天就是对的
   （`stage:"parse"` + `hint` + 位置），本 WO 只让摘要视图追上它。改事件流会撞双 GOLDEN
   与所有消费者。
5. **不把 parse 失败改成 `ok:false`**（理由见"期望行为"第 3 条）。
6. **不动内核**（`crates/kernel/**`）、不动 parser 语法、不动 `front::session` 产生
   `parse_error` 的逻辑（`crates/front/src/session.rs:69`、`:155`）。
7. **不改诊断文案**：不重写 `unexpected-token` 的 message/hint。

## 验收（三层）

- **front 单测**（红先；落在 `crates/front/src/query/tests.rs`，现有
  `check_reports_kernel_failures:291-303` 与 `check_counts_match_the_event_stream:261-289`
  旁边）：
  - `check_reports_a_parse_error_instead_of_all_zeros`：坏文本（如
    `infix:50 " e " => mem`）→ `check().failed` 长度 1、`code == "unexpected-token"`、
    `start == end`（本次实测 offset 9）、`counts` 全 0、`failed[0].name == None`。
  - `check_keeps_reporting_kernel_failures`（回归）：`example : Prop := 1` 仍 `failed`
    非空、code 是内核 code——确认两类失败没被混成一个。
  - **缓存路径不变量**：构造"报告来自缓存"的装填（`set_cached_entry`）并对坏文本断言
    `check().failed` 非空；实现者若判断该路径在类型上不可达，就把断言写成对
    `set_cached_entry` 前置条件的显式检查 + 注释，并在测试里钉住
    `store_if_clean` 只存 clean（`crates/cli/src/project_cache.rs:32-41`）。
  - **一致性契约不回归**：`check_counts_match_the_event_stream` 必须仍绿（clean 画布上
    counts 与 `--json` 逐项相等）。
- **CLI e2e**（`crates/cli/tests/query.rs`）：
  - `query_check_reports_parse_errors_with_exit_one`（**主验收**，仿 `:300-309` 的写法）：
    `--text` 与临时 `--file` 两条输入通道各一次 → `code == 1`、`ok == true`、
    `failed[0].code == "unexpected-token"`、`failed[0].start == failed[0].end`、
    `counts` 全 0。
  - `query_check_still_exits_one_for_kernel_rejections`（`:300-309` 已存在 → 不动）。
  - 项目路径回归：`query check --file <entry>` 对"依赖坏、入口好"的夹具仍报
    `import-dependency-failed` + exit 1（夹具抄 `crates/cli/tests/project_features.rs:222-253`）。
  - 复现脚本本身：`bash docs/gaps/repro/G10-query-check-parse-error.sh` 从 **exit 0 翻成
    exit 1**（脚本断言"全零 + failed 空"，修好后该断言失败并打印"更新 G-10"）。
- **课程用例**：
  - 正例（必须保持绿）：`courses/set-theory/units/unit05-pairs-products.sokonanoda` 的开放
    练习 **`prod_fst_mk`**（同单元另有 `prod_snd_mk`、`prod_mk_inj`、`prod_mk_eq_iff`、
    `mem_prod_iff`、`prod_set_swap_ne`、`kura_degenerate`，共 7 个 open；名字由本次
    `grade --json` 的 `exercise.open` 事件逐条读出）。判据（本次基线，实测）：

    ```
    node scripts/soko query check --file "$PWD/courses/set-theory/units/unit05-pairs-products.sokonanoda" --compact
    # decl_checked = 5 · exercise_open = 7 · failed = [] · exit 0
    ```

    改动后必须仍是这组数（`sorry` 是合法状态，`docs/protocol.md:673-675`）。
  - 反例（本次新增判据）：把该单元复制到 `/tmp` 后**故意删掉一个右括号**（或删一个 `:=`），
    `query check --file <副本>` 必须 `failed` 非空 + **exit 1**，且与 `grade <副本>` 的
    `unexpected-token` + exit 1 **口径一致**——这正是台账 `today` 里"两条通道不一致"
    的反面。
  - 课程门禁（不回归）：`python3 courses/set-theory/tools/check.py` 本次实跑 =
    **34 个目标 · 296 checked · 93 open · 0 个被判负**（exit 0）。修完必须同数。
- **影响面（事件计数 / 双 GOLDEN）**：
  - 本 WO 只改 `front::query` 的**摘要视图**；退出码是既有的"由 failed 数量推导"规则
    （`crates/cli/src/query.rs:267-275` 一行不改），`--json` 事件流逐字节不变。
  - 因此 `crates/cli/tests/course.rs:86-98` 的 `GOLDEN`（11 单元
    `(checked, open, reduced)`）与 `crates/cli/tests/course_status.rs:68-82` 的 `GOLDEN`
    （11 单元 `(checked, open, failed, reduced)`）**都不需要同步**。PR 描述里请明说
    "两条 GOLDEN 未动是有意的"；**若发现其中任一条需要改，说明改动漏进了 `compile`
    层**（改错地方了），回退重做。
  - 课程画布一个字不改（`courses/set-theory/` 只动说明文字与（可选）`check.py`）。

## 文档同步清单

- `docs/protocol.md`：**必改**（对外契约）：`:650` 的 `check` 行（`failed[]` 现含 parse
  诊断；若选 B 则加 `stage`/`hint`）+ `:673-675` 退出码（1 = 内核拒绝**或源文本解析失败**）。
- `docs/design/agent-query-channel.md`：`:209`（check 负载的 diagnostics 形状）与
  `:250-257`（退出码表）的 as-built 回填；§7 的 A4 一致性契约旁补一句"parse 失败同样
  进 `failed`"。
- `docs/design/teaching-project.md`：`:264-265`（"文档与实现分叉"这条教训）标注
  "0.59.0 已修，注释与实现重新对齐"；`:376`（P1 排序）标记 WO-003 完成。
- `skills/`：`skills/sokonanoda-teacher/SKILL.md:63`、`:139`（判卷命令旁）加一句
  "parse 诊断也在 `failed[]` 里、exit 1，`query check` 与 `grade` 同口径"；
  `.agents/skills/` 是薄入口，不动（`crates/cli/tests/dsh.rs` 守漂移）。
- `AGENTS.md:22`、`courses/set-theory/AGENTS.md:20`、`courses/set-theory/README.md:19`：
  绕行说明改为"G-10 已修（≥0.59.0）：`query check` 带 parse 诊断；课程门禁仍以 `grade`
  退出码为准"。
- `dsh/mcp/server.js:182-193`：`check` 工具描述措辞（模型可见）。
- `docs/HANDOVER.md`（§3 剩余 TODO 的 G-10 行）、`STATUS.md`（本轮，只留 3 轮）、
  `REQUIREMENTS.md` §9（用户可见：agent 通道退出码语义变化，带日期）。
- `editor/vscode/`：`package.json` 版本 + `CHANGELOG.md`（若确认扩展不消费 `query check`，
  README 不必动）。
- `docs/gaps/`：本 WO 的 `wo` 字段回填 + G-10 关账；`docs/gaps/README.md:49` 附近的
  复跑清单可加一行"修好后 `query check` 与 `grade` 同口径"。

## 门禁

```bash
scripts/soko gate                                   # fmt + clippy + test + playground 锚点
cargo test --workspace --locked                     # 全量
bash docs/gaps/repro/G10-query-check-parse-error.sh # 期望 exit 1（脚本会喊"更新台账"）
python3 scripts/gap.py check                        # G-10 显示"行为已变 ← 台账写的是缺口仍在"
python3 courses/set-theory/tools/check.py           # 34 目标 · 296 checked · 93 open · 0 判负
```

> `scripts/soko gate` 的 anchor 用**运行中二进制**的内嵌编译器，版本不一致会 exit 3；
> 先 `scripts/soko update`（`AGENTS.md` 贡献者小节）。另：本次实测环境里
> `scripts/soko version --json` 的 `cli.source = repo-build`、`cli.marker = "0.55.0
> darwin-arm64"`，而版本自述 0.58.0——复现行为与台账 0.58.0 的记载逐字一致，故不影响
> 本单判据；但实现者跑 gate 前请确认用的是当轮构建。**这条现象属于启动器版本源
> （台账 G-11）的邻域，不要与 G-10 混为一谈、不要顺手修。**

## 关账

```bash
python3 scripts/gap.py close G-10 --version 0.59.0
```

- `close` 会先复跑 `docs/gaps/repro/G10-query-check-parse-error.sh`：修好后它 exit 1
  ⇒ 关账通过（仍复现即 exit 0，会被拒绝）。
- 台账回填：`status=fixed`、`fixed_in=0.59.0`、
  `wo=docs/gaps/WO-003-query-check-parse-error.md`（`wo` 只在文件确实存在时填，
  `docs/design/teaching-project.md:282-283`）。
- 版本：`Cargo.toml:6` 与 `editor/vscode/package.json:5` 两处一致 bump 到 0.59.0
  （`docs/RELEASE.md:12-14`、`:77-79`），`editor/vscode/CHANGELOG.md` 同轮一条。
- 课程侧：`courses/set-theory/sokonanoda.toml` 的 `requires = "0.58"` 升到 `"0.59"`，
  并复跑 `python3 courses/set-theory/tools/check.py` 确认 0 判负。
