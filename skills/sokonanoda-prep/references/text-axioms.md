# 文本公理（教学表达底线）

> 判据一句话：**好表达 = 更多人能理解 × 更少时间能理解。**
> 十条公理是底线；可机检的部分由 `courses/set-theory/knowledge/text_lint.py` 判，
> X2/X9/X10 用 `--review` 出**待人工勾选**清单。本文件是公理与判据的唯一说明；
> 脚本的误报治理、基线机制与用法都在这里。

## 1. 十条公理

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

## 2. 机检范围（v2，2026-11-28）

| 公理 | 级别 | 机检内容 | 判据位置 |
|---|---|---|---|
| X1 | error | 正文没有句末标点 / 逗号收尾 / 未完成形式收尾 / 必填字段为空 | `check_unit` |
| X3 | warning / error | 出现定义记号（`:=`/`定义为`/`称为`…）却无白话句；定义卡必须有白话的 `oneLiner`（error）；术语首现处无白话句 | `check_unit` / `check_term_first_occurrence` |
| X4 | warning | 解释性正文出现定理/命题/性质/结论，却没有任何「例如/比如/反例」或行内代码 | `check_unit` |
| X5 | error / warning | 单句 >72 字判红、>48 字提示；逗号粘连（≥2 个逗号、无连接词、无分号、≥40 字）提示 | `check_unit` |
| X6 | warning | 段首代词且后面不是「这一条/这三种…」这类自带先行词的名词短语 | `check_unit` |
| X7 | warning | 「很重要/很有用/至关重要…」所在句没有后果（因为/否则/导致/才能…） | `check_unit` |
| X8 | error / warning | 标题含「故事/历史」的页缺四要素中 ≥2 个判红、缺 1 个提示（`--tex`） | `check_story` |
| X2/X9/X10 | 人工 | `--review` 输出每段首末句、结论复述与「显然/很容易看出」清单，供人勾选 | `review` |

**扫描范围**（默认、`python3 courses/set-theory/knowledge/text_lint.py`）：

- `data/lessons.json`：`goal`（元信息行，按 `；` 分句量长度）、`intuition`；
- `data/cards.json`：`oneLiner`、段落 `sections[].text`、列表项 `sections[].items[]`、术语行 `types[].oneLiner`；
- `data/questions.json`：`prompt`、`why`；
- `data/glossary.json`：`oneLiner`。

**不扫**：标题/`kicker`/`heading`（标题可以是名词短语）、表格单元格（速记格，
不是段落）、`definition.code`、题目 `options`（选项长度/答案泄漏由
`text_audit.py` 报告，不属于表达公理）。`--tex <file.tex>` 可选扫描讲解图的
`\node` 正文与故事页四要素（`.tex` 源文件不在仓库里，不入库）。

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

## 4. 存量基线（只减不许增）

门禁策略：**错误即红**；存量欠账冻结在
`courses/set-theory/knowledge/text-lint-baseline.json`，键 =
`file + id + field + code`（`code` 形如 `X5-long` / `X5-splice`，按 `(file,id,axiom)`
口径的细分）。规则：

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

## 7. 清账记录（2026-11-28，#144）

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
