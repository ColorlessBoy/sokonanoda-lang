# D3 —— 站点「实验室」数据：模式、来源、命令与缺失行为

本文件是 `scripts/gen-site-lab.py` 的**规格与取证记录**：它生成哪六个文件、
每个文件的模式长什么样、数据从哪个源来、用哪条命令产生、**源缺失时怎么办**。

信息架构与内容模型见 `docs/design/site-rebuild/spec/D2-information-architecture.md` §2
（本文是它的落地细节卷）；本文件只描述**已经实现并在本机跑通**的东西。

---

## 0. 一条命令与六个文件

```bash
python3 scripts/gen-site-lab.py                  # 生成全部六个文件
python3 scripts/gen-site-lab.py --only gaps      # 只生成某一个（调试用；不点名的保持原样）
python3 scripts/gen-site-lab.py --only evidence --with-tests   # 额外真跑 cargo 数测试
```

| 文件 | 一句话 | 主源 |
|---|---|---|
| `site/data/walkthrough.json` | 一道题的**逐行**真实目标状态（`query state` 的原始输出） | `courses/set-theory/units/unit01-sets-membership.sokonanoda` |
| `site/data/events.json` | 一次真判卷的**完整 `--json` 事件流**（含一条内核阶段真拒绝） | 脚本自建的 `events.sokonanoda` + `scripts/soko grade` |
| `site/data/diagnostics.json` | 诊断码字典（64 条）+ **真跑出来的**复现 | `docs/design/site-rebuild/content/C1-language.md` §4.4 |
| `site/data/timeline.json` | 真实时间线（tag / CHANGELOG / 轮次 / §9） | `git tag`、`editor/vscode/CHANGELOG.md`、`STATUS.md`、`REQUIREMENTS.md` §9 |
| `site/data/gaps.json` | 缺口台账（24 条 + 汇总计数） | `docs/gaps/ledger.jsonl` |
| `site/data/evidence.json` | 质量证据数字（每条带产生它的命令） | 课程门禁、四本台账、git、`gh release view` |

**`--only` 只影响写盘**：没被点名的文件保持原样。这让调试某一节不需要重跑全部。

---

## 1. 共同约定

### 1.1 信封（每份文件都有）

| 字段 | 来源 | 说明 |
|---|---|---|
| `version` | `Cargo.toml` 的 `[workspace.package].version` | 数据**所描述的那棵树**的版本。与 `site.json` 的 `version` **不是同一个事实**，别混：那个是**已发布 tag**（见下） |
| `source_commit` | `git rev-parse --short HEAD` | 数据描述的是哪个仓库状态 |
| `generated_at` | `git log -1 --format=%cI HEAD` | **不是墙上时间**：用墙上时间会让每次重跑都产生无意义的 diff。数据是否过期由 `source_commit` 判断 |
| `generated_at_from` | 常量 `"git-head-commit-time"` | 把上面这条约定写进数据本身 |

> ### ⚠️ `version` 的两个含义（2026-09-21）
>
> `gen-site-data.py` 的 `version` 现在是**最新发布 tag**（`release_version()`），
> 因为站点的每个页脚、每条下载指令都从它拼出来，而 `compare.html` 明说这是
> 「已发布的版本，不是工作树」。上面这个信封里的 `version` 则是**数据所描述的
> 那棵树**的版本——正常情况下就是最近一次发布快照。
>
> 两者今天都是 `0.61.0`，但那是**巧合**：信封的 `version` 直接读 `Cargo.toml`，
> 而 `Cargo.toml` 已经是 `0.62.0` 了。也就是说，在一次版本 bump 之后、
> 下一次发布之前重跑 `gen-site-lab.py`，会把信封写成 `0.62.0` + 未发布的
> `source_commit`，同时把用工作树量出来的数（playground 事件等）混进站点——
> K10（`kernel.html` 引用了 `source_commit`/`generated_at`）与 K16 会因此判红。
> **发布之前不要重跑 `gen-site-lab.py`**；发布后从 tag 重跑（这是
> `STATE.md`「0.62.0 发布后的动作」里的那一项，尚未做）。

### 1.2 确定性（重跑逐字节一致）

- `json.dump(..., sort_keys=True, indent=2, ensure_ascii=False)` + 结尾一个换行；
- 所有列表都显式排序（见各节的「稳定顺序」），不依赖 `os.listdir` / `dict` 的插入序；
- `generated_at` 取自 HEAD 提交时间（§1.1），不是时钟；
- 唯一的例外源是外部命令的输出（`git` / `gh` / 内核），它们在同一仓库状态下是确定的。

### 1.3 零伪造

- 源缺失、解析失败、命令跑不起来 ⇒ **该字段/该条留空**，stderr 打一句人话原因
  （`  ! …`），脚本本身**只有自己坏了才非零退出**；
- 诊断复现由脚本**自己核对**：只有在真输出里出现了那个码才记为复现（§4.3）；
- 绝不估数：拿不到的数字不进 JSON（§7）。

### 1.4 绝不触发工具链下载

所有 CLI 调用带 `SOKONANODA_OFFLINE=1`，并且在跑判卷前先做
`scripts/soko doctor --json` 预检（`cli_ready()`）。判卷环境没就绪时，
依赖内核的那几节**不生成**，而不是生成一份编造的数据。

### 1.5 允许写的路径

脚本只写 `site/data/*.json` 与 `.cache/site-lab/`（`.cache` 是 `.gitignore` 的
暂存区：判卷样例画布、诊断复现件、cargo 测试计数缓存都在那里）。

---

## 2. `walkthrough.json`（B2 走查页）

### 2.1 模式

| 字段 | 类型 | 来源 |
|---|---|---|
| `file` | string | 选中的画布（仓库相对路径） |
| `source` | string | 画布**全文**（页面直接渲染，不需要第二次读文件） |
| `line_count` | int | 行数 |
| `sampling` | `"every-line"` | 采样口径：**不采样**，每一行都真的问一次内核 |
| `decls[]` | array | `query goals`：`name` / `kind` / `status` / `start_line` / `end_line` / `ty`（+ `ty_runs` 高亮片段） |
| `holes[]` | array | `query holes`：`id` / `decl` / `redundant` / `ty` / `start_line` / `start_col` |
| `hints` | object | 每个声明的 `-- soko:hint` 梯子（`query hints`，游标在声明首行第 1 列）；没有梯子的声明**不进这个对象** |
| `lines[]` | array | 每一行一条：`line` / `decl` / `ok`，`ok:true` 时带内核原样的 `goal` / `goal_runs` / `goals` / `step` / `total` / `decl_status`，`ok:false` 时带 `error` |

### 2.2 命令

```bash
scripts/soko query goals  --file <abs>/unit01-sets-membership.sokonanoda
scripts/soko query holes  --file <abs>/unit01-sets-membership.sokonanoda
scripts/soko query hints  --file <abs>/unit01-sets-membership.sokonanoda --line L --col 1
scripts/soko query state  --file <abs>/unit01-sets-membership.sokonanoda --line L --col 1   # 每行一次
```

判卷一律传**绝对路径**（台账 G-12：相对路径 + 祖先清单会让模块根退化）。

### 2.3 稳定顺序

`decls` / `holes` 保持内核给的顺序（源码顺序）；`lines` 按行号 1..N；
`hints` 的键由 `sort_keys=True` 排序。

### 2.4 缺失行为

| 缺什么 | 结果 |
|---|---|
| 画布文件不存在 | 不生成 `walkthrough.json`，stderr 说明 |
| 判卷环境未就绪（`doctor` 不过） | 不生成（**不编造逐行状态**） |
| `query goals` 没答上来 | 不生成（声明表是这一页的骨架） |
| `query holes` / `query hints` 没答上来 | 仍生成，但**不带** `holes` / `hints`，stderr 说明 |
| 某一行的 `query state` 没答上来 | 该行记 `ok:false` + `error:{code:"query-failed",message}`，其余行照常 |
| 某行落在声明之外 | 内核的**真实答案**（`ok:false` + `outside-declarations`），原样记录 |

### 2.5 诚实注记

- 记录的是内核的**原始字段**，不做二次渲染：目标里的 `@Eq.{1}` 这类 pp 文本是冻结内核的输出，逐字保留；
- span 一律按**字节**解释（台账 G-15），offset→行号的换算全部走
  `byte_line_starts()`，绝不用 `text[:off]` 字符切片。

---

## 3. `events.json`（B1 事件流页）

### 3.1 判卷样例画布

脚本在 `.cache/site-lab/events.sokonanoda` 写一份**故意小**的画布，它同时产生：

| 事件 | 来自哪一行 |
|---|---|
| `decl.checked` ×3 | `def id`、`def compose`、`theorem and_comm` |
| `exercise.open` ×2 | `example : True := sorry`（**无名**事件）、`theorem open_exercise (a : Prop) : a -> a := sorry` |
| `expr.typed` / `expr.reduced` / `decl.printed` | `#check id` / `#reduce 1 + 2` / `#print id` |
| `diagnostic`（**stage=kernel**） | `theorem kernel_rejected : True := True.intro True.intro` → `kernel-expected-pi` |

最后一条是**内核阶段**的真拒绝：签名 elaborate 得过、类型也成立，是**内核在判定
证明项时**说不——这正是「判定永远走 kernel」的活证据（不是 elab 阶段的语法拒绝）。

### 3.2 模式

| 字段 | 说明 |
|---|---|
| `canvas` | 画布路径（`.cache/…`，仓库相对） |
| `source` | 画布全文（`.cache` 是临时的，数据必须自足） |
| `command` | 产生这份事件流的**确切命令** |
| `exit_code` | `grade` 的退出码（这份样例是 `1`：有一条真拒绝） |
| `counts` | 按 `type` 计数 |
| `stage_counts` | 按 `stage` 计数（诊断/警告） |
| `decls[]` | `query goals` 的声明表：`name` / `kind` / `status` / `start_line` / `end_line` / `ty` |
| `events[]` | 见下 |

`events[]` 的每一条：

```jsonc
{
  "index": 0,            // 事件在流里的序号（从 0 起）
  "type": "decl.checked",// 原样
  "line": 2,             // ★ 对齐字段：这条事件指源码哪一行
  "line_end": 2,         //   事件带 span 时取 span 的字节范围；否则取声明的行范围
  "decl": "id",          // ★ 对齐字段：这条事件属于哪个声明
  "event": { … }         // ★ 原始事件对象，**一个键都不改**
}
```

### 3.3 事件怎么对齐到源码行

页面要把事件流和代码并排显示，所以每条事件都要有行号：

1. 事件带 `span` ⇒ 用 `span.start.offset` / `span.end.offset`（**字节**）反查行列；
2. 事件带 `name`（`decl.checked` / `decl.printed`）⇒ 用 `query goals` 给的声明字节 span；
3. `example` 的 `exercise.open` **既没有 span 也没有 name** ⇒ 按源码顺序对齐到
   `query goals` 里 `status == "open"` 的声明上（内核把匿名 example 叫
   `example@7`，名字里就带行号）。这是从**内核自己的声明表**推出来的，不是文本比对。

### 3.4 稳定顺序

事件**按 `--json` 吐出的顺序**（`index` 即序号），一条不筛、一条不排序。

### 3.5 缺失行为

| 缺什么 | 结果 |
|---|---|
| 判卷环境未就绪 | 不生成 `events.json` |
| `grade --json` 跑不起来 / 一条事件都没有 | 不生成 |
| 有若干行不是合法 JSON | 丢弃那几行并 stderr 报告条数 |
| `query goals` 没答上来 | 仍生成，但事件**不带** `line` / `decl`（原始事件仍在） |
| 这次判卷没有内核阶段拒绝 | 仍生成，`stage_counts` 如实记录 + stderr 说明（**不伪造一条**） |

---

## 4. `diagnostics.json`（B8 诊断页）

### 4.1 为什么解析 C1 §4.4，而不是重新推导

`docs/design/site-rebuild/content/C1-language.md` §4.4 是**已经核对过的权威摘要**
（60 个错误码 + 4 个警告，按阶段分组，hint 逐字），并且写明了每个阶段的行号出处。
本脚本**只解析这张表**：

- 重新推导（去 grep `crates/**`）会与这份摘要产生**第二个真相**，两边迟早不一致；
- 表里每组自带「（12 个，`crates/front/src/diagnostic.rs:78-90`）」这样的**声明计数与
  出处**，脚本把它们一起收下，并在解析出的条数与声明计数不符时 stderr 报警。

> 注：`D2-information-architecture.md` §2 写的是「解析 `docs/protocol.md`」。
> 实测 `docs/protocol.md` 只在正文里举例提到几个码，**没有**完整码表；C1 §4.4 才是
> 完整且逐字的那一份。本文以此为准。

### 4.2 模式

| 字段 | 说明 |
|---|---|
| `source` | `docs/design/site-rebuild/content/C1-language.md §4.4` |
| `table_heading` | 表标题（逐字） |
| `output_normalization` | 记录下来的 CLI 输出里，仓库绝对路径已替换为 `<repo>` |
| `repro_dir` | 复现件暂存目录（`.cache/site-lab/repro/<code>/`） |
| `totals` | `codes` / `errors` / `warnings` / `by_stage` / `with_repro` / `without_repro` |
| `codes[]` | 见下 |

`codes[]` 的每一条：

```jsonc
{
  "code": "kernel-expected-pi",
  "stage": "kernel",              // parse | elab | kernel | import | warning
  "meaning": "非函数值被当函数用，或应用参数给多了",   // C1 §4.4 的「含义」列
  "hint": "你把一个不是函数的值当函数用了…",          // 逐字
  "hint_kind": "verbatim",        // verbatim = 「」里逐字；note = 原文不是逐字文案
  "source_ref": "crates/front/src/compile/error.rs:191-202",
  "repro": { … } | null
}
```

`repro` 不为 null 时：

| 字段 | 说明 |
|---|---|
| `mode` | `query-check`（默认）或 `grade-json` |
| `command` | 产生这份输出的确切命令 |
| `exit_code` | 命令的退出码 |
| `dir` | 复现件目录（`.cache/site-lab/repro/<code>/`） |
| `entry` | 入口文件名 |
| `manifest` | 同目录 `sokonanoda.toml` 的内容（给模块根定位；`manifest-invalid` 用的是坏 TOML） |
| `files` | `{文件名: 源码}`——**复现件全文**，数据自足，不依赖 `.cache` 还在 |
| `observed_codes` | 这次输出里出现的**全部**码（失败 + 警告） |
| `output` | 命令的**完整输出**：`query-check` 是那个 JSON 对象；`grade-json` 是事件对象数组 |
| `unparsed_lines` | 输出里不是合法 JSON 的行数 |

### 4.3 复现是**核对过**的

`_run_repro()` 只在**真输出里出现了这个码**时才记 `repro`。所以：

- 内核行为一变，复现会**自动退化成 `repro: null`** + stderr 的人话原因，而不是留下
  一条与事实不符的「复现」；
- 复现表里有、C1 §4.4 里没有的码会被 stderr 报出来（两处漂移要被看见）。

`mode` 的选择有实质理由：`query check` 给的是**入口文件视图**，依赖模块自己的病只以
`import-dependency-failed` 进 `failed[]`。所以 `import-cycle` 用**自导入**（病在入口上，
`query check` 看得见），`import-module-invalid` 用 `grade --json`（病在被导入模块上，
只有事件流带 `module` 字段）。

### 4.4 稳定顺序

`codes[]` **按 C1 §4.4 的表序**（parse → elab → kernel → import → warning，组内按表内
行序），不重排——表序是文档作者按「学习者先撞到哪个」排的。

### 4.5 缺失行为

| 缺什么 | 结果 |
|---|---|
| C1 §4.4 读不出来 / 找不到 §4.4 区间 / 一条码都解析不出 | 不生成 `diagnostics.json`（**不重新推导**） |
| 某个分组头解析不了 | 跳过该组并 stderr 报告（组内行会因为没有 stage 而被跳过） |
| 解析出的条数与文档声明的个数不符 | 仍生成，stderr 报警 |
| 判卷环境未就绪 | 仍生成**字典部分**，所有 `repro: null` + 原因写「判卷环境未就绪」 |
| 某条复现没触发那个码 | 该条 `repro: null` + `repro_absent_reason` 写**实际输出**，stderr 报告 |
| 某条码本脚本没写复现 | `repro: null` + 原因（区别于「写了但跑不出来」） |

---

## 5. `timeline.json`（E1 进度页）

### 5.1 三个硬事实（C4 §2.3，忽略任何一条时间线就是错的）

1. **有 CHANGELOG 条目但没有 tag 的版本**——其中 `0.57.0` 是用户可见的大版本。
   网站写「0.57.0 发布了什么」要写功能，**不要**声称有对应 tag；
2. **若干版本的 CHANGELOG 日期比 tag 提交日期早一天**（跨零点提交）；
3. **一天可以发很多版**——必须聚合，否则时间线会被一天 24 行淹没。

脚本把这三条**算出来**（`facts` 字段），不抄文档：`versions_without_tag`、
`changelog_date_skew`、`tags_per_day` + `busiest_day`。

### 5.2 日期口径（只选一种，逐条写明）

> 有 tag 的版本取 **tag 指向提交的日期**（`git log -1 --format=%ad --date=short <tag>`）；
> 无 tag 的版本取 `editor/vscode/CHANGELOG.md` 的标注日期。

每一条骨架行带 `date_source`（`tag-commit-date` / `changelog-date`）与
`date_matches_tag`（该行的日期是否等于 tag 提交日期），页面不需要猜口径。

### 5.3 模式

| 字段 | 说明 |
|---|---|
| `date_convention` | 上面那段口径的原文 |
| `sources` | 五个源各自的读法（含确切命令） |
| `facts` | 三个硬事实 + 各种计数（见 §5.5） |
| `days[]` | 按日期聚合：`{date, count, tags[]}`——**一天多版就是一行** |
| `spine[]` | C4 §2.2 的精选骨架（39 行），逐行对 `git tag` 核对 |
| `tags[]` | 全部 tag：`{tag, version, date, subject}` |
| `changelog[]` | 每个 CHANGELOG 版本：`{version, date, one_liner, truncated, line}` |
| `rounds[]` | `STATUS.md` + `docs/STATUS-ARCHIVE.md` 的轮次标题 |
| `requirements[]` | `REQUIREMENTS.md` §9 的 dated 记录：`{date, text, truncated}` |

`spine[]` 的每一行在 C4 原表四列之外，另加：

| 字段 | 说明 |
|---|---|
| `tags_found` / `tags_missing` | 该行提到的版本号里，哪些有 tag、哪些没有 |
| `tag_date` / `date_source` / `date_matches_tag` | 核对结果 |
| `changelog_date` | 没有 tag 时，CHANGELOG 里的日期 |

### 5.4 命令

```bash
git tag
git log -1 --format=%ad --date=short <tag>      # 每个 tag 一次
git log -1 --format=%s <tag>                    # 每个 tag 一次（tag 指向提交的标题）
grep -n '^## \[' editor/vscode/CHANGELOG.md     # 版本 + 日期（脚本用正则解析）
grep -n '^## 本轮进度' STATUS.md docs/STATUS-ARCHIVE.md
grep -n '^- 2026-' REQUIREMENTS.md              # §9 的 dated 记录
```

### 5.5 `facts` 里有什么

`tag_count` / `changelog_versions` / `versions_without_tag`(+`_count`) /
`versions_without_changelog` / `changelog_date_differs_from_tag_date` /
`changelog_date_skew[]` / `days_with_tags` / `tags_per_day` / `busiest_day` /
`spine_rows` / `spine_rows_with_tag` / `rounds` / `requirements_entries` /
`v0_57_0_has_tag`。

### 5.6 稳定顺序

- `tags`：按版本号自然序（`0.9.1` 排在 `0.10.0` 前面，不是字典序）；
- `spine`：保持 C4 §2.2 的表序（那是**叙事顺序**，不能重排）；
- `days`：按日期升序；`changelog`：保持 CHANGELOG 的倒序（最新在前）；
- `rounds`：按 `(日期, 轮次号, 来源, 原文)` 排序；
- `requirements`：保持 §9 的顺序（那是要求追加的时间序）。

### 5.7 缺失行为

| 缺什么 | 结果 |
|---|---|
| `CHANGELOG.md` 读不出来 / 一条版本都没解析出 | 不生成 `timeline.json`（时间线的一半是它） |
| `git tag` 读不到 | 不生成（tag 日期是日期口径的基础） |
| 某个 tag 的提交日期读不到 | 该 tag 的 `date` 记 `null`，其余照常 |
| C4 §2.2 区间找不到 | 仍生成，但**不带** `spine`，stderr 说明 |
| C4 §2.2 里某行的版本既没 tag 也不在「已知无 tag」名单 | stderr 报警（这是漂移，不是既成事实） |
| 某条轮次标题连日期都读不出 | 跳过该条 + stderr 报告 |
| 轮次标题没有「第N轮：标题」结构 | **仍收录**：`round` 记 `null`、`round_parsed: false`、`raw` 留原文（归档标题是人写的散文，不丢信息） |
| `REQUIREMENTS.md` §9 找不到 | 仍生成，但不带 `requirements`，stderr 说明 |

---

## 6. `gaps.json`（E2 工程页）

### 6.1 模式

| 字段 | 说明 |
|---|---|
| `source` | `docs/gaps/ledger.jsonl` |
| `summary` | 汇总计数（见 §6.3） |
| `entries[]` | 见下 |

`entries[]` 的每一条：

| 字段 | 来源 |
|---|---|
| `id` / `title` / `kind` / `severity` / `status` / `found` / `found_by` / `fixed_in` / `owner` | 台账字段原值 |
| `repro` / `wo` | 台账字段原值（可能是 `null`） |
| `repro_exists` / `wo_exists` | **磁盘上真的有没有那个文件**（实测） |

`repro_exists` 是必须的：台账字段可以指向一个已经删掉的文件，页面据此渲染链接就会
404。两个字段都记，页面可以据此把坏链渲染成「已撤下」。

### 6.2 稳定顺序

`entries` 按 `id` 字典序（台账本身按主题分组，顺序会随编辑漂移）。

### 6.3 `summary` 里有什么

`entries` / `by_status` / `by_severity` / `by_kind` / `by_fixed_in` /
`with_repro` / `with_repro_on_disk` / `with_work_order` / `with_work_order_on_disk` /
`open`（`status` 既不是 `fixed` 也不是 `workaround` 的条数）。

注意 `with_work_order`（**引用**工作单的台账条数）与 `evidence.json` 的
`work_orders`（**磁盘上**的工作单文件数）是两个不同的口径，页面不要混用。

### 6.4 命令

```bash
wc -l < docs/gaps/ledger.jsonl        # 行数
python3 scripts/gap.py list           # 仓库自己的台账视图（人读；脚本不解析它的输出）
```

脚本**逐行 `json.loads`**，不重实现 `gap.py` 的判据（判据双实现必然漂移）。

### 6.5 缺失行为

| 缺什么 | 结果 |
|---|---|
| `ledger.jsonl` 不存在 / 读不出来 | 不生成 `gaps.json` |
| 某一行不是合法 JSON / 不是对象 | **整节不生成** + stderr 指出第几行——半解析的台账会把计数说小 |
| 某条的 `repro` / `wo` 指向的文件不在盘上 | 该条照常收录，`*_exists` 记 `false` |

---

## 7. `evidence.json`（E2 工程页）

### 7.1 模式

```jsonc
{
  "rule": "只收能复现的数字；每条带 command；拿不到的不进 JSON，只在 stderr 说明",
  "measured": {
    "<id>": {
      "value": <数字 | 对象>,
      "command": "<产生它的确切命令>",
      "source": "<源文件 / git / GitHub Release>",
      "note": "…"            // 可选
    }
  }
}
```

### 7.2 收了哪些数字

| id | 内容 | 命令 |
|---|---|---|
| `repo_version` | 仓库版本 | `grep -n '^version' Cargo.toml` |
| `extension_version` | 扩展版本（必须与上者一致） | 读 `editor/vscode/package.json` |
| `course_gate` | 卷 I 门禁 `summary`（targets / checked / open / rejected / units / volumes / chapters…） | `python3 courses/set-theory/tools/check.py --json` |
| `playground_events` | `playground.sokonanoda` 的事件计数 | `scripts/soko grade playground.sokonanoda --json` |
| `gaps_ledger` / `perf_ledger` / `e2e_ledger` / `courses_ledger` | 四本台账的行数 | `wc -l < <path>` |
| `ledger_rows` | 上面四个的合并视图 | 同上 |
| `ci_failures` | `docs/CI-FAILURES.md` 的 dated 条目数 | `grep -cE '^#{2,3} [0-9]{4}-…'` |
| `work_orders` / `repro_files` | 工作单 / 复现件文件数 | `ls docs/gaps/WO-*.md \| wc -l` 等 |
| `git_tags` / `git_commits` / `first_commit_date` / `latest_commit_date` | git 事实 | `git …` |
| `changelog_versions` | CHANGELOG 版本条目数 | `grep -c '^## \[' …` |
| `sokonanoda_files` | 仓库里的 `.sokonanoda` 文件数（不含 `target/`、`.cache/`） | `find … \| wc -l` |
| `e2e_latest` | 最新一次真 VS Code e2e（版本 / commit / VS Code 版本 / 用例数 / exit） | `tail -1 docs/e2e/ledger.jsonl` |
| `release_assets` | 每个 Release 的资产数 | `gh release view v<version> --json assets -q '.assets \| length'` |
| `test_counts` | 测试总数 / `#[ignore]` / 可运行数 | `cargo test --workspace --locked -- --list`（见 §7.3） |

### 7.3 贵的那一个：`test_counts`

`cargo test --workspace --locked -- --list` 在本机要**约 5 分钟**（每次都会重新链接
测试二进制），所以默认运行**不跑它**：

- 默认：读 `.cache/site-lab/test-counts.json`（`--with-tests` 写下的一次实测），
  连同 `command` / `measured_at` / `measured_commit` / `stale` 一起记进数据；
- `--with-tests`：真跑 cargo（total + ignored 两次）并刷新那个缓存；
- 缓存不存在：**不进 JSON**，stderr 说明「加 `--with-tests`」。

`stale: true` 表示测量时的 commit 已不是 HEAD——数据自己说得出来。

> 本机 cargo 需要 `DEVELOPER_DIR=/Library/Developer/CommandLineTools` 才能链接
> （Xcode 许可未接受，`docs/HANDOVER.md` 已文档化的坑，**不是仓库 bug**）；
> 脚本用 `setdefault` 注入它，所以外部已经设了就不会被覆盖。

### 7.4 稳定顺序

`measured` 的键由 `json.dump(sort_keys=True)` 排序；对象内的字段顺序固定。

### 7.5 缺失行为

| 缺什么 | 结果 |
|---|---|
| 课程门禁跑不起来 / 输出不是 JSON / 没有 `summary` | 不带 `course_gate`，stderr 说明 |
| `playground.sokonanoda` 判卷没有事件 | 不带 `playground_events` |
| 某本台账不存在 | 不带对应的 `*_ledger`（其余三本照常） |
| `gh` 没装 / 没登录 / 没网络 | 不带 `release_assets`（**不猜 26**） |
| cargo 没量过 | 不带 `test_counts` |
| `e2e/ledger.jsonl` 最后一行不是 JSON 对象 | 不带 `e2e_latest` |

---

## 8. 缺失源行为总表

| 源 | 缺了会怎样 |
|---|---|
| `Cargo.toml` 的 workspace version | 所有文件都不带 `version`（stderr 说明） |
| git（`rev-parse` / `log`） | 不带 `source_commit` / `generated_at` |
| `scripts/soko` 或判卷环境 | `walkthrough` / `events` 不生成；`diagnostics` 只留字典 |
| `courses/set-theory/units/unit01-…sokonanoda` | 不生成 `walkthrough.json` |
| `C1-language.md` §4.4 | 不生成 `diagnostics.json` |
| `CHANGELOG.md` 或 `git tag` | 不生成 `timeline.json` |
| `C4-status-roadmap.md` §2.2 | `timeline.json` 不带 `spine` |
| `docs/gaps/ledger.jsonl` | 不生成 `gaps.json` |
| 四本台账 / `CI-FAILURES.md` / `gh` | `evidence.json` 少对应的那几组数字 |

**一条通则**：缺源 ⇒ 少一个字段或一份文件 + stderr 一句人话；**绝不**用「差不多的
数字」顶替。脚本只有自己坏了才非零退出。

---

## 9. 实测记录

**怎么跑的**：`python3 scripts/gen-site-lab.py`（全量，无 `--only`），连跑两次。
本机 macOS / arm64，仓库版本 `0.61.0`，`source_commit` = 本次运行时的 HEAD。

### 9.1 运行时间与体积

| 文件 | 字节 | KB | 上限（本任务） |
|---|---:|---:|---:|
| `site/data/walkthrough.json` | 133 455 | 130.3 | 400 KB |
| `site/data/events.json` | 5 288 | 5.2 | 400 KB |
| `site/data/diagnostics.json` | 112 553 | 109.9 | 400 KB |
| `site/data/timeline.json` | 158 627 | 154.9 | 400 KB |
| `site/data/gaps.json` | 12 557 | 12.3 | 400 KB |
| `site/data/evidence.json` | 5 125 | 5.0 | 400 KB |

- **全量墙上时间**：暖缓存 `36.0 s` / `37.4 s`，冷启动或系统有负载时 `41.1 s` / `58.5 s`
  （`walkthrough` 的 83 次 `query state` 与课程门禁的 25 s 是大头；诊断的 55 条复现
  只要 5.6 s）。
- **最大产物 158 627 B（154.9 KB）**，离 400 KB 上限还有一倍余量。
- `site/data/site.json` **未被本脚本改动**（它由 `scripts/gen-site-data.py` 生成）。

### 9.2 确定性（两次运行的 sha256）

| 文件 | sha256（两次相同） |
|---|---|
| `walkthrough.json` | `836c38a745668fe133f86eee48a4637d5bb7f0bfe2f132b6677e3711f8ea6b86` |
| `events.json` | `65da16a5cb8f896d5db3dbe70c4a71da878c58ec9e223ee80e4ee2e4a2fee572` |
| `diagnostics.json` | `6f7714649f839f9722af4aa6902e11c7b65fb0bc9f714bdc8b77f57cac02a924` |
| `timeline.json` | `511f1cdc054fa59ae04d75bd27fdc1bea29ab931f5a9ef6e801a6c08ed039de0` |
| `gaps.json` | `dd1bb182213c9327e3f4b01fdc8c11f98d6a1d8a0806cb61dc25502308d5821a` |
| `evidence.json` | `db841320ce1b852566c803e2aa326308cbf7e86849803bbdae1da9b50182174c` |

`diff` 两次的 `shasum -a 256` 输出：**逐字节一致**。六份产物全部
`json.loads` 通过，且都带 `version=0.61.0` + `source_commit` + `generated_at`。

### 9.3 诊断复现覆盖率

**55 / 64**（错误码 55/60，警告 4/4）。9 条没有复现的码与原因见 §10 第 1 条，
完整原因逐条写在 `diagnostics.json` 的 `repro_absent_reason` 里。

`totals` 实测：`{codes: 64, errors: 60, warnings: 4, by_stage: {parse: 12, elab: 29,
kernel: 12, import: 7, warning: 4}, with_repro: 55, without_repro: 9}`——
与 C1 §4.4 的声明分组（12/29/12/7 + 4）**逐组相符**。

### 9.4 事件流实测

`.cache/site-lab/events.sokonanoda`（12 行）→ `9` 条事件、`grade` 退出码 `1`：
`decl.checked=3` · `exercise.open=2` · `expr.typed=1` · `expr.reduced=1` ·
`decl.printed=1` · `diagnostic=1`，其中 **`stage_counts = {kernel: 1}`**——
内核阶段的真拒绝（`kernel-expected-pi`）拿到了。9 条事件全部有 `line` 对齐字段。

### 9.5 时间线实测（三个硬事实）

| 事实 | 实测 |
|---|---|
| tag 数 | **69** |
| CHANGELOG 版本条目 | **75** |
| 有 CHANGELOG 但**没有 tag** | **6**：`0.1.0` / `0.2.0` / `0.3.0` / `0.57.0` / `0.6.0` / `0.9.1` |
| CHANGELOG 日期 ≠ tag 提交日期 | **19** 个（`changelog_date_skew[]` 逐条列出） |
| 一天最多的 tag | **2026-09-15：24 个**（`v0.29.0` → `v0.48.0`） |
| `v0.57.0` 有 tag 吗 | **没有**（`facts.v0_57_0_has_tag = false`） |
| C4 §2.2 骨架 | **39 行**，其中 **33 行**核到 git tag；6 行没有 tag（M0/M1 里程碑 + 4 个无 tag 版本） |
| 轮次标题 | **108** 条（`STATUS.md` + `docs/STATUS-ARCHIVE.md`），2 条无「第N轮」结构 |
| `REQUIREMENTS.md` §9 | **123** 条 dated 记录 |

与 `C4-status-roadmap.md` §2.3 的三条硬事实**逐条相符**（69/75/6、19、24）。

### 9.6 `gaps.json` 与台账逐项对照

| 口径 | `ledger.jsonl` | `gaps.json` | 一致 |
|---|---|---|---|
| 条数 | 24 | 24 | ✅ |
| `status` | `fixed 22` / `workaround 2` | 同 | ✅ |
| `kind` | `language 10` / `tooling 7` / `library 6` / `infra 1` | 同 | ✅ |
| `severity` | `blocker 12` / `painful 10` / `nice 2` | 同 | ✅ |
| `open` | 0 | 0 | ✅ |
| 带 `repro` 字段 | 20 | `with_repro: 20`，**在盘 20/20** | ✅ |
| 带 `wo` 字段 | 15 | `with_work_order: 15`，**在盘 15/15** | ✅ |

> `with_work_order = 15` 是**引用工作单的台账条数**（多条形如 G-11/G-16 共用一张
> WO-001）；`evidence.json` 的 `work_orders = 11` 是**磁盘上的工作单文件数**。
> 两个口径都对，页面不要混用（C4 §6.3 写的是 11 张工作单）。

### 9.7 `evidence.json` 收到的数字（21 组）

全部与 C4 §7.1 的标注值相符，**除了两个**（都是仓库长了，不是漂移）：

| id | 实测 | C4 §7.1 标注 |
|---|---|---|
| `course_gate` | `targets 36` · `checked 329` · `open 99` · `rejected 0` · `units 12` · `volumes 1` · `chapters 4` | 36 / 329 / 99 / 0（✅） |
| `playground_events` | `decl.checked 30` · `example.checked 2` · `exercise.open 4` · `warning 2` | 30 / 2 / 4 / 2（✅） |
| `gaps_ledger` / `perf_ledger` / `e2e_ledger` / `courses_ledger` | 24 / 14 / 37 / 1 | 24 / 14 / 37 / 1（✅） |
| `ci_failures` | 35 | 35（✅） |
| `work_orders` / `repro_files` | 11 / 22 | 11 / 22（✅） |
| `git_tags` / `git_commits` | 69 / 367 | 69 / 367（✅） |
| `first_commit_date` / `latest_commit_date` | 2026-09-06 / 2026-09-19 | 同（✅） |
| `changelog_versions` | 75 | 75（✅） |
| `release_assets` | **26**（`gh release view v0.61.0`） | 26（✅；C4 标 ⚠️，本脚本用 `gh` 复核了） |
| `e2e_latest` | 0.61.0 · VS Code 1.138.0 · `passed 15 / failed 0` | 15 / 15（✅） |
| `test_counts` | **total 1170 · ignored 6 · runnable 1164** | C4 写 1169 / 6 / 1163（**差 1**，见下） |
| `sokonanoda_files` | 109 | 109（✅） |

**`test_counts` 差 1 的解释**：C4 §7.1 第 12–14 行写的是写作时的实测值
（1169 / 6 / 1163）；本次实测是 **1170 / 6 / 1164**——仓库在这之后又多了 1 个测试。
本脚本记的是**现在量出来的数**，带 `measured_commit`（`stale: false` 表示测量的
commit 就是 HEAD）。数字以数据里的 `command` 为准，不以文档为准。


---

## 10. 已知边界与诚实注记

1. **64 条码里有 9 条跑不出最小复现**——每一条都在 `repro_absent_reason` 里写明了
   *为什么*（多数是 `ErrorKind` 从未被构造、或触发条件需要 >65535 个嵌套结构）。
   这些是**语言/工具的边界事实**，不是本脚本的失败。
2. **`query check` 不读 `-- sokonanoda:prelude none` 指令**（实测：`grade` 报
   `elab-unknown-identifier`，`query check` 报 `decl_checked: 1` + 零失败）。
   这与台账 G-10/G-17 同族，但**不在本任务的面内**——本脚本只是绕开它
   （需要 Bare 模式的码改用 `grade --json`）。
3. **记录下来的 CLI 输出做过一处替换**：仓库绝对路径 → `<repo>`。原样落盘会把数据
   绑死在这台机器的检出目录上；替换无损、可逆，且写进了数据（`output_normalization`）。
4. **`timeline.json` 的 `changelog[].one_liner` 是第一条 bullet 的折叠文本**，
   超过 240 字的标 `truncated: true`；它不是全文，全文在 CHANGELOG 里。
5. **`rounds[].round` 有 2 条是 `null`**（归档标题没有「第N轮」结构），原文在 `raw` 里。
6. **单文件脚本**：`AGENTS.md` 硬规则 5 建议接近 500 行即拆分，但本任务的允许修改面
   只有 `scripts/gen-site-lab.py` 一个脚本文件，所以六个生成器共处一文件、按
   `───` 分区。若日后要拆，最自然的切法是「每个 `build_*` 一节一个模块」。
