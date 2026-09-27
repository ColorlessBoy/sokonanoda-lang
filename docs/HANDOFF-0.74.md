# 交接单 —— 开 v0.74.0（E01 起）

> 写于 2026-09-27，上一会话上下文 ~49 万 ⇒ **按换会话信号收尾** ✓。
> **唯一真相 = `docs/PLAN-0.74-0.79.md`**（顶部有「⚡ 执行索引」；环节与判据**以当前发版点章节为准，不凭记忆** ✗）。
> 本文件只是**起点快照**，不取代 PLAN 或 census ✓。

## 1. 当前阶段

**🚀 v0.74.0「记法补齐 + 编译体验修复 + Infoview 面」的起点。**
**Stage ①（推一次让 CI 真验）已完成并逐 job 真绿** ✓ —— 前置阶段收官，从干净上下文开 v0.74 正合适。

| 验收项 | 证据 |
|---|---|
| 推送 | ✅ `fccd7f7..b63e77f  main -> main`，**待推 0** ✓ |
| **CI 逐 job 真绿** | ✅ run **`36308599184`** ⇒ `python3 scripts/ci-green.py --run 36308599184` **exit 0** ✓：**28 success · 1 skipped · 0 failure · 重活 10/10 实跑且 success** ✓ |
| 那个 skipped | = **`fast-fail`**，**条件 job（失败才跑）⇒ 跳过恰恰是绿的证据** ✓，**不是"拿 skipped 当绿"** ✗ |
| 课程门禁 | ✅ **36 目标 · 328 checked · 99 open · 0 判负** |
| 工作树 | 干净 ✓（`git status --short` 空） |

## 2. 剩余环节清单（PLAN §v0.74.0）

**E01–E04 · E21–E23 · E27–E31**（共 11 个）。**只有 E01 做过判红，其余全部未开始** ⬜。

| 环节 | 一句 | 判红状态 |
|---|---|---|
| **E01** | `Rel.comp` / `Function.comp` 补 `•` / `∘` 记法（`infixr`），涉及单元改用记法 | ✅ **判红已完成**（证据见 §7），**实现未做** ⬜ |
| E02 | `Set.prod` 从单元⑤画布**收进 `lib/Prod.sokonanoda`** | ⬜ |
| E03 | `r ⁻¹`（`Rel.inv`）与 `A ≈ B`（`Set.Equiv`）记法（速查表点名"写不了"的那批） | ⬜ |
| E04 | hover 折记法（根因 `crates/front/src/compile/check/mod.rs:1220-1262`） | ⬜ |
| E21 | Infoview 声明卡片**去掉多余的「目标 ⊢」行** | ⬜ |
| E22 | **build/rebuild 以项目为默认目标** | ⬜ |
| E23 | **build/rebuild 加进度**（状态栏那一路） | ⬜ |
| E27 | **Infoview 内导航到定义**（现在只发 `ready`/`reveal`，`{a}`/`∈`/声明名点不动） | ⬜ |
| E28 | Infoview 空态/错误态判据（三种文案已实现、**无 e2e**） | ⬜ |
| E29 | Infoview 进度区覆盖 build/rebuild（E23 漏的那一路） | ⬜ |
| E30 | Infoview 增加「项目」区块（**`requires_warning` 必须显眼、不许只在 tooltip**） | ⬜ |
| E31 | 补「Clean Cache（清除缓存）」命令（CLI 早有 `build --clean`，**扩展里没有入口**） | ⬜ |

⚠ **PLAN 另有 4 条硬约束**（本阶段**不碰**）：内核判定 · bump/release · 跨模块重构 · **G4 只比名字不比命题（属 v0.79）**。

## 3. 已做完、**不需要新会话重做**的

| 项 | 状态 |
|---|---|
| **开工前 5 前置** | ①计划入库 ✅ · ②真绿脚本 ✅ · ③**worktree ⬜（唯一未做，可与 v0.74 并行）** · ④磁盘+守卫 ✅ · ⑤回退点 ✅（PLAN `:28-29` 是权威 ✓） |
| **E00 判据强度普查** | ✅ 总览表已标 `✅ 99a28db（变异总账 90）`。产出 = `docs/gaps/criteria-census.md`（**从属 PLAN**，不是计划唯一真相 ✓） |
| **G-44** | ✅ **已结案**（`open → fixed`，`fixed_in: 0.73.0`）—— 依它**自己的 `workaround`**：「或确认是发布二进制 vs 仓库构建的差异后更新台账」✓。证据链已写进 `docs/gaps/ledger.jsonl` |
| **G-07 跷跷板根因** | ✅ **已查清、留痕不阻塞**：G-44 与 G-07 **共用同一个复现件、状态曾相反** ⇒ exit 0 ⇒ G-07 红；exit 1 ⇒ G-44 红 ⇒ **永远不可能全绿** ✗。同类**共 4 组**：`G-07·G-44` · `G-14·G-18` · `G-11·G-16` · `G-29·G-31`。⇒ 建议复发类条目给**独立复现件**，或加 `recurs_of` 字段 ✓（**别在本阶段顺手做** ✗） |
| **E00 的 20 条已修** | ✅ 都读过源码、各带先判红 + 反向验证 + 独立 commit ✓ |

## 4. 已知坑（**别再重新发现一遍** ✗）

| 坑 | 应对 |
|---|---|
| 本机删 `target/` 下的文件 ⇒ **EPERM** | 用 `rm -rf` 且放开批量删除阈值；或改用 `CARGO_TARGET_DIR=/tmp/...` 构建 ✓ |
| **`cargo clean` 被 EPERM 拦** | 同上；回收路径见 `scripts/target-hygiene.py` 的提示 ✓ |
| 托管 `python3` **缺 pyyaml** ⇒ 假红 | 用 **`/opt/homebrew/bin/python3`** ✓（本会话全程用它 ✓） |
| **macOS `grep` 的 BRE 没有 `\|` 和 `\b`** | **一律 `grep -E`** ✓ |
| `scripts/soko gate` 会因 **G-07 红** | **非本会话造成** ✓（详情 §5）；**别为了让 gate 绿而改判据迁就现状** ✗ |
| **`query project` 有重编副作用** | 实测会写/删 `courses/*/.sokonanoda/compiled/*.tmp` ⇒ 面板刷新**优先用 LSP 的 `soko/project` 推送回答** ✓ |
| ⚠ **`scripts/soko` 的二进制解析顺序** | 顺序 = `$SOKONANODA_BIN` → **版本匹配的仓库构建** → 缓存。**`target/debug/sokonanoda` 一旦版本匹配就会被优先选中** ⇒ 门禁跑**未优化 debug**、从 0.48s 变 **>55s** ✗（本会话踩过）。**待用户拍板**：清掉 debug 产物 / 建 release / 改解析顺序优先 release ✓ |

## 5. CI / 门禁的现状与边界（**重要**）

- **pre-push 钩子在 `scripts/githooks/pre-push`**（`git config core.hooksPath` = `scripts/githooks` ✓，**不在 `.git/hooks/`** ✗）。
- ⚠ **`--fast` 并【不】跳过台账门禁** ✗：日志写「**gates：缺口台账（`--strict` ✓ 本地必须跑完）**」✓。
  灯序实测：`✅ workflow · ✅ fmt · ✅ clippy · ✅ gates：课程门禁 · ❌ gates：缺口台账`（当时因 G-44）。
- **逃生门**（钩子自己写的）：`git push --no-verify` 或 `SOKO_SKIP_HOOK=1 git push` —— **不许成为默认动作** ✗。
- ⚠ **CI 有 `concurrency: cancel-in-progress: true`**（`ci.yml:8-10`）⇒ **在 CI 跑着时推新提交会把它顶掉** ✗
  （`AGENTS.md` 记的「连续 4 次 push ⇒ 四轮全 cancelled」就是这个）⇒ **等一轮跑完再推** ✓。
- ⚠ **CI 盲区**：只改 `docs/**` 的推送会被 `paths-filter` 跳过 rust job ⇒ **`ledger` job 曾连续 12 轮 `skipped`** ✗。
- **判 CI 一律用 `scripts/ci-green.py --run <id>`**（**是 `--run`，不是位置参数** ✗）；退出码 **0 真绿 · 1 红 · 2 假绿 · 3 不能判**。
  ⚠ **`perf-gate` 是 `continue-on-error`（第一轮只报不拦）⇒ 看整轮结论会读成绿** ✗，**必须逐 job 看** ✓。

## 6. 关键指针（file:line）

| 要什么 | 去哪 |
|---|---|
| **计划总账 / 执行索引 / 环节与判据** | `docs/PLAN-0.74-0.79.md` —— 顶部「⚡ 执行索引」（`:1-30`）· **总览表状态列**（`:220-234` 一带，**进度的单一落点**）· **§v0.74.0**（`:296` 起，标题已修成 `E27–E31` ✓） |
| E00 产出 + 交接 + 更正 + 行号刷新表 | `docs/gaps/criteria-census.md` |
| 方法口径（四类弱判据 + 第⑤类「自指期望」） | `docs/design/criteria-strength.md` |
| 缺口台账（契约：`status: fixed` ⇒ 复现件必须 exit ≠ 0） | `docs/gaps/ledger.jsonl` + `scripts/gap.py` |
| 判 CI 真绿 | `scripts/ci-green.py --run <id>` |
| 磁盘/`target` 守卫 + 回收路径 | `scripts/target-hygiene.py` |
| 5 前置详情 / goal 适配 / 防翻车复盘 | `docs/design/PLAN-appendix-goal-and-antifragile.md`（附录，**日常不用读**） |

## 7. 下一步第一件事：**E01 的判红**（**只读**，产出写回 §2 与 PLAN 状态列）

**E01** = 给 `Rel.comp` / `Function.comp` 补 `•` / `∘` 记法（`infixr`），涉及单元改用记法。
**判红基线已确立** ✅（本会话实测，下一会话**不必重新构造** ✓）：

```bash
# ① 全课程没有 ∘/• 记法声明（与 PLAN 描述一致 ✓）
grep -rnE 'infix.*" (∘|•) "' courses/set-theory/     # ⇒ 空 ✓
# ② 两个目标定义（PLAN 给的行号已核 ✓）
#    lib/Rel.sokonanoda:47   def Rel.comp (A B C : Type) (r : Rel A B) (s : Rel B C) : Rel A C
#    lib/Fun.sokonanoda:61   def Function.comp (α β γ : Type) (g : β → γ) (f : α → β) : α → γ
```

**⭐ 判红的确切证据 = 内核自己给的诊断**（可直接当反向验证的判据 ✓）：
写一个用 `∘` 的文件判卷，内核报
> **`符号 '∘' 在本文件里还没有声明过记法；先用 infix/notation 命令声明它，或改用点名写法`**（rule = `notation`）

⇒ **补上 `infixr` 后这条必须消失** ✓；**撤掉声明必须让它回来** ✓ = E01 的「先判红 → 反向验证」闭环 ✓。

⚠ **两个坑（本会话踩了第一个 ✗，别重踩）**：
1. **判卷夹具别放 `/tmp`** ✗：`import Fun` 按**入口文件所在目录**解析 ⇒ 会报「找不到模块 `Fun`」✗。
   **那种失败与记法无关**，**别当成判红证据** ✗（实测两条诊断**同时**出现：一条 `找不到模块`、
   一条 `∘ 没有记法` —— **只有后者**是 E01 的证据 ✓）。夹具放课程树内，或判卷后即时清理 ✓。
2. **记法不改计数** ✓：`AGENTS.md` 硬规则 3 —— 记法是「**源级糖，不引入新语义、不产生事件**」
   ⇒ 补声明后**课程门禁应仍是 36/328/99/0** ✓。**计数若变了就是动作错了**，不是"预期内的变化" ✗。

**⚠ 本会话已把这条 E01 改动**（给 `lib/Rel.sokonanoda` 加 `infixr:80 " • " => Rel.comp`）**完整还原** ✓
（`git checkout --`）—— **当前工作树是干净的、没有半成品** ✓，新会话从零开始 ✓。

## 8. 环境状态

- **`target/` 已清到约 2 GB**（本会话起点 3.0 GB；`deps/*.rcgu.o` 988 个 · `incremental/` 527.7 MB · **无第二套 target** ✓
  —— 由 `scripts/target-hygiene.py` 实测）。
- ⚠ **不要再改 `Cargo.toml` 的 `incremental`** ✗：`~/.cargo/config.toml` 里有 **2026-09-26 用户批准**的
  「**不要全局关增量**」（为消灭"跨版本二分 ⇒ 缓存颠簸 ⇒ 全量重编"，实测占 `cargo test` 墙钟 **97%**）。
  **磁盘靠"守卫 + 定期回收"解决，不靠关增量** ✓；**205GB 的磁盘成本已记进 `~/.cargo/config.toml` 的注释** ✓。
- **`target/lang`**：**已不存在** ✓。来源**至今没查清** ✓（已如实记在 `scripts/target-hygiene.py:98`），
  但守卫**会对"第二套 target"判红** ✓ ⇒ 「杜绝两套 target 并存」**已达成** ✓（不是悬空项 ✓）。
- **三份二进制的自报版本**（本会话实测）：`target/debug/sokonanoda` **0.73.0**（我重建的，**慢**）·
  `target/release/sokonanoda` **0.72.0**（旧）· 缓存 `~/.local/share/sokonanoda/bin/sokonanoda` **0.73.0**（release，快）·
  仓库钉 **0.73.0** ✓。详见 §4 最后一行。
