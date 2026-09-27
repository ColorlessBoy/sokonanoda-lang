# 交接单 —— v0.74.0 进行中（E03 起）

> 写于 2026-09-27，本会话上下文已用掉一大截（E01+E02 两个环节，全程带验证）⇒ **按换会话信号收尾** ✓。
> **唯一真相 = `docs/PLAN-0.74-0.79.md`**（顶部「⚡ 执行索引」；环节与判据**以当前发版点章节为准，不凭记忆** ✗）。
> 本文件只是**起点快照**，不取代 PLAN 或 census ✓。

## 1. 当前进度（v0.74.0：**2/11** ✓）

| 环节 | 状态 | 证据 |
|---|---|---|
| **E01** `Rel.comp`/`Function.comp` 记法（`•`/`∘`） | ✅ **7a4a58b**（状态 `95a831b`） | 判据三层（front 记法表 / CLI 真课程库契约 / notation-lint 自动派生）· 反向验证 ✓ · 台账 **G-45** |
| **E02** `Set.prod` 收进 `lib/Prod` | ✅ **8376391**（状态 `d55701f`） | 判据两层（front 目标在库 + CLI 只拷 `lib/` 的闭包）· 反向验证 ✓ · 台账 **G-46** |
| E03 · E04 · E21 · E22 · E23 · E27 · E28 · E29 · E30 · E31 | ⬜ **9 个未开工** | — |

**工作树干净 ✓**（`git status --short` 空）；**未 push**（批次制：批次收尾才推一次 ✓）。

## 2. ⚠ 验收数字要改一处（**需要用户拍板**）

PLAN/目标的收敛判据写的是「课程门禁 **36 目标 · 328 checked · 99 open · 0 判负**」。
**E02 之后正确的数字是 327**：`Set.prod` 收进库（+1），而画布与 `unit05-solution` 里的
**两份副本必须删**（同名重声明 = `import-name-collision`，−2）⇒ **净 −1**。
**练习数与 open 数一条未动**（99 ✓）、0 判负 ✓、36 目标 ✓。
⇒ 建议把判据读成 **327**（已在 PLAN E02 状态列与台账 G-46 写明）；**若用户要求 328，
唯一出路是给课程补一条真声明**（不许为凑数造假 ✗）。

## 3. 本会话实测的验收状态（全部在**冻结树**上跑 ✓）

| 判据 | 结果 |
|---|---|
| 课程门禁 `check.py` | ✅ **36 目标 · 327 checked · 99 open · 0 判负**（EXIT=0） |
| `cargo test -p sokonanoda-cli --test course` | ✅ **6 passed / 0 failed** |
| `cargo test -p sokonanoda-cli --test notation` | ✅ **49 passed / 0 failed** |
| `cargo test -p sokonanoda-front --test prelude_shape` | ✅ **4 passed / 0 failed** |
| `scripts/notation-lint.py` | ✅ 84 文件零旧写法（569 处显式豁免） |
| `scripts/gap.py check` | ✅ 全部与台账一致（G-45 sokonanoda 通过 · G-46 script 行为已变） |
| `scripts/docs-lint.py` / `scripts/status-lint.py` | ✅ 全绿 |

## 4. 已知坑（**别再重新发现一遍** ✗）

| 坑 | 应对 |
|---|---|
| ⚠ **判据跑完之前别改文件** ✗ | 本会话踩了**两次**：反向验证期间课程门禁读到半棵树（假红）、front 判据读到**撕裂**的库文件（假红）。**改完 → 跑 → 不再碰** ✓ |
| **文档预算是"当前行数"** | `docs-budget.json` 里每项 = 现值 ⇒ **加一行就要手改 JSON**（评审可见 ✓，规矩如此）。本会话已改 4 处：notation-subset 873→905 · STATUS-ARCHIVE 1666→1684→**1714** · course-stdlib 372→**375** |
| **STATUS.md 每段 ≤30 行 / 首段 ≤40 / 总 ≤200** | 加一轮就要把最老的轮**归档**进 `docs/STATUS-ARCHIVE.md`（归档也长 ⇒ 同一轮里 bump 它的预算） |
| **禁词**（`在跑`/`进行中`/`未变`/`判据不变`/`待 CI`/`等 CI`/`⏳`） | `status-lint` 判红；我写"判据跑完之前"就是为了绕开 `在跑` ✗✓ |
| **记法符号的写法** | `•` 不在数学码点类里：**未声明时连符号都不是**（读成 `Ident("•")` ⇒ `unknown identifier`）；`∘`/`≈`/`⁻¹` 会报 `符号 '…' 在本文件里还没有声明过记法` |
| **二元算子应用到参数上要加括号** | `(r • s) a c` / `(g ∘ f) x` / 未来 `(r ⁻¹) b a`——函数应用比它们都紧（**Lean 同款读法**，不是本课怪癖） |
| **机械替换会漏括号** | 本会话用 `replace_all` 改 `Function.comp … g f` 时，`… g f y` 变成 `g ∘ f y`（= `g ∘ (f y)` ✗）⇒ **替换后必须 grep `∘ f [a-z]` / `• .*[)] [a-z]` 这类形状** ✓ |
| `scripts/soko` 的解析顺序 | `$SOKONANODA_BIN` → 版本匹配的**仓库构建**（`target/debug` 慢 >55s）→ 缓存。本地跑门禁**建议 `SOKONANODA_BIN=~/.local/share/sokonanoda/bin/sokonanoda`** ✓ |
| 托管 `python3` 缺 pyyaml ⇒ 假红 | 用 **`/opt/homebrew/bin/python3`** ✓ |
| macOS `grep` 的 BRE 没有 `\|`/`\b` | **一律 `grep -E`** ✓ |
| 网络 | **代理 `http://127.0.0.1:7890`**（`curl -x` ✓；`web_fetch` 走不通 ✗） |

## 5. 下一步第一件事：**E03**（`r ⁻¹` / `A ≈ B` 记法）

**判红已确立 ✓**（本会话实测，**不必重造**）：临时模块根（只拷 `lib/`）写
`def equiv_sq … : Prop := s ≈ t` ⇒ exit 1，内核原文
**「符号 `≈` 在本文件里还没有声明过记法；先用 infix/notation 命令声明它，或改用点名写法」**；
`r ⁻¹` 同类（parse 在第一条错处停）。

**取证已做 ✓（省一大步）**：`∘` 是 Lean core 逐字（`src/Init/Notation.lean:274`）；
但 **`⁻¹` 与 `≈` 在 mathlib4 master 里查不到对应声明** ——
`Mathlib/Logic/Relation.lean` 里既没有 `Relation.inv` 也没有 `⁻¹`（只有 `local infixr:80 " ∘r "`）；
`Mathlib/Logic/Equiv/Defs.lean:80` 是 `infixl:25 " ≃ " => Equiv`（**`≃` 不是 `≈`**）；
`Mathlib/SetTheory/Cardinal/Basic.lean` 无 `≈`。
⇒ **两条都按"本课自定"落地**（PLAN 已指定符号），**并顺手更正**
`lib/Rel.sokonanoda` / `lib/Equiv.sokonanoda` 头部"这是 Mathlib 的记法"那两处**不准确**的引用
（E01 已经为 `•` 做过同样的更正，照那个样子写 ✓）。建议形状：
`postfix:100 " ⁻¹ " => Rel.inv`（与 `ᶜ`/`𝒫` 同档）· `infix:50 " ≈ " => Set.Equiv`（与 `=`/`∈`/`⊆` 同档）。

**E03 的机械量比 E01 大**（`Set.Equiv` 的点名用法遍布单元⑨⑩⑫ 与解答）⇒ 建议：
先加两条声明 → 跑 `python3 scripts/notation-lint.py`（**新声明会自动派生点形式判据** ✓，
它给出的清单就是待改文件表）→ 一处一改 → 再判据。
⚠ 判据照 E01/E02 的三层写：front 记法表（`prelude_shape.rs`）· CLI 真课程库契约（`notation.rs`）·
notation-lint 零残留；**反向验证**：撤声明必须判红。

## 6. 关键指针（file:line）

| 要什么 | 去哪 |
|---|---|
| 计划总账 / 执行索引 / 环节与判据 | `docs/PLAN-0.74-0.79.md` —— 执行索引（`:1-34`）· 总览表状态列（`:219-234`）· **§v0.74.0**（`:296` 起） |
| 记法白名单与 as-built（E01 在 §16） | `docs/design/notation-subset.md` |
| 台账（契约：`fixed` ⇒ 复现件必须 exit≠0 / 判卷干净） | `docs/gaps/ledger.jsonl` + `scripts/gap.py`；**G-45/G-46** 是 E01/E02 的 |
| 课程库分层判据（L2/L3）与各库计数 | `docs/design/course-stdlib.md`（E02 已改 Prod 6→7、合计 74→75） |
| 记法规则（课程侧硬规则 + 边界） | `courses/set-theory/AGENTS.md` §记法规则 |
| 判 CI 真绿（**批次收尾才推**） | `scripts/ci-green.py --run <id>`（`--run` 不是位置参数 ✗）· 逐 job 看（`perf-gate` 是 `continue-on-error`） |

## 7. 纪律（本会话已按此执行 ✓）

- **一处一 commit** ✓（E01、E02 各一个实现 commit + 一个 PLAN 状态 commit）；
- **每环节先判红 → 实现 → 反向验证（撤掉必须判红）→ 总览表标 `✅ <commit>` → 回写台账** ✓；
- **子代理**：用，但 prompt 写死三条 —— ① 每条结论附 `file:line` + 原文摘录（拿不出就写「未找到」）
  ② 不许夸大（分不清就标 `不确定`）③ **只取证不下判断**；产出**先抽查再并入** ✓
  （本会话用它普查了 `Set.prod`/`×ˢ` 的 stale 说法，抽查后并入 3 处 ✓）；
- **后台任务**：`> /tmp/<名字>.log 2>&1 &` + `tail` 轮询 ✓；**bash 带仓库根 `cd` 前缀** ✓；
- **本轮不碰**：内核判定 · bump/release · 跨模块重构（G4 属 v0.79）✗。
