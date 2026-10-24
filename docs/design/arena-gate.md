# 发版前的 Lean Kernel Arena 门禁（2026-10-24）

> 入口 `python3 scripts/arena-gate.py`；强制点 `scripts/bump.py`（写版本号前）与
> `scripts/release-preflight.sh` ④。

## 1. 契约

- **语料 = 最新 master**：每次查 `leanprover/lean-kernel-arena` 的
  `Build and Deploy Site` 最近一次 master push 成功 run，下载 `test-tarball`；
  缓存只按 run id + sha 复用，取不到 ⇒ exit 3（不静默跳过）；`--offline` 打印
  `NOT LATEST`，只给调试。需要 `gh` 登录 + 网络；只解压 `good/`/`bad/` 普通文件。
- **两层**：tarball tier（≤10MB 计分用例，必经）· full tier（`LEAN_KERNEL_ARENA`
  / `--full`，>10MB；仍受 64MB 上限，跳过数打印）。`either` 在 full tier 跳过。
- **每 case 有超时**（默认 120s）；超时按失败，`arena_known_slow.txt` 里的
  `perf/magma-list-pair-n21` 除外（pin `7b51784` >400s，当前上游 6.5s；只允许
  timeout，完成的 case 仍判 accept/reject）。
- **已知不支持** `tutorial/140_falseFromUnsafe`、`141_falseFromPartial`：必须
  declined/rejected，**accepted ⇒ 判负**；名单缺条目 ⇒ 判负。
  其他任何 decline/timeout/线程失联 panic 判负；内核 panic 按 arena 的
  **exit 1 = rejected** 契约算拒绝。
- **自检**：每次先用 `Empty` + `RuleDomainMismatch` 搭 2 条临时语料（必须过），
  再把 good 放进 `bad/`（必须判负）；不咬 ⇒ exit 3。
- **结果公开**：官方最新语料通过后写 `site/data/arena.json`；`bump.py` 同一步
  跑 `gen-arena-summary.py --write` 写 README 块、`gen-site-data.py` 带进
  `site.json`；`check-site.py data` 核对三处。`--corpus`/`--offline` 不写。

## 2. 内核契约（本门禁依赖）

1. 嵌套 iota 字段数核对：`check_restored_recursor1` 与非嵌套路径共用
   `assert_rec_rule_field_count`；谎报 ⇒ 稳定 panic
   `iota rule field count does not match the constructor telescope`。
2. unsafe/partial 显式 `decline`（不是 panic）。
3. `MetaContext` 未赋值计数 O(1) 闸：没有未赋值元变量时 `has_unassigned_meta`
   直接 false（`perf/app-lam` 30s+ → 0.46s；S1 深扫在共享 DAG 上指数）。
   `tc.rs` 串行/并行 checker 用 `resume_unwind` 透传原始 panic payload。

回归件：`crates/kernel/test_resources/{NestedRecRuleNFields,UnsafeDef}` +
`crates/kernel/src/tests/arena_regressions.rs`。

## 3. 接线 / 判据

```bash
python3 scripts/arena-gate.py --selftest
python3 scripts/arena-gate.py                  # 最新 master（gh auth + 网络）
python3 scripts/arena-gate.py --corpus <dir>   # 本地 good/bad（调试）
python3 scripts/arena-gate.py --write-result   # 通过后写 arena.json
python3 scripts/gen-arena-summary.py --check   # README ⇄ arena.json
python3 scripts/check-site.py                  # data 项核对三处
```

- `bump.py <x.y.z>`：写任何文件之前跑 gate；失败一个字都不写。`--check` 不跑。
- `release-preflight.sh` ④：bump 之后再跑并核对 README/site 不漂；`--fast` 不走。
- 退出码 0/1/2/3 = 通过/判负/用法/取不到语料·构建失败·自检没咬住。
- 反向验证：撤字段数检查 ⇒ `nested_rec_rule_field_count_lie_is_rejected` 红；
  unsafe 改回 `assert!` ⇒ decline 测试红；撤 O(1) 闸 ⇒ `perf/app-lam` 30s+。

## 4. 已记录的限制

- tarball 不含 >10MB 的 `init`/`std`/`mathlib`/`cslib`（需本地全量）；
  不调用官方 Lean 工具链。
- 没有 CI job（用户选择 preflight + bump；外部 artifact 下载在 CI 需 PAT）；
  手工 `git tag` 应急仍可绕过，但 RELEASE.md 明确它不是可接受发版路径。
- 官网/README 如实写 **126/127 + 1 条性能压力超时**，不写 127/127。
