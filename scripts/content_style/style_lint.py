#!/usr/bin/env python3
"""课程文案 / 可读性硬门禁（C8.4）：AI 写作雷点 + 结构噪声 + 正文可读性。

扫描对象（两种模式，可同时给）：

1. **源码注释**（默认）：`courses/analysis/units/**/*.sokonanoda` 里的
   `--` 散文注释（画布 + 解答）。**只读**：不修改任何课程文件。
2. **生成的书页 HTML**（`--html PATH`）：只读可见文本（跳过
   `script/style/pre/code/head/textarea`；`pre` 里的代码与证明不算散文）。

判红项（exit 1；全部是**硬判据**，不是提示）：

* `banned-phrase`  AI 写作雷点：空泛开头/结尾、元叙述与 AI 自指、营销腔、
  假证据/含糊引证、翻译腔、模板结构、空泛模糊、过度格式（清单见 RULES）。
* `separator-noise` ASCII 分隔线 / 文件名抬头噪声（`====`、`----`、`****`、
  `unitNN-....sokonanoda ——` 这类文件头）。
* `redundant-rule` 相邻重复的装饰线（同一行 `────` 连写两遍）。
* `adjacent-duplicate` 相邻重复的散文行/段落（规范化空白后逐字相同）。
* `duplicate-prose` 同一文件内重复出现的散文段落（非相邻也算）。
* `template-boilerplate` 课程工程口条模板（如「L3：律必须留在单元里」），
  这类句子是给维护者看的，不该反复出现在读者可见的段落里。
* `missing-unit-lead` 单元画布缺少导语段（`courses/analysis/units/<章>/*.sokonanoda`）。
* `long-paragraph` 散文段落超过 1200 字符（与书页 `MAX_PROSE_CHARS` 同口径）。
* `unjustified-clear` 「显然 / clearly」没有在附近给出理由（数学写作用词本身
  不禁止——见 allowlist；禁的是不解释就断言）。
* `over-formatting` markdown 标题、行内小标题清单、粗体整行、HTML 标签、emoji
  装饰等过度格式。
* `raw-marker`（`--html`）可见文本里漏出 `soko:hint` / `**` / `__` / 分隔线。

**数学误报白名单**（硬编码在 ALLOWLIST；见 `--show-allowlist`）：
绝对值/绝对、本文件/本文（课程文档自称）、注意、因此/所以/设/定义/定理/证明、
「显然（因为/由定义/展开可见…）」、`关键件`、`最后`、`之一`（普通用法）、
`我们`（数学叙述）、`可能`（数学讨论）等——这些是数学语域的正常用词，不判红。

用法（仓库根或任意目录均可）：

```bash
python3 scripts/content_style/style_lint.py                    # 源码模式，人类可读
python3 scripts/content_style/style_lint.py --json             # 机器可读（含计数）
python3 scripts/content_style/style_lint.py --root courses/analysis/units
python3 scripts/content_style/style_lint.py --html dist/book/book-v2/index.html
python3 scripts/content_style/style_lint.py --selftest         # 判据自检（坏夹具必须判红）
python3 scripts/content_style/style_lint.py --show-allowlist   # 打印白名单与规则
```

退出码：0 = 干净；1 = 有判红项；2 = 用法/IO 错误。

设计来源见同目录 `AI_WRITING_BAN_LIST.md`（雷点清单与出处）与
`GOOD_WRITING_GUIDE.md`（正向可读性规则、改写样例与评分表）。
"""

from __future__ import annotations

import argparse
import html as html_mod
import json
import re
import sys
import tempfile
from dataclasses import dataclass, field
from html.parser import HTMLParser
from pathlib import Path
from typing import Iterable, Sequence

MAX_PROSE_CHARS = 1200  # 与 scripts/build_book_proto.py 的 MAX_PROSE_CHARS 对齐
MIN_LEAD_CHARS = 18  # 导语至少这么长（去空白后）

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ROOT = REPO_ROOT / "courses" / "analysis" / "units"

# ── 源码注释的形态 ────────────────────────────────────────────────────────────

COMMENT_PREFIX = "--"
HINT_PREFIX = "soko:hint"
DECL_RE = re.compile(r"^(?:def|theorem|axiom|inductive|example|instance|abbrev|structure)\b")
IMPORT_RE = re.compile(r"^import\b")
TITLE_RE = re.compile(
    r"^\s*单元\s*(?:[①-⑳]|[㉑-㉟]|[㊱-㊿]|\d+)"  # 单元① / 单元23 / 单元㊿
)
# 装饰线：与 build_book_proto.DECOR_RE 一致（书页把它渲染成 <hr>）。
DECOR_RE = re.compile(r"^\s*[=─━\-—_*]{4,}\s*$")
# ASCII 分隔线：`====` / `----` / `****` / `____` —— 渲染成正文空段，属于噪声。
ASCII_SEP_RE = re.compile(r"^\s*[=\-_*]{4,}\s*$")
# 文件名抬头：`unit02-quotients-review.sokonanoda —— M1 单元② ...`
FILENAME_HEADER_RE = re.compile(r"^[A-Za-z0-9_.\-]+\.sokonanoda\s*(?:——|--|：:)")
# 课程工程注记（与 build_book_proto.COURSE_NOTE_RE 同一族；书页会把它们收进
# 折叠的「编写注记」面板）。它们不是判红项，只是比普通散文更「维护者向」。
COURSE_NOTE_RE = re.compile(
    r"^(?:"
    r"靶子(?:\s*[（(]教材[）)])?\s*[：:]"
    r"|大纲\s*[：:]"
    r"|依赖\s*[：:]"
    r"|本单元只允许用"
    r"|本单元\s*`?[Ii]mport`?"
    r"|本单元自带"
    r"|本文件按\s*L3"
    r"|⚠"
    r")"
)
# 只给维护者看的模板口条（判红：重复出现且读者不可见地重复）。
TEMPLATE_BOILERPLATE_RE = re.compile(
    r"(?:L3\s*[：:]\s*律必须留在单元里|律必须留在单元里)"
)

# ── AI 写作雷点规则 ──────────────────────────────────────────────────────────
# 每条：id / category / pattern / why（为什么这是 AI 痕）/ 可选 allow（命中处
# 附近出现即放过，用于数学误报）。字符串用 `|` 连接的等价写法，正则里已分组。
# 规则只收「确定性高、数学语域不会正常出现」的短语——宁可漏，不可误伤数学。

BANNED_RULES: list[dict] = [
    # 1. 空泛开头 / 结尾（empty openers & closers）
    dict(
        id="zh-empty-opener",
        category="empty-opener",
        pattern=re.compile(
            r"(综上所述|总而言之|总的来说|简而言之|归根结底|归结起来|"
            r"毋庸置疑|不可否认|众所周知|不言而喻|显而易见的是|"
            r"值得注意的是|需要指出的是|需要强调的是|不难发现|不难看出)"
        ),
        why="空泛的总结/开启话术：没有给读者任何新信息，只是告诉读者『接下来要说点什么』。",
    ),
    dict(
        id="en-empty-opener",
        category="empty-opener",
        pattern=re.compile(
            r"(?i)\b(in conclusion|in summary|to sum up|in a nutshell|"
            r"it is worth noting|it is important to note|it should be noted|"
            r"needless to say|as we all know|it goes without saying|"
            r"at the end of the day|in today's (?:world|society|landscape))\b"
        ),
        why="英文同理：总结句与『值得注意』是 LLM 的高频填充。",
    ),
    # 1b. 高钩子 / curiosity-gap 开场（hook-opening）
    #     Owner 2026-11-12: cheap hooks are now an AI-writing tell; credibility
    #     collapse. The math book opens by stating the question/missing idea.
    dict(
        id="zh-hook-opening",
        category="hook-opening",
        pattern=re.compile(
            r"(你有没有(?:想过|发现|注意)|你是否(?:曾经|想过|注意)|你可曾|"
            r"你知道(?:吗|么)|想象一下|设想一下|试想一下|"
            r"如果我说|假如我说|如果告诉你|大多数人(?:都)?不知道|"
            r"99%\s*的?人|百分之九十九|别急着|先别急|先别划走|看到最后|"
            r"看完这|颠覆你的|刷新你的|你以为|其实不然|真相是|"
            r"故事的开始|故事要从|这一切要从|凌晨三点|我曾经也|直到我|"
            r"你可能没注意|你一定(?:会|遇到|见过)|不是普通的|这不是一篇)"
        ),
        why="劣质钩子/伪悬念：第二人称提问或制造好奇缺口，是 AI 文章高频开场；数学书应先说清问题。",
    ),
    dict(
        id="en-hook-opening",
        category="hook-opening",
        pattern=re.compile(
            r"(?i)\b(imagine (?:a|an|that|if)|have you ever|did you know|"
            r"what if i told you|most people don't (?:know|realize)|"
            r"nobody (?:talks|tells)|here(?:'s| is) the thing|the truth is|"
            r"let me tell you|it all started|at 3\s*am|stop scrolling|"
            r"you won't believe|this changed (?:everything|my life)|"
            r"i used to think|until i (?:discovered|realized))\b"
        ),
        why="English cheap-hook opening: curiosity gap/manufactured suspense, a known AI-writing tell.",
    ),
    # 1c. 元标签 / 目录式正文（meta-label）
    #     Owner 2026-11-12: "本单元只做一件事" 作为原则可以，但不能写成正文再罗列；
    #     正文应当是解释本身，不是写作说明。
    dict(
        id="zh-meta-label",
        category="meta-label",
        pattern=re.compile(
            r"(本单元只做一件事|本单元讲什么|本单元的[“\"]?该学的定理|"
            r"本单元(?:的)?[“\"]?要学的定理)"
        ),
        why="把写作原则/目录标签写进正文，是课堂笔记/索引，不是教材；应直接写动机与解释。",
    ),
    # 2. 元叙述 / AI 自指（meta / AI self-reference）
    dict(
        id="zh-meta-ai",
        category="meta-ai",
        pattern=re.compile(
            r"(作为一个?\s*AI|作为\s*AI\s*语言模型|作为一?个?\s*语言模型|"
            r"我只是一个?语言模型|本回答|希望对(?:你|您)(?:有|有所)帮助|"
            r"如有(?:任何)?(?:问题|需要)|欢迎(?:随时)?(?:提问|交流|反馈)|"
            r"让我们(?:一起)?(?:来)?(?:看看|探索|探讨|深入|开始)|"
            r"我们可以(?:看到|发现|注意到)|接下来(?:我们|让我们))"
        ),
        why="课程正文不该出现「AI/回答/让我们」这类对话助手口吻。",
    ),
    dict(
        id="en-meta-ai",
        category="meta-ai",
        pattern=re.compile(
            r"(?i)\b(as an? (?:AI|language model)|as a large language model|"
            r"I hope (?:this|that) helps|let me know|feel free to ask|"
            r"certainly!|of course!|here(?:'s| is) a (?:detailed|comprehensive) breakdown|"
            r"let(?:'s| us) (?:dive|explore|take a look)|we(?:'ll| will) (?:explore|dive))\b"
        ),
        why="对话助手的套话，不是课程正文。",
    ),
    # 2b. 元写作泄漏 / 讲稿腔（owner 2026-11-12）
    dict(
        id="zh-meta-writing-leak",
        category="meta-writing",
        pattern=re.compile(
            r"(本单元\s*只\s*(?:做|立|讲|写|证)|本单元\s*将|本单元\s*先|本单元\s*要|"
            r"本单元的?(?:目标|任务|目的)|本单元\s*[：:]|本单元只允许用|本单元自带|"
            r"本节将|本节介绍|本节只|本小节)"
        ),
        why="写作计划/维护者口条写进正文：读者要看问题从哪来、为什么这样定义；删掉这句信息不损失。",
    ),
    dict(
        id="zh-meta-unit-opener",
        category="meta-writing",
        pattern=re.compile(r"^本单元"),
        why="以「本单元…」作为句子开头是讲稿/工程笔记口吻；改写为从上一单元遗留问题或反例讲起。",
        warn_only=True,
    ),
    dict(
        id="zh-meta-narrative-leak",
        category="meta-writing",
        pattern=re.compile(r"(接下来我们|下面我们|下面来看|本节来看|接下来看)"),
        why="报幕式导航语；教材应直接进入下一步的动机与推理。",
    ),
    # 3. 营销腔 / 夸张（marketing hype）
    dict(
        id="zh-marketing",
        category="marketing-hype",
        pattern=re.compile(
            r"(至关重要|举足轻重|堪称|完美地|极致的|极具|颠覆性|革命性|"
            r"史无前例|前所未有|无与伦比|淋漓尽致|令人惊叹|令人瞩目|大放异彩|"
            r"熠熠生辉|赋能|抓手|闭环|护城河|赛道|全方位|多维度|深层次|"
            r"具有(?:重要|深远|重大)的?(?:理论|现实|历史|指导)?意义|"
            r"作出了?(?:重大|巨大|突出)?贡献)"
        ),
        why="宣传文案的形容词：可读性应来自事实与结构，而不是抬高语气。",
    ),
    dict(
        id="en-marketing",
        category="marketing-hype",
        pattern=re.compile(
            r"(?i)\b(pivotal|crucial|groundbreaking|game[- ]chang(?:er|ing)|"
            r"revolutionary|unparalleled|state[- ]of[- ]the[- ]art|cutting[- ]edge|"
            r"seamless(?:ly)?|leverage|delv(?:e|ing) into|tapestry|vibrant|"
            r"holistic|foster(?:ing)?|underscor(?:e|es|ed|ing)|showcas(?:e|es|ed|ing)|"
            r"elevate|unleash|empower(?:ing)?|transformative|meticulous(?:ly)?|"
            r"multifaceted|intricate)\b"
        ),
        why="英文「AI vocabulary」：delve/tapestry/underscore/showcase 是 2023 起被反复统计的 LLM 高频词。",
    ),
    # 4. 假证据 / 含糊引证（fake evidence & vague attribution）
    dict(
        id="zh-fake-evidence",
        category="fake-evidence",
        pattern=re.compile(
            r"(研究(?:显示|表明|指出|发现)|数据(?:显示|表明)|"
            r"专家(?:表示|认为|指出|一致)|有研究(?:表明|显示|发现)|"
            r"据(?:调查|统计|报道)|业内人士|相关人士|多方(?:消息|证实)|"
            r"普遍认为|被广泛(?:认为|认可|使用|接受))"
        ),
        why="没有具体出处的『研究表明/专家认为』是典型的伪证据；本课程引文必须给出精确来源。",
    ),
    dict(
        id="en-fake-evidence",
        category="fake-evidence",
        pattern=re.compile(
            r"(?i)\b(studies (?:show|have shown|suggest)|research (?:shows|suggests|indicates)|"
            r"experts (?:agree|say|believe|argue)|data (?:show|suggests)|"
            r"many (?:experts|researchers|studies)|some critics argue|"
            r"observers have (?:noted|cited)|industry reports|widely regarded as|"
            r"it is widely believed|sources say)\b"
        ),
        why="含糊引证：把观点挂到不点名的『专家/研究』上，读者无法核对。",
    ),
    # 5. 翻译腔（translationese）
    dict(
        id="zh-translationese",
        category="translationese",
        pattern=re.compile(
            r"(被(?:人们)?(?:认为|称为|视为)(?:是)?|被广泛(?:使用|接受)|"
            r"对于[^。，；！？]{0,24}来说|关于[^。，；！？]{0,24}的问题|"
            r"进行了?(?:详细|深入|系统|全面)?的?(?:研究|分析|讨论|探索)|"
            r"作出了?(?:详细|深入)?的?(?:研究|分析|贡献|决定|反应)|"
            r"具有(?:很|较)高的(?:知名度|可读性|重要性)|"
            r"最[^。，；！？]{0,18}之一|作为[^。，；！？]{0,14}之一)"
        ),
        why="西化句法（被字句/抽象名词+弱动词/之一）：中文有更直接的写法。",
    ),
    dict(
        id="en-translationese",
        category="translationese",
        pattern=re.compile(
            r"(?i)\b(due to the fact that|in order to|a large number of|"
            r"it can be seen that|as can be seen|in terms of|with regard to|"
            r"one of the most|there (?:is|are) (?:a |an )?\w+ (?:that|which))\b"
        ),
        why="英文冗词与套句（due to the fact that / in order to）：能删就删。",
    ),
    # 6. 模板结构（template structure）
    dict(
        id="zh-template-sequence",
        category="template-structure",
        pattern=re.compile(r"(首先[^。]{0,60}其次[^。]{0,60}(?:最后|再次))"),
        why="『首先…其次…最后』的万能三段式：结构先于内容，典型 LLM 骨架。",
    ),
    dict(
        id="en-template-sequence",
        category="template-structure",
        pattern=re.compile(
            r"(?i)\b(first(?:ly)?,?.{0,120}second(?:ly)?,?.{0,120}(?:third(?:ly)?|finally))"
        ),
        why="parallel 三段式列出，而非按内容需要组织。",
    ),
    # 7. 空泛模糊（vague hedging）
    dict(
        id="zh-vague-hedge",
        category="vague-hedge",
        pattern=re.compile(
            r"(在某种意义上|某种程度上|可以说(?:是|，)|也许可以|或许可以说|"
            r"大致上(?:可以)?说|基本上可以说)"
        ),
        why="模糊限定词让句子看起来谨慎，实际上没有承诺任何可检验的内容。",
    ),
    dict(
        id="en-vague-hedge",
        category="vague-hedge",
        pattern=re.compile(
            r"(?i)\b(arguably|it seems (?:that|to)|to some extent|in a sense|"
            r"it could be argued|somewhat (?:more|less|better|worse))\b"
        ),
        why="英文模糊限定：数学写作里该给出可核对的断言。",
    ),
    # 8. 过度格式（over-formatting）——行级规则在结构化检查里
    dict(
        id="zh-overformat-inline",
        category="over-formatting",
        pattern=re.compile(r"^\s*\*\*[^*]{1,40}\*\*\s*$"),
        why="整行加粗是『重点提示』式排版；正文靠句子结构突出重点。",
    ),
    dict(
        id="zh-overformat-header-list",
        category="over-formatting",
        pattern=re.compile(
            r"^\s*(?:[-•·–—]\s*)?\*\*[^*]{1,32}\*\*\s*[：:]\s*\S"
        ),
        why="行内小标题清单（**标签**：说明）是 AI 输出的结构痕迹，散文应写成自然段。",
    ),
    dict(
        id="md-overformat",
        category="over-formatting",
        pattern=re.compile(
            r"^\s*#{1,6}\s|^\s*[-*_]{3,}\s*$|\[[^\]]+\]\(https?://|"
            r"</?(?:b|i|em|strong|br|p|div|span|ul|ol|li|h[1-6])\b[^>]*>"
        ),
        why="markdown 标题/链接/HTML 标签与课程注释的纯文本约定不一致。",
    ),
    dict(
        id="emoji-overformat",
        category="over-formatting",
        pattern=re.compile(r"[\U0001F300-\U0001FAFF\u2705\u274C\u2B50\u2728]"),
        why="emoji 标题/清单是营销号排版；数学排版里不用 emoji 装饰。",
    ),
]

# 白名单上下文：命中处前/后这么多个字符内出现这些词，就不判红（数学误报）。
ALLOWLIST: list[dict] = [
    dict(
        rule="zh-clear-with-reason",
        pattern=re.compile(r"显然|\bclearly\b|\bobviously\b"),
        allow=re.compile(
            r"(看着|看似|貌似|表面上|字面上|“|”|\"|「|」|"
            r"因为|由于|由[^。]{0,20}(?:可得|得|知|可见)|展开|归约|定义性|"
            r"直接(?:算|由|用|套)|同义反复|就是|即|by definition|because|since|"
            r"follows from|unfold)"
        ),
        window=160,
        why="「显然」只有在给出理由/可还原为定义性展开时才允许（作业/教材也要讲理）。",
    ),
    dict(
        rule="zh-absolute-value",
        pattern=re.compile(r"绝对(?!值|收敛|误差|连续|可积|有界|单调)"),
        allow=re.compile(r"绝对(?:值|收敛|误差|连续|可积|有界|单调)"),
        window=8,
        why="『绝对』在数学里是绝对值/绝对收敛等术语，不按营销词处理。",
    ),
]
# 明确不判的数学语域词（任务要求写出来的白名单；用于文档与自检，不参与正则）：
ALLOWLIST_TERMS = [
    "绝对值/绝对收敛/绝对误差（绝对）",
    "本文件/本文（课程文档自称）",
    "注意/注意点（教学提示）",
    "因此/所以/设/定义/定理/证明/引理（数学说理词）",
    "显然（同段给出因为/由定义/展开可见的理由时）",
    "关键件/目标形态/思路（课程提示模板标签）",
    "最后/于是/接着（证明叙事的顺序词）",
    "之一（普通用法，如『三本参考书之一』）",
    "我们（数学叙述的『我们』）",
    "可能/如果/假设（数学讨论的限定词）",
]

# ── 数据结构 ─────────────────────────────────────────────────────────────────


@dataclass
class Hit:
    path: str
    line: int
    rule: str
    category: str
    message: str
    snippet: str = ""

    def as_dict(self) -> dict:
        return dict(
            file=self.path,
            line=self.line,
            rule=self.rule,
            category=self.category,
            message=self.message,
            snippet=self.snippet,
        )


@dataclass
class Paragraph:
    start_line: int
    text: str
    kind: str = "prose"  # prose | note | title | hint
    hint_label: str = ""


class LintReport:
    """hits = 判红项（exit 1）；warnings = 记录但不判红（如数学事实重复陈述）。"""

    def __init__(self) -> None:
        self.hits: list[Hit] = []
        self.warnings: list[Hit] = []
        self.stats: dict = {}

    def add(self, hit: Hit) -> None:
        self.hits.append(hit)

    def warn(self, hit: Hit) -> None:
        self.warnings.append(hit)

    @property
    def ok(self) -> bool:
        return not self.hits

    def by_category(self) -> dict:
        counts: dict[str, int] = {}
        for hit in self.hits:
            counts[hit.category] = counts.get(hit.category, 0) + 1
        return dict(sorted(counts.items()))


# ── 源码注释解析 ─────────────────────────────────────────────────────────────


def strip_comment(line: str) -> tuple[bool, str]:
    """返回 (是否注释行, 去掉 `--` 前缀后的正文)。保留正文里的前导空格。"""
    stripped = line.lstrip()
    if not stripped.startswith(COMMENT_PREFIX):
        return False, ""
    rest = stripped[len(COMMENT_PREFIX):]
    if rest.startswith(" "):
        rest = rest[1:]
    return True, rest.rstrip("\n")


def is_solution_path(path: Path) -> bool:
    return "solutions" in path.parts


def iter_source_files(root: Path) -> list[Path]:
    return sorted(p for p in root.rglob("*.sokonanoda") if p.is_file())


def _normal(text: str) -> str:
    """规范化：去空白、去行内 markdown 粗体/行内代码标记，用于重复检测。"""
    text = re.sub(r"\*\*|`", "", text)
    return re.sub(r"\s+", "", text)


def collect_prose_paragraphs(
    lines: Sequence[str], first_line: int = 1
) -> list[Paragraph]:
    """把连续的非空散文注释行合并成段落。

    * `soko:hint` 及其缩进续行单独成段（kind=hint），不参与段落重复检测；
    * 装饰线单独成段（kind=title）；
    * 首行单元标题（kind=title）不参与重复检测。
    """
    paragraphs: list[Paragraph] = []
    buf: list[str] = []
    start = 0
    in_hint = False
    hint_label = ""
    hint_body: list[str] = []
    hint_start = 0

    def flush_prose() -> None:
        nonlocal buf, start
        if buf:
            paragraphs.append(Paragraph(start_line=start, text="\n".join(buf)))
            buf = []
            start = 0

    def flush_hint() -> None:
        nonlocal in_hint, hint_body, hint_label
        if in_hint and hint_body:
            paragraphs.append(
                Paragraph(
                    start_line=hint_start,
                    text="\n".join(hint_body),
                    kind="hint",
                    hint_label=hint_label,
                )
            )
        in_hint = False
        hint_body = []
        hint_label = ""

    for offset, raw in enumerate(lines):
        lineno = first_line + offset
        is_comment, body = strip_comment(raw)
        if not is_comment:
            flush_prose()
            flush_hint()
            continue
        stripped = body.strip()
        if not stripped:
            flush_prose()
            flush_hint()
            continue
        if stripped.startswith(HINT_PREFIX):
            flush_prose()
            flush_hint()
            rest = stripped[len(HINT_PREFIX):].strip()
            label = ""
            match = re.match(r"^([^：:]{1,16})[：:]\s*(.*)$", rest)
            if match:
                label = match.group(1).strip()
                rest = match.group(2).strip()
            in_hint = True
            hint_start = lineno
            hint_label = label
            hint_body = [rest] if rest else []
            continue
        if in_hint and body.startswith((" ", "\t")):
            hint_body.append(stripped)
            continue
        flush_hint()
        if DECOR_RE.match(body):
            flush_prose()
            paragraphs.append(
                Paragraph(start_line=lineno, text=body.strip(), kind="title")
            )
            continue
        if offset == 0 and TITLE_RE.match(stripped):
            flush_prose()
            paragraphs.append(
                Paragraph(start_line=lineno, text=stripped, kind="title")
            )
            continue
        if not buf:
            start = lineno
        buf.append(stripped)
    flush_prose()
    flush_hint()
    return paragraphs


def scan_banned_phrases(
    path_label: str,
    lines: Sequence[str],
    report: LintReport,
) -> None:
    """逐行扫雷点正则；白名单按命中处 ±window 字符判断。"""
    for offset, raw in enumerate(lines):
        lineno = offset + 1
        is_comment, body = strip_comment(raw)
        if not is_comment or not body.strip():
            continue
        for rule in BANNED_RULES:
            if rule["category"] == "over-formatting":
                continue  # 行级过度格式由 check_structure 统一判
            match = rule["pattern"].search(body)
            if not match:
                continue
            start = max(0, match.start() - 2)
            end = min(len(body), match.end() + 2)
            snippet = body[start:end].strip()
            hit = Hit(
                path=path_label,
                line=lineno,
                rule=rule["id"],
                category=rule["category"],
                message=rule["why"],
                snippet=snippet,
            )
            if rule.get("warn_only"):
                report.warn(hit)
            else:
                report.add(hit)


def check_clear_justification(
    path_label: str, lines: Sequence[str], report: LintReport
) -> None:
    """「显然/clearly」必须带理由：段落内 ±window 找不到理由才判红。"""
    rule = next(r for r in ALLOWLIST if r["rule"] == "zh-clear-with-reason")
    pattern = rule["pattern"]
    allow = rule["allow"]
    window = rule["window"]
    paragraphs = collect_prose_paragraphs(lines)
    for para in paragraphs:
        if para.kind != "prose":
            continue
        if not pattern.search(para.text):
            continue
        for match in pattern.finditer(para.text):
            around = para.text[max(0, match.start() - window): match.end() + window]
            if allow.search(around):
                continue
            # 报告该段落首行；snippet 给出命中词附近 50 字。
            start = max(0, match.start() - 20)
            snippet = para.text[start: match.end() + 20].replace("\n", " ")
            report.add(
                Hit(
                    path=path_label,
                    line=para.start_line,
                    rule="zh-clear-unjustified",
                    category="unjustified-clear",
                    message=(
                        "「显然/clearly」没有马上给出理由：数学写作用词本身允许，"
                        "但必须让读者能核验（补『因为…』或改成定义性展开）。"
                    ),
                    snippet=snippet.strip(),
                )
            )


MATH_HINT_RE = re.compile(r"[=≤≥∈∀∃→←×·±÷≠∧∨⊆⊂∪∩]|`")


def looks_like_math(text: str) -> bool:
    """带代码标识符或数学符号的段落：重复陈述按『数学事实重复』记录、不判红。"""
    return bool(MATH_HINT_RE.search(text))


def check_structure(
    path: Path,
    lines: Sequence[str],
    report: LintReport,
) -> None:
    """行级结构：ASCII 分隔线、文件名抬头、相邻重复、模板口条、过度格式。"""
    path_label = display_path(path)
    paragraphs = collect_prose_paragraphs(lines)
    prev_prose: Paragraph | None = None
    prev_rule_line: int | None = None
    for para in paragraphs:
        norm = _normal(para.text)
        if para.kind == "title" and DECOR_RE.match(para.text):
            # 相邻重复装饰线（同一行 ───── 连写两遍）
            if prev_rule_line is not None and para.start_line == prev_rule_line + 1:
                report.add(
                    Hit(
                        path=path_label,
                        line=para.start_line,
                        rule="redundant-rule",
                        category="redundant-rule",
                        message="相邻重复的装饰线（书页会渲染成两条 <hr>），删掉一条。",
                        snippet=para.text[:40],
                    )
                )
            prev_rule_line = para.start_line
            prev_prose = None
            continue
        prev_rule_line = None
        if para.kind != "prose":
            prev_prose = None
            continue
        # 相邻重复散文段（整段逐字相同）
        if prev_prose is not None and norm and _normal(prev_prose.text) == norm:
            report.add(
                Hit(
                    path=path_label,
                    line=para.start_line,
                    rule="adjacent-duplicate-paragraph",
                    category="adjacent-duplicate",
                    message="与上一段逐字重复的散文段（读者会明显感到『卡带』）。",
                    snippet=para.text.replace("\n", " ")[:60],
                )
            )
        prev_prose = para
    # 行级检查
    prose_seen: dict[str, int] = {}
    for offset, raw in enumerate(lines):
        lineno = offset + 1
        is_comment, body = strip_comment(raw)
        if not is_comment:
            continue
        stripped = body.strip()
        if not stripped:
            continue
        if ASCII_SEP_RE.match(body):
            report.add(
                Hit(
                    path=path_label,
                    line=lineno,
                    rule="ascii-separator",
                    category="separator-noise",
                    message=(
                        "ASCII 分隔线（==== / ---- / ****）是文件头噪声；"
                        "章节结构用一行小标题即可。"
                    ),
                    snippet=stripped[:60],
                )
            )
            continue
        if FILENAME_HEADER_RE.match(stripped) and lineno <= 3:
            report.add(
                Hit(
                    path=path_label,
                    line=lineno,
                    rule="filename-header",
                    category="separator-noise",
                    message=(
                        "文件抬头把文件名写进正文（`unitNN-....sokonanoda ——`）；"
                        "读者只需要一行中文单元标题。"
                    ),
                    snippet=stripped[:70],
                )
            )
            continue
        if TEMPLATE_BOILERPLATE_RE.search(stripped):
            report.add(
                Hit(
                    path=path_label,
                    line=lineno,
                    rule="L3-boilerplate",
                    category="template-boilerplate",
                    message=(
                        "课程工程口条模板（L3 分层规则）重复出现在读者可见段落里；"
                        "分层规则归 `docs/design/course-stdlib.md`，正文只留一句自然的话。"
                    ),
                    snippet=stripped[:70],
                )
            )
        if stripped.startswith(HINT_PREFIX) or DECOR_RE.match(body):
            continue
        norm = _normal(stripped)
        if norm:
            prose_seen[norm] = prose_seen.get(norm, 0) + 1
        # 过度格式：粗体整行 / 行内小标题清单 / markdown 标题等
        for rule in BANNED_RULES:
            if rule["category"] != "over-formatting":
                continue
            match = rule["pattern"].search(body)
            if not match:
                continue
            if rule["id"] == "md-overformat" and DECOR_RE.match(body):
                continue
            report.add(
                Hit(
                    path=path_label,
                    line=lineno,
                    rule=rule["id"],
                    category="over-formatting",
                    message=rule["why"],
                    snippet=stripped[:70],
                )
            )
    # 冒号 + 连续 >=3 条 `·` 列表：讲稿提纲式罗列。
    bullet_run = 0
    trigger_line = 0
    for offset, raw in enumerate(lines):
        is_comment, body = strip_comment(raw)
        if not is_comment:
            continue
        stripped = body.strip()
        if not stripped:
            continue
        if stripped.startswith(("·", "•")):
            if bullet_run == 0:
                trigger_line = offset + 1
            bullet_run += 1
            continue
        if bullet_run >= 3 and trigger_line > 1:
            prev_is_comment, prev_body = strip_comment(lines[trigger_line - 2])
            if prev_is_comment and prev_body.strip().endswith(("：", ":")):
                report.add(
                    Hit(
                        path=path_label,
                        line=trigger_line - 1,
                        rule="zh-list-after-colon",
                        category="meta-writing",
                        message="『……：』+ ≥3 条裸列表：讲稿提纲式罗列；改成叙述段，条目之间补『因为/于是/反过来』。",
                        snippet=prev_body.strip()[:60],
                    )
                )
        bullet_run = 0
    if bullet_run >= 3 and trigger_line > 1:
        prev_is_comment, prev_body = strip_comment(lines[trigger_line - 2])
        if prev_is_comment and prev_body.strip().endswith(("：", ":")):
            report.add(Hit(path=path_label, line=trigger_line - 1, rule="zh-list-after-colon", category="meta-writing", message="『……：』+ ≥3 条裸列表：讲稿提纲式罗列。", snippet=prev_body.strip()[:60]))

    # 同一文件内重复的散文行（≥3 次；提示模板行除外）
    for norm, count in prose_seen.items():
        if count < 3:
            continue
        if norm.startswith(HINT_PREFIX) or len(norm) < 10:
            continue
        first_line = None
        for offset, raw in enumerate(lines):
            is_comment, body = strip_comment(raw)
            if is_comment and _normal(body.strip()) == norm:
                first_line = offset + 1
                break
        if first_line is None:
            continue
        report.add(
            Hit(
                path=path_label,
                line=first_line,
                rule="repeated-line",
                category="repetition",
                message=f"同一文件里同一句散文出现 {count} 次（模板复制痕迹）。",
                snippet=norm[:60],
            )
        )
    # 同一文件内重复的散文段（非相邻）
    seen_paragraphs: dict[str, Paragraph] = {}
    for para in paragraphs:
        if para.kind != "prose":
            continue
        norm = _normal(para.text)
        if not norm:
            continue
        previous = seen_paragraphs.get(norm)
        if previous is not None:
            hit = Hit(
                path=path_label,
                line=para.start_line,
                rule="duplicate-paragraph",
                category="duplicate-prose",
                message=(
                    f"与第 {previous.start_line} 行的散文段重复（非相邻也算模板重复）。"
                ),
                snippet=para.text.replace("\n", " ")[:60],
            )
            if looks_like_math(para.text):
                # 例外（明说、记录、不判红）：数学事实/引理说明在不同声明下各带
                # 一份是有意重复；删掉会让读者失去上下文。
                hit.rule = "duplicate-math-paragraph"
                hit.category = "math-repeat-exception"
                hit.message = (
                    f"数学事实说明与第 {previous.start_line} 行相同——按设计保留"
                    "（不做静默删除，仅记录）。"
                )
                report.warn(hit)
            else:
                report.add(hit)
        else:
            seen_paragraphs[norm] = para


def check_long_paragraphs(
    path: Path, lines: Sequence[str], report: LintReport
) -> None:
    paragraphs = collect_prose_paragraphs(lines)
    for para in paragraphs:
        if para.kind != "prose":
            continue
        text = _normal(para.text)
        if len(text) <= MAX_PROSE_CHARS:
            continue
        report.add(
            Hit(
                path=display_path(path),
                line=para.start_line,
                rule="long-paragraph",
                category="long-paragraph",
                message=(
                    f"散文段 {len(text)} 字符 > {MAX_PROSE_CHARS}（与书页同一上限）："
                    "拆成 2–3 段，一段只说一件事。"
                ),
                snippet=para.text.replace("\n", " ")[:60],
            )
        )


def check_unit_lead(
    path: Path, lines: Sequence[str], report: LintReport
) -> None:
    """单元画布必须有导语段：第一段正经散文（非标题/非工程注记/非提示）。"""
    path_label = display_path(path)
    # 找第一个声明行（忽略 import/注释/空行）。
    first_decl = None
    for offset, raw in enumerate(lines):
        stripped = raw.strip()
        if not stripped or stripped.startswith(COMMENT_PREFIX):
            continue
        if IMPORT_RE.match(stripped):
            continue
        if DECL_RE.match(stripped):
            first_decl = offset
        # 无论第一个非注释行是什么（比如 import 后接声明），都视为正文开始
        break
    limit = first_decl if first_decl is not None else len(lines)
    paragraphs = collect_prose_paragraphs(lines[:limit])
    for para in paragraphs:
        if para.kind != "prose":
            continue
        text = _normal(para.text)
        if len(text) < MIN_LEAD_CHARS:
            continue
        if COURSE_NOTE_RE.match(para.text):
            continue
        return
    report.add(
        Hit(
            path=path_label,
            line=1,
            rule="missing-unit-lead",
            category="missing-unit-lead",
            message=(
                "单元画布缺少导语段：第一个声明之前要有一两段『本单元讲什么、为什么』"
                "的正文散文（不能只有标题行/工程注记/提示）。"
            ),
            snippet="",
        )
    )


def check_source_file(path: Path, report: LintReport, root: Path) -> int:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()
    label = display_path(path)
    scan_banned_phrases(label, lines, report)
    check_clear_justification(label, lines, report)
    check_structure(path, lines, report)
    check_long_paragraphs(path, lines, report)
    if not is_solution_path(path):
        check_unit_lead(path, lines, report)
    return len(lines)


# ── HTML 模式：只读可见文本 ──────────────────────────────────────────────────

SKIP_TAGS = {"script", "style", "pre", "code", "head", "textarea", "svg"}
BLOCK_TAGS = {"p", "li", "h1", "h2", "h3", "h4", "h5", "h6", "div", "td", "th",
              "blockquote", "summary", "figcaption"}
SEPARATOR_TEXT_RE = re.compile(r"^[\s=\-_─━*]{4,}$")
LITERAL_MARKER_RE = re.compile(r"\*\*|__|soko:hint")


class BookHTMLParser(HTMLParser):
    """收集书页里读者可见的段落块 + 单元归属 + <hr> 计数。"""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.skip_depth = 0
        self.unit_stack: list[str] = []
        self.current_unit = ""
        self.current_block: list[str] | None = None
        self.current_tag = ""
        self.blocks: list[dict] = []
        self.hr_count = 0
        self.unit_count = 0

    def handle_starttag(self, tag: str, attrs) -> None:
        data = dict(attrs)
        if tag in SKIP_TAGS:
            self.skip_depth += 1
            return
        if tag == "hr":
            self.hr_count += 1
            return
        if tag == "section":
            classes = str(data.get("class") or "").split()
            entry = ""
            if "unit" in classes:
                self.unit_count += 1
                entry = str(data.get("id") or f"unit-{self.unit_count}")
            self.unit_stack.append(entry)  # 嵌套 section 也要入栈，出栈才不串位
            if entry:
                self.current_unit = entry
        if tag in BLOCK_TAGS and self.current_block is None:
            self.current_tag = tag
            self.current_block = []

    def handle_endtag(self, tag: str) -> None:
        if tag in SKIP_TAGS:
            self.skip_depth = max(0, self.skip_depth - 1)
            return
        if tag == "section" and self.unit_stack:
            self.unit_stack.pop()
            self.current_unit = next(
                (unit for unit in reversed(self.unit_stack) if unit), ""
            )
            return
        if tag in BLOCK_TAGS and self.current_block is not None:
            text = "".join(self.current_block).strip()
            if text:
                self.blocks.append(
                    dict(unit=self.current_unit, tag=self.current_tag, text=text)
                )
            self.current_block = None

    def handle_data(self, data: str) -> None:
        if self.skip_depth:
            return
        if self.current_block is not None:
            self.current_block.append(data)


def display_path(path: Path) -> str:
    try:
        return path.resolve().relative_to(REPO_ROOT).as_posix()
    except (ValueError, OSError):
        try:
            return path.resolve().as_posix()
        except OSError:
            return path.as_posix()


def lint_source(root: Path) -> tuple[LintReport, dict]:
    report = LintReport()
    files = iter_source_files(root)
    if not files:
        raise SystemExit(f"style-lint: 在 {root} 下没有找到 *.sokonanoda")
    units = [f for f in files if not is_solution_path(f)]
    total_lines = sum(check_source_file(f, report, root) for f in files)
    comment_lines = 0
    for path in files:
        for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
            if raw.lstrip().startswith(COMMENT_PREFIX):
                comment_lines += 1
    report.stats = dict(
        mode="source",
        root=str(root),
        files=len(files),
        unit_files=len(units),
        solution_files=len(files) - len(units),
        file_lines=total_lines,
        comment_lines=comment_lines,
    )
    return report, report.stats


def lint_html(path: Path) -> tuple[LintReport, dict]:
    report = LintReport()
    parser = BookHTMLParser()
    parser.feed(path.read_text(encoding="utf-8", errors="replace"))
    label = display_path(path)
    # 段落级：重复 / 超长 / 分隔线 / 字面标记 / 雷点
    prev_by_unit: dict[str, str] = {}
    literal_hits = 0
    separator_hits = 0
    paragraphs = [b for b in parser.blocks if b["tag"] in ("p", "li")]
    for block in paragraphs:
        text = re.sub(r"\s+", "", block["text"])
        if not text:
            continue
        unit = block["unit"]
        if not unit:
            continue  # 封面 / 目录 / 附录导航不是正文散文，不做重复与长度判据
        if SEPARATOR_TEXT_RE.match(text):
            separator_hits += 1
            report.add(
                Hit(
                    path=label,
                    line=0,
                    rule="html-ascii-separator",
                    category="separator-noise",
                    message="书页可见文本里出现 ASCII 分隔线（应在源注释里删掉）。",
                    snippet=text[:40],
                )
            )
            continue
        marker = LITERAL_MARKER_RE.search(block["text"])
        if marker:
            literal_hits += 1
            report.add(
                Hit(
                    path=label,
                    line=0,
                    rule="html-literal-marker",
                    category="html-marker",
                    message=f"书页可见文本漏出标记 {marker.group(0)!r}（渲染层没吃掉）。",
                    snippet=block["text"][:60],
                )
            )
        norm = _normal(block["text"])
        if norm and prev_by_unit.get(unit) == norm:
            report.add(
                Hit(
                    path=label,
                    line=0,
                    rule="html-adjacent-duplicate",
                    category="adjacent-duplicate",
                    message="书页同一单元内相邻段落逐字重复。",
                    snippet=block["text"][:60],
                )
            )
        prev_by_unit[unit] = norm
        if len(norm) > MAX_PROSE_CHARS:
            report.add(
                Hit(
                    path=label,
                    line=0,
                    rule="html-long-paragraph",
                    category="long-paragraph",
                    message=f"书页散文段 {len(norm)} 字符 > {MAX_PROSE_CHARS}。",
                    snippet=block["text"][:60],
                )
            )
    # 雷点规则：扫全部块文本（含标题/列表），把块归属写进 snippet
    for block in parser.blocks:
        text = block["text"]
        for rule in BANNED_RULES:
            if rule["category"] == "over-formatting":
                continue  # 书页渲染后的可见文本不再有 markdown 源标记
            match = rule["pattern"].search(text)
            if not match:
                continue
            hit = Hit(
                path=label,
                line=0,
                rule="html-" + rule["id"],
                category=rule["category"],
                message=rule["why"],
                snippet=(block["unit"] + ": " + text[max(0, match.start() - 10): match.end() + 10])[:90],
            )
            if rule.get("warn_only"):
                report.warn(hit)
            else:
                report.add(hit)
    report.stats = dict(
        mode="html",
        path=str(path),
        unit_sections=parser.unit_count,
        blocks=len(parser.blocks),
        paragraphs=len(paragraphs),
        hr_count=parser.hr_count,
        literal_markers=literal_hits,
        separator_lines=separator_hits,
    )
    return report, report.stats


# ── 输出 ────────────────────────────────────────────────────────────────────


def print_report(report: LintReport, stats: dict) -> None:
    for hit in report.hits:
        location = f"{hit.path}:{hit.line}" if hit.line else hit.path
        print(f"{location}: [{hit.category}] {hit.message}")
        if hit.snippet:
            print(f"    {hit.snippet}")
    for hit in report.warnings:
        location = f"{hit.path}:{hit.line}" if hit.line else hit.path
        print(f"{location}: [记录·不判红] {hit.message}")
        if hit.snippet:
            print(f"    {hit.snippet}")
    counts = report.by_category()
    total = len(report.hits)
    if stats.get("mode") == "source":
        print(
            f"\n合计 {total} 处 · {stats['files']} 文件"
            f"（单元 {stats['unit_files']} + 解答 {stats['solution_files']}）"
        )
    else:
        print(
            f"\n合计 {total} 处 · {stats.get('unit_sections', 0)} 个单元 section · "
            f"<hr> {stats.get('hr_count', 0)}"
        )
    if counts:
        print("按类别：" + "、".join(f"{k}×{v}" for k, v in counts.items()))


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="课程文案 / 可读性硬门禁（AI 写作雷点 + 结构噪声 + 可读性）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=None,
        help=f"源码扫描根目录（默认 {DEFAULT_ROOT}）",
    )
    parser.add_argument(
        "--html", type=Path, default=None, help="同时扫描生成的书籍 HTML（可见文本）"
    )
    parser.add_argument(
        "--html-only", action="store_true", help="只扫 --html，不扫源码注释"
    )
    parser.add_argument("--json", action="store_true", help="输出 JSON（含计数）")
    parser.add_argument(
        "--selftest", action="store_true", help="跑判据自检（坏夹具必须被判红）"
    )
    parser.add_argument(
        "--show-allowlist", action="store_true", help="打印规则与数学误报白名单"
    )
    parser.add_argument(
        "--dump-rules",
        metavar="PATH",
        type=Path,
        default=None,
        help="把生效规则与白名单导出为 JSON（供书页检查/其它门禁复用）",
    )
    args = parser.parse_args(argv)

    if args.show_allowlist:
        print("判红规则：")
        for rule in BANNED_RULES:
            print(f"  - {rule['id']:28s} [{rule['category']}] {rule['why']}")
        print("\n数学误报白名单（这些词在数学语域里正常，不判红）：")
        for term in ALLOWLIST_TERMS:
            print(f"  - {term}")
        return 0

    if args.dump_rules is not None:
        payload = dict(
            schema="soko.content-style-rules/1",
            generated_by="scripts/content_style/style_lint.py --dump-rules",
            max_prose_chars=MAX_PROSE_CHARS,
            min_lead_chars=MIN_LEAD_CHARS,
            categories=sorted({rule["category"] for rule in BANNED_RULES}),
            rules=[
                dict(
                    id=rule["id"],
                    category=rule["category"],
                    language=(
                        "zh"
                        if rule["id"].startswith("zh-")
                        else "en"
                        if rule["id"].startswith("en-")
                        else "markup"
                    ),
                    pattern=rule["pattern"].pattern,
                    why=rule["why"],
                )
                for rule in BANNED_RULES
            ],
            allowlist=[
                dict(
                    rule=entry["rule"],
                    pattern=entry["pattern"].pattern,
                    allow=entry["allow"].pattern,
                    window=entry["window"],
                    why=entry["why"],
                )
                for entry in ALLOWLIST
            ],
            allowlist_terms=ALLOWLIST_TERMS,
            structural_checks=[
                "separator-noise: ASCII separators (`====`/`----`/`****`) and filename headers",
                "redundant-rule: adjacent duplicate decorative rule lines",
                "adjacent-duplicate: adjacent identical prose lines/paragraphs",
                "template-boilerplate: `L3：律必须留在单元里` policy phrases",
                "missing-unit-lead: unit canvas has no lead paragraph before first declaration",
                "long-paragraph: prose paragraph > MAX_PROSE_CHARS",
                "unjustified-clear: 显然/clearly without nearby reason",
                "over-formatting: markdown headings/links/HTML, bold-only lines, inline-header lists, emoji",
            ],
        )
        args.dump_rules.write_text(
            json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        print(f"rules dumped: {args.dump_rules} ({len(payload['rules'])} rules)")
        return 0

    if args.selftest:
        return selftest()

    reports: list[tuple[LintReport, dict]] = []
    try:
        if args.html is not None:
            if not args.html.is_file():
                print(f"style-lint: --html 文件不存在：{args.html}", file=sys.stderr)
                return 2
            reports.append(lint_html(args.html))
        if not args.html_only:
            root = args.root if args.root is not None else DEFAULT_ROOT
            if not root.is_dir():
                print(f"style-lint: 源码根目录不存在：{root}", file=sys.stderr)
                return 2
            reports.append(lint_source(root))
    except SystemExit as exc:  # noqa: BLE001 - 统一成退出码 2
        print(str(exc), file=sys.stderr)
        return 2

    if args.json:
        payload = dict(
            ok=all(report.ok for report, _ in reports),
            reports=[
                dict(
                    **stats,
                    counts=report.by_category(),
                    errors=[hit.as_dict() for hit in report.hits],
                    warnings=[hit.as_dict() for hit in report.warnings],
                )
                for report, stats in reports
            ],
        )
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        for index, (report, stats) in enumerate(reports):
            if index:
                print()
            print_report(report, stats)
    return 0 if all(report.ok for report, _ in reports) else 1


# ── 判据自检 ────────────────────────────────────────────────────────────────

SELFTEST_GOOD_UNIT = """\
-- 单元⑨ 示例单元：可读性判据的正向夹具
--
-- 本单元示范一段合格的导语：先说要做什么，再说为什么。
-- 因此这里故意给出一个「显然」的合法用例：显然是因为两边逐字相同。
-- 绝对值的记法允许出现；本文件与本文也允许出现；注意这里都在白名单里。
-- 设 ε > 0，定义 d 为误差；定理与证明都按数学语序写。

def demo (n : Nat) : Nat := n
"""

SELFTEST_BAD_UNIT = """\
-- unit07-demo.sokonanoda —— 演示单元
--
-- 众所周知，本文要讲一个很重要的东西。
-- 让我们一起来深入探索这个至关重要的主题，综上所述，它堪称完美。
-- 研究表明，专家认为这具有很高的可读性，是数学中最优美的定理之一。
-- **重点**：下面是很关键的一步。
-- 显然，两边相等。
-- ==========================================================================
-- 这里是正文。
--
-- 这里是正文。
"""

SELFTEST_SOLUTION_OK = """\
import lib.Logic

theorem demo (n : Nat) : n = n :=
  Eq.refl n
"""


def _lint_text_as_source(text: str, filename: str = "unit_fixture.sokonanoda") -> LintReport:
    report = LintReport()
    with tempfile.TemporaryDirectory(prefix="style-lint-") as tmp:
        path = Path(tmp) / filename
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        lines = text.splitlines()
        label = filename
        scan_banned_phrases(label, lines, report)
        check_clear_justification(label, lines, report)
        check_structure(path, lines, report)
        check_long_paragraphs(path, lines, report)
        if not is_solution_path(path):
            check_unit_lead(path, lines, report)
    return report


def _selftest_html() -> list[str]:
    problems: list[str] = []
    good = """<!doctype html><html><body>
<section class="unit" id="unit-1"><p>导语：本单元讲一件事。</p>
<p>绝对值与本文都允许出现；注意这里的数学用词。</p></section>
</body></html>"""
    bad = """<!doctype html><html><body>
<section class="unit" id="unit-1"><p>导语：本单元讲一件事。</p>
<p><strong>重要</strong>：pivotal 的 delve 与 tapestry。</p>
<p>============</p>
<p>soko:hint 思路：**重点**。</p>
<p>这一句话是重复的。</p><p>这一句话是重复的。</p></section>
</body></html>"""
    with tempfile.TemporaryDirectory(prefix="style-lint-html-") as tmp:
        good_path = Path(tmp) / "good.html"
        good_path.write_text(good, encoding="utf-8")
        report, _stats = lint_html(good_path)
        if not report.ok:
            problems.append(
                "HTML 正向夹具被误判：" + ", ".join(h.as_dict()["rule"] for h in report.hits)
            )
        bad_path = Path(tmp) / "bad.html"
        bad_path.write_text(bad, encoding="utf-8")
        report, _stats = lint_html(bad_path)
        rules = {hit.rule for hit in report.hits}
        expected = {
            "html-literal-marker",
            "html-ascii-separator",
            "html-adjacent-duplicate",
            "en-marketing",
            "html-en-marketing",
            "zh-marketing",
            "html-zh-marketing",
        }
        if not (expected & rules):
            problems.append("HTML 负向夹具没有被判红（一条规则都没命中）")
        if not report.ok:
            pass
        else:
            problems.append("HTML 负向夹具被判绿（判据通道失效）")
    return problems


def selftest() -> int:
    """故意坏的正向/负向夹具；任何一条不按预期就判自检失败。"""
    problems: list[str] = []
    # 1) 正向：干净单元必须判绿（白名单误报测试）
    report = _lint_text_as_source(SELFTEST_GOOD_UNIT)
    if not report.ok:
        problems.append(
            "正向夹具被误判："
            + ", ".join(f"{h.rule}@{h.line}" for h in report.hits)
        )
    # 2) 解答文件没有导语要求，必须判绿
    report = _lint_text_as_source(
        SELFTEST_SOLUTION_OK, "solutions/II.1/unit01-solution.sokonanoda"
    )
    if not report.ok:
        problems.append(
            "解答正向夹具被误判："
            + ", ".join(f"{h.rule}@{h.line}" for h in report.hits)
        )
    # 3) 负向：坏单元必须全部命中且判红
    report = _lint_text_as_source(SELFTEST_BAD_UNIT)
    rules = {hit.rule for hit in report.hits}
    expected = {
        "ascii-separator",
        "filename-header",
        "zh-empty-opener",
        "zh-meta-ai",
        "zh-marketing",
        "zh-fake-evidence",
        "zh-translationese",
        "zh-overformat-header-list",
        "zh-clear-unjustified",
    }
    missing = expected - rules
    if missing:
        problems.append("负向夹具漏判：" + ", ".join(sorted(missing)))
    if report.ok:
        problems.append("负向夹具被判绿（判据通道失效）")
    # 4) 缺导语
    no_lead = "-- 单元⑧ 只有标题和工程注记\n--\n-- 本单元自带定义与前序地基。\n\ndef x : Nat := 0\n"
    report = _lint_text_as_source(no_lead)
    if "missing-unit-lead" not in {hit.rule for hit in report.hits}:
        problems.append("缺导语夹具没有被判红")
    # 5) 长段落
    long_para = "-- 单元⑦ 长段落夹具\n--\n-- " + "这是很长的说明。" * 200 + "\n\ndef x : Nat := 0\n"
    report = _lint_text_as_source(long_para)
    if "long-paragraph" not in {hit.rule for hit in report.hits}:
        problems.append(f"长段落夹具没有被判红（阈值 {MAX_PROSE_CHARS}）")
    # 6) 相邻重复装饰线
    dup_rule = (
        "-- 单元⑥ 装饰线夹具\n--\n-- 导语一段：本单元讲一件事，所以这里有一段合格的导语。\n--\n"
        "-- ────────────────────────────────────────────\n"
        "-- ────────────────────────────────────────────\n"
        "def x : Nat := 0\n"
    )
    report = _lint_text_as_source(dup_rule)
    if "redundant-rule" not in {hit.rule for hit in report.hits}:
        problems.append("相邻重复装饰线没有被判红")
    # 7) 模板口条
    boilerplate = (
        "-- 单元⑤ 模板口条夹具\n--\n-- 导语一段：本单元讲一件事，所以这里有一段合格的导语。\n--\n"
        "-- 本单元自带定义与前序地基（L3：律必须留在单元里）。\n"
        "def x : Nat := 0\n"
    )
    report = _lint_text_as_source(boilerplate)
    if "L3-boilerplate" not in {hit.rule for hit in report.hits}:
        problems.append("模板口条（L3）没有被判红")
    # 8) 重复散文段（非相邻）
    dup_para = (
        "-- 单元④ 重复段夹具\n--\n-- 导语一段：本单元讲一件事，所以这里有一段合格的导语。\n--\n"
        "-- 这一段是重复的说明文字。\n--\n-- 中间隔一段别的说明。\n--\n"
        "-- 这一段是重复的说明文字。\n"
        "def x : Nat := 0\n"
    )
    report = _lint_text_as_source(dup_para)
    if "duplicate-paragraph" not in {hit.rule for hit in report.hits}:
        problems.append("重复散文段没有被判红")
    # 9) HTML 模式
    problems.extend(_selftest_html())
    # 10) 元写作泄漏（owner 2026-11-12）
    meta = (
        "-- 单元③ 元写作夹具\n--\n"
        "-- 本单元只做一件事：把整数看成 ℕ×ℕ 按差相等作商。\n"
        "-- 这里先给出定义，再给出证明。\n\n"
        "def x : Nat := 0\n"
    )
    report = _lint_text_as_source(meta)
    rules = {hit.rule for hit in report.hits}
    if not ({"zh-meta-writing-leak", "zh-meta-label"} & rules):
        problems.append("元写作泄漏夹具没有被判红")

    # 11) 高钩子开场（owner 2026-11-12）
    hook = (
        "-- 单元⑨ 钩子夹具\n--\n"
        "-- 你有没有想过，数学其实可以这样看？\n"
        "-- 这一单元讲一件事，所以这里有一段合格的导语。\n\n"
        "def x : Nat := 0\n"
    )
    report = _lint_text_as_source(hook)
    if "zh-hook-opening" not in {hit.rule for hit in report.hits}:
        problems.append("高钩子开场夹具没有被判红")

    if problems:
        print("style-lint --selftest: 失败", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    print("style-lint --selftest: 通过（正向夹具绿、负向夹具红、白名单不误伤）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
