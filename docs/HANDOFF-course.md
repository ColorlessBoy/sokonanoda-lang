# 交接单 · 课程线（卷 I《集合论》）—— 2026-10-02 · 会话到线（14.2 MB / 压缩 8 次）

> **给下一条干净会话**：入口按 §4 开；**先读本文 + `courses/set-theory/OPEN-ITEMS.md`**，别凭印象 ✓。
> 队列权威 = `docs/ONBOARDING.md` **§0.2**；大点路标见值守单。

## 1. 手上这一棒的确切状态（`1948d969` 之后）

**单元 112（Cantor–Bernstein）—— 数学全部推完，剩写 Lean + 建单元** ✓：

- ✅ **已判绿 7 条**（判据文件 `courses/set-theory/gaps/C-112-cb-prop.sokonanoda`，
  重放：`python3 /tmp/soko/bisect.py <该文件>` ⇒ **ALL GREEN · checked=7**）：
  `cbK`（**单调**算子 `A ∖ g '' (B ∖ f '' C)`）· `cbFix` · `cbC0 = ⋂𝓕` · `cbK_mono` ·
  `cbC0_least` · **`cbC0_in_fix`**（`K(C₀) ⊆ C₀`）· **`cbC0_sdiff`**（`A∖g''B ⊆ C₀`）。
- ✅ **第 5 步案情推完**（`courses/set-theory/gaps/C-112-cb-derivation.md` §8）：
  关系版图 `cbRel x y := (x ∈ C₀ ∧ f x = y) ∨ (x ∉ C₀ ∧ g y = x)` ✓；四件（全/单值/单射/满射）
  逐情形用哪条已绿事实已列表化 ✓。
- ⏭ **唯一还缺的零件 = `cbC0_gf`**（`x ∈ C₀ ⇒ g (f x) ∈ C₀`）：只需 `g (f x) ∈ K(C₀)`
  （用已绿的 `cbC0_in_fix` ✓）+ `g (f x) ∈ A`（前提 `hgA`/`hfB`）+ `g (f x) ∉ g '' (B∖f '' C₀)`
  （反证 + `hg_inj`）⇒ **一条短引理** ✓。
- ⚠ **卡在哪条判据**：**没有卡在任何判据** ✗ —— 门禁健康（**246/2020/891/0** ✓，G7 **7 条 · 0 条不一致** ✓），
  树上**无半成品** ✓（WIP 在 `gaps/` ✓，不登记进 `course.json` ✓）。卡的是**上一会话的上下文余量** ✓。
- ⚠ **写单元时要显式写四条前提**（本课没有"`A → B` 的函数"类型）：
  `hfB : ∀ x ∈ A, f x ∈ B` · `hgA : ∀ b ∈ B, g b ∈ A` · `hf_inj` · `hg_inj` ✓。

## 2. 下一步要做的两件事

**(a) 112 收口（第一优先）**
1. 写 `cbC0_gf`（一条短引理）⇒ `cbRel` ⇒ 四件 ⇒ 用 **`choice`** 把"全 + 单值"的关系**取出成函数**
   ⇒ 装配 `A ≈ B` ✓；
2. **建单元 112**：画布（把已绿 7 条 + `cbRel`/四件编成练习）+ 解答（`unit112-solution.sokonanoda` ✓）
   + `course.json` 登记（配额 / 练习数）⇒ 跑全量门禁 ⇒ **`failed=0`** ✓；
3. **交付那一刻**翻 C-112 复现件语义（**三件事一起做** ✓）：改 `closed` · 期望翻正（"CB 主定理在位且判绿"）·
   反向验证改成"拿掉/换 `sorry` ⇒ 判红" ✓。**交付前一刻都不要提前翻** ✗。

**(b) `notation-lint.py --census`（第二件，30–50 处小批量先验口径）**
- **只加能力、不改任何判据、不动豁免语法** ✗（不许为出数好看松动守卫 ✓）；
- 按类产出候选：**可机械换的**（"删前导实参"类：`And.intro A B h1 h2`→`And.intro h1 h2`、
  `Exists.intro A p w h`→`Exists.intro w h` 等）直接生成 ✓；**点名→中缀**（`Set.mem α a A`→`a ∈ A`、
  `Eq.{1} T a b`→`a = b`）要**重构表达式** ⇒ 标 **`needs-manual`** ✓，**别用正则硬咬** ✗
  （本仓有"正则守卫咬不住 ⇒ 不发布"的失败实录 ✓）；
- 产出「**试改 N → 绿 x / 红 y**」+ **可机械换 / 需手工**两类计数 ✓。

## 3. 值守给的口径与坑（照办 ✓）

- **路径限定提交**：`git commit -- <明确路径>` ✓；**禁 `git add -A` / `git commit -a`** ✗
  （两线共用一个 index，10-02 已双向交叉提交过一次 ✓）；**提之前 `git status --short` 逐条核** ✓；
  **不碰 `crates/**` 与 `docs/gaps/**`** ✗（内核线在用 ✓）；**只留本地、不要自己推** ✓。
- **一环节一提交** ✓；**不许留 todo 降级记账** ✗；**改了判据必须有反向验证** ✓（能证明"撤掉改动会变红"）✓。
- **普查阶段只统计 + 记账** ✗：**别提前改课程写法**（整批适配的前置 = 内核 IA-4 的 **U1/U2** ✓）。
- **可测口径（已实证）** ✓：逐处取**原始片段**（豁免登记自带 ✓）→ 生成记法候选 → **在副件里**跑 `grade`
  → 出「试改 N → 绿 x / 红 y」✓。⚠ 注意：`soko:notation-ok` 只是 `notation-lint` 的**豁免标记** ✗
  ⇒ **删掉它跑 `grade` 结果不变** ⇒ "删标记 → 跑 grade"是**空转普查** ✗（这条已纠正值守原口径 ✓）。
- **副件不污染主树** ✓：放临时目录（**注意要留在课程树内**才有 `lib.*` 模块根 ✓，
  且**别命名成 `unit*`** ✗ —— `audit-pairs.py` 会扫 `unit*.sokonanoda` ✓），跑完即弃 ✓。
- **CB 偏离标注不许弱化** ✓（写进单元头 + 提交信息 + `OPEN-ITEMS.md`）：
  「本课 CB **经选择公理**（用 `choice` 取前像函数），而 **CB 数学上不需要选择**
  ⇒ **这是偏离，不是"标准证法"**」✓；零公理 CB 要 **G-6** 一族（仍 open ✓），继续挂在「仍写不了」里 ✓。

## 4. 下一棒需要的入口 / 判据 / 实测数字

**入口文件**：`courses/set-theory/OPEN-ITEMS.md`（C-01…C-05 · C-110 · C-111 · **C-112** 全在册 ✓）·
`courses/set-theory/gaps/C-112-cb-derivation.md`（推导件 ✓）· `gaps/C-112-cb-prop.sokonanoda`（7 条绿 ✓）·
`gaps/notation-ok-census.md`（普查设计与实数 ✓）· `courses/set-theory/AGENTS.md`（课程手册，踩坑全在里面 ✓）·
`docs/design/set-theory-unlocked.md`（§四 顺序 ✓）· `courses/set-theory/README.md`（门禁行 + 现状表 ✓）。

**判据命令**：
```bash
python3 courses/set-theory/tools/check.py            # 全量门禁（~10 分钟）⇒ failed 必须 0
python3 courses/set-theory/tools/check.py --gaps-only # 秒级：重放 gaps/*.sh，0 条不一致
python3 courses/set-theory/tools/audit-pairs.py --unit <N>   # 画布/解答对账（3 秒，抓命名坑）
python3 scripts/docs-lint.py && python3 scripts/status-lint.py
bash courses/set-theory/gaps/C-112-cantor-bernstein.sh       # C-112 复现件（登记 open ⇒ 期望 exit 0）
```

**当前实测数字**：
- 门禁 **246 目标 · 2020 checked · 891 open · 0 判负** ✓；G7 **7 条复现件 · 0 条不一致** ✓；
  单元 ⑬–**111** 已交付 ✓（112 未交付 ✗）。
- 普查（#5）：豁免 **1579** 处 ✓（§0.2 自述 1584，差 5 = 整文件豁免的 cheatsheet ✓）·
  **`units/solutions/` 1210（77%）** · `units/` 334 · `lib/` 35；
  按类：**D 类（`⋃`/`⋂`/`𝒫` 无记法）≈235 可直接跳过** ✓ · "删前导实参"类**可机械换** ✓ ·
  "点名→中缀"类**需手工/半自动** ✗；**小批量（30–50 处）尚未跑** ✗（0 试改 / 0 绿 / 0 红）。
- 最近提交：`1948d969`（第 5 步案情）· `1d705ea8`（7 条绿）· `c649582e`（推导件）· `b478426e` · `39b1a526`（普查实数）。

**已知的坑（都踩过 ✓）**：**没亲手写过的库函数先 `sed` 读签名** ✓（我踩 5 次：`Exists.elim` 隐式实参 ·
`choice_spec` · `mem_sInter_iff` 实参序 · `image_mono` 的 `{α β}` 隐式 · 像 `g '' D` 的**见证域在 `β`**）·
解答**必须** `unitNN-solution.sokonanoda` ✓（踩 4 次，生成后先跑 `audit-pairs` ✓）·
bisect 探针有**四处**（`units/` · `units/solutions/` · `lib/` · **`gaps/`**）✓ ·
heredoc 定界符别与内容撞 ✓ · 改文档脚本**带断言 + 看退出码** ✓（我有一笔提交说明与文件内容不一致，就是脚本 SyntaxError 静默没跑 ✓）·
门禁全量 ~10 分钟（前台会转后台 ✓）。
