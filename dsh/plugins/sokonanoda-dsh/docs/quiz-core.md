> **状态（2026-11-23 迁移）**：这份文档原本描述独立的 `quiz-proxy` 进程 / `local-quiz/quiz-auto`
> model provider。现在它的实现已内嵌进 `sokonanoda-dsh`（`lib/quiz-proxy.mjs`，
> 由插件进程内 `startQuizService()` 在 loopback 临时端口启动），不再有独立进程，
> 也不再是 model provider。运行状态目录见插件 README 的
> `~/.dsh/state/sokonanoda-dsh/quiz/`；本文保留的是引擎接口、I2/I3/I4 不变量与
> 排障口径，仍然适用。

# quiz-proxy —— 本地中转模型站（做题快路径 + opencode-go 慢路径）

给 DeepSeek Harness 用的**本地 OpenAI 兼容中转服务**：

- **做题类消息**（选字母、开始做题）→ 直接调用仓库里的**本地题库引擎**
  `courses/set-theory/knowledge/engine.py`，**不调用任何模型**（实测 60–160ms）；
- **卡片模式**（`QUIZ_PROXY_CARDS=learn` + DSH 装了真卡片插件 `@local/dsh-learn-card`）：
  代理直接以 assistant 身份发 `learn_card` 的 tool_call ⇒ DSH 聊天里渲染**真卡片**
  （选择题 / 知识卡 / 教学环节）；工具执行完的回显按 `tool_call_id` 认领并回空 stop
  （不再走慢模型）；用户在输入框回字母 → 本地判卷 → 再发下一张卡片。
  卡片 + 零模型延迟，全程不经过慢思考模型；**默认关闭**（安全档是纯文本快路径）；
- **其它消息** → 原样转发到 `https://opencode.ai/zen/go/v1/chat/completions`
  （用你自己的 `OPENCODE_GO_API_KEY_6`，慢思考不改变）。

DSH 只需要把 provider 的 `baseURL` 指向 `http://127.0.0.1:3090/v1`、模型选 `quiz-auto`。

- 零依赖：单个 `server.mjs`，Node ≥ 20（实测 Node 24.20 / macOS）。
- 只监听 `127.0.0.1`；日志写 stderr；**从不打印 API key**。

## 文件

| 路径 | 作用 |
|---|---|
| `server.mjs` | 服务本体（HTTP 路由 / 快路径 / 慢路径 / SSE / per-fingerprint 串行锁） |
| `start.sh` / `stop.sh` | 后台 nohup 启动 / 停止；pid、端口、日志在 `run/` |
| `test/quiz-proxy.test.mjs` | `node --test` 端到端测试（mock 上游，62 个用例） |
| `state/<fingerprint>.json` | 每个对话一份的题库引擎状态（引擎原生格式，运行时生成） |
| `run/quiz-proxy.log` · `run/quiz-proxy.pid` · `run/quiz-proxy.port` | 日志 / pid / 端口（运行时生成） |

## 启动 / 停止

```bash
cd .cache/quiz-proxy
./start.sh                       # 后台启动（默认 127.0.0.1:3090）
curl -s http://127.0.0.1:3090/health
# {"ok":true,"fastturns":0,"slowturns":0,"cards":false,"cardsMode":"off"}
./stop.sh                        # 停止
```

换端口：`QUIZ_PROXY_PORT=3091 ./start.sh`。启动脚本会等 `/health` 就绪才返回 0，
失败时打印日志末尾。重复 `./start.sh` 是幂等的（已在运行就提示并退出 0）。

## curl 自测

```bash
# 0) 存活 / 模型列表
curl -s http://127.0.0.1:3090/health
curl -s http://127.0.0.1:3090/v1/models

# 1) 出题（快路径，不调模型）
curl -sS -X POST http://127.0.0.1:3090/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"quiz-auto","stream":false,"messages":[{"role":"system","content":"你是题库助教"},{"role":"user","content":"开始做题"}]}'

# 2) 判卷（快路径）：带上同一对话的历史（system + 第一条 user 不变 ⇒ 同一 fingerprint）
#    把上一步的回复放进 assistant，再追加 {"role":"user","content":"D"}
curl -sS -X POST http://127.0.0.1:3090/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"quiz-auto","stream":false,"messages":[
        {"role":"system","content":"你是题库助教"},
        {"role":"user","content":"开始做题"},
        {"role":"assistant","content":"（上一步返回的题目）"},
        {"role":"user","content":"D"}]}'
#    → 差一点。/✅ 对了！ + 正确答案 X. … + 解析：… + 出处：… + --- + 📝 下一题

# 3) 流式（快路径也是 SSE：delta.content 分块 → finish_reason stop → [DONE]）
curl -sN -X POST http://127.0.0.1:3090/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"quiz-auto","stream":true,"messages":[{"role":"system","content":"你是题库助教"},{"role":"user","content":"开始做题"}]}'

# 4) 慢路径：非做题消息自动转发 opencode-go（需要 key，见下）
curl -sS -X POST http://127.0.0.1:3090/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"quiz-auto","stream":false,"messages":[{"role":"user","content":"用一句话解释外延公理"}]}'
```

判卷/出题两种模板：

```text
📝 题目（I.1 · ④ 外延性与集合恒等式）

<题干>

A. …  B. …  C. …  D. …

（回复字母作答：A / B / C / D）
```

```text
差一点。保持这个节奏。
正确答案：B. <答案文本>
解析：<why>
出处：<source>

---

📝 下一题（…）

<题干 + A/B/C/D>

（回复字母作答：A / B / C / D）
```

## 正确性不变量（I2 题目句柄 / I3 幂等与单一写者 / I4 模型不自作题）

背景事故：代理把「出题」和「判分」拆成两次请求，屏幕上的题、engine 的
`state.issued_qid`、模型上下文里的题可能不是同一道 ⇒ 用户看到 A 题却被记到 B 题。
现在用三条不变量堵住：

### I2 题目句柄（qid + nonce）

- `nonce = sha1(qid + plain/prompt + 选项).slice(0,16)`，**可由 engine 题面随时复算**。
- 每次出题（文本题 / learn_card / ask 卡片）都记录 `stateKey → {qid, nonce}`，
  并在**响应头**随题返回（对用户不可见、DSH 忽略）：
  `x-quiz-qid: I.1.empty-1`、`x-quiz-nonce: 3f2a…`。SSE/JSON 两种响应都有。
- 判分前先对齐「屏幕上的题」和 engine 当前 issued 题；**对不上 ⇒ 不记分**，返回
  「这道题已更新，请按屏幕上的题重答。」+ engine 当前题（不 advance、不写历史、
  响应头 `x-quiz-stale: 1` 带当前题句柄）。
- 高级用法：判分请求可回显句柄（`x-quiz-qid`/`x-quiz-nonce` 头，或 body
  `quiz_handle:{qid,nonce}`），这样代理能识别「针对旧题的迟到答案」并拒绝记分。
  不回显也安全：代理用自己 per-state-key 的出题记录对齐 engine issued 题。

### I3 幂等 + 单一写者

- 锁的键 = **state key**（不是 fingerprint）：同一状态文件同一时刻只有一个写者；
  `perConversation: true` 时 state key 退化为每个对话的 fingerprint（隔离/调试用）。
- 同一 `(qid, nonce)` 的显式句柄重试判分**幂等**：返回第一次结果，历史只 +1、θ 不变。
  （不带句柄的旧式字母作答保持原语义：每封答案判当前题。）

### I4 模型不得产出题目

- 慢路径转发前若「当前有未答选择题」，会监控上游回复：出现「题干 + ≥2 行
  `A./B./C./D.` 选项」的保守结构 ⇒ **整段丢弃**，改回 engine 当前题，
  日志 `guard:model-question`，响应头 `x-quiz-guard: model-question`。
- 只有有未答题时生效；流式慢回复此路径会先缓冲再判定（正确性优先）。
- 判定刻意保守（至少两个独立选项行 + ≥8 字非选项文字），普通解释里的列表不会误拦。

## 卡片模式（真卡片插件 `@local/dsh-learn-card`）

**前置**：DSH 已安装并 applied `@local/dsh-learn-card`（工具名 `learn_card`，
请求的 `tools` 里会出现）。代理用 `QUIZ_PROXY_CARDS=learn` 启动才启用；默认 `off`
（纯文本快路径，安全档）。`QUIZ_PROXY_CARDS=1` 保留旧的 `ask_user_question` 点选卡片（legacy）。

```bash
QUIZ_PROXY_CARDS=learn ./start.sh     # 启用 learn_card 真卡片
./start.sh                            # 默认：纯文本快路径
```

事件序列（`learn_card` 是**只读展示**工具：不点选、不判卷，用户直接在输入框回字母）：

```text
① 用户：「开始做题」
   ← assistant：content=""，
     tool_calls=[learn_card({
       kind: "lesson" | "question",
       payload: { chapter, unit, unitNo, unitTotal, questionNo, questionTotal,
                  stem, plain?, options: [{key:"A", text:"<选项原文>"}, …] } })]
   DSH 执行该工具 → 聊天里渲染真卡片（教学环节 / 选择题）

② DSH 执行完会再请求一次模型，最后一条是 {role:"tool", tool_call_id:"call_…"}
   ← 代理按 tool_call_id 匹配前面 assistant 的 learn_card 调用认领，
     直接回 content=""、tool_calls:[]、finish_reason:"stop"
     —— 绝不转发慢路径（否则每张卡后面都会多一条慢模型回复、还要花 key）

③ 用户在输入框回「A/B/C/D」
   ← 本地 engine 判卷：content="✅ 对了！/ 差一点 + 正确答案 + 解析 + 出处"
     同一条响应再发下一张 learn_card（新单元的 lesson / primer 知识卡会先出卡，
     用户回「继续」或「开始做题」再出题）
```

payload 形状（照 card-dev README §Exact proxy change spec，逐条对齐）：

- **question**：`{chapter, unit, unitNo, unitTotal, questionNo, questionTotal, stem, plain?,
  options:[{key:"A", text}, …]}`；`unitNo/questionNo` 来自 engine `next` 的 `position`
  （`unitIndex/questionIndex`），`stem` 是题干原文，`options` 不带 A/B/C/D 前缀。
- **knowledge**：`我不懂 / 讲讲 / 解释一下` → engine `explain` 的 card 对象原样透传
  （客户端认 `oneLiner` / `definition.code` / `sections[].items` 等别名）。
- **lesson**：engine `next` 的 lesson 对象原样透传（`goal` / `knowledgePoints[].title` /
  `intuition`）；教学环节先出卡，题目排在「继续」之后。

要点：

- **正文留空**：题目/知识都在 `payload` 里，`content` 只放判卷反馈（避免和卡片重复）。
- **回显认领靠 `tool_call_id`**，不解析 tool result 的 content（那只是 Host `render()` 的一行说明）。
- **回退**：`tools` 里没有 `learn_card` ⇒ 自动退回纯文本快路径；state 里没有 issued 题目时
  「不懂」给文本引导；简答/证明题仍走文本（learn_card 只渲染选择题）。
- 日志：`route=card:*`、`route=card:echo`、`cards=learn learn=1`、`card echo call=…`；
  `/health` 的 `cards` / `cardsMode` 可确认开关。

> 兼容性已验证（DSH 实际使用的 pi-ai `openai-completions` 客户端）：
> R1 `stopReason=toolUse`、`learn_card` 参数可解析（lesson payload 完整）；
> R2 工具回显 → `stopReason=stop`、`content=[]`（没有多余模型回复）；
> R3 用户「继续」→ 下一张 `question` 卡片，payload 的
> `unitNo/unitTotal/questionNo/questionTotal` 齐全。见 `.cache/quiz-proxy/test/` 与交付报告。

## DSH 接入

1. 先启动本服务：`cd .cache/quiz-proxy && ./start.sh`，确认
   `curl -s http://127.0.0.1:3090/health` 里有 `"ok":true`。
   要用真卡片：`QUIZ_PROXY_CARDS=learn ./start.sh`（需要 DSH 已 applied `@local/dsh-learn-card`）；
   不装插件时保持默认，代理自动走纯文本快路径。
2. 在 DSH profile 的 `cordis.patch.yml`（本机是
   `~/.dsh/profiles/web/cordis.patch.yml`）里，找到 **`llm-pi-ai`** 条目，
   在它的 `config.providers` 下**加一条**（`opencode-go-6` 原样保留）：

   ```yaml
   - id: llm-pi-ai
     name: "@deepseek-ai/dsh-llm-pi-ai"
     config:
       providers:
         opencode-go-6:
           # ……原有配置保持不动……
         local-quiz:                       # ← 新增一条 provider
           displayName: 本地题库（快路径）
           apiKeyEnv: OPENCODE_GO_API_KEY_6  # 复用已有凭证；本地代理不校验它
           api: openai-completions
           baseURL: http://127.0.0.1:3090/v1
           models:
             - id: quiz-auto
               contextWindow: 1000000
               maxTokens: 32000
               # 可选但推荐：把 opencode-go 的思考配置抄过来，
               # 慢路径转发出去的请求就与以前直连 opencode-go 完全一致。
               reasoningEfforts:
                 off: null
                 low: low
                 high: high
                 max: max
               compat:
                 thinkingFormat: deepseek
                 maxTokensField: max_tokens
                 requiresReasoningContentOnAssistantMessages: true
   ```

   > 说明：`llm-pi-ai` 的 provider 配置里**没有 `apiKey` 字段**，它只认
   > **`apiKeyEnv`（凭证引用名）**。最省事的是复用你已经有的
   > `OPENCODE_GO_API_KEY_6`（`~/.dsh/.credentials.yaml` 的 `refs:` 里已经有它）；
   > 本地代理对请求里的 `Authorization` 不校验，慢路径会用自己解析到的 key 转发。
   > 想更干净也可以新建一个凭证（在 **设置 → 模型** 里添加 provider 时，API Key
   > 随便填 `local`，UI 会自动生成 `apiKeyEnv` 并写进 `credentials.yaml`）。

3. 想让它成为默认模型，再改 `agent-default-model` 条目：

   ```yaml
   - id: agent-default-model
     name: "@deepseek-ai/dsh-agent-default-model"
     config:
       provider: local-quiz        # 原来是 opencode-go-6
       model: quiz-auto            # 原来是 deepseek-v4.1-flash
       reasoningEffort: max        # 与原来一致（quiz-auto 声明了 max 档；
                                   # 快路径忽略它，慢路径按它思考）
   ```

4. **重启 DSH Web**（改 patch 后要重启才生效；也可以不重启，直接在
   **设置 → 模型** 里把默认模型切到 `local-quiz / quiz-auto`）。
   重启后先问一句「开始做题」：应当在 **< 1s** 内回一道题；问别的问题仍走 opencode-go。

> 提示 1：opencode-go 的慢路径要求请求带 `x-opencode-session` 头（缺失会 400：
> `Request is missing x-opencode-session…`）。本代理默认补
> `x-opencode-session: quiz-proxy`；若 DSH 的 provider 配置里写了
> `headers.x-opencode-session`，会原样转发过去（可用环境变量
> `QUIZ_PROXY_OPENCODE_SESSION` 覆盖默认值）。
>
> 提示 2：DSH 的 `agent-default-model` 只是起点；在 GUI 里随时可以切回
> `opencode-go-6 / deepseek-v4.1-flash` 做慢思考（下次切回来时题库状态还在）。
>
> 提示 3：如果「快路径 + 慢路径混合」的对话里慢路径报 400 且信息与
> `reasoning_content` 有关，把 `compat.requiresReasoningContentOnAssistantMessages`
> 这行删掉再试（快路径回复没有思考内容，某些上游对空串比较敏感）。

## 分流规则（model = `quiz-auto` 时）

| 最后一条 user 消息 | 行为（无工具声明 / 卡片模式） |
|---|---|
| `A` / `B` / `C` / `D`（大小写、两侧空白都行） | 快路径判卷 → 正确答案 + 解析 + 出处 + **自动附下一题**（卡片模式：下一题作为卡片 tool_call 发出） |
| `开始做题` / `练几题` / `出题` / `下一题` / `继续` / `再来一题` / `next` | 快路径出题（新对话新建 state；已有对话推进一题；卡片模式发卡片） |
| `不懂` / `讲讲` / `解释一下` / `知识卡` … | 快路径给**知识卡文本**（`engine explain`，不判卷、不换题；卡片模式下再附一张原题卡片） |
| 最后一条是 `role=tool` 且 `id` 前缀 `quiz-` | **卡片被点了**：按 `selected` 的 label 判卷 → 反馈文本 + 下一张卡片 |
| 最后一条是 `role=tool` 但 `id` 不是 `quiz-` | 慢路径：原样转发 opencode-go（其它工具的结果） |
| 其它任何文本 | **慢路径**：原样转发 opencode-go |
| `model` 不是 `quiz-auto` | 慢路径（该请求不参与分流） |

- 快路径只认**最后一条** user 消息；状态按 fingerprint 隔离。
- fingerprint = `sha1(system 内容 + "\n" + 第一条 user 内容)`：DSH 每次都会带完整历史，
  所以同一对话始终落到同一个 `state/<fp>.json`；新对话天然隔离。
- 同一 fingerprint 的并发请求**串行处理**（内部锁），不会互相踩状态。

## 环境变量

| 变量 | 默认 | 说明 |
|---|---|---|
| `QUIZ_PROXY_PORT` | `3090` | 监听端口 |
| `QUIZ_PROXY_HOST` | `127.0.0.1` | 监听地址（**不要**改成 0.0.0.0） |
| `QUIZ_PROXY_REPO` | 自动向上查找 | 仓库根（须含 `courses/set-theory/knowledge/engine.py`） |
| `QUIZ_PROXY_STATE_DIR` | `<repo>/.cache/quiz-proxy/state` | 题库状态目录 |
| `QUIZ_PROXY_UPSTREAM` | `https://opencode.ai/zen/go/v1/chat/completions` | 慢路径上游 |
| `QUIZ_PROXY_UPSTREAM_MODEL` | `deepseek-v4.1-flash` | `quiz-auto` 映射到的上游模型名 |
| `QUIZ_PROXY_OPENCODE_SESSION` | `quiz-proxy` | 慢路径默认 `x-opencode-session` 值 |
| `QUIZ_PROXY_CARDS` | `0`（off） | `learn` = learn_card 真卡片；`1` = 旧 ask 点选卡片（legacy）；其它/默认 = 纯文本 |
| `QUIZ_PROXY_MAX_BODY_BYTES` | `8388608`（8MB） | 请求体上限 |
| `QUIZ_PROXY_UPSTREAM_TIMEOUT_MS` | `120000` | 慢路径上游超时 |
| `QUIZ_PROXY_ENGINE_TIMEOUT_MS` | `30000` | python 引擎子进程超时 |
| `QUIZ_PROXY_CREDENTIALS` | `~/.dsh/.credentials.yaml` | 读 key 的文件（**只读**） |
| `OPENCODE_GO_API_KEY_6` | — | key 优先从环境变量取；没有才解析 credentials 文件 |

## 测试

```bash
cd .cache/quiz-proxy
node --test                                  # 自动发现 test/*.test.mjs（62 个用例）
node --test test/quiz-proxy.test.mjs          # 或指定文件
```

测试用本地 mock 上游（不联网、不花 key），覆盖：快路径出题/判卷/自动下一题、
「不懂」知识卡、无效字母与未知消息走慢路径、共享学习流与并发串行锁、快路径 SSE、
慢路径 JSON/SSE 透传（含 model 映射、Authorization、x-opencode 头）、上游错误映射、
坏 JSON / 缺 messages / 请求体超限 / 缺 key / 上游超时、credentials 解析（断言日志不泄露 key），
以及**卡片模式**：legacy `ask_user_question` 点选卡片全套（出卡/判卷/幂等/回退），
和 **learn_card 真卡片**：`hasLearnCardTool` 判定、tool_call_id 回显认领（不解析 content）、
lesson/question 出卡与 payload 形状（stem/options key/position 字段）、knowledge 知识卡、
回显 → 空 content + stop + 不访问上游（流式/非流式）、字母判卷 + 下一张卡片、
默认 off 纯文本回归、`QUIZ_PROXY_CARDS` 模式解析；
以及**正确性不变量**：出题响应带 `x-quiz-qid`/`x-quiz-nonce`、显式句柄判分、
nonce 不匹配不记分（响应头 `x-quiz-stale`）、代理句柄漂移不记分、同一 `(qid,nonce)`
重复判分幂等（历史 +1、θ 不变）、per-state-key 隔离、慢路径「模型自作题」被拦
（流式/非流式）并改回引擎题。

## 排错

| 现象 | 排查 |
|---|---|
| `start.sh` 提示端口占用 | `lsof -nP -iTCP:3090 -sTCP:LISTEN`；或换 `QUIZ_PROXY_PORT` |
| `/health` 通、问做题没反应 | 确认消息文本完全落在「分流规则」里；看 `run/quiz-proxy.log` 的 `fast ...` / `slow ...` 行 |
| 慢路径 401/403 | key 过期或无权限：检查 `~/.dsh/.credentials.yaml` 的 `OPENCODE_GO_API_KEY_6`（不要打印内容） |
| 慢路径 400 `missing x-opencode-session` | 设置 `QUIZ_PROXY_OPENCODE_SESSION`（或给 DSH provider 配置 `headers.x-opencode-session`） |
| 快路径报 `engine_error` | 先手动跑 `python3 courses/set-theory/knowledge/engine.py next --state /tmp/q.json` 看引擎是否正常 |
| 聊天里看不到真卡片（还是文本题目） | 看 `/health` 的 `cardsMode` 是不是 `off`（默认；用 `QUIZ_PROXY_CARDS=learn ./start.sh` 开启）；看日志 `route=` 行里的 `learn=1` —— `learn=0` 说明本次请求 `tools` 里没有 `learn_card`（插件未 applied / 该请求来自无工具子 agent）；`learn=1` 但 `route=slow` 说明消息没被判定为做题 |
| 每张卡后面都多一条慢模型回复 | 正常不该有：看日志是否有 `route=card:echo`；没有说明回显走了慢路径（旧代理 / 没识别 tool_call_id），工具回显必须回空 stop |
| 点卡片后提示工具错误/无反应 | learn_card 是只读展示卡（不点选）；用户应在输入框回字母。若工具报 schema 错误，检查 payload 形状（`{kind, payload}`） |
| 判分提示「这道题已更新，请按屏幕上的题重答」 | 正常防线：屏幕上的题与 engine 当前 issued 题不一致（换题/并发/模型自作题）。按提示重答即可；响应头 `x-quiz-stale:1` |
| 日志出现 `guard:model-question` | 慢模型自己出了道题，被代理拦下并改回引擎题（正常防线，不记分） |
| 想重置某个对话 | 删掉 `state/default.json`（共享学习流；fp 见日志里的 `fp=xxxxxxxx`） |

## 已知限制

- 快路径只处理**选择题字母**与几个出题口令；简答/解释/证明题会被提示「直接写回答」，
  随后由慢路径的 opencode-go 接手（不做本地判分）。
- 不做工具调用（tools / function calling）转发加工；慢路径为纯透传。
- 状态目录按 fingerprint 累积，长期使用可定期清理 `state/`（不影响 DSH 对话历史）。
- 本服务只做本地中转：无鉴权、无 TLS，**只应监听 127.0.0.1**。
