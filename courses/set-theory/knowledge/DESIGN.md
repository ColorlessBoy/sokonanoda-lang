# 自适应学习系统 · 设计（三层架构）

> 目标：把「出题—判卷—讲解—进度」做成**不依赖模型延迟**的做题软件；
> 模型只做低频维护。数据固化在 `knowledge/`，交互由算法驱动。
>
> 状态：**v1 已批准，P0 进行中**（2026-11-21）。实现进度见 §7。
>
> **P0 已完成部分**：数据 schema 扩展（difficulty/variants/prereq/tags）、
> `data/params.json`、`engine.py`（next/answer/state/selftest/simulate）、`validate.py` 扩展、
> `review.py`（L3）、`README.md`。证据：`validate.py` 全绿；`engine.py selftest` 8/8 PASS；
> `engine.py simulate` 输出 `reports/eval-*.json`；CLI 往返烟测通过（5 题，去重、判卷、进度）。
> **P0 题库扩充已完成并独立复核**：27 题并入 → **72 题（I.1 26 / I.2 26 / I.3 20，L1 21 / L2 38 / L3 13）**；
> 出处逐条核验（33 个 lib 名字全部存在、行号落在真实声明行）、validate 全绿、engine selftest 全绿。
> **待办**：P1 面板与 Host 接线（PM）。

## 0. 非目标

- 不做通用题库平台；只服务卷 I 集合论（`courses/set-theory/`）与后续扩展卷。
- 不让模型出现在每次作答的回路里（这是过去"半小时一题"的根因）。

## 1. 三层总览

```
┌──────────────────────────────────────────────────────────────┐
│ L1 交互层（0 延迟，无模型）                                  │
│  右栏「学习」面板 / 聊天流式卡：一张一卡、点选即判卷、        │
│  进度条、连对、错题本、不会时插入简版知识卡                   │
│  数据：knowledge/data/*.json（题库+知识卡）                   │
└───────────────┬──────────────────────────────────────────────┘
                │ 作答事件（异步，不阻塞 UI）
┌───────────────▼──────────────────────────────────────────────┐
│ L2 调度层（确定性算法 / 低思考 agent，毫秒级）               │
│  掌握度模型 θ（每 tag）、召回打分（盲区/挑战度/新题/覆盖）、  │
│  去重与覆盖约束、遗忘曲线复习、知识卡触发、可解释决策         │
└───────────────┬──────────────────────────────────────────────┘
                │ 日志（作答序列 + 决策 + mastery 轨迹）
┌───────────────▼──────────────────────────────────────────────┐
│ L3 维护层（高思考 agent，低频：内容迭代 / 参数校准 / 评测）   │
│  题库与知识卡维护、难度标注校准、离线模拟评测、                │
│  版本化与回滚、bad case 回流、设计文档更新                    │
└──────────────────────────────────────────────────────────────┘
```

**延迟预算**：L1 判卷 < 16 ms（本地）；L2 选下一题 < 50 ms（Host 内存）；
L3 离线（分钟级，不在用户路径上）。

## 2. L1 交互层

### 2.1 形态

- **DSH 右栏「学习」页签**（client half）：一卡一屏，触控优先（44px），深/浅色；
- 题目卡：题干（行内代码渲染）+ A–F 选项；点选后**立刻**：正确项绿框 ✓、错选红框 ✗、
  显示「答案 B · 文本」+ 解析（`why`）+ 出处（`source`）+ 「下一题」；
- 知识卡（玩家不会时）：同一容器内插入**简版知识卡**（标题 + 一句话 + 定义 + 例子 + 误区），
  下方「我懂了，继续」→ 接一道**同 tag 变式**；
- 进度条：本轮已答/总数；顶部「掌握概览」可折叠（各 tag 的 θ 条）；
- 错题本：本轮/累计错题列表；「复制成绩单」导出文本（粘回聊天给老师复盘）；
- 聊天内可选：一轮结束发一张摘要卡（得分/盲区/下一站），不参与作答。

### 2.2 正向激励（学习多邻国，2026-11-21 用户拍板）

- **不显示正确率 / 得分**：顶部只放正向指标——已答 N 题、🔥 连对、🧩 覆盖知识点、
  📚 章节完成率、🎉 里程碑（连对 3/5/10、新章节、掌握 tag +1）；
- **GitHub 风格绿色热力图**：按章节分行，格子颜色 = 该章节各知识点掌握度的
  **智能推测**（0–4 级），标注「智能推测，仅供参考」；逐格错峰淡入、掌握度提升时脉冲；
- **错题本弱化为「再练练」**：可展开的小入口，不报分、不占焦点；
- **答错反馈鼓励化**：不出现「错/失败」字样，用「再试试 / 差一点」；答对加庆祝动画；
- **动画（多邻国式）**：卡片滑入回弹、选项按压缩放 + 正确 pop、进度条 spring、
  连对庆祝（轻量 confetti，40 个绝对定位方块，禁止 canvas 每帧重绘）、
  知识卡下滑入、热力图点亮；全部只用 transform/opacity，尊重 `prefers-reduced-motion`。
- **热力图语义（2026-11-22 修）**：`touched` 只由 **history 的 tags** 决定（未作答的 tag
  不算接触；`theta_of` 读时不写 state——否则发出卡就会"提前点亮"）；未接触 = level 0 灰，
  图例来自引擎 `heatmap.legend`；每行显示 `已接触 X/Y · 已答 N`；答完**立即**刷新 state
  （5s 轮询只兜底），level 变化的格子脉冲一次。

数据来源：`/dsh-learn/state` 的 `positive` / `coverage` / `heatmap` 字段（见 §5.1）。

### 2.3 问答题（choice / short / explain / proof，2026-11-22）

- `kind` 字段：`choice`（选项，客户端本地判）、`short`（短答）、`explain`（解释）、`proof`（证明）；
- 开放题数据：`prompt` + `rubric {minScore, points:[{id,weight,any[],note}]}` + `reference` + `why` + `source`；
- **判卷三路**：短答/解释 → rubric 关键词自动判（确定性、毫秒级、零模型；命中要点、漏点与参考答案都回显）；
  未达标的解释题 → 进**批改队列**，由**低思考 grader agent** 异步复核（pass/partial/fail + 一句评语），
  客户端「老师批改中…」30s 轮询；`proof` → 内核判卷（复用 `scripts/soko` 练习通道，后续阶段）；
- 开放题也是正向口径：不显示分数/正确率，只显示要点命中与「老师评语」。

### 2.4 判卷与数据

- 答案随题下发（题目数据里有 `answer/why`），**客户端即时判卷**；
- 作答事件异步 `POST /dsh-learn/answer`（幂等：`sessionId + questionId + seq`），失败重试，不阻塞 UI；
- 离线可用：整个题库随面板下发，断网也能刷题（联网后补事件）。

## 3. L2 调度层（推荐系统式）

### 3.1 掌握度模型

每个知识标签 `t`（题目的 `tags` 与知识卡一一对应）维护能力值 `θ_t ∈ [-3, 3]`：

```
p = σ(θ_t − d_q)          # 预期答对概率（d_q = 题目难度，同量纲）
θ_t ← clamp(θ_t + η·(y − p))   # y∈{0,1} 实际对错；η=0.3；答错降、答对升
m_t = σ(θ_t)              # 掌握度 ∈ (0,1)
```

- 题目难度 `d_q`：题目数据显式标注（L1/L2/L3 → −1.5/0/1.5，维护层校准）；
- 初始 `θ_t = 0`（m=0.5）；**连续答对 3 次**或 `m_t > 0.85` → 判"已掌握"，该 tag 降权。

### 3.1b 开局校准与"刷新不变题"（2026-11-22）

- **不随机**：选题并列时按 id 排序取第一（确定性）；session 重新打开不会换题——
  `next` 默认 **resume**（返回已发出且未作答的卡），`--advance` 才推进；`state.issued_qid/issued_card` 记录在案；
- **校准阶梯**：新会话前 6 题走固定阶梯（I.1 L1→L3、I.2 L1→L3、I.3 L1→L3，`reason='calibration'`），
  快速定位水平；之后进自适应（盲区/最近发展区/复习/变体/去重）；客户端显示「校准中 x/6」；
- 引擎会话状态里 `calibration.done` 一旦为真不再回退。

### 3.2 召回打分（选出下一题）

候选 = 题库中满足 `chapter 过滤` 且未在最近 N 题出现；对每题打分：

```
score = w_gap·(1 − m_{tag})            # 盲区优先：掌握度最低的 tag
      + w_chal·(1 − 2·|p − 0.5|)       # 最近发展区：p≈0.5 的信息量最大
      + w_new·[未见过]                  # 新题召回
      + w_cov·[章节覆盖率不足]          # 覆盖约束
      − w_rec·[最近 N 题重复]           # 去重（硬约束：最近 20 题不重复）
```

权重初值 `w_gap=0.45, w_chal=0.35, w_new=0.10, w_cov=0.10, w_rec=∞`（硬排除）；
维护层可调（参数固化在 `knowledge/data/params.json`，带版本号）。

**答错后的特殊规则**：同一 tag 连续错 2 次 → 强制插入该 tag 的知识卡；
答错后优先同 tag 的**变式题**（`variants` 组），再回到主队列。

### 3.3 遗忘与复习

- `m_t < 0.75` 且距上次作答 `> interval(m_t) = 1 + 2·(1 − m_t)` 天 → 该 tag 的题进入"复习队列"优先；
- 复习队列与盲区队列交替（避免一直刷不会的）。

### 3.4 可解释与可复现

- 每次调度输出 `decision = {qid, reason, m_snapshot, p}`；`reason ∈ gap|challenge|review|coverage|new|variant`；
- 引擎是**纯函数**：`next(state, data, seed) → (question, decision)`；同输入同输出（测试可钉死）。

## 4. L3 维护层（高思考 agent = 老师 / 维护者）

职责（全部产物进 `knowledge/`，版本化）：

| 职责 | 产物 | 频率 |
|---|---|---|
| 题库维护：补题、修题、难度校准、变式组 | `data/questions.json` + `version` | 迭代期 |
| 知识卡维护：覆盖缺口、前置关系、简版卡 | `data/cards.json` + 图谱边 | 迭代期 |
| 参数校准：η、权重、阈值 | `data/params.json` | 每周/每批 |
| 质量门：答案核验、重复检测、覆盖报告 | `validate.py` + `reports/` | 每次改数据 |
| 离线评测：模拟学习者（弱/中/强）跑引擎 | `reports/eval-*.json` | 每次参数变更 |
| bad case 回流：从 L2 日志挑错题/低效调度 | 题目/参数 diff | 每轮 |

**L3 不看屏幕、不看单次作答**——它读 L2 的日志摘要（JSONL → 聚合报告），做低频决策。

## 5. DSH 插件适配

复用现有 bundle 骨架（`@local/dsh-whiteboard` 的 Host 半已经证明可行）：
新增**独立命名空间** `learn_*`，与 `board/exercise/quiz` 隔离（互不影响、可单独关）。

### 5.1 Host 半（index.js 新增段）

| 类型 | 名称 | 作用 |
|---|---|---|
| 工具 | `learn_next` | 取下一张卡（题目/知识卡 + 判答数据 + 决策 explain） |
| 工具 | `learn_answer` | 落账一次作答（幂等），更新 θ，返回 mastery 快照 |
| 工具 | `learn_state` | 进度 / 掌握概览 / 错题本 / 到期复习 |
| 工具 | `learn_export` | 导出本轮/累计 JSONL（给 L3 agent 复盘） |
| 工具 | `learn_bank` | 审批后的题库/知识卡/参数批量更新（L3 专用，写文件） |
| 路由 | `GET /dsh-learn/next` · `POST /dsh-learn/answer` · `GET /dsh-learn/state` · `GET /dsh-learn/export` | 同源守卫 + JSON + no-store |

状态持久化：`~/.dsh/learn/<sessionId>.json`（env `DSH_LEARN_DIR` 覆盖），
原子写（temp + rename），坏文件降级为空态并报 warning。

### 5.1b Agent 感知（**核心要求**，2026-11-21 用户拍板）

DSH 插件必须让 **agent 能感知学习系统的全部状态**（不只是 UI 能看见）：

| 通道 | 覆盖 |
|---|---|
| `learn_state` 工具 / `GET /dsh-learn/state` | progress（已答/连对/错题）、mastery（每 tag θ）、**positive**（正向指标）、**coverage**（知识点/章节覆盖）、**heatmap**（章节×知识点掌握度推测，0–4 级）、**history_tail**（最近 20 次作答：qid/choice/correct/decision/prompt/why/tags）、wrong 账本、pending_card、last_decision |
| `learn_export` 工具 / `GET /dsh-learn/export` | `soko.learn-log/1` JSONL：meta + 逐答（含决策原因），给 L3 `review.py` 复盘 |
| 状态文件 `~/.dsh/learn/<sessionId>.json` | 唯一真相：θ/last_at/history/seen/wrong/streak/cards_shown/pending_card；原子写，坏文件降级空态 |

⇒ 客户端 UI 与 agent 读的是**同一份状态**；agent 讲解/复盘前先 `learn_state`，
拿到用户答过什么、错什么、覆盖到哪、掌握度推测多少——无需用户复述。

### 5.1c 术语表与"我不懂"（2026-11-22）

- `data/glossary.json`（15 条：Type/Prop/Set/Rel/Prod/函数/Type.Equiv/Cardinal/Nat/True-False/
  Injective/Surjective/IsOrdinal/IsSuccOf/∅）；Host `GET /dsh-learn/glossary` 下发，客户端「📘 术语表」
  搜索+展开（term/一句话/code/source）；
- 「我不懂」按钮**不放界面**（用户偏好：直接在聊天里问）；`POST /dsh-learn/explain` 保留给
  agent/聊天复盘，按题目 tags 匹配知识卡，fallback 会列出本题涉及的类型/术语（来自 glossary）。

### 5.2 Client 半（client.js）

- 新增 `kind='learn'` 右栏页签 + guide 卡「学习」；
- 一卡一屏，点选即判（答案在题卡里，本地判）；
- 作答事件异步上报（失败静默重试），UI 永不等待网络；
- 进度/掌握从 `/dsh-learn/state` 拉，5s 轮询 + 手动刷新。

### 5.3 测试与验收

| 层 | 判据 |
|---|---|
| 引擎 | `python3 knowledge/engine.py --selftest`：确定性、去重、盲区优先、变式组、复习到期 5 条断言 |
| Host | `node test/run.mjs`：`learn_*` 工具/4 路由 e2e + 幂等 + 坏文件降级 |
| Client | smoke：页签注册、一卡一卡推进、点选即判（<50ms）、知识卡插入、进度条、错题本 |
| 真实设备 | 手机刷新 → 右栏「学习」→ 10 题（点选即时反馈）→ 进度卡显示 θ |

### 5.4 闭环测试（不依赖生产重启，2026-11-21 立）

`node .cache/dsh-whiteboard/test/closedloop.mjs`：在仓库内建隔离 `DSH_HOME`
（profile 副本 + 端口 3099），用同一个 dsh 二进制启动一次性实例，等 Host 半真正
apply 后，用真实 HTTP 跑通：bundle rev/字节一致 → fresh state（positive/heatmap）
→ next 出题 → answer 判卷+进度 → 幂等 → 连续错触发知识卡 → state 全量 →
export JSONL → 引擎状态落盘 → **启动日志无 `JsonSchemaError`/whiteboard 未激活**。
生产服务与 `~/.dsh` 不受影响，可在生产实例运行期间随时跑；这是"重启前先证明
能激活"的守卫（对应 2026-11-21 插件未激活事故）。

## 6. 数据 schema（v1 增量）

```jsonc
// questions.json 每题新增
{ "difficulty": 0.0,            // -1.5 / 0 / 1.5，维护层校准
  "variants": ["I.2.antisymm-2"], // 同 tag 变式组（可空）
  "prereq": ["I.2.rel-1"] }     // 前置（用于知识卡图谱与提示）
// cards.json 每张新增
{ "tags": ["antisymmetric"], "prereq": ["symmetric"] }
// learner-state.json
{ "version": 1, "session": "…", "theta": {"tag": 0.0},
  "history": [{"qid": "...", "choice": 1, "correct": true, "at": "...", "decision": "gap"}],
  "seen": {"qid": 1}, "wrong": {"qid": 2}, "streak": 0 }
```

## 7. 实现阶段（每阶段可验收、可停）

| 阶段 | 内容 | 验收 |
|---|---|---|
| **P0 数据与引擎** | 题库/知识卡 schema 扩展（difficulty/variants/tags）；`engine.py`（纯函数 + selftest）；离线模拟脚本 | `validate.py` + `engine.py --selftest` 全绿；模拟 3 类学习者输出 eval 报告 |
| **P1 面板** | Host `learn_*` + 路由；client「学习」页签（一卡一卡、即时判卷、进度、知识卡、错题本） | `run.mjs` 全绿 + 手机 10 题实测（点选 → 即时反馈） |
| **P2 引擎接线** | 面板走 `learn_next` 调度；去重/盲区/复习/变式生效 | 20 题序列无重复、盲区题占比上升（报告对比） |
| **P3 维护闭环** | `learn_export` + L3 复盘脚本 + 参数/题库版本化 + bad case 回流 | L3 读一份日志 → 产出 diff → 重启后验收 |

## 8. 与旧卡片的关系

- 旧的白板/练习/进度三卡**已按用户要求删除**，不再恢复；
- 本系统是**新设计**：唯一的「学习」页签；白板保留为 Host 能力（不注册 UI，老师可在聊天里发板书文本）；
- 旧 quiz 自写卡片视图不恢复；聊天问答继续用 DSH 内建问答卡。

## 9. 风险

1. DSH 聚合桥（老壳）在切换 bundle key 时**静默不投递**——P1 验收必须用**真机**点一次（见历史事故）；
2. 引擎参数漂移：所有参数进 `params.json` + 版本号，评测报告随版本存；
3. 题库覆盖不足（当前 45 题）：P0 先补到 ≥80 题（I.1–I.3 各 25+），否则"推荐"会反复召回同几题；
4. 难度标注主观：用模拟学习者 + 实际作答校准（p 偏差 > 0.2 的题进校准队列）。
