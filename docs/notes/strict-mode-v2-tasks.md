# strict-mode-v2 任务清单（F5 版）

> 配套 `docs/design/strict-mode-v2.md`。**F5 已实现并全量验证**；本清单只剩
> 安全腿的迁移与文档。每个任务 ≤1 天、一条判据、一个回退；写操作串行、读操作可并行。
> 基线 `f6a27097`，证据 `docs/notes/strict-mode-v2-evidence.md`。

## §1 现状（2026-10-20 收口）

- **F5 已提交**：R3 + 两处「解出值 elaborate」→ `pp_shape::elab_engine_value`。
- **全量**：356 文件两档 sweep **18 diffs 与 HEAD 逐行 0 mismatch**（16 USER + 2 invalid）；
  耗时 117.7s vs HEAD 537.9s；`front --lib` 1007；canary 3；`b2` strict 红；loose 24/24。
- **F5 修好 9/10 ENGINE**；`unit05-solution` 是真实证明错误（基线也红）。
- **迁移进度 10/16**（手工集 6 + C-112/unit04/unit78/unit05）；**仍红 6 个已知限制**（loose 全绿）。
- **已取消**：P2 provenance 骨架（已删）、W1–W7 AST provenance（F5 取代）。

## §2 红集（16 USER，按可否机械修分）

**机械（建议先做）**：message 含「位置实参 + 出路 ②」或顶层码
`elab-implicit-argument-unsolved` —— 删多余隐式实参即可。
`gaps/C-112` · `units/solutions/I.1/unit04` · `I.10/unit78` · `I.2/unit05` ·
`I.2/unit06` · `I.3/unit08` · `I.3/unit09` · `I.3/unit10` · `I.4/unit11` ·
`I.4/unit12`。

**手工（逐个看目标；后 6 个已列已知限制）**：`units/I.3/unit08-images-preimages` ·
`units/notation-cheatsheet` · `solutions/I.11/unit46` · `I.12/unit51` ·
`I.13/unit39` · `notation-cheatsheet-solution` —— 多为 `kernel-rejected` 的
`Sort(1)/Sort(2)` 或复杂 `exact` 不匹配。

## §3 任务表（剩余 5 项）

| ID | 交付物 | 判据（一条命令 / 期望） | 依赖 | 写者 | 估时 |
|---|---|---|---|---|---|
| **P5** | `query check` 的 `failed[].name` 修成真实名字 | 新增契约测试：坏声明的 `failed[].name` == 声明名；`query check` 与 `--json` 计数不变 | — | 主 | 0.5d |
| **M3** | 迁 3 个 gaps/units 的 USER 文件 | 每文件 strict `failed 0` + 两档 stdout 逐字节；`check.py --json` | M1 | 副 | 0.5d |
| **M4** | 迁 13 个 solution（先机械、后手工） | 同上；手工集逐个记录改法 | M1 | 副 | 1–1.5d |
| **M5** | 10/16 绿 + 6 已知限制 + 课程门禁 + `notation-lint` | `strict-compat.py --sweep` diffs = 8（6 已知 + 2 invalid）；loose 全绿 | M3, M4 | 副 | 0.5d |
| **V2** | opt-in 文档：AGENTS/skills/编辑器说明（默认不动） | `cargo test -p sokonanoda-cli --test skill --test extension --test dsh` 绿 | M5 | 主 | 0.5d |

## §4 已完成的批 A（证据留档）

| ID | 交付 | 判据 | commit |
|---|---|---|---|
| P1 | `scripts/strict-compat.py`（sweep/json/selftest） | 356 files / 18 diffs；selftest 5 项 | `f0cc8729` |
| P2 | provenance 骨架 | 已被 F5 取代，**已删除** | `90f9dac1` → `323ec868` |
| P3 | CLI canary（lib/Prod + unit05 + b2） | 3 passed；R3 后前两条是警报 | `c7f7545c` |
| P4 | 泄漏调查 → 真因 + F5 | `lib/Prod` 10 checked / 0 diag | 报告（无代码） |
| F5 | R3 + 2 调用点 + helper | 356/18/0 mismatch；1007 passed | `3d500a72` |
| M1 | `--suggest` 迁移建议 | 3 文件子集 2 mechanical + 1 manual | `2009db56` |

## §5 依赖与并行（只剩短链）

```
P5（独立，可并行）
M1 ── M3 ── M5 ── V2
   └─ M4 ──┘
```

- 写者纪律：M3/M4 只写 `courses/`；P5/V2 写 `crates/`/文档 —— 文件不相交，可 2 写者并行。
- **顺序**：先机械集（M4 的 10 个），再手工集；每文件两档对拍，禁止批量正则。
- **V2 与 M5 同批**：迁移完成后一次性交付 opt-in 说明。

## §6 每项成功纪律

1. **先红后绿**：每文件迁前 strict 红、迁后 strict 绿 + 两档逐字节；红/绿输出进 commit。
2. **一条判据**：`SOKO_STRICT_AT=1 … grade --json <file>` 的 exit + 两档 `cmp`。
3. **一个回退**：`git revert` 后该文件回到红（V1 抽查）。
4. **手工集不硬套**：`kernel-rejected` 若需要改语义，先记录“改法 + 为什么”，再动。
5. **默认值不动**：V2 只写 opt-in 文档；不碰 `Dialect::default`。

## §7 批次

- **批 A（已收口）**：P5 + M3/M4（10/16 绿 + 6 已知限制）。
- **批 B（收尾）**：M5（10/16 口径）+ V2 + `scripts/soko gate` + 两轮 CI。
- **不做**：默认翻转（另立项）。

## §8 与主计划映射

| 主计划 | 任务 |
|---|---|
| A 安全腿 | P1 + P3 + P4 + F5 + M1 + M3/M4 + M5 + P5 |
| B 交付 | V2 |
| C 以后 | 默认翻 Strict（出范围） |
| 已取消 | P2 / W1–W7（F5 取代 AST provenance） |
