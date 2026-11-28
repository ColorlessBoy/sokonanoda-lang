# 知识卡匹配改 kp/IDF 评分：Before / After 证据（#142）

## 结论

同一份真实题库、同一个命令
`python3 courses/set-theory/knowledge/engine.py explain --state <fresh> --qid <qid>`；
**Before = `origin/main` @ `20db8dbe`**（`before-*.json`），**After = 本分支
`agent/card-scoring-491a6f48f991`**（`after-*.json`）。本 PR 改的是**引擎选哪张卡**，
渲染层（#139 已合并）没有变，所以证据是引擎原样输出的 card payload —— DSH Web 的
知识卡就是拿它渲染的。

| 题目（复现件） | Before（`20db8dbe`） | After（本 PR） |
|---|---|---|
| `I.9.product-2`（`pi` 的成员引理，tags `family/pi/projection`） | `sunion-sinter`「广义并交」❌（`family` 命中 1 分、按 `cards.json` 顺序先到） | **`set-pi-lemmas`**「Set.pi 的四个引理」✅（`matchedKps: kp.i9.product`） |
| `I.3.le-mono-1`（`≼` 的单调，tags `le/monotone`） | `fallback-card`（main 上没有对应卡）❌ | **`cardinal-le-mono`**「≼ 的搬运」✅（`matchedKps: kp.i3.le.mono`） |
| `I.9.product-ac-1`（choice 造积的成员，tags `family/pi/choice`） | `set-pi`（对的，但靠 `pi/choice` 两个标签碰巧命中） | **`set-pi`** ✅（`matchedKps: kp.i9.product.ac`，由 kp 钉住） |

- 三条 `explain` 输出原样见 `before-*.json` / `after-*.json`，读数见 `verify.txt`。
- 两张新卡（`set-pi-lemmas` / `cardinal-le-mono`）的正文取自用户 live 工作区的草稿、
  **逐字未改**；但**只带描述性标签**（`pi/product`；`cardinal-le/mono/trans/monotone`），
  没有为「把匹配拽过来」加内容标签 —— 三条用例靠 `matchedKps` 命中，正是本次引擎修复
  的目的（「不再需要内容标签绕过」）。

## 行为变化（同一份 25 张卡数据，Before 引擎 vs After 引擎）

409 道题里选中卡不同的共 **59** 道，逐类可解释：

- **18 道旧的「通用标签假匹配」→ fallback**：例如 `apply` 把 `I.1.subset-*` 配到
  「关系复合」卡、`family` 把 `I.3.image-family-*` 配到「广义并交」卡、`type` 把
  `I.3.ordinal-3` 配到「基数」卡、`quotient` 把 `II.1.addlaws-*` 配到「Quot.lift」卡。
  这些正是「打地鼠」来源；现在只带一个通用标签不接受，退回该题自己的 why 摘要。
- **28 道旧的 fallback → 有卡**：由 kp 命中或 IDF 选出的相关卡（如 `I.9.product-*`
  → `set-pi-lemmas`、`I.3.equiv-algebra-*` → `equiv`、`I.3.cardinal-*` → `cardinal-mk`）。
- **13 道卡→卡**：多数是 kp 命中的同类卡（如 `I.3.le-*`：`cardinal-le-mono` →
  `cardinal-le`）或破平规则从「文件顺序」改成「kp → IDF → id」。

## 复跑

```bash
python3 courses/set-theory/knowledge/engine.py explain --state /tmp/state.json --qid I.9.product-2
python3 courses/set-theory/knowledge/engine.py explain --state /tmp/state.json --qid I.3.le-mono-1
python3 courses/set-theory/knowledge/engine.py explain --state /tmp/state.json --qid I.9.product-ac-1
python3 courses/set-theory/knowledge/engine.py selftest
```

## 已知限制

- 本 PR 改的是**知识卡选择**，不是渲染或布局 ⇒ 证据采用「同一命令、同一输入、两边
  引擎原样输出」而非 DSH Web 截图；渲染路径与 #139 之后完全一致。
- `I.3.le-mono-1` 的 Before 证据取自 `origin/main`（该题在 main 上落 fallback）；
  线上看到的「错配到 Set.pi 四点」发生在用户 live 工作区（已带两张新卡、旧评分按
  标签计数）—— 本 PR 把这两张卡落库后，旧评分会经标签错配、新评分由 kp 命中
  `cardinal-le-mono`（`after-I.3.le-mono-1.json`）。
- 展示给学习者的卡面文本仍来自 `cards.json`；本 PR 只改选择与两张新卡的数据落地，
  不重写既有卡正文。
