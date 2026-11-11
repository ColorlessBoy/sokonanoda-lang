# ε–N 虚拟同伴演示（C8.3）

一个**单页**教学演示：两位 2D 虚拟同伴 + 一个 ε–N 动画 + 5 道游戏题，全部围绕
`aₙ = 1/n → 0` 这一个概念。没有课程单元、没有内核/前端/CI/docs 改动，只新增
`scripts/learn_demo/**`。

- **演示页**：`scripts/learn_demo/index.html`
- **评审副本**：`scripts/learn_demo/preview/index.html`（逐字节相同，供 raw.githack）
- **预览（commit 固定）**：<https://raw.githack.com/ColorlessBoy/sokonanoda-lang/36ca1406c6bc1e2126319213d352d6ef89585db7/scripts/learn_demo/preview/index.html>
（同一份 `preview/index.html` 也随本分支提交；页面自包含，无外部资源。）

```bash
open scripts/learn_demo/index.html          # 本地 file://，不需要服务器
node scripts/learn_demo/build.mjs           # 从源文件重新生成 index.html + preview
node scripts/learn_demo/build.mjs --check   # 只校验提交的产物是否漂移
node scripts/learn_demo/check.mjs           # 静态 + 真 Chrome，rc=0 才算通过
node scripts/learn_demo/check.mjs --json    # 机器可读摘要
```

## 为什么是这一页

owner 已批准虚拟同伴形态，并明确要**多角色 cast**、要**有游戏题**。这一页回答的是
产品问题：**当动画、同伴台词和答题都在同一页时，每条台词/每次交互能不能都服务于
同一个数学动作？**

- 一个概念：`aₙ = 1/n` 的 ε–N 定义（不扩成课件、不做单元、不接内核）。
- 两个角色：小析问学习者会问的问题；阿理每次只给一行可执行提示。
- 五道题：覆盖单选、判断、点选、排序、滑块 + 选择；全部在 JS 里按公式校验。
- 一条形式化桥：折叠卡把「带子 / 起点 / N 之后 / 严格小于」映射到
  `∀ε > 0, ∃N, ∀n ≥ N, |aₙ − L| < ε`，并说明本演示尚未接内核。

## Cast

| 角色 | 定位 | 说话方式 | 数学动作 |
|---|---|---|---|
| **小析** | peer-guide | 一次问一个问题 | 指认带子、追问「N 是什么」、在答对后确认边界 |
| **阿理** | mentor | 一行提示，不替学习者做题 | 提示先算 `1/ε`、比较 `a₄`、检查严格小于 |

`content.json` 的 `characters[]` 是唯一数据源。每个角色对每个上下文
（`intro` / `scene` / `q1`…`q5` / `complete`）都有台词；每道题还有
`successLines[xiaoxi|ali]`。台词约束由 `lib/contract.mjs` 检查：非空、单行、≤ 90 字符、
不含感叹号、上下文齐全。两套 SVG 共用同一张脸/同一组表情机制，但轮廓、配色、道具
不同：小析是圆短发 + 青绿围巾 + 笔记本；阿理是侧分短发 + 眼镜 + 琥珀开衫 + 教鞭。
点击角色卡切换当前发言；两位始终可见，非发言者缩小、降饱和。

## ε–N 场景（`aₙ = 1/n`）

- 数轴 0–1，前 20 项 `aₙ = 1/n`；误差带 `0 ≤ aₙ < ε`；ε 虚线；N 起点线；序号轴
  `n = 1…20` 标出从哪一项起在带内。
- ε 滑块范围 `0.001–0.250`，按千分位整数存储；预设 `0.25 / 0.22 / 0.10 / 0.05 / 0.01`。
- 播放从 `0.25` 收紧到 `0.05`；可暂停；步进按 `0.25 → 0.22 → 0.18 → … → 0.001`；
  重置回 `0.22`。
- 读数：`N`、画面内带内项数、`1/N`、严格检查
  `1/N < ε` 与 `1/(N−1) ≥ ε`（N > 1 时）。
- 可见字幕随 ε/N 更新；`aria-live` 只在 N/ε 变化时节流播报。
- 形式化桥：可折叠的「形式定义 / 为什么是这个 N」，含定义、直白解释、
  `N = ⌊1/ε⌋ + 1` 的推导、4 步证明草稿、直觉 ↔ 形式对照表，以及「未接内核」的
  明确说明。

### 严格边界（唯一公式）

```text
|aₙ| < ε  ⇔  1/n < ε  ⇔  n > 1/ε
N = ⌊1/ε⌋ + 1      （1/ε 为整数时也必须 +1）
```

| ε | 1/ε | N | 边界检查 |
|---|---|---|---|
| 0.25 | 4 | 5 | `a₄ = 0.25` 不小于 ε，所以不是 4 |
| 0.22 | 4.545… | 5 | `a₅ = 0.2 < 0.22`，`a₄ = 0.25 ≥ 0.22` |
| 0.10 | 10 | 11 | `a₁₀ = 0.10` 不算严格小于，所以 +1 |
| 0.052 | 19.23… | 20 | `a₂₀ = 0.05 < 0.052` |
| 0.001 | 1000 | 1001 | 超出画面 20 项；定义仍成立 |

页面里**没有硬编码的 N 显示**：初始占位符是 `—`，`app.js` 的 `computeN()` /
`isInside()` 算出全部读数、带内颜色、公式和字幕。`content.json` 只保存题目与预期
答案（例如 Q1 的 `5`），`check.mjs` 会从公式重算并拒绝不一致的答案键。

## 5 道游戏题

| # | 类型 | 题目 | 正确答案 | 数学 |
|---|---|---|---|---|
| Q1 | 单选 | ε = 0.22 → N？选项 4 / 5 / 10 | **5** | `⌊1/0.22⌋+1 = ⌊4.545…⌋+1 = 5` |
| Q2 | 判断 | 「N = 4 对 ε = 0.22 也成立」 | **错误** | `1/4 = 0.25 ≥ 0.22`，第 4 项在带外 |
| Q3 | 点选 | 点出第一个带内点 | **n = 5** | 前四项 `1, 0.5, 0.333…, 0.25` 都 ≥ 0.22；`a₅ = 0.2 < 0.22` |
| Q4 | 排序 | 量词块排序 | **∀ε > 0 → ∃N → ∀n ≥ N → \|aₙ − L\| < ε** | 先接受 ε 挑战，再回应 N，再限定所有 n，最后断言距离 |
| Q5 | 滑块 + 选择 | 把题内 ε 调到 0.10，选 N（10 / 11 / 21） | **11** | `1/0.10 = 10` 是整数；`a₁₀ = 0.10` 不满足严格小于，所以 `10 + 1` |

- 所有选择/点选/排序都是原生 `<button>`；Q4 是「点块放入下一格 / 点已放置块取回」，
  不依赖拖拽，键盘可用。
- Q3 的散点图横轴是项号 n、纵轴是 aₙ，带子是 `aₙ < ε` 的水平区域；点的位置和带内
  判定都由 JS 算，不写死颜色。
- Q5 先拖滑块再选 N；如果先选 N，只给一次「先把 ε 调到 0.10」的提示，不算错、不扣
  连对。

### 计分

- 每题基础 **20 XP**；每用一层提示 **−5 XP**；答错后重试 **−5 XP**；单题最低
  **5 XP**。答错不锁题、不清进度，只把连对归零。
- 顶栏显示 XP、连对、`已完成 x / 5` 和原生 `progressbar`；答对后「下一题」，第五题
  后显示完成卡。
- 完成徽章标题固定为 **「你理解了 ε–N」**，附 5 题回顾、XP、最长连对。

## 可访问性 / 移动端

- 390px 与 1440px 均无 document/body 横向溢出；`check.mjs` 还会逐元素检查没有元素
  越过视口。移动端保留 210px 高的场景图，并隐藏 SVG 内的小字，关键数字放在 HTML
  读数/字幕里。
- 键盘：所有交互是原生 button / range / summary；Q2 的检查里真的用 `Tab` 聚焦 +
  `Enter` 触发（CDP 键盘事件）。Q3 的 20 个点也是坐标命中的 24px 目标 button。
- `aria-live`：气泡、反馈、全局 announcer 都是 polite；进度条有
  `aria-valuemin/max/now`；滑块有 `aria-valuetext`（含当前 N）；错题反馈可聚焦。
- `prefers-reduced-motion`：CSS 关掉呼吸/过渡，JS 让播放直接跳到最终 ε（0.05），
  不做动画。
- 无 textarea、无 contenteditable、无文本输入、无代码编辑器、无外部字体/图片/CDN。

## 文件

```text
scripts/learn_demo/
  content.json              唯一内容源：角色台词、场景参数、5 题、计分、徽章
  template.html             页面骨架 + 占位符
  styles.css                全部样式（含两套 SVG 表情/配色）
  app.js                    场景、cast、答题状态机、公开的只读 state API
  index.html                生成物（内联 CSS/JS/JSON）
  preview/index.html        逐字节副本（raw.githack）
  build.mjs                 生成 / --check 漂移检查
  check.mjs                 静态 + 真 Chrome 验收
  lib/math.mjs              computeN / isInside / boundaryHolds / termsFor
  lib/contract.mjs          内容契约 + 负例（缺角色/缺题/错答案键必须被拒）
  lib/page.mjs              渲染与构建
  lib/verify-static.mjs     文件、自包含、250 个 ε 的严格性、负例
  lib/verify-browser.mjs    真 Chrome：cast/场景/5 题/键盘/溢出/网络/截图
  lib/cdp.mjs               零依赖 CDP 客户端
  screenshots/              提交的验收截图
  README.md                 本文件
```

## 验收

```bash
node scripts/learn_demo/build.mjs --check
node scripts/learn_demo/check.mjs
echo $?   # 0
```

`check.mjs` 的三层：

1. **静态**：提交的 `index.html` 必须等于从源重新渲染的字节；preview 必须逐字节相同；
   零绝对 URL、零外链、零 textarea/contenteditable；内容契约；250 个千分位 ε 的
   `N = ⌊1000/k⌋ + 1`、`1/N < ε`、`1/(N−1) ≥ ε`、成员关系；负例（缺角色、缺题、
   错 Q1/Q3/Q4/Q5 答案键、缺成功台词、缺角色上下文）必须被拒。
2. **浏览器**（真 headless Chrome，file://）：两个角色可见且可切换；三段提示逐层变化；
   预设 0.10 → 11、0.25 → 5（整数边界）、0.01 → 101、Home 键 → 1001；播放/暂停/步进；
   形式卡折叠；5 道题逐题用真实点击/键盘答对；错答不 award XP；Q4 错误顺序指出第
   二格；Q5 先选 N 的提示不计错；完成徽章标题精确；重启；390/1440 逐元素溢出；
   减少动态；零外部请求、零 console/page error。
3. **截图**：默认写到 `/tmp/learn-demo-*.png`；`SOKO_UPDATE_SHOTS=1 node scripts/learn_demo/check.mjs` 才刷新提交的 `scripts/learn_demo/screenshots/`（避免 Chrome 每次差几像素把仓库弄脏）。

本次验收结果（提交前）：`node scripts/learn_demo/check.mjs` → `rc=0`，静态 80 项 + 浏览器 187 项，共 **267 项**通过。截图：

- `screenshots/overview-1440.png` — 桌面首屏：cast、场景、公式、形式卡
- `screenshots/cast-scene-390.png` — 390px cast + 210px 场景
- `screenshots/game-q3-390.png` — Q3 点选（散点 + 带）
- `screenshots/game-q4-390.png` — Q4 排序（错误反馈态）
- `screenshots/complete-390.png` / `screenshots/complete-1440.png` — 完成徽章

## 已知限制

- **答案校验是静态 JS**：`content.json` 是答案键，`app.js` 用 `computeN()` 复核；
  还没有课程内核 / WASM 判分，不能在这里写 Lean 证明，也不产生 `decl.checked`。
- **Q4 是点选排序**，不是自由拖拽；触屏和键盘优先。
- **没有持久化**：刷新即重置；没有账号/后端。
- **只讲一个概念**：`aₙ = 1/n → 0`。换成一般序列/一般 L 需要新的 content schema 和
  新的数学检查，不在本演示范围。
- **raw.githack 有缓存**：评审用 commit 固定的 preview 链接，不要依赖 `master` 路径。
