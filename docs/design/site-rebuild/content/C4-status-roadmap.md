# C4 —— 现状与路线图（站点重建用事实卷宗）

> **这份文件是什么**：官网要重建「进展（progress）」与「愿景/路线图（vision）」两页，
> 本文是这两页的**唯一事实来源**。它只写能验证的东西：版本、轮次、命令输出、`file:line`。
>
> **读者**：写页面的人（人或 agent）。**不是**给终端用户读的成品文案——它是文案的原料。
>
> **证据约定**：
> - 每个数字后面给**产生它的确切命令**或 `文件:行`；两者都有时都写。
> - 引号里的中文若出自 `REQUIREMENTS.md` §9，即用户原话，**逐字**照抄（含标点与错字）。
> - 凡本机实测的数字，都在 §7 标注「可复现」与当时的复现环境；
>   没有命令能复现的，一律标 **未证实**，不许写进网站。
> - 快照时间：**2026-09-19**，仓库工作树状态见 §7.3。
>
> **不要做的事**：不要从这份文件里挑形容词；它刻意不含形容词。网站要「内容为王」，
> 内容就是下面这些可验证的事实本身。

---

## 1. 项目现在到底在哪

### 1.1 一句话与快照

仓库自己的「一句话」（`STATUS.md:17-21`，逐字）：

> `.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
> 练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
> CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

快照（全部当场验证）：

| 项 | 值 | 验证命令 / 出处 |
|---|---|---|
| 仓库版本 | **0.61.0** | `grep -n "^version" Cargo.toml` → `Cargo.toml:6` |
| 扩展版本（必须一致） | **0.61.0** | `grep -n '"version"' editor/vscode/package.json` → `editor/vscode/package.json:5` |
| 开发轮次 | **108 轮** | `cat STATUS.md docs/STATUS-ARCHIVE.md \| grep -c "^## 本轮进度"` → 108 |
| 已发布版本数 | **69 个 git tag** | `git tag \| wc -l` → 69 |
| CHANGELOG 版本条目 | **75 个**（0.1.0 → 0.61.0） | `grep -c "^## \[" editor/vscode/CHANGELOG.md` → 75 |
| 提交数 | **367** | `git log --oneline \| wc -l` → 367 |
| 首个提交 | 2026-09-06 `92ef75e`（M0：内核快照迁入） | `git log --reverse --format="%h\|%ad\|%s" --date=short \| head -1` |
| 最新提交 | 2026-09-19 `084da05`（0.61.0 发布结果入档） | `git log -1 --format="%h\|%ad\|%s" --date=short` |
| 最近一次全量测试 | **1163 passed / 0 failed / 6 ignored** | `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --workspace --locked` → exit 0；计数见 §6.2 |
| 课程门禁（卷 I 集合论） | **36 目标 · 329 checked · 99 open · 0 判负** | `SOKONANODA_BIN="$PWD/target/debug/sokonanoda" python3 courses/set-theory/tools/check.py --json` |
| 缺口台账 | **24 条：22 fixed + 2 workaround，open = 0** | `python3 scripts/gap.py list`；`python3 scripts/gap.py check` → exit 0 |
| 内核改动 | 0（0.61.0 这一版一个字节未动） | `STATUS.md:57`；硬规则见 `REQUIREMENTS.md:15` |

三条**产品现状**（不是愿景）：

1. **判卷是真内核判卷**：CLI `sokonanoda grade` / `--json` 事件流 / `query <op>` 单 JSON
   三条出口都驱动同一份完整内核；「判定永远走 kernel，不做文本比对」是硬规则
   （`REQUIREMENTS.md:22`）。
2. **用户/agent 零工具链**：拿 Release 二进制或 VSIX 就能跑，**不要求 Rust/cargo**
   （`REQUIREMENTS.md:23-26`，硬规则 9）。
3. **发布全自动**：push main → `ci.yml` 的 auto-tag 打 tag → dispatch `release.yml`
   → 26 个资产 + Marketplace（`STATUS.md:58-64`、`docs/RELEASE.md`）。

### 1.2 已落地（今天能端到端跑通的事）

判定口径：**有发布产物、有测试、有文档，且用户拿 Release 二进制或 VSIX 就能用**。

| 能力 | 现状 | 证据 |
|---|---|---|
| `.sokonanoda` 语言（lexer/parser/elaborator/练习引擎） | 完整可用；纯声明式，无 `#` 命令 | `STATUS.md:19-21`；`docs/architecture.md` |
| 完整 sokonanoda 内核（inductive / quot / proof irrelevance / conv / eval / pp） | 完整迁移，冻结快照 | `ROADMAP.md:49`；`REQUIREMENTS.md:15` |
| prelude（Full / Bare 双模式） | Full 模式自带 **47 个**顶层名字；Bare 完全不加载 | `grep` 实测 `PRELUDE_NAMES` 长度 = 47（`crates/front/src/compile/prelude.rs`）；`REQUIREMENTS.md:64-73` |
| CLI：`grade` / `--json` 事件流 / `query <op>` / `build` / `course` / `watch` / `setup` / `doctor` / `update` / `version` | 全部可用，退出码契约稳定 | `AGENTS.md` Setup 段；`docs/protocol.md` |
| `query` 七视图：`check`/`state`/`goals`/`holes`/`hints`/`reduce`/`project` | 单 JSON 对象，退出码 0/1/2 | `ROADMAP.md:546-600`（I15 落地） |
| LSP：诊断 / hover / goal 视图 / semantic tokens / codeLens / code action / inlay / 跨文件跳转引用改名 | 可用 | `ROADMAP.md:366-367`；`docs/protocol.md` |
| VS Code 扩展（9 平台 VSIX，内嵌 LSP + CLI） | 可用；Marketplace 收录 | `STATUS.md:126-127`、`STATUS.md:243-244` |
| 入门课 `course/` | **11 单元**（10 单元锁定 + 单元⑪ 模块与项目），中英双语 + 解答钥匙 | `course/course.json`（11 条）；`ROADMAP.md:393-416`；I16 P6 见 `ROADMAP.md:623` |
| 卷 I 集合论课程 `courses/set-theory/` | **12 单元 / 4 章 / 1 卷**，8 个 lib 模块 + `Demo` 自检入口 + 记法对照页 + 12 份解答 | `python3 courses/set-theory/tools/check.py --json` → `units=12`；`courses/set-theory/course.json`（`soko.course/2`）；lib 清单见 `ls courses/set-theory/lib` |
| 多文件 `import` + 项目管理 | `import Foo.Bar` + 可选 `sokonanoda.toml`，闭包编译、闭包哈希缓存、跨文件 LSP | `ROADMAP.md:602-648`（I16）；`docs/HANDOVER.md:266-334` |
| 用户自定义记法（`infix`/`infixl`/`infixr`/`notation`/`prefix`/`postfix`/`binder_notation`/`scoped`/集合字面量） | 三刀全落（0.59.0 → 0.61.0） | `STATUS.md:29-34`；`docs/design/notation-subset.md` |
| `namespace`/`open`/`export` + 子句 + `open … in` | 已落（0.60.0 起，0.61.0 扩展） | `STATUS.md:35-38`；`docs/design/namespace-open.md` |
| agent 通道：MCP 六个工具 + DSH 技能/命令 | 可用；MCP 默认关闭需 opt-in | `ROADMAP.md:568-572`；`dsh/README.md` |
| 性能台账 / e2e 台账 / 缺口台账 / CI 失败台账 | 四本台账都在仓库里，且**接进门禁** | §6.3 |

**端到端最小证据**（本机实跑，命令可复制）：

```bash
# 判卷一个画布：30 条 decl.checked + 2 条 example.checked，4 道练习开着，2 条 warning
target/debug/sokonanoda --json playground.sokonanoda | \
  python3 -c "import sys,json,collections;c=collections.Counter(json.loads(l)['type'] for l in sys.stdin if l.strip());print(dict(c))"
# → {'decl.checked': 30, 'example.checked': 2, 'exercise.open': 4, 'warning': 2}

# 入门课聚合：11 单元 · 86 checked · 65 open · 0 failed
node scripts/soko course "$PWD/course/course.json" --json | grep course.summary

# 卷 I 聚合：12 单元 · 65 checked · 96 open · 0 failed（只算入口模块）
node scripts/soko course "$PWD/courses/set-theory/course.json" --json | grep course.summary
```

### 1.3 进行中（半成品 / 有明确边界的东西）

「进行中」= 主干能用，但存在**已登记、有理由**的缺口。每条都有台账或 TODO 编号。

| 项 | 到哪一步了 | 剩什么 | 证据 |
|---|---|---|---|
| 记法系统 | 三刀全落：运算符 / 前缀后缀 / binder 记法 + 重载 + `scoped` + 集合字面量 | **print-back 不做**（内核 pp 冻结，见 §5.1） | `STATUS.md:29-34`、`STATUS.md:52-54` |
| `namespace`/`open` | 子句 / `open … in` / `export` / 遮蔽 warning 全落 | `section`/`variable` **做不动**（有实测，见 §5.1） | `STATUS.md:35-40` |
| 宇宙层级 | `u+1` 数字后缀加法；`Eq.mp`/`Eq.mpr` 宇宙多态；`cast`/`Eq.ndrec` | `u+v` / `max` **不做**（内核 `Level::Max` 无公开构造入口） | `STATUS.md:41-45` |
| `match` | 递归 IH / 依赖 motive / 参数化 / 带索引 / 嵌套字面量守卫模式全落 | `as`/or 模式、多 scrutinee、`if/then/else` 表达式 | `docs/HANDOVER.md:177-187` |
| 项目层（I16） | P0–P6 全落，P5 全部做完 | P7 长尾：`watch` 项目模式、`[deps]`（`namespace` 已于 0.60.0 落地） | `ROADMAP.md:639-641`、`docs/HANDOVER.md:128-129` |
| DeepSeek Harness 适配 | H0–H4 全绿（技能 / 启动器 / LSP 接线 / deny / 治理） | H5 backlog：Infoview 客户端插件、诊断通道、`SessionStart` provisioning、npm 插件包 | `ROADMAP.md:586-588`、`docs/HANDOVER.md:263-264` |
| goal 子洞期望类型 | spine meta 方案 A 落地（请求期内核探针） | 更深嵌套、`def` 包裹结果类型的 whnf → 仍走 B′（需内核/pp 暴露 whnf，违反冻结） | `docs/HANDOVER.md:473-474` |
| 性能例行化 | perf 套件 + 台账 + CI report 全落 | 外部基准（Lean Kernel Arena）仍 **opt-in**，不进 CI | `docs/HANDOVER.md:479-480` |
| 官网 | **10 个页面**（`find site -name "*.html"`，含 `site/en/index.html`）+ 生成式数据 + 卫生检查 | **本轮正在重建**（本文即输入之一）；WASM playground 在 backlog | `python3 scripts/check-site.py` → `site hygiene: ok (10 pages, links and versions clean)` |
| 模块化（~500 行惯例） | `run_pass` 已拆完（791/951/413） | 仍有大文件：见 §5.4 实测行数 | `docs/HANDOVER.md:460-466`；本机 `wc -l` 见 §5.4 |

### 1.4 未开始（明确没做的）

| 项 | 状态 | 证据 |
|---|---|---|
| WASM / 浏览器内判题 | **没有 WASM 构建**；明确列为 backlog | `docs/design/site.md:90-96`、`REQUIREMENTS.md:425`、`docs/notes/gap-analysis.md:51` |
| L2/L3：多人协作、远程、compiler service 跨文件转播 / `setContent` | 远期，v1 未做 | `docs/HANDOVER.md:449-451`、`ROADMAP.md:650-658` |
| 类型类（typeclass）、宏（macro） | 不在白名单 | `ROADMAP.md:145`（L0 不包含项） |
| 隐式参数自动插入 | 没有；应用是逐位显式的 | `docs/design/namespace-open.md:364`、`STATUS.md:39-40` |
| 互递归 / 嵌套递归归纳、宇宙多态参数 | 不做 | `docs/HANDOVER.md:475` |
| `[deps]` 跨项目依赖、decl 级编译产物 | backlog | `ROADMAP.md:640-641`、`ROADMAP.md:1151` |

---

## 2. 真实时间线

### 2.1 怎么提取的（可复制的命令）

```bash
# ① 每个 tag 的日期与标题
for t in $(git tag | sort -V); do
  echo "$t|$(git log -1 --format=%ad --date=short "$t")|$(git log -1 --format=%s "$t")"
done

# ② 每个版本的用户可见一句话（权威文案在扩展 CHANGELOG）
grep -n "^## \[" editor/vscode/CHANGELOG.md

# ③ 轮次标题（进度页的原始输入）
grep -n "^## 本轮进度" STATUS.md docs/STATUS-ARCHIVE.md

# ④ 用户要求的时间线（按日期逐条）
grep -n "^- 2026-" REQUIREMENTS.md      # → 123 条

# ⑤ 发布提交
git log --oneline --grep="^release" | head -40
```

### 2.2 时间线（2026-09-06 → 2026-09-19）

**日期口径**：有 tag 的版本取 **tag 指向提交的日期**（`git log -1 --format=%ad --date=short <tag>`）；
无 tag 的版本（0.1.0/0.2.0/0.3.0/0.6.0/0.18.0/0.57.0）取 `editor/vscode/CHANGELOG.md` 的标注日期。
功能描述取自 CHANGELOG 首条或对应轮次的 `STATUS.md` 标题。**注意**：19 个版本的
CHANGELOG 日期比 tag 提交日期早一天（多为跨零点提交），见 §2.3。

| 日期 | 版本 | 一句话 | 证据 |
|---|---|---|---|
| 2026-09-06 | —（M0） | 内核快照（上游 `7b51784`）完整迁入 workspace，测试基线全绿 | `git log --reverse` 首提交 `92ef75e`；`ROADMAP.md:153-176` |
| 2026-09-06 | —（M1） | `.sokonanoda` lexer/parser + CLI 端到端：文件 → AST → 内核声明 → 人读结果 | `6c73dca`；`ROADMAP.md:179-219` |
| 2026-09-07 | 0.1.0 | 首个 LSP 反馈通道：诊断、hover、goal 视图、CodeLens、semantic tokens、code actions | `editor/vscode/CHANGELOG.md` `[0.1.0]` |
| 2026-09-07 | 0.2.0 | go-to-definition、document highlight、binder 补全 | `[0.2.0]` |
| 2026-09-08 | 0.2.1 | CI 在 tag push 时自动发布到 VS Code Marketplace | `[0.2.1]`；`v0.2.1` |
| 2026-09-09 | 0.3.0 → 0.4.2 | 括号/运算符 hover 显示外围表达式类型；括号两侧对称；Marketplace 文案按实况重写 | `[0.3.0]`–`[0.4.2]` |
| 2026-09-09 | 0.5.0 | `by` tactic 块（`intro`/`exact`/`apply`/`assumption`/`rfl`） | `[0.5.0]`；`REQUIREMENTS.md:208-220` |
| 2026-09-09 | 0.5.1 / 0.5.2 | LSP 下载按扩展版本锁定；`by` 块 span 收在最后一个 tactic | `[0.5.1]`、`[0.5.2]` |
| 2026-09-10 | 0.6.0 | 练习树「当前光标处」goal 组 | `[0.6.0]` |
| 2026-09-10 | 0.7.0 → 0.9.0 | LSP 二进制内嵌进 VSIX（per-target）；平台矩阵 4 → 8；VSIX 同时内嵌 CLI | `[0.7.0]`–`[0.9.0]`；`REQUIREMENTS.md:238-262` |
| 2026-09-10 | 0.10.0 | `#check` 结果常驻显示（inlay，对齐 Lean Infoview） | `[0.10.0]` |
| 2026-09-11 | 0.11.0 / 0.12.0 | 保留名 warning；`Type n` 记法 | `[0.11.0]`、`[0.12.0]` |
| 2026-09-11 | 0.13.0 | onboarding 进 CLI：`version`/`doctor`/`setup`/`update`/`grade`/`gate` 子命令 | `[0.13.0]`；`REQUIREMENTS.md:335-343` |
| 2026-09-12 | 0.14.0 / 0.15.0 | 值位 `intro`；声明级 binder（Lean 风格） | `[0.14.0]`、`[0.15.0]` |
| 2026-09-12 | 0.16.0 → 0.17.0 | 原地重启语言服务器；hover 一键展开 `intro` | `[0.16.0]`、`[0.17.0]` |
| 2026-09-13 | 0.20.0 | 发布链自动化闭环（auto-tag + dispatch release）+ 官网上线 | `REQUIREMENTS.md:439-445` |
| 2026-09-13 | 0.21.0 → 0.22.0 | 值位关键字 v2 收口（`funapply` 随后移除） | `[0.21.0]`、`[0.22.0]` |
| 2026-09-13 | 0.23.0 / 0.24.0 | judge 结果缓存 + 半截表达式 goal-state hover；性能测试例行化 | `[0.23.0]`、`[0.24.0]`；`REQUIREMENTS.md:446-460` |
| 2026-09-14 | 0.25.0 / 0.26.0 | `sorry` 洞期望类型精确化；课程量词单元⑦ | `[0.25.0]`、`[0.26.0]` |
| 2026-09-14 | 0.27.0 | 多目标显示 + tactic hover goal state；移除值位 `funintro` | `[0.27.0]`；`REQUIREMENTS.md:495-500` |
| 2026-09-14 | 0.28.0 / 0.29.0 | 值位 `let`；`watch` 编译器服务事件流（`file.didChange` + `service.hello`） | `[0.28.0]`、`[0.29.0]` |
| 2026-09-14 | 0.30.0 / 0.31.0 | webview Infoview 目标面板；扩展强制内置 LSP + `doctor` 自检 | `[0.30.0]`、`[0.31.0]` |
| 2026-09-14 | 0.32.0 → 0.33.1 | spine meta 方案 A；early-cutoff 依赖精确化；`match` v1；tactic hover 排版 | `[0.32.0]`–`[0.33.1]` |
| 2026-09-14 | 0.34.0 → 0.39.1 | 无注解 `let`；`match` 递归 IH / prelude `Nat` / 参数化 / 依赖 motive；`judge_infer` 往返健壮性 | `[0.34.0]`–`[0.39.1]` |
| 2026-09-15 | 0.40.0 | 统一 goal 呈现 + Infoview 落右侧（`front::semantic` 单一分类源） | `[0.40.0]`；`REQUIREMENTS.md:652-667` |
| 2026-09-15 | 0.41.0 / 0.42.0 | prelude `Bool`（真归纳）；`match` 模式编译器 v1（嵌套/字面量/通配/守卫） | `[0.41.0]`、`[0.42.0]` |
| 2026-09-15 | 0.43.0 / 0.44.0 | 呈现面高亮统一（`sokonanoda` 围栏全量）；Infoview 声明类型 + 点击跳转 | `[0.43.0]`、`[0.44.0]` |
| 2026-09-15 | 0.45.0 → 0.47.0 | 应用位 binder 推断；`match` 作为 tactic；带索引归纳（`Vec A n`） | `[0.45.0]`–`[0.47.0]` |
| 2026-09-15 | 0.48.0 / 0.49.0 | 编译结果落盘缓存（olean 式）；共享缓存 + `sokonanoda build` + 高亮单一起源 | `[0.48.0]`、`[0.49.0]` |
| 2026-09-16 | 0.50.0 / 0.51.0 | Infoview 自研固定调色板；`by` 块换行分隔 tactic | `[0.50.0]`、`[0.51.0]` |
| 2026-09-16 | 0.52.0 → 0.54.0 | 课程大纲重构 P1/P2/P3：锁定 10 单元（`by` 提前、归纳拆 Ⅰ/Ⅱ、新增关系与读证明） | `[0.52.0]`–`[0.54.0]`；`REQUIREMENTS.md:804-841` |
| 2026-09-17 | 0.55.0 | DeepSeek Harness 适配 H0–H4 + harness 中立启动器 `scripts/soko` | `v0.55.0`；`REQUIREMENTS.md:886-925` |
| 2026-09-17 | 0.56.0 | 内核真相查询通道：`front::query` + CLI `query` + MCP 六工具；两个 front 缺口修复 | `v0.56.0`；`ROADMAP.md:546-600` |
| 2026-09-17 | 0.56.1 / 0.56.2 | LSP 测试文件债清零；`redundant-sorry` 诊断（「多写了一行」≠「还没证出来」） | `v0.56.1`、`v0.56.2`；`REQUIREMENTS.md:1098-1121` |
| 2026-09-18 | 0.57.0 | 多文件 `import` + 项目管理（闭包编译、闭包哈希缓存、跨文件 LSP）+ 单元⑪ | `v0.57.0` 无 tag，见 §2.3；`ROADMAP.md:602-648` |
| 2026-09-18 | 0.58.0 | 项目状态视图（`query project` / `soko/project` / VS Code 项目树）+ 真 VS Code e2e 进 CI + 性能采样口径 | `v0.58.0`；`REQUIREMENTS.md:1297-1412` |
| 2026-09-19 | 0.59.0 | 语言线五刀（签名受检 / 构造子命名空间 / 派生 recursor 判据 / L1 prelude / 用户记法）+ 卷 I 上站点 + 课程与台账双门禁 | `v0.59.0`；`STATUS.md:134-270` |
| 2026-09-19 | 0.60.0 | 编辑器 `build`/`rebuild` + 五个缺口收口（`namespace`/清单 v2/`abbrev`/Type 层重写/累积性边界）+ 记法第二刀 | `v0.60.0`；`STATUS.md:69-132` |
| 2026-09-19 | 0.61.0 | 设计文档里「未做」的全部收口：记法第三刀、`namespace` 扩展、层级算术 + `Eq` 多态 + `cast`、编辑器词表、课程多清单聚合 | `v0.61.0`；`STATUS.md:23-67` |

> 上表 **39 行**是**精选**，不是全部。完整版本序列是 75 个 CHANGELOG 条目 / 69 个 tag；
> 想看每个 patch 的细节，直接读 `editor/vscode/CHANGELOG.md`（1336 行）。

### 2.3 关于版本与 tag 的三个硬事实（写页面时必须小心）

1. **69 个 tag，75 个 CHANGELOG 版本**。差的 6 个是
   `0.1.0 / 0.2.0 / 0.3.0 / 0.6.0 / 0.9.1 / 0.57.0`——
   命令见 §7.1 第 7 行与附脚本（比对 `git tag` 与 CHANGELOG 头）。
   **其中 0.57.0 是用户可见的大版本**（多文件 `import` + 项目管理），
   仓库里有它的 CHANGELOG 条目与 release commit（`0030b12`），但**本地没有 `v0.57.0` tag**。
   网站若要写「0.57.0 发布了什么」，请写功能，**不要**声称有对应 tag。
2. **19 个版本的 CHANGELOG 日期比 tag 提交日期早一天**（如 `0.10.0`：
   CHANGELOG 2026-09-10 / tag 提交 2026-09-11）。命令见 §7.1 第 8 行与附脚本。
   写时间线时**只选一种口径**，不要混。
3. **一天可以发很多版**：
   `for t in $(git tag); do git log -1 --format=%ad --date=short "$t"; done | sort | uniq -c`
   → 2026-09-15 一天 **24 个 tag**（0.29.0 → 0.48.0）；2026-09-13 有 6 个、09-16 有 6 个、
   09-19 有 3 个（0.59.0 / 0.60.0 / 0.61.0）。把「09-15 发了 0.29.0–0.48.0」写成一条，
   比铺开 24 行更诚实也更好读。

---

## 3. 愿景与原则

### 3.1 产品愿景（`REQUIREMENTS.md:7-11`，逐字）

> 用户与 code agent 共同看着同一个 `*.sokonanoda` 文件（共享画布）：
> agent 从零讲课、出题；用户作答；我们自己的编译器实时给出反馈，
> 同一份反馈既给用户也给 agent。反馈通道以 LSP 为中心（LSP-first）。

`ROADMAP.md:15-43`（§0 终极形态）把同一件事画成了图，并明确「万里长城第一步」：

> 用户与 code agent 共同看着 VS Code 中打开的同一个 `*.sokonanoda` 文件。
> agent 从零开始逐段定义概念、写出示例、抛出练习；
> 用户在文件里作答；
> 我们自己的编译器实时给出反馈——**同一份反馈既给用户看，也给 agent 看**，
> agent 据此决定下一步教什么、怎么调整题目。

> “万里长城第一步”：上面的 UI、agent、实时通道都先不做。
> **第一层编译器**先独立成立：它不依赖 VS Code、不依赖任何 agent，
> 但已经能解析 `.sokonanoda`、驱动完整 kernel、判定练习并输出结构化事件。

**给网站的翻译**（不是原文，是解释）：这个项目先造的是**判卷的那颗心脏**——
一个自己的 Lean 式内核 + 一套教学前端；编辑器、讲课 agent 都是后面接上去的。
今天接上去的程度见 §1.2。

### 3.2 十条不可动摇的硬规则（`REQUIREMENTS.md:13-36`）与它们为什么存在

| # | 规则（`REQUIREMENTS.md` 行号） | 为什么存在（依据） |
|---|---|---|
| 1 | kernel 保持完整，不因教学裁剪（`:15`） | 内核是判卷的唯一权威；裁剪 = 判出来的「对」不算数。冻结快照 + 热路径不动，是为了守住性能优势（§3 规则之外，见 `REQUIREMENTS.md:38-44`） |
| 2 | 无官方 Lean 工具依赖：`lean`/`lake`/`lean4export`/`leanc`/`elan` 一律不调用（`:16`） | 让 CI 离线可跑、教学内容可复现、不怕上游版本漂移（`docs/notes/research.md:69`）；opencode 用权限 deny、DSH 用 hooks 拦（`AGENTS.md` 硬规则 2） |
| 3 | 教学语法是真实 Lean 4 的子集，填完洞的声明放进官方 Lean 依然合法（`:17`） | 学生学到的东西不能作废；这是产品的兼容性承诺 |
| 4 | 语法白名单即课程：parser 只认课程引入过的语法点（`:18`） | 防止编译器悄悄长成「第二套 Lean」；新语法 = 课程 + 测试 + 白名单三件套 |
| 5 | 分层推进：L0 编译器 → L1 服务 → L2 编辑器 → L3 agent 协作（`:19`） | 每层只依赖下一层的公开接口，避免返工（`ROADMAP.md:79-88`） |
| 6 | TDD 与重复测试：front 单测 + CLI 端到端 + 课程语料三层覆盖（`:20`） | 「充分测试是大规模合作与多次大规模重构的重要资产」（用户原话，`REQUIREMENTS.md:133`） |
| 7 | 反馈即功能：类型/化简/打印/错误都结构化输出，人与模型都能无文档驱动（`:21`） | 同一份反馈要同时服务人和 agent（§3.1 愿景） |
| 8 | 判定永远走 kernel，不做文本比对（`:22`） | 文本比对会假绿/假红；`proof.rs::assumption` 的文本比对草案 2026-09-07 已删除 |
| 9 | 用户/agent 使用路径零工具链依赖（`:23-26`） | 用户原话把 `cargo run` 叫「重大失误」（`REQUIREMENTS.md:255`）；获取与运行只靠 Release 资产或 VSIX |
| 10 | 课程标准库三层分界（`:27-36`） | 判据只有一条：「Mathlib 有 ≠ 我们不该练；Mathlib 有且没有数学内容（纯定义展开）才归库」；违反的样子 = 让学习者在证明里手写 `Eq.symm`/`Or.elim`（「暴力」，用户原话见 §3.4） |

补充的两条工程标准（不在硬规则编号内，但同样是长期有效要求）：

- **内核性能是产品优势**（`REQUIREMENTS.md:38-44`）：任何重构/新功能都不得触碰
  `crates/kernel` 热路径；守护手段 = 内核全量测试 + 前端 perf 冒烟 + CI。
- **模块化：单文件不许越长越大**（`REQUIREMENTS.md:46-62`）：文件接近 ~500 行即拆分，
  公开 API 用 re-export 保持稳定。**当前仍有欠账**，见 §5.4——这条规则的价值恰恰
  在于它是**可以被违反并被测量**的。

### 3.3 用户的声音（`REQUIREMENTS.md` §9 逐字摘录）

网站的愿景页应当多用下面这些原话——它们比任何转述都更像人话。

**为什么做这个 / 要做到什么程度**

> 接手项目，先 I6 后推进至终极形态；用户要尽快在 playground.sokonanoda 开课，agent 当老师。
> —— `REQUIREMENTS.md:126-127`

> 增加一个 github pages，相当于当前项目的官网，充分介绍本项目的用法、远大目标和当前进展
> —— `REQUIREMENTS.md:410`

> 先好好调研、头脑风暴、规划，做好文档，然后再启动分步骤计划，多用 subagent 执行
> —— `REQUIREMENTS.md:411-412`

**课程与「暴力」**（这是整个卷 I 集合论课程的起点）

> 你的教程出的题目，在做的时候，你会发现要补充很多其他的定理，边边角角的定理，这个是在其他的教程里头会认为是天然应该知道的，或者说是标准库里已经实现的。这个我感觉有点暴力，所以我需要你去用这个项目，然后重新做一遍，然后才能把这些暴力给消除掉。你要记录下这些其实是需要实现的，其实是没有的。
> —— `REQUIREMENTS.md:1452-1455`

> 进硬规则，课程建在当前项目内新建一个文件夹
> —— `REQUIREMENTS.md:1482`

> 尽可能多的拆分任务，让子代理闭环实现并测试
> —— `REQUIREMENTS.md:1495`

**工具链与可用性**

> 用户路径零工具链依赖（用户纠正"cargo run 是重大失误"）
> —— `REQUIREMENTS.md:255`

> 单文件和项目文件编译器能自动区分吗？比如单文件不会找项目配置文件，能像脚本一样直接跑
> —— `REQUIREMENTS.md:1233-1234`

**教学法**

> 课程层只是给大模型提供路线图，具体执行层需要大模型适配各个用户、灵活调整
> —— `REQUIREMENTS.md:139-140`

> 从零教学场景确认（用户重申）：用户明确要在教学中关闭 prelude、自建 Eq/Nat。
> —— `REQUIREMENTS.md:282`

> 教学文案文风硬约束（用户指令）：拒绝翻译腔、AI 味、抖音味、小红书味
> —— `REQUIREMENTS.md:172-173`

> 跟 funapply 一样实现得稀里糊涂，不如直接删了
> —— `REQUIREMENTS.md:495-496`（对做得不好的功能，用户的处置是删掉而不是留着）

**质量与过程**

> 充分测试是大规模合作与多次大规模重构的重要资产
> —— `REQUIREMENTS.md:133`

> 性能是生命线（用户明确）
> —— `REQUIREMENTS.md:446`

> 各个环节的性能例行化检测并记录在案，方便后续分析检查。再多增加点项目相关的测试，功能和性能，包括 vscode 前端会不会卡，有没有实现不对的地方
> —— `REQUIREMENTS.md:1187-1189`

> 你配置相关套件，启动 VSCode 实际验证一下，本来就应该做成例行化检测。远程不行，本地例行化也可以接收
> —— `REQUIREMENTS.md:1323-1324`

> CI如果可以做 e2e 的测试那就太好了，多用多用
> —— `REQUIREMENTS.md:1340`

> 验证好就让 CI 往仓库追加吧
> —— `REQUIREMENTS.md:1351`

**细节也要对**（最能体现「不是营销」的一组）

> 326和327行有问题。编译器的信息应该是 sorry 没有用，而不是
> `declaration 'exists_intro_rule' uses 'sorry' (exercise not yet solved)`。sorry
> 去掉你试试，就能编译通过。差别很大，会让用户觉得没有证明出来。
> —— `REQUIREMENTS.md:1099-1101`

> hover 信息加一个按钮，直接替换 `intro`，跟 tab 补全一样，防止错过 tab 补全
> —— `REQUIREMENTS.md:392-393`

**最近三轮的原话**（现状页可以引用）

> 设计文档里的东西都做了吧。
> —— `REQUIREMENTS.md:1661`

> vscode 还是没有 sokonanoda: build 或者 sokonanoda: rebuild 的命令。你这个剩下的没做的也要做。
> —— `REQUIREMENTS.md:1673-1674`

> 实现工作单里描述的那个缺口
> —— `REQUIREMENTS.md:1518`（在 0.59.0 那一批缺口修复里反复出现的一句）

### 3.4 教学理念（网站上要讲清楚的几条）

1. **逻辑先行，宇宙由问题引出**（`REQUIREMENTS.md:90-97`）：
   不要一上来教 `Prop`/`Sort`/`Type`；先讲连接词与量词，让学生先「证明命题」；
   等函数类型出场、学生自然问「函数类型的类型是什么？」——由这个**自然触发的问题**
   引入 `Sort`。「教学顺序跟着直觉与问题触发走，而不是跟着类型论自身的知识结构走；
   产品设计同理：好的设计应该不言自明。」
2. **课程层 ≠ 执行层**（`REQUIREMENTS.md:77-81`）：`course/` 只是给大模型的
   路线图/素材库；真正面对用户的是动态画布（`playground.sokonanoda`），
   由 agent 按每个用户的错误历史、节奏、兴趣实时调整。「禁止把课程文件当成
   "用户直接消费的固定课程"。」
3. **三层分界**（硬规则 10，`REQUIREMENTS.md:27-36`）：L1 prelude（Lean core 级逻辑与
   等式骨架）/ L2 课程标准库（纯定义展开）/ L3 单元练习（一切有数学内容的陈述）。
4. **未完成是合法状态**：`sorry` 洞不导致整文件失败，练习开着不算错
   （`STATUS.md:19-21`；`docs/gaps/README.md:41-43` 明确「故意不看 `exercise_open` 计数」）。
5. **同一份反馈给人和 agent**：`--json` 事件流、`query` 单 JSON、LSP 三处同源
   （`REQUIREMENTS.md:21`、`ROADMAP.md:86`）。

---

## 4. 路线图

来源：`ROADMAP.md` §10、`docs/HANDOVER.md` §3/§4、`docs/gaps/ledger.jsonl`。
**「已确认要做」= 有编号、有验收标准、在仓库里被追踪**；
**「探索性」= 有设计但待拍板或明确 backlog**。不要把后者写成承诺。

### 4.1 已确认要做（按依赖顺序）

| # | 事项 | 是什么 | 对用户意味着什么 | 现状 | 承诺级别 | 证据 |
|---|---|---|---|---|---|---|
| 1 | I16 P7 长尾 | `watch` 项目模式（`sokonanoda watch` 对项目闭包而不是单文件） | 长驻服务能监听整个项目而不是一个文件 | 未开始 | 已确认（backlog，已登记） | `ROADMAP.md:639-641`、`docs/HANDOVER.md:321` |
| 2 | I16 P7：`[deps]` | 跨项目依赖声明 | 课程仓可以依赖语言仓的库，而不必 vendored | 未开始 | 已确认（backlog） | `ROADMAP.md:640-641`、`docs/design/imports-and-projects.md` P7 |
| 3 | H5-B1/B2：DSH Infoview + 诊断通道 | DSH 侧客户端插件 / 让内核诊断进 agent 视野 | 在 DeepSeek Harness 里也能像 VS Code 那样看到 goal 面板 | 未开始 | 已确认（backlog） | `docs/HANDOVER.md:263-264`、`docs/design/deepseek-harness.md:373` |
| 4 | H5-B3/B4：`SessionStart` provisioning + npm 插件包 | 会话启动自动准备环境；把启动器 + deny hook + 命令打成插件 | 新机器上零手工步骤 | 未开始 | 已确认（backlog） | `docs/HANDOVER.md:263-264` |
| 5 | 结构债清偿（大文件拆分） | 按 ~500 行惯例继续拆 `tests.rs`/`elab.rs`/`parser.rs`/`lib.rs`/`extension.js` | 不直接影响用户；影响接手成本 | 未开始（有排期） | 已确认 | `docs/HANDOVER.md:460-466`；实测行数见 §5.4 |
| 6 | 开练习签名的类型子表达式 hover | 让练习签名里的 `And`/`Nat` 悬停也有类型行 | 编辑体验一致性 | 刻意保留现状（改动需配回归） | 已确认但**刻意不做** | `docs/HANDOVER.md:467-472` |
| 7 | 课程内容继续长（卷 II+） | 卷 I 集合论 12 单元已完成；后续卷/单元按需 | 更多可学的数学 | 进行中（卷 I 已收口） | 已确认（方向） | `courses/set-theory/README.md`；`docs/design/set-theory-syllabus.md` |

### 4.2 探索性（有设计，未承诺）

| 事项 | 现状 | 卡在哪 | 证据 |
|---|---|---|---|
| WASM playground（浏览器内判题） | 明确 backlog，不阻塞上线 | 要把 `crates/kernel` + `crates/front` 编到 WASM，是独立大工程 | `docs/design/site.md:90-96`、`REQUIREMENTS.md:425` |
| L2/L3：多人协作、远程、compiler service 跨文件转播 / `setContent` | 远期 | v1 未做 | `docs/HANDOVER.md:449-451`、`ROADMAP.md:650-658` |
| Lean Kernel Arena 外部基准进 CI | 已立项但 **opt-in** | 不 vendor、不进 CI（避免引入官方 Lean 工具链） | `docs/HANDOVER.md:479-480`、`REQUIREMENTS.md:571-574` |
| spine meta 方案 B′（更深嵌套 / `def` 包裹的 whnf） | 未做 | 需内核/pp 暴露 whnf，违反内核冻结 | `docs/HANDOVER.md:473-474` |
| 独立课程仓（把 `courses/` 抽成子仓） | 设计保留，命令 `scripts/new-course-repo.sh` 可用 | 已决定先建在当前仓内（D-1a → 硬规则 10 同轮） | `REQUIREMENTS.md:1427-1430`、`REQUIREMENTS.md:1486-1488` |
| 已检查声明的跨进程复用 | 设计里列为「先量收益再开」 | 收益未量 | `ROADMAP.md:618-619` |

### 4.3 明确不做（有理由，不是「还没做」）

| 项 | 理由 | 证据 |
|---|---|---|
| canonical formatter | 与洞/span 稳定性冲突 | `docs/notes/gap-analysis.md:49-53` |
| call/type hierarchy、selectionRange、pull diagnostics、salsa 查询框架 | 已有等价能力或不需要 | 同上 |
| 浏览器/WASM 编辑器、成就系统、LSIF/SCIP | 不在产品路径上 | 同上 |
| `section` / `variable` | **做不动**（有实测）：本语言应用是逐位显式的，没有隐式参数插入 | `docs/design/namespace-open.md:238,359-364`、`STATUS.md:39-40` |
| 源码级 print-back（记法回写源码） | 类型文本由内核 pp 产出，记法不进内核；内核冻结 ⇒ 销不掉 | `STATUS.md:52-54`、`docs/design/notation-subset.md:13` |
| 累积性（cumulativity）、Prop 大消去 | 内核性质，冻结内核下不改 | `STATUS.md:52-54`、`docs/design/prop-cumulativity-boundary.md` |
| `u+v` / `max` 层级算术 | 内核 `Level::Max` 无公开构造入口 | `STATUS.md:41-45` |
| typeclass / macro | 不在白名单 | `ROADMAP.md:145` |
| 互递归 / 嵌套递归归纳、宇宙多态参数 | v1 不做 | `docs/HANDOVER.md:475` |
| `as`/or 模式、多 scrutinee、`if/then/else` 表达式 | v1 不做 | `docs/HANDOVER.md:177-187` |
| 嵌套 `intro`（`fun … => intro`） | 那里的 `intro` 是普通标识符 | `REQUIREMENTS.md:404-405` |

---

## 5. 诚实的未完成清单

**这一节是给网站直接用的**：用户明确要求「内容为王」，那么把不能做的事写清楚，
比写十个形容词更能建立信任。每条都给出处。

### 5.1 语言与内核边界

| 限制 | 具体表现 | 为什么 | 证据 |
|---|---|---|---|
| **内核冻结** | `crates/kernel/**` 一个字节不改；bugfix 必须带三层回归测试 | 内核是判卷权威，也是项目定位里性能优势的载体（`REQUIREMENTS.md:40` 的「10–100x」是**目标表述**，本仓没有实测基准，见 §7.2） | `REQUIREMENTS.md:15,38-44`；`STATUS.md:57`（0.61.0 内核零改动） |
| **没有累积性（cumulativity）** | `def T : Type := True` 被拒（`kernel-prop-not-cumulative` + 人话 hint） | 内核性质；官方 Lean 4 有，本语言没有 | `STATUS.md:106-110`、台账 `L-06`（`workaround`） |
| **`Exists.elim` 的 `Q` 只能是 `Prop`** | 取数据的引理写不出来；等势只能 `Prop` 值 | 同上；课程统一改数据版 | 台账 `L-06`；`docs/HANDOVER.md:371` |
| **Prop 大消去不可用** | 单构造子 `Prop` 的 motive 只能落 `Prop` | 内核性质，「是正确的行为」 | `docs/HANDOVER.md:371`、`docs/design/prop-large-elim-mirror.md` |
| **无隐式参数插入** | `#check id Nat` ⇒ `Nat -> Nat`；`def u : Nat := id Nat` 被内核拒绝 | 应用是逐位显式的 | `docs/design/namespace-open.md:364`、`STATUS.md:39-40` |
| **无 `section` / `variable`** | 不能写 Lean 的 auto-bound | 做不动（需隐式参数插入） | `docs/design/namespace-open.md:359-364` |
| **无 typeclass / macro** | 不在语法白名单 | 课程未到 | `ROADMAP.md:145` |
| **记法没有源码级 print-back** | goal/hover 的类型文本仍由内核 pp 产出，不显示用户定义的记法 | 内核 pp 冻结 | `STATUS.md:52-54`、`docs/design/notation-subset.md:13,206` |
| **`u+v` / `max` 层级算术不可写** | 只支持 `u+1` 数字后缀加法 | 内核 `Level::Max` 无公开构造入口 | `STATUS.md:41-45` |
| **裸构造子名是「扩展」不是 Lean 语义** | `ctor mk` 的规范名是 `Ind.mk`；裸名是解析别名（唯一时可用，撞名报 `elab-ambiguous-ctor-alias`） | 真 Lean 里裸 `mk` 不可解析 | `docs/HANDOVER.md:351-359`；台账 `G-02` |
| **`match` 的边界** | 无 `as`/or 模式、无多 scrutinee、无 `if/then/else` 表达式 | v1 范围 | `docs/HANDOVER.md:177-187` |
| **参数化归纳的边界** | 无宇宙多态参数、无互/嵌套递归 | v1 范围 | `docs/HANDOVER.md:475` |
| **prelude `Nat` 经 `Nat.rec` 归约的显示** | 结果可能是不合并的一元链（与 numeral def-eq） | 已文档化并钉测试 | `docs/HANDOVER.md:477-478` |
| **源内 `inductive Nat` 时 `#reduce` 显示混合表示** | `Nat.succ (Nat.succ (Nat.succ 1))` | 命中内核 name cache 快路径；逐条实测重钉 | `docs/HANDOVER.md:357-359` |

### 5.2 工具与平台边界

| 限制 | 具体表现 | 证据 |
|---|---|---|
| **没有 WASM / 浏览器构建** | 浏览器内判题要把 kernel + front 编到 WASM，是独立大工程；一期用零后端方案（静态站 + 装扩展）替代 | `docs/design/site.md:90-96` |
| **启动器缓存过期即拒绝运行** | 缓存 marker 与版本钉不一致 ⇒ 拒绝执行并给人话；这是历史上最常见的故障源 | `AGENTS.md`；台账 `G-11`/`G-16` |
| **版本钉解析不出就绝不 exec** | exit 3 + 指名文件报错（多源不一致也报错） | `REQUIREMENTS.md:1553-1566` |
| **禁止 `releases/latest`** | 下载一律按仓库版本锁定 | `AGENTS.md`；`REQUIREMENTS.md:23-26` |
| **本地 `gate` 可能 exit 3** | 探不到 python3 ⇒ exit 3（「无法判定 ≠ 绿」）；二进制版本与仓库不一致也 exit 3 | `AGENTS.md`；`STATUS.md:65-67` |
| **平台矩阵有限** | 8 个 CLI/LSP tarball + 9 个 VSIX（含 alpine / win32-arm64）；不含 linux-armhf | `REQUIREMENTS.md:248-254`、`STATUS.md:126-127` |
| **发布仍有外部依赖** | Marketplace 的 Azure gallery 超时反复出现（台账里 10 条与 marketplace/gallery 有关） | `docs/CI-FAILURES.md`（见 §6.3） |

### 5.3 编辑器 / harness 能力边界

**同一个 LSP，在不同 harness 里能用的东西不一样**——网站若要写「支持 XX 编辑器」，
必须按这张表写：

| harness | 能用的 | 不能用的 | 证据 |
|---|---|---|---|
| **VS Code** | 全量：诊断、hover、goal 面板（webview Infoview）、练习/项目/课程三棵树、codeLens、code action、inlay、跨文件跳转引用改名、`build`/`rebuild` 命令 | — | `docs/HANDOVER.md:317-320`；`STATUS.md:75-84` |
| **opencode** | 启动插件自动接线 LSP；`/sokonanoda/*` 命令；`teacher` 主 agent；skills 自动加载；Lean 工具链命令 deny | — | `AGENTS.md`；`REQUIREMENTS.md:229-237` |
| **DeepSeek Harness** | 技能目录自动发现（技能名即 `/sokonanoda-teacher` 等命令）；两个**人工**运维命令 `/sokonanoda-update`、`/sokonanoda-doctor`；`dsh web --patch ./dsh/cordis.patch.yml` 后 LSP 可 hover/跳定义；MCP 六工具（默认关闭，需 opt-in） | **LSP 只有 4 项只读操作**（definition/references/implementation/hover）；**服务端诊断被显式丢弃**；`workspace/applyEdit` 被拒；`soko/*` 无消费者；**项目/家目录 `.env` 不能设 PATH、没有项目级钩子** | `docs/design/deepseek-harness.md:59,182,289`；`docs/HANDOVER.md:258-262`；`AGENTS.md` |
| **其他 harness** | `.opencode/lsp/sokonanoda-lsp.sh` shim → `scripts/soko lsp` | 无命令/技能发现 | `AGENTS.md` |

另有一条**所有 harness 共有**的 LSP 限制：

- **`soko/nextHole` 无法在同一源码位置的多子目标之间导航**（同址子目标只能整组导航），
  已记入 `docs/protocol.md` 的 `soko/nextHole` 小节（`ROADMAP.md:480-482`）。
- **DSH 侧没有 `soko/*` 消费者**，所以判卷一律走 CLI `--json`
  （`AGENTS.md`；`docs/design/deepseek-harness.md:289`）。

### 5.4 工程债（实测行数，2026-09-19）

命令（逐文件，输出即上表数字）：

```bash
for f in crates/front/src/compile/tests.rs crates/front/src/parser.rs \
         crates/front/src/compile/elab.rs editor/vscode/extension.js \
         crates/lsp/src/lib.rs crates/front/src/compile/check/walk.rs \
         crates/front/src/compile/check/mod.rs \
         crates/front/src/compile/check/kernel_phase.rs; do wc -l < "$f"; done
```

（`find … | xargs wc -l` 也能跑，但会连 `editor/vscode/node_modules/` 一起统计，
必须自己过滤。）`REQUIREMENTS.md:46-62` 定的惯例是「接近 ~500 行即考虑拆分」。

| 文件 | 行数（实测） | 登记时的行数（`docs/HANDOVER.md:460-466`，2026-09-18） |
|---|---|---|
| `crates/front/src/compile/tests.rs` | **7926** | 4828 |
| `crates/front/src/parser.rs` | **4702** | 2065 |
| `crates/front/src/compile/elab.rs` | **4130** | 2854 |
| `editor/vscode/extension.js` | **1880** | 1493 |
| `crates/lsp/src/lib.rs` | **1610** | 1554 |
| `crates/front/src/compile/check/walk.rs` | **1273** | 951（0.57.0 拆分后） |
| `crates/front/src/compile/check/mod.rs` | **832** | 791 |
| `crates/front/src/compile/check/kernel_phase.rs` | **527** | 413 |

**诚实结论**：`run_pass` 那个 ≈1174 行的单函数确实拆掉了（这是 2026-09-18 的成果，
验收手段是**二进制对拍**：改动前后两个 CLI 跑全部 58 个 `.sokonanoda` 文件，
stdout 逐字节相同——`docs/HANDOVER.md:509-511`）；但**语言在长，几个大文件比登记时更大了**。
这是公开可查的欠账，不是秘密。

### 5.5 其他已知限制（都在仓库里有登记）

- **perf 套件是哨兵不是基准**：外部基准（Lean Kernel Arena）opt-in
  （`docs/HANDOVER.md:479-480`）。
- **本地 perf 哨兵在机器重载时会误报**（`incremental_edit_anywhere_is_fast` 受 CPU 争用）；
  单跑通过即环境问题，重跑 `gate`（`docs/HANDOVER.md:554-555`）。
- **编辑器 LSP 与发布版本可能不一致**：VS Code 扩展默认强制内置 LSP；
  `sokonanoda version` 报的是**下载缓存**，不是正在跑的服务器
  （`docs/HANDOVER.md:537-548`）。
- **Xcode 许可未接受会让 cargo 直接构建失败**（本机实测；绕过用
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools`）——`docs/HANDOVER.md:556-559`。
  **本次写这份卷宗时就撞上了**，见 §7.3。
- **`?`/`???` 不是当前语法**：练习洞写 `sorry`（`STATUS.md:19-21`）；
  `ROADMAP.md:104` 里 `???` 是早期草案，不要写进网站。

---

## 6. 质量与过程证据

这一节是**差异化卖点**：不要写「我们很严谨」，写下面这些能被别人跑一遍的东西。

### 6.1 CI 门禁（`.github/workflows/ci.yml`）

`scripts/soko gate` 一条命令 = CI 门禁的全部（`AGENTS.md`）：

```bash
scripts/soko gate
# = cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
# + cargo clippy --workspace --all-targets
# + cargo test --workspace --locked
# + playground 锚点（用运行中二进制的内嵌编译器）
# + 课程门禁  python3 courses/set-theory/tools/check.py
# + 缺口台账门禁 python3 scripts/gap.py selftest && python3 scripts/gap.py check
# 探不到 python3 ⇒ exit 3（绝不静默跳过）
```

三个 workflow 文件：`ci.yml`（lint / auto-tag / test）、`release.yml`（build / package-vsix /
github-release / marketplace-publish）、`pages.yml`（站点部署）。
`ci.yml` 的 `test` job 里挂了课程门禁与台账门禁两个 step（**不新建 job**，
课程红自动挡住 `auto-tag` 的发布）——`STATUS.md:172-179`、`docs/HANDOVER.md:394-405`。

**禁止 `cargo fmt --all`**：会重排冻结内核；只 fmt 教学 crates（`AGENTS.md`；
教训见 `docs/LESSONS.md:169-177`）。

### 6.2 测试（本机实测）

```bash
# 全量：exit 0（本机需要 Xcode 绕过，见 §7.3）
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --workspace --locked

# 计数
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --workspace --locked -- --list | grep -c ": test"
# → 1169
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --workspace --locked -- --list --ignored | grep -c ": test"
# → 6
# ⇒ 1163 个可运行测试，与 STATUS.md:55 记录的 "1163 passed / 0 failed" 一致
```

| 包 | 测试数（`--list`） | 说明 |
|---|---|---|
| `sokonanoda-front` | **662** | 前端/elaborator/判定/query |
| `sokonanoda-cli` | **308** | 21 个集成测试文件（含 `launcher.rs` 真跑 Node 的行为契约） |
| `sokonanoda-lsp` | **141** | 协议与编辑器行为 |
| `sokonanoda`（kernel） | **58**（其中 **6** 个 `#[ignore]`，需重建 NDJSON fixture） | 冻结快照的全量测试 |
| **合计** | **1169（1163 可运行）** | `ls crates/cli/tests/` → 21 个文件 |

命令：`cargo test -p sokonanoda-front --locked -- --list | grep -c ": test"`（其余包同理；
**注意 kernel 的包名是 `sokonanoda`，不是 `sokonanoda-kernel`**，后者会报
`package ID specification did not match any packages`）。

测试分层的**纪律**（网站上值得写）：

- 每个语法点走 **TDD 三件套**：front 单测 + CLI e2e + 课程用例（`ROADMAP.md:391`）。
- **删除旧实现前先做「全输入对拍」**：H6-A 抽真相层时在 5 个画布的每一个光标 offset 上
  对拍新旧实现（709 次比较、424 处不一致全在同一分支）——`docs/LESSONS.md:270-289`。
- **位置搬移用二进制对拍验收**，不要只信测试全绿（`docs/LESSONS.md:369-389`）。
- **测试里别写死「当前版本」**（0.57.0 bump 当天变红，`docs/LESSONS.md:353-367`）。
- 真实 VS Code 集成测试：`SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh`，
  结果进 `docs/e2e/ledger.jsonl`（手册 `docs/E2E.md`）。

### 6.3 四本台账（这是「过程可审计」的实体）

| 台账 | 规模（实测） | 记什么 | 门禁 |
|---|---|---|---|
| `docs/gaps/ledger.jsonl` | **24 条**（22 `fixed` + 2 `workaround`；blocker 12 / painful 10 / nice 2；kind：language 10 / tooling 7 / library 6 / infra 1） | 每条缺口带**最小复现**、`status`、`fixed_in` | `python3 scripts/gap.py check` → **exit 0**；`selftest` **14 条判据** |
| `docs/gaps/WO-*.md` | **11 张工作单** | 每张工作单 = 一个可交给另一个 agent 的闭环任务 | 同上 |
| `docs/gaps/repro/` | **22 个复现件**（`.sokonanoda` + 自断言 `.sh`） | 复现脚本退出码 0/1/2 有约定：0 = 缺口仍在、1 = 行为已变、2 = 环境缺失 | `docs/gaps/README.md:31-43` |
| `docs/CI-FAILURES.md` | **35 条**（34 条 dated + 1 条格式模板） | 每次 CI 红：原因 / 修复 / 预防 | `AGENTS.md`：同一类失败不犯第二次 |
| `docs/LESSONS.md` | **34 个 `##` 小节**（6 个分类头 + 28 条带日期的教训） | 做错了什么、学到什么 | 无自动门禁（人读） |
| `docs/perf/ledger.jsonl` | **14 条记录** | 每条带 version / commit / dirty / host / 分阶段毫秒 | `scripts/perf-ledger.sh` |
| `docs/e2e/ledger.jsonl` | **37 条记录** | 真 VS Code 集成测试：版本 / commit / dirty / 宿主 / VS Code 版本 / 用例数 / exit / 服务器自述 / LSP sha256 / 日志路径 | CI 的 `e2e-ledger` job 自动回提交 |
| `docs/courses/ledger.jsonl` | **1 条** | 课程门禁成本台账（`--ledger` 默认关，避免 CI 写仓库） | `STATUS.md:50-51` |

**CI 失败台账的特征**（`grep -cE "^#{2,3} .*(marketplace|gallery|vsce)" docs/CI-FAILURES.md` → **10 条**
与 Marketplace 发布相关；`grep -icE "clippy"` → 9；`grep -icE "fmt"` → 11；
`grep -c "哨兵"` → 10）。三类典型：

1. **外部服务抖动**：Marketplace Azure gallery 超时反复出现（台账里明确写「已知类，复发」
   「第 4 次复发」）——处置是探活 + `gh run rerun --failed`，不是改代码。
2. **本地假绿**：`clippy int_plus_one`（`docs/CI-FAILURES.md:128-139`）、
   新测试没跑 fmt（`:327-336`）。
3. **性能哨兵假红**：2026-09-18 front 缩放哨兵、2026-09-19 LSP 项目哨兵
   （`:337-413`，三条记录）——修法是改**采样口径**（串行 + best-of-N），**阈值不动**；
   并且台账里明确写「无产品回归」的证据。

### 6.4 发布自动化与产物

```bash
# 发布（人只做两件事：bump 两处版本 → push main）
#   Cargo.toml:6  version = "0.61.0"
#   editor/vscode/package.json:5  "version": "0.61.0",
# 之后：ci.yml auto-tag 打 tag → dispatch release.yml
```

| 事实 | 值 | 证据 |
|---|---|---|
| Release 资产数 | **26**（lsp ×8 / cli ×8 / vsix ×9 / `SHA256SUMS`） | `STATUS.md:59-60`、`docs/HANDOVER.md:552-553` |
| 平台矩阵 | CLI/LSP 各 8 个 tarball；VSIX 9 个（per-target + universal 回退） | `REQUIREMENTS.md:248-254` |
| 供应链 | 每个 Release 附 `SHA256SUMS` + SLSA provenance | `REQUIREMENTS.md:602-607` |
| 版本一致性 | 两处版本号由契约测试守着：`crates/cli/tests/extension.rs::cargo_and_extension_versions_match` | `STATUS.md:142-144` |
| 产物实测 | 每次发布后下载产物 → `shasum -c` → `--version` → 跑一段新语法文件 | `STATUS.md:60-64`、`STATUS.md:245-255` |
| 回提交 | CI 的 `e2e-ledger` job 把各腿台账合并成**一条**提交推回仓库（幂等、冲突可诊断） | `REQUIREMENTS.md:1351-1358` |

### 6.5 过程纪律（可验证的规则，不是口号）

| 规则 | 落在哪 | 怎么被强制 |
|---|---|---|
| 每轮落 commit 前更新 `STATUS.md`（只留最近 3 轮，旧轮归档） | `AGENTS.md` 收尾义务 | 网站进度页自动读最新轮标题（`scripts/gen-site-data.py`） |
| 用户新要求追加进 `REQUIREMENTS.md` §9 并注明日期 | `REQUIREMENTS.md:5` | 现有 **123 条** dated 记录（`grep -c "^- 2026-"`） |
| 每次 CI 红了写一条 `docs/CI-FAILURES.md` | `AGENTS.md` | 35 条记录 |
| 每条缺口带最小复现，且复现与 `status` 必须一致 | `docs/gaps/README.md:31-63` | `scripts/gap.py check` 进 `gate` 与 CI |
| 用户可见改动**同一轮**更新 VS Code 扩展 + `skills/` + `AGENTS.md` | `REQUIREMENTS.md:796-802` | `crates/cli/tests/skill.rs`、`crates/cli/tests/extension.rs` |
| 设计先行：新功能先写 `docs/design/` 再动手 | `AGENTS.md` | `docs/design/` 现有 **60 篇**（`ls docs/design/*.md \| wc -l`） |
| 门禁注入的环境变量不得短路复现夹具 | `docs/LESSONS.md:537-556` | `gap.py` 的 `clean_env()` + `selftest` 钉住 |
| 判定只认 `decl.checked` 与 `diagnostic`，**不拿 `exercise.open` 当「签名没坏」的证据** | `AGENTS.md` 硬规则 3；`docs/gaps/README.md:85-89` | 台账 G-01 的复现期望是 `rejected` |

---

## 7. 数字总表

**规则**：网站只能使用「可复现」列里标 ✅ 的数字；标 ⚠️ 的必须带上限定语；
标 ❌ 的**不要用**。

### 7.1 数字表

| # | 值 | 含义 | 确切命令 | 来源文件 | 可复现 |
|---|---|---|---|---|---|
| 1 | `0.61.0` | 当前仓库/扩展版本 | `grep -n "^version" Cargo.toml` | `Cargo.toml:6` | ✅ |
| 2 | `0.61.0` | 扩展版本（与上者必须一致） | `grep -n '"version"' editor/vscode/package.json` | `editor/vscode/package.json:5` | ✅ |
| 3 | `0.61` | 卷 I 课程的版本约束 | `grep -n requires courses/set-theory/sokonanoda.toml` | `courses/set-theory/sokonanoda.toml:4` | ✅ |
| 4 | `108` | 开发轮次数 | `cat STATUS.md docs/STATUS-ARCHIVE.md \| grep -c "^## 本轮进度"` | `STATUS.md` + `docs/STATUS-ARCHIVE.md` | ✅ |
| 5 | `69` | git tag 数 | `git tag \| wc -l` | git | ✅ |
| 6 | `75` | CHANGELOG 版本条目数（0.1.0→0.61.0） | `grep -c "^## \[" editor/vscode/CHANGELOG.md` | `editor/vscode/CHANGELOG.md` | ✅ |
| 7 | `6` | 有 CHANGELOG 条目但**没有 tag** 的版本数 | §7.1 附脚本 | git + CHANGELOG | ✅ |
| 8 | `19` | CHANGELOG 日期与 tag 提交日期差一天的版本数 | §7.1 附脚本 | git + CHANGELOG | ✅ |
| 9 | `367` | 提交数 | `git log --oneline \| wc -l` | git | ✅ |
| 10 | `2026-09-06` | 首个提交日期 | `git log --reverse --format=%ad --date=short \| head -1` | git | ✅ |
| 11 | `2026-09-19` | 最新提交日期 | `git log -1 --format=%ad --date=short` | git | ✅ |
| 12 | `1169` | 测试函数总数（含 6 个 ignored） | `DEVELOPER_DIR=… cargo test --workspace --locked -- --list \| grep -c ": test"` | 本机实测 | ✅ |
| 13 | `6` | `#[ignore]` 测试数 | 同上加 `--ignored` | 本机实测 | ✅ |
| 14 | `1163` | 可运行（通过）测试数 | 1169 − 6；与 `STATUS.md:55` 的 `1163 passed / 0 failed` 一致 | `STATUS.md:55` | ✅ |
| 15 | `662 / 308 / 141 / 58` | front / cli / lsp / kernel 测试数 | `cargo test -p <pkg> --locked -- --list \| grep -c ": test"`（kernel 包名是 `sokonanoda`） | 本机实测 | ✅ |
| 16 | `36` | 课程门禁目标数（卷 I） | `python3 courses/set-theory/tools/check.py --json` → `targets` | `courses/set-theory/tools/check.py` | ✅ |
| 17 | `329` | 卷 I `checked` 声明数（含 lib + 解答 + 对照页） | 同上 → 各 target `checked` 求和 | 同上 | ✅ |
| 18 | `99` | 卷 I `open` 练习数 | 同上 → 各 target `open` 求和 | 同上 | ✅ |
| 19 | `0` | 卷 I 判负数 | 同上 → `failed` | 同上 | ✅ |
| 20 | `12 / 4 / 1` | 卷 I 单元数 / 章数 / 卷数 | `node scripts/soko course "$PWD/courses/set-theory/course.json" --json \| grep course.summary` | `courses/set-theory/course.json` | ✅ |
| 21 | `65 / 96 / 0` | 卷 I 入口模块聚合：checked / open / failed | 同上 | 同上 | ✅ |
| 22 | `11` | 入门课单元数 | `node scripts/soko course "$PWD/course/course.json" --json \| grep course.summary` | `course/course.json` | ✅ |
| 23 | `86 / 65 / 0` | 入门课聚合：checked / open / failed | 同上 | 同上 | ✅ |
| 24 | `30 / 2 / 4 / 2` | `playground.sokonanoda`：`decl.checked` / `example.checked` / `exercise.open` / `warning` | `target/debug/sokonanoda --json playground.sokonanoda`（按 `type` 计数） | `playground.sokonanoda` | ✅ |
| 25 | `47` | Full 模式 prelude 顶层名字数 | 见 §7.1 附脚本（正则读 `PRELUDE_NAMES`） | `crates/front/src/compile/prelude.rs` | ✅ |
| 26 | `24` | 缺口台账条数 | `python3 scripts/gap.py list \| tail -1` | `docs/gaps/ledger.jsonl` | ✅ |
| 27 | `22 / 2 / 0` | 缺口状态：fixed / workaround / open | `python3 scripts/gap.py list` | 同上 | ✅ |
| 28 | `12 / 10 / 2` | 缺口级别：blocker / painful / nice | 同上 | 同上 | ✅ |
| 29 | `10 / 7 / 6 / 1` | 缺口种类：language / tooling / library / infra | 同上 | 同上 | ✅ |
| 30 | `18 / 4` | `fixed_in`：0.59.0 / 0.60.0 | `python3 -c` 统计 ledger（见 §7.1 附脚本） | `docs/gaps/ledger.jsonl` | ✅ |
| 31 | `11` | WO 工作单数 | `ls docs/gaps/WO-*.md \| wc -l` | `docs/gaps/` | ✅ |
| 32 | `22` | 复现件数 | `ls docs/gaps/repro \| wc -l` | `docs/gaps/repro/` | ✅ |
| 33 | `35` | CI 失败台账条数 | `grep -cE "^#{2,3} [0-9]{4}-[0-9]{2}-[0-9]{2}" docs/CI-FAILURES.md` | `docs/CI-FAILURES.md` | ✅ |
| 34 | `34` | LESSONS 小节数（含 6 个分类头） | `grep -c "^## " docs/LESSONS.md` | `docs/LESSONS.md` | ✅ |
| 35 | `123` | `REQUIREMENTS.md` §9 的 dated 记录数 | `grep -c "^- 2026-" REQUIREMENTS.md` | `REQUIREMENTS.md` | ✅ |
| 36 | `60` | 设计文档数 | `ls docs/design/*.md \| wc -l` | `docs/design/` | ✅ |
| 37 | `109` | `.sokonanoda` 文件总数 | `find . -name "*.sokonanoda" -not -path "./target/*" \| wc -l` | 仓库 | ✅ |
| 38 | `14` | 性能台账记录数 | `wc -l < docs/perf/ledger.jsonl` | `docs/perf/ledger.jsonl` | ✅ |
| 39 | `37` | e2e 台账记录数 | `wc -l < docs/e2e/ledger.jsonl` | `docs/e2e/ledger.jsonl` | ✅ |
| 40 | `15 / 15` | 最新一次真 VS Code 集成测试（0.61.0，VS Code 1.138.0） | `tail -1 docs/e2e/ledger.jsonl` → `tests.passed=15, failed=0` | `docs/e2e/ledger.jsonl` | ✅ |
| 41 | `26` | 每个 Release 的资产数 | 发布记录 | `STATUS.md:59-60` | ⚠️（历史事实，需 GitHub Release 页核对） |
| 42 | `35.26 ms` | 0.59.0 项目按键重编译 best（front `keystroke_recompile_closure`，4 模块 × 20 声明） | `tail -1 docs/perf/ledger.jsonl` | `docs/perf/ledger.jsonl` | ⚠️（0.59.0 基线；跨口径不可比，见 `docs/PERF.md`） |
| 43 | `15174 / 38061 / 15872 / 8712` | kernel / front / cli / lsp 的 `.rs` 总行数 | `find crates/<x> -name "*.rs" \| xargs wc -l \| tail -1` | 仓库 | ✅ |
| 44 | `10` | 站点页面数（卫生检查口径） | `python3 scripts/check-site.py` | `site/` | ✅ |
| 45 | `108` | 网站数据里的最新轮次 | `python3 -c "import json;print(json.load(open('site/data/site.json'))['round'])"` | `site/data/site.json` | ✅ |

**§7.1 附脚本**（第 7/8/25/30 行用到的命令，一次性跑完）：

```bash
python3 - <<'EOF'
import json, re, subprocess
# 第 7/8 行：tag 与 CHANGELOG 的差集 / 日期差
txt = open('editor/vscode/CHANGELOG.md', encoding='utf-8').read()
cl = dict(re.findall(r'^## \[([0-9.]+)\] - (\d{4}-\d{2}-\d{2})', txt, re.M))
tags = subprocess.run(['git', 'tag'], capture_output=True, text=True).stdout.split()
have = {t.lstrip('v') for t in tags}
print('无 tag 的版本:', sorted(set(cl) - have))
diff = [(v, cl[v], subprocess.run(['git','log','-1','--format=%ad','--date=short','v'+v],
        capture_output=True, text=True).stdout.strip()) for v in have if v in cl]
print('日期差一天:', len([d for d in diff if d[1] != d[2]]))
# 第 25 行：PRELUDE_NAMES 长度
p = 'crates/front/src/compile/prelude.rs'
t = open(p, encoding='utf-8').read()
m = re.search(r'PRELUDE_NAMES[^=]*=\s*&?\[(.*?)\];', t, re.S)
print('PRELUDE_NAMES:', len(re.findall(r'"([^"]+)"', m.group(1))))
# 第 30 行：fixed_in 分布
rows = [json.loads(l) for l in open('docs/gaps/ledger.jsonl') if l.strip()]
from collections import Counter
print('fixed_in:', Counter(r.get('fixed_in') for r in rows))
EOF
```

### 7.2 未证实 / 不要写进网站的数字

| 数字 | 为什么不能用 |
|---|---|
| 「10–100x 快于官方内核」 | `REQUIREMENTS.md:40` 是**用户要求里的定性表述**，不是本仓库的基准结果；外部基准（Lean Kernel Arena）仍是 opt-in（`docs/HANDOVER.md:479-480`）。网站若要用，必须写成「项目目标/定位」，不能写成实测结论 |
| 「用户数 / 下载量 / 教学效果」 | 仓库里没有任何来源；一条都没有 |
| 「课程有 XX 道题」以外的题量口径 | 现有可复现口径是门禁的 `open`（卷 I 99 / 入门课 65）。`docs/design/teaching-project.md` 里的历史数字（如 308/93、355/99）是**当时快照**，会随课程生长变化（`STATUS.md:189-191` 明确「课程在长，所以门禁只判形状、不锁计数」） |
| 「0.57.0 的 tag」 | 本地无 `v0.57.0` tag（§2.3） |
| `ROADMAP.md:405` 的 `units=10 checked=78 open=59 failed=0` | 这是 I7 完成时的**历史 golden**；现在入门课是 11 单元、`86 checked / 65 open`（第 22/23 行实测） |
| `STATUS.md:189-191` 的 `355 checked` | 0.59.0 当时的课程门禁数；现在是 **329**（P4 把 `lib/Logic` 26 条退化成空壳，checked 相应减少，`STATUS.md:236-238`） |

### 7.3 复现环境（重要，否则数字对不上）

- 本机：macOS / arm64；`python3 --version` → **Python 3.14.7**；`node --version` → **v24.20.0**。
- **cargo 链接失败**：本机 Xcode 许可未接受（`cc` exit 69）。这是仓库已文档化的坑
  （`docs/HANDOVER.md:556-559`）。绕过：
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --workspace --locked`。
  网站上若展示「一条命令跑测试」，**不要**把 `DEVELOPER_DIR` 写进去（那是本机环境问题），
  但本卷宗里的测试数字都是带这个变量跑出来的。
- **课程门禁要指定二进制**：`SOKONANODA_BIN="$PWD/target/debug/sokonanoda" python3 courses/set-theory/tools/check.py --json`
  （不带时门禁自己解析版本钉；`scripts/soko gate` 会把解析到的二进制透传）。
- **工作树**：`git status --short` 应为空才算「干净工作树」；性能台账的 `dirty` 字段
  就是记这个（`docs/perf/ledger.jsonl` 最新一条 `dirty: false`）。
- **本卷宗写作时未改动任何被计数的文件**：只新建了本文件。

---

## 附：给写页面的人的三条提醒

1. **进度页**用 §2.2 的时间线 + §1.2 的能力表 + §6 的证据；**不要**把 §4 的计划
   混进「已完成」。已发布 / 进行中 / 未开始，这三档是这份文件的核心价值。
2. **愿景页**用 §3.1 的原文 + §3.3 的用户原话 + §3.4 的教学理念；
   硬规则要配「为什么存在」（§3.2 的右列），否则十条规则读起来像教条。
3. **诚实页（limits）**直接用 §5，一条都别删。这份项目最稀缺的内容不是「我们能做什么」，
   而是「我们明确不做什么、以及为什么」——`section`/`variable` 的「做不动」有实测，
   print-back 的「销不掉」有内核冻结的理由，WASM 的「没做」有工程量的估算。
   这些才是「内容为王」里那个「内容」。
