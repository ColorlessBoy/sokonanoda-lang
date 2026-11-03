# CI `board` 红：`ux-queue.py --selftest` 的正对照（2026-11-03）

## 现象

CI run `41606288133`（提交 `be95f4f5`）· job `board` · 步骤「队列门禁（覆盖自检 + 调度器不变量）+ 自检」：

```
FAIL ④ 正对照：同批事件都有 `release-open` ⇒ 输出里**没有**「孤儿」（不误报 ✓）
ux-queue.py selftest: 1 条判据不成立 ✗（共跑 1631 个 `expect` 调用点）
```

重跑该 job（`--failed`）**同样的地方再红一次** ⇒ 不是 flake。

## 已排除（全部在本地实跑）

| 维度 | 取值 | 结果 |
|---|---|---|
| Python | 3.12.14 · 3.13.5 · 3.15.0 | 全绿 |
| 时区 | `TZ=UTC` · `TZ=America/New_York` | 全绿 |
| 语言 | `LANG=C LC_ALL=C` | 全绿 |
| git 身份 | `HOME=/tmp/emptyhome GIT_CONFIG_GLOBAL=/dev/null` | 全绿 |
| 连跑 | 10 次 | 0 失败 |
| 浅克隆 | 该 job 已是 `fetch-depth: 0` | 不适用 |

## 与本批次的关系

本批次改动 = `crates/front/**`（`plain` 放过 `let`、`lambda_source_type` 搬家）+ 孤儿 doc 删除 +
`scripts/strict-invariant-baseline.json` + `reports/**`。**没有一处**碰 `scripts/ux-queue.py`、队列台账、
`docs/` 索引。同一 job 在上一提交 `d0dc1326` 是绿的。

## 下一步（需一次诊断性 CI）

在那条 `expect` 失败时把 `_out` 原文打出来（批次制的例外第 3 条：本地复现不了才单跑一次 CI）。
拿到那一行才能判断：是环境真的产生了「孤儿」条目，还是输出里别的话术含这两个字。

**不许**为了让它绿而放宽判据 —— 按 `docs/CI-FAILURES.md` 的老规矩，咬不住的守卫等于没有。
