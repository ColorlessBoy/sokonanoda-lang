# 交接单 · 课程线（卷 I《集合论》）—— 2026-10-03 · **C-112 已交付 + #5 普查第 2 步跑通**

> **给下一条干净会话**：入口按 §4 开；**先读本文 + `courses/set-theory/OPEN-ITEMS.md`**，别凭印象 ✓。
> 队列权威 = `docs/ONBOARDING.md` **§0.2**；大点路标见值守单。

## 1. 上一棒（C-112 线）留下的确切状态

- ✅ **单元112（Cantor–Bernstein）已交付** ✓：画布 `units/I.3/unit112-cantor-bernstein.sokonanoda`
  （20 条给定件 + **12 条练习**）· 解答 `units/solutions/I.3/unit112-solution.sokonanoda`
  （**32 checked · 0 open · 0 判负** ✓，**全项风格、无 `by` 块**）· `course.json` 章 **I.3** ✓。
- ✅ **CB 经选择公理**（`choice` 取两次）——**偏离标注**写在画布头 + `gaps/C-112-cantor-bernstein.sh`
  + `OPEN-ITEMS.md` C-112 ✓（**零公理 CB 仍缺 G-6 一族**，open ✗）。
- ✅ **复现件已翻成 `closed-green`** ✓：三条断言（零件在 / 主定理在位且判绿 / **墙仍在**）+ 三段反向验证
  实测（换 `sorry` ⇒ exit 1 ✓ · 注入"墙消失" ⇒ exit 1 ✓ · 恢复 ⇒ exit 0 ✓）。
- ✅ **`notation-lint.py --census`**（#5 第 2 步 ✓）：**只加统计、判据不变** ✓ ⇒ 1876 处命中
  （可机械换 **629** / 需手工 **1210** / 注释 **37** / 空转 **0**）；**试改 71 → 绿 46 / 红 25** ✓
  （口径与实数：`gaps/notation-ok-census.md` ✓）。`notation-lint` 全课程 **exit 0** ✓
  （此前因 WIP 件的旧写法是红的 ✗）。
- **门禁实测（交付后）**：**249 个目标 · 2072 checked · 903 open · 0 判负** ✓。

## 2. 下一步（别猜，按队列走）

1. **#5 整批适配**：仍**等内核 IA-4 的 U1/U2** ✗（值守护栏）⇒ 现在能做的是
   「可机械换」剩下的 **583 处**批量试改（工具已就绪：`--census --census-limit 0` ✓）与
   「需手工」**1210 处**的**真改写器**（点名 → 中缀要**重构表达式** ⇒ 不是正则活 ✗）；
2. 队列 §0.2 里 #5 之后的项按值守单走 ✓。

## 3. 口径与坑（本棒实测，务必先读 ✓）

- **本子集的三条引擎边界**（都实测过 ✗ ⇒ 写解答时直接绕开）：
  ① 同一个 `cases` 的**两条臂都再嵌一个 `cases`** ⇒ **声明被吞掉** ✗（名字都查不到 ✓）；
  ② `Exists.intro` 的**字面 λ 谓词**判红 ✗（改用**具名 def 谓词** ✓）；
  ③ `Eq.{1}` 出现在 **`have` 的类型位**会解析错位 ✗。
  ⇒ 单元112 的解答因此是**全项风格 + `Or.elim` 证明项** ✓（嵌套分支一律不用 `cases` ✓）。
- **`drop-args` 改写不是安全动作** ✗：`And.left A B h` ⇒ `And.left h` 在 **`h` 的类型是 def-应用**时判红
  （35% 实测红 ✓）⇒ **先逐处测再改**，禁止批量正则替换 ✗。
- 沿用的老坑：**路径限定提交** ✓（禁 `git add -A` ✓、提之前 `git status --short` 逐条核 ✓）·
  **bisect 之后立刻删探针**（`zzprobe*.sokonanoda` ✓）· 解答**必须** `unitNN-solution.sokonanoda` ✓ ·
  改文档脚本**带断言 + 看退出码 + grep 复核命中数** ✓。

## 4. 入口 / 判据 / 实测数字

**入口**：`courses/set-theory/OPEN-ITEMS.md`（C-01…C-05 · C-110 · C-111 · **C-112 closed** ✓）·
`gaps/C-112-cb-derivation.md`（案情）· `gaps/C-112-cb-step5.sokonanoda`（**32 条 = 单元解答的来源** ✓）·
`gaps/notation-ok-census.md`（#5 实数 + 工具 ✓）· `courses/set-theory/AGENTS.md`（课程手册 ✓）。

```bash
python3 courses/set-theory/tools/check.py                    # 全量门禁（~10 分钟）⇒ 0 判负
python3 courses/set-theory/tools/audit-pairs.py --unit 112    # 画布/解答对账（3 秒，缺名 0 ✓）
python3 scripts/notation-lint.py                              # 记法门禁 ⇒ exit 0（零旧写法 ✓）
python3 scripts/notation-lint.py --census-selftest             # 普查改写器反向验证（8/8 ✓）
python3 scripts/notation-lint.py --census --census-limit 40     # 小批量试改 ⇒ 「绿 x / 红 y」
bash courses/set-theory/gaps/C-112-cantor-bernstein.sh          # C-112 复现件（closed ⇒ exit 0 ✓）
```
