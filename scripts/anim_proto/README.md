# ε–N 交互视觉探针 · `aₙ = 1/n`

> **这是 issue #34 的产品/视觉探针，不是课程单元。**
> 单文件 HTML，主动画 + 形式定义卡 + 可复跑的截图/GIF 导出。
> 没有 CDN、没有外部字体、没有请求、没有 Manim、没有 kernel/Lean 依赖。

## 打开

```bash
open scripts/anim_proto/index.html
# 或任意静态服务器；页面不加载任何外部资源
python3 -m http.server 8000
open http://127.0.0.1:8000/scripts/anim_proto/index.html
```

`preview/index.html` 是 `index.html` 的逐字节副本，专门给 raw.githack 评审用：

```bash
node scripts/anim_proto/build.mjs   # 重新生成 preview/index.html
```

raw.githack 地址（把 `<sha>` 换成 PR 分支上的 commit）：

```text
https://raw.githack.com/ColorlessBoy/sokonanoda-lang/<sha>/scripts/anim_proto/preview/index.html
```

## 页面里有什么

- **全景数轴 0–1**：20 个点 `aₙ = 1/n`，误差带、`1/N` 竖线、20 个 n 标签。
- **近 0 放大 0–0.26**：把挤在 0 附近的 17 个点（n=4…20）展开；`n=1…3` 见上方全景。
- **序号轴 n=1…20**：N 的起点在序号轴上向右移动；“N 之后整段在带内”一眼可见。
- **ε 滑杆**：0.001–0.25，对数刻度；四个预设 `0.25 / 0.10 / 0.052 / 0.01`。
- **播放 / 单步 / 重播 / 重置**；键盘 `←/→`（滑杆聚焦或全局）、`P`、`S`、`R`、`0`。
- **形式定义卡**：`∀ ε > 0, ∃ N, ∀ n ≥ N, |1/n − 0| < ε`，plain-language 解释，以及实时
  `N = ⌊1/ε⌋ + 1`、`1/N < ε`、`1/(N−1) ≥ ε` 的最小性证据。
- **读法小卡**：三句话把“带 → 1/N → 序号轴”串起来，并提醒整数倒数处的 +1。
- **减少动态**：`prefers-reduced-motion: reduce` 时点击播放直接跳到最终状态，不做 tween。

## 数学：为什么不是简单 `⌈1/ε⌉`

定义用**严格小于**：

```text
1/n < ε  ⇔  n > 1/ε
```

所以 N 必须是**第一个严格大于 `1/ε` 的整数**：

```text
N = ⌊1/ε⌋ + 1
```

在 `1/ε` 不是整数时，它等于课本速记 `⌈1/ε⌉`；但在 `ε = 0.25, 0.1, 0.05, 0.01`
这类**整除点**，`⌈1/ε⌉` 会少 1：

| ε | `⌈1/ε⌉` | 正确 N | 边界项 |
|---|---:|---:|---|
| 0.25 | 4 | **5** | `a₄ = 0.25 = ε`，不在带内 |
| 0.10 | 10 | **11** | `a₁₀ = 0.10 = ε`，不在带内 |
| 0.05 | 20 | **21** | `a₂₀ = 0.05 = ε`，不在带内 |
| 0.01 | 100 | **101** | `a₁₀₀ = 0.01 = ε`，不在带内 |

页面在整数倒数时会把这条“边界项不算带内”的说明点亮；`check.mjs` 对每个
采样 ε 断言 `1/N < ε` 与 `1/(N−1) ≥ ε`，因此不是只对“好看的数字”成立。

### 20 个点只是样本

`aₙ` 是无穷序列，画面只画 20 项。当 ε 很小（例如 0.01）时 `N=101`，画面前
20 项**全部在带外**；这是诚实的，也正是定义的重点：`n ≥ 101` 之后还有无穷多项
都在带内。页面会显示“画面外还有无穷多项”，不会把 20 项冒充整条序列。

### 动画方向

- **值轴**：ε 变小，带变窄，`1/N` 向左靠近 0；带内的点逐项**退出**（不是进入）。
- **序号轴**：N 变大，起点向右移动；N 之后的那一段仍然是“之后所有项”。

这是同一个事实的两种视角。任务描述里的 “dots enter band one by one” 与
`aₙ=1/n`、`ε↓` 的几何方向相反；这里以数学正确为准，并在 caption 里写清
“退出”，形式定义卡保证“之后所有项都在带内”。

## 设计笔记

- **为“挤在 0 附近”做一个聚焦 + 上下文**：20 个 `1/n` 点中有 17 个落在
  `[0.05, 0.25]`（含端点）；单纯 0–1 线性轴在 390px 上无法标清全部 n。于是保留
  0–1 全景，再用 0–0.26 放大区承担可读性，用序号轴承担“N 之后”的顺序感。
- **标签车道**：放大区的 17 个 n 标签按 x 排序后贪心放进 5 条车道，避免横向
  相撞；窄屏隐藏奇数 minor 标签，保留 10 个关键标签，序号轴仍然给出 1–20。
- **SVG + DPR**：所有坐标用 viewBox 单位，页面在 1440/390/320 都不横向溢出；
  浏览器以 deviceScaleFactor=2 截帧/GIF，没有位图放大。
- **颜色语义**：朱砂 = 误差带/ε；青绿 = 带内/起点 N；墨 = 带外。定义卡是深色
  反白，和纸面拉开层级。
- **确定性**：视觉状态是 `epsilon` 的纯函数；`window.SokoAnim.seek(t)` 不含
  `Date.now()` 分支，导出帧可以逐帧复跑。播放由 rAF 驱动，但导出只用 seek。
- **无外部依赖**：唯一形如 URL 的字符串是 SVG 命名空间
  `http://www.w3.org/2000/svg`（元素命名空间，不是网络请求）；Chrome 验收断言
  `Network.requestWillBeSent` 里零 `http(s)`。没有 `fetch`/XHR/WebSocket/import。

## 验收 / 导出

```bash
node scripts/anim_proto/build.mjs         # preview 副本
node scripts/anim_proto/check.mjs         # 文件层 + 浏览器层；rc=0
node scripts/anim_proto/export-gif.mjs    # 验收 + 截图 + GIF；rc=0

# 只要验收，不截图/GIF
node scripts/anim_proto/export-gif.mjs --check

# 只截三张证据图
node scripts/anim_proto/export-gif.mjs --stills-only
```

`check.mjs` / `export-gif.mjs` 的判据：

- 文件层：无外部引用、无自由输入（唯一 input 是 range）、正式定义/plain 文案/
  `1/n < ε ⇔ n > 1/ε`/`⌊1/ε⌋+1`/`⌈1/ε⌉` 都在、preview 逐字节一致。
- 浏览器层：19 组 ε 的 N 公式与最小性；6 个滑杆状态下的 N、带内计数、颜色、
  `1/N` 与 ε 的相对位置、why 卡数字、预设、`aria-valuetext`；Step 循环、
  Play/Pause、全局快捷键、滑杆 Home/End/方向键；减少动态；`seek()` 单调性；
  390/1440/320 布局、标签不重叠、零外部请求/零页面错误；
  390px + ε=0.001（N=1001）的极值状态也断言无横向溢出、无文字裁切。

本次实际结果：

```text
static: ok · 68,259 bytes
browser: ok · 417 checks
external_requests: 0
page_errors: 0
console_errors: 0
390px overflow: 0 · detail labels 10
1440px overflow: 0 · detail labels 17
320px overflow: 0 · detail labels 10
390px / ε=0.001 (N=1001): overflow 0 · clipped text 0
```

> 数字以当前命令输出为准；`check.mjs` 会打印最新值，不在 README 里硬编码
> 检查条数之外的结论。

## 产物

- `scripts/anim_proto/index.html` — 单文件源
- `scripts/anim_proto/preview/index.html` — 逐字节预览副本
- `scripts/anim_proto/epsilon-n.gif` — 880×962，20 fps，120 帧，`window.SokoAnim.seek(t)`
  逐帧真实截图后由 ffmpeg 编码；约 0.57 MiB（最终体积以 `export-gif.mjs` 输出为准）
- `/tmp/anim-first.png` — t=0，ε=0.25，N=5
- `/tmp/anim-band.png` — t=3.0s，ε=0.10，N=11
- `/tmp/anim-converged.png` — t=6.0s，ε=0.052，N=20
- `/tmp/anim-mobile.png` — 390px 全页，t=3.0s

`export-gif.mjs` 会优先用 880px 宽；若超过 2 MB 自动降到 800/720/640/580/520/460，
并保持 20 fps。若所有宽度都超预算，脚本以非零退出，不会生成一个“假装成功”的
GIF。

## 局限

- **是探针，不是课程单元**：没有接入 `courses/`、kernel、front、CI、docs；没有
  持久化、账号、判分、DSH 插件形态。它只验证“这个概念动画是否讲得清、看得下去”。
- **不是 Manim**：用 SVG/CSS/JS 手绘；这换来零重依赖和可访问性，但没有 Manim
  的 3D/摄像机/复杂缓动。若之后要做更电影化的版本，Manim 仍是一个选项。
- **20 项样本**：`N > 20` 时画面内没有带内点；页面明确写出“画面外还有无穷多项”，
  而不是把 20 项当成整个序列。
- **动画只走到 ε=0.052**：再小会让 390px 上的带接近不可见；滑块仍可到 0.001
  （N=1001），用于展示“定义不封顶”。GIF 不覆盖极值。
- **单页演示**：没有老师侧/学生侧、没有题库、没有“下一步该教什么”的 agent 决策；
  这些属于 #34 的 lesson player，不属于这条动画探针。
- **字体**：只用系统字体栈；不同 macOS/Windows/Linux 的衬线字形会有差异，但
  布局和数学不受影响。
- **GIF 是证据，不是交互**：真正的交互以 `index.html` / raw.githack 预览为准。
