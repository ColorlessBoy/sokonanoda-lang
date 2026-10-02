# 内核线交接单（换会话用）

> 写于 2026-10-03 00:15 · 分支 **main**（本地，**未推送 105 笔** —— 用户要求只留本地 ✗ 别推）
> 最近一笔 commit = **`d039aca5`**（G-84 关账）· 临时产物：接棒人看完即可删
> （`scripts/docs-expiry.json` 已登记 2026-11-15 过期 ✓）

## 0. 一句话现状

内核线 **0.81.0 的 9 条 blocker 全清** ✓、**G-81（顶≡底）修复已落** ✓、**G-84（VS Code CLI 安装/卸载）已收口** ✓；
手上唯一"修了一半"的是 **G-82**（病根已钉死、修复未开工 ✗）；另有一条**读数对齐**待用户回答（G-83 ✗）。

## 1. G-82 病根（**已钉住，别丢**）

**一句话**：**证明体的第一个招式决定生死** —— 卡片剥不剥绑元，取决于洞前面有没有 tactic ✗。

* `by sorry` ⇒ 卡片 `binders=[a,b,h]` ⊢ `a ∧ a` ✓（对）
* `by constructor; sorry` ⇒ 卡片 `binders=[]` ⊢ `(a : Prop) → (b : Prop) → (h : a) → a ∧ a` ✗（整句、上下文空）

最小复现（两条同形、只差第一个招式；完整件 `docs/gaps/repro/G82-card-does-not-peel-with-tactics.sh` ✓）：

```sokonanoda
theorem plain (a b : Prop) (h : a) : a ∧ a := by
  sorry
```

```sokonanoda
theorem with_tactic (a b : Prop) (h : a) : a ∧ a := by
  constructor
  sorry
```

**落点**：`crates/front/src/compile/goals.rs::goal_under_binders` ——
值位是 `Expr::Lambda … Expr::Hole` 时它按 λ 链逐层剥 ✓；但证明体里**有 tactic** 时走**兜底分支**
（`binders: Vec::new()` + `render_expr(ty)` ✗）⇒ 卡片退回整句声明类型 ✗。

**修复思路走到哪一步**：**根因已定位到分支，代码未改一行** ✗。
拟改法：兜底分支不要空上下文，改成按**声明自身的具名绑元**剥（源级 binder 列表）✓；
⚠ **别复用 `params_of_ty`** —— 它数的也是**内核前导 Forall**，会把返回类型里的 `∀`/`→` 一起数进去 ✗
（`walk.rs` 那条 G-72 注释踩过同款 ✓）。
判据已登记、现在是 **exit 0 = 缺口仍在** ✓；改完必须变 **exit 1** ✓。
⚠ 与 G-81 的交互：**顶 ≡ 底** 现在成立 ✓ ⇒ 顶会跟着这张错的卡片一起错 ✗（别只修顶 ✗）。

## 2. 「三件未落账的东西」—— 其实**都已落账** ✓

提醒里那三件（`walk.rs` ~122 行 · `query/tests.rs` ~19 行 · G-81/G-82 复现件）是**看到工作区未提交时的快照** ✗，
它们已经全部进了 **`5dd64b6e`**（`fix(query): G-81 顶 ≡ 底`）：

| 东西 | 落盘了吗 | 在哪 | 差什么才能落 |
|---|---|---|---|
| `crates/front/src/compile/check/walk.rs` | **✓ 已提交**（该 commit 里 +152/−… 行） | 分支 `main`（本地未推） | 无 ✓ |
| `crates/front/src/query/tests.rs` | **✓ 已提交**（+32 行） | 同上 | 无 ✓ |
| `docs/gaps/repro/G81-top-equals-bottom-root-state.sh` | **✓ 已提交**（+184 行，`5dd64b6e`） | 同上 | 无 ✓ |
| `docs/gaps/repro/G82-card-does-not-peel-with-tactics.sh` | **✓ 已提交**（+83 行，`5dd64b6e`） | 同上 | 无 ✓ |

自检命令：`git status --short -- crates/front/src/compile/check/walk.rs crates/front/src/query/tests.rs docs/gaps/repro/` ⇒ **空** ✓。

## 3. goal 状态 + 下一个最小可验证动作

* 持久化 goal（`goal-11dcda31-…`，**disarmed**）：目标是 **9 条 blocker**（G-56/58/59/64/71/72/74/75/76）
  ⇒ **全部 fixed、`fixed_in: 0.81.0`** ✓ —— **这个 goal 的正文已经做完** ✓（台账现状：`fixed 70 · open 20 · workaround 3 · wo-filed 1`）。
* **下一个最小可验证动作 = 修 G-82 的兜底分支**。证伪只要三条命令：
  ```bash
  bash docs/gaps/repro/G82-card-does-not-peel-with-tactics.sh   # 现在 exit 0（缺口在）；改完必须 exit 1
  cargo test -p sokonanoda-front --lib query::                  # 前端判据不许红
  cargo test -p sokonanoda-cli --test query --locked            # 顶≡底 的 CLI 侧判据（先 cargo build -p sokonanoda-lsp ✗ 见 §4）
  ```
* 之后的大块（用户已排序，别抢跑）：**G-68**（per-entry 缓存键 ⇒ 共享依赖重复编译 **4.14×**，判据用**编译次数**这类结构计数 ✓）→
  再回 **大点 1 IA-4** 的 **M4 → K1 → B1 → B2 → B3 → U1/U2 宇宙层**（它是**大点 4「隐变量课程适配」的前置** ✓）。

## 4. 已知坑（都踩过，别再踩）

1. **两条线共用 git index** ⇒ 提交一律 `git commit -- <路径>`，**禁** `git add -A` / `-a` ✗
   （已实锤双向交叉提交：`0ebace86` 扫进 208 个课程文件、`76aa9d67` 扫进我 7 个 `crates/` 文件 ✗）。
2. **CLI≡LSP 那条测试会假红** ✗：`cargo test -p sokonanoda-cli` **不会**重建 `sokonanoda-lsp` 二进制 ⇒ 先 `cargo build -p sokonanoda-lsp` ✓。
3. **`query state` 一次约 6s CPU**（判卷要重跑整份前缀 ✗）⇒ 全语料扫描 10–30 分钟；`gap.py` 的复现超时是 **300s**
   ⇒ 慢复现件会变成「**响亮跳过并计数**」（不判红 ✓，但别当成已验 ✓）。
4. **`gap.py clean_env()` 会设 `SOKONANODA_NO_PROJECT_ARTIFACTS`** ⇒ 需要模块根产物的复现件要自己在子进程里摘掉它 ✓。
5. **复现件退出码约定**（`docs/gaps/README.md`）：`0` = 缺口仍在（与 `open` 一致）· `1` = 已修 · `2` = 环境不满足；
   `scripts/expect-red.sh` 要的是**命令失败** ⇒ 反向验证要写成 `--criteria`（判据不成立时 exit≠0 ✓）。
6. **Python heredoc 里的 ASCII 双引号会炸** ✗ —— 我因此**两次**写了台账却没写进去（commit message 说"已立账" ✗）。
   ⇒ 一律用「」，并且**写完必须回读确认**（`python3 -c` 数一下该 id 的行数 ✓）。
7. **内核不许跑 stable rustfmt** ✗：`cargo fmt --all` 会重排 24 个内核文件；fmt 门禁只覆盖三个教学 crate ✓；
   `crates/kernel/Cargo.toml` 保持 `0.5.0` 不动 ✓。
8. **文档预算**：`docs/` 顶层是 L1 冻结层 ⇒ 交接/笔记写 `docs/notes/` ✓，并在 `scripts/docs-expiry.json` 登记过期日 ✓（否则 pre-commit 拦 ✗）。
9. **perf 类测试比绝对毫秒** ⇒ 并发跑扫描时出现过两次**假红** ✓（`perf_project` / `cli_build_heartbeat…`），单独复跑即绿 ✓。
10. **别信"看起来对"**：这条线两次栽在「声明与守卫之间有缝」（受信路径不提 `by_root` ✓、缓存里 `#[serde(default)]` 静默用旧答案 ✓）
    ⇒ 任何"必红/强制/例行"的声明都要**逐条核对它真的拦** ✓。

## 5. `d039aca5` 之后手上还有什么（**均未提交，因为都不是我的改动** ✓）

`git status --short` 现在只剩**课程线自己的**：`courses/set-theory/gaps/{C-112-cb-prop.sokonanoda(M), C-112-cb-step5, zzprobe, zzprobe2, zzprobe4, zzprobe5, zzprobe6}` ✓ —— **别提交它们** ✗。

真正的未完成清单（按建议次序）：

1. **G-82 修复**（open，复现件已就绪 ✓）—— 见 §1/§3。
2. **G-83 / G-81 全语料读数对齐**（open ✓）：用户在他那台机器上量到 `开放声明 875｜顶≡底 29｜顶≠底 841` ✗，
   而我在当前树抽样（`g2` / `g5` / `subset_refl` / `mem_of_subset` / `not_symm_trans_implies_refl`）**全部 顶≡底** ✓
   （用 `SOKO_STATE_ROOT=legacy` 显微镜判的 ✓，`by_root` 有值 ✓）⇒ **卡在等用户确认**（要他的 `git rev-parse HEAD` + 二进制 sha）；
   若他授权「即便抽样绿也照样加固」，就动两条**已核实的代码缝**：① `decl_prefix_state` 的剥层数改成**源级具名 binder 列表**；
   ② 五处 `if trusted { … return; }`（`walk.rs` 约 646/919/1166/1316/1518）**早退前仍要写 `by_root`** ✗（不许让"受信"变成"根状态静默退化" ✗）。
3. `crates/cli/tests/extension.rs` 那条**假绿断言**还没换 ✗（它比的是"自己刚拷的文件"）—— 真判据已由登记在册的
   `G84-vscode-cli-install.sh` 守住 ✓，但 Rust 测试里那条旧断言要改成「**新开登录 shell** 里那份的版本」+ PATH 行断言 ✓。
4. **版本号待用户拍板** ✗：G-84 的 CHANGELOG 记在 `## [Unreleased]` ✓，没动版本号（并进 0.81.0 / bump 0.82.0 ⇒ 后者要重 stage 扩展自带 CLI ✗）。
5. 大点 1 **IA-4**：M0/M1/M2/M3 ✓ 已收口 ⇒ 余 **M4（默认开 + 发版）→ K1（内核）→ B1 期望类型传播 → B2 元变量进项 → B3 → U1/U2 宇宙层**；
   设计在 `docs/design/metavar-engine.md` ✓。
6. **大点 3**：U1（G-81 全语料）见第 2 条；U2（G-84）**已收口** ✓；U3（编辑慢）在课程线侧待实测 ✓。

## 6. 交接验收（接棒人先跑这三条，确认起点一致）

```bash
git log --oneline -1                      # 期望 d039aca5
git status --short                        # 期望只剩 courses/set-theory/gaps/*（课程线的）
bash docs/gaps/repro/G84-vscode-cli-install.sh; echo "exit=$?"   # 期望 exit 1（G-84 已修 ✓）
bash docs/gaps/repro/G82-card-does-not-peel-with-tactics.sh; echo "exit=$?"  # 期望 exit 0（G-82 未修 ✓）
```
