---
name: sokonanoda-figure
description: Author or review professional mathematical figures for Sokonanoda/DSH using TikZ + LaTeX, with Chinese/Latin font pairing, contrast tokens, grid layout, label collision rules, and automated PDF/text-box lint. Use when drawing theorem diagrams, set diagrams, arrows, or when a generated PNG looks amateurish, crowded, or has overlapping text.
---

# sokonanoda-figure

本 skill 是**数学图设计与验收规范**。目标不是“画一张示意图”，而是产出能进教材/讲义的 TikZ 图：字体、层级、间距、标签位置和验收都可复现。

## 0. 先决条件

```bash
tectonic --version        # TeX 驾驶
pdftoppm -v               # PDF → PNG
pdftotext -v              # 文字框 lint
pdftocairo -v             # 矢量路径 lint（可选）
```

缺工具时先报告缺哪个，不要用 HTML/SVG 手摆坐标冒充数学图。

## 1. 默认字体组合

**默认：Latin Modern + 宋体**

```lean
\usepackage{lmodern}
\usepackage{amsmath,amssymb}
\usepackage{xeCJK}
\setCJKmainfont{Songti SC}
```

理由：

- Latin Modern = Computer Modern 的现代可缩放版本，数学/TikZ 最稳；
- Songti SC 与 Latin Modern 的衬线气质一致；
- **不要**用 Computer Modern 数学 + PingFang/黑体中文，中英气质会分裂。

允许的替代：

| 风格 | 英文/数学 | 中文 |
|---|---|---|
| 教材默认 | `lmodern` | `Songti SC` |
| 现代书版 | `newpxtext,newpxmath`（Palatino） | `Songti SC` |
| 紧凑书版 | `newtxtext,newtxmath`（Times） | `Songti SC` |
| 屏幕/幻灯片 | `TeX Gyre Heros` | `PingFang SC` |

换字体必须成对换，不能只换中文。

## 2. 字号与颜色

### 字号

```text
标题        \large\bfseries
公式        \normalsize
正文         \footnotesize
图注/说明    \footnotesize
集合标签     \footnotesize
```

不要为了塞下内容而加粗；先用字号、间距和布局解决。

### 颜色 token

| 角色 | 黑度 | 对比度（白底） |
|---|---|---|
| 标题 / 主公式 | `black!100` | 21.0:1 |
| 正文 / 数学 | `black!96` | 19.8:1 |
| 集合标签 | `black!94` | 19.2:1 |
| 次级说明 | `black!80` | 12.6:1 |

规则：

- 数学符号、集合名、坐标标签一律按 primary（≥94%）；
- `black!65`（7:1）虽然过 AA，但小字号细衬线会发虚，**不用于可读数学文字**；
- 层级靠字号/字重/位置，不靠把字变灰；
- 不加粗也能靠提高黑度解决“看起来浅”。

## 3. 画布与 grid

默认画布：

```text
12.25cm × 5.8cm（220 dpi ≈ 1147 × 588 px）
```

TikZ 用固定 bounding box：

```lean
\useasboundingbox (0,-1.30) rectangle (12.25,4.50);
```

四行 grid：

```text
第 1 行：标题
第 2 行：主公式
第 3 行：图形（左右两栏或单栏）
第 4 行：一句图注
```

- 左右面板中心线对齐；
- 公式与图形之间留固定 gap；
- 图注不超过两行；
- 同一组图必须同画布、同边距、同标题/公式基线。

## 4. 标签与箭头

- 集合标签用白底：

```lean
setlabel/.style={fill=white, inner sep=2pt, text=black!94}
```

- 标签不要压在椭圆/矩形边线上；优先放到集合中心或外侧；
- 点标签 `x`、`f(x)` 放在点下方，不压箭头；
- 箭头标签放在路径上方/外侧，标记 `f` 等；
- 箭头不要穿过文字；
- 箭头头部大小统一，线宽约 0.7–0.8pt。

## 5. 图形语义

每张图先写一句“这张图要证明什么”，再决定元素：

- 集合用椭圆/圆角矩形；
- 子集用嵌套；
- 映射用箭头；
- 原像用“沿箭头回拉”；
- 像用“至少一个原像”；
- 冲突/差异用颜色或高亮，不改变数学符号。

## 6. Lint（硬门禁）

生成后必须跑：

```bash
tectonic figure.tex
pdftoppm -png -r 220 -singlefile figure.pdf figure
pdftotext -bbox figure.pdf - > /tmp/figure-bbox.html
```

检查：

1. 文字框不越出页面；
2. 任意两个文字框不重叠（阈值 > 2pt × 2pt）；
3. 字号高度不低于阈值（约 6pt）；
4. 同一组图画布尺寸一致；
5. 集合标签必须使用白底 label 样式；
6. 箭头标签不与箭头路径相交（源级检查或白底 label）。

建议把检查写成脚本，失败即 exit 1；**lint 不通过不得输出 PNG**。

## 7. 验收清单

- [ ] 标题、公式、图形、图注四行清晰；
- [ ] 字体成对：英文数学 + 中文宋体；
- [ ] 主文字 ≥94% 黑，说明 ≥80%；
- [ ] 无文字越界；
- [ ] 无文字重叠；
- [ ] 集合标签不压边线；
- [ ] 箭头不穿字；
- [ ] 同一组图画布一致；
- [ ] 已跑 lint。

## 8. 最小 TikZ 骨架

```lean
\documentclass[border=14pt]{standalone}
\usepackage{lmodern}
\usepackage{amsmath,amssymb}
\usepackage{xeCJK}
\setCJKmainfont{Songti SC}
\usepackage{tikz}
\usetikzlibrary{arrows.meta,positioning,calc}
\begin{document}
\begin{tikzpicture}[
  font=\footnotesize,
  >={Latex[length=2.1mm]},
  every node/.style={text=black!96},
  set/.style={draw=blue!55!black, fill=blue!4, thick, rounded corners=12pt},
  subset/.style={draw=blue!70!black, fill=blue!13, thick, rounded corners=8pt},
  map/.style={->, draw=blue!75!black, line width=.75pt},
  caption/.style={font=\footnotesize, text=black!80},
  setlabel/.style={fill=white, inner sep=2pt, text=black!94},
  dot/.style={circle, fill=blue!80!black, inner sep=1.6pt},
]
\useasboundingbox (0,-1.30) rectangle (12.25,4.50);
\node[anchor=west, font=\large\bfseries] at (0,4.02) {定理名：一句中文标题};
\node[anchor=west, font=\normalsize] at (0,3.45) {$A\subseteq B\Longrightarrow f''A\subseteq f''B$};
% 图形 ...
\node[anchor=west, caption] at (0,-0.55) {一句图注，最多两行。};
\end{tikzpicture}
\end{document}
```
