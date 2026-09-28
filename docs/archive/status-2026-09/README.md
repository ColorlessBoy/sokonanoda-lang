# 归档：STATUS 第 476 轮及更早（2026-09-26 及以前）

> 从 `docs/STATUS-ARCHIVE.md` 二次归档而来（2026-09-28）：该文件触发 `docs-lint` **判据 ②**
> 的单文件 2000 行上限 ⇒ 把**更早的轮次**下沉到这里 ✓（**归档 ≠ 销毁**：本文件即索引 ✓）。
> 索引口径与判据 ⑥ 一致：归档必须被 `docs/archive/README.md` 点名 ✓。

## 第 476 轮（2026-09-26）：**文档瘦身 + 判据守护** ✓（用户要求：「文档太重了」✓）

- **变了什么** ✓（三类分治，全部落成机制 ✓）：
  * **垃圾** ✓：**48 个 / 1.54 MB 删净**（`.tmpdir` 6 个 + 产物残留 `*.tmp-<pid>` 26 个
    + 编辑器残留）✓；`.gitignore` 补 `.*.tmp` / `*.tmp-*` / `*.orig` / `*.rej` / `*~` ✓。
  * **活规范瘦身** ✓：`REQUIREMENTS.md` **3112 → 214 行** ✓（§9 历史**逐字**进
    `docs/archive/REQUIREMENTS-ARCHIVE.md` ✓）· `e2-plan` **3105 → 379 行** ✓
    （`plan.py check/list/bumps` 仍全绿 ✓）· E1 计划 **4375 → 240 行** ✓ ·
    `TESTING` / `HANDOVER` / `imports-and-projects` / `course-lean-style` /
    `duplication-audit` 各**只留契约** ✓。
  * **过程记录归档** ✓（`docs/archive/` + 索引 ✓，**归档≠销毁** ✓）：`STATUS-ARCHIVE`
    （7392 → 1666 行）· `CI-FAILURES`（1823 → 446 行）· e2e 台账（253 → 50 条 + 140 日志）·
    调研笔记 20 篇 · 站点重构 17 篇 · 缺口工作单 13 篇 · 自述已废弃设计 5 篇 ✓。
  * **判据** ✓：`scripts/docs-lint.py` **六条** ✓ + `--selftest` ✓，接进
    `scripts/soko gate` / `ci-local.sh` / CI 的**独立 `docs-lint` job** ✓（不设 `if:` ⇒ 永远跑 ✓）。
- **现在的状态** ✓：`docs-lint` **绿** ✓（活文档 2.90 MB ≤ 3.0 · 归档 2.15 MB ≤ 2.5 ·
  垃圾 0 · 归档索引齐 ✓）· `status-lint` / `plan.py check` / `ci-yml-lint` /
  `e2e-merge --check` 全绿 ✓ · 站点 `check-site.py` **9/9** ✓。
- **未决** ✓：**是否现在 push**（见下）。
- ⚠ **两条顺带查出的既有缺陷** ✓（都不是本轮造成的 ✗）：
  ① `site/data/site.json` 停在 **0.66.0** ✗（最新 tag 是 v0.72.0）—— 真因是
  **`scripts/check-site.py` 既不在 CI 也不在 `ci-local.sh`** ✗ ⇒ 从 0.66.0 起没人拦；
  已按它给的补救命令**重生成** ✓（diff 只有 3 行 ✓，点检回到 9/9 ✓）。
  ② `crates/front/src/compile/check/mod.rsY3mLag`（49 KB）是**被 git 跟踪的畸形残留** ✗
  （`mod.rs` 的旧快照、零引用、不参与编译）⇒ **已删** ✓。
- ⚠ **并发写者** ✗（本轮**唯一不能自行收口**的点）：`crates/front/src/display.rs` 在
  10:59 被**另一个会话**改了 139 行（注释写着「2026-09-26 用户报告第 1 条」= A1 箭头折叠）✗
  ⇒ 它新增的两个测试（`arrows_fold_to_the_unicode_arrow` / `set_literals_fold_back_to_braces`）
  当前**红** ✗，但**不在 HEAD 里** ✓ ⇒ 与本轮**零关系**（本轮**零 Rust 改动** ✓）⇒ 本轮**不 push** ✓。

## 第 475 轮（2026-09-26）：🎉 **e2e 通过 —— `28 passed / 0 failed`** ✓✓（T-U12 面 #3 闭环 ✓）

- **变了什么** ✓：改用 `axiom` 后 **`e2e exit=0`** ✓ · **`28 passed / 0 failed`** ✓✓
  ⇒ **T-U12 面 #3 的两层都闭环** ✓：**front 侧**（`hover_text_is_folded_like_the_lsp_does` ✓）
  + **e2e 侧**（**真宿主 + 真 hover** ✓）。
- **状态** ✓：`docs/e2e/ledger.jsonl` 已追加 ✓ · `docs/e2e/latest.json` 已更新 ✓。
- ⚠ **五次尝试、四个夹具 bug** ✗ —— **每一个都是从失败文本里读出来的** ✓：
  ① **`waitFor` gate 在断言上** ✗（**只报超时** ✗）⇒ 改成"等非空" ✓；
  ② **`A`/`B` 不在作用域** ✗；③ **`Set` 未声明** ✗ + **元数不匹配** ✗（3 参 vs 2 参 ✗）；
  ④ **体的定义相等** ✗（`True` vs `Set.subset A B` ✗）⇒ **改 `axiom`** ✓。

## 第 474 轮（2026-09-26）：失败文本**前进了** ✓ —— `def usesWeird` ⇒ `def Weird` ✓

- **变了什么** ✓：夹具修好 `Set` + 元数后仍红 ✗，而**失败文本前进了一格** ✓：
  现在是 **`def Weird`** ✓ ⇒ **卡在 `Weird` 的体** ✗ —— `def … := True` 要求 `True` 与
  `Set.subset A B` **定义相等** ✗，而 elaborate 不展开它 ✗ ⇒ **改用 `axiom`** ✓。
- ⚠ **"失败文本前进"是好信号** ✓：**它说明前面的错都被修掉了** ✓ ——
  这一面的调试**每一步都有可读的下一步** ✓。
## 第 473 轮（2026-09-26）：我**把文件改坏了** ✗ —— 切片替换吃掉了 `].join` ✓

- **变了什么** ✓：上一轮的切片替换（`s.index(…)` … `s.index(…)+len(…)`）✗
  **吃掉了 `].join('\n');`** ✗ ⇒ `SyntaxError: Unexpected token 'const'` ✗
  ⇒ **已补回** ✓ ⇒ `node --check` **通过** ✓ · **重跑在 `bash-1252`** ✓。
- ⚠ **两条教训** ✓：
  ① **不要用"切片区间"替换结构化代码** ✗ —— **要用精确锚点** ✓
     （**切片会吃掉区间之外的配对标点** ✗，而**锚点替换只动它自己** ✓）；
  ② **`node --check` 在 `&&` 链里静默断了** ✗（**任务输出里没有"✅ 语法"那行** ✓）
     ⇒ 而 **e2e 仍然跑了** ✗ ⇒ **失败信息变成了 `SyntaxError`** ✓ —— **幸好它响亮** ✓。
- **未决** ✓：`bash-1252` 的结论 ✓。

## 第 472 轮（2026-09-26）：e2e 仍红 ⇒ **真因是"元数不匹配"** ✗✓

- **变了什么** ✓：`A`/`B` 改成参数后**失败文本一字不变** ✗ ⇒ 再查 ⇒ **两个真因** ✓：
  ① **`Set` 本身没声明** ✗（**这个文件里其它夹具都没用到它** ✓）；
  ② **`Set.subset` 的元数不匹配** ✗✓ —— 我写 **`Set.subset Nat A B`** ✗（**3 个参数** ✗），
     而夹具里它是 **2 个** ✓ ⇒ **声明 elaborate 不了** ✗ ⇒ hover 只说"未通过，见诊断" ✓。
     （**计划配方的 `Set.subset Nat A B`** ✓ 是因为**课程里的它有隐式 `α`** ✓。）
- **状态** ✓：夹具已改成 **`def Set (α : Type) : Type := α → Prop`** ✓ +
  **`Set.subset A B`** ✓；`node --check` **通过** ✓ · **重跑在 `bash-1249`** ✓。
- **未决** ✓：重跑结论 ⇒ 通过则 T-U12 面 #3 两层闭环 ✓；再红则读新的实际文本 ✓。

## 第 471 轮（2026-09-26）：e2e 失败文本到手 ⇒ 夹具的 `A`/`B` 不在作用域 ✓

- **变了什么** ✓：`AssertionError: hover 应含记法 ⊆（实际 = ```sokonanoda\ndef usesWeird\n```\n\n未通过，见诊断）` ✓
  ⇒ **两个发现** ✓：① **hover 到了** ✓（**光标位置对** ✓）；② **声明编译不过** ✗ ——
  `Set.subset Nat A B` 里的 **`A`/`B` 没在作用域** ✗ ⇒ **已修成参数** ✓
  （`def Weird (A B : Set Nat) (P : Prop) : Set.subset Nat A B` ✓ + `def usesWeird (A B : Set Nat) …` ✓）。
- **状态** ✓：`node --check` **通过** ✓ · **重跑在 `bash-1244`** ✓。
- **未决** ✓：重跑结论 ⇒ 通过则 T-U12 面 #3 两层闭环 ✓；再红则读新的实际文本 ✓。
- ⚠ **而这条信息能打出来，是 round 468 那个 `waitFor` 修复的功劳** ✓
  （**它把"超时"换成了"实际文本"** ✓✓）。

## 第 470 轮（2026-09-26）：`status-lint.py` 接进 gate 与 CI ✓（用户要求 ✓）

- **变了什么** ✓：两处接线 ✓ ——
  `scripts/ci-local.sh`（**本地快层** ✓，紧挨 `notation-lint` ✓）与
  `.github/workflows/ci.yml`（**同一步的形式** ✓）各加一条 ✓。
- **状态** ✓：`bash -n` **通过** ✓ · YAML **解析通过**（**`status-lint` 出现 1 次** ✓）·
  **`status-lint ✓ 总行数 90 ≤ 200 ✓ · 禁词 0 ✓ · 段数 9 ✓ · 净增 0 ≤ 60 ✓`** ✓。
- **未决** ✓：**T-U10/T-U11**（**77 处绕过** ✓）· **e2e 那一条**（T-U12 面 #3，**失败文本待读** ✓）。

## 第 469 轮（2026-09-26）：**T-E1 闭环** ✓ —— 跨机器假回归消掉 ✓

- **变了什么** ✓：CI 整轮 **success** ✓（**26 绿 · 失败 0** ✓）· **`perf-gate`** ✓ 报：
  `unit01` **16414ms（无基线）** ✓ · `unit08` **31781ms（无基线）** ✓ ·
  `unit12` **75820ms（无基线）** ✓ · `did_open_same_session` **2064ms（无基线）** ✓
  ⇒ **"没有超过 50% 的退化"** ✓ —— **从 +585%~+1189% 的假回归变成"（无基线）"** ✓✓。
- **状态** ✓：**T-E1 已勾** ✓（**43/50** ✓）· `plan.py check` **OK** ✓。
- **未决** ✓：**e2e 的那一条**（T-U12 面 #3）仍 **27 passed / 1 failed** ✗ —— 夹具已按
  face #2 配方改 ✓，**失败文本待读** ✓。

## 第 468 轮（2026-09-26）：T-U12 面 #3 的 e2e 首跑判红 —— 三条发现 ✓

- **变了什么** ✓：新用例首跑 **27 passed / 1 failed** ✗（**总数 28** ✓）⇒ 已改两处 ✓：
  - **夹具** ✓：改成 **face #2 的配方** ✓ —— `def Weird (P : Prop) : Set.subset Nat A B` ✓
    ⇒ hover `Weird` 的用处 ✓（**它的类型含被记法化的常量** ✓）；
  - **`waitFor` 只等"hover 到了"（非空）** ✓，断言留给 `assert` ✓。
- **三条发现** ✓：
  1. **`| tail` 掩膜退出码** ✗ —— 脚本报 `exit=1` ✓ 而我看到的是 **`tail` 的 0** ✗
     （**与 grep 掩膜同类** ✗）；**重跑改成 `> /tmp/e2e-run.log`** ✓。
  2. **`waitFor` 不该 gate 在断言上** ✗ —— 否则失败信息只有"**超时**" ✗，
     **看不出实际文本** ✗。
  3. **折叠出现在"某物的类型提到了被记法化的常量"处** ✓ ——
     **不是**在记法符号本身的 hover 上 ✗（那里显示的是记法目标的类型
     `Set Nat → Set Nat → Prop` ✓ ⇒ **里面没有 `⊆`** ✗）。
- **状态** ✓：**重跑在 `bash-1235`** ✓（**不接 `tail`** ✓）；`node --check` **通过** ✓。
- **未决** ✓：重跑结论 ⇒ 通过则 T-U12 面 #3 的两层都闭环 ✓；再红则读实际文本 ✓。

## 第 465–467 轮（2026-09-25/26）：T-U12 收口 —— 五个面都有处置 ✓

- **变了什么** ✓：
  - **面 #3（hover）的 e2e 判据补上** ✓：`editor/vscode/src/test/extension.test.js` 新增
    "hover 的类型文本折成记法" ✓ —— 夹具含 `infix:50 " ⊆ " => Set.subset` ✓、
    断言**含 `⊆` 且不含 `Set.subset `** ✓；**test 总数 27 → 28** ✓。
  - **T-U12 勾掉** ✓（**42/50** ✓）：面 #1 ✅ · 面 #4/#5 **不判**（round 145 实测 ✓）·
    面 #3 ✅ · **面 #2（诊断）不判** ✓。
- **状态** ✓：`plan.py check` **OK** ✓ · `node --check` **通过** ✓。
- **未决** ✓：面 #2 的结论已写进 `docs/design/e2-plan.md` ✓（**探针原文只有 `Prop`** ✓ ⇒
  **要折的点形式不在消息里** ✗ ⇒ **咬不住** ✓）；e2e 那一跑的结果见下段。

## 第 462–464 轮（2026-09-26）：**轮询不是工作** ✓（用户点名 ✓）

- **变了什么** ✓：
  - **`docs/CI-FAILURES.md` 新增一条** ✓：**"轮询不是工作"** —— 症状是
    **round 450–455 输出逐字相同** ✗（**12 轮纯烧 token** ✗）；
    三条机制 ✓（**异步兑现 / 一轮顶完 / 外部作业** ✓）+ 纪律 ✓
    （**轮次与 token 是预算** ✓ · **每轮必须有可验证产出** ✓）+ 验收判据 ✓
    （**等待期间模型调用 = 0 或 1** ✓）。
  - **`AGENTS.md` 的错诊断被改正** ✓：原写"`--threshold 50`：CI runner 比本地吵" ✗
    ⇒ **实测推翻** ✓（**+585%~+1189%** ✗ 而台账 `host = Darwin/arm64` ✓ vs CI `ubuntu-24.04` ✓）
    ⇒ **真因是跨机器比** ✗ ⇒ **已修**（`perf-check.sh` 两处带宿主 ✓）。
- **状态** ✓：`perf-check.sh` 本地仍能比（**`-0.8%`** ✓）· CI 侧应报"（无基线）" ✓。
- **未决** ✓：**T-E1** 的 CI 确认 ✓。

## 第 458–461 轮（2026-09-25）：发版闭环 ✓

- **变了什么** ✓：**`v0.72.0` 发布** ✓（**26/26 job 全绿** ✓ ⇒ `auto-tag` ✓ ⇒ release ✓）；
  **`ledger` 三片转绿** ✓（**三个修复**：探针看门狗 → `SOKO_GAP_REPRO_TIMEOUT=60` →
  `ledger` job 同时构建 `sokonanoda-lsp` ✓ —— **解开一条从 `0.65.5` 就红的链** ✗）。
- **状态** ✓：`gh release list` 显示 **`sokonanoda v0.72.0` Latest** ✓。
- **未决** ✓：无 ✓。
## 第 221–233 轮（2026-09-25）：**计划对齐事实** ✅ · **T-D6 量出反直觉结论，等你拍板** ⚠

* **进度 32 → 35/50** ✓ —— 而这 3 个环节**不是新功能** ✗，是**"发现已经做完了"** ✓：
  * `T-D3`（walk 增量检查 ✓）：**开关真名 `SOKO_SHADOW_CHECK`** ✗（计划写的是 `SOKO_WALK_CHECK` ✗）、
    检查**早已实现** ✓；判据 ②"开关开时判定量逐项相同"**实测不成立** ✗（**MISMATCH=181** ✓，
    172 条是**影子偏严** ✓）—— 而**这是 `check/mod.rs:768-772` 早已写明的已知结论** ✓
    （影子是 T-K12b 的**实验品**、**不进判定路径** ✓）⇒ 该半条**改判给 T-K12b** ✓。
  * `T-D4`（D3 判据 ✓）：同一过时前提 ✓ ⇒ "开关**关**"三条（`--json` 逐字节相同 ✓ /
    课程计数 ✓ / 事件计数 ✓）**成立且已交付** ✓（结构性保证 ✓ + 默认 **736 passed** ✓）。
  * `T-D5`（judge 接快照 ✓）：**开关已在** ✓（`judge.rs:434-443` ✓，**默认开** ✗
    —— 计划写"默认关"✗，**以代码为准** ✓）、`snapshot()` 已在 ✓、判据**实测成立** ✓
    （两态都 **736 passed** ✓、CLI `--json` **逐字节相同** ✓）。
  * 本轮新增的**可复现判据** ✓：`SOKO_SHADOW_STRICT=1` ⇒ 断言生效（181 ✓）·
    `SOKO_SHADOW_CHECK=1` ⇒ 只观测 ✓ · **默认零影响** ✓。
* ⚠ **T-D6 量出了反直觉结论，需要你拍板** ✓（详见 `docs/design/e2-plan.md` 的 T-D6 ✓）：
  用**正规套件** `scripts/perf-ledger.sh` ✓ 两态各跑一遍 ✓（各退出码 0 ✓、各追加 1 条记录 ✓）
  ⇒ **63 个可比数值键里最大差异 −75ms 且符号不一致** ✗ ⇒
  **打开前缀复用在当前 perf 套件里没有可测收益** ✗（套件是 project/query/edit ✓，不是 `by` 密集 ✗）；
  而 `by` 密集课程文件上**命中率 91.7%** ✓、两态 `--json` **逐字节相同** ✓
  ⇒ **机制有效且等价** ✓，**但收益量不出来** ✗。
  ⇒ **计划说"收益成立才默认打开，否则保持关闭并记录"** ✓，而**代码已经默认打开** ✗（先于测量 ✓）
  ⇒ **选项：(a) 保持默认开 + 记录"本套件无可测收益" ✓ / (b) 改回默认关 ✗（更保守 ✓）**
  —— **它影响真实路径，所以留给你 ✓**（我不擅自翻 ✗）。**T-D6 未勾** ✗。
* **我自己的错（都记了 ✓）**：① 同一类错**两轮内犯两次** ✗（结论就在目标上方 5 行 / 15 行 ✓，
  却先做了五轮、两轮实验 ✓）；② 测量上**绕了七轮** ✗（重定向 ✗ → 形状 ✗ → 冷跑 ✗ →
  才去读"这个数从哪来" ✓），而**仓库一直有正规入口** ✓（`perf-ledger.sh` ✓）；
  ③ `ci-push.sh` 小 bug ✓（网络超时时打出**空 run id** ✗ ⇒ 应"拿到 id 才打印" ✓）。

## 第 478 轮（2026-09-27）：🚀 v0.74.0 开工 —— **E01 复合记法 `•` / `∘`** ✓（11 个环节的第 1 个）

- **变了什么** ✓：`Rel.comp` / `Function.comp` 补记法 —— `lib/Rel.sokonanoda` 声明
  `infixr:80 " • "`、`lib/Fun.sokonanoda` 声明 `infixr:90 " ∘ "`（各紧跟自己的 `def`）；
  单元⑥⑦⑫ 与三份解答改用记法（点形式一律消除）。**记法零事件 ⇒ 门禁 36/328/99/0 不变** ✓。
- **判红** ✓（内核原文）：`g ∘ f` ⇒ `符号 '∘' 在本文件里还没有声明过记法`；
  `r • s` ⇒ `unknown identifier '•'`（`•` 不在数学码点类里，未声明时连符号都不是 ✓）。
- **判据三层** ✓：front `the_course_libraries_declare_the_composition_notations`
  （记法表：`•`→`Rel.comp`/lib.Rel/80、`∘`→`Function.comp`/lib.Fun/90）·
  CLI `the_course_composition_notations_grade_like_the_pointful_forms`（真课程库、
  两种写法五元计数相等）· `scripts/notation-lint.py` 零残留（新声明**自动**派生点形式判据 ✓）。
- **反向验证** ✓：撤掉两条声明 ⇒ 两层判据**都红**，诊断逐字回到上面那两条 ✓（已还原 ✓）。
- ⚠ **取证更正** ✗✓：`lib/Rel` 头部原写「`r • s` 是 Mathlib 的记法」——**不准确**：
  Mathlib 的 `Relation.Comp` 是 `local infixr:80 " ∘r "`（`Mathlib/Logic/Relation.lean:158`）；
  `∘` 取 Lean core 逐字（`src/Init/Notation.lean:274`）。本课用 `•` 是为了与 `∘` 区分 ✓。
- **台账** ✓：新登记 **G-45**（library/painful/fixed_in 0.74.0，自足复现件 `G45-comp-notation.sokonanoda`）；
  `scripts/gap.py check` ⇒ **全部与台账一致** ✓。
- **文档** ✓：as-built 进 `docs/design/notation-subset.md` §16；速查表补【速查表 2b】+ 梯子两行；
  ⚠ 预算按规矩**手改** `scripts/docs-budget.json`（notation-subset 873→905，评审可见 ✓），
  另删掉 syllabus 里一段**无表头的重复表**（陈旧名 `Set.diff`/`Set.power`）⇒ 该文件 303→294 ✓。
- **未决** ✓：E02–E04 · E21–E23 · E27–E31 共 10 个环节未开工；v0.74.0 收尾要**推一次 CI 逐 job 真绿** ✓。
- ⚠ **本轮踩到的坑（两次同形 ✗✓）**：**判据跑完之前别改文件** ✗ —— 反向验证期间课程门禁读到
  半棵树（报 `符号 '∘' 还没有声明过记法` ✗）、front 判据读到**撕裂**的库文件（假红 ✗）。
  **判据必须在"冻结树"上跑** ✓：改完 → 跑 → 不再碰 ✓。

## 第 479 轮（2026-09-27）：E02 —— `Set.prod` 收进 `lib/Prod` ✓（v0.74.0 第 2 个环节）

- **变了什么** ✓：`def Set.prod` 从**单元⑤ 画布**收进 `courses/set-theory/lib/Prod.sokonanoda:73`
  （`lib/Prod` 新增 `import lib.Set`；记法 `×ˢ` 仍由 `lib/Set` 声明）；画布与
  `unit05-solution` 里那**两份副本删掉**（不删就撞 `import-name-collision`）。
- **判红** ✓（内核原文，修前实测）：临时模块根只放 `lib/` 时写 `s ×ˢ t` ⇒ exit 1
  `elab-notation-unknown-target`「记法 `×ˢ` 指向的目标 `Set.prod` 不存在」。
- **判据两层** ✓：front `the_set_product_notation_target_lives_in_the_library`
  （`×ˢ`→`Set.prod`/80/Infixr、声明点仍在 lib.Set；`Set.prod` 由 **lib.Prod** 提供且 checked；
  画布不再声明它）· CLI `the_set_product_notation_resolves_from_the_libraries_alone`
  （只拷 `lib/` 的模块根 + 只 import 库的画布 ⇒ exit 0）。
- **反向验证** ✓：把 `def Set.prod` 从 `lib/Prod` 撤掉 ⇒ 两层判据都红，诊断逐字回到上面那条（已还原 ✓）。
- ⚠ **净账（要记住）** ✗✓：副本消失 ⇒ 课程门禁 checked **328 → 327**（**练习数与 open 数一条未动**；
  `lib/` 合计 74 → 75）。**这是 E02 的正确结果、不是回归** —— 目标从「两份副本」变成「一份库定义」。
  门禁其余全绿：**36 目标 · 327 checked · 99 open · 0 判负** ✓。
- **台账** ✓：新登记 **G-46**（library/painful/fixed_in 0.74.0 + 自断言脚本复现件）；
  复现件**两个方向都自测过**（缺口在 ⇒ exit 0；修好 ⇒ exit 1）——自测当场抓到脚本里
  `$CODE）` 被 `set -u` 判 unbound（全角括号被吃进变量名）✗✓，已改 `${CODE}` ✓。
- **子代理** ✓：派了一次**只读取证**（普查全仓 `Set.prod`/`×ˢ` 的 stale 说法，带 file:line + 原文，
  禁夸大、禁下判断）⇒ 我抽查后并入 3 处：`course-lean-style.md:268`（C4 边界 #7 标已落）、
  PLAN A10 根因行、G-37 复现件注释 ✓。
- **未决** ✓：E03 · E04 · E21–E23 · E27–E31 共 9 个环节未开工；v0.74.0 收尾要**推一次 CI 逐 job 真绿** ✓。

## 第 481 轮（2026-09-27）：E04 —— hover 的类型面接上折叠 ✓（v0.74.0 第 4 个环节）

- **变了什么** ✓：`compile/check/mod.rs::resolve_hovers` 的文本从内核 pp **直出** ✗
  改成 `display.fold(&name_loose_bvars(…))` ✓（显示表从调用点传进去，那里本来就有 ✓）。
  用户看得见：编辑器/Infoview 里悬停**子表达式或假设**，类型面从 `Set.subset α A B` /
  `Not a` / `forall …` 变成 `A ⊆ B` / `¬ a` / `∀ …` ✓。
- **判红** ✓（实测，修前）：`report.hovers` 里没有 `⊆`、却有 `Set.subset α A B` 与
  `forall (α : Type 0) (A B : Set α), Set.subset α A B -> Set.subset α A B` ✗。
- **判据三层** ✓：真相层 front 新单测（修前判红）· **wire 契约层** LSP hover_brackets
  （`… h : ¬ a` + 反向断言不许出现 `: Not `）· **用户可见层** e2e 补**子表达式** hover
  （夹具 `u01:11` 的假设使用处 `h`）。
- **反向验证** ✓：`SOKO_NO_NOTATION_FOLD=1` ⇒ 那条 front 判据**判红** ✓（exit 101，不改代码）。
- **同步更新的旧判据** ✓（钉的是修前点形式 ⇒ 必须一起改）：front **5 条** + LSP **1 条**
  （期望值按**实测文本**写，含 `a ∧ (¬ a)` 的括号 —— 先 dump 再定，别一条条猜 ✗✓）。
- **实测数字** ✓：front **748 passed / 0 failed** · LSP **164 passed** ·
  CLI **25 target / 0 FAILED** · 完整 `scripts/soko gate` **exit 0** ✓。
- ⚠ **更正一条既有结论** ✗✓：`notation-paths-audit.md` 的用户可见面表早先写
  「LSP hover … 已迁 ✓」——**不准确**：`HoverType.text` 出自 `resolve_hovers`，那条没过
  `fold` ✗（**错误结论把缺口盖住了**）。已更正 + 记 E04。教训：**"某面已迁"必须能指到
  调用点或一条会判红的测试**。
- **台账** ✓：**G-50**（hover 漏折叠，fixed）；`gap.py check` 仍**全部与台账一致** ✓。
- **CI** ✓：E03 批次那次 run **`36316083145` 真绿**（28 success · 1 skipped=fast-fail ·
  重活 10/10）✓ —— v0.74.0 **第一次**拿到完整的逐 job 真绿。
- **未决** ✓：E04 未推（按批次制留给下一批）；E21–E23 · E27–E31 共 **7 个环节**未开工。

## 第 480 轮（2026-09-27）：E03 —— `r ⁻¹` / `A ≈ B` 记法 ✓（v0.74.0 第 3 个环节）

- **变了什么** ✓：`lib/Rel` 声明 `postfix:100 " ⁻¹ " => Rel.inv`、`lib/Equiv` 声明
  `infix:50 " ≈ " => Set.Equiv`；lib/Rel · lib/Equiv · lib/Demo · 单元⑥⑨⑩⑫ · 四份解答改用记法
  （`notation-lint` 零残留）。**记法零事件 ⇒ 计数中性**（36/327/99/0 不变 ✓）。
- **判红** ✓（内核原文）：`A ≈ B` ⇒ exit 1「符号 `≈` 在本文件里还没有声明过记法」；
  `r ⁻¹` ⇒ exit 1「unknown identifier `⁻¹`」（`⁻¹` 不在数学码点类里，未声明时连符号都不是）。
- **判据两层** ✓：front `the_course_libraries_declare_the_inverse_and_equinumerous_notations`
  （`⁻¹`→Rel.inv/100/Postfix · `≈`→Set.Equiv/50/Infix · 且 `⁻¹'`→Set.preimage **同时可见**）·
  CLI `the_course_inverse_and_equinumerous_notations_grade_like_the_pointful_forms`
  （真课程库、两种写法五元计数相等，含 `f ⁻¹' B` 共存用例）。
- **反向验证** ✓：撤两条声明 ⇒ 两层都红，诊断与判红逐字一致（已还原 ✓）。
- ⚠ **取证又更正两处引用** ✗✓：`lib/Rel` / `lib/Equiv` 头部原写「是 Mathlib 的记法」——
  **两条都不是**：mathlib4 master 的 `Logic/Relation.lean` 没有 `Relation.inv`/`⁻¹`；
  `Logic/Equiv/Defs.lean:80` 的 `≃` 是**等价的类型**不是等势；`SetTheory/Cardinal/Basic.lean` 无 `≈`
  ⇒ 两条都按**本课自定**落地（与 E01 的 `•` 同一个毛病）。
- ⚠ **新暴露一个真边界（G-48，open）** ✗✓：`≈` 的**两侧都是零元糖**时（`∅ ≈ {b}`）补不出论域
  ⇒ `elab-notation-argument-unsolved`；单元⑨ 练习 5 按设计写点名 + 行内标记（画布与解答都写明理由）。
  修法方向 = E19 甲案（求解器加元变量）。
- **台账** ✓：**G-47**（记法缺失，fixed，自足复现件）· **G-48**（零元糖操作数，open）·
  **G-49**（类型错误报裸 de Bruijn 编号 `期望 $4，实际是 $5`，open——写复现件时实测到的诊断质量问题）；
  `scripts/gap.py check` ⇒ **全部与台账一致** ✓（5 条 E01/E02/E03 条目逐条对）。
- **子代理** ✓：4 份解答的机械改写外包（prompt 自带判据+边界+三条取证纪律）⇒ 我**抽查后**
  自己补了它明确说"没做"的那一步：**画布↔解答签名逐字对拍**，当场抓出 **3 处括号不一致**
  （`Set.univ Nat ≈ …` vs `(Set.univ Nat) ≈ …`）⇒ 已按画布改齐（10 条全一致 ✓）。
- **未决** ✓：E04 · E21–E23 · E27–E31 共 8 个环节未开工；v0.74.0 收尾要**推一次 CI 逐 job 真绿** ✓。

## 第 482 轮（2026-09-27）：E21 结案「**不改**」+ E22 **build/rebuild 以项目为目标** ✓（v0.74.0 第 5–6 个）

- **先推后验** ✓：上一会话的 4 个提交（含 E04）rebase 后推上 main ⇒ run **`36321102036`** 逐 job 真绿
  （28 success · 1 skipped = `fast-fail` 条件 job · 0 failure · **重活 10/10** ✓ `ci-green.py` exit 0）。
- **E21 = 不改（resolved-no-change）** ✓（`ce0371b` · 状态 `9530851`）：用户 I1 说「theorem 卡片那行
  『目标 ⊢ A = B』多余」、计划原判是删；**复核推翻前提** —— 那行是**还没证完的目标**（闭合声明本就没有它，
  实测 `goal=null`；`intro h` 之后变成剩下的 `B`）⇒ 用户拍板保留。行为零改动 + 渲染点写死结论 +
  **防漂移判据**（`test-webview.js` 的 E21 用例）+ 依据进 `docs/gaps/criteria-census.md`；反向验证：
  把「目标 == 语句就不画」加回去 ⇒ 判据判红 ✓。
- **E22** ✓（`b90c0ff`）：判红两半 —— ① `buildTarget()` 取活动文件 ⇒ 只编一个（实测 `files: 1` vs 项目 2）；
  ② rebuild 的 `--clean` **不带目标** ⇒ 只清全局、项目条目原地不动 ⇒ 紧跟的 build 全是 `hit`（假动作 = R-3/T-B5）
  ⇒ **G-51**（fixed）。改法：目标 = 服务端 `soko/project` 的**模块根**（不自己找清单；没答上来退回工作区根，
  **永远是目录**）+ clean 带同一目标。判据：stub 宿主钉 argv（38/38）+ e2e 钉**用户看得见的数字**
  （`N 个文件` == 项目文件数、`清掉 N` ≥ 1）+ 文档同步；反向验证两半**各自**判红、逐字一致 ✓。
- 附带 ✓（`1d17818`）：`LSP_CUSTOM_METHODS` 漏了 `soko/project` ⇒ 同步 skill 时被守卫误判；补上后仍咬得住
  （`soko/bogus` 判红 ✓）。**未决**：E23 · E27–E31 共 **7 个**；门禁 **36/327/99/0** ✓。
- **E23 + E29** ✓（`3a5e94f` + CLI 事件 `0ad970f`）：build/rebuild 的进度接到**三处同一份**
  （状态栏逐帧 `3/13 文件 · lib/Set.sokonanoda` / Infoview 三行区 / 概览尺）+ 原生进度条**可取消**
  （顺手修「完全不能取消」✗）；`runBuildProcess` 流式化，CLI 新增 additive 事件 `build.begin`（带总数）。
  判据 stub 宿主 39/39（**中间态**逐帧 + `increment > 0` + 取消真 kill）；**反向验证**两处各自判红 ✓。
  ⚠ 如实记账：设计里的 `{viewId}` 标题栏进度条**未做**（扩展宿主看不见视图 chrome ⇒ 无判据）。
- ⚠ **上一批 CI 红了一条（已修）** ✗✓：run `36323798295` 的 **e2e (ubuntu 1.138.0)** ——
  `T-A60-2` 的 1.5s 窗口被**邻居用例迟到的落盘**压红（多出的两条 2101/5174B **不是夹具闭包**：
  u01 是 155KB 级、`lib/Set` 单独编 26KB 级）⇒ 治法与 T-A60-3 同款：**先等缓存静止再取基线**
  （`90cb15b`），台账进 `docs/CI-FAILURES.md` ✓。⚠ 那轮最终是 **`cancelled`**（26 success · 1 failure ·
  2 skipped）—— 不是 `failure` 收尾，**别把它读成"绿过"** ✗。

## 第 484 轮（2026-09-27 深夜）：🚀 v0.75.0「跳转与高亮」**E05–E08 全 ✅**

- **E05 + E06** ✓（`3c6ac01`）：**记法声明的目标名 F12 落点**修好（Bug A，一行级）—— `range` 原来用了
  **光标处**那个 span（真定义 span 被 `_` 丢掉）⇒ **原地跳** ✗。真课程库实测：G-37 复现件
  **exit 0 → exit 1**，`Set.powerset` **L125→L81** · `Set.compl` **L126→L79** ✓；单测钉「落点行号 == `def`
  那一行、不许自跳」；反向验证（换回光标 span）报错**逐字一致** ✓。**G-37 → fixed** ✓。
- **E07** ✓（`eee6cc2`）**定案**：① 落点与**编辑器 F12 一致**（同一条通道）⇒ 闭包外**诚实 `null`**、
  **不扩到全仓**（要先编译闭包外模块，代价不成比例）② 高亮**实测已一致** ✓（真库 **11 条**目标名全
  `DefUse`；计划里「3 条 `variable`」是过时描述 ✗✓）⇒ 产出 = **两条防漂移判据**；3 条登记 **G-54**。
- **E08** ✓（`eee6cc2`）**定案：本轮不做**（要做先回答「目标名的『同一个定义』含哪些位置」，三种备选语义
  不同 ⇒ 设计决定，不顺手拍）⇒ **G-55** + 「断言当前行为（`null`）」的防漂移判据 ✓。
- ⚠ **判据自己假绿过一次** ✗✓：E07 第一版只断言「11 条**同色**」⇒ 反向验证（全变 `UnknownIdent`）时
  "同色"**依然成立**、判据照样绿 ✗ ⇒ 补「必须是**已知引用**那一种颜色」后才咬得住 ✓。

## 第 483 轮（2026-09-27）：Infoview 面收口 —— **E27 / E28 / E30 / E31** ✓（v0.74.0 收尾，11/11）

- **E31** ✓（`bc9a62c`）：新命令 **`Clean Cache (清除编译缓存)`** —— **只清不编**（Rebuild 是
  clean→build 串成一步，用户没有"清完就停"的入口 ✗）；三个数逐字来自 CLI 的 `build.clean` 事件；
  判据 stub 40/40 + e2e **实测 N→0 且等 1.5s 仍为 0**（专防"clean 偷偷编了一次"）；反向验证两条各自判红 ✓。
- **E28** ✓（`086c220`）：**主机侧**三态判据（loading / ready-empty / error 分得开）—— 渲染层早有
  判据、**接缝没人验** ✗；反向验证：把失败说成"没有声明" ⇒ 判红 ✓（E28 只补判据、不改行为 ✓）。
- **E30** ✓（`5323889`）：Infoview **「项目」区块**（转发 `soko/project`、**零额外取数**；
  `requires_warning` 是**可见块不是 tooltip** = G-24 的唯一可见信号）；顺手补上
  `audit-wire-fields.py` 的 **front 侧结构解析**（= 审计 #17 的 7 处盲区）并**收紧扫描器**
  （去注释 + 引号内不算读取；⚠ 顺序错会让 `compiled/*.tmp` 的散文卡住 `in_block` ⇒ **整份文件后半段被静默跳过**，
  实测把 `value_runs` 也扫没了 ✗）。
- **E27** ✓（`1444780`）：Infoview **声明名可点 ⇒ 跳到定义**（`executeDefinitionProvider` = 编辑器 F12
  同一条命令；落点与 F12 **逐字段相同**、跨文件 ✓）；反向验证退回 `reveal` 判红 ✓。
  ⚠ **未接的一半**：**记法符号**（`{a}`/`∈`）—— runs 只有 `{text,kind}`（实测 577 条）⇒ 要动 wire，
  登记 **G-53**（open，带自足复现件）✓；⚠ 另实测**声明名位置不返回定义**（本 LSP 解析使用处）⇒ 点名字会
  看到"这里没有可跳转的定义"（诚实，但有用的落点正是被 G-53 挡住的符号）。
- **预算放宽（用户 23:12 拍板：可放宽，但同一 commit 记一笔）**：`STATUS.md` 上限 **200 → 240 行**
  （`scripts/status-lint.py` 的 `MAX_TOTAL`）—— 原因：一轮里落了 6 个环节，200 行顶格后**只能删旧轮**，
  而归档目标 `docs/STATUS-ARCHIVE.md` 也被冻结 ⇒ 实际是"逼着删历史" ✗；`MAX_GROWTH`（≤60）**不动** ✓。
  **后续统一 refactor 时清理**（旧轮搬进 `docs/archive/` 再调回 200）。

## 第 485 轮（2026-09-28）：🚀 v0.77.0 · **ST1 ✅**（类型论/外挂分界 —— 决策记录 + 两端对账守卫）

- **变了什么** ✓：**新增决策记录** `docs/design/v077-st1-boundary.md`（89 行）—— 逐项判定
  「哪些用类型论自身表达、哪些确实必须外挂」：① `Set α` 谓词式**够用**（分离 / 无限并交 / 幂集）
  ② **序数写成谓词**（Isabelle `Ord x ≡ Transset x ∧ …`；**全体序数 `ON` 不是集合** ⇒ 本来就不该是类型）
  ③ **基数必须有商**（Mathlib `Cardinal := Quotient Cardinal.isEquivalent`）④ **秩/超限递归两条路都要
  「良基递归可用」**（Mathlib `Acc.recOn` · Isabelle `foundation` **公理** + `wfrec`）。
  **基准逐条带 URL**（Mathlib `Ordinal/Basic` · `Cardinal/Defs` · `Ordinal/Rank` · Lean core `Init/WF`
  · AFP `ZFC_in_HOL` §1.4/§2.7/§2.6 · TPiL §12.4）—— 用户要求「不许凭记忆编」✓。
- **判红（实测原文）** ✓：**两条新的** —— ① 源语言**没有 `Quot`**：`elab-unknown-identifier` /
  「unknown identifier `Quot`」（内核其实**内建** `Declar::Quot` + `Quot.lift`/`ind` 的 iota 归约 ⇒
  缺的是**前端产出**，不是内核）② **`Acc` 立不起来**：`kernel-rejected` /「rejected: inductive
  occurrence is not applied uniformly to the block parameters and universe levels」——**对照组**
  `Even : Nat → Prop` 同形状能过 ⇒ 被拒的是「**下标会变**」，不是「载体是函数」。
- **判据与测试** ✓：**新增** `crates/cli/tests/st1_boundary.rs`（2 个判据）+ **4 个自足复现件**
  `docs/gaps/repro/ST1-*.sokonanoda`（无 `import` ⇒ 单文件判卷）+ **新缺口 G-56**
  （`docs/gaps/repro/G56-acc-well-founded-recursion.sh`，含对照组；`scripts/gap.py check` 全绿 ✓）。
  守卫是**两端对账**：记录里写的诊断必须在探针输出里**逐字**出现，探针输出的每条诊断也必须在记录里
  找到（记录漏记 / 结论过期都判红）✓；另有「`ST1-*.sokonanoda` 与对账表一一对应」的反向守卫 ✓。
- **反向验证两次** ✓：① 把记录里 `Quot` 那行改成 `positive / 7` ⇒ 判红「`decl.checked` 数与记录不符
  （记录 7 / 内核 0）」；② 把 `Acc` 探针换成一条必过的声明 ⇒ 判红「记录 0 / 内核 1」；两次都**撤掉即回绿** ✓。
- **没做** ✗：**ST2（商类型）未开** —— 用户明确「等我对 ST1 的决策记录确认后再开」✓；
  内核判定零改动 ✓、课程内容零改动 ✓。
- **耗时账** ✓：`cargo test -p sokonanoda-cli --test st1_boundary` **0.6 s**（2 判据，跑 4 个复现件）；
  `python3 scripts/gap.py check` 全量 **~1 min**（含 G-56）；`python3 scripts/docs-lint.py` ✓。

## 第 486 轮（2026-09-28）：🚀 v0.77.0 · **ST2 ✅**（商类型 `Quot` 装进源语言 —— 路线 A）

- **变了什么** ✓：`install_quot`（`crates/front/src/compile/prelude.rs`）把 `QUOT_TYPES_SRC`
  五条类型交给**前端自己的 elaborator** 建成 **`Declar::Quot`**（`Quot`/`Quot.mk`/`Quot.lift`/
  `Quot.ind`）+ `Quot.sound`（**唯一**公理，TPiL §12.4）。**内核零改动** ✓（用户核实：
  `quot.rs`、`RigidHead::QuotConst`、`STANDARD_AXIOMS`、按名查找四条全在，缺的只是前端产出）。
  用户拍板**路线 A**（理由见 `docs/design/v077-st1-boundary.md` §3）。
- **判红（修前原文）** ✓：`def mkQuot … := Quot α r` ⇒ `elab-unknown-identifier` /
  「unknown identifier `Quot`」。
- **⚠ 最贵的一课（10+ 轮）** ✗✓：内核 `quot.rs::check_quot` 的 `mk_var(n)` 索引与「按
  de Bruijn 深度推」**不一致** —— 手搓 `EnvBuilder` 表达式结构「看起来对」（`#check` 能渲染
  对的形状），但 `def q … := Quot.{1} α r` 判红「期望 `… $0 …`，实际 `… $2 …`」✗。
  **正解 = 类型写成源文本交给前端 elaborator，只改声明种类** ✓（文本是真的、与
  `prelude_source()` 同源、F12 可用）。
- **判据（放在归约上）** ✓：`docs/gaps/repro/ST2-quot-reduces.sokonanoda`（4 checked，含
  `Eq.refl` 证 `Quot.lift … (Quot.mk …) = f a`）+ `ST2-quot-family-yields`（让位口径）+
  `crates/front/src/compile/tests.rs` 的 `st2_*` **五条** + `crates/cli/tests/st2_quot.rs`
  **两条**；新登记 **G-57**（fixed_in 0.77.0）。**为什么必须在归约上**：装成普通 `Axiom`
  时名字在、类型对、**归约死** ⇒ 只有「算得出来」同时证明「装上了 + 类型对 + 种类对」✓。
- **反向验证** ✓：撤掉 re-kind ⇒ `st2_quot_lift_computes_on_quot_mk` **判红**（`Quot.ind`
  那条仍绿 —— 它靠 `False.elim` 也能过，**这正是"判据要选对那条"的实测**）；
  撤掉 `install_quot` ⇒ `st2_quot_names_are_installed` 判红。
- **ST1 记录随之更新** ✓：ST2 一落地，ST1 守卫**当场咬住**（`ST1-quot-unavailable` 记录 0 /
  内核 1 ⇒ 判红）⇒ 商那一半搬进 ST2 探针，ST1 只留「没有累积性」（L-06，改名
  `ST1-no-cumulativity.sokonanoda`）✓。
- **没做** ✗：**ST10（基数）未开**（用户明确「做完停下，不要顺手开」）· **G-56 本轮不修** ✓。

## 第 487 轮（2026-09-28）：G-56 顶层探针 —— **大消去不可用**（ST15 第一批条目：G-56 / G-58 / G-59）

- **用户指派**：「先用最低成本把 G-56 的『顶层能不能通』探清楚，再决定投入多少」+ **时间盒 2–3 轮**、
  「把不足逼出来才算是做完，把它修好不是本版目标」。**零改动**（`git diff crates/kernel/` 空 ✓）。
- **① 大消去探针 ⇒ 被拒** ✗：`def andToType (A B : Prop) (h : A ∧ B) : Type :=
  And.rec A B (fun (_ : A ∧ B) => Type) A B h` ⇒ `kernel-rejected` /「期望
  `Pi (x : ((And.[] $2) $1)), Sort(0)`，实际是 `Pi (_ : ((And.[] $2) $1)), Sort(2)`」。
  **对照组**（消去到 `Prop`）**checked** ✓ ⇒ 不是写错 recursor。机制：`mk_elim_level` 问
  `large_elim_test`，false ⇒ `elim_level = 0` ⇒ `mk_motive_dep` 把 motive 钉在 `Sort(0)`。
  ⇒ **即使 `Acc` 声明成功，ST7 的 rank 也返回不了序数**（`Acc` 在 Prop、`Ordinal` 在 Type）✗。
- **② 拦路点确认唯一**（只读）✓：`crates/kernel/src/inductive.rs:44`（`check_ctor` 循环）调
  `check_uniform_inductive_occurrences`；`:150` 是 `_at` 入口包装、其余全自递归；**前端零镜像**。
  它要求递归出现**恰好**套用 `num_params` 个实参（`args_rev.len() <= num_params` 才进断言）⇒
  `Acc r b` 有 2 个实参 > `num_params=1` ⇒ 直接 assert 失败。⚠ 同文件 `which_valid_ind_app_v`
  （`:1049`）用 `is_bvar_at` 判参数位置、**允许索引位任意** ⇒ 最小放宽应**对齐它**。
  改动**局部**：只跑在声明级（`check_ctor` 里、`mk_elim_level` 之前），不参与
  `check_generated_recursors` / `check_positivity1` / 归约。
- **③ 顺带发现（独立）** ✗：住在 `Type` 的归纳块**默认 motive 也是 `Prop`**；显式 `.{1}` 只修好
  「`#check` 的显示」，**消去仍被拒**（`Sort(1)` vs `Sort(2)`）⇒ **本版没有绕法**。
  内核探针实测（临时 `eprintln`，**已还原**）：该块 `large_elim_test` 返回 **true** ⇒
  疑似**前端派生递归子的宇宙参数默认值**与内核期望不一致（根因未定位，时间盒到了）。
- **落台账（ST15 第一批条目）** ✓：**G-56**（`Acc` 声明被拒，收敛为一条 + 拦路点定位）·
  **G-58**（大消去不可用）· **G-59**（`Type` 值块默认 motive + 无绕法）；两个 `.sokonanoda`
  复现件 + `scripts/gap.py check` 全绿 ✓；判据
  `crates/front/src/compile/tests.rs::g58_g59_large_elimination_is_unavailable`（三条断言，含
  「显式 `.{1}` 仍被拒」的防漂移）。
- **结论与下一步** ✓：G-56 那四个环节（ST6/ST7/ST9/ST11）**本版不做**、留给 ST15；
  按用户决策规则**直接开 ST3/ST4/ST5**（按 ST1 结论只差 binder 记法、不需要任何新机制）。

## 第 488 轮（2026-09-28）：ST3 / ST4 / ST5 / ST8 / ST10 —— **五章收口** + 四条新缺口（G-60…G-63）

- **用户拍板顺序**：「① 先推进不依赖 Acc 的：ST8 → ST10 → ST12 → ST13 → ST14 → ST15；
  ② 撞墙的一律不硬做，登记进台账 + 进 ST15 清单（**登记本身就是交付物**）；
  ③ 卡住的四项（ST6/ST7/ST9/ST11）放到最后，别自己开工」。本轮按此推进 ✓。
- **ST3 分离** ✓：`lib/Set` 新增 `Set.sep` + `mem_sep_iff`/`sep_subset`/`sep_self`。
  ⚠ **记法那一半做不到** ⇒ 新登记 **G-60**（`{x | P x}` ⇒ `set-literal-shape`；
  `{x : α | x ∈ A}` ⇒ `unexpected-token`「found Pipe」；根因：六种记法形状里**没有
  「操作数在括号里」**，且 `{` 被判成 binder 组）⇒ 课程点名写 `Set.sep` ✓。
- **ST4 集族并交** ✓：**新模块** `lib/SUnion`（`Set.sUnion`/`Set.sInter` + 四条展开引理 +
  **记法 `⋃₀`/`⋂₀`**）。**有意偏离 Mathlib**：按**依赖**拆模块（无限并要 `∃`，
  而 `lib/Set` 不 import `Exists`）⇒ 理由写进文件头 ✓。
- **ST5 不交并 / 函数空间** ✓：**新模块** `lib/Sum`（+ 记法 `⊕`）；`lib/Fun` 增 `Set.pi`
  + `Set.mem_pi`（**不另立** `Set.funSpace` —— 非依赖版就是 `Set.pi s (fun _ => t)` ✓）。
- **ST8 序数（谓词式）** ✓：**新模块** `lib/Ordinal`（7 定义 + 3 条 L2 引理），**零新类型**。
  ⚠ 与 Isabelle 的**两处有意不同**写进文件头：① Isabelle 的 `Ord` **不含良基性**（靠全局
  公理 `foundation`），我们**显式写进 `IsOrdinal`**（本语言没有那条公理、也没有 `Acc`）；
  ② `Transset` 用 `E` 的传递性而非子集序 ✓。
- **ST10 基数（类型的商）** ✓：**新模块** `lib/Cardinal`，**7 条全 checked**（含 `Cardinal.sound`
  = `Quot.sound` 直接实例、`Cardinal.lift_mk` = `Eq.refl` 级 ⇒ **ST2 的归约真的发生** ✓）
  —— **ST2 的 `Quot` 第一次实战检验通过** ✓。
  ⚠ **三条新缺口**（ST15 条目）：**G-61** 没有 η ⇒ `Quotient`/`Setoid` 包装做不出来；
  **G-62** def/展开不同一 ⇒ `Type.Equiv` 三条等价律写不出来（核心不受影响）；
  **G-63** `Quot.lift` 宇宙实参对不上内核签名 —— ⚠ **实测推翻「Quot 消去只进 Prop」的初判**
  ✗✓（`Quot.lift.{1, 2}` **能**进 `Type` ✓）⇒ 是**实参难对准**，不是缺能力。
- **门禁** ✓：课程 **40 目标 · 360 checked · 99 open · 0 判负**；`notation-lint` OK；
  `scripts/gap.py check` 全部与台账一致（G-56…G-63 八条 open 全带复现件）；
  `scripts/soko gate` **exit 0** ✓。

## 第 489 轮（2026-09-28）：**v0.77.0 分批表收口** —— ST12/ST13/ST14/ST15 四章 + **内核不足清单**

- **用户拍板顺序**「① ST8 → ST10 → **ST12 → ST13 → ST14 → ST15**；② 撞墙的不硬做，
  登记本身就是交付物；③ 卡住的四项放最后、别自己开工」⇒ 本轮把 ① 全部走完 ✓。
- **ST12 选择公理** ✓：新模块 `lib/Choice` —— `axiom choice`（外挂，与 `Set.ext` 同档）+
  `Nonempty` + 2 条展开，**4 条全 checked**。形式照 Isabelle `AC_imp_2`（集合版）。
  ⚠ 从 `choice` **取函数**要在 `Prop` 里取数据 ⇒ **撞 G-58** ⇒ L3/后续。
- **ST13 ZF 公理系统本体** ✓：新模块 `lib/ZF` 7 条全 checked（`axiom regularity` +
  `IsEmpty`/`IsPair`/`IsUnionOf` + 3 条展开）。**核心产出是「ZF 公理表」**（文件头）：
  分离/配对/并集/幂集/替换**都是定理**（谓词就是集合），**只有外延性、正则性、选择必须外挂** ✓
  —— 这是 ST1 结论 ① 的具体体现。
- **ST14 `propext`/`funext`** ✓：新模块 `lib/Extensionality` 4 条全 checked。
  ⚠ **核心判断**：`Set.ext`/`Rel.ext` 在 Mathlib 里是这两条的**推论**，我们两者都没有 ⇒
  那两条一直是**公理**；本档立起来后**原则上可改写**，但**本版不做**（重构 ≠ 补缺口）✗。
  **univalence 不做、也不登记**（更强的公理，非本课必需件）✓。
- **ST15 内核不足清单** ✓（**本版的核心交付物**）：`docs/design/v077-kernel-deficiencies.md`（130 行）——
  **3 blocker**（G-56 `Acc` · G-58 大消去 · G-59 `Type` 值块也消去不到 `Type`）+
  **4 painful**（G-60 花括号 · G-61 没有 η · G-62 def/展开不同一 · G-63 `Quot.lift` 宇宙实参），
  每条带**复现件 + 影响面 + 绕法 + 拦路点源码定位**；卡住的四章（ST6/ST7/ST9/ST11）逐条对照
  「被谁挡住」；**给后来者的动手顺序建议**（G-58 先修 → G-59 同批 → G-56 最后，且要先问用户）✓。
- **v0.77.0 分批表 ① 全部收口** ✓：完成 **9 章**（ST1/ST2/ST3/ST4/ST5/ST8/ST10/ST12/ST13/ST14）
  —— 实为 10 章，其中 ST1/ST2 在第 485/486 轮 ✓。
- **门禁** ✓：课程 **43 目标 · 375 checked · 99 open · 0 判负**；`notation-lint` OK；
  `gap.py check` 全部一致（G-56…G-63 八条 open 全带复现件）；`soko gate` **exit 0** ✓。
- **改内核判定：零** ✓（`git diff crates/kernel/` 为空）—— 用户明确「不许为了让它 checked
  通过去改内核判定」✓。

