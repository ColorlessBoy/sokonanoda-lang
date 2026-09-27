# 交接单 —— 前置收尾 → 准备开 v0.74.0

> 写于 2026-09-27（会话上下文 ~52 万 token、已触发 compaction）。
> 依 `docs/PLAN-0.74-0.79.md` 顶部「⚡ 执行索引」的换会话信号 ⇒ **不硬撑，交単换会话** ✓。
> **唯一真相仍是 `docs/PLAN-0.74-0.79.md`**（看总览表 + §v0.74.0）；本文件只是**当轮状态快照**。

## 1. 当前状态（下一轮开工前先核对）

| 项 | 状态 |
|---|---|
| 推送 | ✅ **已完成** —— `95a2f90..171145a`，53 个提交，`待推 0` |
| pre-push 本地快层 | ✅ **全绿放行**（`[pre-push] ✅ 本地快层全绿 ✓ ⇒ 放行 ✓`） |
| CI run **`36308599184`**（head `171145a`） | 🔄 **进行中**：**16 success · 11 in_progress · 0 失败** |
| 课程门禁 | ✅ **36 目标 · 328 checked · 99 open · 0 判负**（未受影响） |
| v0.74.0 | ⬜ **未开始**（E01–E04 · E21–E23 · E27–E31） |

**⇒ 下一轮第一件事**：读 `/tmp/ci-verdict-final.log`（终局监控会自动写入
`ci-green.py --run` 的判定 + 退出码 + 逐 job 结论表）。
⚠ 若该文件不存在或只有 `in_progress`，直接跑：
```bash
cd <repo> && gh run view 36308599184 --json status,conclusion,jobs
python3 scripts/ci-green.py --run 36308599184   # ← 注意是 --run，不是位置参数 ✗
```
⚠ **`perf-gate` 是 `continue-on-error`（第一轮只报不拦）⇒ 看整轮结论会把它读成绿** ✗，**必须逐 job 看** ✓。

## 2. 本会话查清、**别重新发现一遍**的事实

### 2.1 pre-push hook 的真面目（推送曾被它拒绝）
- `git config core.hooksPath` = **`scripts/githooks`** ⇒ 钩子在 **`scripts/githooks/pre-push`** ✓
  （不在 `.git/hooks/` —— 我第一次找错地方 ✗）。
- 它跑**本地快层**，而且 **`--fast` 并【不】跳过台账门禁** ✗✓：
  日志明确写「**gates：缺口台账（`--strict` ✓ 本地必须跑完）**」✓。
- 实测红灯序列：`✅ workflow · ✅ fmt · ✅ clippy · ✅ gates：课程门禁 · ❌ gates：缺口台账(exit=1)`。
  ⇒ **`--fast` 会跳过 workspace test，但不会跳过台账** ✓。
- **逃生门**（钩子自己写的）：`git push --no-verify` 或 `SOKO_SKIP_HOOK=1 git push` ——
  **但"不许成为默认动作"** ✗，且要在 `STATUS.md` 写明原因 ✓。

### 2.2 G-44 已结案（**依它自己的 `workaround`**，不是"为绿改台账" ✗）
- G-44 的 `workaround` 原文就给了出路：「**或确认是发布二进制 vs 仓库构建的差异后更新台账**」✓。
- 证据链：复现件 ①②③ **一直通过** ✓；**只有 ④** 因 `check.py --bin target/debug/sokonanoda`
  自报 **0.72.0**（仓库构建过期）、仓库钉 **0.73.0** ⇒ `--bin` 分支**早就存在**的版本核对**正确地** exit 2 ⇒
  **是前置/环境，不是缺口** ✓。`cargo build -p sokonanoda-cli --locked`（27.36s）后
  该二进制自报 **0.73.0** ⇒ **同一份复现件 exit 0 → exit 1（已修）** ✓，`judge()` 判 `一致=True` ✓。
- 已把 `status: open → fixed`、`fixed_in: 0.73.0` + **完整证据链**写进 `docs/gaps/ledger.jsonl` ✓。

### 2.3 ⚠ 两个仍在的隐患（**别在本阶段顺手做** ✗）
1. **台账跷跷板**：**两条条目共用同一个复现件、状态相反 ⇒ 永远不可能全绿** ✗。
   同类**共 4 组**：`G-07·G-44` · `G-14·G-18` · `G-11·G-16` · `G-29·G-31`。
   ⇒ 建议：复发类条目给**独立复现件** ✓，或加 `recurs_of` 字段 ✓。
2. **CI 盲区**（G-44 `notes` 自记）：**「最近 12 轮 CI 的 `ledger` job 全部是 `skipped`」** ——
   只改 `docs/**` ⇒ `paths-filter` 跳过 rust job ⇒ **台账门禁长期没在 CI 上跑过** ✗。

### 2.4 ⚠ `scripts/soko` 的二进制解析顺序有个坑（**我踩了**）
顺序 = `$SOKONANODA_BIN` → **版本匹配的仓库构建** → 缓存。三份二进制实测：

| 路径 | 自报 | 说明 |
|---|---|---|
| `target/debug/sokonanoda` | **0.73.0** | 我重建的（**未优化**）—— **现在被 soko 选中** ⇒ 门禁从 0.48s 变 **>55s** ✗ |
| `target/release/sokonanoda` | 0.72.0 | 旧，版本不匹配 ⇒ 被跳过 |
| 缓存 `~/.local/share/sokonanoda/bin/sokonanoda` | 0.73.0 | release，**快** |

⇒ **重建 debug 修好了"版本不一致"，却让门禁慢了约两个数量级** ✗✓。
**待定**（需用户拍板，**我没擅自再动环境** ✗）：① 清掉/改名 debug 产物让 soko 回落缓存 ✓；
② 重建 release；③ **改 `scripts/soko` 解析顺序：debug 与 release 都匹配时优先 release** ✓（根因修复）。

### 2.5 `ci-green.py` 的正确用法
```bash
python3 scripts/ci-green.py --run <run-id>   # ✅
python3 scripts/ci-green.py      <run-id>    # ❌ unrecognized arguments（我第一次就写错了）
python3 scripts/ci-green.py --selftest       # 判据通道自检
```
退出码：**0 真绿 · 1 红 · 2 假绿 · 3 不能判** ✓。

## 3. E00 的产出与**可信度边界**（读 `docs/gaps/criteria-census.md` 时务必知道）
- **已修 20 条**（**都读过源码** ⇒ 可信度最高 ✓）；**变异总账 90 条**（10 个自检入口全绿 ✓）。
- ⚠ **未复核约 95 条**（P2 为主，需 cargo 才能升级/验证）。
- ⚠ **子代报告的采信口径**（抽样校准过）：**对"代码里写的是什么"可靠** ✓（B 3/3 · D 11/12）；
  **错在「定级」**（把有打印、有成文理由的跳过报成「④ 静默当绿」✗）与**「转录」**
  （crate 路径写错 ✗）⇒ **引用位置一律先 `ls`/`grep` 确认** ✓。
- ⚠ **`check.py` 的行号全部漂移过**（本会话给它加了 ~90 行）⇒ census 里有**逐条对照表** ✓，
  **引用它一律先 `grep -n`** ✗。

## 4. 下一轮的开场动作（建议顺序）
1. 读 CI 终局（§1）⇒ 确认 **③ 逐 job 真绿**；若红，先修红项（**别开 v0.74**）。
2. 读 `docs/PLAN-0.74-0.79.md` **总览表 + §v0.74.0**（不凭记忆 ✗）。
3. 从 **E01** 起，逐环节：**先判红 → 一处一 commit → 反向验证 → 总览表状态列标 `✅<commit>` + 回写台账** ✓。
4. 顺手决策 §2.4 那个二进制解析顺序问题（**它现在让每次 `git push` 都慢十几倍** ✗）。

---

## 5. 前置阶段已收尾 ✅（供下一会话对齐基线）

| 项 | 证据 |
|---|---|
| **推送** | ✅ `fccd7f7..b63e77f  main -> main`，**待推 0** ✓（中途遇 non-fast-forward：远端先前进到 `fccd7f7`（e2e 台账，针对我推的 `171145a`）⇒ **没有强推** ✗，先 `fetch` 查清**文件零重叠**（它动 `docs/e2e/**`、我动 `docs/HANDOFF`/`docs/PLAN`）⇒ **rebase 干净** ⇒ 再推 ✓） |
| **CI 真绿** | ✅ run **`36308599184`** ⇒ `ci-green.py --run` **exit 0** ✓：28 success · **1 skipped（`fast-fail`——条件 job，失败才跑 ⇒ 跳过恰恰是绿的证据 ✓，不是"拿 skipped 当绿" ✗）** · **0 failure** · **重活 10/10 实跑且 success** ✓。**这是本阶段的验收基线。** |
| **课程门禁** | ✅ 36 目标 · 328 checked · 99 open · 0 判负 |

## 6. E01 的**判红基线已确立** ✅（下一会话可直接接着做）

**E01** = 给 `Rel.comp` / `Function.comp` 补 `•` / `∘` 记法（`infixr`）。
PLAN 说「全课程没有 `∘`/`•` 记法声明」——**已实测确认** ✓：

```bash
grep -rnE 'infix.*" (∘|•) "' courses/set-theory/     # ⇒ 空 ✓
```
定义行也核过：`lib/Rel.sokonanoda:47` 的 `def Rel.comp (A B C : Type) (r : Rel A B) (s : Rel B C) : Rel A C`
· `lib/Fun.sokonanoda:61` 的 `def Function.comp (α β γ : Type) (g : β → γ) (f : α → β) : α → γ` ✓。

**⭐ 判红的确切证据（内核自己给的，可直接当反向验证的判据 ✓）**：
写一个用 `∘` 的文件去判卷，内核报
> **`符号 '∘' 在本文件里还没有声明过记法；先用 infix/notation 命令声明它，或改用点名写法`**（rule = `notation`）

⇒ **补上 `infixr` 声明后这条必须消失** ✓；**撤掉声明必须让它回来** ✓ ——
**这就是 E01 的「先判红 → 反向验证」闭环** ✓，下一会话不必重新构造 ✓。

⚠ **两个坑（我踩过，别重踩 ✗）**：
1. **判卷夹具别放 `/tmp`**：`import Fun` 是按**入口文件所在目录**解析的，放 `/tmp` 会报
   「找不到模块 `Fun`」✗ ⇒ **那种失败与记法无关**，**别把它当成判红证据** ✗
   （我第一次就混淆了：两者**同时**出现 —— 一条 `找不到模块`、一条 `∘ 没有记法` ✓，
   只有**后者**是 E01 的证据 ✓）。
2. **记法不改计数**：`AGENTS.md` 硬规则 3 写明记法是「**源级糖，不引入新语义、不产生事件**」✓
   ⇒ 补声明后**课程门禁应仍是 36/328/99/0** ✓ —— 若计数变了，说明**动作错了**，
   不是"预期内的变化" ✗。
