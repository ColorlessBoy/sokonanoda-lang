# 交接单：**内核 · `Acc` 索引族**（头号优先级）+ X2 收尾（3/5）

> ⚠ **2026-09-28 用户拍板：「卡内核的，改内核就好了嘛」⇒ 不要再停在"登记完成"**；**X2 让路**（已收尾 3/5，见本文 §4）✓。内核是现在的头号优先级 ✓。
> 分支 **`kernel/acc-indexed-families`**（从 `main` 开出，本地）。

> 上一棒：2026-09-28 会话（已发生 1 次 compaction）。**本地已全部 commit 并推送** ✓。
> 起点 `592a2e1e` → 终点 **`77617f1f`**（`main`，工作区干净 ✓；**CI `36417501766` 真绿**：`ci-green.py` exit 0 · 重活 10/10 · failure 0 ✓）。

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

---

# 🔴 P0（用户 18:11 实测，**最高优先级**）：Infoview 点声明名 ⇒ 弹空

## 0. 用户动作级实测（**改前**，2026-09-28 · 真 LSP，脚本 `/tmp/p0/lsp_probe.py` 的配方见下）

夹具 `courses/set-theory/units/unit01-sets-membership.sokonanoda`，用**真 `sokonanoda-lsp`** 走
`initialize` → `didOpen` → **轮询** `textDocument/definition`（轮询是必需的：编译完成前问会得到
null，第一版就是这么误判的 ✗）：

| 位置 | 结果 |
|---|---|
| **① 声明名** `demo_mem_def`（第 32 行第 11 列，1-based） | **`null`** ⇒ 面板弹「**这里没有可跳转的定义**」✗ |
| **② 使用处** `∈`（第 32 行第 48 列） | **跳到 `Set.sokonanoda:56`** ✓ |

⇒ **用户报的现象复现了** ✓，且**测试用②、用户点①** 的错位也被实测坐实 ✓。

## 1. 事故根因（判据绑错了对象）

`editor/vscode/src/test/extension.test.js:743` 的注释**自己写明**了：
「本 LSP 的 definition 解析的是**使用处**，声明名本身**不返回定义**（拿声明名当位置 ⇒ 空结果）
⇒ 判据取使用处；**webview 目前接的是声明名**」。
⇒ **判据验的是"链路通"（`decl-name` → `definition` → `executeDefinitionProvider`），
不是"用户点下去看到什么"** ⇒ 标 ✅、用户一点就空 ✓。

**另一条独立的机制**（G-53，用当前数据实测）：`query goals` 的 `ty_runs`/`goal_runs`/`goals_runs`
共 **503 条 run，字段集合 = `['kind','text']`** ⇒ **没有源位置** ⇒ webview 渲染的 `tok-*` span
**无从知道"点的是源码哪一处"** ⇒ 类型/目标里的**符号**（`∈`/`{a}`）**点不动** ✗。

## 2. 三条修法（用户倾向 ③ 为主、② 兜底）—— **规格已定，实现留给下一棒**

| | 做法 | 判据（**必须绑用户动作**） | 风险 |
|---|---|---|---|
| **②**（兜底，最小） | 声明名点击 ⇒ **本文件内 `reveal` 到声明自身**（诚实、不弹空）；**符号**点不动时**仍弹**「没有可跳转的定义」（不许把"没定义"说成"跳了"） | 点声明名 ⇒ 编辑器选中/滚动到该声明（**不是**弹提示） | 低 |
| **③**（主，用户真正要的那半） | **接上 G-53**：给 wire 的 runs 加**源位置**（`start/end` 偏移）⇒ webview 的 `tok-*` span 带位置 ⇒ 点击发 `definition` | 在 Infoview 里**点类型行里的 `∈`** ⇒ 跳到 `lib/Set` 的定义行（跨文件 ✓） | 中（动 wire 契约 ⇒ `docs/protocol.md` 同步 + LSP 单测断言**字段存在性**） |
| **①** | definition 在声明名处返回自身 | 同 ②，但要证明**不与 F12 语义冲突** | 中 |

**为何 ③ 是正解**：`∈` 在**类型行里**被渲染成源级记法（A3/A5 的折叠）⇒ 用户**看得见**它、
**想点**它 ⇒ 那才是"跳转"有用的地方 ✓；而②只让**声明名**这一处不再弹空 ✓。

**⛔ 若时间不够**：**至少落 ②** —— 它把"弹空"消掉（用户的可见症状），并**如实标注**
「符号跳转仍缺（G-53 open）」✗，**不许把 ② 说成"跳转做好了"** ✗。

## 3. ⚠ 流程修复（用户：「这条比 bug 本身重要，必须落地」）

**新增硬规矩**：凡**可点击 / 可跳转 / UI 交互**类交付，判据**必须绑用户动作**
（点下去 ⇒ 发生什么**可见结果**）；**不许只验"消息发出 / 函数被调用 / 链路通"**；
**更不许用"能跑通的位置"代替"用户实际点的位置"** ✗（E27 就是这么漏的）。
**外加**：STATUS / PLAN 里标 **✅** 的项，**必须有一条判据是"用户动作 → 可见结果"** ✓。

**⇒ 已落成三样东西**（只写文档几轮就失效 —— E18 已证明过一次 ✓）：
1. **`AGENTS.md`** 的「验证设计纪律」四条硬规则**前面**加一条第 0 条（本节原文）；
2. **`scripts/check-user-action-evidence.py`** —— 扫 STATUS/PLAN 里标 ✅ 的 UI 类环节，
   **必须**在测试里找到"用户动作 → 可见结果"的证据（见脚本头注释的判据）；
3. **E27 的标记改成如实状态**（已做一半：机制通了 + **用户点的那一处仍空**）✗。

## 4. 怎么复现本节 §0 的实测（配方，免得下一棒重写）

```python
# /tmp/p0/lsp_probe.py 的要点（真 LSP、真夹具）
# 1) initialize（rootUri = 课程根）→ initialized → didOpen（uri + 全文 text）
# 2) **轮询**：反复发 textDocument/definition，直到有非 null 结果或超时；
#    ⚠ 编译完成前一律 null ⇒ 不轮询会误判成"缺口" ✗（第一版就踩了）
# 3) 两个位置：声明名 (31, 10) 0-based；使用处 `∈` (31, 47) 0-based
# 实测：声明名 → null；`∈` → file …/lib/Set.sokonanoda, line 55 (0-based) = 第 56 行 ✓
```

---

# 2026-09-28 18:00 之后这一整段（**已全部落地并推 main**，HEAD `77617f1f`）

## 已交付（按时间）

| commit | 内容 | 关键判据 |
|---|---|---|
| `192404bd` | **P0 纪律落 main**：`AGENTS.md` 验证设计纪律**第 0 条**（(a) 判据绑用户动作 · (b) 同类问题横向排查）· PLAN 的 **E27 改如实状态** · 台账 **G-65/G-66/G-67** | `docs-lint` ✓（⑦ 咬住 6 次，**全部真删行**：AGENTS.md 436→429） |
| `6d0af61c` | **G-66 横向排查 7 处双重压暗**（用户只指了 `.section-title` 一处）· 标题字号 `0.8em → 1em` · 删 `uppercase`/`letter-spacing` | 新 `scripts/check-infoview-hierarchy.py`（**进 `soko gate`** + `--selftest` **9 反例/5 必须判红**） |
| `d4da5359` | **P0-b**：点声明名**不再弹空** ⇒ `revealRange` 到声明自身；符号点不动时**仍如实弹提示** | 新 stub 用例（**绑用户动作**：编辑器真动 + `__messages` 空 + 降级前仍问服务端）；**反向验证 43/44 → 44/44** |
| `7eb93699` | **G-67 项目区块重设计**：三问（编完了吗/哪个版本/有没有问题）放最前 | 新 `editor/vscode/test-project-first-screen.js`（跑**真的** `infoview.js` + DOM shim，取**未收起**文本）· **反向验证：旧版 5 条判红** |
| `612c6a39` + `77617f1f` | **E30 回归修复**（见下）+ **连推守卫** | `test-webview.js` **23/23** · `test-extension-host.js` **44/44** · `soko gate` exit 0 |

## ⚠ 本轮最重要的教训：**G-67 撞掉 E30**（"重设计"不等于"删掉既有交付"）

**CI `editor` job 判红**：`project: the manifest warning is visible text, never a tooltip (E30)`。
**我犯的三处错**（都是"重设计过头"）：
1. 把 **`.project-warning` 这个独立可见元素删了**、只留人话 ⇒ 撞「必须独立可见」+「文本原样含 `requires`」；
2. 把**模块列表包进 `<details>`** ⇒ E30 按**直接子节点**的 className 比顺序 ⇒ `indexOf("project-modules")` = **-1** ⇒ 判红；
3. 把 `.project-facts`/`.project-counts`/`.project-artifacts` **折进「高级」** ⇒ 撞「事实行含 toml/根/入口」+「产物行含字节数」。

**⇒ 教训**：**E30 钉的是「整个区块的交付契约」，不是"某一行好看"** ✗ ——
**"重设计"≠"删掉既有交付"** ✓。⚠ 用户 18:24 说的是「产物字节数 ⇒ **删掉或**放进高级折叠区」
—— 而**"删掉"会让 E30 判红** ⇒ 取**次要**那一支 ✓（用户给的是二选一，我选了会撞判据的那支 ✗）。

**修法（两者都给，不是二选一）**：**E30 的元素全部原样保留 + 三问摘要加在它们前面** ✓（顺序即优先级）；
`requires_warning` = **人话在前 + 原始细节在后**（`.project-warning-detail`）✓；
`编译 N` → **`已编译 N/M`**（给分母；**不是删数据** —— E30 只看「2 模块」✓）。
⚠ **没回滚 G-67、没放松 E30、没改测试** ✓。

**G-67 守卫也同步修正**：撤掉「内部词一律不许上第一屏」✗（**与 E30 正面冲突**）
⇒ 改为只管用户真正抱怨的两件：**三问答得上** + **版本出现在内部细节之前** + **不许「编译 N」** ✓。

## ⚠ 第二条教训：**连推三次 ⇒ HEAD 一条跑完的 CI 都没有**

用户 19:00：「`36402084828` / `36403200616` / `36410130673` / `36412806730` **全部 cancelled** ⇒
**当前 HEAD 一条跑完的 CI 都没有** ✗ —— **这就是"本地绿就算绿"的翻版**」✓。

**已落成守卫** ✓：**`scripts/check-one-run.py`** —— 推送前若已有未完成的 `ci` run ⇒
**exit 1 拒绝推送**并指名那条 run + 三条出路；**接进 `scripts/githooks/pre-push`** ✓；
`--selftest` **3 反例**（含 2 个必须判红）✓。
**设计取舍**（写进脚本头）：**不自动关旧 run** ✗ —— `scripts/ci-push.sh` 那样做，而**连推时
关掉的是上一批** ⇒ 那批**永远没被验证过** ✗；`gh` 不可用 ⇒ **exit 2「判不了」**（不是绿 ✗）
⇒ pre-push 放行但**说出来**（不许静默）✓。

⚠ **另一条实测**：**空提交（只改 git、不动 rust/courses/editor）会得到"假绿"**
—— `ci-green.py` 判 **exit 2**：「重活被 skipped ⇒ **这一轮没有验证任何东西**」✗
（`bf151f74` 就是）。⇒ **要拿真绿证据，必须推一个动了 rust/courses/editor 的 commit** ✓。

## 接手第一件事：**P1（rebuild 非常卡、非常慢）**

用户原话：「一个这么小的项目就这么卡，后面大项目完全吃不消」。
**按顺序做，别跳**：
1. **先确认用户说的 "rebuild" 是哪个命令**（VS Code `Sokonanoda: Rebuild`（`alt+shift+b`，clean→build）？
   还是 `soko build`？`cargo build`？）—— **别猜**；不确定就**列候选各测一次**，用数字定位 ✓。
2. **量化，不许凭印象优化**：一次 rebuild **总时长 + 各阶段**；**编译了几次**（同一文件有没有被
   重复编译 —— 用户点名怀疑这条）；**增量缓存是否每次失效**（失效 = 全量重编）；有没有**全量重扫**。
   量具：`closure_compile_scaling` / `keystroke_recompile_closure`（`docs/perf/ledger.jsonl`）·
   `SOKO_JUDGE_STATS`（⚠ 见下）· `cargo build --timings`；不够就补，**但先有数字** ✓。
3. **拿到数字再定改法**；若真是重复编译 ⇒ **架构问题**（缓存键/失效判定），**不是调参能解决的，如实说** ✓。

**⚠ `SOKO_JUDGE_STATS` 的一个坑（本会话修过 ✓）**：它原来只在 **`by` 路径**装打印机 ⇒
项风格下 `calls == 0` **早退** ⇒ `JUDGE_INFER` 一行都不打 ✗。**已修**（`judge_infer_with` 也调
`install_printer()`）⇒ 现在**打得出来** ✓（实测 `JUDGE_INFER calls=668 total_ms=500 avg_us=749`）。

## 其余未做（不占 P1）

- **P2** 进度粒度（声明级/目标级/阶段级 + **"最长无输出间隔 ≤ N 秒"**判据）；
- **G-65 的 ③**（接 **G-53**：给 wire 的 runs 加**源位置** ⇒ 类型/目标里的**符号**能跳）——
  实测 **503 条 run 字段只有 `kind`/`text`** ⇒ webview 无从知道"点的是源码哪一处" ✓；
- **X2 剩余 3/5**（§3 e2e ledger 落后守卫 · §4 两个假红，**"pyyaml 缺失"已被实测推翻**）；
- **内核**（`kernel/acc-indexed-families`，`3a4e1c5e` **挂起保留**）：下一刀 =
  `inductive.rs:1705` 的 `old.info().uparams`（空）vs `st.rec_uparams`（1 个）⇒ 查
  `mk_recursor_aux`（`inductive.rs:1727`）✓。

## 本轮新落地的守卫（都在 `soko gate` 或 `pre-push` 里，**别绕过**）

| 守卫 | 挡什么 | 自检 |
|---|---|---|
| `scripts/check-infoview-hierarchy.py` | 标题字号 < 正文 · `<1em` + `opacity` 双重压暗 | **9 反例 / 5 必须判红** ✓ |
| `editor/vscode/test-project-first-screen.js --check` | 项目第一屏答不上三问 · 版本排在内部细节之后 · 「编译 N」 | 旧版跑出 **5 条判红** ✓ |
| `scripts/check-one-run.py`（**pre-push**） | 已有未完成的 run 时**拒绝推送** | **3 反例 / 2 必须判红** ✓ |
| `scripts/check-timing-evidence.py`（pre-push + gate） | 动内核 / 自称 perf 的提交缺实测计时 | **17 反例 / 4 必须判红** ✓ |
| `docs-lint.py` 判据 ⑦ | 接手路径超标（**只许收紧，不许抬上限**） | **11/11** ✓ |

