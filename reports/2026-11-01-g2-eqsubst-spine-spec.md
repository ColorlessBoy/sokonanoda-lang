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

## 7. 实施回执（2026-11-01，已落盘；**改了两处与本单不同的决定，附实测证据**）

分支 `strict/2026-10-29-kernel-app`，**没有 `git commit` / `git push`**。

### 7.1 `:296` **不标 `true`**（本单 §2 第二条被实测推翻）

`:296` 是 `Eq.symm` 的**一层包装**（`Eq.symm <等式>`，**1** 个实参），不是 6 位前缀：
`Eq.symm : {α : Sort u} → {a b : α} → @Eq α a b → @Eq α b a` ⇒ 源码里那 1 个位置实参绑的
正是**显式**形参（正常形式）。标 `explicit_spine: true` = 声明"这条脊逐位对**整条**望远镜"
⇒ 渲染成 `@Eq.symm h` ⇒ `h` 被绑到 `{α : Sort u}`。

反向验证（复现件 C：`apply Eq.subst h hx`，`h : x = a`、`hx : a ∈ A`、目标 `x ∈ A`
⇒ 引擎必须包一层 `Eq.symm`）：标 `true` ⇒ **loose 与 strict 都** `rc=1`，
`elab-tactic-failed`「`apply` 的被应用项没有通过内核检查：期望 `Sort(0)`，实际是 `(((Eq.[1] α) x) a)`」；
还原成 `false` ⇒ 两档 `rc=0`、0 诊断。⇒ 这一位**必须留 `false`**（已加单测钉住）。

`:207`（`rewrite_goal_motive` 的 5–6 位内联前缀）与 `:398`（`fn prefix` 的 6 位前缀）
按本单标 `true` ✓ —— 两处都反向验证过（见 §7.4）。

### 7.2 本单没写到的**第四处改动**：`named_arg::at_arg_expected` 补"洞"那一支

只改那三处（`:207`/`:398` → `true`）时，复现 A（`apply Eq.subst _ hx`，**洞在等号位**）
在**两档**都变红：`elab-hole-misplaced`（`sorry` is only allowed as the value of an open
exercise）。根因：旧路的 old-style 读法会给每个位置实参喂"这一层的域"（`compile/hole.rs:177`
就是这句），而 `@` 脊走的是通用应用臂，`at_arg_expected` 的便宜闸门要求"实参是常量" ⇒ 洞恒 `None`。

修法（8 行，`named_arg.rs`，**不碰 `elab.rs`**）：`@` 脊上实参是 `Expr::Hole` 时返回
`next_arg_expected(fun, …)`（= 偏应用的第 0 个 Π 域）；与 Lean `Elab/App.lean:263`
`getArgExpectedType` 对**任何**实参都成立同构。修后复现 A 两档 `rc=0`。

### 7.3 `sanitize` 的**位置**要下沉（本单 §3 写的 `kernel_app_report.rs:223` 咬不住）

实测：修前 A/B 那条"你写了 6 个位置实参"的诊断**来自老路** `elab.rs:4785`
（`implicit_msg::strict_rejection_message`），**不经过** `kernel_app_report::report`
⇒ 只改 `kernel_app_report::sanitize` 一个字节都碰不到它（反向验证时实测：出路里
`_soko_eq276` 原样露着）。⇒ 把 `sanitize` 的**实现**搬到 `implicit_msg.rs`（`pub(crate)`），
在 `strict_rejection_message` / `unsolved_message` 的**出口**各净化一次（两个生产者共用：
老路 `elab.rs:4785` 与内核路径 `kernel_app_report::report`）；`kernel_app_report::sanitized`
的 `debug_assert` 保留（守其余正文）。`_soko_eq<数字>` 的前一位要求**标识符边界**
（`x_soko_eq1` 不动）。反向验证读数：净化后出路变成
`@Eq.subst (Set α) (fun (一个没解出来的形参 : Set α) => x ∈ 一个没解出来的形参) A empty sorry hx`。

### 7.4 判据读数（二进制：`target/debug/sokonanoda`，含本刀）

| 项 | 读数 |
|---|---|
| ① A loose | 修前 `rc=0`（0 诊断）→ 修后 `rc=0`、0 诊断、`t` checked |
| ① A strict | 修前 `rc=1`：`elab-implicit-argument-unsolved` **7:3–7:22** + 级联 `elab-tactic-failed` **8:3–8:10**（`t` 不 checked）→ 修后 `rc=0`、0 诊断、`t` checked |
| ① B loose | 修前 `rc=0` → 修后 `rc=0`、0 诊断、`t` checked |
| ① B strict | 修前 `rc=1`：`elab-implicit-argument-unsolved` **10:3–10:32** → 修后 `rc=0`、0 诊断、`t` checked |
| ② `--lib -- eqsubst` / `-- tactic` / `--test strict_diagnostics` | 20 / 30 / 4 passed，0 failed ✓ |
| ② `--lib -- kernel_app` | 69 passed / **2 failed** —— 与本刀**无关**（隔离实测：只留别人的 G3 改动 `pp_shape.rs`、把本刀全部还原，同样红；两条都是 G3 落盘后**夹具过期**：`switch_on_reports_a_readable_error…` 的"今天还漏名"前提不再成立，`c1_unsolved_fallback_is_counted_as_old_err` 的 `old_ok == 0` 不再成立） |
| ③ loose 语料 | `scripts/strict/corpus-hashes.sh` 与 `/tmp/s6-after.txt`（714 行，16:09 基线）的 **loose 半 357 行 diff = 0 行** ✓；strict 半只动 **1** 个文件（`I.4/unit12-solution`），隔离实测证明那是 **G3** 的（只留 `pp_shape.rs` ⇒ 同一哈希 `2c11fd83e50ea044`） |
| ④ 反向 `:207` | 改回 `false` ⇒ 复现 D（`apply Eq.subst h`，5 位内联前缀）strict `rc=1`「你写了 **5** 个位置实参」；A/B 仍绿（隔离性 ✓）；还原后 D 两档 `rc=0` |
| ④ 反向 `:398` | 改回 `false` ⇒ 复现 A、B strict 都 `rc=1`（同上文案，6 位）；D 仍绿；还原后全绿 |
| ⑤ 新判据 | `tests/eqsubst_predicate.rs` +4 条 strict 臂（A / B / `Eq.symm` 包装 / 单实参 `:207`）+ `--lib -- eqsubst` 21 passed+ `kernel_app_report::tests::eqsubst_motive_name_never_leaks`（出路无引擎名 + 边界） |
| 边界 | `crates/kernel/**` **0 行**；`elab.rs` **8841 = 8841**（0 行）；`kernel_app_fallback.rs` 489（未动）；`kernel_app_report.rs` 412 → **395**；`implicit_msg.rs` +66；`eqsubst.rs` 444 → 458；`named_arg.rs` +8；`docs-lint` ✓ / `cargo fmt -p sokonanoda-front -- --check` ✓ / `clippy -p sokonanoda-front --all-targets` ✓ |
