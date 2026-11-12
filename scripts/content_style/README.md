# scripts/content_style —— 课程文案 / 可读性门禁（C8.4）

两件事：

1. **硬门禁**：`style_lint.py` 扫 `courses/analysis/units/**` 的 `--` 散文注释与
   生成书页 HTML 的可见文本，命中 AI 写作雷点 / 分隔线噪声 / 模板重复 / 缺导语 /
   超长段落就 **exit 1**。
2. **人读文档**：
   * `AI_WRITING_BAN_LIST.md` —— 雷点分类、确切短语/正则、为什么是痕迹、数学误报
     白名单、出处（研究可用性如实记录）。
   * `GOOD_WRITING_GUIDE.md` —— **正向**写作标准：通用规则 + 数学说明文规则 +
     中文翻译腔对策 + 本项目 before/after 改写 + 六维可读性评分表。
     **可读性以这份评分表为准；linter 只是下限。**
   * `LONGFORM_PATTERNS.md` —— **平台验证**的长文模式（HN/Quanta/Aeon/Substack/掘金/
     少数派等 14 篇 + 公开信号）：14 条可迁移模式 + 书/单元模板 + 5 个改写。
     票数是弱信号；偏差与不可达平台如实记录。

## 用法

```bash
# 源码模式（默认扫 courses/analysis/units；0 = 干净，1 = 判红，2 = 用法/IO）
python3 scripts/content_style/style_lint.py

# 机器可读：errors（判红）+ warnings（记录但判红，如数学事实重复陈述）
python3 scripts/content_style/style_lint.py --json

# 同时扫生成书页（可见文本；跳过 script/style/pre/code/head/textarea）
python3 scripts/content_style/style_lint.py --html dist/book/book-v2/index.html

# 只扫 HTML（书页 agent 单独跑时用）
python3 scripts/content_style/style_lint.py --html scripts/book/preview/index.html --html-only

# 判据自检：正向夹具必须绿、负向夹具必须红、白名单不许误伤
python3 scripts/content_style/style_lint.py --selftest

# 打印当前生效规则 + 数学白名单
python3 scripts/content_style/style_lint.py --show-allowlist
```

脚本只用 Python 标准库（无外部依赖），可在仓库根或任意目录调用；默认路径由
脚本自身位置推导。

## 边界（谁判什么）

* 判：散文注释的**文字形态**——空泛话术、模板结构、重复、噪声、过度格式、
  缺导语、长段落、「显然」没给理由。
* **不判**：Lean 声明与证明（`courses/analysis/tools/check.py` 与 kernel 的事）、
  数学正确性（课程门禁 / 人）、书页视觉设计（book presentation，C8.3）。
* **不改**：脚本只读。清理动作由人/agent 显式提交，且**只动 `--` 注释行**
  （C8.4 的清理原则：数学陈述与提示一条不删；重复的数学事实记 warning 不判红）。

## 接入建议（给书页 agent / CI）

书页 `--check` 可以在生成之后加一条：

```bash
python3 scripts/content_style/style_lint.py --html <生成的 index.html> --html-only
```

它只读生成的 HTML，不碰 `scripts/book/**` 与 `scripts/build_book_proto.py`；
判红时输出「单元归属 + 原文块」，与书页自己的 `readability_checks` 互补
（书页查渲染后的段落重复/长段/分隔线可见性；本脚本查源注释的雷点词与模板口条）。

## 已知例外（明说，不静默）

* **数学事实重复陈述**：不同声明下各带一份「前驱：`pred (succ n) = n` …」这类
  说明是有意的（删掉会让读者失去上下文）。linter 记 `math-repeat-exception`
  warning、**不判红**；审计记录在 `/tmp/copy-audit.md`（C8.4 报告）。
* **单条装饰线**（`─` 行）：书页的章节分隔素材，保留；只判「相邻重复」与
  ASCII 抬头噪声。
* **`soko:hint` 模板标签**（思路/目标形态/关键件）：课程提示的既定结构，不判。
