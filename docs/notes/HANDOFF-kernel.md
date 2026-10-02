# 内核线交接单（换会话用）

> 写于 2026-10-03 05:0x · 分支 **main**（本地，**未推送** —— 用户要求只留本地 ✗ 别推）
> 起点：上一版（03:0x）接棒时 HEAD = `e1591d01`；本轮两棒依次落了
> **G-82**（`0a2b1b69`…`ed1dc5f7`）与 **G-85**（`93ecc197` + `abefd5a4`）✓。
> ⚠ 课程线**同时在写** `courses/**`（本轮实测：`740d5fad`「#5 批量适配第一批：105 处豁免删掉」
> 在我们两次语料扫描之间落地 ⇒ **语料对拍前先 `git status` / 先记 HEAD** ✗，否则差异不是你的 ✓）。
> （`scripts/docs-expiry.json` 已登记 2026-11-15 过期 ✓）

## 0. 一句话现状

内核线 **0.81.0 的 9 条 blocker 全清** ✓ · **G-81（顶≡底）已修** ✓ · **G-84 已收口** ✓ ·
**G-82（声明卡片剥绑元）已修并关账** ✓ · **G-85（省前导隐式实参 + 结果再收一个实参）已修并关账** ✓；
手上只剩**一条读数对齐待用户回答**（G-83 ✗）。

## 1.5 G-85（**本轮已修** ✓ —— 起因是课程线 #5 普查的「86 处红」，但**那 86 处不是它** ✗）

* **缺口**：`And.right : {a b : Prop} → And a b → b`（k=2 · m=1）—— n=1(`And.right h`) ✓ ·
  **n=2(`And.right h x`) ✗** · n=3(`And.right p (∀…) h x`) ✓ · n=4 ✓；坏的只有 **`m < n <= k`**。
  诊断：`期望 Sort(0)，实际是 And …` ✗（`h` 被按位置装进 `a : Prop`）。
* **病根**：`elab.rs::try_implicit_application` 路线③（富余实参落到**结果**上）靠**展开结果类型**
  造虚拟层 —— 而 `And.right` 的结果是**变量** `b` ✗ ⇒ 展不动 ⇒ 落到「旧写法」分支 ⇒ 按位置装 ✗。
* **修法**（front-only ✓）：`surplus_layers` 抽成纯函数 + 路线③ 加**第二趟**（第一趟失败 **且
  `args.len() <= k`** ⇒ 旧写法**结构上够不着显式层** ⇒ 读法唯一 ✓：先解前导隐式参数、代进结果再展 ✓）。
  ⚠ **`args.len() <= k` 不能少** ✗：少了它 `L03-eq-type-level` 从 exit 0 变 exit 1 ✗
  （`Eq.refl.{2} Type A` 这类**宇宙显式给出**的调用被误判 ✗ —— **守卫太宽的典型面孔** ✓）。
* **判据**：`bash docs/gaps/repro/G85-dropped-implicits-overapplied.sh` ⇒ **exit 1** ✓（反向验证：
  修复前 `checked=3 && failed=1` ⇒ exit 0 ✓；⚠ 判据必须用 `checked/failed` **计数** ✗ —— 内核拒绝那条的
  `failed[].name` 是 `None` ⇒ 按名字判会**静默判绿** ✗）· front 单测 ✓ · CLI e2e
  `crates/cli/tests/implicit_application.rs` **3/3** ✓ · 红线：同一批文件 × 两二进制 ⇒
  `grade --json` **344 文件只 1 处不同**（G73 复现件：接受/拒绝与计数不变 ✓，只有一条拒绝的**文案**变 ✓）、
  `query goals` **133 文件 0 差异** ✓。
* **⚠⚠ 两条对课程线 #5 普查的更正**（**下一棒别再照那 86 处开工** ✗）：
  1. **那 86 处「红」是普查工具的产物，不是引擎缺口** ✗ —— `notation-lint.py::census_candidate` 是
     **逐行**改写器，**跨行**应用会把实参**删错**（实测 `unit72-solution:33` ✗）。用**跨行忠实**改写器
     重放：**绿 74 · 红 0 · 不可机械改写 12** ✓（红变绿 0 / 仍红 0 / 新红 0）⇒ **要修的是工具**
     （跨行解析、或拒绝跨行命中标 `needs-manual` ✗）—— **课程线的文件，本轮没动** ✓。
  2. 那 12 处里 **2 处**（`unit04-solution` 的过度应用）**正是 G-85** ⇒ 修好后短写判卷
     **8/8 checked · 0 failed** ✓；其余 10 处是入门课 `course/` 的 `Exists`（**自建 axiom，前导参数显式**
     ⇒ 删不得 ✗）与同类过度应用（需工具的 `allowed` 也随富余实参一起数 ✓）。

## 1. G-82（**已修** ✓ —— 病根**两处**，上一版诊断只指对了症状 ✗）

* **症状**：`theorem (a b : Prop) (h : a) : a ∧ a := by constructor; sorry` 的卡片是
  `binders=[] ⊢ 整句声明类型` ✗（`by sorry` 那条对 ✓），且 `goal_runs` 把 `a`/`b`/`h` 全标
  `unknown_ident` ⇒ **整句判红** ✗。
* **病根两处**（缺一不可）：① `goals.rs::ctor_spine_case` 的目标头用只认 `Ident`/`App` 的
  `spine_head_args` ⇒ 记法目标 `a ∧ a`（`Expr::Notation{target:And}`）拿不到族名 ⇒ `open_goal`
  返回 `None` ✗；② `by` 引擎 `apply` 出来的是**内核口径全应用** `And.intro a a ? ?`，而归纳块
  构造子模板只记构造子自己的绑元（`ha`/`hb`）⇒ 旧守卫 `4 > 2` 把它整条拒掉 ✗。
  ⚠ **手写最小形测不出第②条**（`axiom` 视图有 4 个显式绑元 ⇒ 4 ≤ 4 过关 ✓）⇒ 既有测试全绿而
  缺口仍在 ✗ —— **第三次「声明与守卫之间有缝」**。
* **修法**：目标头走 `spine::head_and_args` ✓ + 前导实参只在**恰好等于目标实参个数**时才当族参数 ✓
  + 归纳块构造子补 `result_arg_names`（族的参数名，子洞期望类型按目标实参代换 ✓；同时用
  `refine_skeleton` 把 refine 代码动作**钉回原行为** ✗，不开新面 ✓）+ `walk.rs` 兜底改答题面状态 ✓。
* **判据**：`bash docs/gaps/repro/G82-card-does-not-peel-with-tactics.sh` ⇒ exit 1 ✓ · front 单测
  `compile::tests::open_card_peels_named_binders_when_the_body_has_tactics` ✓ · CLI e2e
  `query::query_goals_card_peels_binders_with_tactics`（文本 + **着色** ✓）· 全语料
  `grade --json` **344 文件 0 差异** ✓、`query goals` **133 文件 0 差异** ✓（语料里没有那种形状 ✓）。
* 细节 ⇒ commit `0a2b1b69`/`c33abcf0` + 台账 **G-82** 的 notes ✓。

## 2. 上一棒顺手做的两件（都已落 ✓）

* **G-84 的假绿断言换掉了** ✓：`crates/cli/tests/extension.rs` 那条「比我们自己刚 stage 的文件」
  的断言改名 `the_staged_cli_matches_the_extension_version` 并更正文档 ✓；新增
  `the_installed_cli_resolves_in_a_fresh_login_shell`（真跑扩展代码 + 临时 HOME + **新开登录 shell** ✓，
  并要求驱动器报出 `✓ ③`/`✓ ④` 防守卫空转 ✓）。⚠ 它在 CI 的 `test` lane 会跳过（那条 lane 不 stage `bin/` ✗）。
* **修掉一条既有红** ✗：`command_naming_inventory_covers_every_contributed_command` —— G-84 给
  `package.json` 加了 `sokonanoda.uninstallCli` 却没进 `docs/design/command-naming.md` §1 盘点表 ⇒
  补行 + 同文件压缩一行（**净增 0** ✓，不抬文档预算 ✓）。

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
