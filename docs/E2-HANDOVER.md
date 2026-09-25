# E2 交接书（给接手的 code agent）

> **要一句话把活交出去？** 直接复制 **`docs/E2-PROMPT.md`** 里那段 prompt ✓。
>
> **你只需要读这一页 + `docs/design/e2-plan.md`，就能接着干** ✓。
> 本文件写于 2026-09-24，交接时的仓库状态是：**版本 0.65.5（已发版上线 ✓）**、
> **E1 计划的 124 条全部勾完 ✓**、**E2 计划刚定稿、0/38 条已开工** ✓。
>
> ---
>
> **⚡⚡ 2026-09-25 状态更新（第 2 次 · round 239 · **以这段为准** ✓）**
>
> **进度 `35/50` ✓ · 版本 `0.68.0`（Latest ✓）· 目标 phase=active ✓**
> （上面那段"19/38 / 0.67.0"是**当天早些时候**的事实 ✓，保留作历史 ✗。）
>
> **① CI 已经不是瓶颈了** ✓（当天早些时候那句"`test` job 每轮 30–80 分钟"**已过时** ✗）。
> 按用户要求做了六条 ✓（详见 `docs/design/ci-parallelism.md` ✓）：
> `gates` 拆成 **debug/release 两条并行** ✓（16 分钟 ⇒ **2m09s** ✓）· `lint` 拆
> **`lint-fmt`（7–18 秒出红灯 ✓）** + `lint-clippy` ✓ · `test` 矩阵扩成
> **`pkg × kind`（lib/tests/doc）= 12 条腿** ✓（**doctest 显式成腿，不会被 nextest 吞掉** ✗）·
> **快速失败链**（慢 job `needs` 快层 ✓ ⇒ 快层红则它们 **skipped** ✓）· **`fast-fail`**
> （任一 job 红 ⇒ `gh run cancel` 掐整轮 ✓）· **e2e 进快层** ✓。
> **识别"这轮有问题"：约 30 分钟 ⇒ 本地 ≤1 分钟 / CI 快层 ~2 分钟** ✓。
>
> **② 三个新工具（都在 `scripts/` ✓，先找入口再动手 ✓）**：
> * **`ci-watch.sh`** ✓ —— **按 job** 看 ✓（整轮的 `in_progress` 会**掩盖已发生的失败** ✗）+
>   耗时 + **失败 job 的注解** ✓；`--follow` **一红即退** ✓（不必等整轮 ✓）。
> * **`ci-push.sh`** ✓ —— **推前先关掉旧的没跑完的 run** ✓ ⇒ push ⇒ 报新一轮 id ✓。
> * **`expect-red.sh`** ✓ —— 断言某条命令**必须判红** ✓（"必须红"变成**命令的形状** ✓）。
> * **pre-push hook** ✓（`scripts/install-hooks.sh` 装一次 ✓）：推前自动跑
>   `ci-local.sh --fast`（约 1 分钟 ✓），红了**拒推**并**指名**是哪一条 ✓
>   —— 已经**实战拦截过一次 `cargo fmt`** ✓。逃生门 `--no-verify` / `SOKO_SKIP_HOOK=1` ✓。
>
> **③ 用户的两条要求** ✓：**①**（记法绕过逐条处置 ✓）**11 组 / 87 条全部收口** ✓
> （① 迁 9 · ② 已有 3 · ③ 不做 75 ✓，每条有理由 ✓；**凡用户可见文本全部走 ①** ✓）；
> **②**（面级判据 ✓）五面各有裁决 ✓ —— `ty` ✓ 与 hover ✓ 有**真判据**（可复现 ✓）、
> 状态栏/项目树 ③ 实测 ✓、**诊断面查明需内核结构化改造** ✓（那条消息的文本是内核拼的
> `String` ✗ ⇒ 不动内核无法折 ✓）。
>
> **④ ⚠ 现在**卡在一个用户决策上** ✗（接手先问 / 先看 `STATUS.md` ✓）**：
> **T-D6** —— 前缀复用 **(a) 保持默认开** ✓ / **(b) 改回默认关** ✗。
> 事实 ✓：机制**有效且等价** ✓（`by` 密集课程文件命中 **91.7%** ✓、两态 `--json`
> **逐字节相同** ✓），但**正规 perf 套件量不出收益** ✗（`scripts/perf-ledger.sh` 两态对拍，
> 63 个键里最大 Δ −75ms 且符号不一致 ✓）。计划原文是"**收益成立才默认打开** ✓，
> 否则保持关闭并记录 ✗"，而**代码已默认开** ✗（`judge.rs:441` ✓，先于测量 ✓）。
> **建议 (a)** ✓ + 记一条"本套件无可测收益" ✓；**但影响真实路径 ⇒ 由用户定** ✗。
> 定了之后 ⇒ **T-D7 = 阶段 D-1 发版（bump `0.69.0`）** ✓ → T-D10 → T-D13 → 阶段 E ✓。
>
> **⑤ 本段最贵的四条经验** ✗（每条都值几轮 ✓）：
> * **交接与计划都会过期** ✗ —— 本段**四次**靠**读代码**纠正它们 ✓
>   （`quiet_catch` 已可嵌套 ✓、影子走查已实现 ✓、判据已满足 ✓、**结论就写在目标上方
>   5 行/15 行的注释里** ✓）⇒ **动手前先读目标周围那几十行** ✓。
> * **要量性能/跑门禁，先找 `scripts/` 里的正规入口** ✓ —— 手搓 CLI 绕了**七轮** ✗，
>   `perf-ledger.sh` 一轮就给出答案 ✓（而且它**不迁就我** ✓：答案是"量不出收益" ✗）。
> * **按退出码判、也按"有没有拿到值"判** ✗ —— 本 session **四次**同类 ✗
>   （`set -e` 遇管道 ✗、clippy ✗、自检 `FAIL` ✗、`ci-push` 空 run id ✗）。
> * **失败可以，半成品不可以** ✗ —— 无效改动要**回退** ✓（`EnvLimit` 那次：改了 ✓、量了 ✓、
>   发现 181→181 毫无变化 ✓、回退 ✓）；判据**咬不住就不留** ✓。
>
> ---
>
> **⚡ 2026-09-25 状态更新（接手前必读）**：进度 **19/38**，阶段 A 已发 **0.66.0** ✓；
> 阶段 B（R-4 命令名 + R-3 产物目录）**代码全部完成**（T-B1..T-B6 ✓）、版本已 bump
> **0.67.0**，**但 release 还没走完** ✗ —— 卡在 CI 的 `test` job 极慢（每轮 30–80 分钟）
> 与 `auto-tag` 的机制：**`auto-tag` 只在 `event_name == 'push'` 时运行**
> （`.github/workflows/ci.yml`），用 `gh run rerun` 触发的重跑**永远不会出 tag** ✗
> —— 要出 tag 必须往 `main` **push** 一次 ✓。当前有一轮 push 事件 CI 在跑
> （head 见 `gh run list`），绿了就会 auto-tag **v0.67.0** 并 dispatch release ✓；
> 之后核对 `gh release list` 并把 **T-B7** 勾上。
>
> **阶段 C 已按实测重新定位**：T-C1 设计 ✓、T-C2 `plan_module` API ✓、T-C3 等价性判据 ✓
> （**负结果**：平坦批编对课程形状**不等价** —— 单元间重名 ⇒ 假重复声明 —— 且同口径慢 3×）
> ⇒ T-C4/T-C5 按 E2 §3 **刹车**（不接 `build`、不默认打开）；T-C6 重新界定为
> "把 LSP 的**写**路径也搬进模块根产物目录" ✓ 并**已随 0.67.0 一起发布**；
> ⇒ **T-C7 不发空的 0.68.0**（没有第二个用户可见增量）。详见
> `docs/design/module-batch.md` §8/§10 与 `docs/design/project-artifacts.md` §8b。
>
> **阶段 D 已开工准备**：T-D1（纯重构：抽出 `check_then_add_one`）的执行方案已写死在
> `e2-plan.md` 的 T-D1 下（抽取位置/签名/不许抽走的东西/风险），判据用现成工具
> `scripts/kernel-diff.sh --fast <改前> <改后>`（自检已过：`2/2，人为差异被抓到` ✓），
> 改前二进制已存档在 `target/debug/sokonanoda.before` ✓。
>
> **已知的两条环境事实**（免得重复踩）：① 受限沙箱会**拒绝仓库内的 rename 与删除**
> ⇒ 在仓库内跑项目构建时产物条目落不了盘（本地 gate 可加
> `SOKONANODA_NO_PROJECT_ARTIFACTS=1` 绕开，CI 不需要）；② 本机 `scripts/soko` 的缓存
> 二进制要**与仓库版本一致**（现在是 0.67.0），否则 gate 直接 exit 3 —— 重建后把二进制
> 落到 `target/debug/` 即可 ✓。

---

## 0. 三十秒版

* **这是什么**：`sokonanoda-lang` —— 一个教学用的 Lean 4 子集编译器 + 课程 + VS Code 扩展。
* **现在在做什么**：**`docs/design/e2-plan.md`** ✓ —— 5 个阶段 38 个环节，
  硬约束是「**每个阶段都能推一个完整可用的版本，性能只升不降**」✓。
* **你要做的**：按 `python3 scripts/plan.py next` 取环节，**一个环节一个 commit** ✓，
  阶段收尾才 push ✓。

```bash
cd <repo>
python3 scripts/plan.py next     # 下一条该做什么（含完整规格）
python3 scripts/plan.py list     # 0/38 的进度
python3 scripts/plan.py bumps    # 7 个发版点
python3 scripts/plan.py done T-A2   # 勾掉一条
```

---

## 1. 先读这些（按序，别跳）

1. `AGENTS.md` —— 项目入口，含**硬规则速记**与命令表 ✓；
2. `REQUIREMENTS.md` §2/§3/§9 —— **用户要求的权威总账**（§9 是追加区，最新的 4 条需求在那里 ✓）；
3. `docs/design/e2-plan.md` —— **你要执行的计划** ✓；
4. `docs/architecture.md` §6/§8 —— 内核改动台账 + **gotchas**（改内核前必读 ✓）；
5. `STATUS.md` —— 最新进度快照 ✓。

---

## 2. 当前状态（交接时的事实）

| 项 | 状态 |
|---|---|
| 线上版本 | **0.65.5** ✓（`gh release list` 可核对；26 assets） |
| E1 计划（`vscode-editor-feedback-plan.md` §13） | **124/124 全部勾完** ✓（其中 2 项实测否决 ✗、2 项存档 ⏸、1 项重新定级 ✗，**都有实测依据** ✓） |
| E2 计划（`docs/design/e2-plan.md`） | 刚定稿 ✓，**0/38** ✓；阶段 A 的 `T-A1`–`T-A3`（R-1 修复）**已完成** ✓，只是还没勾 |
| 用户最新 4 条需求 | 见 `REQUIREMENTS.md` §9 的 **R-1…R-4** ✓（R-1 已修 ✓；R-2/R-3/R-4 待做 ✓） |

**已经做完、别重做的** ✓：
* **R-1**（Infoview 的 `def` 卡片缺 `:=` 值行）：根因是 `crates/lsp/src/query_map.rs`
  的 `decl_info()` **漏映射** `value`/`value_runs` ⇒ 已修 ✓ + 判据 ✓
  （`crates/lsp/src/tests/goals.rs`）✓；
* 三个**内核原语**（`EnvBuilder::with_env` / `snapshot()` / 六个 interner 与 `Dag` 的 `Clone`）✓
  —— 各有判据 ✓，已进 `architecture.md` §6 台账 ✓；
* 一套**环境等价性对照器**（`SOKO_SHADOW_CHECK=1`）✓ —— 但它目前报 `一致=false` ✗（见 §5 陷阱）。

---

## 3. 你的任务：按 E2 计划推进

**五个阶段**（每行 = 一个可发布版本 ✓）：

| 阶段 | 做什么 | 版本 |
|---|---|---|
| **A** | 显示链路修复（R-1 ✓ 已完 / **R-2 待做**） | 0.66.0 |
| **B** | 命令名标准化（R-4）+ `.sokonanoda/` 产物目录（R-3 前半） | 0.67.0 |
| **C** | 模块级批量编译（`build <dir>`：146s → 模块数量级） | 0.68.0 |
| **D** | **前缀复用（重型重构）**：3 小步、3 次发版（0.69/0.70/0.71） | — |
| **E** | 收尾：性能回归进 CI + 文档/技能同步 | 0.72.0 |

**每个环节的完成定义（§0.3）** ✓：复现判红 → 最小改动 → 判据 → e2e 转绿 →
**性能无退化** → 文档 → commit ✓。

---

## 4. 硬规则（违反会被 CI 或用户打回）

> **并行纪律**（用户 2026-09-24 拍板，吸取两次教训 ✓）：subagent 只做
> **可并行/只读/边界清晰**的活（调研、审计、验证、量数字 ✓）；**判定与发布
> 走主线** ✓；**并发 2–4** ✓；每个 subagent 的 prompt **必须带**规则摘要 +
> **可执行判据** + 边界 ✓；产出**验证后才并入** ✓；结论**回写文档** ✓。
> 详见 `AGENTS.md` 的「并行与 subagent 纪律」一节 ✓。

1. **判定红线**（内核可以改，但**判定正确性不变** ✗）：同一批输入
   **接受/拒绝不变、事件计数不变、golden 与 `--json` 逐字节不变** ✓。
   改内核必须跑 `scripts/kernel-check.sh`（**五步** ✓：三层回归 / 逐字节对拍 /
   课程门禁计数 / 性能台账 / 语料对拍 ✓）。
2. **提交粒度**（用户 2026-09-24 明确 ✓）：**一个环节一个 commit** ✓；
   **一个阶段可以有很多 commit** ✓；**阶段收尾才 push 一次** ✓（跑一轮 CI ✓）；
   bump → auto-tag → release → **`gh release list` 核对** ✓（BUMP 必须闭环 ✓）。
3. **性能只升不降** ✓：有收益的才**默认开** ✓；前置/中性改动**默认关**（开关 ✓）
   或**零调用**（惰性 API ✓）；每阶段复量三个基准 ✓（见 E2 计划 §0）。
4. **课程一律写记法** ✓（`a ∈ A` 而不是 `Set.mem α a A` ✓）：
   `python3 scripts/notation-lint.py` 会判红 ✓。
5. **改 VS Code 扩展要同步四份** ✓：`editor/vscode/`（README/CHANGELOG/package.json）
   + `skills/` 三个技能 + `AGENTS.md` + `docs/vscode-dev-guide.md` ✓
   （`crates/cli/tests/skill.rs`/`dsh.rs` 会挡漂移 ✓）。
6. **用户新要求先入账** `REQUIREMENTS.md` §9 ✓（注明日期 ✓），再动手 ✓。

---

## 5. 陷阱清单（都是**这个仓库里踩过**的，别重踩）

| 陷阱 | 正确做法 |
|---|---|
| **`git checkout -- <file>` 在本机不可用** ✗（沙箱拒绝 unlink） | 回退用 python 从 git 读原文件再写回 ✓ |
| **`git merge-tree --write-tree` 冲突时退出码非 0** ✗，输出的树**带冲突标记** ✗ | **必须查退出码** ✓；非 0 就手工解，**绝不 `\| head -1`** ✓（我因此把带标记的 ledger 推上去过 ✗） |
| **`bash grep` 在本机会静默失联** ✗（返回空但文件里有） | 用**工具级** grep ✓；或 python 读文件判断 ✓ |
| **`find ~` / 全仓递归 grep 极慢** ✗ | 用 glob/grep 工具 ✓，或直接读已知文件 ✓ |
| 仓库 `target/` 有删不掉的陈旧文件 ✗ | 本地构建用 `CARGO_TARGET_DIR=/tmp/soko-target` ✓ |
| 本机 `cargo` 需要 `DEVELOPER_DIR=<命令行工具目录>` ✓ | 每条命令都带上 ✓。**值随机器而变**：交接时写的是 `/Library/CommandLineTools`，但**这台机器上该目录不存在** ⇒ 链接期会报 `xcrun: error: missing DEVELOPER_DIR path` ✗（编译能过、**只有链接失败**，很容易误判成代码问题）。先用 `ls -d /Library/CommandLineTools /Library/Developer/CommandLineTools "$(xcode-select -p)"` 挑一个**存在**的；本机实测可用的是 **`/Library/Developer/CommandLineTools`** ✓（`xcode-select -p` 给的是 Xcode 那份，也在） |
| **`SOKO_JUDGE_ENV_REUSE` / 影子（`SOKO_SHADOW_CHECK`）都不是"已修"** ✗ | 见下 |
| **别重试 T-K31**（`TcCache` 的 4MB 复用池）✗ | 实测**无收益** ✓（mmap 惰性零页 vs 强制 memset）；结论在 `docs/perf/ledger.jsonl` ✓ |
| **别指望"影子彩排"** ✗ | E1 实测三个假设全否证 ✓；阶段 D 只能**直接做** ✓，安全网是四件套 ✓ |
| **仓库 `target/` 在本机写不进去** ✗（cargo 删旧文件被文件策略拒 ⇒ `Operation not permitted (os error 1)`，编译能过、**只有链接/收尾失败**） | 本地构建一律 `CARGO_TARGET_DIR=/tmp/soko-target` ✓。但 `scripts/vscode-e2e.sh`（`cargo build --release`）与**部分 gap repro**（`target/debug/sokonanoda`）**硬编码仓库 `target/`** ⇒ 两条出路：① e2e 用 `--no-build` + 先手工 stage（`editor/vscode/bin/<target>/`，台账会记 staged 的 sha256 ✓）；② 把**版本匹配**的构建**就地覆盖**到 `target/release|debug/sokonanoda{,-lsp}`（那是 gitignored 的 scratch 目录；启动器的 `repo-build` 链优先于缓存 ✓） |
| **缓存可能"内容比 marker 旧"** ✗（本机实测：marker 写 `0.65.5`，二进制其实是 **0.65.4** 的逻辑） | `scripts/soko gate` 会 exit 3 并说"anchor 结果不可信"✓（守卫是对的）。判据：`scripts/soko version --json` 的 marker **加上**二进制自己的 `--version`（两者不一致就是这种病）。修法：`scripts/soko update`，或按上一条把正确构建放进去 |
| **本机 `scripts/soko update` 下载不了** ✗（代理下 release 资产 `HTTP 404` / `fetch failed`） | 用 `SOKONANODA_RELEASE_BASE=<本地 http 镜像>` ✓（启动器与 CLI 的下载器都读它；把 `v<版本>/sokonanoda-{cli,lsp}-<triple>.tar.gz` 放好，tar 里**顶层**就是二进制 ✓）。**很多 gap repro 会隔离 `SOKONANODA_CACHE_DIR`** ⇒ 强制走下载链 ⇒ 没有镜像时它们全报"环境异常" ✗（**不是回归**） |
| **Node 启动器的代理警告会污染 repro 的捕获** ✗（`(node:…) [UNDICI-EHPA] Warning:` 走 **stderr**，而不少 repro `2>&1` 合并后直接 `json.loads`） | 跑 `scripts/soko gate` 时加 **`NODE_NO_WARNINGS=1`** ✓（否则缺口台账会**假红**：G-07/G-11/G-16/G-17/G-15 这一批） |
| **缺口台账在 CI 绿、本地红** ✗ | 先按上面四条对齐环境（`NODE_NO_WARNINGS=1` + `CARGO_TARGET_DIR` + 版本匹配的 `target/` 构建 + 本地 release 镜像）；**再用"换修前的二进制复跑同一条 repro"** 判断是不是自己的回归（判据：`SOKONANODA_BIN=<旧构建> bash docs/gaps/repro/Gxx-….sh`）✓ |

**关于 `SOKO_SHADOW_CHECK=1`** ✓：它打印"影子环境 vs 内核阶段的失败表对照"，
**现在恒为 `一致=false`** ✗。**不要**把它的输出当成"产品坏了" ✗ —— 它是
**实验工具** ✓，结论已归档（`docs/design/vscode-editor-feedback-plan.md` 的 T-K12c 段 ✓）。

---

## 6. 性能病灶（阶段 D 的靶心，已量清 ✓）

```
冷开 courses/set-theory/units/solutions/unit12-solution.sokonanoda ≈ 12.4s
  JUDGE_INFER calls=51156 total=11183ms  ← 占 90%
    misses=247 ≈ 9.5s = 全文件的 77%     ← 每次 miss 重编译整份前缀
```

**墙**（已查明 ✓）：`decl_idx` 是"**每个名字全局唯一**"的槽位 ⇒ **两个环境无法共存** ✗
⇒ 唯一出路是让 **walk 自己的环境**就是那个环境（**把 check-then-add 移进 walk** ✓）
⇒ 这正是**阶段 D** ✓（3 小步、每步先开关、收益量出来才默认打开 ✓）。

**三个基准**（每阶段复量，只许变好 ✓）：
```bash
SOKO_JUDGE_STATS=1 sokonanoda --json courses/set-theory/units/solutions/unit12-solution.sokonanoda
sokonanoda build courses/set-theory          # 基线 146.07s / 35 文件
SOKO_PERF_COURSE_SLOW=1 cargo test -p sokonanoda-lsp --lib perf_course -- --nocapture
```

---

## 7. 收尾清单（每次阶段结束时）

```bash
scripts/soko gate                                   # fmt + clippy + test + 锚点 + 课程门禁 + 缺口台账
python3 courses/set-theory/tools/check.py           # 计数必须逐项不变：36 目标 · 328 checked · 99 open · 0 判负
scripts/kernel-check.sh                             # 改过内核才需要（五步）
SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh   # 批次收尾跑一次，记 docs/e2e/ledger.jsonl
# 然后：一次 push → CI 绿 → bump（Cargo.toml + editor/vscode/package.json 两处）→
#       auto-tag → release → gh release list 核对
```

**推送前自检（三条，都是"本地看不见"的坏数据 ✓）**：
```bash
python3 scripts/e2e-merge.py --check     # e2e 台账完整性（记录引用的日志必须在）
git diff --stat origin/main..HEAD        # 有没有"凭空消失"的文件
python3 scripts/audit-wire-fields.py     # 扩展读了但 LSP 不发的字段
```
> **为什么是这三条** ✗：它们对应三次真事故 —— ① merge-tree 合成 merge **丢了 4 个
> 被 ledger 引用的日志** ⇒ CI 的 `e2e ledger` job 红；② 同一手法**把冲突标记提交了**；
> ③ LSP 漏发字段 ⇒ 用户看不见但 e2e 全绿。**前两条现在已进 `scripts/soko gate`** ✓
> （所以本地 gate 全绿 = 它们也过 ✓）。

**发版前必做的一行校验**（专抓"本地看不见"的坏数据 ✓）：
```bash
git show origin/main:docs/e2e/ledger.jsonl | python3 -c "import sys,json;[json.loads(l) for l in sys.stdin if l.strip()]"
```

---

## 8. 有问题时

* **CI 红了** ⇒ 在 `docs/CI-FAILURES.md` **追加一条**（原因/修复/预防 ✓，硬规则 ✓）；
* **发现新的性能点** ⇒ 先**量**（`docs/perf/ledger.jsonl` ✓）再改 ✓ ——
  E1 里三次"先改后量"都以**回退**收场 ✗，三次"先量"都省下了返工 ✓；
* **判断不了某条路能不能走** ⇒ 看 `docs/design/vscode-editor-feedback-plan.md`
  的 T-K12c/T-K13/T-K30/T-K31 四段 ✓（那四段把"哪些路走不通、为什么"记到了代码行 ✓）。
