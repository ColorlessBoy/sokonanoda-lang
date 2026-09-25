# E2 计划：以"每一阶段都能发一个完整可用的版本"为硬约束

> 取代 E1 的线性清单（`vscode-editor-feedback-plan.md` §13，124/124 已勾完 ✓）。
> **E1 的结论没有被丢掉** —— 它把"哪些路走不通"实测清楚了（见 §5），E2 从那些
> 结论**往上走** ✓。
>
> **接手的人先读 `docs/E2-HANDOVER.md`** ✓（一页交接书：状态 / 开工方式 /
> 硬规则 / 陷阱 / 验收命令 ✓）。
>
> **用户定的三条硬约束（2026-09-24）**：
> 1. **每个阶段都能推送一个完整可用的东西** ✓ —— 不允许"半成品停在树上" ✗；
> 2. **性能只升不降** ✓ —— 任何阶段若让某条路径变慢 ✗，必须**关在默认关闭的开关后** ✗；
> 3. **尽可能多步骤** ✓ —— 一步一验收，好把控进度 ✓。

---

## 0. 三条硬约束怎么落地（每条都可机械检查）

| 约束 | 落地方式 | 检查方式 |
|---|---|---|
| **每阶段可发布** ✓ | **阶段内可以有很多 commit** ✓（每个环节一个 ✓，方便回溯与验收 ✓）；**阶段收尾**才 = `scripts/soko gate` 全绿 + 课程门禁计数逐项不变 + **一次 push**（批次制 ✓）+ bump + release + 核对 | 阶段定义里写死"发版判据"一栏 ✓ |
| **性能只升不降** ✓ | ① 有收益的改动才**默认开** ✓；② 中性/前置改动**默认关**（开关 ✓）或**零调用**（惰性 API ✓）；③ 每阶段量一次 `docs/perf/ledger.jsonl` | 冷开 `unit12-solution`、`build courses/set-theory`、keystroke 三个基准数字 ✓ |
| **多步骤** ✓ | 每阶段 3–8 个环节，每环节**独立判据** | `- [ ]` 清单 + `plan.py` 可勾 ✓ |

**提交粒度（2026-09-24 用户明确）** ✓：
* **一个环节 = 一个 commit** ✓（信息里写清 `T-XX` + 做了什么 + 判据是什么 ✓）；
* **一个阶段 = 多个 commit** ✓ —— **不要求**阶段内每步都 push ✗；
* **阶段收尾**才 push 一次 ✓（跑一轮 CI ✓）⇒ bump → release → `gh release list` 核对 ✓；
* 例外：**诊断性 CI**（本地复现不了、怀疑平台差异 ✓）可以单独跑，但必须写进 `STATUS.md` ✓。

**三个基准数字（每阶段都要复量，只许变好 ✓）**：
```bash
# ① 大文件冷开（judge 密集）
SOKO_JUDGE_STATS=1 sokonanoda --json courses/set-theory/units/solutions/unit12-solution.sokonanoda
# ② 目录构建（闭包重复编译）
sokonanoda build courses/set-theory            # 基线 146.07s / 35 文件
# ③ 编辑中热编译（生命线）
SOKO_PERF_COURSE_SLOW=1 cargo test -p sokonanoda-lsp --lib perf_course -- --nocapture
```

---

## 1. 阶段总览（每行 = 一个可发布版本）

| 阶段 | 交付物（用户能感知什么） | 发版 | 性能预期 |
|---|---|---|---|
| **A 显示链路修复** | Infoview 的 `def` 卡片有 `:=` 值行 ✓；`unit12-synthesis` 的 goal 完全记法化 + 高亮 ✓ | 0.66.0（minor：用户可见修复） | 中性 |
| **B 命令与产物标准化** | VS Code 命令名统一成 `Sokonanoda: <Command> (说明)` ✓；模块根下有 `.sokonanoda/` ✓，vscode 与 code agent 共用产物 ✓ | 0.67.0 | **变好**：首次之后不再重复算 ✓ |
| **C 模块级批量编译** | `build <dir>` 从"每文件一份闭包"变成"每模块一次" ✓ | 0.68.0 | **大幅变好**：146s → 模块数量级 ✓ |
| **D 前缀复用（重型重构，5 小步）** | 大文件 `by` 密集解答的判定不再重编译整份前缀 ✓ | 0.69.0 → 0.70.0 → 0.71.0 | **每小步都更好**：12.4s → ~3s ✓ |
| **E 收尾与固化** | 文档/台账/技能同步；性能回归进 CI ✓ | 0.72.0 | 持平（防退化） |

**顺序理由** ✓：A/B 是**用户直接可感知**的修复与体验 ✓，先做 ✓；C 是**模块级批量编译**的地基 ✓，同时是 R-3 的实现 ✓；D 依赖 C 的"模块级"视野（D 的"前缀"在模块级才完整 ✓）；E 固化 ✓。

---

## 13. 执行清单

### 阶段 A：显示链路修复（R-1 / R-2）

#### 批次 A：显示链路修复（R-1 / R-2）

- [x] `T-A1` **R-1 根因已确认**（E1 第 98 轮 ✓）：`crates/lsp/src/query_map.rs:74` 的 `decl_info()` 漏映射 `value`/`value_runs` ⇒ `GoalDeclInfo` 里没有这两个字段 ⇒ 扩展恒拿 `undefined`（**前端算好、CLI 给了、LSP 没转发**的三段式断链）
- [x] `T-A2` **R-1 修**：`GoalDeclInfo` 加 `value: Option<String>` + `value_runs: Vec<RunInfo>`；`decl_info` 里映射（与 `ty_runs` 走**同一个** `run_info` ✓）
- [x] `T-A3` **R-1 判据**：LSP 单测断言 `soko/goals` 的 `Set.mem` 条目带 `value`/`value_runs` ✓ + **防分叉守卫**（`ty_runs` 与 `value_runs` 同实现 ✓）+ e2e 断言 Infoview 卡片出现 `:=` 行 ✓
- [x] `T-A4` **R-2 复现判红**：先写复现件，证明 `unit12-synthesis` 的 `flawed_equalities_refuted` / `project_chain` 的 goal 文本**未完全记法化**、且 Infoview 目标**不高亮**（照 R-1 的"三段式断链"逐段验：前端 → LSP → 扩展）
- [x] `T-A5` **R-2 修（真因已定位到行 ✓）**：`crates/front/src/semantic.rs:308` 把
  **内建记法表（含 `"="`）**喂给词法 ⇒ `fun (x : Nat) => z` 里 `=>` 的 `=` 被当声明符号
  吃掉 ⇒ 剩下的 `>` 无匹配 ⇒ 整段**降级成 1 个无 kind 的 run** ⇒ webview 只画纯文本
  ⇒ **凡 λ 出现在 goal/类型文本里的都掉色**（实测：`fun … => …` → 1 run；`z = z` → 5 runs ✓）
  ⇒ 修法：**别把 `=` 交给词法符号表**（它已有专用臂且先认 `=>`）；
  另补 `goal_runs` 的父子缺口（types/protocol/query_map/infoview 四处，**但只补它治不了本** ✗）：按复现件定位的那一段修（**先查明再改** ✗）
  - ✅ **已落地（2026-09-24）**：① `=`（与词法保留符号）不再喂给词法 ⇒ 含 λ 的
    goal **1 → 313 段** / **1 → 216 段**、对照组 **94 段不变**（反例守卫：普通 `=` 仍
    着 keyword 色）；② 父子 runs 补齐：front `DeclInfo.goal_runs`（父）+
    `goals_runs`（子，与 `goals` 按位置对齐）→ LSP `GoalDeclInfo` 转发 →
    `infoview.js` 渲染 `.decl-goal-line`（`目标` / `目标 i/n` + `⊢ …` 的 `tok-*` span）
    ⇒ **屏幕上**：开放练习的声明卡片多一行带色的 `⊢ <目标>`，闭合声明零变化；
    ③ 三层判据**都做过修前判红**（front 单测 / LSP wire 单测 / `test-webview.js`
    DOM 2/3 红 / 真宿主 e2e 对修前服务器 `1 failing`）；④ A∖B 守卫
    `scripts/audit-wire-fields.py` 改成**按消费者分组**（原来取全体并集 ⇒ `goal_runs`
    被 `soko/stateAt` 的同名字段顶包，声明侧漏了也不报 ✗），并把 `infoview.js` 的
    `decl.` 扫描从写死行区间改成整份文件（加目标行后 renderDecls 长过了区间末端）。
    as-built：`docs/design/goal-rendering.md` §9；R-2(a)（记法引擎 R5）**未修** ✗
- [x] `T-A6` **R-2 判据**：复现件转绿 + LSP/e2e 各一条断言 + 课程门禁计数逐项不变 ✓
  - ✅ **已收口（2026-09-24）**：**复现件转绿**——同一光标（三个声明的 `sorry` 行）
    对**修前**的 0.65.5 与修后的二进制各跑一次 `query state`：
    `flawed_equalities_refuted` **1 → 313 段**（带 kind **0 → 96**）、
    `project_chain` **1 → 216**（0 → 69）、对照组 `project_chain_cardinal`
    **94 → 94 不变**（34 → 34）；声明卡片那条路（`query goals`）同样从"无字段"变成
    289 / 165 段（全带 kind）✓。
    **LSP/e2e 断言**在 T-A5 的 commit 里（`crates/lsp/src/tests/goals.rs` +
    `editor/vscode/src/test/extension.test.js`，都做过修前判红）✓。
    **课程门禁计数逐项不变**：`python3 courses/set-theory/tools/check.py` →
    **36 个目标 · 328 checked · 99 open · 0 判负**（与基线逐项相同）✓。
    提交后复跑：front **720 passed** / LSP **161 passed** / webview **16/16** /
    stub 宿主 **34/34** / `audit-wire-fields.py` **NONE ✓ exit 0** ✓。
    性能（本环节自己的差分，不是三基准）：冷缓存 `query goals` 三个大文件
    old/new 中位数 **−3.6% / −0.4% / −10.9%** ⇒ **无退化** ✓。
- [x] `T-A7` **阶段 A 收尾**：三个基准数字复量（应持平 ✓）→ `scripts/soko gate` 全绿 → **一次 push** → CI 绿 → bump `0.66.0` → release → `gh release list` 核对 ✓
  - ✅ **已完成（2026-09-24）**：三基准复量（① 冷开 `unit12-solution` **11.4s** /
    `JUDGE_INFER` 51156 calls · 10623ms · misses 247；② 冷 `build courses/set-theory`
    **154.3s** —— 与历史基线 146.07s 的 +5.6% 是**跨会话漂移**，改动不在 `build` 的
    调用图上（`tag_runs*` 只有 query/LSP 展示路径会调，grep 实证）；③ `perf_course`
    全绿）→ `scripts/soko gate` **全绿**（含缺口台账「全部与台账一致」）→
    **一次 push**（`69a05bb`）→ CI **绿**（run 36029265840：lint + test +
    三平台 e2e 各 **26/26**）→ auto-tag → release（**v0.66.0**，**26 assets**，
    与 0.65.5 逐项同形）→ `gh release list` 显示 `sokonanoda v0.66.0 Latest` ✓。
    性能台账 + e2e 台账各一条、CHANGELOG 与两处版本号都在批次里 ✓。
  - 📌 **两条留给下一批的**：① `site/data/site.json` 顺手对齐到 **v0.66.0**
    （它从 **0.63.0** 起就没再生成过 —— `check-site.py` 之前是 8/9，**不是本轮引入的**；
    改完 9/9 ✓），这条改动**留在本地**，随下一次 push 生效；
    ② 本机 `git rebase`/`git merge` 都要 unlink 被改写的文件 ⇒ 被文件策略拒 ✗，
    **合 CI 的台账回写用 plumbing**（`git write-tree` + `git commit-tree -p HEAD
    -p origin/main` + `git update-ref`，全程不 checkout）。
  - ⬆ **BUMP**：`minor` —— Infoview 的 def 值行与 goal 记法化/高亮修复（用户可见）

### 阶段 B：命令与产物标准化（R-4 / R-3 前半）

#### 批次 B：命令与产物标准化（R-4 / R-3 前半）

- [x] `T-B1` **R-4 现状盘点**：列出 `editor/vscode/package.json` 里**全部** `sokonanoda.*` 命令的当前标题（做一张对照表 ✓）
  - ✅ **已盘点（2026-09-24）**：交付 `docs/design/command-naming.md` —— **15 条**命令的
    "现状（title/category/面板实际显示）→ 目标（`Sokonanoda: <Command> (说明)`）"对照表，
    外加机制取舍、静态契约影响、判据、T-B2/T-B3 清单。三个发现：
    ① **两套机制混用** ✗（9 条用 `category: "sokonanoda"`、6 条把前缀写进 `title`）；
    ② **前缀双写**两条 ✗：`openInfoview` 的 title 里又写一遍 `sokonanoda:`、`doctor`
    写成 `doctor:` ⇒ 面板里是 `sokonanoda: sokonanoda: 打开目标面板 (Infoview)` /
    `sokonanoda: doctor: 诊断服务器与版本`；③ 括号（全角/半角）与语言（纯英文/纯中文）
    都不统一。
    **判据**：`crates/cli/tests/extension.rs::command_naming_inventory_covers_every_contributed_command`
    —— 表里第一列的 id 集合与 `contributes.commands` **双向相等**（少一条 = 漏盘点、
    多一条 = 表说谎）；**抽掉一行实测判红**（exit 101）✓ 恢复后绿 ✓。
    **机制推荐 (A)**：`category: "Sokonanoda"` + 纯 `title`（前缀只写一处、命令面板还会
    按类别分组）；备选 (B) 把前缀写进 title（表里两列都已备好，切换只改一列）。
    **红线**：只改显示名，**`command` id 一个都不动**（键位/菜单/executeCommand/文档/
    技能都在用）。
    ⚠ **T-B2 的先决动作**：`build`/`rebuild` 的契约断言 `title.contains("build")`
    **大小写敏感**，而目标名是 `Build (…)` ⇒ 必须同时把断言改成大小写不敏感，
    否则必红 ✗。
- [x] `T-B2` **R-4 改标题**：统一 `Sokonanoda: <Command> (说明)` ✓（前缀固定、命令词首字母大写、括号内中文说明 ✓），含 `Infoview (目标面板)`、`Restart Server (重启服务器)` 等
  - ✅ **已改（2026-09-24）**：按 T-B1 的对照表改 **15 条**命令 —— 机制取推荐 **(A)
    `category: "Sokonanoda"` + 纯 `title`**（前缀只写一处、命令面板按类别分组）。
    `package.json` 的改动**只有 commands 块**（36+/28−）。
    **屏幕上**：命令面板 15 行全部变成 `Sokonanoda: <Command> (说明)`；两条**前缀双写**
    消失（`sokonanoda: sokonanoda: 打开目标面板 (Infoview)`、`sokonanoda: doctor: …`）；
    语言统一成"英文命令词 + 中文说明"；括号统一半角。另更新一处**用户可见文案**
    （声明栏读不到时的提示里那句命令名，`media/infoview.js`）。
    **判据**：静态契约两条（都做过修前判红 ✓）——
    `command_titles_follow_the_r4_naming_rule`（**四条断言**：`category == "Sokonanoda"`、
    title 里不许再出现包名、title 形如 `<Title Case 命令词> (中文说明)`、只用半角括号；
    回退 category 大小写 ⇒ exit 101、回退前缀双写 ⇒ exit 101，恢复后绿 ✓）+
    `command_naming_inventory_covers_every_contributed_command`（表↔manifest 双向相等）；
    同时把 `build`/`rebuild` 的 needle 断言改成**大小写不敏感**（目标名是 `Build (…)`，
    原断言大小写敏感必红 —— T-B1 盘点时发现并写进文档 ✓）。
    三层复跑：`cargo test -p sokonanoda-cli --test extension` **39 passed** ·
    `test-extension-host.js` **34/34** · `test-webview.js` **16/16** ✓。
    **说明**：命令面板的**显示文本本身没有 API 可断言**（VS Code 只暴露 id）⇒ 这一环的
    可判层就是 `contributes.commands` 的静态契约（它同时是唯一事实源）✓。
- [x] `T-B3` **R-4 同步四份**（AGENTS.md 硬规则）：`editor/vscode/` 的 README/CHANGELOG ✓ + `skills/` 三个技能 ✓ + `AGENTS.md` ✓ + `docs/vscode-dev-guide.md` ✓；`crates/cli/tests/skill.rs` / `dsh.rs` 不许漂移 ✓
  - ✅ **已同步（2026-09-24）**：旧标题引用**一处不剩**（`git grep` 复核；只剩历史
    记录：`REQUIREMENTS.md` §9 的旧轮次、`STATUS-ARCHIVE.md`、
    `docs/design/site-rebuild/**`（已标历史存档）、`CHANGELOG.md` 的旧版本条目）——
    改的 11 个文件：`editor/vscode/README.md`（8 处）·`skills/sokonanoda-teacher/SKILL.md`（3）·
    `AGENTS.md`（2）·`docs/vscode-dev-guide.md`（3）·`docs/protocol.md`（1）·
    `scripts/dev-loop.sh`（3）·`media/infoview.js` 的用户可见提示（1，见 T-B2）·
    `crates/lsp/src/lib.rs` / `tests/lifecycle.rs` / `crates/cli/tests/extension.rs` /
    `extension.js` / `extension.test.js` 的注释各 1。
    契约复跑：`skill.rs` **4 passed**（含 `skill_referenced_repo_paths_exist` ——
    改标题不许带坏路径）· `dsh.rs` **8 passed**（`.agents/skills/` 入口不漂移）·
    `extension.rs` **39 passed** ✓。
    **CHANGELOG 条目按仓库惯例留到 T-B7 的 bump commit**（`6490f1b`/`b0c35da` 两次
    都是这么做的：CHANGELOG 与版本号在同一个 commit 里进，日期才是发布日）——
    草稿："命令面板 15 条统一成 `Sokonanoda: <Command> (说明)`"，T-B7 直接抄。
- [x] `T-B4` **R-3 设计**：`.sokonanoda/` 目录的**契约**（放什么：编译产物 / 依赖 / 元数据；命名；清理策略；`--clean` 语义；与现有缓存 `~/.local/share/sokonanoda` 的关系 —— **模块根下的产物 vs 全局缓存**的分工 ✓）
  - ✅ **已出设计（2026-09-24）**：`docs/design/project-artifacts.md`（240 行，含 8 节 +
    逐条出处）。**刹车点解除** ✓（`e2-plan.md` §3 要求"分工说不清就只做 R-4"）——
    分工一句话：**产物按项目落盘、全局缓存退居跨项目共享的后备；项目条目只认模块根、
    单文件条目只认全局**，依据是**实测**：项目条目的键含**入口绝对路径**（两份逐字相同的
    项目 = **2 个条目**；同目录第二次 = **命中**），单文件条目的键只含内容
    （2 处相同文本 = **1 个条目**）。
    设计要点：目录树（`.gitignore`/`meta.json`/`compiled/<key>.json`，**预留** `prefix/`
    给 T-D11、`deps/` 给将来的依赖）· 命名**沿用**既有格式与键（不造第二套）·
    `--clean` **改成两处都清**（否则 `rebuild` 会命中项目条目 = 假动作 ✗），事件
    additive 加 `global`/`project` · `.gitignore` 采用**自忽略一行 `*`**（实测
    `git status` 完全看不见；变体 `*`+`!.gitignore` 会漏 `?? .sokonanoda/` ✗）·
    **红线：产物扩展名不许是 `.sokonanoda`**（`build <dir>` 的 `collect_files`
    **不跳隐藏目录** ⇒ 自我吞掉；VS Code 又监视 `**/*.sokonanoda` ⇒ 写-编回声）·
    **新不变量**：项目目录只放"磁盘状态产物"（带未落盘 overlay 的条目仍进全局缓存）·
    逃生门 `SOKONANODA_NO_PROJECT_ARTIFACTS=1` ⇒ 行为逐字节回到今天。
    **判据**：新增 `project::tests::closure_digest_is_scoped_to_the_entry_location`
    （两份逐字相同、不同目录 ⇒ digest 必须不同；抽掉 `digest_path(&self.entry)` 那行
    实测 **exit 101** ✓ 恢复后绿 ✓）——它钉的正是分工决定的地基。
    **两处口径纠正**：① `REQUIREMENTS`/计划里写的"现有缓存
    `~/.local/share/sokonanoda`"是**二进制**缓存；**编译产物**缓存在平台缓存目录
    （`compile/cache.rs:67-79`）；② `compile-cache.md` 里"`requires` 漂移 ⇒ 永不写
    缓存"的散文**已过期**（`report.rs:163-177` 实测：漂移不算"不干净"）。
    取证由 3 个只读 subagent 并行做（落盘清单 / 消费者清单 / 既有约束），
    已并入并**逐条复核**：其中"风险 #6（漂移挡写缓存）"与代码事实不符 ⇒ **未采纳**。
- [x] `T-B5` **R-3 实现（CLI 侧）**：`build`/`grade` 把模块级产物写进 `<模块根>/.sokonanoda/` ✓；第二次调用**命中** ✓
  - ✅ **已实现（2026-09-24）**。**屏幕上**：项目文件第一次 `build`/`grade` 之后，**模块根下
    出现 `.sokonanoda/`**（`compiled/<key>.json` + `.gitignore` 一行 `*` + `meta.json`），
    vscode 与 code agent 都能直接取；单文件（无 `import`）不产生它；`--clean` 两处都清。
    落点：`front::project::cache` 增 `load_at`/`store_at`/`store_if_clean_at`/`clean_at`/
    `artifacts_dir`（复用 `compile::cache` 的三个目录原语，**不造第二套格式**）+
    保留策略：**先按"32 条上限 + mtime 淘汰"做，实测踩到互相淘汰 ⇒ 已改成
    "每个入口只留最新一条"**（索引在 `meta.json` 的 `entries`，天然有界；设计 §3.6/§8b、
    判据 `every_entry_keeps_its_own_artifact_round_after_round`）+
    逃生门 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`；CLI 四处（`build`/`check`(grade)/`query`/
    `course`）传 `plan.root`；**LSP 读路径同轮接上**（否则 CLI 预热不再帮到编辑器 =
    **性能退化** ✗，写路径留给 T-C6）；`--clean` 两处都清（事件 additive：`{removed, global,
    project}`）；顺手修 `SOKONANODA_NO_CACHE=1` 会让 `--clean` 恒 0 的 bug。
    **判据**：新增 `crates/cli/tests/artifacts.rs`（5 条**真进程**用例：①产物落模块根 + 同格式 +
    自忽略 + meta schema + 全局无项目条目 + 第二次 **hit**；②单文件不建目录；③逃生门 ⇒ 不建目录 +
    退回全局 + 仍命中；④`--clean` 两处都清且保留元数据；⑤**跨根不回放**）。**修前判红**：
    `store_at` 改回只写全局 ⇒ ①**exit 101**；抽掉跨根守卫 ⇒ ⑤**exit 101**；恢复后 5/5 绿 ✓。
    全量回归 **1260 通过 / 35 套件 / 0 失败**（front+cli+lsp）· fmt ✓ · clippy exit 0 ✓。
    **取证 B 揪出的两个真问题都已处理**：① 项目摘要只含**入口路径**、不含模块根 ⇒
    `load_at` 的全局兜底按 `ProjectReport::root` 校验（上面判据⑤）；② VS Code e2e 的
    `cacheStamp()` 原来直接扫 `<cacheDir>/compiled` ⇒ 产物挪窝后会红，已改成**两处都扫**
    （顺带成为 R-3 在 e2e 层的断言），`scripts/vscode-e2e.sh` 每次跑前清夹具产物。
    **另外发现一条历史遗留**（不是本环节引入）：`course` 与 `build` 对入口路径的写法不同
    （闭包加载 `canonicalize`、CLI 用原样路径）⇒ **符号链接路径**（macOS `/tmp`、`/var`）
    下两者摘要不同、共用失效；实测规范化路径 ⇒ 命中、`/var` 原样 ⇒ miss。已记进设计 §6.3a
    （修法 = 统一归一化，会让旧条目作废，排在 R-3 之后）。
- [x] `T-B6` **R-3 判据**：同一模块连跑两次 `build`，第二次**显著更快** ✓（数字进台账）+ 产物目录内容可读（`--json` 能列 ✓）+ `.gitignore` 指引 ✓
  - ✅ **已完成（2026-09-24）**，三件事 + **三层判据**各一条：
    **① 数字进台账**：`crates/cli/tests/perf_project.rs` 增
    `project_build_hit_is_far_cheaper_than_a_cold_build` —— 同一模块连跑两次 `build`：
    **冷 40.3ms → 热 3.6ms（11.3×）**，并断言"产物确实在模块根"（否则这条数字与 R-3 无关）；
    经 `scripts/perf-ledger.sh` 记进 `docs/perf/ledger.jsonl`
    （`case: build_cold_warm_artifacts`，20 条记录那一批）✓。
    **② 产物目录可读（`--json` 能列）**：`ProjectView` 增**只读派生**字段
    `artifacts {dir, entries, bytes, compiler}`（`query project` 与 LSP `soko/project`
    共用同一份类型 ⇒ wire 自动带上；`#[serde(default)]` 保持协议只加不删）。
    纪律：只 `read_dir`，**目录不存在返回 `null`、绝不创建**（守 `project-view.md`
    的"只读派生"）。三层判据：**真相层** front 单测（`None` + 不创建 + 只数 `*.json` +
    字节/meta）、**wire 层** LSP 断言 `project.artifacts` 字段存在且值正确（确定性夹具）、
    **可见层** CLI 断言 `query project` 列出条目、逃生门下如实报 `null`。
    **③ `.gitignore` 指引**：产物目录**自带**一行 `*` 的 `.gitignore` ⇒ 默认零配置；
    想显式忽略就加一行 `.sokonanoda/`（本仓自己就这么做，双保险）；要提交则 `git add -f`
    —— 写进 README / AGENTS / 技能 / 设计文档 ✓。
    回归：**1263 通过 / 35 套件 / 0 失败** · fmt ✓ · clippy exit 0（我改的文件零提示）✓。
- [ ] `T-B7` **阶段 B 收尾**：基准复量（① ② 应变好 ✓）→ gate 全绿 → 一次 push → CI 绿 → bump `0.67.0` → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— 命令名标准化 + `.sokonanoda/` 产物目录（vscode 与 agent 共用，首次之后不再重复算）

### 阶段 C：模块级批量编译（T-K30 的正解）

#### 批次 C：模块级批量编译（T-K30 的正解）

> **为什么必须在这里做** ✗：E1 已实测"按模块根分组、每模块编一次"在**现有 API 下做不到** —— `plan_project` 只加载**单入口闭包** ⇒ 同模块互不 import 的文件是**不同闭包** ⇒ 需**新 API** ✓。

- [x] `T-C1` **设计**：`plan_module(root)` 的契约 —— 把模块根下**全部**文件作为**一个 unit 集**编一次 ✓、按文件给出报告 ✓、`ok(file)` 的语义与今天"以它为入口编一次"**逐项等价**（或明确记录差异 ✓）
  - ✅ **已出设计（2026-09-25）**：`docs/design/module-batch.md`（99 行）。
    关键事实（都带出处）：编译层**已经**支持多单元（`SourceUnit`/`unit_ranges`/
    `split_report`/`compile_all_units`，`compile/units.rs:113`），缺的只是**规划侧**；
    `run(units, options, collect)` 的第三参是 `collect` 不是"入口标志"（`check/mod.rs:446`）
    ⇒ 平坦批编没有"最后一个单元"的特例。
    契约要点：枚举**必须跳过 `.sokonanoda/`**（`build` 今天的 `collect_files` 不跳隐藏目录，
    而产物目录就在模块根下 ⇒ 两道防线一起保证产物不被当源文件）；取所有文件**闭包的并集**
    去重（既是入口又是依赖的文件只编一次 = 省下来的部分）；依赖先、平手按路径（确定性）；
    失败按文件隔离；缓存键从"入口闭包"扩成"整根并集"，逐文件旧条目仍可回退命中。
    **等价性契约**（写给 T-C3 判）：对每个文件 `F`，批报告必须与"以 `F` 为入口单独编"
    在**状态/诊断/`decl.checked` 事件**上逐项相同；并**预先点名三类可能差异**
    （名字遮蔽、重复声明诊断、未使用类警告）——因为平坦序让 `F` 的环境包含它**没有 import**
    的模块。**刹车沿用 E2 §3**：判不过就不接 `build`，只留 API + 判据 + 差异记录 ✓。

- [x] `T-C2` **实现 API**：新增规划入口（底层 `units: &[SourceUnit]` 本来就支持多单元 ✓）
  - ✅ **已实现（2026-09-25）**：`crates/front/src/project/module_plan.rs` ——
    `module_files(root)`（递归、排序、**跳过 `.sokonanoda/`**）、`plan_module(root)`
    （逐文件取自己的闭包 → **按路径去重**、保拓扑序地合并 → 加载期诊断去重）、
    `compile_module(plan, options)`（`compile_all_units` **编一次**，报告按文件归位）。
    单文件/单入口路径一行未动（新增模块，`build` 是否用它由 T-C4 的开关决定）。
    **判据**（2 条，都做过修前判红 ✓）：
    `module_files_are_sorted_and_skip_the_artifacts_directory`（产物目录里的
    `README.sokonanoda` **不许**出现在清单里 —— 抽掉跳过逻辑 ⇒ **exit 101** ✓）、
    `plan_module_dedups_shared_dependencies_and_keeps_topological_order`
    （Lib 被 B/C 共享 ⇒ 单元序 `[Lib, B, C]` 且 Lib **只出现一次**、三个文件都有报告 ——
    抽掉去重 ⇒ **exit 101** ✓）。恢复后 front **724 passed** ✓。
- [x] `T-C3` **等价性判据**：对同一门课，**逐文件**比较"新 API 的报告"与"旧路径（逐入口编译）的报告" —— 状态、诊断、`decl.checked` 事件**逐项相同** ✓
  - ⚠ **结论已按后续实测修正（见下）**：等价性只在"不重名的切片"上成立。
  - ⚠ **刹车触发（2026-09-25，实测）**：**同一模块根里两个会重名的单元**
    （e2e 夹具 `u01`/`u02` 都声明 `mem_self`/`subset_mem`/`extra_*`）在平坦批编下，
    后者会报**假的重复声明错误** ✗（逐入口真相：`u02` 全部 checked、无错误）。
    课程单元的常态就是重名 ⇒ **平坦批编对课程形状不等价**，**不接 `build`** ✗：
    CLI 接线与 `SOKO_BUILD_MODULE_BATCH` 开关**已撤掉**，只留 front API + 三条判据
    （含一条"**已知限制**"判据，断言这个差异存在）。设计 §10 有完整实测与机制。
    ⇒ **T-C4/T-C5 的"默认打开"前提不成立**：要真做模块级批量编译，必须先解决
    "单元之间的命名空间隔离"（T-K12c 那面墙，正是阶段 D 前缀复用要啃的）。
  - ✅ **（上述之前的）切片结论**：`crates/front/tests/module_batch.rs` ——
    夹具用例（进 CI）**绿**；**真课程切片**（真 `lib/**` + 2 个真单元，11 文件）
    **逐文件差异 0 条**（声明状态 / 错误 / 警告 / 事件序列含重基 `cmd` 全同；
    T-C1 §3 点名的三类差异在真实内容上**没有出现**）。
  - ⚠ **但量出负结果（这条改变了阶段 C 的结论）**：**同口径**下平坦批编
    **11,491ms** vs 逐入口 **3,723ms** ⇒ **慢约 3×** ✗。机制：批里每个单元都在
    **更长的前缀**上工作，单元越大越贵；合成小单元项目里反而是批编快 1.98×
    （`crates/front/tests/perf_module_batch.rs`）。⇒ 按 E2 §3 的刹车精神：
    **API 与判据留在树上（惰性、默认关、零成本）**，**不**默认打开；`T-C5` 的
    "默认打开（收益已证）"**不成立**，除非阶段 D 的前缀复用把长前缀代价压下来。
    设计 §8 有完整数字与机制分析。
- [x] `T-C4` **`build <dir>` 接入**（开关 `SOKO_BUILD_MODULE_BATCH=1` 默认**关** ✓，先保证零退化 ✓）
  - ⛔ **按刹车处理：接线已实现又撤掉（2026-09-25）**。先按设计 §9 的分组
    （(模块根, prelude 模式)）把批路径接进 `build.rs`、开关默认关，一验就发现
    **等价性不成立** ✗：同一目录下 `u01 + u02` 一起批编时 `u02` 报 **假的重复声明错误**
    （两者都声明 `mem_self`/`subset_mem`/`extra_*` —— 课程单元的常态），而逐入口真相是
    **零错误**。⇒ 按 E2 §3 **撤掉接线与开关**（不留"打开就产生假失败"的暗雷），
    `build.rs` 回到逐入口路径、逐字节不变 ✓。判据保留：含一条**反向**的
    `colliding_unit_names_are_a_known_batch_limitation`（断言差异存在）。
- [x] `T-C5` **量收益**：`build courses/set-theory` 从 **146.07s** 降下来 ✓（数字进台账）；**默认打开** ✓（收益已证 ✓）
  - ⛔ **结论：收益为负 ⇒ 不默认打开（2026-09-25）**。同口径实测（真课程切片：
    真 `lib/**` + 2 个真单元）：批编 **11,491ms** vs 逐入口 **3,723ms** ⇒ **慢约 3×** ✗；
    机制 = 每个单元都在**更长的前缀**上工作（单元越大越贵）。合成小单元项目里反过来
    （批编 1.98× 快，`perf_module_batch`）⇒ **收益取决于"共享依赖重复量"与"前缀增长"
    谁占上风**，而课程形状（大 lib + 大单元 + 单元间重名）**两者都不利** ✗。
    数字在 `docs/perf/ledger.jsonl`（`batch_vs_per_entry`）与设计 §8。
    ⇒ **不打开、不发这条**；要真做模块级批量编译，先解决**单元间命名空间隔离**
    （T-K12c 那面墙 = 阶段 D 前缀复用的题目）。
- [x] `T-C6` ~~**LSP/agent 接入**：从 `.sokonanoda/` 取模块级产物~~ ⇒ **按实测重新界定（2026-09-25）**：
  **"模块级产物"随批编一起作废** ✗（批编不接线 ⇒ 没有"一批的产物"这种东西）。
  改成接**阶段 B 留下的那一半**：**LSP 的写路径搬进模块根产物目录** ——
  T-B5 只把**读**路径接上了（否则 CLI 预热帮不到编辑器 ✗），**写**仍在全局缓存；
  搬过来之后，编辑器里编出来的东西也落 `.sokonanoda/` ⇒ "vscode 与 code agent
  一处取用"这条 R-3 承诺才真正完整 ✓，而且与 `query project` 的 `artifacts` 字段
  互为印证（同一目录、同一索引）。判据要用**真进程/真宿主**：LSP 打开项目文档后
  `<模块根>/.sokonanoda/compiled/` 出现条目、`soko/project` 的 `artifacts` 非空；
  **不变量照旧**：带未落盘 overlay 的摘要**只写全局缓存**（项目目录只放"磁盘状态产物"）。
  - ✅ **已实现（2026-09-25）**：`lsp/src/lib.rs` 的项目写站点改走
    `store_if_clean_at(&root, …)`；**判据不是 `overlay.is_empty()`** —— 编辑器里它
    **永远为假**（打开文档本身就带文本 ✗，实测直接红了），而是"**overlay 里每份文本都与
    磁盘一致**"才算磁盘状态 ✓，有未落盘编辑就退回全局缓存。
    判据：`crates/lsp/src/tests/project.rs::project_request_describes_…` 断"打开项目文档后
    `<模块根>/.sokonanoda/compiled/` 真有条目、`artifacts` 非空且 `compiler` 是当前版本" ✓；
    顺带修了取证 B 早点名的布局耦合：`crates/lsp/tests/lsp_cache.rs` 原来只数
    `<cache>/compiled`（T-C6 之后该数**两处之和**）⇒ LSP 套件 **161 passed / 0 failed** ✓。
- [ ] `T-C7` **阶段 C 收尾**：基准复量（② 应大幅变好 ✓）→ gate 全绿 → 一次 push → CI 绿 → bump `0.68.0` → release → 核对 ✓
  - 📌 **收尾口径已按实测改写（2026-09-25，待执行）**：本阶段的**用户可见产出只有
    T-C6**（编辑器编出来的产物也落 `<模块根>/.sokonanoda/`）—— 而它**随 0.67.0 一起发布**
    （那一批的 CHANGELOG 已并入，见 `editor/vscode/CHANGELOG.md` 的 `[0.67.0]`）。
    剩下的 T-C4/T-C5 是**刹车后的负结果**（不接线、不默认打开），T-C1..T-C3 是内部 API
    与判据（惰性、零成本）⇒ **阶段 C 没有第二个用户可见增量**。
    ⇒ **不发空的 `0.68.0`** ✗（一个没有任何变化的版本号只会污染发版历史）。
    T-C7 因此改为：**基准复量 + gate 全绿 + 文档收口**，并在本文与
    `docs/design/module-batch.md` 里写明"阶段 C 的发布已并入 0.67.0、0.68.0 取消"。
    下一批有真实用户可见改动时再 bump（阶段 D 的前缀复用若成功，就是那批的内容）。
  - ⬆ **BUMP**：`minor` —— `build <dir>` 按模块只编一次（146s → 模块数量级）

### 阶段 D：前缀复用（重型重构；**5 个小步，每步都可发布**）

#### 批次 D：前缀复用（重型重构；**5 个小步，每步都可发布**）

> **病灶**（已量 ✓）：冷开 `unit12-solution` 的 `judge_infer` **11183ms / 90%**，
> 其中 **misses=247 ≈ 9.5s = 77%** —— 每次 miss 都把**整份前缀**重编译一趟。
> **墙**（已查明 ✓）：`decl_idx` 是"每个名字**全局唯一**"的槽位 ⇒ **两个环境无法共存**
> ⇒ 唯一出路是让 **walk 自己的环境**就是那个环境（check-then-add 移进 walk ✓）。
> **E1 的教训**（已实测 ✓）：**影子彩排路走不通** ✗（三个假设全否证 ✓）⇒ D 只能
> **直接做** ✓，安全网是"五步 + 课程计数 + 全语料逐字节 + 事件计数"四件套 ✓。

- [ ] `T-D1` **把 check-then-add 收进单一函数**（纯重构，**零行为变化** ✓）：`kernel_phase` 里逐条"检查→加入"的逻辑抽成一个可复用单元 ✓
  - 📌 **执行方案（已勘定，2026-09-25，照着做即可）**：
    * **抽取位置**：`crates/front/src/compile/check/kernel_phase.rs` 的
      `finish_pass(walked)`（:43）里那个**逐命令循环**（`for (j, sig_slot) in sigs.iter_mut().enumerate()`，
      :82 起）—— 循环体里 `PendingOp::Decl` 分支就是"检查→加入"本体（`declar_signature` /
      类型检查 / `add_declar` / 记录 `decl_states` 与 `kernel_checks`）。
    * **抽成什么**：`fn check_then_add_one(env: &mut Env, declar: &Declar, cmd: usize, …) -> Contribution`
      —— 入参 = 环境 + 那条声明的 payload + 命令下标 + 它要写回的累积量
      （`decl_states` / `kernel_checks` / `failed_cmds` / hover）；返回值 = 这一条对环境的
      **贡献**（`sigs[j]` 用它算签名，早期截断用它做 `acc_new == acc_old` 比较 ✓）。
    * **必须留在循环里的**（**不许**抽进去 ✗）：早期截断（`allow_cutoff` / `before` /
      `text_unchanged` / `acc_new`/`acc_old` 比较，:88-95）—— 它是**跨命令**的状态机 ✓；
      以及**趟级**的东西（`display` 记法表、`env.config.pp_options.proofs`，:63-71 ✓）。
    * **重构判据（= T-D2）**：① 全语料两态 `--json` **逐字节相同**；② 课程计数逐项不变
      （36 目标 · 328 checked · 99 open · 0 判负）；③ 五步全绿（kernel tests + front 单测 +
      CLI e2e + 语料对拍 + 性能）。**做法**：动手前先跑一次存档 `--json` 输出（两态各一份），
      改完再跑、`cmp` 逐字节比 ✓ —— 比"看代码觉得没变"强得多 ✓。
    * **风险**：这个循环同时管**判定**与**呈现**（`kernel_checks` 喂判卷、`decl_states` 喂 UI）
      ⇒ 抽取时最容易漏掉"哪几个累积量是判定用的、哪几个是显示用的" ✗；
      按"判定量必须逐字节一致、显示量也必须"来对拍最稳 ✓。
- [ ] `T-D2` **D1 判据**：全语料两态 `--json` **逐字节相同** ✓ + 课程计数逐项不变 ✓ + 五步全绿 ✓（**不变量**：这一步不改任何行为 ✓）
- [ ] `T-D3` **walk 增量检查（开关默认关）**：walk 边 elaborate 边 `with_env` 检查并 `add_declar` ✓；`SOKO_WALK_CHECK=1` 才启用 ✓ ⇒ 默认路径**零变化零成本** ✓
- [ ] `T-D4` **D3 判据**：开关两态 `--json` 逐字节相同 ✓ + 课程计数不变 ✓ + 事件计数不变 ✓（**开关开**时也相同 ✓ ⇒ 证明"两遍检查"语义等价 ✓）
- [ ] `T-D5` **judge 接快照（开关默认关）**：`run_by`/`judge_infer` 拿 `Option<&EnvBuilder>` ⇒ `snapshot()` 查合成声明 ✓；拿不到**回退**旧路径 ✓；`SOKO_JUDGE_ENV_REUSE=1` 才启用 ✓
- [ ] `T-D6` **量收益 + 默认打开**：`SOKO_JUDGE_STATS` 看 `JUDGE_INFER` 的 miss 成本是否塌下来 ✓；**收益成立才默认打开** ✓（否则保持关闭并记录 ✗）
- [ ] `T-D7` **阶段 D-1 收尾**：基准 ① 复量（应大幅变好 ✓）→ gate + 四件套 → 一次 push → CI 绿 → bump **`0.69.0`（minor）** → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— judge 前缀复用第一刀：大文件 by 密集解答不再重编译整份前缀
- [ ] `T-D8` **去掉重复检查**（第二刀）：`kernel_phase` 不再重查 walk 已核的声明 ✓（**只删重复** ✓，语义由 D4 的对拍保证 ✓）
- [ ] `T-D9` **D8 判据 + 量收益**：四件套全过 ✓ + 基准 ① 再降 ✓（数字进台账 ✓）
- [ ] `T-D10` **阶段 D-2 收尾**：bump **`0.70.0`** → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— 去掉重复检查（第二刀）
- [ ] `T-D11` **跨会话复用**（第三刀）：把"已验证的前缀环境"按 `.sokonanoda/` 持久化 ✓ ⇒ **编辑中热编译**再降 ✓（生命线 ✓）
- [ ] `T-D12` **D11 判据**：keystroke 基准复量 ✓ + 四件套 ✓
- [ ] `T-D13` **阶段 D-3 收尾**：bump **`0.71.0`** → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— 跨会话复用已验证前缀（第三刀，编辑中热编译）

### 阶段 E：收尾与固化

#### 批次 E：收尾与固化

- [ ] `T-E1` **性能回归进 CI**：把三个基准做成 CI 可跑的 smoke（阈值宽松 ✓，只抓**大幅退化** ✗）
- [ ] `T-E2` **文档收口**：`architecture.md` §6 台账 / `docs/PERF.md` / 技能与 `AGENTS.md` 同步 ✓
- [ ] `T-E3` **`STATUS.md` 与 `HANDOVER.md` 更新** ✓
- [ ] `T-E4` **阶段 E 收尾**：bump **`0.72.0`** → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— 性能回归进 CI + 文档/技能收口

---

## 3. 风险与刹车点（每阶段都有一条"停下"的判据）

| 阶段 | 刹车点 | 停下后做什么 |
|---|---|---|
| A | R-2 的复现件做不出**判红** ✗ | 说明"症状"描述需要更具体 ⇒ 请用户补一个最小例子 ✓ |
| B | `.sokonanoda/` 与全局缓存的**分工**说不清 ✗ | 只做 R-4（命令名 ✓），R-3 单独出设计再议 ✓ |
| C | **等价性判据不过** ✗（T-C3） | **不接 `build`** ✗，把 API 与判据留在树上（惰性 ✓ 零成本 ✓）并记录差异 ✓ |
| D | **四件套任一不过** ✗ | 该小步**默认关** ✓（不影响已发布版本 ✓）并记录；D 可停在任一小步 ✓ |
| E | — | — |

**D 的额外护栏** ✓：D3/D5/D8 全部**先开关后默认** ✓ ⇒ 任何一步出问题，**已发布的版本都不受影响** ✓，且**性能不会倒退** ✓（默认路径零变化 ✓）。

---

## 4. 与 E1 的关系（哪些结论被继承）

| E1 的结论 | E2 怎么用 |
|---|---|
| T-K11（K1-a）**实测零收益** ✗ | 代码保留为**惰性开关** ✓；D5 复用它那条链 ✓ |
| T-K12c / T-K13 接线**存档** ⏸（`decl_idx` 墙 ✓） | **D 就是那次重构** ✓，从 D1 起重新分步 ✓ |
| T-K30 **重新定级** ✗（需新 API ✓） | **阶段 C** ✓（先设计契约、再做等价性判据 ✓） |
| T-K31 **实测无收益** ✗ | **不做** ✓（记账即可 ✓） |
| 三个内核原语（`with_env` / `snapshot` / `Clone` ✓）+ 对照器 | D 的**现成积木** ✓ |
| 四份实测账 | 每阶段的**基准** ✓（只许变好 ✓） |

---

## 14. 规格索引（`plan.py` 取规格用；每条一行，正文见 §13 的清单）

##### T-A1 **R-1 根因已确认**（E1 第 98 轮 ✓）：`crates/lsp/src/query_map.rs:74` 的 `decl_info()` 漏映射 `value`/`value_runs` ⇒ `GoalDeclInfo` 里没有这两个字段 ⇒ 扩展恒拿 `undefined`（**前端算好、CLI 给了、LSP 没转发**的三段式断链）

见 §13 清单里 `T-A1` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-A2 **R-1 修**：`GoalDeclInfo` 加 `value: Option<String>` + `value_runs: Vec<RunInfo>`；`decl_info` 里映射（与 `ty_runs` 走**同一个** `run_info` ✓）

见 §13 清单里 `T-A2` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-A3 **R-1 判据**：LSP 单测断言 `soko/goals` 的 `Set.mem` 条目带 `value`/`value_runs` ✓ + **防分叉守卫**（`ty_runs` 与 `value_runs` 同实现 ✓）+ e2e 断言 Infoview 卡片出现 `:=` 行 ✓

见 §13 清单里 `T-A3` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-A4 **R-2 复现判红**：先写复现件，证明 `unit12-synthesis` 的 `flawed_equalities_refuted` / `project_chain` 的 goal 文本**未完全记法化**、且 Infoview 目标**不高亮**（照 R-1 的"三段式断链"逐段验：前端 → LSP → 扩展）

见 §13 清单里 `T-A4` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-A5 **R-2 修**：按复现件定位的那一段修（**先查明再改** ✗）

见 §13 清单里 `T-A5` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-A6 **R-2 判据**：复现件转绿 + LSP/e2e 各一条断言 + 课程门禁计数逐项不变 ✓

见 §13 清单里 `T-A6` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-A7 **阶段 A 收尾**：三个基准数字复量（应持平 ✓）→ `scripts/soko gate` 全绿 → **一次 push** → CI 绿 → bump `0.66.0` → release → `gh release list` 核对 ✓

见 §13 清单里 `T-A7` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-B1 **R-4 现状盘点**：列出 `editor/vscode/package.json` 里**全部** `sokonanoda.*` 命令的当前标题（做一张对照表 ✓）

见 §13 清单里 `T-B1` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-B2 **R-4 改标题**：统一 `Sokonanoda: <Command> (说明)` ✓（前缀固定、命令词首字母大写、括号内中文说明 ✓），含 `Infoview (目标面板)`、`Restart Server (重启服务器)` 等

见 §13 清单里 `T-B2` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-B3 **R-4 同步四份**（AGENTS.md 硬规则）：`editor/vscode/` 的 README/CHANGELOG ✓ + `skills/` 三个技能 ✓ + `AGENTS.md` ✓ + `docs/vscode-dev-guide.md` ✓；`crates/cli/tests/skill.rs` / `dsh.rs` 不许漂移 ✓

见 §13 清单里 `T-B3` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-B4 **R-3 设计**：`.sokonanoda/` 目录的**契约**（放什么：编译产物 / 依赖 / 元数据；命名；清理策略；`--clean` 语义；与现有缓存 `~/.local/share/sokonanoda` 的关系 —— **模块根下的产物 vs 全局缓存**的分工 ✓）

见 §13 清单里 `T-B4` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-B5 **R-3 实现（CLI 侧）**：`build`/`grade` 把模块级产物写进 `<模块根>/.sokonanoda/` ✓；第二次调用**命中** ✓

见 §13 清单里 `T-B5` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-B6 **R-3 判据**：同一模块连跑两次 `build`，第二次**显著更快** ✓（数字进台账）+ 产物目录内容可读（`--json` 能列 ✓）+ `.gitignore` 指引 ✓

见 §13 清单里 `T-B6` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-B7 **阶段 B 收尾**：基准复量（① ② 应变好 ✓）→ gate 全绿 → 一次 push → CI 绿 → bump `0.67.0` → release → 核对 ✓

见 §13 清单里 `T-B7` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-C1 **设计**：`plan_module(root)` 的契约 —— 把模块根下**全部**文件作为**一个 unit 集**编一次 ✓、按文件给出报告 ✓、`ok(file)` 的语义与今天"以它为入口编一次"**逐项等价**（或明确记录差异 ✓）

见 §13 清单里 `T-C1` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-C2 **实现 API**：新增规划入口（底层 `units: &[SourceUnit]` 本来就支持多单元 ✓）

见 §13 清单里 `T-C2` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-C3 **等价性判据**：对同一门课，**逐文件**比较"新 API 的报告"与"旧路径（逐入口编译）的报告" —— 状态、诊断、`decl.checked` 事件**逐项相同** ✓

见 §13 清单里 `T-C3` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-C4 **`build <dir>` 接入**（开关 `SOKO_BUILD_MODULE_BATCH=1` 默认**关** ✓，先保证零退化 ✓）

见 §13 清单里 `T-C4` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-C5 **量收益**：`build courses/set-theory` 从 **146.07s** 降下来 ✓（数字进台账）；**默认打开** ✓（收益已证 ✓）

见 §13 清单里 `T-C5` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-C6 **LSP/agent 接入**：从 `.sokonanoda/` 取模块级产物 ✓（与 B5 合流 ✓）

见 §13 清单里 `T-C6` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-C7 **阶段 C 收尾**：基准复量（② 应大幅变好 ✓）→ gate 全绿 → 一次 push → CI 绿 → bump `0.68.0` → release → 核对 ✓

见 §13 清单里 `T-C7` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D1 **把 check-then-add 收进单一函数**（纯重构，**零行为变化** ✓）：`kernel_phase` 里逐条"检查→加入"的逻辑抽成一个可复用单元 ✓

见 §13 清单里 `T-D1` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D2 **D1 判据**：全语料两态 `--json` **逐字节相同** ✓ + 课程计数逐项不变 ✓ + 五步全绿 ✓（**不变量**：这一步不改任何行为 ✓）

见 §13 清单里 `T-D2` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D3 **walk 增量检查（开关默认关）**：walk 边 elaborate 边 `with_env` 检查并 `add_declar` ✓；`SOKO_WALK_CHECK=1` 才启用 ✓ ⇒ 默认路径**零变化零成本** ✓

见 §13 清单里 `T-D3` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D4 **D3 判据**：开关两态 `--json` 逐字节相同 ✓ + 课程计数不变 ✓ + 事件计数不变 ✓（**开关开**时也相同 ✓ ⇒ 证明"两遍检查"语义等价 ✓）

见 §13 清单里 `T-D4` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D5 **judge 接快照（开关默认关）**：`run_by`/`judge_infer` 拿 `Option<&EnvBuilder>` ⇒ `snapshot()` 查合成声明 ✓；拿不到**回退**旧路径 ✓；`SOKO_JUDGE_ENV_REUSE=1` 才启用 ✓

见 §13 清单里 `T-D5` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D6 **量收益 + 默认打开**：`SOKO_JUDGE_STATS` 看 `JUDGE_INFER` 的 miss 成本是否塌下来 ✓；**收益成立才默认打开** ✓（否则保持关闭并记录 ✗）

见 §13 清单里 `T-D6` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D7 **阶段 D-1 收尾**：基准 ① 复量（应大幅变好 ✓）→ gate + 四件套 → 一次 push → CI 绿 → bump **`0.69.0`（minor）** → release → 核对 ✓

见 §13 清单里 `T-D7` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D8 **去掉重复检查**（第二刀）：`kernel_phase` 不再重查 walk 已核的声明 ✓（**只删重复** ✓，语义由 D4 的对拍保证 ✓）

见 §13 清单里 `T-D8` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D9 **D8 判据 + 量收益**：四件套全过 ✓ + 基准 ① 再降 ✓（数字进台账 ✓）

见 §13 清单里 `T-D9` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D10 **阶段 D-2 收尾**：bump **`0.70.0`** → release → 核对 ✓

见 §13 清单里 `T-D10` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D11 **跨会话复用**（第三刀）：把"已验证的前缀环境"按 `.sokonanoda/` 持久化 ✓ ⇒ **编辑中热编译**再降 ✓（生命线 ✓）

见 §13 清单里 `T-D11` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D12 **D11 判据**：keystroke 基准复量 ✓ + 四件套 ✓

见 §13 清单里 `T-D12` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-D13 **阶段 D-3 收尾**：bump **`0.71.0`** → release → 核对 ✓

见 §13 清单里 `T-D13` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-E1 **性能回归进 CI**：把三个基准做成 CI 可跑的 smoke（阈值宽松 ✓，只抓**大幅退化** ✗）

见 §13 清单里 `T-E1` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-E2 **文档收口**：`architecture.md` §6 台账 / `docs/PERF.md` / 技能与 `AGENTS.md` 同步 ✓

见 §13 清单里 `T-E2` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-E3 **`STATUS.md` 与 `HANDOVER.md` 更新** ✓

见 §13 清单里 `T-E3` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-E4 **阶段 E 收尾**：bump **`0.72.0`** → release → 核对 ✓

见 §13 清单里 `T-E4` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。
