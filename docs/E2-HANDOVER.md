# E2 交接书（给接手的 code agent）

> **要一句话把活交出去？** 直接复制 **`docs/E2-PROMPT.md`** 里那段 prompt ✓。
>
> **你只需要读这一页 + `docs/design/e2-plan.md`，就能接着干** ✓。
> 本文件写于 2026-09-24，交接时的仓库状态是：**版本 0.65.5（已发版上线 ✓）**、
> **E1 计划的 124 条全部勾完 ✓**、**E2 计划刚定稿、0/38 条已开工** ✓。

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
