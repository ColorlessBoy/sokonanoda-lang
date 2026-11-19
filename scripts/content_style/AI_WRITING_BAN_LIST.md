# AI 写作雷点硬禁止清单（C8.4）

> **用途**：`scripts/content_style/style_lint.py` 的判红规则集；本文件是这份规则集的
> 人读版——分类、**确切短语/正则**、**为什么这是 AI 痕迹**、**数学误报白名单**、**出处**。
> 规则与代码一一对应：`python3 scripts/content_style/style_lint.py --show-allowlist`
> 打印当前生效的规则与白名单；改规则必须同时改本文件与 `--selftest` 夹具。
>
> 适用对象：`courses/analysis/units/**` 的 `--` 散文注释（画布 + 解答）与
> 生成书页 HTML 的**可见文本**。**不判** Lean 声明/证明本身（那是 kernel 与课程门禁的事）。

## 0. 先说清楚：这份清单的边界

1. **雷点词只是下限，不是可读性证明。** 一份文字可以一个雷点词都没有，仍然难读
   （结构乱、指代不清、数学跳跃）。正向标准见 `GOOD_WRITING_GUIDE.md` 的评分表。
2. **本课程的实测：经典 AI 词汇雷点在 72 个单元正文里本来就是 0 条**（2026-11-12
   全量扫描）。真正的问题是**结构噪声与模板重复**（`====` 抬头、L3 口条 88 处、
   行内小标题、重复装饰线），这些在 C8.4 里已清。不要为了「有产出」而把正常数学
   用词硬改成同义词——那是另一种坏文风。
3. **宁可漏，不可误伤数学。** 规则只收「数学语域里正常不会出现」的短语；拿不准的
   一律进白名单（§3），并由人工复核。
4. **AI 检测工具不可靠。** 本清单判的是**文本形态**（可复核的短语/结构），不判
   「这段是不是 AI 写的」；见 Wikipedia 该页 Caveats 一节。

## 1. 研究可用性（诚实记录，2026-11-12）

| 渠道 | 状态 |
|---|---|
| `web_search` 工具 | ✗ 不可用（运行环境没有搜索 API key） |
| 直接 `curl` 外网 | ✗ 直连超时 / 连接重置 |
| 本地代理 `127.0.0.1:7890` + `curl` | ✓ 可用（本清单的英文出处即由此取得） |
| `r.jina.ai` 阅读器（经同一代理） | ✓ 可用（PDF 与 JS 页面转纯文本） |
| 通用搜索引擎（Google/Bing/DDG/Baidu） | ✗ 基本被反爬拦截（Bing/DDG 返回挑战页；经 jina 的搜索结果被投毒成无关内容） |

因此：**英文出处是实际抓取并读过原文的**（§4 标「已抓取」）；中文出处里
《文心》《王鼎钧》《张大春》等为**有版权的书**，未抓到全文，只按既有的公共知识
引用其通行观点并标明「未抓到全文」——不编造具体页码或原文引语。

## 2. 硬禁止分类（判红 = exit 1）

规则 id 与 `style_lint.py` 的 `BANNED_RULES` 一致。**任何命中都判红**，没有 warning 档；
确有数学语域理由的走 §3 白名单。

### 2.1 空泛开头 / 结尾（empty openters & closers）

**中文**：`综上所述`、`总而言之`、`总的来说`、`简而言之`、`归根结底`、`毋庸置疑`、
`不可否认`、`众所周知`、`不言而喻`、`值得注意的是`、`需要指出的是`、`需要强调的是`、
`不难发现`、`不难看出`、`显而易见的是`。

**英文**：`in conclusion`、`in summary`、`to sum up`、`in a nutshell`、`it is worth noting`、
`it is important to note`、`it should be noted`、`needless to say`、`as we all know`、
`it goes without saying`、`at the end of the day`、`in today's world/society/landscape`。

**为什么是痕迹**：这些句子在语义上**不增加信息**——它们只说「接下来/前面有个观点」。
LLM 用它们填缝；人类作者要写总结时会直接写出结论本身。判定标准一句话：
**删掉这句话，读者损失了什么事实？** 没有损失 → 删。

### 2.2 元叙述 / AI 自指（meta / AI self-reference）

**中文**：`作为一个 AI`、`作为 AI 语言模型`、`作为语言模型`、`我只是一个语言模型`、
`本回答`、`希望对你有帮助`、`如有任何问题`、`欢迎随时提问/交流`、`让我们一起来看看`、
`让我们一起来探索`、`我们可以看到`、`接下来我们`。

**英文**：`as an AI`、`as a language model`、`as a large language model`、`I hope this helps`、
`let me know`、`feel free to ask`、`Certainly!`、`Of course!`、`here's a detailed breakdown`、
`let's dive/explore/take a look`、`we'll explore`。

**为什么是痕迹**：这是**对话助手对用户说话**的语域，不是教材对读者说话。课程正文的
默认视角是「作者 → 学习者」，不是「我（模型）→ 你（提问者）」。

### 2.3 营销腔 / 夸张（marketing hype）

**中文**：`至关重要`、`举足轻重`、`堪称`、`完美地`、`极致的`、`颠覆性`、`革命性`、
`史无前例`、`前所未有`、`无与伦比`、`淋漓尽致`、`令人惊叹`、`令人瞩目`、`大放异彩`、
`熠熠生辉`、`赋能`、`抓手`、`闭环`、`护城河`、`赛道`、`全方位`、`多维度`、`深层次`、
`具有重要/深远/重大的理论/现实/历史/指导意义`、`作出了重大/巨大/突出贡献`。

**英文**：`pivotal`、`crucial`、`groundbreaking`、`game-changer`、`revolutionary`、
`unparalleled`、`state-of-the-art`、`cutting-edge`、`seamless`、`leverage`、`delve into`、
`tapestry`、`vibrant`、`holistic`、`foster`、`underscore`、`showcase`、`elevate`、`unleash`、
`empower`、`transformative`、`meticulous`、`multifaceted`、`intricate`。

**为什么是痕迹**：可读性来自**事实与结构**，不来自抬高语气。数学写作里
「重要的」必须能回答「对什么问题重要」；答不上来就是空转。英文 `delve/tapestry/
underscore/showcase` 是 2023 起被批量统计、被 Wikipedia 编辑器单列的 LLM 高频词。

### 2.4 假证据 / 含糊引证（fake evidence & vague attribution）

**中文**：`研究表明/显示/指出`、`数据表明/显示`、`专家表示/认为/指出/一致`、
`有研究表明/显示/发现`、`据调查`、`据统计`、`据媒体报道`、`业内人士`、`相关人士`、
`多方消息`、`普遍认为`、`被广泛认为/认可/使用/接受`。

**英文**：`studies show`、`research shows/suggests/indicates`、`experts agree/say/believe`、
`data show/suggests`、`many experts/researchers/studies`、`some critics argue`、
`observers have noted`、`industry reports`、`widely regarded as`、`it is widely believed`、
`sources say`。

**为什么是痕迹**：观点被挂到**不点名的权威**上，读者无法核对。本课程的引文必须精确
（`Tao《Analysis I》§5.2`、`issue #18`、`docs/design/course-stdlib.md`、`单元③`）；
点得出名字与位置的引用**不判**。

### 2.5 翻译腔（translationese）

**中文**（对应余光中《怎样改进英式中文》指出的西化句法）：
`被（人们）认为/称为/视为`、`被广泛使用`、`对于……来说`、`关于……的问题`、
`进行了详细/深入/系统的研究/分析/讨论`、`作出了研究/贡献/决定/反应`、
`具有很高的知名度/可读性/重要性`、`最……之一`、`作为……之一`。

**英文**：`due to the fact that`、`in order to`、`a large number of`、`it can be seen that`、
`as can be seen`、`in terms of`、`with regard to`、`one of the most`、
`there is/are ... that/which`。

**为什么是痕迹**：中文有更直接的写法：`因此` 而不是 `基于这个原因`；`问题很多`
而不是 `有很多问题存在`；`贡献很大` 而不是 `作出了重大的贡献`；`用老鼠试验`
而不是 `在老鼠身上进行试验`。余光中的诊断：**化简为繁、以拙代巧、化动为静、
名词成灾**——与 LLM 的「抽象名词 + 弱动词」输出形态同源。

### 2.6 模板结构（template structure）

**中文**：`首先……其次……最后……`（同段三连）、`第一，……第二，……第三，……`（同段）、
`一方面……另一方面……`、`从……角度来看`。

**英文**：`first(ly) ... second(ly) ... third(ly)/finally`（同段）、
`on the one hand ... on the other hand`、`in this article/paper/section we will ...`。

**为什么是痕迹**：结构先于内容。真正需要列举时，**列表**是对的（数学步骤就该列）；
被判红的是**用连接词骨架撑篇幅**：把一句话拆成三段并列，只是为了看起来完整。

> 例外的正确做法：数学证明路线用**编号列表**（1. / 2. / 3.）——这不是模板结构，
> 是读者需要的路线图；本课程大量使用，**不判**。

### 2.7 重复（repetition）

* **相邻**逐字重复的散文行/段落 → 判红（读者会明显感到「卡带」）；
* 同一文件里同一句散文出现 **≥3 次**（非提示模板）→ 判红；
* 同一文件里逐字重复的**非相邻**散文段 → 判红；**例外**：带数学符号/代码标识符的
  段落（数学事实在不同声明下各带一份是设计，**不静默删除**，只记 warning）。

**为什么是痕迹**：LLM 受上下文影响会复读；人写长文也会重复，但通常是**改述式**
重复（提供新角度），不是逐字复制。

### 2.8 空泛模糊（vague hedging）

**中文**：`在某种意义上`、`某种程度上`、`可以说`、`也许可以`、`或许可以说`、
`大致上可以说`、`基本上可以说`。

**英文**：`arguably`、`it seems that/to`、`to some extent`、`in a sense`、
`it could be argued`、`somewhat more/less/better`。

**为什么是痕迹**：限定词让句子**看起来**谨慎，实际没有承诺任何可检验内容。
数学写作该说「存在 N，使 n ≥ N 时 …」，而不是「在某种意义上 …」。

### 2.9 过度格式（over-formatting）

| 形态 | 正则 | 说明 |
|---|---|---|
| 整行加粗 | `^\s*\*\*[^*]{1,40}\*\*\s*$` | 「重点提示」式排版 |
| 行内小标题清单 | `^\s*(?:[-•·–—]\s*)?\*\*[^*]{1,32}\*\*\s*[：:]\s*\S` | `**标签**：说明` 竖排清单 |
| markdown 标题/分隔/链接/HTML | `^\s*#{1,6}\s`、`^\s*[-*_]{3,}\s*$`、`\[[^\]]+\]\(https?://`、`</?(b|i|em|strong|br|p|div|span|ul|ol|li|h[1-6])\b` | 课程注释是纯文本约定 |
| emoji 装饰 | `[\U0001F300-\U0001FAFF\u2705\u274C\u2B50\u2728]` | 标题/清单不放 emoji |
| ASCII 分隔线 | `^\s*[=\-_*]{4,}\s*$`（源码行） | `====` / `----` / `****` 抬头噪声 |
| 文件抬头 | `^[A-Za-z0-9_.\-]+\.sokonanoda\s*(——|--|：:)` | 把文件名写进正文 |
| 相邻重复装饰线 | 同一行 `────` 连写两遍 | 书页会渲染成两条 `<hr>` |

**为什么是痕迹**：过度格式是「用排版代替写作」——重点应该由句子结构承担。
数学书页可以有**少量**装饰线做章节分隔（`─` 行在本仓库是书页的既定渲染素材，
C8.4 只删「相邻重复/ASCII 抬头」噪声，是否保留单条装饰线是 C8.3 书页展示决定）。

### 2.10 高钩子 / 伪悬念开场（hook-opening）

**硬禁止（owner 2026-11-12：高钩子反而是 AI 文章的大雷，信用已崩塌）**

| 形态 | 例子（正则命中） | 说明 |
|---|---|---|
| 第二人称伪提问 | 你有没有想过、你是否曾经、你知道…吗 | 制造好奇缺口，不提供信息 |
| 想象/假设式钩子 | 想象一下、如果我说、假如告诉你 | 把读者拖进虚构场景 |
| 夸张承诺 | 99% 的人不知道、看完这篇、颠覆/刷新你的认知 | 标题党残留 |
| 剧情钩子 | 凌晨三点、我曾经也、直到我…、故事要从…说起 | 与数学正文无关的叙事钩子 |
| 英文 cheap hook | Have you ever、What if I told you、Most people don't know、Here's the thing、Let me tell you、At 3 am | 同上 |

**为什么是痕迹**：钩子不是解释；它用悬念替代问题，用「你」替代论证。数学书应当第一段直接说：这一单元缺什么、要证什么、为什么现在需要它。**不制造悬念、不写故事引子、不用第二人称逼问。**

**允许的替代**：直接写「上一单元证了 X；还差 Y；本单元补 Y」；给具体例子可以，但必须立刻服务于数学问题，而不是当钩子。

## 3. 数学误报白名单（**不判红**）

这些词在数学语域里正常；linter 显式放过（代码里 `ALLOWLIST` / `ALLOWLIST_TERMS`）：

| 白名单 | 理由 |
|---|---|
| `绝对值`、`绝对收敛`、`绝对误差`、`绝对有界` | 数学术语；「绝对」不按营销词处理 |
| `本文件`、`本文` | 课程/文档自称（本清单自己也在用） |
| `注意`、`注意点` | 教学提示（"注意这里 Γ 的定义"是必要的） |
| `因此`、`所以`、`设`、`定义`、`定理`、`证明`、`引理` | 数学说理词，频率高是正常的 |
| `显然`（**附理由**） | 只有在同段 160 字内出现 `因为/由于/由…可得/展开/归约/定义性/直接算/就是/by definition/because/since` 等理由时才放过；**裸断言判红** |
| `关键件`、`目标形态`、`思路` | 课程提示模板标签（`soko:hint` 的结构字段） |
| `最后`、`于是`、`接着` | 证明叙事的顺序词（不是「首先/其次/最后」骨架） |
| `之一`（普通用法） | 只有 `最……之一` / `作为……之一`（西化句法）判红 |
| `我们` | 数学叙述的「我们」（"我们构造…"）正常 |
| `可能`、`如果`、`假设` | 数学讨论的限定词 |

**白名单也是判据**：`--selftest` 的正向夹具专门塞了这些词，它们必须判绿——否则
规则会开始误伤数学，那比漏判更糟。

## 4. 出处（按实际抓取状态标注）

已抓取（经本地代理 `127.0.0.1:7890`；`r.jina.ai` 用于 PDF/JS 页面）：

1. Wikipedia, *Wikipedia:Signs of AI writing* —
   <https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing>（§2.1–§2.9 的分类骨架；
   其 §Language and grammar 的「AI vocabulary」「negative parallelisms」「rule of three」、
   §Style 的 bold/list/title-case 清单直接支撑本清单）
2. Wikipedia, *Wikipedia:Signs of AI writing/Style* —
   <https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing/Style>
3. Wikipedia, *AI slop* — <https://en.wikipedia.org/wiki/AI_slop>
4. W. Strunk Jr., *The Elements of Style*（1918 版，Gutenberg #37134）—
   <https://www.gutenberg.org/files/37134/37134-h/37134-h.htm>
   （规则 10–13：active voice / positive form / definite specific concrete / omit needless words）
5. George Orwell, *Politics and the English Language* —
   <https://www.orwell.ru/library/essays/politics/english/e_polit>
   （dying metaphors、operators/verbal false limbs、pretentious diction、meaningless words）
6. G. D. Gopen & J. A. Swan, *The Science of Scientific Writing*, American Scientist 78(6), 1990 —
   <https://cseweb.ucsd.edu/~swanson/papers/science-of-writing.pdf>
   （读者预期七原则：主谓靠近、stress/topic position、old→new、一个单位一个要点、动词承担动作）
7. D. E. Knuth, T. Larrabee, P. M. Roberts, *Mathematical Writing*（Stanford CS209 讲义）—
   <https://jmlr.csail.mit.edu/reviewing-papers/knuth_mathematical_writing.pdf>
   （先有内容再有形式、组织先于润色、例子为中心、读者会跳读、标题与定理陈述要能扫读）
8. P. R. Halmos, *How to Write Mathematics* —
   <https://sites.stat.columbia.edu/liam/teaching/HalmosWrite.pdf>
   （有东西要说/对谁说/先组织/记号先设计/「显然」要能还原）
9. Terence Tao, *Advice on Writing*（写作建议索引页）—
   <https://terrytao.wordpress.com/advice-on-writing/>
10. 余光中《怎样改进英式中文——论中文的常态与变态》（原载《明报月刊》1987-10）—
    <https://language.chinadaily.com.cn/a/201509/01/WS5b20a851a31001b825720b30.html>
    （抽象名词/弱动词「作出/进行」、`之一`/`最…之一`、`关于/有关/由于`、被字句、and 意识）
11. Google, *Developer Documentation Style Guide: Highlights* —
    <https://developers.google.com/style/highlights>
    （pre-announcement 禁止、第二人称、主动语态、条件先于指令、句首大写、列表选用）
12. *Diátaxis*（tutorial / how-to / reference / explanation 四象限）— <https://diataxis.fr/>
13. 美国政府 *plainlanguage.gov* 指南（现由 digital.gov 承接）—
    <https://www.plainlanguage.gov/guidelines/> / <https://digital.gov/guides/plain-language>
14. Wikipedia, *Made to Stick*（Heath 兄弟 SUCCESs 框架的条目）—
    <https://en.wikipedia.org/wiki/Made_to_Stick>

未能抓取全文（只作通行观点引用；**未编造引语**）：

* Pinker, *The Sense of Style*；J. M. Williams, *Style: Lessons in Clarity and Grace*；
  W. Zinsser, *On Writing Well*（站点 403）；J. Swales 的 CARS 模型；
  G. Graff & C. Birkenstein, *They Say / I Say*；B. Minto, *The Pyramid Principle*；
  *The Chicago Manual of Style*；B. Dreyer, *Dreyer's English*；AP Stylebook；
  J. McPhee, *Draft No. 4*；J. Didion；NN/g 微文案文章（404）；
  Amazon narrative memo（页面取回为空）。
* 夏丏尊/叶圣陶《文心》、王鼎钧《作文四书》、张大春（有版权，未抓全文；按公共
  常识引用其观点：「读写结合」「平实」「想清楚再写」）。
* 数学写作：S. Krantz, *A Primer of Mathematical Writing*；M. Schimel, *Writing Science*
  （未抓全文，按通行观点引用）。

## 5. 维护规则

1. **改规则必须三件套**：`BANNED_RULES` / 本文件 §2 / `--selftest` 夹具。三者必须一致。
2. **新增规则先问两个问题**：(a) 数学语域里会不会正常出现？会 → 进白名单或收窄正则；
   (b) `--selftest` 能不能咬住一条已知坏句子？咬不住（永远 0 命中）的守卫等于没有。
3. **判红必须可定位**：输出 `file:line: snippet` 或（HTML 模式）`unit 归属 + 原文块`。
4. **不许把「没命中」当可读性通过**：见 §0 与 `GOOD_WRITING_GUIDE.md` §6 的评分表——
   可读性要人读样本 + 六维评分，linter 只负责下限。
