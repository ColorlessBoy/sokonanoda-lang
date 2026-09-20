# D9 —— 页面施工标准（每个页面作者都先读这一份）

> 这一份是**通用**要求。你的任务提示里会给你这一页的**专属**内容与来源。
> 两者冲突时，以任务提示为准；都没说的，按 D1 的规则手册。

## 0. 先加载技能

调用 `skill` 工具，依次加载 **`frontend-ui-engineering`** 与 **`ai-smell-detector`**。
两个都已装好。

## 1. 先读（按顺序，别跳）

| 文件 | 为什么读 |
|---|---|
| `docs/design/site-rebuild/spec/page-template.html` | **从它开始**。head 的每一条元数据、导航标记、`data-page` 都在里面，`check-site.py` 会逐条断言 |
| `docs/design/site-rebuild/spec/D5-css-components.md` | 页面级组件的类名与用法 |
| `docs/design/site-rebuild/spec/D6-editor-components.md` | 编辑器面板（`.editor` / `.goal` / `.events` / `.terminal` / `.diff` / `.completion`）的可粘贴片段 |
| `docs/design/site-rebuild/spec/D1-design-rules.md` | 规则手册：硬禁令、令牌、排版、动效 |
| `site/assets/tokens.css` | 令牌唯一源。**读注释**——它解释了方向与两个语义色 |
| `site/_partials/header.html` + `footer.html` | 导航与页脚的唯一源。**不要改**；你只要放标记 |
| 你自己那一页的内容卷宗 | 任务提示会指名 |

## 2. 写

- 路径与文件名由任务提示给出，写在 `site/` 下。
- `<body data-page="X">` 的 X 必须是页脚里某个 `data-nav` 的值（任务提示会给）。
- 放 `<!--#nav-->` … `<!--/#nav-->` 与 `<!--#footer-->` … `<!--/#footer-->` 两组标记，
  内容留空——`scripts/gen-site-nav.py --write` 会填。
- **写文件要增量**：先写一个能跑通的骨架，再一节一节 `edit` 追加。
  一次性巨型写入已经害死过好几个做这件事的 agent。

## 3. 硬规则（违反任何一条即任务失败）

| 规则 | 说明 |
|---|---|
| 有且只有一个 `<h1>` | |
| head 元数据齐全 | title / description / canonical / theme-color×2 / og:title / og:description / og:type |
| **不写死版本号** | 字面 `0.x.y` 会被 `check-site.py` 拒绝；用 `data-site-version` 占位符，值由 `site.js` 从 `site/data/site.json` 填 |
| **不写裸 hex、不写 `style=`** | 颜色只走令牌 |
| **不用 `<img>`、不引位图** | 全站零位图；编辑器面板是真 HTML/CSS |
| 字号只用 `--fs-*`，圆角只用 `--r-0`/`--r-1`/`--r-full`，间距只用 `--sp-*` | |
| 区块级垂直间距是 **24 的整数倍** | 格线才对得上 |
| 至少一个 `.turnstile` | 全站唯一的记忆点 |
| 单页 HTML ≤ 60 KB | 断言会量 |
| 无 JS 也必须完整可读可导航 | 折叠用 checkbox+label，标签页用 radio |

**被点名的 AI 长相，一律禁止**：居中 hero、标题上方的 eyebrow 标签、链接尾巴 `→`、
emoji 当图标、所有容器同一个圆角、三张一样的卡片、紫蓝渐变、入场动画。

## 4. 内容纪律

### 4.0 ⚠️ 先钉住你量的是哪个版本（这一条最容易被忽略，也最致命）

**本工作区的 `crates/` 是脏的**：有 +862 行未提交 WIP（给 `by` 块加了
`constructor` / `cases` / `left` / `right` / `use` / `exfalso`）。而 `scripts/soko`
的解析顺序里「仓库构建」优先，`target/debug/sokonanoda` 正是用这棵脏树编的
（它的 marker 写着 `0.55.0`，`--version` 却报 `0.61.0`）。

**后果**：`scripts/soko` 量的是**未发布代码**。实证——同一份 `by cases c`：

| 用哪个二进制 | 结果 |
|---|---|
| `scripts/soko`（仓库构建） | `decl.checked` + `exercise.open`，**能用** |
| 已发布的 0.61.0（扩展自带） | `parse` 阶段的 `unexpected-token`，**不能用** |

**站点写的是已发布版本的事实。** 所以任何要引用内核行为的地方，先把二进制钉住：

```bash
export SOKONANODA_BIN=~/.vscode/extensions/sokonanoda-lang.sokonanoda-0.61.0-darwin-arm64/bin/darwin-arm64/sokonanoda
# 或者先确认这棵树是干净的：
git status --short crates/     # 必须无输出
```

**已经核对过、不受影响的**（不用重做）：诊断码集合 HEAD 与工作树完全相同
（99/99，增 0 删 0）；`site/data/*.json` 里 55 条复现没有一条用到新 tactic；
`events.json` 的样例源码不含 `by` 块。**但你自己新写的复现必须重新钉版本跑。**

### 4.1 其余内容纪律

- **每个主张都要能点开看到真实证据**。写"真内核判卷"就要有真事件流；写"零工具链"
  就要有可复制的命令。
- **零伪造**：数字来自 `site/data/*.json` 或内容卷宗；代码来自仓库真源码；
  内核输出必须真跑过（`scripts/soko …`）或取自 `site/data/events.json`。
  **拿不到真实例子就写进 `.callout--limit` 说明，不要编。**
- **诚实的限制是加分项**，不是减分项。每页至少考虑一次：这里有什么是做不到的？
- 颜色是语义的：**绿**只给「内核真的通过过」的东西，**朱**只给「诊断 / 更正 /
  诚实的限制」。看到强调色就问"这里背后有什么真实判定"。一个元素只带一个语义色。
- 已实测的仓库文档错误见 `docs/design/site-rebuild/STATE.md` §5——**按实测写，别抄文档**。

## 5. 验证（跑完把真实输出贴进报告）

```bash
python3 scripts/gen-site-nav.py --write
python3 scripts/check-site.py 2>&1 | grep -E "<你的页>|^site hygiene"
python3 scripts/site-audit.py site/<你的页>.html
python3 scripts/site-shot.py site/<你的页>.html --out .cache/shots
```

- `check-site.py` 里指向**尚未建好**的页面的 broken link 是预期的，可以接受；
  指向**不存在且不在 D2 §1 站点地图上**的路径就是真错误。
- `site-audit.py` 必须报 `no overflow` 且 `no blank colours`。
- 报告里给出：页面字节数、你用到的**每一个数字/事实及其出处**、你选择省略了什么以及为什么。

## 6. 报告

中文散文；代码与命令英文。**不要**说"基本完成"——要么跑通了，要么说清楚哪里没跑通。
