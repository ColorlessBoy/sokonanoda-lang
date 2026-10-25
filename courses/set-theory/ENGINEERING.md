# 卷 I《集合论》—— 工程状态

> 教材与学习入口是 `README.md`。本文件只放课程线的工程事实：判据、怎么跑、读数、
> 章级交付状态、CI 接线。课程内容不进这里，工程读数也不进 README。

## 判卷

| 目的 | 命令 |
|---|---|
| 画布还剩几个洞 | `scripts/soko query check --file "$PWD/courses/set-theory/units/<章>/unitNN-<slug>.sokonanoda"`，读 `counts.exercise_open` |
| 解答是否全绿 | 同上，文件换成 `units/solutions/<章>/unitNN-solution.sokonanoda`；`exercise_open` 应为 0、`decl_checked` 应大于 0 |
| 单文件诊断 | `scripts/soko grade "$PWD/courses/set-theory/units/<章>/<文件>.sokonanoda"`；退出码 0 = 没有解析/判定错误 |
| 整门课 | `python3 courses/set-theory/tools/check.py`（G1–G7；较慢） |
| 机器可读 | 在整门课命令后加 `--json` |
| 定位第一条判负 | `python3 courses/set-theory/tools/check.py --only "<标签>" --bisect` |
| 判据通道自检 | `python3 courses/set-theory/tools/check.py --selftest` |
| 课程侧复现件重放 | `python3 courses/set-theory/tools/check.py --gaps-only` |

画布上的 `sorry` 是合法状态：`grade` 对合法 open 返回 0，所以「有没有做完」要看
`query check` 的 `exercise_open`，不能只看退出码。

判卷一律用绝对路径（台账 G-12）。退出码：0 = 全绿 · 1 = 有目标被判负 · 2 = 前置缺失
或用法错误（无法判定 ≠ 绿）。

## 判据

判红只有下面七条；`checked` / `open` 计数与 `quota.exercises` 的差额只进报告与台账，
从不参与判红。

| # | 判据 |
|---|---|
| G1 | 每个目标 `grade` 退出码 0（只认退出码） |
| G2 | 目标存在：`course.json` 条目、`lib/`、每个画布的解答文件 |
| G3 | 解答：`exercise.open == 0` 且 `decl.checked > 0` |
| G4 | 解答覆盖画布每个具名练习（防「删题代替填洞」） |
| G5 | `lib` + Demo：`exercise.open == 0` |
| G6 | 清单自洽：volume/chapter id 唯一且非空、每个 unit 恰好属于一个 chapter、`prereqs` 指向存在的章 |
| G7 | 课程侧复现件重放：`gaps/*.sh` 逐条与登记状态一致 |

唯一真相是 `tools/check.py` 的 docstring + 实现。设计见
`docs/design/course-gate-in-ci.md` 与 `docs/design/course-manifest-v2.md`。

## 读数

本文件不保存逐轮流水。要现状就现算：

```bash
python3 courses/set-theory/tools/check.py --json
```

有记录的历史点：

- 全量收口（2026-10-14）：**258 目标 · 2190 checked · 915 open · 0 判负**
  （见 `OPEN-ITEMS.md`；此后课程仍有改动，现算为准）。
- `docs/courses/ledger.jsonl` 是人工追加的趋势点，`--ledger` 默认关闭；目前只有
  2026-09-19 一条（v0.60.0，36 目标 · 329 checked），只作历史。

## 章级交付

13 章共 112 个单元，另有一页记法对照；全部已交付，解答 0 open、课程门禁 0 判负。
逐单元清单以 `course.json` 为准。

| 章 | 标题 | 单元数 | 备注 |
|---|---|---:|---|
| I.1 | 集合、子集与集合运算 | 15 | 含经典逻辑线（104–105） |
| I.2 | 序对、关系与函数 | 4 | 含积对并交的分配（75） |
| I.3 | 像、原像与基数 | 16 | Cantor（94）、Cantor–Bernstein（112，经选择公理，单元头标注） |
| I.4 | 论域、悖论与综合 | 12 | 反例与不可证（55–57）、综合（12）、单满射（87–91） |
| I.5 | 序关系与良序 | 4 | 序嵌入（53）、⊆ 是偏序（67） |
| I.6 | 序数 | 13 | 三歧（107–108）、递归与加法（109）、ℕ 的算术（111） |
| I.7 | 选择公理与 ZF 公理体系 | 7 | AC（17/110）、ZF（18）、广义并（48–49/69/71–72） |
| I.8 | 基数与基数算术 | 4 | 基数算术（19–20/54）、连续统假设（113） |
| I.9 | 集族、广义积与 n 元关系 | 3 | 21–23 |
| I.10 | 关系闭包与等价关系 | 12 | 商（24–27/43）、核与像（76–86） |
| I.11 | 集合代数 | 7 | 28–32/46/50 |
| I.12 | 幂集与函数空间 | 7 | 33–35/44/51/68/70 |
| I.13 | 关系的代数 | 8 | 36–41/45/52 |

## 已知边界与未结项

未覆盖的主题与已知限制以 `OPEN-ITEMS.md` 和 `docs/gaps/ledger.jsonl` 为准。不要把
「哪些墙还挡着」抄进本文件：那份清单会随内核修复变化，抄一份就会过期（旧 README 的
「可做区现状」表就是这样过期的）。课程侧撞到的新缺口先放 `gaps/`，再收编进
`docs/gaps/ledger.jsonl`。

## 工具与 CI

| 工具 | 作用 |
|---|---|
| `tools/check.py` | 课程门禁，判据 G1–G7 |
| `tools/test_manifest_v2.py` | 清单 v2 展平、G6、成本台账字段的单测 |
| `tools/audit-pairs.py` | 画布/解答名字对账（诊断，不是判据；G4 仍是权威） |
| `tools/check-lib-closure.sh` | 把所有 `lib` 一次 import 完再判卷，必须 exit 0 |

`scripts/soko gate` 已包含课程门禁；`.github/workflows/ci.yml` 的 `test` job 跑同一条
命令。探不到 `python3` 时直接 exit 3，不静默跳过。

清单 v2（`soko.course/2`）的字段语义与消费者见 `docs/design/course-manifest-v2.md`、
`docs/protocol.md`。

## 历史

旧 README（776 行）把教材、逐轮门禁流水与逐单元 as-built 混在一起。2026-10-24 拆开：
教材与学习路径进 `README.md`，工程事实进本文件；逐轮流水与逐单元 as-built 不再维护，
原文用 `git log --all -- courses/set-theory/README.md` 取。
