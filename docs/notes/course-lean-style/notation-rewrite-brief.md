# 课程记法清零施工手册（notation rewrite brief）

> 2026-09-21。用户要求：「重新设置一个 courses 的规则，至少 notation 都要换掉，
> lib 和正文都换掉，不要有些还是老版本的。你先实现一个检查脚本，然后一个文件一个
> 文件过。tactic 还比较费时……可以先保持一部分的 term。」随后追加：
> 「基础类型的隐变量也可以尝试和 lean 对齐。……`Eq.{1}` 直接就是一个等于号」。
>
> 本手册 + `scripts/notation-lint.py` 就是**那条规则**。执行者（人 / subagent）
> 按本手册逐文件改写，**判据两条**：① `python3 scripts/notation-lint.py --root <你的文件>`
> 零残留；② `node scripts/soko grade "$PWD/<你的文件>"` exit 0 且零 `diagnostic`。

## 0. 检查脚本

```bash
python3 scripts/notation-lint.py                 # 全课程（卷 I + 入门课 + playground）
python3 scripts/notation-lint.py --root courses/set-theory/lib/Set.sokonanoda
python3 scripts/notation-lint.py --json          # 单 JSON 对象（agent 用）
```

豁免（脚本内置）：`units/notation-cheatsheet*.sokonanoda`（教学装置，**故意**并列
点名 ↔ 记法）；行内/上一行 `-- soko:notation-ok: <理由>` 的边界。

## 1. 旧写法 → 新写法（**必须换**）

### 1.1 逻辑连接符 / 量词（语言内建，零声明可用）

| 旧 | 新 |
|---|---|
| `And X Y` | `X ∧ Y` |
| `Or X Y` | `X ∨ Y` |
| `Iff X Y` | `X ↔ Y` |
| `Not X` | `¬ X` |
| `forall (x : T), p x` | `∀ (x : T), p x` |
| `Exists T (fun (x : T) => p x)` | `∃ (x : T), p x` |
| `A -> B` | `A → B` |

### 1.2 等式 / 不等式的**类型**（用户原话：「`Eq.{1}` 直接就是一个等于号」）

| 旧 | 新 |
|---|---|
| `Eq.{u} T a b` | `a = b` |
| `Ne.{u} T a b` | `a ≠ b` |

⚠️ **只换类型位**。`Eq.refl` / `Eq.symm` / `Eq.trans` / `Eq.subst` / `Eq.rec` /
`Eq.ndrec` / `Eq.mp` / `Eq.mpr` / `cast` / `congrArg` 这些**证明项**保持显式写法
（宇宙层级推断是独立的一刀；见 §3 边界）。

### 1.3 集合论符号（`lib/Set.sokonanoda` 声明，随 `import` 生效）

| 旧 | 新 |
|---|---|
| `Set.mem α a A` | `a ∈ A` |
| `Set.subset α A B` | `A ⊆ B` |
| `Set.union α A B` | `A ∪ B` |
| `Set.inter α A B` | `A ∩ B` |
| `Set.sdiff α A B` | `A \ B` |
| `Set.compl α A` | `Aᶜ` |
| `Set.powerset α A` | `𝒫 A` |
| `Set.empty α` | `∅` |
| `Set.image α β f A` | `f '' A` |
| `Set.preimage α β f B` | `f ⁻¹' B` |
| `Set.prod α A B` | `A ×ˢ B` |
| `Set.singleton α a` | `{a}` |
| `Set.pair α a b` | `{a, b}` |

⚠️ `Set.empty α` 换 `∅` 后**不能出现在 `=` 左边**（实测 `∅ = A` 判红）：
写在左边时保留 `Set.empty α`（加 `-- soko:notation-ok`）或改成 `A = ∅` 的等价形状。

### 1.4 基础类型的隐式实参（prelude 已对齐 Lean，**可以省前导参数**）

| 旧（写全） | 新（省前导隐式实参） |
|---|---|
| `And.intro A B h1 h2` | `And.intro h1 h2` |
| `And.left A B h` | `And.left h` |
| `And.right A B h` | `And.right h` |
| `And.elim A B C f h` | `And.elim f h` |
| `Or.inl A B h` | `Or.inl h` |
| `Or.inr A B h` | `Or.inr h` |
| `Or.elim A B C f g h` | `Or.elim f g h` |
| `Iff.intro A B h1 h2` | `Iff.intro h1 h2` |
| `Iff.mp A B h` | `Iff.mp h` |
| `Iff.mpr A B h` | `Iff.mpr h` |
| `Iff.refl A` | `Iff.refl` |
| `Iff.symm A B h` | `Iff.symm h` |
| `Iff.trans A B C h1 h2` | `Iff.trans h1 h2` |
| `Not.intro A f` | `Not.intro f` |
| `Not.elim A C h a` | `Not.elim h a` |
| `False.elim C h` | `False.elim h` |
| `absurd a b ha hna` | `absurd ha hna` |
| `Exists.intro A p w hw` | `Exists.intro w hw` |
| `Exists.elim A p Q h f` | `Exists.elim h f` |

⚠️ 省参靠「后续显式实参的类型 + 期望类型」反解。**有期望类型的位置**（`exact`/
`apply` 的目标、`fun` 的返回类型、声明值位）一定能用；**没有期望类型的裸项**
（少数 `def x := …`）若报 `elab-implicit-argument-unsolved`，把参数写全并留
`-- soko:notation-ok`。

`congrArg` 的**参数顺序变了**（新签名 `{α β} {a b} (f) (h)`）：
`congrArg.{1} α β f a b h` → **`congrArg.{1} f h`**（或写全新顺序
`congrArg.{1} α β a b f h`）。

## 2. 不要动

- **tactic 块**：已是 `by` 的全部保留；**不把 term 改成 tactic**（用户明说 tactic
  费时、先保持一部分 term）。
- `Eq.*` / `congrArg` 等**证明项**的显式宇宙/参数（§1.2 的边界）。
- `Set.univ α`：没有记法，且零元应用不在本轮隐式插入的覆盖内 ⇒ 保留。
- 声明位（`def mem`、`ctor …`、`inductive …`）：那是定义，不是使用。

## 3. 纪律

1. **只改分给你的文件**；不动 `crates/`、`docs/`。
2. 判卷一律**绝对路径**（台账 G-12）：`node scripts/soko grade "$PWD/<file>"`。
3. 改一个判一个：exit 0 且事件流里没有 `diagnostic`。
4. **计数中性**：纯记法改写不改变判卷计数；整卷门禁必须仍是
   `36 目标 · 328 checked · 99 open · 0 判负`（`python3 courses/set-theory/tools/check.py`）。
5. 撞到边界（改了判红、且不是自己写错）⇒ 写 `-- soko:notation-ok: <理由>`，
   并在交付说明里列出来；**不许**改 `crates/` 绕过。
