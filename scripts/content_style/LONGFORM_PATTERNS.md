> **Not adopted as a hook playbook (owner correction 2026-11-12).** Engagement is a weak, polarizing signal; we keep only structural clarity observations (one idea per section, concrete example, claim headings). No hook/clickbait/emoji/engagement mechanics in the book. The standard is clarity for more readers, not exhaustive knowledge lists.

# 平台验证过的长文模式（C8.4 增量研究）

> **这份文件回答**：除了专家书单（见 `GOOD_WRITING_GUIDE.md` §1），**被大量读者投票验证过**
> 的长文长什么样？我们可以从里面抄哪些**结构**进数学书？
>
> **样本**：2026-11-12 经代理抓取并拆解 **14 篇**（另 1 篇付费墙不可读、3 个平台不可达），
> 覆盖 HN / Quanta / Aeon / Longreads / Substack / Medium / dev.to / 掘金 / 少数派。
> 完整逐篇拆解在 `/tmp/longform-deconstruction.md`（交付副本）；本文件是可直接用于
> 书稿/单元的**结论版**。
>
> **警告**：票数是「被读者验证过可读」的**弱信号**，不是质量证明。§5 列了全部偏差。

## 1. 样本与公开信号（实抓，2026-11-12）

| 平台 | 篇目（短标题） | 公开信号 |
|---|---|---|
| HN（个人博客） | ciechanow.ski《Mechanical Watch》2022-05-04 | 4,298 pts / 413 comments |
| HN（个人博客） | nee.lv《How I cut GTA Online loading times by 70%》2021-03-01 | 3,883 pts / 699 c |
| HN（Substack） | dkb《Google Search Is Dying》2022-02-15 | 3,636 pts / 1,561 c |
| HN（个人博客） | tbray《Bye, Amazon》2020-05-04 | 3,816 pts / 1,097 c |
| HN（个人博客） | moxie《My First Impressions of Web3》2022-01-07 | 3,393 pts / 1,127 c |
| Quanta | 《Undergraduate Upends a 40-Year-Old Data Science Conjecture》2025-02-10 | HN 1,304 pts / 584 c |
| Quanta | 《Math Is Still Catching Up to the Genius of Ramanujan》2024-10-21 | HN 826 pts / 414 c |
| Aeon | 《What I learned as a hired consultant to autodidact physicists》2016-08-11 | HN 765 pts / 396 c |
| Longreads | 《The Second Half of Watergate Was Bigger, Worse, and Forgotten》2018-11-20 | HN 479 pts / 237 c |
| Substack | 《The Tyranny of the Marginal User》2023-09-14 | HN 1,576 pts / 815 c |
| Medium | 《My experience as a Gazan girl getting into Silicon Valley companies》2021-02-24 | HN 1,723 pts / 460 c |
| dev.to | 《You're Not Building Netflix: Stop Coding Like You Are》2025-11-23 | 352 reactions / 107 c |
| 掘金 | 《手机到底有没有偷听我们？我抓包了 72 小时，抓到了真凶》2026-10-20 | 16,752 阅读 / 252 赞 / 201 收藏 / 40 评论 |
| 少数派 | 《从优胜美地到金门大桥：跟着 macOS 游览加州》2026-11-11 | 25 分钟阅读（点赞/阅读数 API 不暴露） |

付费墙只读到标题（不拆解）：The Atlantic《The Coders Programming Themselves Out of a Job》
（HN 991 pts）。不可达：Reddit（403）、知乎（安全验证）、微信公众号（无公开入口）。
原文链接见 `/tmp/longform-deconstruction.md` §7。

## 2. 十四条可迁移模式

1. **第一屏给钩子**：反常识断言 / 具体场景 / 真实代价——不是「本章讲 X」的陈述。
2. **第一屏给承诺 + 路线图**：读完能做到什么、分几步走。
3. **一节一个问题/部件**：小标题用**问句或动作**，不用名词堆。
4. **例子先于抽象**：最小具体例子（具体数字/场景）先算一遍，再上定义。
5. **反直觉 + 证据**：先立「大家以为的」，再用数据/证明推翻。
6. **短段 + 小标题 + 留白**：2–4 句一段；一行一句的节奏优于工整大段。
7. **术语即时解释、允许读者忘**：新记号给「记法 → 人话」，不要求先背完。
8. **可操作收尾**：tl;dr / 检查清单 / 「你现在应该能…」。
9. **结尾回扣开头并承认代价**：先承认缺点/边界，再给意义（不是反过来）。
10. **第一人称与不确定性**：把读者的困惑说出来（「这里容易误以为…」）。
11. **视觉/交互优先于文字**：每单元至少一个可看的图/表/具体例（书页侧决定形态）。
12. **元信息/证据档案块**：实验装置、基准、来源、AI 润色声明——透明本身是信用。
13. **问题式小标题与回指**：每节末「所以现在我们知道…」，下一节接上一节。
14. **一句可转述的收束**：具体断言，不是「综上所述」。

> 平台机制（关注引导、emoji 标题、评论区互动、SEO）**不迁移**进书——那是平台激励。

## 3. 我们的书/单元模板（建议）

**章**：① 具体标题（动作/问题）→ ② 章首钩子（2–4 句）→ ③ 路线图（3–5 项）→
④ 贯穿例子（全章反复回到它）→ ⑤ 主题节（问句小标题，每节一句「所以…」）→
⑥ 边界与代价汇总 → ⑦ 回扣 + 下一章悬念。

**单元**（现有结构的最小增量，**只增 `--` 注释与顺序，不改声明/证明**）：

```
-- 单元N 标题（具体 + 问句感）
-- ① 钩子（1–2 句：反常识/具体例子/代价）
-- ② 路线图（3–4 项：依赖谁、读完成就什么）
-- ③ 最小例子（具体数字先算一遍）        ← 提前/新增
-- ④ 定义与记号（只给用得到的；术语给一句人话）
-- ⑤ 主结果与证明（一段一步；目标形态→工具；soko:hint 三件套保持）
-- ⑥ 边界（明说做不到什么/用什么换的/洞在哪）
-- ⑦ takeaway（一句可复述的话）+ 下一单元问题
-- 画布练习 / 解答（格式不变）
```

书页渲染无需新语法：①–⑦ 都落在现有 `prose / heading / hint / rule` 四类 item 里。

## 4. 五个 before/after（来自我们的单元；完整版见 `/tmp/longform-deconstruction.md` §5）

| # | 位置 | 模式 | 改动 |
|---|---|---|---|
| R1 | `II.5/unit48`（㊽ 三角不等式） | P1+P4 | 把已有的「见证相加」反例**提前到开场**，再给公式与证明路线 |
| R2 | `II.5/unit54`（有理数加法结合律） | P1+P6 | 用问句「先加哪两个，结果一样吗？」开场；4 行长句拆 3 句 |
| R3 | `II.2/unit26`（ℚ 加法代表元公式） | P1+P7 | 从「分母为什么变成 (m+1)(n+1)」进入公式；给通分的人话 |
| R4 | `II.7/unit72`（完备性盘点） | P2+P14 | 第一屏先给「本单元问什么 + 结论先说」，再列三件事 |
| R5 | `II.7/unit69`（数系链） | P1+P13 | 用具体数字 `2` 走四层当钩子（同少数派「壁纸→地理」） |

五个改写都只动 `--` 注释；数学陈述与 `soko:hint` 不动。

## 5. 偏差与诚实边界

1. **票数 ≠ 质量**：受发布时间/标题/社区情绪/平台推荐影响；只当「被大量读者验证过可读」。
2. **幸存者偏差**：只看得到火了的长文；同质量没火的看不到。
3. **受众偏差**：HN 偏工程师、掘金偏中文开发者、Quanta 偏科学爱好者；我们的读者是按顺序学
   数学的本科生——**平台营销动作不迁移**。
4. **平台机制污染**：关注引导/SEO 标题/互动提问是平台激励，不是写作原则。
5. **可读性 ≠ 数学正确**：数学正确性仍由课程门禁 + kernel + 人读负责。
6. **抓取缺口**：Reddit/知乎/公众号不可达；Atlantic 付费墙；少数派点赞/阅读数不暴露。
7. **样本量与实验**：14 篇、无读者实验；模式是**假设**，需书页上线后以阅读数据/人读采样复核。

## 6. 与其它交付物的关系

- `AI_WRITING_BAN_LIST.md`：**下限**（雷点/噪声，机器判红）。
- `GOOD_WRITING_GUIDE.md`：**专家书单**给出的正向规则 + 六维评分表。
- 本文件：**读者投票**给出的结构模式 + 书/单元模板（证据来自平台公开信号）。
- `style_lint.py`：只负责下限；本文件的模板不改变判据。
