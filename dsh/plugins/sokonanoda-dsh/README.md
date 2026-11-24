# sokonanoda-dsh — 统一的 DSH 语言服务插件

一个插件同时提供 **自动快思考** 与 **卡片渲染 / Lean 高亮**，替代此前分离的
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
                 │   key=learn-card → question / knowledge / lesson 卡片（纯展示）    │
                 │   key=lean / sokonanoda → 自定义语法高亮                           │
                 └───────────────────────────────────────────────────────────────────┘
```

- **快路径不换模型、不新增 provider**：同一 provider/model，前缀 cache 不受影响；
  命中时无需模型调用，失败/超时/空回复一律回落 `next()`。
- **卡片是纯展示**：不注册工具、不加 tool call / tool result；交互仍然是在输入框打字。
  底层字符流（assistant 文本）就是卡片数据的来源，历史重放、fork、复制都按文本走。
- **数据不进仓库**：配置、做题进度、决策历史全部在用户目录（见下）。

## 数据目录（唯一位置）

```
~/.dsh/state/sokonanoda-dsh/          # SOKO_DSH_DATA 可覆盖整个目录
├── config.json                       # 可选配置（bundle 行 config 覆盖它）
├── sessions.json                     # /sokonanoda-fast-think on 的会话白名单（默认空）
├── quiz/                             # 课程引擎状态（等价旧 quiz-proxy/state/）
├── history/
│   └── decisions.jsonl               # 快/回落/开关决策 + sokonanoda_fast_think 调用记录
└── logs/quiz-proxy/                  # 进程内 quiz service 的运行日志
```

**不在仓库里**（`SOKO_DSH_DATA` 或 `~/.dsh/state/sokonanoda-dsh/`）：把 GitHub
仓库交给别人时不会带上你的做题记录。旧位置的记录迁移命令见本文件末尾。

## 行为边界

- 只有**显式开启**的会话才拦截（`/sokonanoda-fast-think on`）：`开始做题 / 练几题 / 出题` 可以在
  已开启会话里开始；字母/继续/不懂/跳过 需要同一会话先有 quiz 状态。普通会话里说
  「继续」「A」不会被劫持。
- 「进度/学习地图」是只读快照：已开启会话里直接出进展热点卡；未开启会话走模型
  （模型可通过 `sokonanoda_fast_think` 工具读取同一份 payload）。
- 最后一条消息必须是 user（工具结果触发的续轮一律 `next()`）。
- 非 agent-loop 调用（`purpose` 非空，如 compaction / session-title / auto-review）
  一律放行；loop 请求深度冻结，只读、不修改。
- `llm/stream` 合成流会被 agent loop 当作普通模型流落盘，后续慢请求能在历史里看到。
- 卡片语言词：`learn-card`（卡片 JSON）、`lean` / `sokonanoda`（高亮）。
  流式中 fence 层渲染原始代码块，settled 后替换；解析失败 fail-open 回原始代码块。
- 服务启动失败（引擎缺失、端口异常）不会阻断 Host 启动，插件保持 loaded，所有请求
  回落慢路径。

## 安装（本机）

```bash
# 从仓库安装（plugin_manager 需要批准；无 build scripts）
plugin_manager({ action: "install_bundle", target: "<repo>/dsh/plugins/sokonanoda-dsh" })
```

或 CLI（等价）：

```bash
dsh plugin --profile superagent add <repo>/dsh/plugins/sokonanoda-dsh
```

Host 半改动需要重启 DSH；只改 `client.js` 时由 client HMR 热替换。安装后：

- `Tool.listTools` 里应出现 `sokonanoda_fast_think`；
- `Config.listConfigs`（name=`sokonanoda-dsh`）有该行（没有 Config schema 时状态是
  `absent`，不影响挂载）；
- 开新会话输入「开始做题」→ <1s 出卡片；输入字母 → 判卷 + 下一张卡；
- 普通消息 → 仍走当前模型。

## 配置

`config.json`（用户目录）或 bundle 行的 `config`（行内优先）：

| 字段 | 默认 | 说明 |
|---|---|---|
| `enabled` | `true` | 总开关（关闭后全部 `next()`） |
| `dataDir` | `~/.dsh/state/sokonanoda-dsh/` | 数据根目录 |
| `repoRoot` | 自动向上查找 | 含 `courses/set-theory/knowledge/engine.py` 的目录 |
| `timeoutMs` | `8000` | 快路径超时（超时回落慢路径） |
| `decisionsLimit` | `20` | `sokonanoda_fast_think status` 默认返回条数 |
| `cards` | `true` | 让 quiz service 产出卡片 payload；关闭则退回纯文本快路径 |

## 内部结构

| 文件 | 作用 |
|---|---|
| `index.js` | Host 半：数据目录、进程内 quiz service、`llm/stream` 快路径、`sokonanoda_fast_think` 工具 |
| `client.js` | Client 半：`learn-card` / `lean` / `sokonanoda` 三个 fence 层 + 卡片表面 CSS |
| `lib/quiz-proxy.mjs` | 从旧 `quiz-proxy/server.mjs` 迁入的本地题库服务（`startQuizService` 在进程内起 loopback 端口）；保留 I2/I3/I4 正确性不变量 |
| `test/host.test.mjs` | Host 契约：数据目录、判定、fence 转换、进程内服务、llm/stream 放行 |
| `test/client.smoke.cjs` | Client 渲染契约：3 个 fence 层、卡片/高亮、fail-open、CSS |
| `docs/` 下的 `research.md` | 快思考方案调研（llm/stream、生态对照） |
| `docs/` 下的 `cards-research.md` | 卡片渲染方案调研（fence 层、action-cards、槽位） |

## 测试

```bash
cd dsh/plugins/sokonanoda-dsh
node test/run.mjs     # host 14 + client 17
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
