# sokonanoda-dsh — 统一的 DSH 语言服务插件

一个插件同时提供 **自动快思考**、**卡片渲染 / Lean 高亮** 与 **侧栏做题面板**，
替代此前分离的
`@local/dsh-learn-card`、`@local/dsh-fast-think`、`@local/dsh-quiz-keeper` 三个
插件和 `quiz-auto` model provider / 独立 `quiz-proxy` 进程。

```
                 ┌───────────────────── DSH Host（本插件 host 半）─────────────────────┐
用户消息 ──────► │ agent loop → ctx.llm.stream(request)                              │
                 │   └─ llm/stream waterfall                                          │
                 │        ├─ 命中 quiz 会话（开始/字母/继续/不懂/地图…）                │
                 │        │    → 进程内 quiz service（lib/quiz-proxy.mjs，本地课程引擎）│
                 │        │    → 回复转成 assistant 文本（含 ```learn-card 围栏）      │
                 │        └─ 其它 / 工具结果 / 辅助调用 / 异常 → next()（当前模型）    │
                 └──────────────────────────────┬────────────────────────────────────┘
                                                │ 普通 assistant/message（字符流不变）
                 ┌──────────────────────────────▼──── DSH Web（本插件 client 半）──────┐
                 │ conversation.chat.markdown.fence                                  │
                 │   key=learn-card → question / knowledge / lesson / code 卡片       │
                 │   key=lean / sokonanoda → 自定义语法高亮                           │
                 │ 右栏 page tab（sokonanoda-exercise）→ 编辑器 + 运行 + 真内核结果   │
                 └───────────────────────────────────────────────────────────────────┘
```

- **快路径不换模型、不新增 provider**：同一 provider/model，前缀 cache 不受影响；
  命中时无需模型调用，失败/超时/空回复一律回落 `next()`。
- **卡片是纯展示**：不注册工具、不加 tool call / tool result；交互仍然是在输入框打字。
  底层字符流（assistant 文本）就是卡片数据的来源，历史重放、fork、复制都按文本走。
- **数据不进仓库**：配置、做题进度、决策历史全部在用户目录（见下）。

## 数学图 skill

插件包内附一份 **`sokonanoda-figure`** 作图规范（TikZ + 中文/数学字体搭配、
对比度 token、grid、标签/箭头避让、PDF 文字框 lint）：

```text
dsh/plugins/sokonanoda-dsh/skills/sokonanoda-figure/SKILL.md
```

DSH 的发现入口是对应的
`.agents/skills/sokonanoda-figure/SKILL.md`（指向仓库根下的 canonical 入口）。
作图前先读它，不要手摆 HTML/SVG 坐标冒充数学图。

## 只读源码预览（行号 / 高亮 / 跳行 + 光标 + goal 面板）

右侧辅助侧栏打开 `.sokonanoda` 文件时，插件注册自己的文件预览（不是纯文本回退），
行为对齐普通代码语言的 `CodeBody`：

- **绝对行号 gutter**：行号按 owner 分页的 `page.offset + index` 计算，不是把当前切片
  从 1 重排；懒加载追加页后行号继续增长、滚回顶部也不重置。gutter 宽度按当前最大
  绝对行号算（三位数不会挤断）。
- **Lean 高亮**：复用插件的 `highlightLean`，逐行着色，不改动源文本。
- **点「出处」跳到真实行**：行元素挂 `data-textpreview-line`（原生 owner 的
  `scrollToLine`/`markLine` 直接命中），并保留 `data-source-line` 给光标面板；
  目标行滚动到可见位置并闪一下。目标行所在页尚未加载时，由 owner 先加载下一页，
  到页后再定位。
- **懒加载**：按 owner 的 `text-pages` 分页读取，不把整份 `content.text` 重新
  `split('\n')`；滚动到底自动加载下一页，面板在文件未读完时提示「滚动到底部加载更多」。
- **空文件 / 加载中 / 读失败**：加载中与读失败由文档预览 owner 显示；0 行文件由
  本预览显示「空文件（0 行）」，不伪造第 1 行。
- **复制**：对齐原生代码预览，复制已加载的源码（与懒加载范围一致），成功显示
  「已复制」，失败显示「复制失败」。
- **↑/↓ 移动光标行**（`Ctrl`/`Cmd`/`Alt`/`Shift` 组合键照旧交给浏览器），点击任意行也能定位；
  当前行有背景高亮，预览获得焦点后才响应方向键，不会抢输入框。
- 下方面板实时显示**声明名 / kind / 状态 / `步骤 step/total` / 假设 / `⊢ goal`**；goal 与假设
  使用内核 `query state` 的 `goal_runs`/`ty_runs` 上色，**不重新分词**。没有活动行时只显示操作提示。
- 面板区分 **`光标不在任何声明内`**、**源码有错误（前 3 条诊断）**、**编译/查询超时**、
  **后端不可用** 和 **已无目标（证明完成）**；切换行时保留上一条 goal 并标「更新中…」。
- **只读**：没有编辑、保存、`didChange`；不写文件、不写仓库。Host 路由
  `GET /sokonanoda-state` 只接受**本机 owner**，路径必须落在当前会话工作区或仓库内
  （拒绝 `..`、绝对路径逃逸与符号链接逃逸），并用参数数组调用
  `scripts/soko query state`（不拼 shell、2–3 秒超时、超时杀整个子进程组）。
- 客户端请求去抖 120 ms、单飞（新行会 abort 旧请求），并用序号丢弃过期回答；
  内核每次热态约 0.12 s，命中编译缓存。

> 预览的文件内容仍由 DSH 文档预览按页提供；光标行只在已加载的行内移动。文件超过一页时，
> 滚动到底部会加载下一页（面板会给提示）。

## 数据目录（唯一位置）

```
$DSH_HOME/state/sokonanoda-dsh/       # 默认 ~/.dsh；SOKO_DSH_DATA 可覆盖整个目录
├── config.json                       # 可选配置（bundle 行 config 覆盖它）
├── sessions.json                     # /sokonanoda-fast-think on 的会话白名单（默认空）
├── quiz/                             # 课程引擎状态（等价旧 quiz-proxy/state/）
├── history/
│   └── decisions.jsonl               # 快/回落/开关决策 + sokonanoda_fast_think 调用记录
└── logs/quiz-proxy/                  # 进程内 quiz service 的运行日志
```

**不在仓库里**（`SOKO_DSH_DATA` 或 `~/.dsh/state/sokonanoda-dsh/`）：把 GitHub
仓库交给别人时不会带上你的做题记录。旧位置的记录迁移命令见本文件末尾。

## 隐私边界（仓库会公开分享）

- 用户代码只在系统临时目录里落一份，判完即删；**不进** `history/decisions.jsonl`、
  不进插件日志、不进仓库。
- 做题进度、会话开关、决策历史只在 `$DSH_HOME/state/sokonanoda-dsh/`
  （`SOKO_DSH_DATA` 可整体搬走）；分享 `git clone` 不会带上这些记录。
- 插件对课程/数据集**只读**；`compose.py` 的产物也只落临时目录。
- 公开材料（README / 技能 / 站点）只写路径与命令，不写个人目录、密钥或真实做题记录。

## 行为边界

- 只有**显式开启**的会话才拦截（`/sokonanoda-fast-think on`）：`开始做题 / 练几题 / 出题` 可以在
  已开启会话里开始；字母/继续/不懂/跳过 需要同一会话先有 quiz 状态。普通会话里说
  「继续」「A」不会被劫持。
- 「下一题 / 继续」在当前 issued 题**还没作答**时重发当前题、不推进（中间看知识卡、
  聊别的、隔多少页都不影响）；想明确跳题用「换一题 / 再来一题 / 换一道」。
- 「上一题 / 回看刚才那题」是**只读回顾**：正确选项绿色高亮 + `why` 解析，
  不计分、不推进、不写 history。
- 「进度/学习地图」是只读快照：已开启会话里直接出进展热点卡；未开启会话走模型
  （模型可通过 `sokonanoda_fast_think` 工具读取同一份 payload）。C9.5 起卡的摘要行
  显示学习地图的**已解 / 待做 / 薄弱**知识点数与代码题 `已解/总数`，数字来自
  `engine.py map`（与聊天里「学习地图」命令同一份快照）；不做云同步、不新增账号。
- 「代码题 / 来道代码题 / 写代码」发一道**代码练习卡**：题干 + 提示 + 可信模板
  （带 `sorry`）。把目标声明完整贴回围栏里（` ```lean ` / ` ```sokonanoda `）⇒
  插件组装完整 `.sokonanoda` 并调 `scripts/soko grade --json`，只有目标声明
  `decl.checked`、无 `exercise.open`、无 `diagnostic` 才显示「通过」；否则显示首条
  内核诊断并可**同题重试**。「跳过这道代码题」调 `engine.py skip-code` 记 skipped。
  通过后调 `engine.py mark-code --solved` 记进度。用户代码/诊断**不写日志**；判卷
  超时、缺 `compose.py`/`scripts/soko` 时 503 让 Host 回落 `next()`。
- **侧栏做题面板（C9.7）**：代码练习卡的「在侧栏面板中打开」在右栏开一个
  `sokonanoda-exercise` page tab（题目 + 提示 + 编辑器 + 运行）。面板只把
  `exerciseId` 交给 Host，签名/模板由 Host 从仓库题目集取（**答案从不下发**），
  「运行」仍走 C9.4 的 `compose.py` + `scripts/soko grade --json`；只有
  `passed:true` 才显示通过，`diagnostic/sorry` 显示失败，服务异常显示「不可用」。
  没有右栏服务时按钮不出现，聊天提交路径（上一条）原样保留。面板运行**不写**
  课程进度（`mark-code` 仍在聊天路径）；详见 `dsh/plugins/sokonanoda-dsh/exercise-panel.md`。
- 最后一条消息必须是 user（工具结果触发的续轮一律 `next()`）。
- 非 agent-loop 调用（`purpose` 非空，如 compaction / session-title / auto-review）
  一律放行；loop 请求深度冻结，只读、不修改。
- `llm/stream` 合成流会被 agent loop 当作普通模型流落盘，后续慢请求能在历史里看到。
- 卡片语言词：`learn-card`（卡片 JSON）、`lean` / `sokonanoda`（高亮）。
  流式中 fence 层渲染原始代码块，settled 后替换；解析失败 fail-open 回原始代码块。
- 服务启动失败（引擎缺失、端口异常）不会阻断 Host 启动，插件保持 loaded，所有请求
  回落慢路径。

## 从零安装（干净机器，一条路）

前置：`git`、`node`（DSH 自带同版本即可）、`python3`（题库引擎与
`compose.py` 用它）。**不需要 Rust/cargo**：`scripts/soko` 会按版本钉取已发布
二进制（或复用仓库构建），插件本身只读课程与数据集。

```bash
git clone https://github.com/ColorlessBoy/sokonanoda-lang.git
cd sokonanoda-lang

# 1) 环境：版本锁定的 CLI + LSP（幂等；零 cargo）
scripts/soko setup
scripts/soko doctor --json            # 期望 "ready": true

# 2) 装进 web profile（等价于会话里的 plugin_manager；用别的 profile 就换名字）
dsh plugin --profile web add "$PWD/dsh/plugins/sokonanoda-dsh"

# 3) 启动并开一个会话
dsh web
```

安装是 pnpm 本地路径安装（**不需要网络**），产物在 `$DSH_HOME/profiles/web/`
（`DSH_HOME` 未设时默认 `~/.dsh`）。`--profile` 必须与你启动的 profile 一致
（`dsh web` 默认 `web`；自定义如 `superagent` 就换成 `superagent`），否则插件不会挂载。
装好后按顺序自查（缺一条就不是装好了）：

1. `dsh plugin --profile web list` 能看到 `sokonanoda-dsh`；
   `dsh --profile web --dump-config | grep -A1 sokonanoda-dsh` 有该行；
2. 新会话里 `sokonanoda_fast_think status` 能返回（工具已挂）；
3. 输入 `/sokonanoda-fast-think on`，再说「开始做题」→ 1 秒内出题卡；
   说「代码题」→ 代码练习卡；卡片上的「在侧栏面板中打开」→ 右栏面板
   （题目 + 编辑器 + 运行），运行结果同样来自真内核，服务不可用时明确报错、不显示通过。

Host 半改动需要**重启 `dsh web`**；只改 `client.js` 时由 client HMR 热替换，
没有 HMR 的安装刷新一次页面即可。

### 在会话里安装 / 升级 / 卸载

```bash
# 会话内（需要批准；无 build scripts；与 CLI 等价）
plugin_manager({ action: "install_bundle", target: "<repo>/dsh/plugins/sokonanoda-dsh" })
```

升级：`git pull` 后重跑一次 `dsh plugin --profile web add "$PWD/dsh/plugins/sokonanoda-dsh"`，
再重启 `dsh web`（client 半刷新页面）。卸载：
`dsh plugin --profile web remove sokonanoda-dsh`。数据目录（见下）不随卸载删除；
要一并清掉就删 `$DSH_HOME/state/sokonanoda-dsh/`。

## 题库与代码题（数据从哪来）

插件**不内置题目**，只消费仓库里的两份派生物；课程源是唯一真相，插件只读：

| 题目类型 | 数据入口 | 当前基数（2026-11-27） |
|---|---|---|
| 选择题 / 问答题 | `courses/set-theory/knowledge/data/questions.json`（`engine.py` 调度） | 409 道（400 可服务；choice 392 / short 5 / explain 3） |
| 知识点 | `courses/set-theory/knowledge/data/knowledge-points.json` | 195 个（96 绿 / 99 红；红 = 题量未补，见 `CONTENT-PLAN.md`） |
| 代码题 | `courses/exercise-dataset/data/*.json`（`compose.py` 组装） | 2022 道 / 298 单元（三门课门禁判过的练习，解答 100%） |

- 选择题库同时服务卷 I 与卷 II.1（分析课）；章节覆盖 I.1–I.3、I.8、I.9、II.1。
- **改题改课程源**：数据集/题库都是派生物。改完课程画布/解答后跑
  `python3 courses/exercise-dataset/build.py --check` 与
  `python3 courses/set-theory/knowledge/validate.py`；不要手改生成物。
- 代码题按 `engine.py code-next` 的确定顺序发放（默认 `set-theory`），同一道题
  未完成时重复要「代码题」会重发当前题；「跳过这道代码题」记为 skipped。
- 相关入口：`courses/exercise-dataset/README.md`（数据集 schema 与校验）、
  `courses/set-theory/knowledge/README.md`（题库与调度引擎）、`sokonanoda-prep`
  技能（备课/扩题流程）。

### 代码题怎么提交

1. 说「代码题」→ 卡片给出**题干 + 提示 + 带 `sorry` 的模板**；
2. 在编辑器里补完目标声明，把它整条贴回聊天（` ```lean ` 或 ` ```sokonanoda `
   围栏；只贴证明体也可以）；
3. 插件只取**证明体**，贴回课程模板的可信签名（改签名/换命题不会生效），用
   `compose.py` 组装完整文件，再跑 `scripts/soko grade --root courses/<course>
   --json`；
4. 只有目标声明 `decl.checked`、目标无 `exercise.open`、且 0 个 `diagnostic`
   才显示「通过」并记 `mark-code`；否则显示首条内核诊断，可同题重试。

判卷需要 `python3`、`courses/exercise-dataset/compose.py` 与 `scripts/soko`
（都在仓库里）。超时（默认 45s）或工具缺失会 503 回落当前模型，**不会**把基础
设施失败伪装成「没过」。

## 配置

`config.json`（用户目录）或 bundle 行的 `config`（行内优先）：

| 字段 | 默认 | 说明 |
|---|---|---|
| `enabled` | `true` | 总开关（关闭后全部 `next()`） |
| `dataDir` | `$DSH_HOME/state/sokonanoda-dsh/`（默认 `~/.dsh/...`） | 数据根目录 |
| `repoRoot` | 自动向上查找 | 含 `courses/set-theory/knowledge/engine.py` 的目录 |
| `timeoutMs` | `8000` | 快路径超时（超时回落慢路径） |
| `codeTimeoutMs` | `45000` | 代码题快路径超时（compose + 真内核判卷；超时回落 `next()`） |
| `stateTimeoutMs` | `3000` | 只读 goal 面板调 `query state` 的超时（冷编译可调大；超时显示「查询超时」） |
| `sokoPath` | `<repoRoot>/scripts/soko` | 只读 goal 面板与代码题判卷共用的启动器路径 |
| `decisionsLimit` | `20` | `sokonanoda_fast_think status` 默认返回条数 |
| `cards` | `true` | 让 quiz service 产出卡片 payload；关闭则退回纯文本快路径 |

## 排障（先看这张表）

| 症状 | 先查 | 处置 |
|---|---|---|
| 说了「开始做题」但模型直接回答 | 会话开关 | `sokonanoda_fast_think status`；`/sokonanoda-fast-think on`。普通/新会话默认关闭是设计 |
| 卡片、工具都找不到 | 插件行是否挂上 | `dsh --profile web --dump-config \| grep sokonanoda-dsh`；没装就重跑安装；改过 Host 半要重启 `dsh web` |
| 卡片出现但选项点不动 / 样式丢失 | client 半未加载 | 刷新页面；`client.js` 改动靠 HMR 或页面重载 |
| 代码题总是回落模型 | 判卷依赖 | `scripts/soko doctor --json`；`python3 courses/exercise-dataset/compose.py --help`；缓存过期跑 `scripts/soko update`；必要时调大 `codeTimeoutMs` |
| 判卷通过但进度没变 | 数据目录权限 | `mark-code` 写失败**不改变内核判定**；检查 `$DSH_HOME/state/sokonanoda-dsh/quiz/` 可写 |
| goal 面板一直「状态后端不可用」 | 只读预览依赖 | 确认 `scripts/soko doctor --json` 就绪；改了 Host 半要重启 `dsh web`；冷编译慢就调大 `stateTimeoutMs` |
| 进度 / 答题记录像丢了 | 数据目录被换 | `DSH_HOME` / `SOKO_DSH_DATA` 变了；记录在 `$DSH_HOME/state/sokonanoda-dsh/` |
| `dsh plugin add` 失败 | 路径 / 包结构 | 目标目录必须有 `package.json` 与 `dsh.bundle.patch`；本地安装不需要网络 |
| 引擎 / 题库报路径错 | 仓库根 | 从仓库目录启动 `dsh web`（或设 `repoRoot`）；插件向上找 `courses/set-theory/knowledge/engine.py` |

运行日志在 `~/.dsh/state/sokonanoda-dsh/logs/`；代码题判卷的决策行只记题目 id、
通过与否、字节数与耗时，**不记**用户代码。

## 内部结构

| 文件 | 作用 |
|---|---|
| `index.js` | Host 半：数据目录、进程内 quiz service、`llm/stream` 快路径、`sokonanoda_fast_think` 工具、只读 `GET /sokonanoda-state` |
| `client.js` | Client 半：`learn-card` / `lean` / `sokonanoda` 三个 fence 层 + 卡片表面 CSS + `.sokonanoda` 只读预览（光标行 / goal 面板） |
| `lib/quiz-proxy.mjs` | 从旧 `quiz-proxy/server.mjs` 迁入的本地题库服务（`startQuizService` 在进程内起 loopback 端口）；保留 I2/I3/I4 正确性不变量 |
| `lib/code-grade.mjs` | C9.4 判卷桥：围栏提取 / 只取证明体贴回可信签名 / `scripts/soko grade --json` 事件判定；失败抛 `code` 交调用方 fail-open |
| `lib/exercise-panel.mjs` | C9.7 面板的 Host 桥：可信题目解析（数据集 / 引擎小题库，白名单不含答案）、两条 loopback 路由、请求体上限；判卷复用 `code-grade.mjs` |
| `test/host.test.mjs` | Host 契约：数据目录、判定、fence 转换、进程内服务、llm/stream 放行、只读 state 路由（owner/越权/超时）、面板路由 |
| `test/client.smoke.cjs` | Client 渲染契约：3 个 fence 层、卡片/高亮、fail-open、CSS、光标行与 goal 面板、面板 deep link 与结果状态 |
| `docs/` 下的 `research.md` | 快思考方案调研（llm/stream、生态对照） |
| `docs/` 下的 `cards-research.md` | 卡片渲染方案调研（fence 层、action-cards、槽位） |
| `exercise-panel.md`（插件目录根） | C9.7 侧栏面板：表面、两条路由、信任边界、结果状态、已知限制 |

## 测试

```bash
cd dsh/plugins/sokonanoda-dsh
node test/run.mjs     # host 33 + client 45
```

## 迁移与清理（一次性，2026-11-23）

旧实现（已删除）：`.cache/dsh-learn-card`、`.cache/dsh-fast-think`、
`.cache/dsh-quiz-keeper`、`.cache/quiz-proxy`、profile 里的 `local-quiz` provider。
记录迁移：

```bash
mkdir -p ~/.dsh/state/sokonanoda-dsh/{quiz,history,logs}
cp -R .cache/quiz-proxy/state/. ~/.dsh/state/sokonanoda-dsh/quiz/
cp .cache/dsh-fast-think/decisions.jsonl ~/.dsh/state/sokonanoda-dsh/history/ 2>/dev/null || true
```

迁移后删掉 `.cache/quiz-proxy` 与旧插件目录即可；本插件不再读取它们。
（2026-11-23 已完成：旧插件 bundle、`local-quiz` provider、独立 proxy 进程与
`.cache` 草稿目录均已移除；做题记录已复制到上述数据目录。）
安装/迁移后请**刷新 Web 页面**加载新的 client 半，并**重启 DSH** 让 Host 半新代码生效；
旧会话若曾把模型选成 `local-quiz/quiz-auto`，重启后请在模型选择器里切回默认模型。
升级后记得在形式化证明学习会话里先 `/sokonanoda-fast-think on`。
