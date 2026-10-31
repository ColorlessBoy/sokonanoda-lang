# strict 重构交接（2026-10-31）

接手只读本文 + `reports/2026-10-29-strict-replan.md`（计划与 P1 记录）。本文所有数字注明了出处；没注明的当未核实。

## 1. 现在在哪

- 分支 `strict/2026-10-29-kernel-app`（远端同名），从 main `f8d231d3` 分出，领先约 43 个提交。**只有这一条工作分支**，其余本地/远端分支与 worktree 已于 2026-10-31 清理（备份见 §6）。
- 新路径开关：`SOKO_STRICT_KERNEL_APP=1`，只在 `--dialect strict` 生效。开关关 ⇒ 不执行新代码，`--json` 与 main 逐字节相同（每刀合并前都对拍过）。
- 计划进度：P0 ✓ · P1 ✓ · P2-1…P2-9 ✓ · P2-C 普查 ✓ · **P2-10 / P2-11a 进行中** · P3/P4 未开始。
  **进度权威 = `reports/2026-10-29-strict-replan.md` §4「P2 接管」拆刀表**（本节只是摘要）。

| 刀 | 提交 | 内容 |
|---|---|---|
| P2-1 | `9118256b` `305946b5` | 推迟解不出的显式实参，按内核给的域重来（对齐 Lean `Elab/App.lean:433-443`）；`KERNEL_APP_MISS` 归因计数 |
| P2-2…P2-6 | `0abaf058`…`eaf42058` | 多轮推迟、交 Solved 前会话内重检、记法走内核、探测模式 + 重做 |
| — | `56869482` | 课程 unit12 L285-286 λ 体里的 `∅` 写成 `@Set.empty Nat`（用户选的：约束不足处 Lean 也报错，不让 strict 猜） |
| P2-8 | `f8b095cf` | 开关的生效档进所有缓存键（`kernel_app::effective/key_tag/key_bit`）；关 ⇒ 所有键逐字节不变 |

## 2. 读数（开关开，冷跑，二进制 `a5e9b3eb`，macOS/arm64）

| 文件 | `judge_infer` 未命中 strict/loose | 耗时 strict/loose |
|---|---|---|
| unit01 | 5 / 5 | 1.05s / 1.41s |
| Equiv | 4 / 4 | 2.16s / 2.44s |
| unit12 | 4 / 4 | 4.35s / 4.16s |

lib strict 17/17。计分板 `work_le_loose` 三文件全绿。⇒ **性能目标（strict ≤ loose）在开关开时已达成**；剩下的是把它变成默认并删掉旧路径。

P2-8 合并后我复跑：`cargo test -p sokonanoda-front --lib kernel_app` 25/25；front lib 1130/0。
`scripts/soko gate --fast` rc=1，唯一红项是 `infoview-hierarchy`（`.section-title` 字号/透明度）——`editor/` 与 main 无差异，属 main 上的既有问题，与本分支无关（未在 main 上单独复跑）。

## 3. 下一步（按顺序）

1. **消掉剩余回落**（P2-7 设计方案 A，`reports/2026-10-31-p2-7-design.md` §2/§6 第 3–5 步）：
   外层元变量出现在实参域里时，把域当 `expected` 传进内层会话（注册外部元变量 + 内层 `meta_base` 防撞号），`NeedArgs.blocked` 带上域。
   这类回落（`And.left/right`、`Set.image`、`Set.inter` 的 `fallback_unsolved`≈169 次）命中缓存、不费时，但**不消掉就删不了旧路径**。
2. **strict 默认走内核路径**：去掉开关判断（`kernel_app::env_on`），解不出 ⇒ 报错点名 binder，不再回落旧文本路径。
3. **删旧路径**：旧 typed app、`type_oracle` 文本回落、影子档；P3 列的开关全部删掉。
4. **整门课 strict**：`python3 scripts/strict-score.py --course`，判据见 replan §4 P2。
5. P4 发版：默认档由用户拍板；push/release 前先问用户。

## 4. 未决 / 已知问题

- **未核实**：P2-7 设计 §5 的 R1+R2 单独成文件时 strict 与 loose 判定不一致（strict 报 Sort(0)/Sort(1)）。可能是既有的两档分歧，动第 1 步前先钉住。
- 已发布的 strict 档（开关关）在 unit12 `flawed_equalities_refuted` 上拒绝，报 Sort(0)/Sort(1)（原来被 `\0soko_m…` 漏名盖住）。开关开时通过；第 2 步后自然消失。
- 内核元变量赋值要求封闭项，比 Lean 的按元变量局部上下文更严（K1c）。
- 只在 macOS/arm64 上量过。

## 5. 怎么验证（脚本已进仓库 `scripts/strict/`）

```bash
# 计分板：lib 两档 + 工作量门（strict 的未命中 ≤ loose）
python3 scripts/strict-score.py --bin <cli> --env SOKO_STRICT_KERNEL_APP=1
# 全语料逐字节对拍（tracked .sokonanoda × 两档）：先钉二进制，再 diff 两份输出
scripts/strict/corpus-hashes.sh <base-cli> /tmp/base.txt
scripts/strict/corpus-hashes.sh <new-cli>  /tmp/new.txt && diff /tmp/base.txt /tmp/new.txt
# 单文件回落计数（开关开）：一行 JSON
scripts/strict/fallback-one.sh <cli> <file.sokonanoda>
```

坑（都踩过）：
- 长跑前**把二进制拷到 sha 命名的私有路径**——agent 会中途重编 target，读数混用两个二进制就作废。
- macOS 没有 `timeout`，用 `perl -e 'alarm shift; exec @ARGV' <秒> <cmd>`。
- zsh 不会把未加引号的 `$var` 拆成多个参数；向 git 传 ref 列表用 `--stdin`。
- 一条声明只报第一处错，可能盖住另一处：判「既有问题」要拆到合取粒度复现。
- `lib/Logic.sokonanoda` 两档输出本来就为空（rc=0），对拍相同不说明什么。

## 6. 备份（只在本机）

`~/Documents/lean/sokonanoda-archive-2026-10-31/`：
- `branches.bundle`：清理前全部 51 个分支（含旧方案 `strict/2026-10-29-pipeline` 134 个提交、`agent/*` 等）；`refs.txt` 是 ref → sha 对照。
  恢复：`git fetch <bundle> 'refs/remotes/origin/strict/2026-10-29-pipeline:refs/heads/old-pipeline'`。
  replan §5 计划从旧分支摘的 `strict/report.rs`、`strict_app/gate.rs`、`scripts/audit-strict-paths.py` **还没摘**，P3 要用时从这里取。
- `p0-spike.diff` + `p0_spike.rs`（P0 探针）、`k1-agent-uncommitted.diff`（K1 agent 的未提交改动，已被 K1a 取代）、`p27-design.md`。
