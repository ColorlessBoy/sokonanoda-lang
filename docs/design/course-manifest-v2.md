# 设计：课程清单 v2（卷/章/先修/标签/练习配额）—— 台账 G-07

> 触发：缺口台账 **G-07**（`docs/gaps/ledger.jsonl`）——「课程清单格式扁平（无卷/章/
> 先修/标签/练习配额）」；上游计划 `docs/design/teaching-project.md` §6（台账协议）
> 与 §P6（度量与看板）、§P5（站点）。实现版本 **0.60.0**。
>
> 本文 = 现状 + 格式 + 兼容规则 + 消费者逐个的 as-built + 判据 + 验证。
> 证据分两类：**[实测]** = 本机跑过的命令与退出码；**[设计判断]** = 取舍理由。

---

## 0. TL;DR

1. 清单从**扁平 JSON 数组**（`[{file,title,title_en,unit}]`）升成 **v2 结构化对象**
   （`{schema, name, title, volumes[].chapters[].units[]}`），
   **旧数组继续被接受**——向后兼容是硬要求（[设计判断]：入门课 11 单元的
   `course/course.json` 与 `scripts/new-course-repo.sh` 生成的课程仓一个字节都不用改）。
2. 新增的元数据**只喂机器读的消费者**（CLI 事件、课程树、站点页、课程门禁），
   **不参与判红**：`quota.exercises` 与画布实际练习数的差额**只报告**。
   这是课程门禁一贯的设计原则——「只判形状、不锁计数」
   （`docs/design/course-gate-in-ci.md` §3）。
3. 五个消费者同一轮跟上：CLI `course`（additive 事件字段）、课程门禁
   `tools/check.py`（展平 + 新判据 **G6**）、VS Code 课程树（卷→章→单元分组）、
   站点生成器 + 卷 I 页面（按卷/章分组，计数仍**实测**自门禁）、文档。

## 1. 现状（改前，[实测]）

```jsonc
// courses/set-theory/course.json（v1：扁平数组，12 单元）
[ { "file": "units/unit01-sets-membership.sokonanoda",
    "title": "单元① 集合与隶属", "title_en": "Unit 1 — Sets & Membership", "unit": 1 }, … ]
```

机器读得到的只有 `file`/`title`/`unit`（`title_en` 是 `scripts/gen-site-data.py` 的
约定）。卷/章/先修/练习配额**只能写进文档**——`courses/set-theory/README.md` 的
「现状」表与 `docs/design/set-theory-syllabus.md` 的大纲表都是人手维护的，
于是「文档说有 4 章」与「机器只知道 12 个平铺单元」必然漂移（台账 G-07 的
`today` 字段原文）。

## 2. v2 格式（`schema: "soko.course/2"`）

```jsonc
{
  "schema": "soko.course/2",
  "name": "set-theory",
  "title": "卷 I《集合论》",
  "volumes": [
    { "id": "I", "title": "卷 I 集合论",
      "chapters": [
        { "id": "I.1", "title": "集合、子集与集合运算",
          "prereqs": [], "tags": ["membership", "subset", "powerset"],
          "quota": { "exercises": 27 },
          "units": [ { "file": "units/unit01-….sokonanoda",
                       "title": "单元① 集合与隶属",
                       "title_en": "Unit 1 — Sets & Membership",
                       "unit": 1 }, … ] }, … ] } ]
}
```

字段语义（**唯一真相**；README 与站点文案都从这里派生）：

| 字段 | 语义 | 判红？ |
|---|---|---|
| `schema` | 格式标签，`soko.course/2`。**没有 `schema` 的对象**按 v1 对象处理（见 §3） | 未知 `schema` ⇒ 清单读不了（exit 非 0） |
| `name` / `title` | 课程 id / 课程标题（人类可读） | 否（G6 只查 id 唯一性） |
| `volumes[].id` | **卷 id**，课程内唯一 | **G6 判红**（重复/空） |
| `volumes[].title` | 卷标题 | 否 |
| `chapters[].id` | **章 id**，课程内唯一 | **G6 判红**（重复/空） |
| `chapters[].prereqs` | 先修 **chapter id** 列表（可空） | **G6 判红**：指向不存在的 chapter id |
| `chapters[].tags` | 主题标签（自由字符串） | 否 |
| `chapters[].quota.exercises` | **计划**练习数（元数据） | **否，永不**——差额只进 G6 报告 |
| `chapters[].units[]` | 与 v1 条目**同形**（`file`/`title`/`title_en`/`unit`） | 是（沿用 G2：条目缺 `file`/`unit` 判负） |

三条格式纪律：

1. **只加不删**：v2 没有删掉 v1 的任何字段；`units[]` 的条目与 v1 逐字段同形，
   所以「展平后就是一个 v1 数组」——所有既有判据（G1–G5）语义**一字不改**。
2. **一个 unit 恰好属于一个 chapter**：展平按 `volumes → chapters → units` 的
   文档序进行，重复 `file` 由 **G6 判红**（防"同一单元挂两章"这种手滑）。
3. **未知字段忽略、缺失字段降级**：`tags`/`quota` 缺失只是没有该元数据，
   不影响判卷；`prereqs` 缺失 = 空列表。[设计判断] 课程仓是长期生长的产物，
   严格模式会让增量写入（先加章、后补先修）变成红。

## 3. 向后兼容（硬要求）

| 输入 | 行为 |
|---|---|
| JSON **数组**（v1，`course/course.json`） | 照旧：每项一个单元，无 `volume`/`chapter`/`tags` 字段 |
| JSON 对象 + `schema: "soko.course/2"` | v2：展平成单元列表，携带卷/章元数据 |
| JSON 对象 + **没有** `schema`，但有 `volumes` | 按 v2 处理（宽松：对象形态即 v2）[设计判断] |
| JSON 对象 + **其它** `schema` 值 | **读不了**（`error: … unknown schema …`，exit 1）——宁可报错也不猜 |
| 非 JSON / 既不是数组也不是对象 | 读不了（exit 1，既有行为） |

**CLI 事件只加不删**：v1 的 `course.unit` 字段（`file`/`title`/`unit`/`checked`/
`open`/`failed`/`reduced`/`error`）与 `course.summary` 的 `units`/`checked`/`open`/
`failed` 一个都不改名、不删除；v2 只是**多出**字段（§4.1）。

## 4. 消费者逐个的 as-built

### 4.1 CLI `sokonanoda course <manifest> --json`（`crates/cli/src/course.rs`）

新增 `crates/cli/src/course/manifest.rs`（纯解析 + 展平，可单测）：

```rust
pub(crate) struct UnitEntry { pub file: String, pub title: String, pub unit: u64,
                              pub volume: Option<VolumeRef>, pub chapter: Option<ChapterRef> }
pub(crate) struct ChapterRef { pub id: String, pub title: String, pub tags: Vec<String> }
pub(crate) struct VolumeRef  { pub id: String, pub title: String }
pub(crate) struct Manifest { pub units: Vec<UnitEntry>, pub volumes: usize, pub chapters: usize }
```

事件契约（`docs/protocol.md` 同步，additive）：

* `course.unit` 在 v2 时**新增**三个字段：
  * `volume`：`{"id": "I", "title": "卷 I 集合论"}`；
  * `chapter`：`{"id": "I.1", "title": "集合、子集与集合运算", "tags": ["membership", …]}`；
  * `tags`：章标签的**扁平副本**（`["membership", …]`）——树/看板按标签筛选时
    不必再下钻一层 [设计判断]：扁平副本比"记得去 `chapter.tags` 取"更不容易写错，
    代价是同一份数据在事件里出现两次（事件是一次性的，不是数据库）。
  * v1 输入**不出现**这三个字段（不是空数组、不是 `null`——是**没有这个键**），
    这样既有消费者（`editor/vscode/test-extension-host.js` 的 stub、
    任何按 `Object.keys` 遍历的脚本）行为逐字节不变。
* `course.summary` 新增 `volumes` / `chapters` 计数（v1 输入为 `0`/`0`，
  字段始终存在——计数是数出来的，不是"有没有"）。

### 4.2 课程门禁 `courses/set-theory/tools/check.py`

* `discover()` 先走 `flatten_manifest()`：v1 数组 / v2 对象都返回
  `(units, problems, chapters)`；**G1–G5 的判定代码一行不改**。
* 新增 **G6 = 清单自洽**（与 G1–G5 并列，同样与规模无关）：

  | G6 子判据 | 判红？ |
  |---|---|
  | volume id 唯一且非空 | 是 |
  | chapter id 唯一且非空 | 是 |
  | 每个 unit 恰好属于一个 chapter（同一 `file` 出现两次） | 是 |
  | `prereqs` 指向存在的 chapter id | 是 |
  | `quota.exercises` 与画布练习数的差额 | **否**（只报告） |
  | chapter 没有单元 / 卷没有章 | **否**（只报告：课程是长出来的） |

  G6 的判负走**同一条 G2 通道**（`problems` → `课程结构（G2/G6）` 行），
  这样 `--json` 的 `summary.rejected` 与退出码自动跟上，不需要第二套判负逻辑。
* `--selftest` 新增三组故意坏的清单（重复 chapter id / `prereqs` 指向不存在的章 /
  同一 unit 挂两章）必须被判负 + 一份**合法**的 v2 夹具必须判绿
  （正控制，防"G6 一律判红"这种假自检）。
* `--json` 报告新增 `chapters` 计数与每行的 `volume`/`chapter`（additive）。

### 4.3 VS Code 课程树（`editor/vscode/extension.js`）

* `courseUnitItem` 不变（v1 平铺路径**逐字保留**）；
* 新增 `courseVolumeItem` / `courseChapterItem` / `courseErrorItem`：
  **只要有任何一个 `course.unit` 带 `volume`，整棵树就按卷→章→单元分组**；
  没有任何一个带 ⇒ 与今天完全一样（平铺）。
  [设计判断] 判据取"数据里有没有卷"而不是"清单是 v1 还是 v2"：树只吃事件
  （`docs/design/course-status.md` §0：聚合在 CLI，客户端不重新推导），
  而"有没有卷字段"就是 CLI 已经算好的结论。
* 分组节点是**可折叠**的（`TreeItemCollapsibleState.Collapsed`），
  单元节点仍是 `None` + `vscode.open`（既有行为）。
* 无卷信息但有的单元报错（`error`）时，挂在一个 `无法分组` 的兜底节点下
  （v2 清单解析失败/单元读不了时不让单元从树上消失）。
* 契约测试 `crates/cli/tests/extension.rs`：脚本必须认 `volume`/`chapter`
  字段**且**保留 v1 平铺路径（`courseUnitItem` + `TreeItemCollapsibleState.None`）。
  stub 宿主 `test-extension-host.js` 加一条真跑分组渲染的用例。

### 4.4 站点（`scripts/gen-site-data.py` + `site/set-theory.html`）

* `gen-site-data.py` 的 `_parse_units` 换成 `parse_manifest()`：v1 数组与 v2 对象
  都返回 `(units, volumes)`；`volumes` 里每章带 `id`/`title`/`prereqs`/`tags`/
  `quota`/`units`（单元的计数仍是**门禁实测**填进去的，一个数字都不手写）。
* 卷 I 页面按 **卷 → 章 → 单元** 分组渲染：章标题行（带先修/标签/配额的小字），
  单元行照旧（练习数 / 已判通过 / 判卷状态）。
* `python3 scripts/check-site.py` 必须绿（链接 + 无写死版本号）。

## 5. 判据（怎么知道这件事做完了）

1. `DEVELOPER_DIR=… cargo test --workspace --locked`（至少
   `-p sokonanoda-cli --test course --test course_status --test extension --test protocol`）
   全绿，其中新增用例：
   * `course_status.rs`：v2 清单 ⇒ `course.unit` 带 `volume`/`chapter`/`tags`、
     `course.summary` 带 `volumes`/`chapters`；**v1 清单 ⇒ 三个新字段不出现**
     （兼容性回归的钉子）；
   * `course_manifest.rs`（新）：解析层单测（v1/v2/未知 schema/缺字段/重复 file）；
   * `extension.rs`：v2 分组 + v1 平铺两条路径都在脚本里；
2. `python3 courses/set-theory/tools/check.py` ⇒ **exit 0 · 0 判负**；
   `--selftest` ⇒ exit 0（含 G6 的三组坏清单 + 一份好清单）；
3. `--json` 的**判卷计数与改前一致**（36 目标 · 329 checked · 99 open · 0 判负；
   只该多出 G6 的自检，不该改变判卷计数）；
4. `node editor/vscode/test-extension-host.js` ⇒ 12/12；
5. `python3 scripts/gen-site-data.py && python3 scripts/check-site.py` ⇒ 绿；
6. `python3 scripts/gap.py check` ⇒ 全绿（G-07 的 repro 翻成 exit 1）。

## 6. 复现件（`.sh`，`docs/gaps/repro/G07-course-manifest-v2.sh`）

约定（`docs/gaps/README.md`）：**exit 0 = 缺口仍在 · exit 1 = 已修 · exit 2 = 环境不满足**。
修好后脚本翻成 exit 1，`gap.py check` 因此要求台账写 `status: fixed` + `fixed_in`。

脚本断言四段（每一段都对应本设计的一条验收）：

1. 卷 I 的 `course.json` 是 v2（`schema` + `volumes[].chapters[]`）；
2. `course --json` 的 `course.unit` 带 `volume`/`chapter`/`tags`，summary 带
   `volumes`/`chapters`；
3. **兼容**：一份 v1 扁平夹具仍能判卷，且事件里**没有** `volume` 字段；
4. 课程门禁 `--json` 的 `chapters` 计数与清单一致。

## 7. 不做的事（明确排除）

* **不动内核**（`crates/kernel/**` 一个字节不改——硬规则 1）；
* **不改入门课** `course/course.json`（v1 继续是合法格式，它就是兼容性的活体测试）；
* **不给 `quota` 判红**（设计原则：只判形状、不锁计数）；
* **不做多卷聚合**（`blocks` 里的「多课程/多卷聚合」属于 P6 看板；v2 只是让它
  成为可能：格式里已经有 `volumes[]`，但 CLI 仍只报**本清单**的卷数）；
* **不改版本号**（`Cargo.toml` / `package.json` 由主线统一 bump——本单只写
  `fixed_in: 0.60.0`）。

## 8. 验证（[实测]，2026-09-19，0.60.0）

命令与退出码（本机 `target/debug/sokonanoda` 0.60.0；课程门禁用 `--bin` 显式指它，
因为缓存里的 0.55.0 会被门禁的版本比对拒绝——这本身是 G-16 的守卫在正常工作）：

```bash
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --workspace --locked
#   全绿；新增套件 course_manifest 5/5（v1 兼容 / v2 增量 / 未知 schema / 卷 I 形状 / 入门课仍 v1）
python3 courses/set-theory/tools/check.py --bin "$PWD/target/debug/sokonanoda"
#   exit 0 · 36 个目标 · 329 checked · 99 open · 0 判负（**与 v2 之前逐个相同**）
python3 courses/set-theory/tools/check.py --bin "$PWD/target/debug/sokonanoda" --selftest
#   exit 0（含 G6：三类结构非法被判负 + 一份合法 v2 判绿）
python3 courses/set-theory/tools/test_manifest_v2.py
#   exit 0 · 11/11（v1/v2 展平同形 + G6 四类判红 + 配额只报告 + 未知 schema 走 Prerequisite）
python3 courses/set-theory/tools/check.py --bin "$PWD/target/debug/sokonanoda" --json
#   course = {volumes: 1, chapters: 4, units: 12}；quota_notes 4 条（差额全 0，只报告）
node editor/vscode/test-extension-host.js
#   exit 0 · 14/14（新增 3 条：v2 分组 / v1 平铺 / 兜底分组）
python3 scripts/gen-site-data.py && python3 scripts/check-site.py
#   门禁实测 36 目标 · 329 checked · 99 open · 0 判负；site hygiene: ok (10 pages)
bash docs/gaps/repro/G07-course-manifest-v2.sh
#   exit 1（四段断言全绿 ⇒ 缺口已修，台账写 fixed）
python3 scripts/gap.py check
#   exit 0 · 全部与台账一致
```

清单 v2 的**判卷计数不变**是本次验收的硬条件：`--json` 的 `summary` 与
`targets[]` 的 `(label, kind, checked, open, status)` 与 v1 时**逐个相同**
（比对脚本在改前/改后两份报告上跑过），只多出 `course.volumes`/`course.chapters`
与 `quota_notes` 两个**报告面**字段。
