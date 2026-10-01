# G-58 设计稿：**归纳命题的大消去**（`Prop` 入、`Type` 出）

> 状态：**设计稿（待用户批准）** · 2026-10-01 · 台账条目 **G-58（blocker）**
> 红线：**判定正确性不变**（REQUIREMENTS §2 第 1 条）—— 接受/拒绝只在**本条要放开的那一类**上变化，
> 事件计数、golden、`--json` 逐字节不变；改动带三层回归（kernel `tests/` + front 单测 + CLI e2e）。
> ⚠ **本稿只写设计与验收判据，不改任何代码** ✗。

## 1. 症状（台账实测，可复现）

```sokonanoda
def andToType (A B : Prop) (h : A ∧ B) : Type :=
  And.rec A B (fun (_ : A ∧ B) => Type) A B h
-- ⇒ kernel-rejected：期望 `Pi (x : And $2 $1), Sort(0)`，实际是 `Pi (_ : And $2 $1), Sort(2)`
```

**对照**：把动机换成落在 `Prop` 的（`And.rec A B (fun _ => B) …`）⇒ **checked** ✓。
⇒ 不是"recursor 写错"，而是**动机落 `Type`** 被拒 ✓。

**自定义归纳命题同病**：`MyAnd (a b : Prop) : Prop`（单构造子、字段全是 `Prop`）也拒 ✗。

## 2. 影响面（为什么它是 blocker）

| 影响 | 具体 |
|---|---|
| 序数线 | `rank : (Prop 值归纳块) → 序数` 写不出来 ⇒ **ST7 过不去**（与 G-56 是**两条独立**的墙 ✓） |
| 课程反例练习 | 要区分两个构造子（如 `Bool.false ≠ Bool.true`）就得造 `Bool → Prop` 的谓词 ✗ ⇒ **所有"举反例"类练习做不了** ✗（单元㉝ 的「`𝒫(A∪B) ⊆ 𝒫A ∪ 𝒫B` 是假的」只能写在注释里 ✓） |
| 库 | 任何"归纳命题 ⇒ 数据"的翻译函数（`IsRegular`、`IsWellFounded` 的**提取**）都受限 ✗ |

## 3. 目标语义（对齐官方 Lean 4）

`Init/Logic.lean` 里 `And.rec` 的动机是 `Sort u` ✓ —— **单构造子的归纳命题**，
只要**每个非 `Prop` 字段都是参数或指标**，就允许大消去 ✓（内核注释
`crates/kernel/src/inductive.rs:1150-1163` 已写着这条规则与例子 `MyTypeLarge` ✓，
**但那个例子实测也被拒** ✗ ⇒ 实现与注释不符 ✓）。空归纳命题（无构造子）同样允许 ✓。

## 4. 定位计划（先诊断，后动手）

1. 在 `mk_elim_level` 打点：打印 `large_elim_test` 的返回值与它检查的每个构造子字段的
   `is_prop / is_param / is_index` 三元组 ✓（**不改语义**，只加临时探针 ✓）；
2. 用台账的**两个最小例**（`And` / `MyAnd`）当输入 ✓ ⇒ 预期看到"字段全 Prop 却被判 false" ✗；
3. 若 `large_elim_test` 本身正确，再查 `mk_motive_dep` 是否**在 `Some(zero())` 时覆盖了
   正确的 `elim_level`** ✗（嫌疑最大：注释说允许、实测拒绝 ⇒ 中间某处把它降级 ✓）；
4. **根因写回台账** ✓（`where.note` 从"根因未定位"更新为具体行 ✓）再动手 ✓。

## 5. 改动草案（仅示意，实施前先过 §4）

- `mk_elim_level`：`large_elim_test` 为 true 时**不要**把 `elim_level` 钉在 `zero()` ✓；
- `mk_motive_dep`：动机的 codomain 用**用户给的** `Sort u` ✓（而非强制 `Sort 0`）；
- 保持 `Prop` 侧行为逐字节不变 ✓（`And.rec A B (fun _ => B) …` 仍 checked ✓）。

## 6. 验收判据（**可执行**）

| # | 判据 | 命令/位置 |
|---|---|---|
| A1 | 台账两个最小例**判绿** | 新 repro：`docs/gaps/repro/G58-large-elimination.sh`（约定：修好 ⇒ exit≠0 ✓） |
| A2 | 反例类练习可做：`Bool.false ≠ Bool.true` 判绿 | 同上脚本内第二条探针 ✓ |
| A3 | **拒绝面不回退**：现有全部"应当判红"的语料**仍判红** | `cargo test -p sokonanoda-kernel` + front 单测 + CLI e2e ✓ |
| A4 | 事件计数与 golden **逐字节不变**（除 A1/A2 新增允许的用例） | `scripts/soko gate` + 对拍 ✓ |
| A5 | 课程门禁保持 | `python3 courses/set-theory/tools/check.py` ⇒ **127+/1226+/503/0** ✓（只增不减 ✓） |
| A6 | 关账 | `python3 scripts/gap.py close G-58 --version <新版本>` ✓ ⇒ `gap.py check` exit 0 ✓ |

## 7. 解锁的课程内容（批准后的第一批交付）

1. **单元55：反例与不可证**（`Bool.false ≠ Bool.true` · 「`𝒫(A∪B) ⊆ 𝒫A ∪ 𝒫B` 的反例」·
   `⋂F ∪ ⋂G ⊄ ⋂(F∪G)` 的反例 ✓）—— 这是**教材里成体系的一块**（Enderton §1.5 的
   "refuting statements" ✓），课程目前**整块缺失** ✗；
2. **序数线重启**（与 G-56 一起）：`rank` · 序数算术 ✓；
3. `lib` 的提取函数（`IsWellFounded` ⇒ 极小元函数 ✓）。
