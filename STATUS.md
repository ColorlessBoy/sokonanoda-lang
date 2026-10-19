## `fix-flip3` 第六刀 —— **`courses/set-theory/**` 的严格档 `@` 残余**（2026-10-19 · 未发版）
- ⭐ **用户拿到什么**：卷 I 语料的**严格档独有诊断 114 → 97**（旧档本来就没有它们）、`check.py` **rejected 29 → 24** ✓
  （5 个目标转绿：`units/I.4/unit11-universe-russell` · `units/I.10/unit27-quotients` · `solutions/I.11/unit31` ·
  `I.12/unit34` · `I.13/unit41`，**无新增判负** ✓）；两个**判负夹具**（`gaps/C-04-prop-to-type-reject` ·
  `gaps/C-112-cb-direct-def-reject`）从严格档的 `elab-implicit-argument-unsolved` **回到它们登记的那条**
  `kernel-rejected 类型不匹配：期望 Sort(0)，实际是 Sort(1)` ✓ ⇒ G7 守卫恢复「**为正确的理由**咬住」✓。
  用户点名的 `solutions/I.10/unit27-solution`：**记法族那条已消失**（真凶是里层 `Quot.{1} α r` ⇒ `@Quot.{1}`，
  最小复现 `(fun (q : Quot.{1} α r) => …) … = f a` 判红 / `@Quot.{1}` 判绿 ✓），剩 1 条级联。
- **读数**（构建身份 release `sha256:16 = eea22ca0b5919747` · `origin/main = 2fc93146` · 本笔 4 个 sha
  `2c856d4a` `193466f3` `3417f91e` `769114fe`）：`check.py` **rejected 29 → 24** ✓ · `front --lib` **1006/0** ✓ ·
  `protocol` **13/13** ✓ · `notation-lint` **exit 0**（330 文件 · 716 处**已标记**豁免）✓ ·
  **旧档中性**（`SOKO_STRICT_AT=0`，除 `span`/`file`/路径外逐字节）逐遍实测 **10/10 · 8/8 · 12/12 · 2/2** ✓ ·
  反向验证：重放 `04091783` 的已知自伤（`@And.intro … Or.inl (And.right hab)`）⇒ 量具报 `rc 0→1` DIFF ✓（不是空转）。
- **量具（本轮新写，`.cache/ux/fix-flip/u27/`，gitignore）**：`fix_precise.py`（**诊断自己的 span 内 + message 点名的头
  + 位置实参 > 显式形参**三重过滤）· `neutral.py`（旧档中性对拍）· `fix_scan.py`。修掉旧工具两处**盲插**：
  `fix_tactic.py` 的「头名**第一次**成词出现」在 `unit08-solution:155` 会插 `@And.right hx`（**kernel-rejected**：
  `期望 Sort(0)，实际是 (P ∧ Q)`，两档都红 ✗ —— 真凶是同 span 里**第三个** `And.right (A x) (B x) (And.left hx)`）；
  `fix_notation.py` 把 tactic 族收进 span 扫描 ⇒ `unit04-solution` 一处插 19 处 ✗。
  ⚠ 量具自检两条：`--root` 必须给**模块根**（给仓库根 ⇒ 两侧同报 `import-not-found` ⇒ **假绿** ✗）；
  pristine 侧要铺**整个课程**（只铺被改文件 ⇒ 副本没有 `lib/` ⇒ **假绿** ✗）。
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

## `fix-flip3` 第四刀 —— **族 (A)**：`@` 脊上**裸常量实参**按该层 domain 重新 elaborate（2026-10-19 · 未发版）
- ⭐ **用户拿到什么**：`apply Set.ext` 一类**裸头补全望远镜**（`@Set.ext A (A ∪ Set.univ) Set.univ h`）的证明**从必红变判绿** ✓（改前 `期望 (Set α)，实际是 ∀(α : Sort(1)), (Set α)` —— `Set.univ` 按「函数值」elaborate ⇒ 内核看到那个 **Pi** ✗）。复现 p17 / d2 / goalside：**改前三红、改后三绿** ✓。真因 = `application_arg_expected` 的 `layers.get(index)`（`index` = 头**已吃掉**的实参数）在**偏应用**上取不到（`@Set.ext α A`：头 2 实参、类型只剩 1 层 Π）⇒ 正解 = Lean `Elab/App.lean:263` `getArgExpectedType` 的「偏应用第 0 个 Π 域」（`:281` `addNewArg` / `:708` / `:1881`；`Meta/Tactic/Apply.lean:160/205/223` ⇒ σ 是内核项 ✓）。修法 = 新 `named_arg::at_arg_expected`（`explicit_spine` + 「有前导隐式的已知常量」两道免费闸门），`apply` 那条搬成 `named_arg::next_arg_expected` **共用一份** ✓。
- **读数 + 反向验证**：`front --lib` **1006/0** · `protocol` **13/13** · `check.py` **rejected 29** · 旧档 **252/252 逐字节相同 · DIFFS=0** · `clippy` 0 错 · `audit-notation-paths` 0 新增 · `elab.rs` **8822/8826**（净减 4 ⇒ 行额未抬）✓；新守卫 `at_spine_bare_constant_argument_gets_its_layer_domain` 在**基线源码**上**判红** ✓、探针（已还原）实测本刀在 p17 上**确实触发一次** ✓。
- ⚠ **如实**：只覆盖 `@` 脊（跨 `@` 边界仍走老路 ✗）· `g1b` 与 p17 同形却不同命（基线本就绿，差异未定位 ✗）· 台账 `--strict` 本树与 pristine 同 **12 条**、逐条同 ID（存量红 ⇒ `--no-verify`）✓。**原文**：`git log --all -- STATUS.md` ✓

## 更早轮次（**原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：`fix-flip3` 引擎侧三刀 + 输入方言迁移（a18）· 发版 `0.94.0` · `land-a15` + 补回 e1 · `SUB-implicit-args-strict-default`（严格档翻默认）· 发版 `0.93.0` · 严格档第 1 批（a1→a5）· 发版 `0.92.0` · `KERNEL-GLOBAL-QUIET-HOOK-SILENCES-PANICS` ✓ （2026-10-19 第六刀按本文件自己的规矩 + `docs-lint` ⑦ 腾空间 ⇒ 只留最近 3 轮 ✓）
## 旧轮墓碑（**原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：`CI-GREEN-SELFTEST-NOT-WIRED` · `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO` · `BITE-PROOF-DOES-NOT-BITE-IN-CI` · `SUB-implicit-named-args` · `SUB-implicit-args-align-lean4` · `SUB-release-metric-measures-unreleased` · `CI-GREEN-READOUT-ROTTED` · `SUB-break-the-guard-recursion` · `BITE-PROOF-READS-LIVE-SHARED-LEDGER` · `BITE-EXPECTATION-GOES-STALE-WHEN-READING-IMPROVES` · `LSP-T2B-TRAILING-BY-LE-3-RED-ON-CI` · `E2E-COVERAGE-4-OF-36` · `AGENT-DOES-NOT-PRODUCE-USER-INPUTS`（第三层）✓
## 旧轮（2026-10-15 · **原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：用户五条**激励纠正**（23:xx 亲提 →「你做吧」）· `AGENT-DOES-NOT-PRODUCE-USER-INPUTS`（判据的输入面 = 生成式夹具）· `soko update` 的 404 + `scripts/soko prune` · `#check <def>` 带定义体 + 记法（§9.22）· 清台账第 **7/8** 轮（队列 0 · 隐变量 13/13 收口 · 构建链与缓存身份两族修完）✓
## 未决项（**权威 = `docs/ONBOARDING.md` §0.2**；**2026-10-13 订正**：这里曾写「现存 6 条」而权威表实测 3 条 ⇒ 以权威表为准，本条不再自己数 ✗）。⚠ 本节下方两条是**收口记录**，**不是**未决项 —— 2026-10-02 前它写着"只有这两条"而两条都已 ✅ ⇒ 读它的人会以为"队列空了"（#4/#5/#6 三处用户报障就这样掉线过））
## 已发布线：`v0.79.0`（E19 甲案 + P 组 · 批次 N 66/66）· `v0.80.0` · `v0.81.0` · `v0.82.0` · `v0.83.0` —— 逐轮记录 ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz` ✓ ｜ ⭐ **「最新是哪一版 · 还有多少没发」不许写在这里**（2026-10-16：本行原写「`v0.83.0`（最新 ✓ · 2026-10-08）」而当时真实最新是 `v0.91.0` —— 落后 **15 个 release**（`v0.84.0`…`v0.91.0`，`gh release list` 实测）的**死格** ✗）⇒ **活读数 = `python3 scripts/release-metric.py`**（+ `gh release list` 核已发布资产）✓ —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## 硬事实（接手先读这 6 条 ✓） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
