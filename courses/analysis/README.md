# 卷 II/III《从集合论到实分析》

> 从卷 I《集合论》（`courses/set-theory/`）继续，目标 Tao《Analysis I》第 4–11 章：
> ℤ / ℚ / ℝ 的构造 → 序列与级数 → 连续与微分 → Riemann 积分。

## 状态

- **C0（M0）完成**：P1–P5 可行性探针 + `ROUTE.md` 设计文档；探针不是课程单元。
- **M1 未开始**：ℤ + ℚ，18 单元；`course.json` / `lib/` / 课程门禁随 M1 落。
- 提案与追踪：issue #13；C0 子 issue #14；C0 PR #23。

## 入口

- `ROUTE.md`：路线、探针结论、前置、里程碑、风险。
- `probes/`：M0 可行性探针（P1–P5），不进课程门禁。
- `probes/Prelude.sokonanoda`：探针本地的最小逻辑 prelude；M1 起换成 `lib/`。

## 边界

- 本目录是独立课程根（`sokonanoda.toml`），不复用 `courses/set-theory/` 的 `lib/`；
  M1 时把需要的库模块**移植**过来（不是跨根 import）。
- C0 探针用 axiom 隔离算术证明，只验证类型/写法骨架；完整 ℤ/ℚ/ℝ 证明在 M1–M2。
