# 交接单：**内核 · `Acc` 索引族**（头号优先级）+ X2 收尾（3/5）

> ⚠ **2026-09-28 用户拍板：「卡内核的，改内核就好了嘛」⇒ 不要再停在"登记完成"**；**X2 让路**（已收尾 3/5，见本文 §4）✓。内核是现在的头号优先级 ✓。
> 分支 **`kernel/acc-indexed-families`**（从 `main` 开出，本地）。

> 上一棒：2026-09-28 会话（已发生 1 次 compaction）。**本地已全部 commit 并推送** ✓。
> 起点 `592a2e1e` → 终点 **`070a363d`**（`main`，工作区干净）。

## 1. 用户任务（原话要点）

「**主解法改成 worktree，加锁只作兜底**」「**先只开一个 worktree 跑通全流程**：建 → 改 → 提交 →
推 → 确认 CI **不被取消**、**不触发 auto-tag** → 合回。**跑通再谈并行**，别一上来开 N 个」
「**共享账本冲突**……约定「**只有 main 能改台账**」，并且**写成守卫**，别只写进文档」
「`e2e ledger` job 会往 main 回提交 ⇒ 分支持续落后 ⇒ 要定期 rebase，**把这条也变成可判的（落后 N 个提交就红）**」
「`pre-push` 钩子每条分支都跑 ⇒ 那**两个已知假红先修**」
「**不许删任何已有会话 / 分支（要删先问我）**」「不许为跑通改判据」「仍不许碰 `crates/kernel/`」

## 2. ✅ 已完成（3/5）

| 步 | 状态 | 落点 |
|---|---|---|
| **§5 skill 清单** | ✅ | `dsh/agent-skills.json` 给 `obra/superpowers` 加 `using-git-worktrees` + `finishing-a-development-branch` ⇒ `fetch-agent-skills.py` 实测 **28 skills**，两条已进 `.dsh/skills/` |
| **§1 worktree 全流程** | ✅ | 实测：worktree 建/改/提交/推**全通**；**推分支不取消 main 的 CI**（`x2/wt-probe` run `36403617878` in_progress 时 main 的 `36403200616` **仍 in_progress** ✓）；**不触发 auto-tag**（无新 tag ✓）。探针已清（本地+远端），远端只剩 `main` ✓ |
| **§2 台账「只有 main 能改」守卫** | ✅ | 新脚本 `scripts/check-multi-session.py`（判据详见 §3）；`crates/kernel/` 零改动 ✓ |

## 3. §2 守卫的判据（**这是本档最值钱的部分，改它前先读完**）

`scripts/check-multi-session.py`，两条判据 + 一条**实测校准出来的文案规则**：

- **① 链接工作树里不许改共享台账**。worktree 判据 = `git rev-parse --git-dir` 含 `/worktrees/`
  （实测：主树 `.git`、探针树 `…/.git/worktrees/soko-wt-probe`）。台账 =
  `docs/gaps/ledger.jsonl` + `docs/e2e/ledger.jsonl`。**为什么"一次都不许"**：`e2e ledger` job
  **会往 main 回提交** ⇒ main 一定在动 ⇒ 分支**必然**落后 ⇒ 冲突是**确定的**，不是概率。
- **② 台账与 `main` 不一致 ⇒ 判红**（合回必冲突）。
  ⚠ **判据是"台账落后"，不是"总提交数落后"** —— 后者每轮往 main 回提交一次、**只会一直涨**
  ⇒ 拿它判红会**每轮都红**（假红 ⇒ 守卫失效）。
- ⚠ **文案必须分三情形**（**两轮实测才修对**，见 `070a363d` 的表格）：工作区改 / 提交里改 / **只是落后**。
  判据对三者**都一样判红**，但"怎么修"完全不同 ⇒ **先判工作区（相对 HEAD），再判合并基**
  （顺序反过来会漏：从旧提交开出的分支上，工作区改了台账但合并基上没动 ⇒ 说错话）。
- **反向验证** ✓：`--selftest` **3 个反例**；真 worktree 三种情形**逐一实测判对** ✓。

## 4. ⬜ 未完成（接手的活，按价值排序）

### 4.1 §3 e2e ledger 落后守卫（**判据已想清，未实现**）
用户要"落后 N 个提交就红"。⚠ 直接数**总提交数**会假红（见 §3）⇒ 正解是**只判台账**：
本分支的 `docs/e2e/ledger.jsonl` 相对 `main` 的版本落后 ⇒ 判红。
**难点（下一棒要先解决）**：`main` 上已有的历史分支（如 `kernel/g56-indexed-inductives`）
**本来就落后** ⇒ 一上线就把所有旧分支判红 ✗。两条出路：
(a) **兼容历史**：只拦"**新增**落后"（记录基线，超过基线才红）；
(b) **只拦推送**（`pre-push` 里对"要推的分支"判），不拦"看历史"。
**选哪条要写进 commit message 的「决定／理由」** ✓。

### 4.2 §4 两个已知假红（**其中一个的旧归因已被推翻**）
PLAN:734 写「本机 pre-push 有**已知假红**（**pyyaml 缺失**、版本钉二进制拿不到）」。
**实测推翻前半条** ✓：
```
$ which -a python3 | head -2   →  /opt/homebrew/bin/python3（PATH 第一个）· /usr/bin/python3
$ /opt/homebrew/bin/python3 -c 'import yaml; print(yaml.__version__)'  →  yaml OK 6.0.3
```
⇒ 「pyyaml 缺失」**在当前环境复现不出来** ✓。另外：**清理探针时 pre-push 完整跑过一次、全绿**
（`ci-yml-lint` ✓ · `gap.py check --strict` ✓ · `docs-lint --selftest` **11/11** ✓ · `editor` 43/43 ✓）
⇒ 两个假红**当前都没出现** ✓。**我没有找到真正的机制** ⇒ **不改判据、不写错归因进
`CI-FAILURES.md`** ✓。下一步：`bash -x scripts/githooks/pre-push 2>&1 | tee /tmp/prepush.log`，
重点看 `ci-local.sh --fast` 的**第一个**红项；**候选（未验证）**：`pre-push` 现在把
**任何非零都当红**，而 `ci-local.sh` 对"环境不可用"用的是 **`exit 3`**（语义不同 ✗）
⇒ 应把 `exit 3` 明确说成"环境未就绪 + 怎么办"。

### 4.3 PLAN 的 X2 段过时数字（**未改 PLAN**）
X2 段写「`target/` 已 **205 GB**、磁盘只剩 **159 GB** ⇒ 开 worktree 就爆盘」。
**用户实测 + 我复核**：`target` = **32 GB**、`df -h /` 可用 **322 GiB** ⇒ **该硬门槛不成立** ✓。
⇒ 改 PLAN 时**同时写清**：独立 target 可行，但**别共享**（共享会被 cargo 加锁串行、抵消并行收益）。

## 5. 硬规矩（照旧）

- 提交前 **`git diff crates/kernel/` 必须为空** ✓（本档全程为空）；
- **不许删任何已有会话/分支**（`kernel/g56-indexed-inductives` 与 `main` **一个没动** ✓，要删先问）；
- **不许为"能并发"改判据或删文件**；**不许靠抬上限达标**；
- `STATUS.md` 里 **ST6 / ST7 / ST9 / ST11 保持 blocked-by-kernel**（G-56 + G-58/G-59；下标写返回位卡 **G-64**）；
- 一个环节一个 commit、批次收尾才 push、commit message 记「**决定／理由**」；
- 长命令落盘 + `timeout` + 轮询 ≤90s；**不许干等**（E18 守卫会判红缺实测数字的提交）。

## 6. 下一棒的 v0.79.0 候选（用户要过，半屏内）

主线 **E19 甲案**（给记法求解器加元变量 ⇒ 课程库隐式化）｜降级 **E20 乙案**｜横切 X1（✅ 已发）/ **X2（本档）**。

| 环节 | 内容 | 风险 | 依赖 |
|---|---|---|---|
| **E19-0** | **冻结基线**（零改动）：六项基线 `--json` + 门禁计数落基线文件；反向验证：改坏一处必须判红 | 低 | 无 —— **必做第一步**，否则"判定不变"无从证明 |
| **E19-1** | 引入元变量、**默认关**；判据 = 关闭时六项基线与 E19-0 **逐字节一致** | 中 | E19-0 |
| **E19-2** | **只在隐式参数求解**上打开 | **⚠ 最高**（会动判定输出） | E19-1；**任一 `--json` 字节差异 ⇒ 停下解释，不许"顺带修好"** |
| **E19-3** | G-43 复现件 `exit 0 → exit 1`（`univ`/`empty`/两者三种情形隔离） | 低 | E19-2 |
| **E19-4** | B2 本体：`lib/Set`(22 条) + `lib/Image`(6 条) 隐式化 → 逐文件迁移 | 中（**13 个单元/解答要全绿**，现状 4 干净 / 9 待修） | E19-3 |
| **E20** | 乙案：课程侧显式写（过渡/降级） | 低 | 可与 E19 并行或替代 |

**建议顺序**：`X2` 收口（本档 §4.1）→ `E19-0`（零风险）→ `E19-1/2`（真风险所在）→ `E19-3/4` → `E20` 视 E19-2 结果定。
**`E19-2` 是唯一可能动判定输出的环节** ⇒ 那一环走内核改动那套（先留判红证据 → 单独 commit → 三层回归 + 语料对拍 + `--json` 逐字节不变）。

---

# 附：内核 `Acc` 一棒的**精确接续点**（2026-09-28 第二轮实测）

## 目标（用户给的顺序，逐个交付）

1. **G-56** —— 让下面 4 行 `checked`（参数位逐字相同 + 指标位任意但不含递归出现）：
   ```
   inductive Acc (α : Type) (r : α → α → Prop) : α → Prop
   ctor intro (x : α) (h : ∀ (y : α), r y x → Acc α r y) : Acc α r x
   end
   ```
2. **G-64** —— 递归子能派生：宇宙代入 + `Acc.rec` 的 motive 与前端生成的 `Sort` 对齐。
3. **G-58 / G-59 大消去** —— 只对「单构造子 + 字段全在 Prop」的 **sub-singleton** 开；
   **`Or` 这种多构造子 Prop 归纳仍不许**消去到 Type（**反向判据**）。
4. **ST6（传递闭包）/ ST7（秩）真落地**；ST9 / ST11 能落就落，落不了**如实说是卡在哪一层**。

## ⭐ 本轮拿到的**新精确机制**（比本文档早先版本准，优先看这段）

**复现**（`end` 必须有）：上述 4 行 ⇒
`kernel-rejected: assertion left == right failed (left: 0 / right: 1)` ✓。

**调用栈**（把 `crates/kernel/src/util.rs` 的**静默 panic hook** ——
`try_check_declar_at` 里的 `std::panic::set_hook(Box::new(|_| {}))` —— 临时换成打
`std::backtrace::Backtrace::force_capture()` 就能拿到）：
```
subst_expr_levels (expr.rs:383)  ←  assert_nonnested_recursors_def_eq (inductive.rs:1692)
                                 ←  check_inductive_declar
```
⚠ **`expr.rs:383` 在 `if ks == vs || self.read_levels(ks).is_empty()` 这个短路块里** ⇒
只要 `ks` 的**指针**等于 `vs`（或 `ks` 空），它就**总是**求值 ⇒ panic 位置**不代表**根因 ✓。

**精确行 = `inductive.rs:1705`**：
`self.ctx.subst_expr_levels(old.info().ty, old.info().uparams, st.rec_uparams.unwrap())`
—— 拿**环境里那份同名递归子**的 `uparams` 去代入**新块要用的** `rec_uparams`。

⚠ **`left: 0 / right: 1` 的真实含义**：`left` = `old.info().uparams` **为空** ⇒
**`Acc` 递归子在环境里的那份 `uparams` 是空的**；`right` = `st.rec_uparams` 有 1 个
⇒ **两个来源的 universe 参数表不一致** ✗。

**⇒ 为什么"放宽 uniform 检查"必然没用**（上一轮已实测，这里给出原因）：
`Acc α r y` 的 `args_rev.len() = 3 > num_params = 2` ⇒ `inductive.rs:168` 的卫为**假**
⇒ 整支**跳过**（既不检查、也不判红）⇒ **判红根本不在 uniform 那一层** ✓。

**下一刀的两个方向**（**都要先量、不许猜** ✗）：
① `old.info().uparams` 为什么是空的 —— `Acc` 是**本次新声明**（不是 import），
所以这个"old"是**检查过程中先前一步写进环境的那份** ⇒ 查清写进去时 `uparams` 怎么算的；
② `st.rec_uparams` 怎么来的 ⇒ `mk_recursor_aux`（`inductive.rs:1727`）。

## 四道保险（用户重申）

内核 commit **单独** · 三件套（三层回归 / 全语料对拍 / `--json` 逐字节不变）**实测数字贴 message** ·
`grep '@@@'` 查插桩残留（**我这次插桩已完整还原**：`git diff crates/kernel/` 空 + `@@@` 0 处 ✓）·
**不许硬凑 / 不许为绿色改判据 / 不许压低难度**。

**反向判据（缺一条不算做成）**：参数位变化的出现**仍判红** · 指标位里含递归出现**仍判红** ·
**多构造子 Prop 归纳（如 `Or`）仍不许**消去到 Type · 现有全语料对拍**零差异**（除新增 `Acc` 相关）。

**分支与落地**：`kernel/acc-indexed-families`，**推分支跑 CI**（`concurrency.group: ci-${{ github.ref }}`
按 ref 隔离 ⇒ **不会取消 main 的 run** ✓，本档 §1 已实测过同类 ✓）；真绿再合 main；
落地后 bump **v0.79.0「良基递归」** ✓。

