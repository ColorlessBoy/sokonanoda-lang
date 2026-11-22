# M0 可行性探针

这些文件是**探针**，不是课程单元：不进 `course.json`、不进课程门禁、不面向学习者。
它们放在独立课程根 `courses/analysis/` 下；`Prelude.sokonanoda` 只是探针本地的最小逻辑 prelude。
它们只回答「这个形状能不能在本语言里写出来」，不承担课程内容的证明责任。

## p2-real-skeleton.sokonanoda

- 目的：验证 `Real := Quot realRel`（有理 Cauchy 序列的等价类）的**类型级**形状：
  `Quot` / `Quot.lift` / `Quot.sound` / `Quot.exact` / iota。
- 抽象 `Q`：`Q` 及其运算 / 序都是 axiom；本探针**不证明** ℚ 的构造与完备性，
  只确认商类型、良定义接口与反射方向能写。
- 运行：

  ```bash
  scripts/soko grade "$PWD/courses/analysis/probes/p2-real-skeleton.sokonanoda"
  ```

- 0.98.3 本机结果：25 条声明 checked、0 diagnostic、0 open；exit 0（0.98.4 分支由 CI 复验）。
- 结论：商类型骨架可用；下一步 P2 需要把抽象 `Q` 换成真正的 ℚ，并补 Cauchy 等价、
  加法/乘法良定义与最小上界。最大风险仍在完备性，不在商类型本身。

## p1-int-skeleton.sokonanoda

- 目的：验证 `Int := Quot intRel`（`Nat × Nat` 的差）的商类型骨架，
  以及 `Quot.sound` / `Quot.lift` / iota。
- 边界：`neg_wd` 是 axiom；整数加法/乘法的良定义与 ℚ 构造不在本片。
- 结果：11 条声明 checked、0 diagnostic、0 open；exit 0。

## p3-sum-rec.sokonanoda

- 目的：验证有限和 / 递归序列的 `Nat.rec` 写法。
- 发现：内置 `Nat.add` 在递归子步骤里不归约；课程自定义 `myAdd` 可以，
  `sum2_succ` 由 `rfl` 判绿。这是 P3 的 workaround，不是语言 bug 的定论。
- 结果：3 条声明 checked、0 diagnostic、0 open；exit 0。

## p4-continuity-ivt.sokonanoda

- 目的：验证连续（ε-δ）/ 连续函数 / 介值定理的**陈述形状**。
- 抽象 `R`：`R` 及其运算 / 序都是 axiom；`IVTStatement` 只是命题，**未证明**。
- 结果：12 条声明 checked、0 diagnostic、0 open；exit 0。

## p5-partition-integral.sokonanoda

- 目的：验证分割 / 上下和 / `Integrable` / `FTCStatement` 的**陈述形状**。
- 抽象 `R`：`R`、上下和、`deriv`、`integral` 都是 axiom；FTC **未证明**。
- 结果：18 条声明 checked、0 diagnostic、0 open；exit 0。

## 当前合集

P1 + P2 + P3 + P4 + P5：**11 + 25 + 3 + 12 + 18 = 69 条声明 checked**，0 diagnostic，0 open。

