# G2 实施单：`eqsubst` 的三处 old-style 前缀 —— 让 strict 与 loose 同判

> 缺口报告：`reports/2026-11-01-strict-gaps-before-default-flip.md` 发现 2。
> 起因：S7（翻默认）撞出来的**既有**缺陷 —— `apply Eq.subst _ hx` / `apply Eq.subst (Eq.symm h)` 在 strict 被拒、
> loose 通过。**不是 S7 引入**，是 S6（strict = 内核路径 + 不回落）之后暴露的。

## 0. 一句话

`crates/front/src/compile/eqsubst.rs` 里有 **三处** `Expr::App { explicit_spine: false }`（**:207** motive /
**:296** `Eq.symm` 包装 / **:398** `fn prefix`），注释自称"old-style 前缀"。它们在 strict 档会被内核当成
"位置实参喂隐式位"来数 ⇒ 诊断谎称"你写了 6 个位置实参"（源码只有 2 个）⇒ 拒绝。
同一不变量在别处是守住的（a17 当年只修了**代换**那一路）。**修法 = 把这三种拼接标成 `explicit_spine: true`**。

## 1. 先复现（修前必须红）

```sokonanoda
-- 复现 A（洞）
def Set (α : Type) : Type := α → Prop
def empty (α : Type) : Set α := fun (x : α) => False
theorem t (α : Type) (A : Set α) (x : α) (hx : x ∈ A) (h : A = empty α) : x ∈ empty α := by
  apply Eq.subst _ hx
  exact h
```
实测基线（`097996f6`）：loose **rc=0** / strict **rc=1**（`elab-implicit-argument-unsolved` · 7:3–7:22 +
级联 `elab-tactic-failed` 8:3）。复现 B（谓词型）：`apply Eq.subst (Eq.symm hx) h` ⇒ 同码 10:3–10:32。

## 2. 改哪

三处 `explicit_spine: false` → **`true`**。⚠ 每处都要单独读上下文确认语义：
* `:207` `rewrite_goal_motive`：这里拼的是"motive 前缀 + premise"，前缀位**确实是显式实参**（源码里
  `@Eq.subst` 的用法也是逐位全显式）⇒ 标 `true`；
* `:296` `Eq.symm` 包装：`Eq.symm` 的目标位是**显式**（`Eq.symm : a = b → b = a`）⇒ 标 `true`；
* `:398` `fn prefix`：拼的是 `@Eq.subst` 的 6 个前导位（α motive a b 等）—— 这些是 `@` 全显式写法里的
  **位置实参** ⇒ 标 `true`。

## 3. 顺带修一处文案（同一条缺口里的第二个症状）

诊断的 `@` 出路里会印出引擎合成的 motive 名 **`_soko_eq<offset>`**（只在 `eqsubst.rs:178`/`:365` 拼出）。
`kernel_app_report.rs:223` 已有 `sanitize`（杀 `\0soko_*` 与 `?<数字>`）—— 把 `_soko_eq<数字>` 也纳入，
**并加一条断言**（出路里不许出现 `_soko_eq`）。⚠ 只加 sanitize，不改文案结构。

## 4. 判据

```bash
# ① 复现 A、B 两件：strict 与 loose 都必须 rc=0、0 诊断（修前 strict 是 rc=1）
# ② 既有判据不许红（尤其 tactic 那批）：
cargo test -p sokonanoda-front --lib -- tactic
cargo test -p sokonanoda-front --lib -- eqsubst
cargo test -p sokonanoda-cli --test strict_diagnostics
cargo test -p sokonanoda-front --lib -- kernel_app
# ③ loose 档逐字节不变：`scripts/strict/corpus-hashes.sh` 与 /tmp/s6-after.txt 的 loose 半对齐（0 行）
# ④ 反向验证：把某处 `explicit_spine` 改回 false ⇒ 对应复现件必须判红（三条各验一次，或至少 :207 与 :398 各一次）
# ⑤ 新增 1–2 条单测钉住（复现 A 的 strict 结局 + 出路里无 `_soko_eq`）
```

## 5. 边界

- **不许碰 `crates/kernel/**`**；`elab.rs 8841` 不许涨（本刀不该改 `elab.rs`）。
- `kernel_app_report.rs` 现有 ~412 行，`kernel_app_fallback.rs` 489（软线 500）。
- 不许把 strict 的拒绝"藏起来"（例如给它加例外回落）—— 要**判得出**，不是**不判**。
- 不许 `git commit` / `git push`。
- 落盘后 `python3 scripts/docs-lint.py` 必须绿。

## 6. 交付

代码 + 测试 + 回复：复现 A/B 的两档 before/after（rc + 诊断码 + span）· 三条 `explicit_spine` 各自为什么是 `true`
（读上下文后的理由，不要只写"照单改"）· ④ 反向验证的判红证据 · ③ loose 是否 0 行 · 有没有碰到 kernel/elab。
