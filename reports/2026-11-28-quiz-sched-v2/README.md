# 快思考调度 v2：单元连续 3 题 + 三题探针 —— Before / After 证据

issue #138。**只改调度与展示**：内核判定与题库内容不变。

## 结论

同一份 seeded 学习者状态（真实 state 的字节副本，113 次作答、校准已完成）、
同一 1280×813 视口、浅色主题、中文 UI、同一操作脚本（新建会话 →
`/sokonanoda-fast-think on` → `进度`；**只读**，不答题）：

| 场景 | Before（`origin/main` @ `956e4fb0`） | After（本分支） |
|---|---|---|
| 进度卡「学习地图」行 | `已解 54 · 待做 41 · 薄弱 1 · 代码 0/916` | **`已解 17 · 探针中 37 · 待做 41 · 薄弱 1 · 代码 0/916`** |
| `engine.py map` 顶部行 | `知识点：已解 54 · 待做 41 · 薄弱 1（共 195，96 个已有题）` | `知识点：已解 17 · 探针中 37 · 待做 41 · 薄弱 1（共 195，96 个已有题）` |
| 作答 1–2 次的 KP（37 个） | 算作「已解」（一题对就算） | 记「探针中」，既不算已解也不算薄弱 |

数字是**重分类**而不是重新作答：`before solved 54 == after solved 17 + after probing 37`；
`covered`（55）、`coverage`（0.573）、`todo`（41）、`weak`（1）、`total`（195）在两边**完全一致**，
这 37 个 KP 的正确率都 ≥ 0.6（旧规则下都算「已解」）。卡片的摘要数字（连对 9 /
正确率 93% / 覆盖 59% / 已答题 113）两边也一致 —— 差异只来自本次改动。

**Before**（`已解 54 · 待做 41 · 薄弱 1`，没有「探针中」）

![before](before-progress.png)

**After**（`已解 17 · 探针中 37 · 待做 41 · 薄弱 1`）

![after](after-progress.png)

## 环境与流程

- **Before** = `origin/main` @ `956e4fb0`（`/tmp/qsv2/before-worktree`，detached、clean）；
  **After** = 本分支工作树（`agent/quiz-sched-v2-80289adfe726`，运行时文件的 sha256 记在
  `after-verify.txt` §1）。两边跑**同一套**插件/引擎代码路径，只有版本不同。
- 两个**隔离** DSH Web 实例：各自的 `DSH_HOME`、`SOKO_DSH_DATA`（quiz 状态目录）、
  端口 3987/3988，各自独立 headless Chrome（CDP 9334/9335）；`dsh plugin --profile web add`
  把 `sokonanoda-dsh` 指到各自的 worktree。
- 状态种子：真实学习者状态 `/Users/penglingwei/.dsh/state/sokonanoda-dsh/quiz/default.json`
  的字节副本（sha256 `f90f2a07…c6b2`），两边各自拷一份（`/tmp/qsv2/{seed,before-data,after-data}`）。
  采集前后种子 sha256 不变（`after-verify.txt` §2）—— 流程没有作答、没有写状态。
- 截图是**真实运行的 DSH Web**（headless Chrome 经 CDP 操作 composer），不是 mock；
  数字来自真实课程引擎（`engine.py map`，与聊天里「学习地图」同一份快照）。

## CLI：同一份 `map` 数据

Before（`/tmp/qsv2/before-worktree` 的 `engine.py`）：

```text
🗺️ 学习地图 · 共 39 个单元，稳了 0 个
知识点：已解 54 · 待做 41 · 薄弱 1（共 195，96 个已有题）
I.1  ◐ ◐ ◐ ◐   （0/4 单元稳 · 已解 16 · 待做 0 · 薄弱 1 · 代码 0/115）
…
你现在在：I.9 ㉓ 广义积与柯里化（本单元已答 0 题，接触 0/10 个知识点）
```

After（本分支）：

```text
🗺️ 学习地图 · 共 39 个单元，稳了 0 个
知识点：已解 17 · 探针中 37 · 待做 41 · 薄弱 1（共 195，96 个已有题）
I.1  ◐ ◐ ◐ ◐   （0/4 单元稳 · 已解 8 · 探针中 8 · 待做 0 · 薄弱 1 · 代码 0/115）
…
```

完整输出见 `before-map.txt` / `after-map.txt`；逐 KP 不变量见 `after-verify.txt` §5
（195 行分区自洽：17+37+1+41+99 = 195；`probing` 行的 `asked` 只出现 1、2；
`solved` 行按有效门槛 `min(min_probes, 可服务题数)` 无一违规；唯一的「asked=2 的 solved」
是 `kp.i1.univ` —— 只有 2 道可服务题，做满即判，属于 issue 的「题目不足 3 题不卡死」边界）。

## 判据 B：真实 state 回放（调度侧）

`replay-verify.py` 在种子状态的**副本**上连续 10 次 `next`（每次答对后 advance），
输出在 `replay.txt`：

```text
units: ["⑳ 基数算术：积", "⑳ 基数算术：积", "⑩ 基数初步", "⑩ 基数初步", "⑩ 基数初步",
        "⑧ 像与原像", "⑧ 像与原像", "⑧ 像与原像", "序数预备", "⑮ 传递集与序数"]
three_same_then_switch: [{"start": 3, "unit": "⑩ 基数初步", "next_unit": "⑧ 像与原像"},
                         {"start": 6, "unit": "⑧ 像与原像", "next_unit": "序数预备"}]
map_kps: {"total": 195, "withQuestions": 96, "covered": 57, "coverage": 0.594,
          "contentGaps": 99, "solved": 22, "probing": 34, "weak": 1, "todo": 39}
solved_with_asked_below_min: []
probing_outside_0_asked_min: []
B_CRITERIA_PASS: True
```

- **连续 3 题同单元、第 4 题才换**：从第 3 步进入 `⑩ 基数初步` 起（steps 3–5），第 6 步换到
  `⑧ 像与原像`；`⑧` 同样 3 题后（steps 6–8）第 9 步换。判据成立两次。
- 前两步仍落在 `⑳ 基数算术：积`：该单元在真实历史里**已做满 3 题** ⇒ 立即放开（v2 的
  「做满即放开」语义），所以判据窗口从「进入新单元那一步」起算；这不是回退，是设计。
- `map.kps.solved` 不再包含作答不足的 KP，`probing` 计数出现（22/34/1/39）。
- 边界：`kp.i1.univ`（可服务题 2 道）做满 2 题即判 solved —— 有效门槛
  `min(min_probes, 可服务题数)`，即 issue「单元/题目不足 3 题时做满现有题即走，不卡死」。

## 限制（如实记录）

- **存量状态的 `probes.*.asked`**：v1 在**发题时**累加，v2 改成**作答时**累加。升级后存量
  会话的探针结案门槛会读到偏大的旧值，最坏情况比 3 题早 1–2 题结案；「探针中/已解/薄弱」
  只读作答历史（`history`），不受影响；新会话语义正确。这是有意的轻量迁移，不做状态重写。
- **跨单元 KP**：一个 KP 可以出现在多个单元（如 `kp.prep.ordinal`）。锁定单元里没有未作答
  的可出题时，`unit_probe_kp` 会放开（pending 检查），不会死循环；代价是这种 KP 的下一题
  可能落在另一个单元 —— 这是「单元里没题可出」时的合理让路。
- 探针的「单元连续」按该单元**总作答数**（跨会话累计）判断；已经在历史里做满 3 题的单元
  会立即放开（见 B 的前两步），不额外补 3 题。
- Before/After 的会话标题由 DSH 自动生成，与本次改动无关；比较的是同一位置的进度卡内容。

## 文件

| 文件 | 内容 |
|---|---|
| `before-progress.png` / `after-progress.png` | 真 DSH Web 进度卡截图（1280×813） |
| `before-card.txt` / `after-card.txt` | 截图里进度卡的可见文字（含精确「学习地图」行） |
| `before-map.txt` / `after-map.txt` | 同一 seeded state 的 `engine.py map` 全量输出 + 环境/校验和 |
| `after-verify.txt` | After 的 provenance、状态未变、逐 KP 分区与不变量 |
| `replay-verify.py` / `replay.txt` | 判据 B：真实 state 副本回放脚本与读数 |
