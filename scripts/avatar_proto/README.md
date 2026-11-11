# 小析（Xiao Xi）· 2D 虚拟数学同伴

> **C8.3 设计/视觉探针，不是课程单元。**
> 原创 2D 角色，不复制 Doubao；无真人出镜、无语音、无 CDN、无外部字体、无网络请求、无 npm 依赖。
> 只新增 `scripts/avatar_proto/**`：没有改 `courses/`、kernel、front、CI、docs。

这个探针回答一个产品问题：**当没有真人老师出镜时，一个 2D 虚拟同伴能不能提供社会存在感，同时让数学动画继续承担“讲清楚”的主要工作？**

小析的定位是 **peer-guide（同伴型引导者）**：问出学习者会问的问题、指认画面、给一行提示、在答对时庆祝。不是讲师，不是监考，也不是自动解题器。每一句台词只做一个数学动作。

## 打开与预览

```bash
open scripts/avatar_proto/index.html
# 或任意静态服务器；页面不加载任何外部资源
python3 -m http.server 8000
open http://127.0.0.1:8000/scripts/avatar_proto/index.html
```

`preview/index.html` 是 `index.html` 的逐字节副本，专门给 raw.githack 评审：

```bash
node scripts/avatar_proto/build.mjs          # 重新生成 preview/index.html
node scripts/avatar_proto/build.mjs --check  # 只校验副本是否逐字节一致
```

本 PR 的预览（commit 固定，`b39c8887`）：

- 场景样张：https://raw.githack.com/ColorlessBoy/sokonanoda-lang/b39c8887/scripts/avatar_proto/preview/index.html
- 角色表情表：https://raw.githack.com/ColorlessBoy/sokonanoda-lang/b39c8887/scripts/avatar_proto/preview/index.html?view=sheet

## 角色圣经 v0.1

### 1. 身份

| 项 | 内容 |
|---|---|
| 名字 | **小析**（Xiao Xi）。“析” = 把问题拆开看。 |
| 角色 | 同伴型引导者（peer-guide）。不是讲师、不是监考、不是真人。 |
| 一句话 | 站在学习者旁边，指出下一步该看哪里。 |
| 性格 | 好奇、准确、克制；站在学习者这一侧，但不会替学习者把题做完。 |
| 气质 | 成年 / 中性 / 友好；好奇但不过分幼态，不卖萌。 |
| 口癖 | 短句；一次问一个问题；先指画面，再说话。 |

### 2. 两种人格：共享形象，只换微文案

同一张脸、同一套动作、同一组道具。人格切换只改变措辞，方便 owner 之后再拍板默认声音。

| 时刻 | 同伴 peer（一起追问） | 导师 mentor（一行提示） |
|---|---|---|
| 开场 5–10s | “嗨，我是小析。这条带子是什么？” | “嗨，我是小析。先看带子，再找 N。” |
| 提示 | “为什么是 N，不是 N−1？” | “提示：先让 1/N < ε。” |
| 答对 | “对，就是这个 N。” | “对。N 取到了。” |
| 没接住 | “诶，等等——a₄ 在带里吗？” | “再看一眼：a_N 落在带里吗？” |
| 收尾 | “原来 ε 一收紧，N 就得往后跑。” | “小结：ε 定带宽，N 定起点。” |

- **peer**：把问题递回给学习者；用“为什么 / 那 N 是什么”保持好奇，而不是装傻。
- **mentor**：直接给一行可执行的提示；不说教、不复述定义。
- 两种人格共用**同一视觉**，所以以后换默认人格不需要重做角色资产。

### 3. 声音规则

**Do**

- 一句话一个数学动作：指认 / 追问 / 提示 / 确认。
- 先问，再给一行提示；让学习者先动手。
- 夸这一步：“这一步对了。”
- 指画面里的具体对象：“这条带子”“a₄”“N 的竖线”。
- 承认画面限制：“画面只画 a₁…a₁₀；序列本身没有最后一项。”

**Don’t**

- 不假装人类：不说“我刚看到你”“我一直在后台陪你”“我很担心你”。
- 不评价人：不说“你真聪明 / 你怎么又错了”。
- 不长篇讲定义、不把证明替学习者写完。
- 不刷感叹号、不卖萌、不堆“太棒了”。
- 不在数学动画上覆盖大段气泡。

### 4. 外观规则

- 圆角几何头身；简单的眼睛、眉、嘴；成年 / 中性比例。
- 两件身份道具始终可见：**笔记本**（封面有 ε）和**铅笔**（别在头侧）。
- 调色板：深蓝 `#17324d`（身体/墨）、青绿 `#1f8a7a`（带内/提示）、琥珀 `#e9a23b`（N/焦点）、纸面 `#f4e3cb`、玫瑰 `#c96f52`（嘴/橡皮）、表面 `#fffdf8`。
- 不写实：没有皮肤高光、鼻子、牙齿、真人 AI 脸；没有 3D 渲染。
- 不幼态：不做大头萌宠、闪粉、换装、夸张腮红。轮廓在 96px 仍要读得出。
- 表情只改眼、眉、嘴与头部角度；不换服装、不换道具，保证跨镜头一致。

### 5. 状态与用途

| 状态 | 英文 | 教学时刻 | 例句 |
|---|---|---|---|
| 平静 | neutral | 出现、等待读题 | “先看这条带子。” |
| 好奇 | curious | 追问、把问题递回 | “那 N 是什么？” |
| 思考 | thinking | 提示前的停顿、等学习者算 | “先别急，找 N。” |
| 开心 | happy | 答对一步、完成小目标 | “对，就是这个 N。” |
| 哎呀 | oops | 没接住、给方向，不评价人 | “再看一眼：a₄ 在带里吗？” |

- 静息 4.2–7.2 秒随机眨眼；开场 / 答对可挥手一次。
- `prefers-reduced-motion: reduce` 或页面开关打开时：不眨眼、不挥手、不做补间。
- 关闭动态时仍保留静态表情与全部信息，不牺牲可用性。

### 6. 出场、位置、比例

| 时刻 | 行为 |
|---|---|
| 开场 5–10s | 角落一句问候 + 一次挥手；可跳过；不是视频，不是真人。 |
| 提示 | 按需或停顿后出现；只有一行。 |
| 反馈 | 对 / 没接住各一句；说完回到平静。 |
| 收尾 | 一行小结；不复述整课。 |

- 常驻右下角，可隐藏；隐藏后不留空洞，数学画面不移动。
- 桌面约 158px 宽；移动 104–126px。气泡最多 2–3 行。
- **不遮数学**：气泡在数轴下方；`check.mjs` 断言 companion 与 plot 的矩形没有交集。
- 页面里的「角色表」是 `?view=sheet` 静态视图，方便一次看全五个状态。

## ε–N 场景样张

`aₙ = 1/n`，极限 0。用一条 0–1 数轴、一条允许误差带、10 个序列点和一条 N 竖线，展示“ε 定带宽，N 定起点”。

- **ε = 0.22 ⇒ N = 5**：因为定义是严格小于，`1/n < ε ⇔ n > 1/ε`。`a₄ = 0.25` 还在带外，`a₅ = 0.2` 起都在带内。
- **ε 减半到 0.11 ⇒ N = 10**：`a₉ = 1/9 ≈ 0.1111` 还在带外，`a₁₀ = 0.1` 进带。ε 收紧，N 只会往后走。
- 数轴只画前 10 项；画面明确写“序列本身没有最后一项”，不把样本冒充整条序列。
- N 竖线画在值轴的 `1/N = 0.2` 处，标出 `a₅` 的位置；N 本身是**序号**，文案里说清“从第 5 项起”。
- 三句台词对应三个高亮，各自只做一个数学动作：

| # | 台词（peer） | 画面动作 |
|---|---|---|
| 01 带子 | “这条带子就是允许误差 ε。” | 高亮 ε 带；说明 0 到 0.22 都算“够近”。 |
| 02 起点 | “从第 N 项开始，后面的点都进来了吗？” | 高亮 N 竖线与带内点；a₄ 在带外，a₅ 在带内。 |
| 03 收紧 | “ε 再小一点，N 会怎么变？” | 出现 ε/2 虚线带与 N′ 竖线；N 从 5 挪到 10。 |

## 控件与可访问性

- **人格**：同伴 / 导师，`aria-pressed` 分段按钮；切换只换微文案与声音表高亮。
- **表情**：5 个原生按钮，各自对应一个教学时刻；下方实时显示“用于……”说明。
- **台词**：3 个按钮切句；每句同步气泡、画面注与高亮。
- **画面**：隐藏 / 显示小析（按钮文字与 `aria-pressed` 同步）、挥手、减少动态（`checkbox` + 可见 44px 标签）。
- 气泡是 `role="status" aria-live="polite" aria-atomic="true"`；切句会读给屏幕阅读器。
- 角色 SVG 是 `role="img"`，`aria-label` 随表情更新；纯装饰元素不进入可读树。
- 原生 `<button>` / `<fieldset>` / `<legend>`；跳过链接；`:focus-visible` 焦点环。
- 所有可见按钮 ≥36px 高（移动 40px+）；开关标签 ≥40px。
- 390 / 1440 两档都零横向溢出；数字标签不重叠；移动端隐藏次要点标签。
- 无文本输入、无表单提交、无自由 prompt。

## 设计决定（为什么这样做）

1. **同伴先于讲师。** 真人老师缺席时，学习者最缺的不是又一段讲解，而是一个“站在我这边、知道下一步看哪里”的存在。所以小析问问题、指画面，不讲课。
2. **一个数学动作一句话。** 气泡最短，画面最大；每句台词绑定一个 `focus`（band / tail / whatif），让角色的“在场”直接服务数学。
3. **一套形象，两种声音。** owner 还没拍板默认人格。共享视觉、只换微文案，能让这个决定晚一点、便宜一点。
4. **HTML/CSS 数轴，而不是一张缩放 SVG。** 390px 上标签要可读、点标签不能相撞；用百分比定位 + 媒体查询比整体缩放更稳。数字与 ε/N 的对应仍由 `check.mjs` 断言。
5. **纸面 + 墨的视觉语言。** 与课程站点的方格纸气质兼容；深蓝/青绿/琥珀只做语义色。头像圆润，数轴和卡片保持克制，避免角色抢走数学的注意力。
6. **零外部资源。** 单文件、内联 SVG、系统字体栈；不给评审加网络依赖，也让检查可以断言“绝对 URL = 0”。
7. **减少动态是产品功能。** 不是附属开关：关闭后静态版本仍然完整可读；系统偏好打开时页面在初始化就进入该状态。
8. **角色表单独可看。** `?view=sheet` 把五个状态放在一张图里，方便一次评审“表情是否一致、是否都可用”。

## 验证

```bash
node scripts/avatar_proto/build.mjs    # rc=0 · preview/index.html 逐字节一致
node scripts/avatar_proto/check.mjs    # rc=0 · 文件层 + 浏览器层
node scripts/avatar_proto/shots.mjs    # rc=0 · 四张证据图
```

本次实测（`check.mjs`）：

```text
static: ok · 74,574 bytes · sha b880f464dc414b84
browser: ok · 152 checks
sheet:   ok · 9 checks
external_requests: 0
page_errors: 0
console_errors: 0
1440px scrollWidth: 1440
390px scrollWidth:  390
```

浏览器层断言覆盖：

- 5 个表情按钮逐个点击 ⇒ 对应嘴/眼组可见、其余隐藏、`aria-pressed` 只有一个、状态说明与 `aria-label` 更新。
- 人格切换 ⇒ 气泡台词 / speaker / chip / 声音表高亮变化；**共享视觉不被换掉**。
- 3 句台词 ⇒ 气泡、画面注、`data-focus`、ε/2 虚线层；N=5 与 N=10 的对应。
- 隐藏 / 显示 ⇒ companion `hidden`、按钮文字与 `aria-pressed`、挥手禁用、**数轴矩形不动**。
- 挥手 ⇒ 换臂、动画结束后换回；眨眼 API 与减少动态的联动。
- 减少动态 ⇒ 开关、`data-motion=off`、零运行动画、无眨眼/挥手；系统 `prefers-reduced-motion` 在 reload 后自动生效。
- 390 / 1440：零横向溢出、companion 与 plot 不相交、气泡不裁切、点标签不重叠、按钮高度。
- 角色表：5 张卡、5 个不同表情、每卡 1 个 SVG、静态、零溢出。
- 零外部请求、零页面错误、零 console error。

### 守卫真的会咬

两个故意破坏的负例（跑完恢复原文件，复跑 rc=0）：

| 变异 | 结果 |
|---|---|
| 注入 `<img src="https://example.com/tracker.png">` 并删掉「哎呀」按钮 | `rc=1`，static 报 5 条：绝对 URL、`<img>`、缺表情、按钮数、preview 不一致 |
| 把 `setPersona` 里的 `state.persona = name` 删掉 | `rc=1`，browser 报 mentor 人格未切换、台词未变、chip 未变等 6 条 |
| 恢复 | `rc=0`，152 browser checks + 9 sheet checks 全绿 |

## 截图

| 文件 | 内容 |
|---|---|
| `/tmp/avatar-sheet.png` → `screenshots/character-sheet.png` | 五表情角色表（1440，静态） |
| `/tmp/avatar-peer.png` → `screenshots/stage-peer-1440.png` | 1440，同伴 + 第 2 句 + 好奇 |
| `/tmp/avatar-mentor.png` | 1440，导师 + 第 2 句 + 思考（本地证据，不提交） |
| `/tmp/avatar-mobile.png` → `screenshots/stage-mobile-390.png` | 390，同伴 + 第 2 句 + 好奇，顶部 1500px |

raw.githack：

- https://raw.githack.com/ColorlessBoy/sokonanoda-lang/b39c8887/scripts/avatar_proto/screenshots/character-sheet.png
- https://raw.githack.com/ColorlessBoy/sokonanoda-lang/b39c8887/scripts/avatar_proto/screenshots/stage-peer-1440.png
- https://raw.githack.com/ColorlessBoy/sokonanoda-lang/b39c8887/scripts/avatar_proto/screenshots/stage-mobile-390.png

## 局限（诚实清单）

- **是探针，不是课程单元**：不接 `courses/`、kernel、front、CI、docs；没有账号、持久化、判分、DSH 插件形态。
- **没有语音**：目前只有文字。TTS 以后再加，必须默认静音、必须带字幕、不能自动播放。
- **不是骨骼动画**：眨眼、呼吸、挥手都是 CSS/JS；没有 Rive/Lottie/Remotion 的骨骼、口型、视线。角色只有正面半身，没有转身 / 多角度。
- **一种 ε–N 样张**：单条数轴 + 固定 ε；没有滑杆、没有连续动画、没有序号轴。连续动画的探针是另一条分支（`scripts/anim_proto/`），这里故意只验证“在场比例”。
- **N 竖线在值轴上**：竖线画在 `1/N = 0.2`，标的是 `a₅` 的位置；N 本身是序号。文案与 aria-label 都写了“从第 5 项起”，但单轴表达不如“值轴 + 序号轴”双轴直白。
- **人格未拍板**：两种声音都是样张；默认用 peer 还是 mentor、是否按单元切换，属于 owner 决策。
- **无暗色模式**：这一版固定纸面浅色；课程站点有暗色，接入时需要补 token。
- **移动页面很长**：角色圣经完整展示是为了评审；产品里不会把整份 bible 发给学习者。
- **字体差异**：只用系统字体栈，macOS/Windows/Linux 的衬线与中文字形会有差异；布局和数学不受影响。
- **截图是证据，不是交互**：真正的状态切换以 `index.html` / raw.githack 预览为准。

## 之后怎么接

1. **先抽 Web Component**：把 SVG + 状态机封成 `<soko-companion persona="peer" expression="curious">`，页面只传台词与 focus。
2. **数据契约**（当前 `window.SokoAvatar` 已经是这个形状的雏形）：

```js
const x = SokoCompanion.mount(el, { persona: "peer", motion: "auto" });
x.say({ line: 2, expression: "curious", focus: "tail", text: "从第 N 项开始，后面的点都进来了吗？" });
x.setPersona("mentor");
x.hide();
x.setMotion("off");
```

3. **动画升级选项**：
   - **Rive / Lottie**：更细的 idle、眨眼、挥手、口型；保持同一组状态名与 `aria-label`。
   - **Remotion**：如果要录成课程视频，复用同一套 SVG 与文案，逐帧导出。
   - **CSS/JS 现状**：零依赖、可访问性最好，适合先验证产品位置与比例。
4. **语音**：TTS 只在有字幕、可静音、用户主动触发时启用；不要把语音当成默认。
5. **接入位置**：开场 5–10s、按需提示、答对/没接住反馈、收尾；始终角落、可隐藏、不覆盖数学。

## 文件

- `scripts/avatar_proto/index.html` — 自包含页面：SVG 角色 + ε–N 场景 + 控件 + 角色圣经 + `?view=sheet`
- `scripts/avatar_proto/preview/index.html` — 逐字节 preview 副本（raw.githack）
- `scripts/avatar_proto/build.mjs` — 生成/校验 preview
- `scripts/avatar_proto/check.mjs` — 文件层 + 浏览器层验收入口
- `scripts/avatar_proto/lib/cdp.mjs` — 零依赖 Chrome DevTools Protocol 客户端
- `scripts/avatar_proto/lib/verify.mjs` — 静态断言 + 真实点击/计算样式/布局断言
- `scripts/avatar_proto/shots.mjs` — headless Chrome 截图
- `scripts/avatar_proto/screenshots/` — 3 张提交证据图
- `scripts/avatar_proto/README.md` — 本文件

## Owner 待拍板

1. 默认人格：peer、mentor，还是按单元/难度切换？
2. 语音：要不要 TTS？谁的声音？是否默认静音？
3. 出现范围：只出现在难点单元，还是所有单元常驻？
4. 动画投入：CSS/JS 够用，还是上 Rive/Lottie/Remotion？
5. 位置与尺寸：角落 158px 是否合适，还是更小/更靠边？
