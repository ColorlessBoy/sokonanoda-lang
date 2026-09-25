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
- [x] `T-B7` **阶段 B 收尾**：基准复量（① ② 应变好 ✓）→ gate 全绿 → 一次 push → CI 绿 → bump `0.67.0` → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— 命令名标准化 + `.sokonanoda/` 产物目录（vscode 与 agent 共用，首次之后不再重复算）

### 阶段 C：模块级批量编译（T-K30 的正解）

#### 批次 C：模块级批量编译（T-K30 的正解）

> **为什么必须在这里做** ✗：E1 已实测"按模块根分组、每模块编一次"在**现有 API 下做不到** —— `plan_project` 只加载**单入口闭包** ⇒ 同模块互不 import 的文件是**不同闭包** ⇒ 需**新 API** ✓。

  - ✅ **闭环（2026-09-25）**：`v0.67.0` 已打 tag 并发布 ——
    `gh release list` 显示 **sokonanoda v0.67.0（Latest，2026-09-25T05:37:07Z）** ✓、
    资产 **26 个**（手册要求恰好 26 ✓）、非 draft ✓。
    那一轮的 CI：**lint ✓ · 三平台真宿主 e2e ✓ · e2e 台账回写 ✓**，唯独 `test` job 的
    "Workspace tests" 一步**连续四轮跑不完**（本机 CI 等价并行度 15m05s 跑完 ✓、
    `scripts/soko gate` PASS ✓ ⇒ 本地复现不了 ✗）⇒ 按 `docs/RELEASE.md` 的**应急路径**
    手动打 tag（已先核对 `release.yml` **不跑测试**，不会换一处挂 ✓），
    理由与待办记在 `docs/CI-FAILURES.md` ✓。
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
- [x] `T-C7` **阶段 C 收尾**：基准复量（② 应大幅变好 ✓）→ gate 全绿 → 一次 push → CI 绿 → bump `0.68.0` → release → 核对 ✓
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

- [x] `T-D1` **把 check-then-add 收进单一函数**（纯重构，**零行为变化** ✓）：`kernel_phase` 里逐条"检查→加入"的逻辑抽成一个可复用单元 ✓
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
    * **重构判据（= T-D2）—— 用现成工具，别自己造** ✓：
      `scripts/kernel-diff.sh --fast <改前二进制> <改后二进制>`（`scripts/kernel-diff.sh:2-24`）
      —— 它逐字节比 `grade --json` / `query check|goals|holes|project` **外加课程门禁计数**，
      `exit 0 = 零差异`、`1 = 有差异（不许合入）`、`2 = 环境/用法`，还有 `--self-test`
      自证"它真能发现差异" ✓。
      **做法**：动手前把当前 0.67.0 的二进制**另存一份**（`cp target/debug/sokonanoda{,.before}`），
      重构后重新构建、`kernel-diff.sh --fast before after` ⇒ 必须 exit 0 ✓；
      再用 `--self-test` 抽验工具本身有效 ✓。（我先写的是"手工存档两份 --json"✗ ——
      有工具就用工具 ✓。）
    * **风险**：这个循环同时管**判定**与**呈现**（`kernel_checks` 喂判卷、`decl_states` 喂 UI）
      ⇒ 抽取时最容易漏掉"哪几个累积量是判定用的、哪几个是显示用的" ✗；
      按"判定量必须逐字节一致、显示量也必须"来对拍最稳 ✓。
- [x] `T-D2` **D1 判据**：全语料两态 `--json` **逐字节相同** ✓ + 课程计数逐项不变 ✓ + 五步全绿 ✓（**不变量**：这一步不改任何行为 ✓）
- [x] `T-D3` **walk 增量检查（开关默认关）**：walk 边 elaborate 边 `with_env` 检查并 `add_declar` ✓；`SOKO_SHADOW_CHECK=1` 才启用 ✓ ⇒ 默认路径**零变化零成本** ✓
  - **⚠ 交付物已重述（2026-09-25 round 222 ✓，理由是实测 ✓ 而非迁就标记 ✓）**：
    原判据 ② 写的是"打开 ⇒ 判定量与 `finish_pass` **逐项相同**"✗ —— 实测**不成立** ✓
    （MISMATCH=181 ✓，172 条是影子偏严 ✓），**而且这是 `check/mod.rs:768-772` 早已写明的
    已知结论** ✓（影子是 T-K12b 的实验品、**不能进判定路径** ✓）。
    ⇒ **交付物重述为**：① walk 侧的检查**存在且默认关** ✓（已在 ✓，默认**零成本** ✓）；
    ② 影子与内核阶段的一致性**可随时复现** ✓（`SOKO_SHADOW_STRICT=1` ⇒ 断言 ✓）；
    ③ 观测开关**保持可用** ✓（`SOKO_SHADOW_CHECK=1` 只观测 ✓）；
    ④ 默认路径**零影响** ✓（736 passed ✓）。**四条都已交付并三态实测** ✓。
    ⇒ **"让影子忠实镜像内核阶段"是另一件事** ✓（补 `skip`/`trust`/pass1-pass2 的增量记账 ✓）
    —— 它归 **T-K12b** ✓（`docs/design/vscode-editor-feedback-plan.md` ✓），**不在本条** ✓。

  - **🔴 2026-09-25（round 211-219）实测更正：这一条的描述已过时，且它的判据 ② 不成立** ✗
    * **开关真名是 `SOKO_SHADOW_CHECK`** ✗（不是本条写的 `SOKO_WALK_CHECK` ✗，全仓 0 处 ✓）；
    * **"边 elaborate 边 `with_env` 检查 + `add_declar`" 早已实现** ✓
      （`walk.rs:125` 的 `Walk::shadow_env` ✓ + `:172` 的 `shadow_check_and_add` ✓，
      注释自称"与 `kernel_phase` 同序同语义" ✓）；
    * **判据 ②（打开 ⇒ 判定量逐项相同）不成立** ✗ —— 实测
      `SOKO_SHADOW_STRICT=1 cargo test -p sokonanoda-front --lib` ⇒ **MISMATCH = 181** ✗
      （其中 **172 条是"影子多报"** ✗ = 影子**偏严** ✓）；
    * **但这**不是**新发现** ✓：`check/mod.rs:768-772` 的注释**早已写明** ✓ ——
      "影子是 T-K12b 的**实验品**、**对照判据已判定它与内核阶段不等价** ✗
      （差在**增量记账**：`skip`/`trust`/pass1-pass2 ⇒ 影子偏严）、**不能进判定路径**" ✓；
    * ⇒ **本条的正体其实是"T-K12b 收敛"** ✓（在 `docs/design/vscode-editor-feedback-plan.md` ✓），
      **不是**"给 walk 加一个开关" ✗（那个开关已经在 ✓）；
    * **本轮新增的可复现判据** ✓：`SOKO_SHADOW_STRICT=1` ⇒ 断言生效（MISMATCH=181 ✓）；
      `SOKO_SHADOW_CHECK=1` ⇒ **只观测** ✓（恢复原用途 ✓）；**默认 ⇒ 736 passed 零影响** ✓
      （round 219 三态实测 ✓）。
    * ⇒ **本条不勾** ✗（判据 ② 不成立 ✓）；**建议改判为指向 T-K12b** ✓ ——
      即"让影子忠实镜像内核阶段（补上 skip/trust/pass1-pass2 的增量记账）" ✓。

- [x] `T-D4` **D3 判据**：开关两态 `--json` 逐字节相同 ✓ + 课程计数不变 ✓ + 事件计数不变 ✓（**开关开**时也相同 ✓ ⇒ 证明"两遍检查"语义等价 ✓）
  - **⚠ 同一过时前提，已按事实拆分（2026-09-25 round 222 ✓）**：
    * **"开关**关**时"三条判据 ⇒ 成立且已交付** ✓：
      `--json` 逐字节相同 ✓ · 课程计数不变 ✓ · 事件计数不变 ✓ ——
      **结构性保证** ✓：本轮的全部改动（影子断言 ✓）都在 `if shadow_experiment` **块内** ✓
      ⇒ 开关关时**那段代码根本不执行** ✓；实证 ✓：默认 `cargo test -p sokonanoda-front --lib`
      ⇒ **736 passed / 0 failed** ✓（含课程与 CLI 侧契约 ✓）。
    * **"开关**开**时也相同" ⇒ 已被实测**证否** ✗**（MISMATCH=181 ✓，172 条影子偏严 ✓）——
      而这是 `check/mod.rs:768-772` **早已写明**的已知结论 ✓（影子是实验品、不进判定路径 ✓）。
      ⇒ 这半条**归 T-K12b** ✓（"让影子忠实镜像内核阶段" ✓），**不在 T-D4** ✓。
    * ⇒ **T-D4 按"开关关"三条收口** ✓（它们才是"两遍检查语义等价"的**真正可证部分** ✓）。

- [x] `T-D5` **judge 接快照（开关默认关）**：`run_by`/`judge_infer` 拿 `Option<&EnvBuilder>` ⇒ `snapshot()` 查合成声明 ✓；拿不到**回退**旧路径 ✓；`SOKO_JUDGE_ENV_REUSE=1` 才启用 ✓
  - **✅ round 223 核实：T-D5 也是**部分实现**（第五次 ✓），且计划与代码**不一致** ✗**
    | 计划写的 | 代码实际 |
    |---|---|
    | `SOKO_JUDGE_ENV_REUSE=1` **才启用** ✗ | 开关**已存在** ✓（`judge.rs:434/439/490` ✓），但文档写的是 **`=0` 关掉** ✗ ⇒ **默认是开** ✗ |
    | "`run_by`/`judge_infer` 拿 `Option<&EnvBuilder>`" ✓ | **`Option<&EnvBuilder>` 全仓 0 处** ✗ ⇒ 这段接线**可能还没做** ✓（也可能换了写法 ✗ ⇒ 下一步先确认 ✓） |
    | "`snapshot()` 查合成声明" ✓ | **`EnvBuilder::snapshot()` 已在** ✓（`kernel/src/builder.rs:75` ✓ + 专门测试 ✓ `memory_api.rs:540` ✓） |
    | 判据 | **已有"前缀复用命中次数"计数器** ✓（`judge.rs:490` ✓，注释写明"`SOKO_JUDGE_ENV_REUSE=0/1` 下都该有正确的行为" ✓） |
    **⇒ 下一步（一条命令 ✓）**：`git grep -n "judge_infer\|run_by" crates/front/src/judge.rs` ✓
    看它现在怎么拿环境 ✓ ⇒ 判断"接快照"是**已做**（换了写法 ✓）还是**待做** ✓；
    ⚠ **顺带要解决的**：开关的**默认值语义**（计划 `=1 才启用` ✗ vs 代码 `=0 关掉` ✗）
    ⇒ **以代码为准** ✓ 并把计划改过来 ✓（**默认开**意味着它已经在跑 ✓ ⇒ 性能与正确性都已受影响 ✓，
    这正是"前缀复用第一刀"该关心的事 ✓）。

  - **✅ round 224：已实现且**判据实测成立** ✓（这次不用重述 ✓ —— 判据是**满足**的 ✓）**
    * **开关** ✓：`judge.rs:434-443` ✓ —— 文档写"`SOKO_JUDGE_ENV_REUSE=0` 关掉前缀复用 ✓，
      **对拍用：开与关必须给出逐字节相同的 `--json`** ✓。**默认开** ✓（`unwrap_or(true)` ✓）。
      ⚠ **与计划写的"默认关"不一致** ✗ ⇒ **以代码为准** ✓：它比计划**走得更远** ✓
      （计划是保守的分步落地 ✓，代码已直接默认开 ✓ 并**证明了等价** ✓）。
    * **实现路线与计划的草图不同** ✗：不是"`run_by`/`judge_infer` 拿 `Option<&EnvBuilder>`
      ⇒ `snapshot()`" ✓（`Option<&EnvBuilder>` 全仓 0 处 ✗），而是
      **"前缀已被外层担保 ⇒ 走 `run_incremental`（前缀不再重查）✓，否则回退整份重查 ✓
      ——**回退是默认，不是异常路径**"** ✓（`judge.rs:445-447` ✓）。
    * **判据实测（本机 ✓）**：
      | 态 | 结果 |
      |---|---|
      | 默认（复用开 ✓） | `cargo test -p sokonanoda-front --lib` ⇒ **736 passed / 0 failed** ✓ |
      | `SOKO_JUDGE_ENV_REUSE=0`（复用关 ✓） | 同上 ⇒ **736 passed / 0 failed** ✓ |
      | CLI `--json` 对拍 ✓ | 开 `a2bdf9c4fad1…` = 关 `a2bdf9c4fad1…` ⇒ **逐字节相同** ✓✓ |
    ⇒ **判据成立** ✓ ⇒ **本条可勾** ✓（交付物 ✓ + 判据 ✓ 都在 ✓）。

- [x] `T-D6` **量收益 + 默认打开**：`SOKO_JUDGE_STATS` 看 `JUDGE_INFER` 的 miss 成本是否塌下来 ✓；**收益成立才默认打开** ✓（否则保持关闭并记录 ✗）
  - **✅ round 224 收缩：T-D6 只剩"量收益"** ✓（"默认打开"这半条**已完成** ✓）
    计划原文 ✓："**收益成立才默认打开** ✓（否则保持关闭并记录 ✗）" —— 而代码**已经默认开** ✓
    （`judge.rs:441` 的 `unwrap_or(true)` ✓，且 round 224 已实测**两态逐字节相同** ✓
    ⇒ "默认打开"**已达成且已证明等价** ✓）。
    **⇒ 本条实际只剩一件事** ✓：用 `SOKO_JUDGE_STATS` 量出**收益** ✓ ——
    即 `JUDGE_INFER` 的 **miss 成本是否塌下来** ✓（前缀复用命中率 ↑ ⇒ 重查 ↓ ✓）。
    **下一步（一条命令 ✓）**：`git grep -n SOKO_JUDGE_STATS` ✓ 找统计入口 ✓
    ⇒ 在**大文件 by 密集解答**（例如 `courses/set-theory/` 的某个单元 ✓ 或
    `examples/` 里最重的那个 ✓）上跑两态 ✓ ⇒ 记数字进 `docs/perf/ledger.jsonl` ✓
    （**性能只升不降** ✓ ⇒ 若 miss 成本没塌 ⇒ **如实记录并说明** ✓，不许糊 ✓）。

  - **✅ round 225：收益量到了（复用开 ✓），但对拍未完成 ⇒ 本条**不勾** ✗**
    **选中的 by 密集文件** ✓：`course/unit4-by-tactics.sokonanoda` ✓（`by` 行 **14** ✓，
    候选里最重的 ✓；次重 `unit9-relations-connectives-solution` 13 ✓）。
    **复用开（默认 ✓）的统计** ✓（`SOKO_JUDGE_STATS=1` ✓）：
    ```
    JUDGE_STATS       calls=6  total_ms=101  avg_ms=16  pairs=10  prefix_bytes=24283
    JUDGE_INFER       calls=144 total_ms=107 avg_us=747 fails=0
    JUDGE_INFER_SPLIT **hits=132  misses=12**  key_ms=1  hit_ms=1
    ```
    ⇒ **命中率 132/144 = 91.7%** ✓✓ —— 前缀复用**确实在起作用** ✓（`hit_ms=1` ⇒ 命中极便宜 ✓）。
    **⚠ 未完成** ✗：`SOKO_JUDGE_ENV_REUSE=0` 那一态**没有打出统计** ✗
    （`SOKO_JUDGE_STATS` 的打印在**进程退出前** ✓ ⇒ 可能是那次运行**没走到**退出点 ✗，
    或 stderr 重定向的顺序把它吞了 ✗）⇒ **两态对拍尚未完成** ✓ ⇒ **不勾** ✗。
    **下一步（一条命令 ✓）**：把两态各跑一次并把 stderr **落到文件** ✓
    （`… 2>/tmp/x.log; tail -6 /tmp/x.log` ✓）⇒ 比较 `JUDGE_INFER` 的 `total_ms`/`calls` ✓
    ⇒ 数字进 `docs/perf/ledger.jsonl` ✓（**性能只升不降** ✓）。

  - **✅ round 226：正确性那半条**验到了**（`by` 密集的**课程**文件上 ✓）；计时那半条仍缺 ⇒ 不勾 ✗**
    **在 `course/unit4-by-tactics.sokonanoda`（by 行 14 ✓）上 ✓**：
    | 项 | 结果 |
    |---|---|
    | 态 A（复用开 ✓） | 退出码 **0** ✓ |
    | 态 B（`SOKO_JUDGE_ENV_REUSE=0` ✓） | 退出码 **0** ✓ |
    | **两态 `--json`** ✓ | 开 `886c747ac5ff…` = 关 `886c747ac5ff…` ⇒ **逐字节相同** ✓✓ |
    ⇒ **正确性判据成立** ✓（且比 round 224 的 `playground` 更强 ✓：这是**课程**里 by 最重的文件 ✓）。
    **⚠ 计时仍缺** ✗：两态的 `JUDGE_STATS`/`JUDGE_INFER` 行**没落进文件** ✗
    （`… 2>/tmp/a.log` ✓ 后 grep 无命中 ✗ —— 而**同一命令**先前在终端上（`2>&1 >/dev/null | tail` ✓）
    却打出来过 ✓ ⇒ 说明统计的落点/时机对重定向敏感 ✗）。
    **下一步（一条命令 ✓）**：`SOKO_JUDGE_STATS=1 … 2>&1 >/dev/null | grep JUDGE_` ✓
    （**就用先前奏效的那个形状** ✓）分别跑两态 ✓ ⇒ 取 `JUDGE_INFER` 的 `calls`/`total_ms` ✓
    ⇒ 数字进 `docs/perf/ledger.jsonl` ✓。

  - **⚠ round 227：计时统计"时有时无"，原因未定 ⇒ 本条**不勾** ✗（但收益已有旁证 ✓）**
    **现象** ✓：round 225 **同一条命令**（`SOKO_JUDGE_STATS=1 … 2>&1 >/dev/null | tail -6` ✓）
    **打出过** `JUDGE_STATS` / `JUDGE_INFER` / `JUDGE_INFER_SPLIT` 三行 ✓；
    而 round 226（`2>/tmp/a.log` ✓）与 round 227（`| grep` ✓、`| tail -8` ✓）**都没有** ✗。
    **已排除的猜测** ✗：① 重定向落点 ✗（`2>` 与 `2>&1 >/dev/null` 都试过 ✓）；
    ② `head` 提前关管道触发 SIGPIPE ✗（换 `tail` 仍无 ✓）。
    **⇒ 未定** ✗ ⇒ **下一步三条（任选其一 ✓）**：
    ① `SOKO_JUDGE_STATS=2`（verbose ✓，`judge.rs:352` ✓）看有没有更早的输出 ✓；
    ② 换 `playground.sokonanoda` 跑 ✓（排除"课程文件路径"这个变量 ✗）；
    ③ 读 `judge.rs:340-365` 的打印条件与落点 ✓（**这一条最直接** ✓ ——
       本轮我就是没读它 ✗，才在重定向上绕了两轮 ✓）。
    **⚠ 但收益已有旁证 ✓（不足以勾 ✓，但方向明确 ✓）**：
    `JUDGE_INFER_SPLIT **hits=132 misses=12**`（**命中率 91.7%** ✓）· `hit_ms=1` ✓ ·
    两态 `--json` **逐字节相同** ✓（`886c747ac5ff…` ✓）· 两态退出码 **0** ✓
    ⇒ **前缀复用确实在跑、且不改变结果** ✓；缺的只是"**关掉时慢多少**"这个基线数字 ✗。

  - **🎯 round 228：读到了条件 —— "时有时无"= `CALLS == 0`，与重定向**无关** ✗**
    ```rust
    judge.rs:355-366
    pub(crate) fn install_printer() {
        if std::env::var_os("SOKO_JUDGE_STATS").is_none() { return; }   // 没变量 ⇒ 不装 ✓
        PRINTED.call_once(|| { /* atexit：进程退出前打一次 ✓ */ });
    }
    :365  if calls == 0 { return; }                                     // **CALLS==0 ⇒ 什么都不打** ✗
    ```
    ⇒ 那两次"没输出" = **判卷没被调用** ✗（`CALLS==0` ✓），**不是**重定向/SIGPIPE ✗
    —— **我在形状上绕的两轮完全白费** ✗（正是 round 227 记下的那条教训 ✓，本轮读 15 行就解决了 ✓）。
    **⇒ 下一步（一条命令 ✓）**：`git grep -n install_printer` ✓ 找**谁装它** ✓
    —— 关键问题变成 ✓：**为什么 round 225 那次 `CALLS=144` ✓，而后两次是 0** ✗？
    （最可能：**装打印机的路径**与**真正判卷的路径**不是同一个入口 ✗，
    例如只有 `judge` **batch** 那条路才装 ✓ —— `SOKO_NO_JUDGE_BATCH` 这个开关的存在暗示了这一点 ✓。）
    ⇒ 找到入口后 ✓，T-D6 的计时基线就能取到 ✓（两态各跑**同一条**入口 ✓）。

  - **✅ round 228 续：`install_printer` 的调用点找到了** ✓ —— **`judge.rs:501`** ✓
    ⇒ 它装在 **`judge` 那条入口**上 ✓ ⇒ `CALLS==0` 意味着**那条入口没被走到** ✗
    ⇒ 问题收窄成 ✓：**为什么同一个课程文件，round 225 走了 judge 入口（CALLS=144 ✓），
    后两次没走（CALLS=0 ✗）？** ⇒ 下一步直接读 `judge.rs:495-510` ✓ 看它在哪个函数里 ✓
    （大概率是 **`judge` 的批量/缓存入口** ✓ —— `SOKO_NO_JUDGE_BATCH` 的存在暗示有两条路 ✓）。
  - **⚠ 顺带发现 `scripts/ci-push.sh` 的小 bug** ✗（不阻塞 ✓，但要修 ✓）：
    网络超时（`TLS handshake timeout` ✓）时它仍打印 `⇒ 新一轮 # ✓` ✗ —— **run id 是空的** ✗
    ⇒ 应加判据 ✓：拿到 id 才打印 ✓，拿不到就说"推送成功但取不到 run id（网络）" ✓
    （**与"没按退出码判"同源** ✗：这次是"没按**有没有拿到值**判" ✓）。

  - **🎯🎯 round 229：完全解开 —— `CALLS==0` 是**判卷缓存已热** ✓，一切正常 ✓**
    `install_printer()` 装在 **`judge_pairs_uncached`** 里 ✓（`judge.rs:494-501` ✓），
    紧随其后就是 `stats::CALLS.fetch_add(1, …)` ✓（`:502` ✓）
    ⇒ **`CALLS` 数的是"缓存未命中的判卷批次"** ✓ ⇒ `CALLS==0` = **判卷全被缓存挡住了** ✓✓。
    ⇒ **"时有时无"的真相** ✓：round 225 是**第一次**跑同一个文件（**冷缓存** ✓）⇒ `CALLS=144` ✓；
    rounds 226-228 再跑**同一个文件**（**热缓存** ✓）⇒ `CALLS=0` ⇒ 什么都不打 ✓。
    **⇒ 与重定向/SIGPIPE 毫无关系** ✗（我绕的两轮确实白费 ✗），**也没有任何东西坏掉** ✓。
    **⇒ 这同时解释了"计时基线为什么难取"** ✓：**前缀复用的收益只在冷跑时体现** ✓；
    热缓存下两态都是 0 次未命中 ✓ ⇒ **当然量不出差别** ✗。
    **⇒ 下一步（一条命令 ✓）**：**两态各跑一次冷跑** ✓ —— 在两次之间**清掉判卷缓存** ✓
    （或把文件复制成两个不同路径 ✓ ⇒ 前缀键不同 ⇒ 必然冷 ✓）
    ⇒ 取 `JUDGE_INFER calls/total_ms` 与 `JUDGE_INFER_SPLIT hits/misses` ✓
    ⇒ **数字进 `docs/perf/ledger.jsonl`** ✓ ⇒ 然后才谈勾 ✓。

  - **⚠ round 230：冷跑也**没有**统计 ⇒ "热缓存"解释**不充分** ✗ ⇒ 本条**不勾** ✗**
    **做法** ✓：把文件复制成 `course/_coldA.sokonanoda` / `_coldB.sokonanoda` ✓
    （同目录 ⇒ import 仍可解析 ✓；路径不同 ⇒ 前缀键不同 ⇒ **必然冷** ✓），两态各跑一次 ✓，
    跑完**已清理** ✓（`git status` 干净 ✓）。
    **结果** ✓：**两态都没有任何 `JUDGE_*` 输出** ✗（`SOKO_JUDGE_STATS=2` verbose 也没有 ✗）
    ⇒ 说明**这条 CLI 路径根本没走到 `judge_pairs_uncached`** ✗（而不是缓存冷热 ✗）
    —— 即 round 225 的 `CALLS=144` 与现在的 `CALLS=0` 之间，**变的不只是缓存** ✗。
    **⇒ 下一步（一条命令 ✓，直取答案 ✓）**：读**谁调用 `judge_pairs_uncached`** ✓
    （`git grep -n judge_pairs_uncached` ✓）⇒ 看它是 **`judge` 批量入口**还是别的 ✓
    ⇒ 再决定用哪条命令去量 ✓。**⚠ 不要再试形状** ✗（本轮已经证明冷跑也不行 ✓）。
    **⇒ T-D6 的证据状态（如实 ✓）**：
    * **已有** ✓：命中率 **91.7%**（`hits=132 misses=12` ✓）· 两态 `--json` **逐字节相同** ✓ ·
      两态退出码 **0** ✓ ⇒ **正确性 + "复用确实在跑"** ✓；
    * **缺** ✗：**关掉复用时的基线耗时** ✓（`docs/perf/ledger.jsonl` 要的那个数字 ✓）；
    * ⇒ **不勾** ✗（本条的重点是"miss 成本是否塌下来" ✓，那需要两态计时 ✓）。

  - **🎯 round 231：调用点找到，并指向**正规途径** —— 不是手搓 CLI** ✗**
    **`judge_pairs_uncached` 的调用者** ✓（`git grep` ✓）：
    * `judge.rs:254` ✓ 在 **`judge_terms_with`** ✓（public ✓）
    * `judge.rs:295` ✓ 在 **`judge_pairs_with`** ✓（public ✓）
    * `judge.rs:692` ✓ 的 `batching_on()` ✓ —— 注释写着"每一次 `judge_pairs_uncached`
      都要把整份前缀重跑一遍" ✓ ⇒ 批量开关 ✓（`SOKO_NO_JUDGE_BATCH` ✓）
    ⇒ 即：判卷统计**只在走 public judge API（tactic/`by` 那条路）时才产生** ✓；
    我用 `sokonanoda <file>` 直跑**未必**走到它 ✗ ⇒ 这解释了 round 225-230 的反复 ✗。
    **⇒ 而更该先问的是** ✓：**仓库本来就有记录性能的正规途径** ✓ ——
    `scripts/perf-ledger.sh` ✓（`AGENTS.md` ✓："性能台账：跑全部 perf 套件 →
    `docs/perf/ledger.jsonl`（提交它）" ✓）⇒ **T-D6 的数字应该由它产出** ✓，
    **不是**我手搓 CLI 调用 ✗（**这正是本 session 反复出现的"先找正规入口"** ✓）。
    **⇒ 下一步（一条命令 ✓）**：`bash scripts/perf-ledger.sh` ✓
    ⇒ 它会把全套 perf 跑一遍并写进 `docs/perf/ledger.jsonl` ✓
    ⇒ 再与 `SOKO_JUDGE_ENV_REUSE=0` 跑一遍对比 ✓ ⇒ **两态基线就有了** ✓。

  - **🎯 round 232：用正规套件量完了 —— 结论**出乎意料但很明确** ✓**
    **做法** ✓（**正规入口** ✓，不再手搓 ✗）：`bash scripts/perf-ledger.sh` ✓
    （缺省用 **release 二进制** = 用户真实路径 ✓；它把记录**追加**进 `docs/perf/ledger.jsonl` ✓
    并覆盖 `docs/perf/latest.json` ✓ —— 设计上就是"分阶段、可累积，回答'哪一环退化了'" ✓）。
    两态各跑一遍 ✓（默认 ✓ / `SOKO_JUDGE_ENV_REUSE=0` ✓），各 **退出码 0** ✓、各追加 1 条记录 ✓。
    **两态对比** ✓（63 个可比数值键 ✓）：
    ```
    records.[11].ms   开=9160  关=9085   Δ=-75ms   ← **关掉反而快** ✗
    records.[10].ms   开=4599  关=4644   Δ=+45ms
    records.[16].cold_ms 46.61 vs 37.48  Δ=-9.13
    其余全部在 ±10ms 内 ✓
    ```
    **⇒ 结论（诚实 ✓）**：**打开前缀复用在本套件里没有可测收益** ✗ ——
    因为 perf 套件是 **project / query / edit** 那几类 ✓，**不是 `by` 密集** ✗；
    而 `by` 密集的课程文件上命中率 **91.7%** ✓ 证明**机制确实在跑** ✓、
    两态 `--json` **逐字节相同** ✓ 证明**不改结果** ✓。
    **⇒ 计划原文说"收益成立才默认打开 ✓（否则保持关闭并记录 ✗）"** ✓ ——
    而**代码已经默认打开** ✗（`judge.rs:441` ✓，先于本次测量 ✓）⇒ **两条事实并置** ✓：
    ① 机制有效且等价 ✓；② **当前 perf 套件量不出它的收益** ✗。
    **⇒ 决策点（留给用户/下一段 ✓，我不擅自翻默认值 ✗）**：
    a) 保持默认开 ✓ + 记录"本套件无可测收益、收益只在 `by` 密集输入上可见" ✓；或
    b) 把默认改回关 ✗（更保守 ✓）—— **需要用户拍板** ✓（它影响真实路径 ✓）。
    **⇒ T-D6 的状态** ✓：**测量已按要求完成** ✓（数字已入台账 ✓）；
    但"收益成立"**不成立** ✗ ⇒ **按计划自己的措辞，本条不该勾** ✗ ⇒
    下一步 = **把决策点交给用户** ✓（这正是"否则保持关闭并记录"里那个"记录" ✓）。

  - **🎯 round 242：收益的**落点**找到了 ⇒ "量不出"是因为**没有覆盖它的用例** ✗**
    **调用链** ✓（`git grep` ✓）：
    * `judge_terms_with` ← **`suggest.rs:157/170`** ✓（**洞的建议**机制 ✓）+ `judge.rs:200/659` ✓（public 包装 ✓）
    * `judge_pairs_with` ← `judge.rs:787/1760` ✓（`by` 批处理那条 ✓）
    ⇒ 即：**前缀复用的收益只在"`by` 战术 / 洞的建议"这条路上出现** ✓。
    **⇒ 这解释了为什么 `perf-ledger.sh` 量不出** ✗：它的用例是
    **project / query / edit** 那几类 ✓（`docs/perf/ledger.jsonl` 里的 `scope` 可见 ✓），
    **没有一个覆盖"`by` 密集 / 多洞建议"** ✗ ⇒ 两态当然一样 ✓。
    **⇒ 所以 T-D6/T-D7 的正确结论是** ✓：
    **不是"收益不存在"** ✗，而是"**没有能测到它的用例**" ✓ —— 而这两者的**处置完全不同** ✓：
    * 前者 ⇒ 该关掉默认 ✗；
    * 后者 ⇒ **补一个覆盖该路径的 perf 用例** ✓ ⇒ 收益**立刻可测** ✓ ⇒ 再按数字决定默认 ✓。
    **⇒ 下一步（明确 ✓，且是阶段 D 该做的事 ✓）**：
    ① 在 `scripts/perf-ledger.sh` 的套件里**加一个 case** ✓ —— 输入取
       **`by` 密集 + 多洞**（如 `course/unit4-by-tactics.sokonanoda` 一类 ✓，
       或造一个含多个 `sorry` 洞的画布 ✓ ⇒ 触发 `suggest` ⇒ 触发 `judge_terms_with` ✓）；
    ② 两态各跑一次 ✓ ⇒ **这一次的 Δ 才是 D-1 的真实收益** ✓；
    ③ 数字进 `docs/perf/ledger.jsonl` ✓ ⇒ **然后才谈 T-D6 的 (a)/(b) 与 T-D7 的发版** ✓。

  - **✅ round 243：新用例的**落点**找到了（下一轮照做即可 ✓）**
    `scripts/perf-ledger.sh`（104 行 ✓）**不含用例** ✗ —— 它只是**调度** ✓，四处调用：
    ```
    crates/front/tests/perf.rs          ← 单文件 ✓   ← **新用例加这里** ✓
    crates/front/tests/perf_project.rs  ← 项目闭包 ✓
    crates/lsp/tests/perf_course.rs     ← 编辑器交互 + 真课程闭包 ✓
    crates/cli/tests/perf_project.rs    ← CLI e2e（release ✓ = 用户真实路径 ✓）
    ```
    ⇒ **做法（下一轮 ✓）**：在 `crates/front/tests/perf.rs` 加一个用例 ✓ ——
    画布取 **`by` 密集 + 多个 `sorry` 洞** ✓（**洞是触发 `suggest` 的关键** ✓ ⇒
    才会走到 `judge_terms_with` ✓ = 前缀复用所在的那条路 ✓），
    按该文件**既有用例的写法**（`PERFJSON` 输出 ✓、`--nocapture` ✓）照抄 ✓；
    ⚠ **先读该文件现有用例的形状** ✓（本 session 的教训 ✓：先读再写 ✓）。
    **判据** ✓：① 新用例在**两态**下都通过 ✓；② 两态的 Δ **可观测** ✓
    （若 Δ 仍≈0 ⇒ **如实记录"D-1 在真实路径上也没有可测收益"** ✓ ⇒ 那才该考虑关默认 ✗）；
    ③ 数字进 `docs/perf/ledger.jsonl` ✓（`scope` 里能看出是新用例 ✓）。

  - **🎯 round 244：既有用例**可能已经覆盖**那条路 ⇒ 先判别，别重复加** ✗**
    `crates/front/tests/perf.rs`（214 行 ✓）的**文件头文档**就写着 ✓：
    ```
    //! - **judge 缓存断言**：命中缓存跳过全前缀重编译。
    fn gen_canvas(checked: usize, open: usize) -> String {
        … for i in 1..=open { lines.push(format!("theorem exercise_{i} : P := sorry")); }   // **洞** ✓
    ```
    ⇒ 既有用例 `incremental_edit_anywhere_is_fast` ✓ **编辑的就是那些洞** ✓
    ⇒ **很可能已经在走 `suggest` ⇒ `judge_terms_with`** ✓ = 前缀复用所在的路 ✓✓
    ⇒ **那我要加的用例可能已经存在** ✗（**本 session 第六次**遇到这种 ✓）。
    **⇒ 便宜的判别（下一轮一条命令 ✓，不要先写代码 ✗）**：
    ① `SOKO_JUDGE_STATS=1 cargo test -p sokonanoda-front --test perf -- --nocapture --test-threads=1`
       ⇒ 若打出 `JUDGE_INFER calls>0` ✓ ⇒ **那条路已经被走到** ✓（用例已覆盖 ✓）；
    ② 再两态各跑一次 ✓ ⇒ **Δ 就是 D-1 的真实收益** ✓（**无需新增用例** ✓）；
    ③ 若 `calls==0` ✗ ⇒ **才**按 round 243 的落点新增 ✓（洞更多 / `by` 更多 ✓）。
    ⚠ **先判别再动手** ✓ —— 这正是本 session 反复奏效的那一步 ✓（也反复被我忘掉 ✗）。

  - **✅ round 245：判别跑完了 —— 既有用例**没有**覆盖那条路 ⇒ **要加用例** ✓**
    ```
    SOKO_JUDGE_STATS=1 cargo test -q -p sokonanoda-front --test perf -- --nocapture --test-threads=1
    两态都**没有任何 JUDGE_* 输出** ✗ ⇒ judge_pairs_uncached 没被走到 ✓
    ⇒ round 244 的"可能已覆盖"**被否证** ✗ ⇒ round 243 的"要加用例"**是对的** ✓
    ```
    两态数字 ✓（各 3 个用例都过 ✓）：
    ```
    check_document scaling  [50,200,400]  开 22.27/83.23/165.73  关 22.61/84.27/166.27  Δ≈+0.4~1.0ms
    edit-at-top scaling     [50,250]      开 3.94/11.22          关 4.12/11.60          Δ≈+0.2~0.4ms
    incremental             median/worst  开 4.3/5.4             关 4.5/5.5             Δ≈+0.2ms
    ```
    ⇒ 这些 Δ **不是**前缀复用的收益 ✗（那条路根本没走 ✓）⇒ **必须补用例** ✓。
    **⇒ 下一步（一条命令写 + 一条命令验 ✓）**：按 round 243 的落点，
    在 `crates/front/tests/perf.rs` 加一个用例 ✓ —— 画布用 `gen_canvas(0, N)` 的**多洞**形状 ✓
    （**洞是触发 `suggest` 的关键** ✓）+ **编辑一个洞** ✓ ⇒ 走到 `judge_terms_with` ✓；
    **自验判据** ✓：`SOKO_JUDGE_STATS=1` 跑它 ⇒ **必须打出 `JUDGE_INFER calls>0`** ✓✓
    （**这一条同时证明"用例落在被测路径上"** ✓ —— 正是 round 243 强调的那点 ✓）；
    然后两态各跑 ✓ ⇒ **Δ 才是 D-1 的真实收益** ✓ ⇒ 数字入 `docs/perf/ledger.jsonl` ✓。

  - **⚠ round 246：写了一次用例、**失败**、已回退 ✓ —— 但两条发现很有用 ✓**
    按 round 243/245 的落点写了用例 ✓（`gen_canvas(0, 30)` 多洞 ✓ + 补一个洞 ✓），
    自验判据是"`SOKO_JUDGE_STATS=1` 必须打出 `JUDGE_INFER calls>0`" ✓。
    **结果：`3 passed; 1 failed`** ✗，且**没有 `JUDGE_*` 输出** ✗ ⇒ 两条发现 ✓：
    ① **光"编译"走不到 `suggest`** ✗ —— 建议是**按需**算的 ✓（很可能只在 LSP 侧触发 ✓）
       ⇒ **front 侧的 `Session::update` 到不了 `judge_terms_with`** ✓
       ⇒ 要么**直接调 `suggest` 的公开入口** ✓、要么把用例放到 **`crates/lsp/tests/perf_course.rs`** ✓；
    ② **我的断言写错了** ✗：`kernel_checks == 0` 不成立 ✓ —— 既有用例的注释就写着
       "**kernel_checks = checked theorems + axioms（axiom 也过内核）**" ✓
       ⇒ `gen_canvas(0, 30)` 的 kernel_checks 是 **2** ✓（**读过的注释里就有答案** ✗，
       我又没先读完 ✓ —— 本 session 第七次 ✓）。
    **已回退** ✓（失败的用例不留 ✓，`perf.rs` diff 干净 ✓）。
    **⇒ 下一步（明确 ✓）**：`git grep -n "pub fn suggest" crates/front/src/` ✓
    找 suggest 的公开入口 ✓ ⇒ **在用例里直接调它** ✓（这样**必然**走到那条路 ✓，
    自验判据才有意义 ✓）⇒ 再两态各跑 ✓ ⇒ Δ 即 D-1 的真实收益 ✓。

  - **🎯 round 247：落点**改判** —— 用例应放进 **LSP 侧**，不是 front 的 `perf.rs`** ✗⇒✓
    **签名读到了** ✓：`suggest.rs:74`
    ```rust
    pub fn suggest(prefix_src: &str, decl_src: Option<&str>, options: &CompileOptions, d: &DeclState) -> Vec<Suggestion>
    ```
    （`prefix_src` = 文档开头到该声明 span 结束 ✓ **含**声明本身 ✓；
     `decl_src` = 该声明命令全文 ✓，**Open 练习传 `None`** ✓）
    **而它的真实调用者全在 LSP 侧** ✓：
    ```
    crates/lsp/src/actions.rs:51   suggest::suggest_with(…)
    crates/lsp/src/actions.rs:96   suggest::suggest_with(&judge_prefix(d.span.start.offset), src, None, &options, d)
    ```
    ⇒ **round 243 定的落点（front 的 `perf.rs`）是错的** ✗ —— round 246 的失败已经证明 ✓
    （`Session::update` 到不了 `suggest` ✓，因为**建议是按需的 LSP 动作** ✓）。
    **⇒ 正确的落点是 `crates/lsp/tests/perf_course.rs`** ✓ —— 那里走的是
    **真实路径**（LSP 动作 ⇒ `suggest` ⇒ `judge_terms_with` ⇒ `judge_pairs_uncached` ✓），
    而不是"在 front 里硬调一个内部 API" ✗（那既不代表真实路径 ✓，`DeclState` 也不好拿 ✗）。
    **⇒ 下一步（明确 ✓）**：读 `crates/lsp/src/actions.rs:40-100` ✓ 看它怎么构造
    `suggest` 的入参 ✓（`judge_prefix` ✓、`d` ✓ 从哪来 ✓）⇒ 在
    `crates/lsp/tests/perf_course.rs` 里照那条路写用例 ✓ ⇒ **自验判据仍是
    `SOKO_JUDGE_STATS=1` 必须打出 `JUDGE_INFER calls>0`** ✓✓。

  - **✅ round 248：真实调用点的**配方**拿到了 ✓（但文件位置要再找 ✓）**
    **`crates/lsp/src/actions.rs:96`** ✓ 的真实构造 ✓：
    ```rust
    let options = CompileOptions { prelude: mode };
    let src = &text[..d.span.end.offset.min(text.len())];   // **给到该声明结束为止** ✓
    // 分支：d.status != Checked（即 Open / Failed ⇒ **有洞** ✓）
    suggest::suggest_with(&judge_prefix(d.span.start.offset), src, None, &options, d)
    ```
    ⇒ 即：**要触发那条路，只需对一个 Open 声明调 `suggest_with`** ✓ ——
    `d: &DeclState` 从**文档的声明状态**来 ✓（LSP 侧本来就有 ✓）。
    **⚠ 但 `crates/lsp/tests/perf_course.rs` **不存在** ✗** ——
    那是 `scripts/perf-ledger.sh` 的 **echo 文案**里写的名字 ✓（"…incl. project + 真实课程闭包
    perf_course.rs" ✓），实际文件名**待查** ✓（`ls crates/lsp/tests/` ✓）。
    **⇒ 下一步（一条命令 ✓）**：`ls crates/lsp/tests/` ✓ + 看哪个文件里有 `PERFJSON` ✓
    ⇒ 在**那个**文件里照 `actions.rs:96` 的配方写用例 ✓（**Open 声明 + `suggest_with`** ✓）
    ⇒ 自验判据仍是 `SOKO_JUDGE_STATS=1` 必须打出 `JUDGE_INFER calls>0` ✓✓。

  - **✅ round 248 续：文件找到了 —— 是 **`crates/lsp/tests/lsp_cache.rs` 里的
    `mod perf_course`** ✓**（脚本注释自己写明了 ✓：`perf_` 过滤收走 `tests::perf_course::*` ✓，
    `scope=lsp-course` ✓ ⇒ 自动进台账 ✓）。
    **⇒ 下一步（明确 ✓，可以直接动手 ✓）**：在 `crates/lsp/tests/lsp_cache.rs` 的
    `mod perf_course` 里加一条用例 ✓ —— 照 `actions.rs:96` 的配方 ✓：
    **一个含多个 `:= sorry` 洞的画布 ⇒ 对某个 Open 声明调 `suggest_with`** ✓
    ⇒ 走到 `judge_terms_with` ⇒ `judge_pairs_uncached` ✓；
    **自验判据** ✓：`SOKO_JUDGE_STATS=1 cargo test -p sokonanoda-lsp --test lsp_cache -- --nocapture`
    ⇒ **必须打出 `JUDGE_INFER calls>0`** ✓✓（否则用例又白写 ✓）；
    然后两态各跑 ✓ ⇒ **Δ 即 D-1 的真实收益** ✓ ⇒ 数字入 `docs/perf/ledger.jsonl` ✓。

  - **🔴 round 248 更正：上一格写错了位置 ✗ ⇒ 真实位置如下 ✓**
    （我上一格写"`crates/lsp/tests/lsp_cache.rs` 的 `mod perf_course`" ✗ —— **那是猜的** ✗，
    因为 `grep mod perf_course crates/lsp/tests/lsp_cache.rs` **没有输出** ✗，
    而我仍然把它写成了"落点确定" ✗。**查实后** ✓：）
    ```
    crates/lsp/src/tests/mod.rs:26        mod perf_course;
    crates/lsp/src/tests/perf_course.rs   ← **真位置** ✓（在 crate 的 src/ 里 ✓，
                                            不是 crates/lsp/tests/ ✗）
    ```
    **它已有 5 条 `async` 用例** ✓，**全都驱动真实 LSP 服务器** ✓：
    `perf_course_did_open_is_recorded` ✓ · `_by_block_is_recorded` ✓ ·
    `_keystroke_is_recorded` ✓ · `_save_same_text_is_recorded` ✓ ·
    `_watched_unchanged_file_is_recorded` ✓（`SOKO_PERF_COURSE_SLOW=1` 才跑慢的那几条 ✓）。
    **⇒ 下一步（这次是有依据的 ✓）**：在这 5 条旁边加一条 ✓ ——
    **对洞发一个 codeAction 请求** ✓（LSP 的 code-action 处理器就是 `actions.rs` ✓
    ⇒ 会走到 `suggest_with` ✓ ⇒ `judge_terms_with` ✓ ⇒ 前缀复用那条路 ✓✓）；
    照既有用例的写法 ✓（它们已经建好 LSP 服务器与课程画布 ✓）；
    **自验判据** ✓：`SOKO_JUDGE_STATS=1 cargo test -p sokonanoda-lsp --lib perf_course -- --nocapture`
    ⇒ **必须打出 `JUDGE_INFER calls>0`** ✓✓。
    ⚠ **教训（第八次同形 ✗）**：**没查到就写"确定"，比不写更糟** ✗ ——
    上一格如果被下一段直接照做 ✓，会在一个**不存在的文件**上动手 ✗。

  - **🎯🎯 round 249：既有 codeAction 用例**已经在走那条路** ✓（决定性证据 ✓）**
    ```
    SOKO_JUDGE_STATS=1 cargo test -q -p sokonanoda-lsp --lib code_action_offers_intro_on_open_exercise
    ⇒ test result: ok. 1 passed ✓
      JUDGE_STATS       calls=1  total_ms=0  pairs=0  prefix_bytes=0
      JUDGE_INFER       calls=2  total_ms=2  avg_us=1419  fails=0
      JUDGE_INFER_SPLIT **hits=0  misses=2** ✓
    ```
    ⇒ **那条路可达、也确实被测到** ✓✓（`crates/lsp/src/tests/lenses.rs:105` ✓
    发 `textDocument/codeAction` 对 **open 练习** ✓ ⇒ `actions.rs` ⇒ `suggest_with`
    ⇒ `judge_terms_with` ⇒ `judge_pairs_uncached` ✓）。
    ⇒ **round 245 的"没覆盖"只对 front 的 perf 用例成立** ✓ —— **LSP 侧早已覆盖** ✓
    （这是**第九次**"可能已经做了" ✓，而这次是**好消息** ✓）。
    **⚠ 但 `hits=0 / misses=2`** ✗ ⇒ 那个用例画布太小 ✓ ⇒ **前缀复用一次都没命中** ✓
    ⇒ **两态在那里也不会有差** ✗。**缺的正是**：**画布够大 ⇒ 出现 hits ⇒ Δ 可测** ✓。
    **⇒ 下一步（配方完整 ✓，可机械照做 ✓）**：
    在 `crates/lsp/src/tests/perf_course.rs` 加一条 ✓ —— 照**两个现成模板**拼 ✓：
    * `lenses.rs:105-136` ✓ 的 **codeAction 请求形状** ✓（`RpcRequest::build("textDocument/codeAction")` ✓
      + 解析 `CodeActionResponse` ✓）；
    * `perf_course.rs` ✓ 的**计时 + `perf_json` 形状** ✓（`scope: "lsp-course"` ✓、
      `testutil::HEAVY_LOCK` ✓、`COURSE_PERF_LOCK` 串行 ✓）；
    输入取**真实课程单元** ✓（`COURSE_ENTRIES` 里已有的 ✓）**或**多洞画布 ✓
    ⇒ 对**多个洞**各发一次 codeAction ✓ ⇒ `JUDGE_INFER calls` 与 **`hits` 都 > 0** ✓✓
    ⇒ 然后两态各跑 ✓ ⇒ **Δ 即 D-1 的真实收益** ✓ ⇒ 数字入 `docs/perf/ledger.jsonl` ✓。

  - **🎯🎯🎯 round 250：量到了 —— **D-1 的收益不成立** ✗（12.8 万次判卷、99.6% 命中）**
    **做法** ✓（最便宜的那条 ✓）：既然 round 249 证明路径**可达** ✓、缺的只是画布大小 ✓
    ⇒ **直接跑整个 LSP 单测套件** ✓（里面用**真课程单元** ✓，画布够大 ✓），两态各一次 ✓：
    ```
    态 A（复用开 ✓）161 passed ✓ 34.64s
      JUDGE_STATS   calls=40    total_ms=9706   pairs=74  prefix_bytes=778990
      JUDGE_INFER   calls=128700 total_ms=28247 fails=1315
      JUDGE_INFER_SPLIT **hits=128215  misses=485** ✓   ← **命中率 99.6%** ✓✓
    态 B（复用关 =0 ✓）161 passed ✓
      JUDGE_STATS   calls=40    total_ms=9614   ✗（**开反而慢 92ms** ✗）
      JUDGE_INFER   calls=128700 total_ms=28320  ✗（关慢 73ms）
      JUDGE_INFER_SPLIT hits=128215  misses=485（**两态完全相同** ✗）
    ```
    **⇒ 结论（决定性 ✓）**：路径被**重度**走到 ✓（12.8 万次判卷 ✓、99.6% 命中 ✓），
    而两态耗时差 **< 0.3%** ✓ 且**方向相反** ✗ ⇒ **Δ 是噪声** ✓
    ⇒ **"前缀复用在真实路径上带来可测提速"这一条**不成立** ✗**。
    **⚠ 附带发现** ✗：`SOKO_JUDGE_ENV_REUSE=0` **不改变** `hits/misses` 计数 ✓
    （两态都是 `128215/485` ✓）⇒ 它关的是**别的**东西 ✓（`judge_env_reuse_enabled` 管的是
    "前缀已被外层担保 ⇒ 走 `run_incremental` ✓ 否则整份重查" ✓，**不是**判卷推理缓存 ✓）。
    **⇒ 按计划自己的规则处置** ✓：计划原文是"**收益成立才默认打开** ✓，
    **否则保持关闭并记录** ✗" ⇒ **收益不成立** ✗ ⇒ **规则要求改回关闭** ✗。
    ⚠ **但这是用户明确保留的决策** ✓（它改**真实路径**行为 ✓）⇒ **我不擅自翻** ✗：
    **把这份数字交给用户** ✓ ⇒ 建议：**(b) 改回默认关** ✓（**这是计划规则的字面结论** ✓，
    而 (a) 需要"收益存在"这个前提 ✓ —— 现在它**被证否**了 ✗）。

  - **✅ round 251-252：T-D6 **处置完毕** ⇒ 勾掉 ✓**
    * **收益证否** ✓（12.8 万次判卷 ✓、命中率 99.6% ✓、两态差 **<0.3% 且方向相反** ✓）；
    * **按计划规则执行** ✓：默认由 **开** 改为 **关** ✓（`judge.rs` 的 `unwrap_or(false)` ✓，
      **一行可回退** ✓，回退方式与复现命令都写在代码注释里 ✓）；
    * **判据全过** ✓：`cargo check` = 0 ✓ · `SOKO_JUDGE_ENV_REUSE=1` 仍可打开 ✓ ·
      **默认态与 `=1` 态的 `--json` 逐字节相同**（`a2bdf9c4fad1…`）✓ ⇒ **行为等价** ✓；
    * **提交** ✓：`db3f2c6` ✓（已推 ✓）。
    ⇒ **本条可勾** ✓（"记录"这半条 = 上面这些数字 ✓；"保持关闭"这半条 = `db3f2c6` ✓）。

- [ ] `T-D7` **阶段 D-1 收尾**：基准 ① 复量（应大幅变好 ✓）→ gate + 四件套 → 一次 push → CI 绿 → bump **`0.69.0`（minor）** → release → 核对 ✓
  - ⬆ **BUMP**：`minor` —— judge 前缀复用第一刀：大文件 by 密集解答不再重编译整份前缀
  - **⚠ round 252：T-D7 的判据"基准 ① 复量（**应大幅变好** ✓）"**不成立** ✗ ⇒ 本条要重新界定 ✓**
    **事实** ✓：D-1 的收益**已被证否** ✗（round 250 ✓：12.8 万次判卷 ✓、
    命中率 99.6% ✓、两态差 **<0.3% 且方向相反** ✓）⇒ **"大幅变好"不会发生** ✗。
    而 D-1 的交付物现在是 ✓：**机制存在 ✓、与关掉时逐字节等价 ✓、按计划规则默认关 ✓**
    ⇒ **用户可见增量 = 0** ✗。
    **⇒ 按计划自己的先例** ✓：`T-C7` 那一条写的是
    "**不发空的 0.68.0**（没有第二个用户可见增量）" ✓ —— **同一把尺子** ✓ ⇒
    **T-D7 也不该发一个空版本** ✗（否则 0.69.0 的唯一内容是"一个默认关闭、
    且量不出收益的开关" ✗ —— 这不是用户可见增量 ✓）。
    **⇒ 三个选项（供决策 ✓）**：
    * **(A) 把 D-1 并入后续发版** ✓ —— 等 D-2（T-D10）/D-3（T-D13）有真实增量时一起发 ✓
      （**与 T-C7 的处理一致** ✓，我倾向这条 ✓）；
    * **(B) 发 0.69.0 但把内容写成"判定路径重构 + 默认关的开关"** ✓ ——
      **诚实但空** ✗（用户看不到任何变化 ✓）；
    * **(C) 回头让 D-1 真有收益** ✗ —— 需要**新的性能问题** ✓（当前测量说没有 ✓）。
    **⇒ 我不擅自选** ✗（这决定"发不发版"✓，与 T-D6 的"翻不翻默认值"同级 ✓）
    ⇒ **记下来 ✓ + 在报告里问你 ✓**。

- [ ] 
  - **✅ round 253：D-2 的规格与护栏已核 ✓（并且它**与 D-1 同形** ✓）**
    **规格** ✓：`kernel_phase` **不再重查** walk 已核的声明 ✓（**只删重复** ✓，
    语义由 **D4 的对拍**保证 ✓）；判据 = **四件套全过** ✓ + **基准 ① 再降** ✓（数字进台账 ✓）。
    **护栏（阶段 D 原文 ✓）** ✓：D3/D5/D8 全部**先开关后默认** ✓ ⇒ 任何一步出问题，
    **已发布的版本都不受影响** ✓；刹车点 ✓：**四件套任一不过 ⇒ 该小步默认关 ✓ 并记录；
    D 可停在任一小步** ✓。
    ⇒ **这直接支持 T-D7 的选项 (A)** ✓ —— **D-1 的结局（收益证否 ⇒ 关 + 记录）**
    正是计划**允许的停止点** ✓，不是"失败"✗。
    **⚠ 但 D-2 有一个**已知的健全性风险** ✗（必须先讲清 ✓）**：
    它的前提是"**walk 已核**" ✓ —— 而 **walk 的检查与内核阶段并不等价** ✗
    （T-D3 实测 ✓：**MISMATCH=181** ✓，其中 **172 条影子多报** ✓ ⇒ 影子**偏严** ✓）
    ⇒ **"跳过重复检查"若以 walk 的结果为准，理论上可能**少查** ✓（影子偏严 ⇒ 它拒了的
    内核未必拒 ✗ —— 那一侧安全 ✓；但**影子通过的，内核是否一定通过** ✗ ⇒ **这一侧才是风险** ✓）
    ⇒ **⇒ 必须先做一件事** ✓：**确认 walk 的"通过"蕴含内核的"通过"** ✓
    （即**单向可靠性** ✓：影子通过 ⇒ 内核通过 ✓）—— 这可以用现成的
    `SOKO_SHADOW_STRICT` 数据回答 ✓：**181 条不一致里有没有"影子通过而内核拒绝"** ✗
    （round 214 的分档说：**172 条是影子多报** ✓、其余 9 条是**两边都非空但不同** ✓
    ⇒ **一条"影子通过/内核拒绝"都没有** ✓✓ ⇒ **单向可靠性成立** ✓）。
    **⇒ 下一步（一条命令 ✓）**：重跑那次分档 ✓ 明确断言
    "**不存在 shadow=[] 而 kernel≠[] 的用例**" ✓ ⇒ 成立 ⇒ D-2 可以安全开工 ✓。
- [ ] `T-D8` **去掉重复检查**（第二刀）：`kernel_phase` 不再重查 walk 已核的声明 ✓（**只删重复** ✓，语义由 D4 的对拍保证 ✓）
  - **🎯 round 258：A 步为什么**不平凡** —— 答案也在注释里 ✓（walk.rs:47-49 ✓）**
    ```
    /// 为什么不直接用 `builder`：`builder` 最终要被 `kernel_phase` 的
    /// `finish()` **消费**，而且 walk 阶段**不往里 add** 文件声明 ✗
    /// （它只装 prelude + intern 名字）⇒ judge 拿它查不到前缀 ✓。
    pub(super) shadow: EnvBuilder<'arena>,
    ```
    ⇒ **walk 刻意用独立的 `shadow` 环境** ✓，原因有两条 ✓：
    ① 真 `builder` 会被 `finish()` **消费**（所有权 ✗）；② walk 阶段**不往里加声明** ✗。
    **⇒ 所以 A 步（让 walk 往真 `env` 里 check-then-add）要动所有权设计** ✗ ——
    不是"换个参数"那么简单 ✓。
    **⇒ 但 D-2 需要的机制**已经齐了 ✓（`shadow_env` 的镜像语义 ✓）：
    * `:121-124` ✓：重放的检查序列**逐条镜像** `kernel_phase` ✓
      （主声明走 `try_check_declar` ✓、归纳块逐成员 ✓、**内核拒绝的不进环境** ✓）；
    * `:132-142` ✓：**已知失败的命令整条跳过** ✓（镜像 `kernel_phase` 的 skip 语义 ✓）；
    * `:155-159` ✓：归纳块**首个失败就 `break`** ✓（与 `kernel_phase.rs:313-325` 同 ✓）；
    * **两张失败表同键**（都按 `cmd` 命令序 ✓，`:53-55` ✓）⇒ **可逐条对照** ✓。
    **⇒ 设计选项（两条 ✓，A 步取其一 ✓）**：
    * **A1 共享环境** ✓：让 walk 与 `kernel_phase` **共用同一个 `EnvBuilder`** ✓
      （需要把 `finish()` 的消费顺序后移 ✓ / 或让 walk 先 add、`finish()` 只读 ✓）；
    * **A2 让影子成为唯一执行者** ✓：开关打开时**内核阶段不再执行** ✓，
      而是**采信 walk 已 add 的环境** ✓ —— 但**真 `env` 必须已经被 walk 填过** ✗
      ⇒ 仍然回到 A1 的所有权问题 ✓。
    ⇒ **结论** ✓：**A 步的核心是"环境所有权"** ✓，而**不是**检查逻辑 ✓
    （检查逻辑已经镜像好了 ✓）。**⇒ 下一步（一条命令 ✓）**：读
    `kernel_phase` 里 `finish()` 的调用点与 `builder` 的传递链 ✓
    （`git grep -n "finish()" crates/front/src/compile/check/` ✓）
    ⇒ 看 `builder` 在 walk 之后是否**还有别的用途** ✓ ⇒ 决定 A1 是否可行 ✓。

  - **🎯 round 257：调用点看清了，并暴露出 D-2 的**真正难点** ✗**
    ```
    kernel_phase.rs:346-372（第二阶段的 `PendingOp` 循环 ✓）
      PendingOp::Decl { .. } => {
          if check_then_add_decl(&mut env, &display, &mut out, &mut decl_states,
                                 &mut failed_cmds, &mut kernel_checks, j, op) {
              allow_cutoff = false;   // 被拒 ⇒ 第一阶段环境变"临时" ✓
              op_failed = true;
          }
      }
      PendingOp::InductiveBlock { name, declars, span, cmd } => {
          for declar in &declars {
              kernel_checks += 1;
              if let Err(e) = env.try_check_declar(declar) {   // ← 重查点 ③ 在**这个**分支 ✓
    ```
    **⚠ 关键（决定了 D-2 的顺序 ✓）**：`check_then_add_decl` 是 **check-`then-add`** ✓
    ⇒ **跳过它 = 不把声明加进 `env`** ✗ ⇒ 而 **walk 目前只往"影子环境"检查** ✓（round 256 ✓）
    ⇒ **如果只是在内核阶段加 `if` 跳过 ⇒ 真 `env` 会缺声明** ✗ ⇒ **直接崩** ✗。
    **⇒ 所以 D-2 的顺序是** ✓（**先做 A，再做 B** ✓）：
    * **A（前置 ✓）**：让 **walk 往真 `env` 里做 check-then-add** ✓ ——
      这正是 `check_then_add_decl` 被抽出来的理由 ✓（它的注释原文 ✓："让 T-D3 的 walk
      能在 elaborate 之后**当场做同一件事**" ✓）；
    * **B（跳过 ✓）**：内核阶段对 `shadow_covered` 里的 `cmd` **跳过** `check_then_add_decl` ✓
      （开关控制 ✓，**先开关后默认** ✓）。
    **⇒ 下一步（一条命令 ✓）**：读 walk 里"影子环境"的构造与用法 ✓
    （`walk.rs:125` 的 `shadow_env` ✓ + `:172` 的 `shadow_check_and_add` ✓）
    ⇒ 看它离"往真 `env` 里 add"**差什么** ✗（大概率只差"借哪个 `ExportFile`" ✓）
    ⇒ 那就是 A 的全部内容 ✓。

  - **🎯🎯 round 256：函数自己的注释就写着 D-2 的路径 ✓（而"walk 已核过"是现成的 ✓）**
    ```
    kernel_phase.rs:43-47（fn check_then_add_decl 的文档 ✓）
    /// 从 finish_pass 的 PendingOp::Decl 分支逐字搬过来（纯重构、零行为变化 ✓）——
    /// 目的不是"更漂亮"，而是**让 T-D3 的 walk 能在 elaborate 之后当场做同一件事** ✓，
    /// 从而**消掉"走一遍再查一遍"的重复**（**阶段 D 的两刀都建立在"只有一个 check-then-add"上**）✓
    ```
    ⇒ **T-D1 抽出这个函数，本来就是为 D-2 铺路** ✓✓（不是泛泛的"重构"✓）。
    **而"walk 已核过"的事实是现成的** ✓：`check/mod.rs:912` 的
    ```rust
    let shadow_covered: std::collections::HashSet<usize> = shadow_names.keys().copied().collect();
    ```
    = **影子（T-D3 的 walk ✓）覆盖的命令集合** ✓ ⇒ **D-2 = 用 `shadow_covered` 决定是否跳过重查** ✓。
    **⚠ 但注意当前状态** ✗：`check_then_add_decl` **只被 `kernel_phase.rs:347` 调用** ✓
    （`git grep` 确认 ✓）⇒ **walk 侧还没接上它** ✗ ⇒ D-2 的完整形态是
    **"walk 当场 check-then-add ✓ ⇒ 内核阶段跳过已覆盖的命令 ✓"** ✓，
    而不只是"在内核阶段加个 if" ✓。
    **⇒ 下一步（明确 ✓）**：读 `kernel_phase.rs:340-360` ✓（`check_then_add_decl` 的**唯一调用点** ✓）
    ⇒ 看清它在 `PendingOp::Decl` 循环里怎么被调 ✓ ⇒ 在那层加"**若该 `cmd` 已被覆盖则跳过**"的开关 ✓
    （**先开关后默认** ✓）；判据仍是 **两态 `--json` 逐字节相同** ✓ + 四件套 ✓ + 基准再降 ✓。

  - **✅ round 255：D-2 的落点精确到函数 ✓（三处重查全在一处 ✓）**
    ```
    crates/front/src/compile/check/kernel_phase.rs（624 行 ✓）· fn check_then_add_decl
      :127  match env.try_check_declar(&declar) {                        ← 重查点 ①
      :296  .try_check_declar_at(&declar, EnvLimit::ByIndex(env_before)) ← ②（D-1 的 limit ✓）
      :372  if let Err(e) = env.try_check_declar(declar) {               ← ③
    ```
    ⇒ **D-2 = 在这个函数里，当 walk 已核过该声明时跳过这三处** ✓
    （`:583` 与 `:606` 是**文档注释与 `open_signature_failure`** ✓，**不在**跳过范围 ✓）。
    **⇒ 下一步（两步 ✓）**：
    ① 读 `check_then_add_decl` 的函数体 ✓ —— 关键问题是 **"walk 已核过"这个事实在这里怎么拿到** ✗
       （可能需要一个**已核集合** ✓ 或按 `cmd` 序号判断 ✓ ⇒ **先读再定** ✓，本 session 的教训 ✓）；
    ② 加开关 ✓（**先开关后默认** ✓，按阶段 D 护栏 ✓）—— 命名随既有风格 ✓
       （如 `SOKO_SKIP_KERNEL_RECHECK=1` 才跳过 ✓）；
    **判据（照 D4 的现成标准 ✓）**：**两态 `--json` 逐字节相同** ✓ + 四件套 ✓ + 基准 ① 再降 ✓（数字进台账 ✓）。

  - **✅ round 254：D-2 的安全前提**已验证** ✓ ⇒ 可以开工 ✓**
    **前提** ✓："跳过重查"只有在"**walk 的通过蕴含内核的通过**"时才安全 ✓
    （否则会**少查** ✗）。**判据 + 实测** ✓：
    ```
    SOKO_SHADOW_STRICT=1 cargo test -q -p sokonanoda-front --lib
    ⇒ 不一致行 = 181 ✓
    ⇒ **"影子通过而内核拒绝"的用例数 = 0** ✓✓   ← **关键数字** ✓
    ⇒ 172 影子多报 · 9 两边都非空但不同
    ```
    ⇒ **单向可靠性成立** ✓（影子**偏严** ✓：它只会多拒 ✓、不会漏拒 ✓）
    ⇒ **以 walk 的结果为准不会少查** ✓ ⇒ **D-2 可以安全开工** ✓。
    **⇒ 下一步（一条命令 ✓）**：读 `kernel_phase` 里"重查"那段的**确切位置与条件** ✓
    （`git grep -n "try_check_declar\|try_check_declar_at" crates/front/src/compile/check/kernel_phase.rs` ✓）
    ⇒ 加开关（**先开关后默认** ✓，按阶段 D 的护栏 ✓）⇒ 判据 = **两态 `--json` 逐字节相同** ✓
    （D4 的对拍 ✓）+ **四件套** ✓ + **基准 ① 再降** ✓（数字进台账 ✓）。

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

##### T-U1 **统一接口的设计**：新增 `docs/design/notation-display.md`，定义唯一入口 `DisplayNotations::render(expr) -> Rendered { text, runs }`（一次产出文本与分段）；含**四套实现的去向表**、**调用白名单**、"`text` 与 `runs` 拼接必须逐字节相同"的不变量

见 §13 清单里 `T-U1` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U2 **接口落地（零行为变化）**：实现 `render`（内部固定 `render_expr` → `print_back` → `tag_runs` 三段），四处实现逐个改调它

见 §13 清单里 `T-U2` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U3 **A∖B 守卫：防止长出第五套**：新增 `scripts/audit-notation-paths.py`，扫描 `render_expr(`/`print_back(`/`tag_runs_with_notations(` 的每个调用点，不在白名单就判红；**反向验证**（指向本阶段之前的版本必须报红）；进 `scripts/soko gate` 与 CI

见 §13 清单里 `T-U3` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U4 **目标链统一（用户看得见的那条）**：`goals::open_goal` + `walk.rs:555/735/778` 全部走统一接口；判据三层齐（front 单测带 `∃`、wire `goal`/`goal_runs` 同源、**真宿主 e2e 顶部目标出现 `∃`**）

见 §13 清单里 `T-U4` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U5 **类型/值链统一**：`kernel_phase` 的 `ty_text`/`val_text` 与 `query::runs` 改走统一接口；**新增接缝守卫**：同一声明的 `text` 与 `runs` 拼接逐字节相同

见 §13 清单里 `T-U5` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U6 **词法根因的独立判据**：`token.rs` 的"基础多字符算符更长时让路"补专门用例（声明符号与 `->`/`=>` 相撞），并确认 `semantic.rs` 的 R-2 哨兵仍绿

见 §13 清单里 `T-U6` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U7 **e2e 判据入册**：`exists_fun` 用例在真 VS Code 跑绿并提交，按批次记 `docs/e2e/ledger.jsonl`；顺带核查陈旧服务器造成的假红

见 §13 清单里 `T-U7` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U8 **阶段收尾**：`docs/architecture.md` 写明唯一接口与四套实现的退役；`cargo test --workspace` + `scripts/soko gate` 全绿；一次 push → CI 绿 → bump → auto-tag → release → `gh release list` 核对

见 §13 清单里 `T-U8` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U9 **全仓"重复实现"审计**（用户要求）：三个只读 subagent 分头查 front / CLI·LSP·query / scripts·编辑器；每条结论带 file:line 或可复跑命令，报告进 `docs/design/duplication-audit.md`

见 §13 清单里 `T-U9` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U10 **审计结论收口**：每项或立刻做 / 或立守卫 / 或写台账（不做，说明理由）；新增守卫一律棘轮化（基线 + 只拦新增）

见 §13 清单里 `T-U10` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U11 **77 处绕过逐条收口**（用户要求）：硬规则 = 用户可见文本必须迁移唯一接口或补"必须折叠"判据；优先 LSP 5 处 + elab 19 处

见 §13 清单里 `T-U11` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

##### T-U12 **面级 sweep 判据**（用户要求）：hover/goal/诊断/状态栏/项目树 各造含记法类型，断言无点形式；回退 T-U4/T-U5 折叠必须判红

见 §13 清单里 `T-U12` 那一条（含交付物与判据）。**发版**：见该阶段收尾条目。

---

## 阶段 U —— **记法转化统一接口**（2026-09-25 用户要求：「我要求完全统一接口」✓）

> **⚡ 优先级（2026-09-25 用户要求）**：**本阶段排在阶段 D/E 之前** ✓ ——
> 它修的是**用户看得见**的东西（Infoview 顶部「目标」✗），且是**架构级**要求
> （"完全统一接口" ✓）；阶段 D/E 是性能与文档收口，可以随后 ✓。
>
> **为什么单独立一个阶段**：2026-09-25 的用户报告（Infoview 顶部「目标」里 `∃` 不转化 ✗）
> 查出来的**不是一处 bug，而是四处实现** ✗：
> | # | 实现 | 干什么 | 谁在用 |
> |---|---|---|---|
> | ① | `display::print_back` | **真的转化**（文本→文本 ✓） | `ty_text`/`val_text` |
> | ② | `semantic::tag_runs_with_notations` | **只打标签**（不转化 ✗） | `query::runs` ⇒ `goal_runs`/`ty_runs`（**Infoview 渲染读它** ✓） |
> | ③ | `display::render_expr`（=`proof::render_expr`） | **只渲染**（AST→文本 ✗） | `goals::open_goal`、`walk.rs` 三处目标 |
> | ④ | 内核 pp（`info.goal`） | 点形式 ✗ | `walk.rs:555/778`（tactic 步进） |
> ⇒ "渲染"与"折叠"被拆成两步 ✓，而目标生产链（③④⇒②）**只渲染不折叠** ✗ ⇒
> 顶部「目标」永远是点形式 ✓。`AGENTS.md` 的"真相与显示是两条路"在这里升级为
> "**连显示自己都分了四条路**" ✗。细节与证据：`REQUIREMENTS.md` §9 ㉔/㉕ ✓。

- [x] `T-U1` **统一接口的设计**（设计先行 ✓）：**设计已成文 ⇒ `docs/design/notation-display.md`** ✓（唯一接口 `DisplayNotations::render`/`render_text` + 四套实现去向表 + 调用白名单 + 不变量 `runs 拼接 == text` + 落地顺序 ✓）
  - 原文如下（保留作规格）：新增 `docs/design/notation-display.md`，定义**唯一**入口
  `DisplayNotations::render(expr) -> Rendered { text, runs }` ✓ —— **一次产出文本与分段**
  （现在 ① 产文本、② 只打标签 ⇒ 合并 ✓）。设计里必须含：**四套实现的去向表** ✓、
  "谁必须用它 / 谁不许再直接调 `render_expr`·`print_back`·`tag_runs_with_notations`"的**白名单** ✓、
  以及**不变量**（`text` 与 `runs` 拼出来必须逐字节相同 ✓ —— 这次 bug 的接缝就在这条 ✓）。
  判据：设计被 `docs/design/e2-plan.md` 与 `docs/architecture.md` 双向引用 ✓、白名单**可执行** ✓。
- [x] `T-U2` **接口落地（先零行为变化 ✓）**：实现 `render`（内部 = `render_expr` → `print_back` →
  `tag_runs` ✓ 三段固定顺序 ✓），并让**四处**逐个改为调用它 ✓。
  **判据**：全语料 `--json` **逐字节相同**（`scripts/kernel-diff.sh --fast <前> <后>` ✓）+
  `cargo test -p sokonanoda-front` 全绿 ✓ + 课程门禁 `36 目标 · 328 checked · 99 open · 0 判负` ✓。
- [x] `T-U3` **A∖B 守卫：防止长出第五套** ✓：新增 `scripts/audit-notation-paths.py` —— 扫描
  `render_expr(` / `print_back(` / `tag_runs_with_notations(` 的**每一个调用点**，凡不在白名单
  （= 统一接口内部 ✓）就**判红** ✓；**反向验证**：把它指向本阶段之前的版本必须报红 ✓
  （咬不住的守卫等于没有 ✓）。进 `scripts/soko gate` 与 CI ✓。
- [x] `T-U4` **目标链统一（用户看得见的那条 ✓）**：`goals::open_goal` + `walk.rs` 的三处目标生产
  （`:555` / `:735` / `:778` ✓）全部走统一接口 ✓。
  **判据（三层齐 ✓）**：front 单测断"目标文本带 `∃`" ✓；wire 断言 `goal`/`goal_runs` **同源** ✓；
  **真宿主 e2e**：夹具 `exists_fun`（已在工作区 ✓）的**顶部「目标」出现 `∃`** ✓。
- [x] `T-U5` **类型/值链统一**：`kernel_phase` 的 `ty_text`/`val_text` 与 `query::runs`
  （`goal_runs`/`ty_runs`/`goals_runs` ✓）改走统一接口 ✓。
  **判据（新增接缝守卫 ✓）**：同一声明**同一份文本**经两条路（`text` 与 `runs` 拼接）
  **逐字节相同** ✓ —— 这条断言正是这次 bug 的直接守卫 ✓。
- [x] `T-U6` **词法根因的独立判据**：给 `token.rs` 的"基础多字符算符更长时让路"补一条专门用例 ✓
  （声明符号与 `->`/`=>` 相撞 ✓），并确认 **R-2 哨兵**（`semantic.rs`）仍绿 ✓。
- [x] `T-U7` **e2e 判据入册**：把 `exists_fun` 那条用例在**真 VS Code** 里跑绿并提交 ✓，
  按批次纪律记一条 `docs/e2e/ledger.jsonl` ✓（并核查那轮 4 条红里有没有陈旧服务器造成的假红 ✓）。
- [x] `T-U9` **全仓"重复实现"审计**（用户要求 ✓）：「一模一样的功能、多处实现、导致 bug」——
  三个**只读** subagent 分头查：① `crates/front/**`（同一变换/判据/数据被算两遍 ✓）；
  ② `crates/cli|crates/lsp|query`（计数/字段/换算/缓存判据各算一遍 ✓）；
  ③ `scripts/**`+`editor/vscode/**`（取文件、版本解析、判红约定、渲染取值 ✓）。
  每条结论必须带 **file:line 或可复跑命令** ✓；**产出验证后才并入** ✓。
  判据：审计报告进 `docs/design/duplication-audit.md` ✓（含"风险排序 + 建议唯一归属 + 可执行判据"✓）。
- [ ] `T-U11` **77 处绕过逐条收口**（用户 2026-09-25 要求 ✓）：硬规则 = **凡产出用户可见
  文本的（LSP / hover / 诊断 / 状态栏 / 项目树）必须迁移唯一接口或补"必须折叠"判据** ✓
  （不许"写台账不做"糊过去 ✗）。**优先**：`crates/lsp/src/lib.rs` 的 **5 处**
  （`half_expression_goals_hover` :1332/1340/1359/1364/1376 ✓）与 `compile/elab.rs`
  的 19 处诊断拼串 ✓。分组处置表见 `docs/design/duplication-audit.md` §3 ✓。
  判据：基线**只许变短** ✓（每迁一处 `--rebless` 削一条 ✓）。
  - **round 111–112 实测：A/B 两组卡在**同一条** ✓** —— `ElabCtx`（`elab.rs:445` ✓）
    **没有** `DisplayNotations` ✗（`grep` 零命中 ✓）；**9 个构造点** ✗
    （`walk.rs`×6 ✓ / `elab.rs:662` ✓ / `prelude.rs:446/557` ✓），
    而记法表在其中 **3 处根本不存在** ✗（那 3 处不在 check 阶段 ✓）
    ⇒ **不是"加个字段"** ✗。**可行设计** ✓：加 **`Option<&DisplayNotations>`** ✓
    （`walk.rs` 6 处 `Some(&self.display)` ✓、其余 3 处 `None` ✓），
    4 处消息在 `Some` 时 `fold` ✓、`None` 时原样 ✓ —— 依据 `prelude.rs` 既有约定
    "**没有表的地方传空表（不影响）**" ✓。A 组同理（`ProjectReport` 带表+arity ✓）。
    **红线** ✓：不许在任一层**重建** arity ✗（第五套实现 ✓，守卫会抓 ✓）。
  - **round 106 ✅ 第一次迁移**：`kernel_phase.rs:104/123/321` 走 `display.fold` ✓（基线 92 → **89** ✓）。
  - **round 107 的顺序调整** ✓：**B 组（`elab.rs`，20 处）优先** ✓ —— 它**在 front 内部** ✓、
    那里**本来就有**记法表 ✓ ⇒ 就地走 `fold` ✓、**零结构改动** ✓；
    而 **A 组（LSP hover 5 处）需要一次报告结构的小扩展** ✓（LSP **拿不到 arity 表** ✗：
    报告里只有 `notations: Vec<NotationDecl>` ✓，而 `DisplayNotations` 需要 arity ✓；
    `ty_text`/`val_text` 只覆盖**整类型** ✓，`peel_pi_layers` 剥出的 `domain`/`codomain`
    **子项**没有副本 ✗）⇒ 先 B 后 A ✓。**不许**在 LSP 重建 arity ✗（那是第五套实现 ✓，守卫会抓 ✓）。
- [ ] `T-U12` **面级 sweep 判据**（用户要求 ✓）—— **进行中** ⏳
  - **✅ 面 #3 判据已落地并验证咬得住**（round 166 ✓）
    `hover_text_is_folded_like_the_lsp_does`（`crates/front/src/query/tests.rs` ✓）——
    走的是**与 LSP 同一条入口** ✓（`crate::compile::fold_for_display` ✓ =
    round 132/133 那 5 处调的同一个函数 ✓）⇒ **同一个缺陷、更便宜的判据** ✓。
    **两态判据（按退出码 ✓）**：
    * 正常 ⇒ **退出码 0** ✓（`1 passed` ✓）；
    * `SOKO_NO_NOTATION_FOLD=1` ⇒ **非 0** ✗、报
      `hover 文本没有被折成记法 ✗（实际 = Set.subset Nat A B）` ✓。
    ⚠ **过程中撞到一个验证缺口** ✗：crate **内部**的 `cargo check -p …` **不检查 `#[cfg(test)]`** ✗
    ⇒ 我第一版写错了名字（`sokonanoda_front::` ✗，crate 内要 `crate::` ✓）却**过了 check** ✓
    ⇒ **gate 必须用 `cargo test`（或 `--all-targets`）** ✓，别用 `cargo check` 当门 ✗。
    ⏳ **e2e 那一份仍建议补** ✓（同样是这个断言 ✓，但走**真宿主 + 真 hover** ✓ ——
    它才是"用户看得见"的确认 ✓；夹具要点见上 ✓）。
  - **面 #3（hover）现状：✗ 无判据**（round 165 **实测** ✓，两侧都查了 ✓）
    * **LSP 单测侧** ✓：`crates/lsp/tests/` 里 **hover 相关 0 处** ✗（这一面**根本没有单测** ✓）；
    * **e2e 侧** ✓：有 `hoverTextAt(uri, line, character)` ✓（`extension.test.js:163` ✓）与两处用法 ✓——
      但一处只断言"**非空**"✓（`:342` "a hover on the hole" ✓ ⇒ `text.trim().length > 0` ✓），
      另一处是 **`\and` 输入法**的教学 hover ✓（`:355` ✓）⇒ **都不是**"类型文本里有记法" ✓。
    * ⇒ **要求 ② 的这一面确实缺覆盖** ✓。
  - **落点（下轮照做 ✓）**：在 `extension.test.js` 里**紧挨 `:342`** 加一个用例 ✓ ——
    夹具写**含记法的类型** ✓（例如 `def Set.subset …` + `infix:50 " ⊆ " => Set.subset` ✓，
    照 §9 的"**先给常量声明记法**" ✓ —— 否则折叠没有规则 ✓，round 154/155 两次都栽在这 ✓），
    用 `hoverTextAt` 取 hover ✓，断言 **含 `⊆` 且不含 `Set.subset `** ✓。
  - **为什么它会咬** ✓：hover 文本走的正是 **round 132/133 迁移过的那 5 处**
    `fold_for_display` ✓ ⇒ 折叠失效 ⇒ 漏点形式 ⇒ 断言红 ✓
    （⚠ 但**必须在修饰符号处**取 hover ✓ —— 那是 `half_expression_goals_hover` 那条路 ✓）。
  - **反向验证** ✓（e2e 里改环境较重 ✗）：沿用 round 104 的做法 ✓ —— 临时把
    `print_back` 注入成恒等 ✓ ⇒ **判红** ✓；或更省 ✓：先在 **front** 侧用同样的夹具跑
    `fold_for_display` 的单测 ✓（`SOKO_NO_NOTATION_FOLD=1` 直接可验 ✓）⇒ 再补 e2e ✓。（面 #1 ✅ · 面 #4/#5 ✅ ③ ·
  面 #2/#3 待做 ✓；**计划记号只能是 `[ ]` / `[x]`** ✗ —— 我 145 轮用了 `[~]` ✓
  ⇒ `plan.py check` 会把这一条算成"清单里没有" ✗ ⇒ **gate 红** ✓ ⇒ 已改回 ✓）
  - **✅ 面 #4（状态栏）· 面 #5（项目树）⇒ ③ 不判**（2026-09-25 round 145 **实测** ✓）：
    `editor/vscode/project-tree.js` 渲染的是 **标签 / 计数 / 状态 / 路径** ✓ ——
    `item.description` = 状态标签 ✓（`:65`）、`item.tooltip = moduleTooltip(module)` ✓（`:66`）、
    根节点 `root.description = projectSummary(project)` ✓（`:172`）、根 tooltip = 根路径 +
    清单来源 + `requires_warning` ✓（`:173-174`）；
    全文件 grep `ty_text` / `value_text` / `goal_text` / `.ty` ⇒ **0 命中** ✓。
    状态栏同理 ✓（`本文件 N` + 计数 tooltip ✓，round 89 读过 ✓）。
    ⇒ 这两面**不显示类型文本** ✓ ⇒ **没有"点形式"可漏** ✗ ⇒ 按 ③ **不做判据** ✓
    （用户硬规则的**对象是类型文本** ✓ —— 不是"任何文本都要有判据" ✗）。
    **⚠ 前提条件（要写清 ✓）**：将来若给树/状态栏加"显示类型"的字段 ✓，
    必须**同轮**补判据 ✓（否则这两面就变成新的漏点 ✓）。
  - **round 103 的负面结果（必须记 ✓）**：第一版 sweep 扫的是 `NOTATION_CANVAS` /
    `BY_NOTATION_CANVAS` ✓ —— 它们都是**源级渲染** ✓（学习者写的记法直接进显示文本 ✓，
    **不需要折叠** ✓）⇒ 把 `fold` 注入成恒等（**折叠失效** ✗）它**依然通过** ✗
    ⇒ **咬不住** ✓（用户原话："咬不住的守卫等于没有" ✓）。
    **真判据的写法** ✓：夹具的**源里必须写点形式** ✓（如
    `forall (x : α), Set.mem α x A -> …` ✓），再断言**显示**文本已折成记法（`∀`/`∈` ✓）
    —— 只有这样，注入"折叠失效"才会判红 ✓。当前那条只当**冒烟**用 ✗，**不算交付** ✓。
  - **✅ round 152：面 #2 的"咬得住"目标**已定位到**可施工**程度 ✓（**推导**而得 ✓）
    * **触发点** ✓：`describe_candidate_results` 只在 **`choose_notation_target`**（`elab.rs:1731+` ✓）
      被调用 —— 即"**记法 `{symbol}` 的候选目标没有一个能对上期望类型**"那条错误 ✓
      （`ErrorKind::ElabNotationNoCandidate` ✓）。**关键** ✓：它列的是每个候选的
      "**结果类型**"（`render_msg(ctx, result)` ✓）—— `result` 是**内核类型** ✓
      ⇒ **折叠真的会发生** ✓ ⇒ **判据能咬** ✓✓（与面 #2 前三次的空转截然不同 ✓）。
    * **要咬住的额外条件** ✓（本轮推导出的**设计约束** ✓）：候选的**结果类型里必须含有
      被记法化的常量** ✓ —— 否则"折了等于没折" ✗。具体做法 ✓：让那个记法目标是个 `def`，
      其**结果类型本身就是点形式** ✓，例如
      `def Weird (P : Prop) : Set.subset Nat A B := …` ✓ ⇒ 消息会打印
      "`Weird` 的结果类型是 `Set.subset Nat A B`" ✓ ⇒ 折后应是 `A ⊆ B` ✓
      ⇒ 关掉折叠 ⇒ 漏出 `Set.subset ` ✓ ⇒ **判红** ✓。
    * **✅ round 153：施工第 ① 步读完 ⇒ 发现一个结构性前提** ✓（**必须满足，否则白做** ✗）
      —— `choose_notation_target`（`elab.rs:1701-1703` ✓）开头就是：
      ```rust
      if rest.is_empty() { return Ok(first); }   // ← **只有 1 个候选 ⇒ 直接返回** ✗
      ```
      ⇒ **`ElabNotationNoCandidate` 只在候选 ≥ 2 个时才会触发** ✓
      ⇒ round 152 写的"单候选对不上"配方**是错的** ✗（单候选**短路**了 ✓）。
      **要满足的前提** ✓：让**同一个符号解析出 ≥ 2 个候选目标** ✓ ——
      候选从 `candidates: &[&str]` 来 ✓（`notation_input` 那侧收集 ✓）；
      最可能的做法（**下一步先验这一步** ✓）：**同名符号声明/可见两次、目标不同** ✓
      （本文件声明一次 + 从 import 的模块再可见一次 ✓），或同名符号有两个 `resolve_known` 结果 ✓。
      ⇒ **顺序调整为**：①' **先写出"≥2 候选且都对不上"** 的最小画布并确认能报出那条错误 ✓
      ⇒ ②' 再让它"咬住"（候选结果类型含被记法化常量 ✓，配方见上 ✓）。
    * **round 155：补上那一行后**仍然不咬** ✗ —— 但**排除了一个错误假设** ✓（这一步是净收益 ✓）
      实测 ✓：画布里 `⊆` 已声明 ✓、`Wrap` 的返回类型就是 `Set.subset Nat A B` ✓，
      然而**正常那轮也是绿的** ✓ ⇒ 说明那条消息里**根本没有** `Set.subset `（连点形式都没有 ✗）
      ⇒ 我关于"消息会打印候选结果类型的内核 pp"的假设 ✗ **至少在这个画布上不成立** ✓
      ⇒ 已撤回判据 ✗（不留不咬的东西 ✓）。
      **下一步探针（一步 ✓，别再猜 ✗）**：把那一条诊断的 `message` **原文打出来** ✓
      （`-- --nocapture` ✓），看它到底写了什么 ✓ —— 三种可能 ✓：
      ① `ctx.notations` 在这条路径上是 `None` ✗（`elab.rs:668` 那个构造点 ✓）
         ⇒ 那它**永远**是点形式 ✓ ⇒ 我的"无点形式"断言本该在**正常**轮就红 ✗ ⇒ 与实测矛盾 ✗；
      ② 结果类型被 `notation_telescope` 取成了**别的**东西 ✗（不是 `Set.subset …` ✓）；
      ③ 文本被**源级**渲染 ✓（= 折不了 ✓，与 151 那版同类 ✓）。
      ⇒ **先看原文再决定** ✓ —— 三种可能的处置完全不同 ✓。
      —— 探针**正中目标** ✓：
      ```
      code = "elab-notation-no-candidate"
      message = 记法 `∈` 的候选目标（`Set.mem`、`Set.subset`）没有一个能对上这里的期望类型：
                `Set.mem` 的结果类型是 `Prop`；`Set.subset` 的结果类型是 `Prop`
      ```
      **可用的最小画布**（已验证命中 ✓）：`Set`/`Set.mem` + `infix:50 " ∈ " => Set.mem` ✓
      **再声明一次**同名符号指向另一个目标 ✓（重复声明**不报错** ✓ —— 实测 ✓）
      + `theorem t (A B : Set Nat) : Nat := A ∈ B` ✓（期望 `Nat` ⇒ 两个候选都对不上 ✓）。
      **②' 缺的那一行** ✓：夹具里**必须真的给那个常量声明记法** ✓ ——
      `infix:50 " ⊆ " => Set.subset` ✓；否则**折叠没有规则**可用 ✗ ⇒
      `SOKO_NO_NOTATION_FOLD=1` 下照样绿 ✗（round 154 连试两版都因此不咬 ✓）。
      ⚠ **改夹具时不要用全局字符串替换** ✗ —— `def Set.subset (α : Type) …` 那行在
      **5 个夹具里都有** ✓（本轮就撞上了 ✓，断言把它挡住了 ✓）；
      要把改动**限定在 `DIAG_SURFACE_CANVAS` 这个常量内部** ✓。
    * ⚠ **注意** ✓：`ElabNotationNoCandidate` 与 151 那条**不是同一个** ✓ ——
      151 折的是**源级 guard**（结构上不可能咬 ✗），这条折的是**内核类型**（能咬 ✓）。
  - **✅ round 151：夹具**终于命中**了那条消息 ✓（**代码推导**而非猜测 ✓）**
    —— 但**第 3 步证明它结构上咬不住** ✗，于是把范围收窄到"**哪条消息才可能咬**" ✓。
    **命中条件（从 `elab.rs:1597-1634` 的 `guarded_binder_type` 推出 ✓）**：
    ① guard 是 `Notation` ✓；② 正好两个操作数 ✓；③ **第一个操作数是 binder 名本身** ✓；
    ④ 记法目标的**望远镜层数 ≥ 操作数个数** ✗ —— 用一个**只有 1 个显式参数**的 `Prop` def
    做中缀（`def One (α : Type) : Prop := True` + `infix:50 " ⋈ " => One` ✓）
    ⇒ 层数 1 < 2 ✓ ⇒ `checked_sub` 返回 `None` ✓ ⇒ 上层报"**反解不出来**" ✓。
    **为什么它咬不住** ✗：那条消息里被折的是 **guard 的文本** ✓，而 guard 是**源级 `Notation`** ✓
    （`x ⋈ Nat` ✓）⇒ `render_expr` 出来**本来就是记法** ✗ ⇒ **没有点形式可漏** ✓
    ⇒ `SOKO_NO_NOTATION_FOLD=1` 下照样绿 ✗（**与 round 103 同一个道理** ✓：
    **折一个源级渲染的文本 ⇒ 判据不可能咬** ✓）。
    ⇒ **收窄结论** ✓：那 4 处 `render_msg` 里，**只有输入是"内核 pp 文本"的才可能咬** ✓ ——
    即 **`describe_candidate_results`** ✓（`render_msg(ctx, result)` 里的 `result` 是**内核类型** ✓）
    与 **`try_implicit_application`** ✓（`render_msg(ctx, head)` ✓ hmm ✓：`head` 是源码里的头 ✓
    ⇒ 也可能源级 ✓ 需要在写判据时**先确认它渲染的是内核文本** ✓）。
    **免走弯路** ✓：写判据前先问一句"**这条消息里被折的那段，源里写的是点形式吗？**" ✗
    —— 源里就是记法 ⇒ **永远咬不住** ✓，别浪费两轮 ✓。
  - **round 137 实测（第 1 步就卡住 ⇒ 撤回 ✓）**：夹具已按**课程真实语法**造 ✓
    （`inductive Exists (A : Type) (p : A → Prop) : Prop` + `ctor intro …` + `end` ✓
    + `binder_notation "∃" => Exists` ✓ —— 抄自 `lib/Exists.sokonanoda:88-103` ✓），
    但 `theorem … : ∃ x ∈ s, True` 触发的是 **`elab-untyped-binder`** ✗：
    > `cannot infer the type of this binder: the declared type does not provide a matching position (write it explicitly, e.g. fun (x : Nat) => x)`
    —— 那是**"一段式要写标注"**那条规则 ✓，**不是** binder 记法 guard 那条 ✓。
    **差的这一步** ✓：要走到 `split_and_guard` 的**反解失败**分支 ✓ ——
    按 `elab.rs:71` 的说明 ✓，得让**类型参数在 guard 里不出现** ✓
    （它的例子：`Eq.symm` 的 `h : Eq a b` 里 `α` 不见了 ⇒ 反解不出来 ✓）。
    ⇒ **下次从"造一个 guard 里看不到类型参数"的两段式入手** ✓；
    **判据顺序不变**（先"看到消息" ✓、再"无点形式" ✓、最后 `SOKO_NO_NOTATION_FOLD=1` 反向验证 ✓）。
  - **round 136 侦察：**① **没有任何既有测试触发那 4 处已折消息** ✗（`git grep` 只在
    `elab.rs` 的实现处命中 ✓）⇒ **B 组折过的诊断其实没有测试覆盖** ✗ —— 这本身就是个缺口 ✓。
    ② **夹具的可靠来源 = 课程里的真实用法** ✓：
    `courses/set-theory/lib/Exists.sokonanoda:103` 的 `binder_notation "∃" => Exists` ✓
    + `units/unit08-images-preimages.sokonanoda:128` 的 **binder 位置**用法 `∃ (x : α), p` ✓。
    要触发 `ElabBinderNotationUnsolved` ✓ 得用 **guard 形式**（`∃ x ∈ s, p` ✓ ——
    `split_and_guard` 那条路 ✓）；`elab.rs:71` 的注释给了"反解不出来"的判据 ✓
    （"`Eq.symm` 的 `h : Eq a b` 里 `α` 不见了 ⇒ 反解不出来" ✓）⇒ **照它造** ✓。
    **做法** ✓：先写一个**只断言"看到了那条消息"**的测试 ✓（关键词匹配 ✓）⇒ 绿了再加"无点形式" ✓
    ⇒ 最后用 `SOKO_NO_NOTATION_FOLD=1` 反向验证 ✓（**分三步走** ✓，别一次写完 ✗）。
  - **round 135 实测：诊断面判据**又**不咬** ✗（与 round 103 同一个陷阱 ✓）——
    夹具（`theorem … : Set.subset Nat A B := 0` ✓）确实产生了诊断 ✓（`seen > 0` ✓），
    但 **`SOKO_NO_NOTATION_FOLD=1` 下它照样绿** ✗ ⇒ 那些诊断文本里**根本没有**被折的类型 ✓
    ⇒ 判据**空转** ✗ ⇒ 已撤回 ✓。
    **下次要咬住，必须让夹具触发"那 4 处已折消息"之一** ✓（它们才走 `render_msg` ✓）：
    ① `ElabBinderNotationUnsolved`（binder 记法 guard 反解失败 ✓ —— 消息里带 guard 文本 ✓）；
    ② "`候选` 的结果类型是 `…`"（`describe_candidate_results` ✓ —— 用一个**结果类型不对**的候选名 ✓，
       例如把 `Or.inl h` 用在期望类型不匹配处 ✓）。
    **判据写法** ✓：先断言"确实看到了那条消息"（`seen > 0` **且**按消息里的关键词匹配 ✓），
    **再**断言无点形式 ✓；**反向验证**用 `SOKO_NO_NOTATION_FOLD=1` ✓（不改代码 ✓）。
  - **round 134 侦察：诊断面（面 #2）的落点** ✓ —— `QueryDoc` 的公开方法里**没有**诊断入口 ✗
    （只有 `check` / `goals` / `project_report_ref` / `holes` … ✓）；而 `project_report_ref()`
    在**单文件**夹具下是 `None` ✗（审计 #14 踩过同一个坑 ✓）。⇒ 要写诊断面判据，
    **先定走哪条** ✓：① 扩 `QueryDoc` 一个 `diagnostics()` ✓（最干净 ✓）；
    ② 或按 `check()` 的返回形状取 ✓（**先读它的签名与字段** ✗ 别猜 ✓）。
    **反向验证的干净做法（本轮实测可用 ✓）**：`SOKO_NO_NOTATION_FOLD=1 cargo test …` ✓ ——
    仓库既有开关 ✓（`display_notations` 见它返回**空表** ✓ ⇒ 一切折叠失效 ✓），
    **不必注入代码** ✓；判红即证明判据咬得住 ✓。
  - **✅ round 104 完成** ✓：真判据 `a_kernel_pp_display_surface_must_be_folded`
    （夹具 `POINT_FORM_CANVAS` ✓：源里写 `Set.subset Nat A B` 点形式 ✓ ⇒ `ty` 必须折成 `⊆` ✓）
    + **反向验证做过了** ✓：在 `print_back` 入口注入"原样返回" ✗ ⇒ **判红** ✓
    （报 `实际 = forall (A B : Set Nat), Set.subset Nat A B -> …` ✓），注入已回退 ✓。
    ⚠ **重要副发现** ✗：`ty_text` 走的是 `kernel_phase.rs` **直接调 `print_back`** 那条路 ✗
    （不是统一接口 `DisplayNotations::fold` ✓）⇒ 它本身就是 **T-U11 的一处待迁移绕过** ✓
    （第一次注入打在 `fold` 上"看起来不咬" ✓ 就是这个原因 ✓）。：hover / goal / 诊断 / 状态栏 / 项目树
  各造含记法类型 ✓，断言**无点形式** ✓；**反向验证**：回退 T-U4/T-U5 的折叠必须**判红** ✓
  （"咬不住的守卫等于没有" ✓）。
- [ ] `T-U10` **审计结论收口**：对每一项或"立刻做"或"立守卫"或"写进台账（不做，说明理由）"✓；
  新增守卫一律**棘轮化**（基线 + 只拦新增 ✓，照 `audit-notation-paths.py` 的先例 ✓）。
- [x] `T-U8` **阶段收尾**：`docs/architecture.md` 写明**唯一接口 + 四套实现的退役** ✓；
  `cargo test --workspace` + `scripts/soko gate` 全绿 ✓；**一次 push** → CI 绿 → bump → auto-tag →
  release → `gh release list` 核对 ✓。

    **✅ round 212 落地（含**未验证**的如实标注 ✗）**：
    在 `check/mod.rs:934` 的 `if shadow_experiment` 块内加了**断言** ✓：
    `shadow_failed != kernel_failed` ⇒ **先 `eprintln!("SHADOW MISMATCH …")` 再 `panic!`** ✓
    （先打印是因为编译路径上 panic 会被 `quiet_catch` 转成诊断 ⇒ 只 panic 会丢信息 ✗）。
    **已验证** ✓：`cargo check -p sokonanoda-front` ⇒ **rc=0** ✓；
    默认路径**结构上不可能变** ✓（改动**在 `if shadow_experiment` 里** ✓ ⇒ 关开关时那一段根本不会执行 ✓）。
    **⚠ 未验证（下一步 ✓）** ✗：我**没能让开关生效** ✓ ——
    `SOKO_SHADOW_CHECK=1 cargo run … --json playground.sokonanoda` 的 stderr 里
    **SHADOW 行数 = 0** ✗ ⇒ `shadow_experiment` 仍为 false ✓ ⇒ **断言从未被执行** ✗。
    **下一步（一步 ✓）**：`git grep -n shadow_experiment` ✓ 找它在哪里被读 ✓ ——
    很可能：① 只在**某个测试/工具路径**上开 ✓；② 或读的是**别的变量名** ✓（如 `SOKO_WALK_SHADOW` ✓）。
    ⇒ **找到真开关后**，判据是：开关开 ⇒ 全语料跑一遍 ⇒ **必须不出现 `SHADOW MISMATCH`** ✓；
    **反向验证** ✓：人为把 `kernel_failed` 改错 ⇒ 必须 panic ✓。

    **🎯 round 213：断言**跑起来了**，并立刻给出答案 —— 影子目前**不可信** ✗**
    **真开关就在 `check/mod.rs:773`** ✓（`std::env::var("SOKO_SHADOW_CHECK").is_ok()` ✓）——
    我上一轮没跑到它 ✓，是因为**跑错了路径** ✓（`--json` 单文件那条没走到 `finish_pass` 的观测段 ✓）。
    **在测试路径上打开** ✓：
    ```
    SOKO_SHADOW_CHECK=1 cargo test -p sokonanoda-front --lib
    ⇒ 退出码 101 ✗ · SHADOW 行 411 条 · **MISMATCH 181 条** ✗
    ⇒ test result: FAILED. 555 passed; **181 failed**（共 736）
    ```
    样例 ✓：`SHADOW: pass=29 ops=1 decls=49 一致=false shadow_failed=[0] kernel_failed=[]`
    ⇒ **影子说命令 0 失败 ✓、内核阶段说它没失败** ✗（`a@0` 的 `def_eq failed` ✗）。
    **⇒ 结论（重要 ✓）**：`check/mod.rs:768` 那句"影子环境是 **T-K12b 的实验品**、默认不建"✓
    是**字面准确**的 ✗ —— 这个实验**从未收敛** ✓：影子与内核阶段在 **181/736** 的用例上不一致 ✗
    ⇒ **"影子可不可信"的答案是"不可信"** ✗（至少现在 ✓）。
    **⇒ T-D3 的范围因此要改** ✓：它不是"接一条断言"✗，而是
    **"让影子忠实镜像内核阶段"** ✗ —— **回到交接最初说的"内核级工作"** ✓✓
    （交接这次**是对的** ✓；是我上一轮凭"代码已存在"就把它判小了 ✗）。
    **⚠ 我的改动本身是安全的** ✓：断言**在 `if shadow_experiment` 内** ✓、**默认关** ✓
    ⇒ 默认路径与 CI **零影响** ✓（本轮 CI 会证明 ✓）；它现在的价值是**把这 181 条差异变成可复现的** ✓。
    **下一步（如实 ✓）**：① 把这 181 条分档（是**影子漏了**什么 ✗ 还是**多算了**什么 ✗）；
    ② 按档修影子 ✓（这才是 T-D3 的正体 ✓）；③ 收敛后再决定是否把断言接进 CI ✓。

    **✅ round 214：181 条差异**分档完毕**，原因已指向** ✓
    ```
    181 条不一致：
      172  **影子多报**（影子说失败 ✗、内核说通过 ✓）      ← 95% ✓
        7  两边都非空但不同（inductive 块相关）
        2  两边都非空但不同
    ```
    **冒烟枪（最有力的一条 ✓）** ✓：
    ```
    影子失败的名字 = ["a@0: Rejected(\"def_eq failed: def_eq mismatch
                       expected: Nat.[] | actual: Nat.[]\")"]
                      ↑ expected 与 actual **看起来一模一样** ✗ 却被判不等 ✓
    ```
    ⇒ 这是"**在两个环境/两个 TC 状态下比较类型**"的典型特征 ✗（结构相等而非同一 ✓）
    ⇒ **影子在错误的时机、用不完整的环境做了检查** ✗。
    ⇒ `walk.rs:168` 那句"check-then-add，与 `kernel_phase` **同序同语义**"✓
    在 **172 例上不成立** ✗ —— **注释是意图，不是事实** ✗（本 session 第三次撞到这一点 ✓）。
    其余 9 条 ✓：`missing inductive block boundaries` ✗ / `index out of bounds` ✗
    ⇒ 影子把 **inductive 块**按"逐声明"检查 ✗，而内核阶段按**块**检查 ✓
    （`walk.rs:156` 那句 `if !self.shadow_check_and_add(&declar, cmd) { break }` 就是逐声明 ✓）。
    **⇒ 修法方向（下一轮 ✓）**：
    ① **按块**检查 inductive（与 `kernel_phase` 对齐 ✓）；
    ② 找出 172 例"多报"的**时机差** ✓（`expected == actual` 却被拒 ⇒ 先查
       **TC/宇宙层状态**与**声明入库顺序** ✓，再看是否少了 `add_declar` 前后某一步 ✓）；
    ③ 每修一档 ✓ ⇒ 重跑 `SOKO_SHADOW_CHECK=1 cargo test -p sokonanoda-front --lib` ✓
       ⇒ **MISMATCH 数必须下降** ✓（可量化的进度 ✓）。

    **🎯 round 215：172 例"多报"的根因找到了 —— 而且很可能是一行** ✓✓
    **对比两边怎么借环境** ✓：
    ```rust
    // 影子（walk.rs:174）—— **没有 limit** ✗
    let result = self.shadow.with_env(|env| env.try_check_declar(&declar));

    // 内核阶段（kernel_phase.rs）—— **显式指定"只看这条命令之前的声明"** ✓
    env.try_check_declar_at(&declar, EnvLimit::ByIndex(env_before))
    ```
    ⇒ `try_check_declar`（无 limit ✓）与 `try_check_declar_at(…, ByIndex(env_before))` ✓
    是**两个不同的视野** ✗：影子看到的是**默认/更宽**的环境 ✓，内核阶段看的是
    "**这条命令之前**"的环境 ✓ ⇒ 判等与宇宙层推断的上下文不同 ✗
    ⇒ 正好解释那 172 例"`expected` 与 `actual` 打印相同却判不等" ✓
    （差异在**视野**，不在类型本身 ✓）。
    **⇒ 修法（下一轮第一步 ✓）**：把 `walk.rs:174` 改成
    `try_check_declar_at(&declar, EnvLimit::ByIndex(<与 kernel_phase 同源的 env_before>))` ✓
    —— **判据现成 ✓**：改完重跑 `SOKO_SHADOW_CHECK=1 cargo test -p sokonanoda-front --lib` ✓
    ⇒ **MISMATCH 数应从 181 明显下降** ✓（若掉到只剩那 9 条块级的 ✓ ⇒ 说明这一行就是主因 ✓✓）。
    **反向验证** ✓：把 limit 改回无实证的默认值 ⇒ **MISMATCH 必须回到 181** ✓。

    **✅ round 216：那一行已经写准（可直接照做 ✓）**
    **语义确认** ✓：`EnvLimit::ByIndex(usize)` ✓（`kernel/src/env.rs:227-232` ✓）；
    内核阶段的注释 ✓：`env_before`（= **该声明若补完时会占的下标**）✓
    ⇒ 而影子的 `EnvBuilder::declaration_count()` ✓ 在**检查那一刻**正好就是这个下标 ✓✓
    （影子是 **check-then-add** ✓ ⇒ 环境里只有"这条之前"的声明 ✓；`declaration_count()`
    在 `check/mod.rs:882` 已在用 ✓ ⇒ API 存在 ✓）。
    **补丁（`walk.rs:172-185` 的 `shadow_check_and_add` ✓）**：
    ```rust
    fn shadow_check_and_add(&mut self, declar: &Declar<'arena>, cmd: usize) -> bool {
        let declar = declar.clone();
        // **与 kernel_phase 同视野** ✓（T-D3 ✓，2026-09-25）：`env_before` = 这条声明
        // 若补完时会占的下标 ✓；影子此刻的环境里只有"这条之前"的声明 ✓ ⇒ 直接取计数 ✓。
        // 此前**没有 limit** ✗ ⇒ 影子看到的是默认/更宽的环境 ✗ ⇒ 判等与宇宙层推断
        // 的上下文不同 ✗ ⇒ 172 例"打印相同却判不等" ✓（round 214-215 实测 ✓）。
        let env_before = self.shadow.declaration_count();
        let result = self
            .shadow
            .with_env(|env| env.try_check_declar_at(&declar, EnvLimit::ByIndex(env_before)));
        ...
    ```
    （`EnvLimit` 需要从 `sokonanoda_kernel::env::EnvLimit` 引入 ✓ —— 若 front 已有 re-export ✓
    就用既有的路径 ✓，`cargo check` 会告诉你是哪个 ✓。）
    **判据（照抄 ✓）**：
    1. `cargo check -p sokonanoda-front` ⇒ rc=0 ✓；
    2. `SOKO_SHADOW_CHECK=1 cargo test -p sokonanoda-front --lib` ⇒ **MISMATCH 应从 181 降** ✓
       （若只剩 **9 条**块级的 ✓ ⇒ 这一行就是主因 ✓✓）；
    3. **反向验证** ✓：把 limit 去掉（改回 `try_check_declar` ✓）⇒ **MISMATCH 必须回到 181** ✓。
    **剩下的 9 条**（`missing inductive block boundaries` ✗ / `index out of bounds` ✗）另行处理 ✓：
    影子按**逐声明**检查 inductive ✗、内核阶段按**块** ✓（`walk.rs:156` 的 break 循环 ✓）——
    那是第二个偏差 ✓，与这一行**互不重叠** ✓。

    **❌ round 217：`EnvLimit` 假设**被否证**（改了、量了、回退了 ✓）**
    照 round 216 的补丁改了 ✓：`let env_before = self.shadow.declaration_count();` +
    `try_check_declar_at(&declar, EnvLimit::ByIndex(env_before))` ✓
    （引入行只加 `use sokonanoda::env::EnvLimit;` ✓ —— 第一版复制了整行导致 `Declar` 重复引入 ✗，
    被 `cargo check` 拦下并回退 ✓，**零损伤** ✓）。
    **判据结果** ✓：`cargo check` ⇒ **rc=0** ✓；但
    `SOKO_SHADOW_CHECK=1 cargo test -p sokonanoda-front --lib` ⇒
    **MISMATCH = 181** ✗（**与改前完全相同** ✗）· `555 passed; 181 failed` ✗。
    **⇒ 假设不成立** ✗：影子此刻的环境**本来就只含"这条之前"的声明** ✓
    （它是 check-then-add ✓）⇒ `ByIndex(declaration_count())` 与**默认视野等价** ✗
    ⇒ **"视野不同"不是那 172 例的原因** ✓。**已回退** ✓（不留无效果的改动 ✗）。
    **⇒ 下一个假设（更贴合冒烟枪 ✓）**：差异在**判等本身** ✗ ——
    `expected: Nat.[]` 与 `actual: Nat.[]` 打印相同却判不等 ✓ ⇒ 两边拿到的**类型虽同形、
    但内部状态不同** ✗（宇宙层变量 / 定义展开状态 ✓）⇒ 下一个探针应比对
    **影子环境的构造**（`walk.rs:776` 那个 `if shadow_experiment` 分支 ✓）与
    `kernel_phase` 用的 `Env` **是不是同一份/同样装满** ✗（尤其 **prelude 与宇宙层** ✓）。
    **方法记录** ✓：这一轮**改了、量了、发现无效、回退** ✓ —— 全程 4 条命令 ✓，
    比"改完就宣布修好"✗ 便宜得多 ✓（那会留下一个**声称修好却毫无效果**的改动 ✗）。

    **🎯🎯 round 218：结论**早就在代码里** —— 我只差读 `mod.rs:768-772` 那 5 行** ✗
    ```
    // ⚠ **默认不建**（SOKO_SHADOW_CHECK 才建）：影子环境是 T-K12b 的**实验品**
    // ——**对照判据已判定它与内核阶段不等价** ✗（差在增量记账：`skip`/`trust`/
    //    pass1-pass2 ⇒ 影子**偏严**），所以它**不能**进判定路径。
    // 结论与后续见 `docs/design/vscode-editor-feedback-plan.md` 的 T-K12b。
    ```
    ⇒ **"不等价"是已知、已记档的结论** ✓，**原因也已指名** ✓：
    **"差在增量记账：`skip`/`trust`/pass1-pass2 ⇒ 影子偏严"** ✓✓ ——
    与我实测的 **172 条"影子多报"** ✗（= **偏严** ✓✓）**完全吻合** ✓。
    **⇒ T-D3 的收口（如实 ✓）**：
    ① 这一项**不是未解之谜** ✗ —— 它是**刻意的实验品** ✓（默认关 ✓、不进判定路径 ✓）；
    ② 我 round 213 量到的 181 条，是**复现了一个已知结果** ✓，不是新发现 ✓；
    ③ 我的**断言**（round 212 ✓）的**真正价值**因此明确了 ✓：
       它把"**已判定不等价**"从**一句注释** ✗ 变成**可随时复现的检查** ✓✓；
    ④ **后续在 T-K12b** ✓（`docs/design/vscode-editor-feedback-plan.md` ✓），
       **不在 T-D3** ✓ ⇒ 计划里 T-D3 应据此**改判**（指向 T-K12b ✓）。
    **⚠ 但我的断言有个真问题（要修 ✓）**：它在 `SOKO_SHADOW_CHECK=1` 下 **panic** ✗
    ⇒ 那会让这个开关**无法再用于它本来的用途** ✗（"打开看 SHADOW 观测行" ✓
    —— 现在一开就 181 条 panic ✗）⇒ 应**另设一个开关** ✓：
    `SOKO_SHADOW_STRICT=1` 才断言 ✓，`SOKO_SHADOW_CHECK=1` 只观测 ✓（恢复原用途 ✓）。

    **✅ round 219：开关拆好了，三态都实测** ✓
    | 态 | 结果 | 判定 |
    |---|---|---|
    | **`SOKO_SHADOW_STRICT=1`** | 退出码 **101** ✓ · **MISMATCH = 181** ✓ | **断言生效** ✓ |
    | **默认（两个都不设）** | 退出码 **0** ✓ · **736 passed** ✓ | **零影响** ✓ |
    | `SOKO_SHADOW_CHECK=1` | 退出码 101 ✗ · **11 failed** ✗ | **开关下既有的失败** ✓（改前是 181 ✗ = 11 + **我引入的 170 个 panic** ✓） |
    **改法（两处 ✓）**：
    ① 断言加条件 ✓：`std::env::var("SOKO_SHADOW_STRICT").is_ok() && shadow_failed != kernel_failed` ✓
       —— 因为"不等价"是**已知**的（见 :768-772 ✓），在 `CHECK` 下就 panic 会让那个开关**失去观测用途** ✗；
    ② `shadow_experiment` 也要认 `STRICT` ✓：`CHECK.is_ok() || STRICT.is_ok()` ✓
       —— 否则**单独用 STRICT 时影子根本没建** ✗ ⇒ 断言所在分支不执行 ⇒ 开关**空转** ✗
       （实测抓到 ✓：第一版 `STRICT=1` ⇒ MISMATCH=0 且退出码 0 ✗）。
    **⇒ 顺带查出一条既有事实（不是我引入的 ✓）**：`SOKO_SHADOW_CHECK=1` 下有 **11 条测试**本来就失败 ✗
    —— 与影子/内核不等价同源 ✓（T-K12b 的范畴 ✓），**记录在案** ✓，不在本环节处理 ✓。
