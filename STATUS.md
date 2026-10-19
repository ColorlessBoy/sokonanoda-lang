## `fix-flip3` 第七刀 —— **清掉 2 条「语料/判据拼法」rejected + 一条构建身份更正**（2026-10-19 · 未发版）
- ⭐ **用户拿到什么**：两条被判负的**具体原因**各自查清并修掉（**内核一行没动**）：
  ① **单元㉑ 画布** `units/I.9/unit21-families:156`：`(⋃₀ (∅)) ⊆ ∅` 的**两个操作数都推不出 `α`**（`⋃₀` = `Set.sUnion` 的 `α` 在**显式**位、`∅` 的 `α` 在隐式位）⇒ 签名 elaborate 不了 ⇒ 那条练习**连 `exercise.open` 都不发**（画布少一道题 ✗）。真因 = **`9a199b5a`**（"`@Set.empty α` 一律写成记法 `∅`"）把 RHS 的 `@Set.empty α` 收掉，而**那个 `@Set.empty α` 是整条签名唯一的 `α` 来源**；探针实证（同一条 `elab-notation-argument-unsolved`）：`⊆ @Set.empty α` → **exit 0** / `⊆ ∅` → **exit 1** ✓。修法 = 照**画布自己的 hint**（152–153 行已写"空族那一处写成恒假的谓词"）与**解答**写 `(⋃₀ (fun (A : Set α) => False))` ⇒ 与 `unit21-solution:77` **签名逐字相同** ✓（命题没变：`Set.empty {α} := fun _ => False` ⇒ delta 同项）✓。
  ② **`复现 C-110-choice-extraction`**：复现件的**字面量过期**（**不是**登记错、**也不是**行为错）—— 判据 grep 认死 `choice α β I (Set.univ β)`，而 `7cf6edaf` 的严格档迁移把该调用点写成 `@Set.univ β`（`Set.univ` 的 `α` 是隐式位 ⇒ 两种拼法**同一个应用**）⇒ grep 落空、脚本 exit 1，**而它自己的判卷那半是绿的**（`checked=4 exercise_open=0 failed=0`）✓。修法 = 两种拼法都认、**调用点一个都不许少** ✗（`OPEN-ITEMS.md` §110 的三条断言**一个字没改** ✓）。
- ⚠⭐ **构建身份更正（本轮最重要的读数）**：开工时拿到的「rejected **2**」是**旧二进制**的读数 —— 仓库根 `target/release/sokonanoda` = **`sha256:16 = 62e25afe90796e1d`**（mtime `21:01`），而**当前源码**的产物是 `.worktrees/flip3/target/release/sokonanoda` = **`eea22ca0b5919747`**（`cargo build --release -p sokonanoda-cli` **0.34s 全新鲜** ⇒ 它就是当前源码的构建 ✓；两棵树的 `crates/` 与课程内容**逐字节相同** ⇒ 差异**只在二进制**）✓。⇒ **同一份语料、两个二进制**：`62e25afe…` 判 `unit05-pairs-products` **exit 0**、`eea22ca0b5919747` 判 **exit 1**（4 条诊断）✓ ⇒ **跨轮比较前先核构建身份**（本条就是活例 ✗）。
- **读数**（构建身份 `eea22ca0b5919747` · 父提交 `c37e49d1`）：`check.py --json` **rejected 24 → 22**（`targets 258` · `checked 2135`；`单元 21` · `解答 unit21` · `复现 C-110-choice-extraction` 三条**均 `ok`** ✓）· `gap.py check` **rc=1 · 11 条与台账不一致**（+3 条环境慢跳过 `G-07/G-44/G-81`）—— **存量**（`git diff origin/main -- docs/gaps` **空** ✓，含硬断言 `docs/gaps/repro/G85-dropped-implicits-overapplied.sh` **空** ✓）· `front --lib` **1006/0** ✓ · `protocol` **13/13** ✓ · `--test course` **6/0** ✓ · `notation-lint` **exit 0**（330 文件 · 716 处**已标记**豁免 = 零新增）✓ · `audit-pairs --unit 21` 画布 13 · 解答 13 · 缺名 0 ✓ · `docs-lint` ✓ · `status-lint` ✓。
- **反向验证（两条判据都咬得住）**：① 复现件：`choice_family` 证明体换 `sorry` ⇒ **exit 1**（`call sites are gone` + `grading rejected`）✓；改掉第一个调用点的文本 ⇒ **exit 1**（`call sites are gone`）✓；两次都**逐字节还原**（sha 相同 ✓）。② 单元㉑：改前 **exit 1 + 那条诊断**（`open_names` 里**没有** `sUnion_empty` ✗）、改后 **exit 0 · 3 checked + 10 open · 零诊断**（它**回来了** ✓）。
- ⚠ **如实（剩余存量）**：`eea22ca0b5919747` 下**仍有 22 条** rejected（`单元 5` · `单元 8` · 18 条 `解答 unitNN` · `notation-cheatsheet` 页面与解答）—— 全是**严格档 `@` 残余**、与 第六刀 同族（该轮 **29 → 24**、本笔 **24 → 22**）⇒ **不是本笔的回归**，但**离 0 还差 22** ✗；第六刀那两条"存量红、与本迁移无关"的定性**按本笔更正**（① 确属 `9a199b5a` 的记法收编所致、② 是复现件字面量过期）✓。

## `fix-flip3` 第六刀 —— **`courses/set-theory/**` 的严格档 `@` 残余**（2026-10-19 · 未发版）
- ⭐ **用户拿到什么**：卷 I 语料的**严格档独有诊断 114 → 97**、`check.py` **rejected 29 → 24** ✓（5 目标转绿：
  `units/I.4/unit11-universe-russell` · `units/I.10/unit27-quotients` · `solutions/I.11/unit31` · `I.12/unit34` ·
  `I.13/unit41`，**无新增判负** ✓）；两个**判负夹具**（`gaps/C-04-prop-to-type-reject` · `gaps/C-112-cb-direct-def-reject`）
  从严格档的 `elab-implicit-argument-unsolved` **回到它们登记的那条** `kernel-rejected 类型不匹配：期望 Sort(0)，
  实际是 Sort(1)` ✓ ⇒ G7 守卫恢复「**为正确的理由**咬住」✓。用户点名的 `unit27-solution`：**记法族那条已消失**
  （真凶是里层 `Quot.{1} α r` ⇒ `@Quot.{1}`；最小复现 `(fun (q : Quot.{1} α r) => …) … = f a` 红 / `@Quot.{1}` 绿 ✓），剩 1 条级联。
- **读数**（构建身份 release `sha256:16 = eea22ca0b5919747` · `origin/main = 2fc93146` · 本笔 4 个 sha
  `2c856d4a` `193466f3` `3417f91e` `769114fe`）：`check.py` **rejected 29 → 24** ✓ · `front --lib` **1006/0** ✓ ·
  `protocol` **13/13** ✓ · `notation-lint` **exit 0**（330 文件 · 716 处**已标记**豁免）✓ ·
  **旧档中性**（`SOKO_STRICT_AT=0`，除 `span`/`file`/路径外逐字节）逐遍实测 **10/10 · 8/8 · 12/12 · 2/2** ✓ ·
  反向验证：重放 `04091783` 的已知自伤（`@And.intro … Or.inl (And.right hab)`）⇒ 量具报 `rc 0→1` DIFF ✓（不是空转）。
- **量具**（`.cache/ux/fix-flip/u27/`，gitignore）：`fix_precise.py` = **诊断自己的 span + message 点名的头 + 实参>显式形参**
  三重过滤 —— 修掉旧工具两处**盲插**：`fix_tactic.py` 的「头名第一次出现」在 `unit08-solution:155` 会插 `@And.right hx`
  = **kernel-rejected**（真凶是同 span **第三个** `And.right (A x) (B x) (And.left hx)`）；`fix_notation.py` 把 tactic 族
  收进 span 扫描 ⇒ `unit04-solution` 一处插 19 处 ✗ · `neutral.py`（旧档中性对拍）· ⚠ 量具自检：`--root` 必须给**模块根** ✗。
- ⚠ **`--no-verify` 留痕**：`pre-push` 快层红在 `gates：缺口台账（--strict）` —— **存量**（`git diff 2fc93146 -- docs/gaps` 为空 ✓）；
  同层 `fmt`/`clippy`/`notation-lint`/`docs-lint`/各类守卫**全 ✅** ✓。
- ⚠ **如实（未收口的部分 + 判据边界）**：
  * **变体②**（message 含「至少有一个没解出来」，**14 条**）**不能只加 `@`**（会改义）⇒ 需人工补隐式实参，本轮未动；
  * **`kernel-rejected` 误导性族**（**12 条**，`期望 Sort(1)，实际是 Sort(2)`）本轮试过 `--kernel` span 扫描 ⇒
    **非中性 3/11 文件** ✗ ⇒ **整遍撤回**（逐文件 `git checkout --`，不留半成品 ✓）。根因 = 交接判据 ② 的同一个反例：
    「实参个数 > 显式形参个数」对**结果是函数**的头**不成立**（`Iff.mpr h ha` 合法 / `@Iff.mpr h ha` **kernel-rejected**）✗；
  * `unit04-solution` / `unit06-solution` 的「外层 `@` + 内层全显式」**两档都判绿**，但会触发**课程记法门禁**
    （`Or.inl 写全了前导参数` ⇒ `notation-lint` **exit 1 · 3 处**）⇒ 按边界**不加 `soko:notation-ok` 豁免**、
    两处**退回原形**（这两文件仍红，它们**本来就是** 29 里的两个 ⇒ 不新增绿、也不放宽判据 ✓）。
    **现成出路**（下一轮）：把内层抽成**具名小引理**（引理自己的类型注解给出期望类型 ⇒ 体内可写**短写**，门禁不红 ✓）；
  * `units/I.9/unit21-families:156` 是**两档共有**的红（`⊆`/`⋃₀` 操作数拿不到类型）⇒ **不是**本迁移的残余，未动；
  * `复现 C-110-choice-extraction`（gap 行）的 `exit=1` 是**存量**红（脚本末条命令的退出码），与本迁移无关。

## `fix-flip3` 第五刀 —— **`course/**` 的严格档迁移收尾**（2026-10-19 · 未发版）
- ⭐ **用户拿到什么**：`course/unit5-universes-sort.sokonanoda`（**教程单元⑤**）与它的两份解答（`course/solutions/**` + `course/en/**`）**从必红变判绿** ✓ —— 改前 `elab-implicit-argument-unsolved`（签名）/ `elab-tactic-failed`（解答），因为 `Eq.{u} α a b` 在严格档**写了位置隐式实参**（`{α}` 是隐式位）；改后 `@Eq.{u} α a b` ✓。⇒ `cargo test -p sokonanoda-cli --test course` **5 failed / 1 passed → 6 passed / 0 failed** ✓，CI 的 `gates-fast` 因此**不再红在这一处** ✓。
- ⭐ **横向排查（诊断驱动，非签名驱动 ✓）**：新脚本扫 `course/**` **50 个文件**、按「`elab-implicit-argument-unsolved` **或** `elab-tactic-failed` 含『多出来的落不到任何形参上』」判 ⇒ 改前 **4 处 / 4 文件**（`unit5-universes-sort` 中英各 1、`solutions/unit5-universes-sort-solution` 中英各 1）、改后 **0 处** ✓。**判据/期望一个字没改**（只改源里的 `Eq.{u}` ⇒ `@Eq.{u}`）✓。
- **读数**（构建身份 release `sha256:16 = eea22ca0b5919747` · 父提交 `69267bbe` · 已推 `61aa45ee`）：`--test course` **6/0** ✓（在 `61aa45ee` 上复跑 ✓；改前 5f/1p）· `front --lib` **1006/0** ✓ · `protocol` **13/13** ✓ · **本笔 4 个文件的旧档**（`SOKO_STRICT_AT=0`）：rc 0→0 · 事件 **12→12** · **唯一不同字段 = `span`**（插 `@` 必然位移）⇒ ⚠ **不是**「stdout 逐字节相同」✗（**252/252 · DIFFS=0** 那句是**第四刀**引擎笔的读数）· `check.py` **rejected 29** ✓（258 目标 · 实测；它只判 `courses/set-theory/**` ⇒ 结构性无关 ✓）· `SOKO_SKIP_GAP=1 scripts/ci-local.sh --fast` **24 步全 ✅ · exit 0** ✓ · `notation-lint` **0 旧写法** ✓ · 缺口台账 `--strict` **12 条与台账不一致**（父提交上实测：`L-01 L-02 L-04 G-30 G-61 G-70 L-07 L-10 G-84 G-85 G-93 G-96`；G-07/G-44/G-81 超时跳过**不计入** ✓；⚠ 本笔 commit message 里那串 ID 是 `8e9622ca` 老基线的、且**「pristine 同 12 条 · 逐条同 ID」不可复现** ✗ —— ⚠ **台账判据环境敏感**：同一份内容在 pristine worktree（同一条 `SOKONANODA_BIN`）跑出 **3 条 + 5 条超时**（`G-12 G-31 G-81 G-92 G-93`）、在本树跑出 **12 条 + 3 条超时** ⇒ **ID 列表不可跨环境当指纹** ✓；存量红的**结构性**证据 = `git diff 69267bbe..61aa45ee -- docs/gaps` **空** + 这 12 条复现件**无一**引用 `course/` ✓）。
- ⚠ **如实**：`course/**` 与 `courses/set-theory/**` 是**两份**语料（前者是教程单元、后者是卷 I 集合论）；a1–a6 那批迁移只覆盖了后者 ✗ ⇒ 本笔补前者 ✓。

## 更早轮次（**原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：`fix-flip3` **第四刀（族 (A)：`@` 脊上裸常量实参按该层 domain 重新 elaborate）** · 引擎侧三刀 + 输入方言迁移（a18）· 发版 `0.94.0` · `land-a15` + 补回 e1 · `SUB-implicit-args-strict-default`（严格档翻默认）· 发版 `0.93.0` · 严格档第 1 批（a1→a5）· 发版 `0.92.0` · `KERNEL-GLOBAL-QUIET-HOOK-SILENCES-PANICS` ✓ （2026-10-19 第六/七刀按本文件自己的规矩 + `docs-lint` ⑦ 腾空间 ⇒ 只留最近 3 轮 ✓）
## 旧轮墓碑（**原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：`CI-GREEN-SELFTEST-NOT-WIRED` · `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO` · `BITE-PROOF-DOES-NOT-BITE-IN-CI` · `SUB-implicit-named-args` · `SUB-implicit-args-align-lean4` · `SUB-release-metric-measures-unreleased` · `CI-GREEN-READOUT-ROTTED` · `SUB-break-the-guard-recursion` · `BITE-PROOF-READS-LIVE-SHARED-LEDGER` · `BITE-EXPECTATION-GOES-STALE-WHEN-READING-IMPROVES` · `LSP-T2B-TRAILING-BY-LE-3-RED-ON-CI` · `E2E-COVERAGE-4-OF-36` · `AGENT-DOES-NOT-PRODUCE-USER-INPUTS`（第三层）✓
## 旧轮（2026-10-15 · **原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：用户五条**激励纠正**（23:xx 亲提 →「你做吧」）· `AGENT-DOES-NOT-PRODUCE-USER-INPUTS`（判据的输入面 = 生成式夹具）· `soko update` 的 404 + `scripts/soko prune` · `#check <def>` 带定义体 + 记法（§9.22）· 清台账第 **7/8** 轮（队列 0 · 隐变量 13/13 收口 · 构建链与缓存身份两族修完）✓
## 未决项（**权威 = `docs/ONBOARDING.md` §0.2**；**2026-10-13 订正**：这里曾写「现存 6 条」而权威表实测 3 条 ⇒ 以权威表为准，本条不再自己数 ✗）。⚠ 本节下方两条是**收口记录**，**不是**未决项 —— 2026-10-02 前它写着"只有这两条"而两条都已 ✅ ⇒ 读它的人会以为"队列空了"（#4/#5/#6 三处用户报障就这样掉线过））
## 已发布线：`v0.79.0`（E19 甲案 + P 组 · 批次 N 66/66）· `v0.80.0` · `v0.81.0` · `v0.82.0` · `v0.83.0` —— 逐轮记录 ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz` ✓ ｜ ⭐ **「最新是哪一版 · 还有多少没发」不许写在这里**（2026-10-16：本行原写「`v0.83.0`（最新 ✓ · 2026-10-08）」而当时真实最新是 `v0.91.0` —— 落后 **15 个 release**（`v0.84.0`…`v0.91.0`，`gh release list` 实测）的**死格** ✗）⇒ **活读数 = `python3 scripts/release-metric.py`**（+ `gh release list` 核已发布资产）✓ —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## 硬事实（接手先读这 6 条 ✓） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
