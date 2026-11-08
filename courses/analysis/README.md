# 卷 II/III《从集合论到实分析》

> 从卷 I《集合论》（`courses/set-theory/`）继续，目标 Tao《Analysis I》第 4–11 章：
> ℤ / ℚ / ℝ 的构造 → 序列与级数 → 连续与微分 → Riemann 积分。

## 状态

- **C0（M0）完成并 merge 进 `agent/real-analysis-integration`**：P1–P5 可行性探针 + `ROUTE.md`。
- **M1 进行中**：ℤ + ℚ，18 单元。已落 `course.json`、`lib/`（9 个模块 + Demo）、单元①–⑧（画布 + 解答）；其余 10 单元继续。
- **课程门禁**：`tools/check.py` 已从卷 I 移植，用 0.98.4 构建跑通；C3 收口前必须 0 判负。
- 提案与追踪：issue #13；C0 子 issue #14；C0 PR #23。

## 入口

- `ROUTE.md`：路线、探针结论、前置、里程碑、风险。
- `probes/`：M0 可行性探针（P1–P5），不进课程门禁。
- `lib/`：M1 从卷 I 移植的 L2 库（`Logic` / `Exists` / `Set` / `Prod` / `Rel` / `Fun` / `Image` / `Equiv` / `Order`）。
- `units/II.1/`：M1 的整数单元（序对商、代表元公式、加法良定义与律、零元与负元）；解答在 `units/solutions/II.1/`。
- `course.json`：M1 课程清单（当前只列已落地的单元，随写作增长）。

## 边界

- 本目录是独立课程根（`sokonanoda.toml`），不复用 `courses/set-theory/` 的 `lib/`；
  M1 时把需要的库模块**移植**过来（不是跨根 import）。
- C0 探针用 axiom 隔离算术证明，只验证类型/写法骨架；完整 ℤ/ℚ/ℝ 证明在 M1–M2。
