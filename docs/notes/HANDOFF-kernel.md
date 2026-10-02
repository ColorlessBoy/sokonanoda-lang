# 内核线交接单（换会话用）

> 写于 2026-10-03 03:0x · 分支 **main**（本地，**未推送** —— 用户要求只留本地 ✗ 别推）
> 上一版（2026-10-03 00:15）的起点是 `d039aca5`；**本轮接棒时 HEAD 已是 `e1591d01`**
> （课程线又推了 4 笔：`93027846` 单元112 · `c40a38f1`/`b573956e` 文档预算/交接单 · `e1591d01` 记法普查）
> —— **内核文件没被动过** ✓，所以起点仍然一致 ✓。
> （`scripts/docs-expiry.json` 已登记 2026-11-15 过期 ✓）

## 0. 一句话现状

内核线 **0.81.0 的 9 条 blocker 全清** ✓ · **G-81（顶≡底）已修** ✓ · **G-84 已收口** ✓ ·
**G-82（声明卡片剥绑元）本轮已修并关账** ✓；手上只剩**一条读数对齐待用户回答**（G-83 ✗）。

## 1. G-82（**本轮已修** ✓ —— 病根**两处**，上一版诊断只指对了症状 ✗）

**症状**：`theorem (a b : Prop) (h : a) : a ∧ a := by constructor; sorry` 的声明卡片是
`binders=[] ⊢ (a : Prop) → (b : Prop) → (h : a) → a ∧ a` ✗（`by sorry` 那条是对的 ✓），
而且 `goal_runs` 把 `a`/`b`/`h` 全标成 `unknown_ident` ⇒ **整句判红** ✗。

**病根（实测钉死，两处，缺一不可）**：

1. `crates/front/src/compile/goals.rs::ctor_spine_case` 的目标头用
   `spine_head_args`（**只认 `Ident`/`App`**）⇒ 记法目标 `a ∧ a` 的源 AST 是
   `Expr::Notation { target: "And" }` ⇒ **族名拿不到** ⇒ 模板查不到 ⇒ 整条
   `open_goal` 返回 `None` ✗。
2. **认了记法也不够** ✗：`by` 引擎 `apply` 出来的是**内核口径的全应用**
   `And.intro <族参数…> <字段…>`（实测 `And.intro a a ? ?`），而归纳块的构造子模板
   **只记构造子自己的绑元**（`And.intro` 的 `ha`/`hb`）⇒ 旧的守卫
   `val_args.len() > binder_names.len()`（4 > 2）把它**整条拒掉** ✗。
   ⚠ **手写最小形测不出第 2 条**：`axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b`
   这种「族结果 axiom」视图有 **4 个显式绑元** ⇒ 4 ≤ 4 过关 ✓ —— 所以既有测试（含 G-81 那条
   六形矩阵）全绿而缺口仍在 ✗（**又一个「声明与守卫之间有缝」**）。

**修法（三刀，都在 front，判定一字未动 ✓）**：

* 目标头改走 `crate::spine::head_and_args`（**既有**的记法感知入口，`by` 引擎同款 ✓）；
* 值 spine 多出来的**前导实参**只在**恰好等于目标实参个数**时才当族参数
  （形状可核对 ✓；对不上照旧保守返回 `None` —— 不猜位置 ✗）；
* 归纳块构造子补上 `result_arg_names` = **族的参数名** ⇒ 子洞期望类型按目标实参代换
  （目标 `And P Q` 的第二字段是 `Q` 而不是字面 `b` ✓）。
* **另加兜底保险**：`walk.rs` 的「空上下文 + 整句声明类型」兜底改成**题面状态**
  （`decl_root_state`，与 `by_root` **同源** ⇒ 顶 ≡ 底 ✓）—— 走查分解不了**不等于**
  题面没有上下文 ✓（这条单独也能把 G-82 的症状消掉，但**只有前两刀才真分解**：
  2 个子洞 + 期望类型 ✓）。

**判据（三条，都核过"真的拦"✓）**：

```bash
bash docs/gaps/repro/G82-card-does-not-peel-with-tactics.sh   # exit 0 → 1 ✓（gap.py close 已重放确认）
cargo test -p sokonanoda-front --lib open_card_peels          # 反向验证：撤掉修复 ⇒ 在 with_tactic 的 binders 上判红 ✓
cargo test -p sokonanoda-cli --test query query_goals_card    # 文本 + **着色**（goal_runs 无 unknown_ident）
```

**红线（判定正确性）** ✓：**全语料对拍** —— `grade --json` **344 文件 0 差异** ✓ ·
`query goals` **133 文件（含 `by`+`sorry`）0 差异** ✓。
**为什么是 0**（不是"没测到"）：脚本扫描全语料，**没有**任何文件含
「`by` 块首个 tactic 是构造子类 + 块里有 `sorry`」这种形状（**0 个** ✓）⇒ 修复只动
**旧行为本来就错**的形状 ✓。

## 2. 本轮顺手做的两件（都在 §5 的清单里）

1. **§5.3 假绿断言已换** ✓：`crates/cli/tests/extension.rs` 里那条「比我们自己刚 stage 的文件」
   的断言，改名 `the_staged_cli_matches_the_extension_version` 并**更正文档**（它管**暂存产物** ✓，
   不是用户装到的那只 ✗）；新增 **`the_installed_cli_resolves_in_a_fresh_login_shell`** ——
   真跑扩展代码（临时 HOME + **新开登录 shell**）断言 `command -v sokonanoda` 解析到安装位 +
   那只的 `--version` == 插件版本，并要求驱动器报出 `✓ ③`/`✓ ④`（**防守卫空转** ✓）。
   ⚠ 它在 **CI 的 `test` lane 会跳过**（那条 lane 不 stage `bin/`）—— 真正在 CI 里咬 G-84 的是
   `gap.py check` 的复现重放；**而下面第 3 条说那条现在也够不着** ✗。
2. **修掉一条既有红** ✗：`command_naming_inventory_covers_every_contributed_command` ——
   G-84 给 `package.json` 加了 `sokonanoda.uninstallCli`，却**没进** `docs/design/command-naming.md`
   §1 的盘点表（main 上留了一条红 ✗）。补行 + 同文件压缩一行 ⇒ **净增 0**（不抬文档预算 ✓）。

## 3. 本轮发现、**没修**的两条（下一棒或用户拍板）

1. **G-84 的 CI 守卫其实是空转** ✗（**已核实**）：`G84-vscode-cli-install.sh` 要
   `editor/vscode/bin/*/sokonanoda`（`stage-lsp.js` 的产物），而 CI 的 `ledger` job **不 stage 它**
   ⇒ 复现件 `exit 2`（"环境不满足"）⇒ `gap.py` **响亮跳过、不判红** ✗ ⇒ 「G-84 已修」在 CI 里
   **没有东西在验** ✗。修法：给 `ledger` job 加一步 `node editor/vscode/scripts/stage-lsp.js`
   （**会动 CI 判据/时长 ⇒ 本轮没动** ✗，留给你拍板）。
2. **`decl_prefix_state` 的剥层数仍是"源位 λ 链长度"**（G-83/U1 那一族）：G-81 的注释写着
   正确口径是**源级具名 binder 列表**，但代码用的还是 `split_by_value(val).len()` ——
   函数型语句（`a → b → a`）那条路上它会不会多剥，**没测**（open 声明现在走 `info`，
   所以只在**失败/已证**声明上暴露）。要动就按 G-83 的授权一起动。

## 4. 待用户拍板（**别停下来等** ✓ 记在这里）

* **版本号**：G-84 与 G-82 都记在 **0.81.0**（未发布 ✓）。**并进 0.81.0** 还是 **bump 0.82.0**
  —— 后者要**重 stage 扩展自带 CLI** ✗（`editor/vscode/bin/**` 是构建产物）。用户拍板前**不要 bump**。
* **G-83**（全语料「顶 ≡ 底」读数对齐 ✗）：用户那台量到 `开放声明 875｜顶≡底 29｜顶≠底 841` ✗，
  我在当前树抽样（`g2`/`g5`/`subset_refl`/`mem_of_subset`/`not_symm_trans_implies_refl`）**全部顶≡底** ✓
  ⇒ **卡在等用户给 `git rev-parse HEAD` + 二进制 sha**。若他授权"即便抽样绿也照样加固"，
  动两条**已核实的代码缝**：① `decl_prefix_state` 的剥层数改成源级具名 binder 列表；
  ② 五处 `if trusted { … return; }`（`walk.rs` 约 646/919/1166/1316/1518）**早退前仍要写 `by_root`** ✗。

## 5. 已知坑（都踩过，别再踩）

1. **两条线共用 git index** ⇒ 提交一律 `git commit -- <路径>`，**禁** `git add -A` / `-a` ✗
   （已实锤双向交叉提交：`0ebace86` 扫进 208 个课程文件、`76aa9d67` 扫进 7 个 `crates/` 文件 ✗）。
   ⚠ **本轮实测**：课程线**正在同一棵树上写文件**（`git status` 中途从 `scripts/notation-lint.py`
   变成 `courses/set-theory/units/solutions/I.10/unit84-solution.sokonanoda` + 新的 `census-probe-*.sokonanoda`）
   ⇒ **语料对拍前先看 `git status`**，否则"差异"可能是别人刚改的 ✗。
2. **CLI≡LSP 那条测试会假红** ✗：`cargo test -p sokonanoda-cli` **不会**重建 `sokonanoda-lsp` 二进制
   ⇒ 先 `cargo build -p sokonanoda-lsp` ✓。
3. **内核 crate 的包名是 `sokonanoda`**（不是 `sokonanoda-kernel` ✗）：`cargo test -p sokonanoda` ✓
   （`cargo test -p sokonanoda-kernel` 报 `did not match any packages` ✗）。
4. **`query state` / `query goals` 一次几秒 CPU** ⇒ 全语料扫描要**并行**（`xargs -P 5`：
   344 文件 `grade` 约 6 分钟、133 文件 `goals` 约 3 分钟 ✓）；串行版约 **1.5 小时** ✗（实测）。
5. **`gap.py` 复现超时 300s**（CI 降到 60s）⇒ 慢复现件变「响亮跳过并计数」（不判红 ✓，但别当成已验 ✓）。
6. **复现件退出码约定**（`docs/gaps/README.md`）：`0` = 缺口仍在 · `1` = 已修 · `2` = 环境不满足。
7. **`gap.py close --note` 里的反引号会被 shell 吃掉** ✗（本轮实测：note 变成一串空格 ✗，
   就是上一版说的"heredoc 引号"同族）⇒ **note 用 Python 写**（`json` 直接改 `notes` 字段）✓，
   写完**必须回读** ✓。`close` 本身**会重放复现件**做预检 ✓（够不着/超时 ⇒ 拒绝关账 ✓）。
8. **内核不许跑 stable rustfmt** ✗：`cargo fmt --all` 会重排 24 个内核文件；fmt 门禁只覆盖三个教学 crate ✓；
   `crates/kernel/Cargo.toml` 保持 `0.5.0` 不动 ✓。
9. **文档预算**：`docs/` 顶层是 L1 冻结层 ⇒ 交接/笔记写 `docs/notes/` ✓ + 登记过期日 ✓。
   `docs/design/command-naming.md` 的**冻结额度就是当前行数**（104）⇒ 加行必须同文件减行 ✓。
10. **perf 类测试比绝对毫秒** ⇒ 并发跑扫描时出现过假红 ✓（单独复跑即绿 ✓）。
11. **别信"看起来对"**：这条线**三次**栽在「声明与守卫之间有缝」（受信路径不提 `by_root` ✓、
    缓存 `#[serde(default)]` 静默用旧答案 ✓、**本轮 G-82 第 2 条病根**：既有六形矩阵测试
    用的是 axiom 视图 ⇒ 4 ≤ 4 过关、真 prelude 的归纳视图 4 > 2 被拒 ✗）
    ⇒ 任何"必红/强制/例行"的声明都要**逐条核对它真的拦** ✓。

## 6. 交接验收（接棒人先跑这几条，确认起点一致）

```bash
git log --oneline -1                      # 期望 = 本轮内核线那笔（G-82）；课程线可能又在其上 ✓
git status --short                        # 期望只剩**课程线自己的**改动（courses/**）—— 别提交它们 ✗
bash docs/gaps/repro/G84-vscode-cli-install.sh; echo "exit=$?"   # 期望 exit 1（G-84 已修 ✓）
bash docs/gaps/repro/G82-card-does-not-peel-with-tactics.sh; echo "exit=$?"  # 期望 **exit 1**（G-82 本轮已修 ✓）
python3 scripts/gap.py check              # 期望 0 条不一致（G-82 已 fixed ⇒ 复现必须 exit 1 ✓）
```

## 7. 下一个最小可验证动作

**⚠ 先更正上一版的一条陈旧项** ✗：上一版说"下一个动作 = IA-4 的 **M4**（默认开 + 发版）"，
**M4 其实早已收口** ✓（本轮核实）：

* 代码：`crates/front/src/compile/implicit.rs::metavar_mode()` —— **不设开关 ⇒ `Engine`** ✓
  （`SOKO_METAVAR=0|off` 逃生门 · `=sibling` 永久回归臂 · 兼容旧 `SOKO_NOTATION_METAVAR=0`）；
* 判据：`crates/cli/tests/metavar_engine.rs`（"默认档 = engine" + "默认 ≡ sibling 回归臂" ✓，
  本轮 CLI 全绿 **399/0** ✓ 里就有它）；
* as-built：`docs/design/metavar-engine.md` **§11**（三态指纹表：默认 ≡ sibling 逐字节相同 ✓；
  版本 `0.79.0` → **`0.80.0`** 已发 ✓）；
* `docs/ONBOARDING.md` §0.2 的 #1 那行还写着"余片未做"（**陈旧** ✗，与 §11 冲突 —— 顺手记在这里，
  下一棒别再照它去追 M4 ✗）。

**⇒ 真正的下一个动作 = IA-4 的 `K1`（内核占位符）** —— 设计 §2.11 + 切片表 §4 ✓；
**D8 已由用户 2026-10-01 拍板 = (i) 内核占位符**（Lean 同构：内核 `Expr` 加占位符构造子，
**判定层硬拒**含占位符的声明；前端 `elab_expr` 协议不变 ✓）⇒ **不需要再问用户** ✓。
判据（设计原话）：**硬不变式**（含元变量 ⇒ 拒绝）· **全语料对拍 0 差异** · **golden/`--json` 无占位符** ✓。
⚠ **这是内核改动** ⇒ 动手前先读 `docs/architecture.md` **§6/§8**（内核台账 + gotchas：arena 生命周期、
panic→Result、`quiet_catch` 不可嵌套 ✓），并按硬规则走**三层回归 + 语料对拍 + 性能台账** ✓。
其后顺序 **B1 期望类型传播 → B2 元变量进项 → B3 → U1/U2 宇宙层** ✓。
⚠ **U1/U2 是课程线 #5 的前置**，别把顺序搞反 ✗。
另有用户已排序的 **G-68**（per-entry 缓存键 ⇒ 共享依赖重复编译 **4.14×**，判据用**编译次数**
这类结构计数 ✓ —— 别用绝对毫秒 ✗）。
