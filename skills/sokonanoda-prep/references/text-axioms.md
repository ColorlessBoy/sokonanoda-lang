# 文本公理（教学表达底线）

> 判据一句话：**好表达 = 更多人能理解 × 更少时间能理解。**
> 十三条公理是底线；可机检的部分由 `courses/set-theory/knowledge/text_lint.py` 判，
> X2/X9/X10 用 `--review` 出**待人工勾选**清单。本文件是公理与判据的唯一说明；
> 脚本的误报治理、基线机制与用法都在这里。

## 1. 十三条公理

- **X1 完整**：每个表达单元是一个完整句，能判断真假；正文不以名词短语收尾。
- **X2 一事一段**：一段只讲一件事；首句是主题句，末句给结论或下一步。
- **X3 先定义后使用**：术语首次出现当场给定义或白话（"就是…"），禁止裸行话；
  出现定义记号（`:="`/`定义为`）的地方必须有白话句。
- **X4 抽象必配例**：定理/命题/性质/结论不能只给断言，要跟一个具体例子或反例。
- **X5 一句一义**：单句 ≤ 48 字（提示线）/ ≤ 72 字（判红线）；并列分句必须有
  连接词或分号，禁止逗号粘连。
- **X6 指代明确**：它/这/该 的先行词在上一句内，否则重写名词。
- **X7 拒绝空话**：不写"很重要/很有用"而不给后果；用"因为…所以…"或"体现在…"。
- **X8 故事四要素**：人物、时间、冲突、结果；四缺二即不合格（缺一提示）。
- **X9 可检验**：每页结论能被读者用一句话复述，或用一个例子检验。
- **X10 不跳过推理**：不用"显然/很容易看出"代替论证。
- **X11 自造术语 = error**：不用「落点/原序对/全称命题」这类自造词代替规则或
  表达式本身（学习者不知道它指哪条规则）；词表可配置在 `text-lint-terms.json`，
  题目 prompt/why/options 命中即判红，必须清零。
- **X12 隐喻克制 = warning**：不用「搬运/拆开/装回/接起来/外壳」这类隐喻代替
  代码或命名（把证明里的替换说成搬东西、把 `↔` 说成拆开再装回）；只提示，
  存量按基线只减不增。
- **X13 选项中文 >24 字 = warning**：选项里的**汉字**（反引号代码/Lean 标识符/
  公式折成 1 字后）超过 24 字就提示——选项是干扰项，长句让人读不完。

## 2. 机检范围（v3，2026-11-28：#149 补 X11–X13、options 进扫描）

| 公理 | 级别 | 机检内容 | 判据位置 |
|---|---|---|---|
| X1 | error | 正文没有句末标点 / 逗号收尾 / 未完成形式收尾 / 必填字段为空 | `check_unit` |
| X3 | warning / error | 出现定义记号（`:=`/`定义为`/`称为`…）却无白话句；定义卡必须有白话的 `oneLiner`（error）；术语首现处无白话句 | `check_unit` / `check_term_first_occurrence` |
| X4 | warning | 解释性正文出现定理/命题/性质/结论，却没有任何「例如/比如/反例」或行内代码 | `check_unit` |
| X5 | error / warning | 单句 >72 字判红、>48 字提示；逗号粘连（≥2 个逗号、无连接词、无分号、≥40 字）提示 | `check_unit` |
| X6 | warning | 段首代词且后面不是「这一条/这三种…」这类自带先行词的名词短语 | `check_unit` |
| X7 | warning | 「很重要/很有用/至关重要…」所在句没有后果（因为/否则/导致/才能…） | `check_unit` |
| X8 | error / warning | 标题含「故事/历史」的页缺四要素中 ≥2 个判红、缺 1 个提示（`--tex`） | `check_story` |
| X11 | error | 题目文本命中自造术语词表（`text-lint-terms.json` 的 `selfInvented`） | `check_question_expression` |
| X12 | warning | 题目文本命中隐喻词表（同一文件的 `metaphors`） | `check_question_expression` |
| X13 | warning | 选项汉字（代码/公式按占位符折掉后）> 24 字 | `check_question_expression` |
| X2/X9/X10 | 人工 | `--review` 输出每段首末句、结论复述与「显然/很容易看出」清单，供人勾选 | `review` |

**扫描范围**（默认、`python3 courses/set-theory/knowledge/text_lint.py`）：

- `data/lessons.json`：`goal`（元信息行，按 `；` 分句量长度）、`intuition`；
- `data/cards.json`：`oneLiner`、段落 `sections[].text`、列表项 `sections[].items[]`、术语行 `types[].oneLiner`；
- `data/questions.json`：`prompt`、`why`、**每个 `options[i]`**（每项一个独立字段）；
- `data/glossary.json`：`oneLiner`。

**题目模式（#149）**：`prompt`/`why` 保留 #144 的全量表达检查（X1–X7），
**额外**跑 X11/X12；`options[i]` **只**跑 X11/X12/X13——选项天然是名词短语或
代码片段，不套 X1/X5 这类段落公理（否则每个纯代码选项都会报「残句」）。

**不扫**：标题/`kicker`/`heading`（标题可以是名词短语）、表格单元格（速记格，
不是段落）、`definition.code`。选项的**样式**问题（长度泄漏答案、`why` 太少句等）
仍由 `text_audit.py` 报告，与本门禁口径不同：这里管**表达**，那里管**题库样式**。
`--tex <file.tex>` 可选扫描讲解图的 `\node` 正文与故事页四要素（`.tex` 源文件不在仓库里，不入库）。

## 3. 误报治理（每条都有来由，改判据前先读）

1. **标题/封面标语/元信息行/列表续行不判 X1**（issue #144 工作项 A4）：
   `section.heading`、`card.title`、`kicker` 这类是名词短语；列表项以 `；` 收尾
   是「列表续行」，不是残句。
2. **数学占位符按 1 字计**（A4）：反引号代码、`$…$` 数学、LaTeX 命令、链接目标、
   没加反引号的 Lean 标识符（`Quot.lift` 写成裸词时）都折成 1 个字再量句长——
   代码不该撑爆「句长」，也不该让 `Quot.sound / Quot.lift / Quot.ind` 这类
   枚举被误判成长句。
3. **目标行按 `；` 分句**：`lessons.goal` 是学习目标清单（元信息），按分号
   分句量长度；它的 `；` 枚举不是逗号粘连。
4. **题干不承担举例义务**：`prompt` 是在问「两种性质的区别是什么」，不是
   「只给断言不给例子」的正文；X4 跳过 `prompt`。
5. **术语行豁免 X6**：`types[].oneLiner` 的「这是允许…的公理」指术语本身，先行词就是术语。
6. **列表项豁免 X6**：列表项可能有上一条的先行词（与原型 v1 同口径）。
7. **「这一条/这三种」不是裸代词**：X6 只抓后面直接接谓词的「它/这/该…」；
   自带量词/名词的指示短语视为有先行词。
8. **`就是/意思是/指的是`算白话**：白话句的判据不是照抄某个词表——只要同段
   出现「就是/意思是/换句话说/一句话…」这类把术语翻译成日常语言的标记即可。
   原型 v1 因词表漏了「就是」而误报过 `Set.empty := fun x => False，故 …`。
9. **术语首现句 ≥6 个汉字才算白话**：`交集是逐点合取：…` 这类 7 字短句是
   合法的白话，不按「太短」误报。
10. **表格单元格与代码块不量**：表格格子的写法是速记（「`a < a` 永远不成立」），
    强行要求完整句只会让卡片变啰嗦；表达底线管的是**段落与列表**。
11. **X11/X12 只扫题目文本**（#149）：讲解段（lessons/cards）的自造术语与隐喻先由
    X1–X8 管；题目才是学习者必须当场读懂、且要据此排除干扰项的地方。`--json`
    里带 `prompt`/`options` 的对象也按题目模式扫（反例验证用）。
12. **X13 只数汉字**（#149）：`normalize()` 已把反引号代码、裸 Lean 标识符、LaTeX
    数学折成 1 个占位符 ⇒ 纯代码选项的汉字数 = 0，永不误报；阈值 24 取自 issue
    #149 的存量实测（63 个选项超线，按基线冻结、只减不增）。
13. **「题干首现库名当场解释」不在本脚本**（#149 边界）：它由 preflight
    （#151，card 覆盖判据）判，`text_lint.py` 不做模糊文本规则；题目文本单元由
    `question_text_units()` 统一产出（prompt/why/options 一份口径），preflight
    需要时可直接复用。

## 4. 存量基线（只减不许增）

门禁策略：**错误即红**；存量欠账冻结在
`courses/set-theory/knowledge/text-lint-baseline.json`，键 =
`file + id + field + code`（`code` 形如 `X5-long` / `X5-splice` / `X11-invented-term` /
`X12-metaphor` / `X13-option-long`，按 `(file,id,axiom)` 口径的细分；题目选项的
`field` 是 `options[i]`）。规则：

- 不在基线里的 finding（error 或 warning）⇒ 判红；
- 基线里已有、但当前**不再出现**的条目（已修好）⇒ 判红，要求
  `--update-baseline` 收紧——「只许减不许增」是双向棘轮；
- 基线里不允许留 error 级条目（errors 清零是 issue #144 的验收线；脚本会
  再查一次并判红）。

收紧基线（内容批次改完后，与内容改动**同一 commit**）：

```sh
python3 courses/set-theory/knowledge/text_lint.py --update-baseline
```

## 5. 用法

```sh
# 门禁（默认扫课程知识库；按基线判红，exit 1 = 红）
python3 courses/set-theory/knowledge/text_lint.py

# 自检：每条公理的红/绿对照 fixtures（改判据必须同步改 fixtures）
python3 courses/set-theory/knowledge/text_lint.py --selftest

# 人工评审清单（X2/X9/X10；输出 Markdown，不判红）
python3 courses/set-theory/knowledge/text_lint.py --review

# 机读 findings / 任意 JSON（临时副本反例验证）
python3 courses/set-theory/knowledge/text_lint.py --json-out
python3 /tmp/copy/text_lint.py --json /tmp/fixture.json --no-baseline   # 残句必须 exit 1
# 题目反例：带自造术语选项 ⇒ exit 1；只有隐喻/超长选项 ⇒ warning；干净 ⇒ exit 0
python3 courses/set-theory/knowledge/text_lint.py --json /tmp/q.json --no-baseline

# 讲解图（可选输入，源文件不入库）
python3 courses/set-theory/knowledge/text_lint.py --tex /tmp/deck.tex
```

退出码：0 = 通过；1 = 有判红/基线漂移；2 = 用法错误。
接线：`scripts/soko gate`、`.github/workflows/ci.yml` 的 `gates-course`、
`scripts/ci-local.sh` 都跑「lint + `--selftest`」两步。

## 6. 内容批次流程（改表达，不改题意）

1. 先跑 `text_lint.py` 拿到**判红清单**（errors 必须清零；warnings 按基线只减不增）；
2. 按 `skills/sokonanoda-prep/SKILL.md` 的批次流程改 `data/*.json` 的
   `prompt`/`why`/`intuition`/`oneLiner`/`items`——**不改 `answer`、不改 id、
   不重编号、不改题意**；
3. 每批收尾全跑：`validate.py` / `coverage.py` / `engine.py selftest` /
   `engine.py simulate --questions 50` / `scripts/notation-lint.py` /
   `text_lint.py`；
4. 报告里写 before/after 计数与改动条目清单；有豁免/基线变化就写清为什么。

## 7. 清账记录

### #144 讲解段清账（2026-11-28）

原型 v1（未提交，只做参考；口径见 issue #144）在作者工作副本上是
**53 errors / 208 warnings**（lessons+cards）。正规化后（占位符折 1 字、误报治理、
判据分级、基线棘轮）在同一棵树上的实测：

| 阶段 | errors | warnings |
|---|---|---|
| 清账前（本树，规范后判据） | 15 | 161 |
| 清账后（本批） | **0** | **107** |

本批改动：lessons 89 条 `intuition`（77 条「本单元围绕…展开，核心是：…」式提纲
改写成真讲解 + 12 条拆句/补白话）、cards 9 处、glossary 5 处、questions 8 条 `why`。
剩余 107 条 warning **全部在 `questions.json` 的 `why`**（X5-long 81 / X5-splice 26），
按卷冻结在基线里（I 卷 55 条、II.1 52 条），只减不增；下一批内容批次逐步改短。
`lessons.json` / `cards.json` / `glossary.json` 当前 **0 warning**。

> 为什么不是把原型 53/208 原样搬进基线：原型把标题/列表续行/代码撑长的句子
> 都算进去了，且数学占位符按整串字符数计——直接照搬会让 CI 一上线就红，也会
> 让内容作者去改本来不该改的标题。正规化后的判据在 §3 逐条记录了豁免理由。

### #149 题目表达门禁（2026-11-28）

options 进扫描并补 X11（error）/ X12（warning）/ X13（warning）后的实测：

| 规则 | 清账前 | 清账后 |
|---|---|---|
| X11 自造术语（error，按字段） | 23（落点 15 / 全称命题 5 / 原序对 3，覆盖 18 道题） | **0** |
| X12 隐喻（warning，按字段） | 30（搬运 7 / 拆开 10 / 装回 6 / 接起来 8；一道选项含两个词） | 30（冻结） |
| X13 选项中文 >24 字（warning） | 63 | 63（冻结） |
| 门禁合计 | 23 errors / 200 warnings | **0 errors / 200 warnings** |

X11 清账 = **只改措辞** 23 个字段（`questions.json`）：落点 → 具体说「`Set.MapsTo`
条件」或「它落在哪」，原序对 → 「原来的序对」，全称命题 → 「这一断言」/
「对任意 `x` 成立」；**不改 id、不改答案、不改题意、不重编号**。
X12/X13 属提示级，按基线冻结（`--update-baseline` 后 0 errors），
`X5` 的 107 条存量 warning 原样保留（本批没有顺手重写 `why`）。
