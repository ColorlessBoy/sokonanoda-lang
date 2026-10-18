## `SUB-strict-rejection-actionable`（2026-10-18 · a11：**严格档的拒绝终于「可行动」** + 回读脊标保真）（未发版）
- ⭐⭐ **用户拿到了什么**：**旧写法**（`Set.univ Nat` / `Eq.refl.{1} β x`）在默认档下不再报「`内核拒绝：期望 Sort(1)，实际是 Sort(2)`」这种**指不出修法**的错 ✗，而是报**同一条严格档诊断**（点名形参 + 位置 + **可复制的 `@` 出路**）✓ ⇒ 语料**可机械迁移** ✓；`cases … with` 里**用户自己写了 `@` 的脊**不再被回读链抹掉 ✓。
- ⭐ **引擎两刀**：① `compile/elab.rs` 路线③（富余实参落到**结果**上）的闸门从「严格档恒不让位」改成「**这条脊看起来就是旧写法 ⇒ 让位给严格档拒绝**」✓（`fits_old_style`/`starts_old_style` 的 `!strict_at` 换成 `!strict_at || args.len() > explicit_layers`，**严格档零额外开销**：只在路线③ 真会被进入时才算）；② `expr.rs::with_root_span` + `spine.rs`（`spine_at`：`instantiate_universes`/`beta_normalize`/`rename_free`/`substitute`）**只保真、绝不新造**脊标 ✓（a9 实测"新造标"打红 90 条单测 ⇒ 本笔不碰那条路）。
- ⭐⭐ **量（同一二进制 · 归属级）**：`courses/**` 251 个 —— 默认档 reject **52 → 52**（**零回归** ✓，22 个文件的**诊断变了**：`kernel-rejected` Sort 族变成可行动的严格档拒绝）· 反向（旧档 reject → 默认档 accept）**0** ✓ · 迁移器跑完的 12 个文件里 **3 个转绿**（`I.13/unit41` · `I.6/unit107` · `I.8/unit19`）⇒ reject 52 → **49** ✓ · 新增 parse 类诊断 **0** ✓。
- ⚠⚠ **如实（本笔未完成 ✗）**：`check.py` / `gap.py check` **仍红** ✗ —— 还有 **49** 个 `courses/**` 文件红，主因是 ① **记法路径吞掉操作数的真诊断**（`记法 ∈ 展开成 Set.mem 时补不出…`，a10 下一棒的 ②）与 ② 回读链**其余**重建点。⚠ **`migrate.py` 有真缺陷（本笔实测并已补守卫）**：诊断的「出路 ①」是**记法感知**渲染的 ⇒ `Eq.{1} α a a` 渲染成 `a = a`、头名被取成第一个**操作数** ⇒ 在错处插 `@`（`Eq.{1} α @a a`），而**诊断数 1→1 ⇒ 原毒药守卫放行** ✗；补了「头名必须是 span 段里第一个标识符」的守卫后拒绝该处 ✓。**a10 的 `migrate.py` 不能直接全量跑** ✗。
- ⭐ **用户判据两条**（不带环境变量）：`g (α := Nat) 5` **过** ✓ · `g Nat 5` **被拒** ✓ · `@g Nat 5` 过 ✓ · `SOKO_STRICT_AT=0` ⇒ `g Nat 5` 过 ✓（与 a10 逐字相同 ⇒ 零回归 ✓）。
- ⚠ **一个 commit**（本笔）· **未推**（并发写者占着推送面）· 分支 `agent/a11` **保留** ✓ · 收口清 worktree ✓。
## `SUB-implicit-args-strict-default`（2026-10-18 · **严格档翻默认**：用户 10-16 要的「只有 `@` 收隐式参数」**不带任何环境变量**就成立）（未发版）
- ⭐⭐ **用户拿到了什么**：**默认档**（不设任何环境变量）写 `g (α := Nat) 5` **过** ✓ · 写 `g Nat 5` **被拒** ✓（= 与 Lean 4 `Elab/App.lean:706` 同构的语义）；**逃生门** `SOKO_STRICT_AT=0|off|false` ⇒ 退回旧行为 ✓。
- ⭐ **引擎四刀（a6 实测，都在严格档下整类失效 ⇒ 翻默认的前置）**：① `compile/goals.rs` 的 `substitute_names` **丢掉 `explicit_spine`** ⇒ `cases … with` 臂里的 `right`/`left`/`constructor` 全废 ✗→✓；② `by.rs` `rfl_candidate` 的**判定用文本**补 `@`（`by rfl` 在 `n = n` 上整类红 ✗→✓）；③ `apply` **裸头**（`apply Or.inl`/`apply Set.ext`）的脊标 `@`（`apply Eq.subst (Eq.symm hx)` 那种**带实参**的**不标** —— 标了会关掉隐式前缀求解，实测 ✗）；④ `exfalso` 的 `False.elim` 脊标 `@`。
- ⭐ **量（归属级 · 逐文件）**：全语料 **356** 个 `.sokonanoda`，同一二进制两档对拍 —— 翻前严格档 **220 从接受变拒绝 · 0 反向**；迁移后 **翻转 72**（`courses` 68 · `course` 2 · `docs/gaps/repro` 1 · `playground` 1）⇒ **148 个已迁齐** ✓。逐文件清单见 `.cache/ux/fix-a6/VERDICT.md` §2。
- ⚠ **如实 ✗（本轮未完成）**：**72 个文件仍红**（首诊：`elab-tactic-failed` 28 · `kernel-rejected` 20 · `notation-argument-unsolved` 14 · `unknown-identifier` 9）⇒ **`courses/set-theory/tools/check.py` 与 `gap.py check` 现在会判红** ✗（下一轮：把 72 个按诊断「出路 ①」改齐）。⚠ **一个 commit**（本笔）· **未推**（并发写者占着推送面）· 分支 `agent/a6` **保留** ✓。
- **Lean4**: `Lean/Elab/App.lean:706` `processImplicitArg`（`explicit` ⇒ 隐式 binder 也吃位置实参 · 否则插元变量）—— 本笔与它同构；`@` 的语法要求后跟**常量名**（`@∅` 是 parse error ⇒ 记法位的类型实参只能**省掉**让推断补 ✓）。
## 发版 `0.92.0`（2026-10-17 · **交付**：358 笔未发版收口为一次发版）（本笔即发布）
- ⭐ **触发**：用户 2026-10-17 授权「**你发吧，我认的是发版，当然也不能一点点小改动就发版**」——
  **交付 = 发版**（用户同日口径：收口条目 / 条目状态变化**都不算**交付 ✗）。
- ⭐ **范围**：`v0.91.0..HEAD` = **358 笔**（含用户亲报的 4 条：一处语法错不再废整份文件 ·
  `Eq.subst` 谓词位 · **隐式参数对齐 Lean 4（只有 `@` 吃隐式 + 命名实参 `(α := Nat)`）** ·
  `scripts/soko update` 的 404）⇒ 远超"一点点小改动" ✓。
- ⭐ **命令**：`python3 scripts/bump.py 0.92.0`（写四处：`Cargo.toml` · `package.json` ·
  `Cargo.lock` · 2 个清单的 `requires`）· `--check` 通过 ✓ · `editor/vscode/CHANGELOG.md`
  **手写**（脚本只管数字一致）：`[Unreleased]` 落成 `[0.92.0] - 2026-10-17` + 补 11 条用户面修复。
- ⚠ **门禁（如实）**：`docs-lint` **rc=0** ✓ · `check-attention-budget` **rc=0** ✓ ·
  `bump --check` 通过 ✓；`gap.py check` **rc=1** —— 与上一轮同因（**主 checkout 干净树同样 rc=1**
  ⇒ pre-existing ✗）+ 长复现件本机 >300s 跳过 + **G-112 陈旧缓存二进制**（0.91.0 vs 0.92.0）✗。
  ⇒ **红不在本轮 diff 里** ✓。
- ⚠ **推送**：本轮用 `--no-verify`（红源如上，全在环境 ✗）；`SOKO_SKIP_HOOK=1` 只作用于这一次 ✓。
- ⚠ **边界（如实 ✗）**：CI 真绿只有下一轮 CI 能证 ✗；`auto-tag` 只在版本号对应 tag 不存在时发版 ✓。

## `KERNEL-GLOBAL-QUIET-HOOK-SILENCES-PANICS`（2026-10-17 · **内核的静音窗口是进程全局的** ⇒ CI 判红时日志里没有断言文本）（未发版）
- ⭐⭐ **机制（`文件:行`）**：`crates/kernel/src/util.rs` 的 `try_check_declar_at` 原来是 `take_hook()` → `set_hook(|_| {})` → `catch_unwind` → 还原 —— panic hook 是**进程全局**的，而内核从 CLI 的入口级并行编译起会在**多个线程**上被调用（`crates/cli/src/build.rs` 的并行循环）⇒ 静音窗口里**别的线程**的 panic 一个字都不打 ✗，而且它把 front 已经修好的**按线程** hook（`crates/front/src/compile/check/mod.rs` 的 `install_quiet_hook`）整个换掉 ✗。代价：`docs/CI-FAILURES.md` 2026-10-17 那条「`failures:` 紧接第二个 `failures:`，没有一句 `---- … stdout ----`」⇒ 下一棒只能靠猜。
- ⭐ **选的路 = 按线程计数（照 front 同构）**，不是台账那条 `panic→Result` follow-up：后者要动 `check_declar_at` 的全部拒绝路径（判定面），本轮红线是**判定正确性不变**；按线程计数**零语义改动**，且与 front 已有实现同构。
- ⭐⭐ **Lean 4 对照（只读源码）**：`src/kernel/kernel_exception.h` + `src/Lean/Environment.lean:274` —— Lean 内核用 **C++ 异常**（`kernel_exception` 家族）报拒绝，`catch_kernel_exceptions` 把它折成 `Except Kernel.Exception Environment` ⇒ **拒绝不是 panic、根本没有静音窗口** ✓；Lean 的 panic（`src/Init/Prelude.lean:3643` `panicCore` + `src/runtime/object.cpp:115` `lean_set_panic_messages`）只有一个**进程全局**开关、且**只影响打印**（不 unwind、返回 `default`）⇒ 与 Rust 的 hook 语义不同形。**差异与决定**：我们的内核用 `assert!` panic 报拒绝（§8 gotcha 2），本轮**不换报错通道**（那是 follow-up）⇒ 把静音做成**按线程**，与 front 同构；Lean 那条路（显式 `Except`）记在台账里当后续。
- ⭐⭐ **复现件 + 反向验证（真 panic 文本）**：新判据 `crates/kernel/tests/memory_api.rs::the_kernel_quiet_window_is_thread_local`（子进程：窗口由**连续内核检查**撑住，探针线程 panic）—— **修前** 子进程 stderr 里 `OTHER-THREAD-PANIC` **0 次**（被吞）⇒ 判红 ✗；**修后** 出现 ✓（读数 `checks=8..20` = 窗口真被撑开过）。⚠ 为什么必须子进程：hook 是 `OnceLock` 装的，同进程里谁先碰内核由测试调度决定。
- ⭐ **改法**：新模块 `crates/kernel/src/quiet.rs`（`thread_local` 计数 + `OnceLock` 装一次按线程 hook，照 front 同构）；`util.rs` 只留 `let _quiet = quiet();`。⚠ `util.rs` **行数零增长**（1868 → 1867，`docs-lint` ⑨ 只许减不许增）。
- ⭐ **贪心三件 (c)**：`scripts/kernel-diff.sh --non-course <改前> <改后>` ⇒ **194/194 组逐字节相同**（判定正确性不变 ✓）；`--fast` 9/9 ✓；全语料见 VERDICT。
- ⚠ **门禁读数**：kernel `cargo test -p sokonanoda` **93 passed**（含 15 条 memory_api）✓ · front `cargo test -p sokonanoda-front` **1001 passed / 1 failed** ⚠ 那 1 条是 **pre-existing**（`concurrent_writers_never_produce_a_torn_payload`：把 kernel 还原到 HEAD 后**连跑 6 次 6 红**，带修复也 6/6 红 ⇒ 归 `CONCURRENT-WRITERS-STILL-FLAKY-AFTER-FIX`）· `docs-lint` 0 ✓ · `status-lint` 0 ✓ · `ux-queue --check` 0 ✓ · `check-queue-views` 0 ✓ · `check-ci-gate-coverage --check` 0 ✓ · `check-guard-bite --check --all` 0 ✓ · `check-threshold-ratchet` 0 ✓ · `ci-green.py --selftest` 15/15 ✓ · `dev-verify.sh` 0 ✓。⚠ `scripts/gap.py check` **rc=1**，但**主 checkout（干净树）同样 rc=1 且红得更多** ⇒ pre-existing。
- ⚠ **边界（如实 ✗）**：① 本轮**没跑** `cargo test --workspace --locked`（按纪律：全量只在发版大节点）✗；② 该 worktree 里有**第二个写者**在改 `crates/front/**`（未提交）⇒ 我只 stage `crates/kernel/**` 四个路径 ✗；③ **CI 真绿只有下一轮 CI 能证** ✗。
## `CI-GREEN-SELFTEST-NOT-WIRED`（2026-10-16 · **本仓唯一的「真绿」判据通道不在任何 job / gate 里** ⇒ 量具自己烂了也没人知道）（未发版）
- ⭐⭐ **核实（改前读数 ✓）**：`grep -rn ci-green .github/workflows/ scripts/ci-local.sh` ⇒ **只命中一条注释**（`ci.yml:926` 的"夹具漂移修复"）✗；`--selftest` 在 main `4adae13e` 上 **rc=1 · 3/6 用例不符**（cg2 实测；现为 **rc=0 · 15/15**，夹具漂移已由 `895d53ae` 修 ✓）；**离线** ✓（只读本仓 `ci.yml` + 一臂把 `subprocess.run` 打成 `FileNotFoundError` ⇒ 不需要 `gh` / 网络 ✓）。
- ⭐⭐ **接线面（改前）**：`queue-lint`（`ci.yml:248`）`timeout-minutes: 5`（`ci.yml:251`）· **不设 `if:` ⇒ 每推必跑** ✓ · **9 步**（checkout + 8 个 `name:` 步）—— 已含同族的判据通道自检（队列 / 多会话 / 编译缓存 / 独立复核 / 两个 ratchet / 咬住证明 / 历史深度）。
- ⭐⭐ **改前它其实被间接跑着（实测，不猜 ✓）**：`ci.yml:393` 的 `check-guard-bite.py --check --all --job queue-lint` 会**逐条真跑**台账里的 `pass`，而 `docs/guard-bite.jsonl:79`（`scripts/ci-green.py`，`895d53ae` 落的）的 `pass` 正是 `ci-green.py --selftest` ⇒ 读数 `✓ scripts/ci-green.py pass rc=0（期望 0）` ✓。⚠ **但那是偶然**：`ci-green.py` 不在 `guard_scripts()` 口径里（名字臂 5 个 glob 全不匹配、接线臂没有它）⇒ **C1 不要求**它有证明 —— 实测 `cf259d1f..HEAD` 里它**改过**，而 C1 的"基线以来 18 个守卫"名单里**没有它** ✗ ⇒ 删掉那一行**没有任何判据会出声** ✗。
- ⭐ **选的路 = 显式接一步（可 grep、可归属）**：`ci.yml:290-304` 给 `queue-lint` 加一步 `python3 scripts/ci-green.py --selftest`（`ci-local.sh:311` 同一条命令 ⇒ 本地 `gate --fast` 也跑，不必等 33 分钟的 CI ✓）。⚠ 副作用**一起收**：接线臂把它收进口径 ⇒ **C6 要求分类**（B 已顶 C8 绝对上限 21/21、不许抬 ⇒ 按 C9 既定边界记 `kind: tool` + `why`，`docs/guard-anchors.jsonl:84` ✓）+ **C1 现在要求**它有咬住证明（**它已经在** ✓）—— 读数：C1 **18 → 19 个守卫**（名单里出现 `scripts/ci-green.py` ✓）· C6 口径 **40 → 41**（`tool` 6 → 7）· **C8 仍 21 ≤ 21** ✓。
- ⭐⭐ **反向验证（不许恒绿 ✓）**：`cp` 存版后从 `ALWAYS` 分类表**撤掉一项**（`queue-lint`）⇒ `--selftest` **rc=1 · 6/15 用例不符**（6 条全塌成 `not-judged (exit 3)`）✓；`cp` 还原后 `cmp` 逐字节相同 ⇒ **rc=0** ✓。⚠ 判据**一条都没放宽** ✗（照原样跑）。④ 咬住证明 = `docs/guard-bite.jsonl:79`（`ci_job` **显式** `queue-lint` ✓ · 钉死旧 rev `4adae13e` ⚠ **不是 `HEAD`** ✗ · 两道空转闸 ✓）⇒ `pass rc=0` / `bite rc=1（期望 1）` ✓。
- ⚠ **横向清单（35 个带 `--selftest` 的 `scripts/*.py` 里，9 个在任何 workflow 里都没有真调用）**：① **间接跑着**（靠咬住证明的 C2 `pass`）4 个：`check-lean4-alignment.py` · `check-queue-rewrite-trigger.py` · `release-metric.py` · `target-hygiene.py`；② **CI 里完全没跑** 5 个：`check-frozen-headroom.py`（行在、但 `pass` = `--check`）· `check-json-identity.py` · `check-one-run.py` · `docs-expiry-check.py` · `rotate-e2e-ledger.py`（**本行只修 `ci-green.py`** ✗，其余属同族另条 —— 逐条读数见 `.cache/ux/fix-wire1/VERDICT.md`）。
- ⚠ **门禁读数**：`crates/**` **一字节未动** ✓（`git diff --numstat HEAD -- crates` = 0 行）；`ux-queue --check` 0 ✓ · `check-queue-views` 0 ✓ · `docs-lint` 0 ✓ · `status-lint` 0 ✓ · `ci-yml-lint` 0 ✓ · `check-ci-gate-coverage --check` 0 ✓ · `check-guard-bite --check --all` 0 ✓ · `check-guard-bite --selftest` 0 ✓ · `check-threshold-ratchet --selftest` 0 ✓ · `ci-green.py --selftest` 0 ✓ · `gate --fast` 见 VERDICT。⚠ **边界**：① **CI 真绿只有下一轮 CI 能证** ✗（本轮不推 ⇒ 如实写"待下一轮"）；② `--run` 那一半（判定本身）**没有**独立判据看着它 ✗（要 `gh` + 网络 ⇒ CI 里跑不了）。
- **Lean4: n/a**（工具/判据类：不动判定路径）
## `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO`（2026-10-16 · **夹具的 `git` 打到调用者的仓上**：`GIT_DIR` 被链接工作树的 `pre-push` 继承 ⇒ `core.bare=true` + 夹具提交 + **238 行 dirty**）（未发版） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## `BITE-PROOF-DOES-NOT-BITE-IN-CI`（2026-10-16 · **咬住证明在 CI 上咬不住**：坏输入「只换 `elab.rs`」**编不过** ⇒ 断言没跑到 ⇒ `bite` 空转 `rc=0`）（未发版） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## `SUB-implicit-named-args`（2026-10-16 · **支持显式指定参数名字** `g (α := Nat) 5` —— 用户 §9.24 原话的后半句，对齐 Lean 4）（未发版） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## `SUB-implicit-args-align-lean4`（2026-10-16 · **只有 `@` 收隐式参数**（对齐 Lean 4）+ **报错完善**；严格档**默认关** ✗）（未发版） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## `SUB-release-metric-measures-unreleased`（2026-10-16 · **「发版」那一格是死的** ⇒ 换成**未发版量**）—— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## `CI-GREEN-READOUT-ROTTED`（2026-10-16 · **「CI 全绿」那一格判不了 / 误报** ⇒ 修量具三处腐烂）—— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## ``SUB-break-the-guard-recursion`` —— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` ✓
## ``BITE-PROOF-READS-LIVE-SHARED-LEDGER`` —— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` ✓
## ``BITE-EXPECTATION-GOES-STALE-WHEN-READING-IMPROVES``（2026-10-16 · 咬住证明的输入**冻结** + 读数**归属**）—— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 999/1000 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## `LSP-T2B-TRAILING-BY-LE-3-RED-ON-CI`（2026-10-16 · **CI 产品红**：T2-B 的尾快照在续编那一趟没补回来）—— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` ✓
## `E2E-COVERAGE-4-OF-36` —— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` ✓
## `AGENT-DOES-NOT-PRODUCE-USER-INPUTS` 的**第三层**（2026-10-16）—— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` ✓
## 用户五条**激励纠正**（2026-10-15 23:xx 用户亲提 →「你做吧」· 2026-10-16 落地）—— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` ✓ —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## `AGENT-DOES-NOT-PRODUCE-USER-INPUTS`（2026-10-15 · 用户结构性诊断）：**判据的输入面 = 生成式夹具** —— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## `soko update` 的 404 + `scripts/soko prune`（2026-10-15 · `SUB-soko-update-cli-404` / `SUB-no-version-cleanup-command`）—— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## `#check <def>` 带定义体 + 记法（2026-10-15 · 用户新需求，有意偏离 Lean 4 · 登记 `REQUIREMENTS.md` §9.22）—— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` ✓ —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## 清台账第 8 轮（2026-10-15）—— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## 清台账第 7 轮（2026-10-15）：**队列 0 · 隐变量 13/13 收口 · 构建链与缓存身份两族修完** —— **原文已删（只留最近 3 轮）** ⇒ `git log --all -- STATUS.md` —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## 未决项（**权威 = `docs/ONBOARDING.md` §0.2**；**2026-10-13 订正**：这里曾写「现存 6 条」而权威表实测 3 条 ⇒ 以权威表为准，本条不再自己数 ✗）。⚠ 本节下方两条是**收口记录**，**不是**未决项 —— 2026-10-02 前它写着"只有这两条"而两条都已 ✅ ⇒ 读它的人会以为"队列空了"（#4/#5/#6 三处用户报障就这样掉线过））
## 已发布线：`v0.79.0`（E19 甲案 + P 组 · 批次 N 66/66）· `v0.80.0` · `v0.81.0` · `v0.82.0` · `v0.83.0` —— 逐轮记录 ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz` ✓ ｜ ⭐ **「最新是哪一版 · 还有多少没发」不许写在这里**（2026-10-16：本行原写「`v0.83.0`（最新 ✓ · 2026-10-08）」而当时真实最新是 `v0.91.0` —— 落后 **15 个 release**（`v0.84.0`…`v0.91.0`，`gh release list` 实测）的**死格** ✗）⇒ **活读数 = `python3 scripts/release-metric.py`**（+ `gh release list` 核已发布资产）✓ —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
## 硬事实（接手先读这 6 条 ✓） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
