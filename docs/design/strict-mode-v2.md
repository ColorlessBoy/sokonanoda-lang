# 严格档 v2：F5 最小修法（安全腿 + 迁移）

> 用户 2026-10-20 两条不变：①「重新规划严格档，**这次不能影响编译**」；
> ②「课程正文**从加 `@` 改成删多余的隐变量**」。
> **F5 已实现并全量验证（v0.96.0：严格档 16/16）**；任务清单与证据已随发版归档到 git 历史。

## §0 先读这六条

1. **13× 已定位，且就是一行**：`elab::operand_type_expr` 按档位选 explicit pp。
   strict `PREFIX_RUNS 16→5`、墙钟 46.5s→4.4s（debug）；但**只删它会让 `lib/Prod` 红**。
2. **P4 反转：不需要 AST provenance。** 那 2 条 line-1 诊断**不是泄漏**，是
   `lib/Prod` 的 2 条声明**真实失败**（`query check` 的 `failed[].name` 恒 None 曾误导）。
3. **F5**：保留 R3，把两处「**解出的隐式实参** elaborate」（`elab.rs:4617` / `:5008`）
   换成 `pp_shape::elab_engine_value` —— 严格档下用**默认档** `ElabCtx`，非严格档
   **原样直调**；用户写的实参仍走严格档。**2 个调用点 + 36 行 helper**，`elab.rs` 净增 0。
4. **F5 已全量验证**：356 文件两档 sweep **18 diffs 与 HEAD 逐行 0 mismatch**；
   耗时 **117.7s vs HEAD 537.9s**；`front --lib` **1007 passed**；canary 3 passed；
   `b2` strict 仍红；loose **24/24** 逐字节不变。
5. **语料**：HEAD 上 strict 红 **16 个 USER**（3 个 gaps/units + 13 个 solution）；
   F5 修好 **9/10 ENGINE**；`unit05-solution` 是**真实证明错误**（基线也红）。
6. **默认不翻**（独立决策：≥2 周 opt-in + 0 回归后再评估）；行数预算贴顶
   （`elab.rs` 余 4 / `judge.rs` 余 1 / `by.rs` 余 1）。

## §1 已核实的机制

| 事实 | 证据（同构建、同文件） |
|---|---|
| 慢的唯一来源是 explicit pp 分支 | strict 16 runs / 46.5s；loose 5 / 3.25s；R3 后 strict 5 / 4.4s |
| 被拒 AST 是**引擎解出的值** | `Prod.prod_mk A B a b` 在 `elab.rs:5008` 被当源码 elaborate；栈到 `build_theorem` |
| 那 2 条是真实声明失败 | `query goals`：`Prod.fst_snd_mk`/`Set.mem_prod_mk` = failed；`decl_checked=8/10` |
| F5 与基线等价 | 25/25 个 strict-diff 文件 exit+诊断数相同；356 sweep 0 row mismatch |
| loose 不受影响 | helper 非严格档原样直调；24/24 逐字节相同 |

## §2 三棒（F5 已完成；只剩迁移与文档）

| 棒 | 交付 | 判据 | 回退 |
|---|---|---|---|
| **A 安全腿** | P1 量具 · P3 canary · P4 调查 · F5（已提交）· M1 suggest · M3/M4 迁移 16 USER · P5 `failed[].name` | 16 USER strict `failed 0` + 两档逐字节；356 快照 strict 不新增诊断；`docs-lint` 绿 | 已交付保留 |
| **B 交付** | V2：opt-in 的 AGENTS/skills/编辑器说明 + 发布 | `scripts/soko gate` + 两轮 CI；**默认值不动** | 只回退文档 |
| **C 以后** | 默认翻 Strict | **不在本计划**；另立项 | — |

**纪律**：WIP=2；新发现进 backlog，只有「红测试阻塞」才能进计划；默认值任何任务都不改。

## §3 F5 改动面（已落地）

1. `elab.rs:4078`：`explicit = false`（R3，引擎类型查询用默认 pp）。
2. `elab.rs:4617`（路线③ 解出值）与 `:5008`（主路径解出值）：
   `elab_expr` → `pp_shape::elab_engine_value`。
3. `pp_shape.rs`：`elab_engine_value`（约 36 行）——严格档时构造
   `CompileOptions::with_dialect(prelude, Loose)` + 新 `ElabCtx`；否则直调。
4. **不动**：用户写的实参（`:5024` / `:4641`）、`Dialect::default`、AST 类型。
5. **不做**：`Expr::Engine` / 线程局部 watchdog / `@` 编码 / span 启发式（都已实测失败）。

## §4 课程迁移（16 个 USER：10 绿 + 6 个已知限制）

- **已绿 10 个**：机械 4（`gaps/C-112`、`unit04`、`unit78`、`unit05`）+ 手工 6
  （`unit08-images`、`notation-cheatsheet`、`unit46/51/39`、`notation-cheatsheet-solution`）。
  判据：strict `failed 0` + 两档 `stdout` 逐字节 + HEAD loose 反查逐字节。
- **6 个已知限制**（loose 0 / strict 1，**不进安全腿**）：
  `I.2/unit06` · `I.3/unit08` · `I.3/unit09` · `I.3/unit10` · `I.4/unit11` · `I.4/unit12`。
- **为什么不再硬试**（子代理逐点二分实测）：
  ① `@` 对 `Or.inl`/`And.intro` 的**第二隐式命题位**不是 term-identical（loose 靠期望类型
  统一补；写 `@` 后改判「两条线索矛盾」）；
  ② 删除只在「隐式位由兄弟实参唯一确定」时成立（`Or.elim`/`And.left/right`/`Eq.symm/trans`/
  `Set.inter/mem/univ` 可删；`Or.inl(b)`/`And.intro(b)`/`Eq.subst(p)`/`Exists.intro(A,p)` 删了 loose 也红）；
  ③ 本子集不支持项内 ascription，`unit06/08/11` 的隐式谓词无处写死；
  ④ `unit12` 撞「语句/证明体分道」的 `Sort(1)/Sort(2)`，要语句+证明体同时重写。
- 结论：这 6 个需要**改证明结构**，超出「安全腿机械迁移」；按「别把计划做成永远做不完」
  原则收口为已知限制（每条 loose 仍绿、无严格档回归）。
- `lib/` 的 10 ENGINE 已由 F5 修好（9 绿 + 1 个真实证明错误），**不在迁移集**。

## §5 验收（冻结 356 快照；禁绝对毫秒）

**F5（已过）**：356 两档 18 diffs 与 HEAD 0 row mismatch；`front --lib` 1007；
canary 3；`b2` strict 红；loose 24/24。

**安全腿（待过）**：
1. **10/16** USER：strict `failed 0` + 两档 `stdout` 逐字节相同；6 个已知限制
   （loose 0 / strict 1）逐条登记，无新增严格档回归。
2. 356 快照 sweep：diffs = **8**（6 个已知限制 + 2 个 invalid 文案）。
3. `docs-lint` 绿；`strict-compat.py --selftest` 咬住 `lib/Prod`。

**完整判定（迁移后）**：356 两档「退出码 + 诊断 code/span」一致；三条反向：
① 只删 R3 行必红；② 撤 F5 回到 10 ENGINE 红；③ `b2` 仍红。

## §6 风险登记

| 风险 | 证据 | 缓解 |
|---|---|---|
| F5 放宽了引擎值的严格档 | 25/25 + 356 与基线等价；用户实参未动 | 保持 356 快照 + `b2` 反向；迁移后重跑完整判定 |
| `query check` 的 `failed[].name` 恒 None | P4 被它误导，误判「声明 checked」 | P5 修成真实名字 + 契约测试（0.5 天） |
| 6 个已知限制需要改证明结构 | `@` 非 term-identical、删除 loose 也红、无项内 ascription | 逐条登记；不硬试；另立「证明体重写」项 |
| 默认翻转过早 | 单向下行风险 | 本计划不含默认；另立项 |
| 行数预算 | elab 余 4 / judge 余 1 / by 余 1 | 新代码放 `pp_shape.rs`（未冻结）；elab 净增 0 |

## §7 不做什么

- 不做 AST provenance / `Expr::Engine`（F5 已用 2 个调用点解决）。
- 不翻默认；不改用户可见 `@` 语义；不新增语法。
- 不靠「探测期宽松」「`@` 编码」「span 启发式」（三条都已实测失败）。
- 不新增绝对毫秒判据。

## §8 预算

F5 已完成（0.5 天）· 迁移 10/16 + 6 条已知限制 1–2 天 · P5 0.5 天 · V2 0.5 天 ⇒ **2–3 个工作日**。
默认翻转另立项：≥2 周 opt-in + 0 回归后再评估。
