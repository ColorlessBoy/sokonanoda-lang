# 当前快照（2026-09-26）

- **进度**：**64/66**（E2 50/50 ✓ + **批次 N** 记法×隐式参数×产品交互 14/16 ✓）
- **已发布**：**`sokonanoda v0.72.0`** ✓（`gh release list` 显示 **Latest** ✓ · 2026-09-25T23:25:07Z ✓）
- **A 组（A0–A5）全部落地** ✓：显示层混合形态（`ty_text` ASCII `->` **227 → 0**）· `{a}` 折回 ·
  `{a}` 可跳转（根因：开放练习的签名**一条 hover 行都没有**）· prelude 可跳转（F12 落到前奏源文件
  的真 span）· `flawed_equalities_refuted` 去点名（标记 5 → 3）· 洞的期望类型**两半分开钉**
  （显示副本必折 / 真相字段一个字节都不许折）。A0 守卫基线 **68 → 59**（结论：**59 是地板**）。
- **真宿主 e2e**：A1/A2/A3/A4 四条 ✓（全量 **32/32**）· 反向验证撤修复 ⇒ **1 passed / 3 failed** ✓。
- **B3 第一刀** ✓（`04d863a`）：**裸常量**的隐式插入（G-40 收口）—— `∅` = 裸 `Set.empty` 以前停在
  那个 Pi 上、`rfl` 判不出来 ✗ ⇒ 走 `solve_prefix` 路线 ② 从期望类型补 ✓；判据 + 反向验证 ✓。
  新登记 **G-41**（记法路径 + 隐式 binder ⇒ 真库 `328/0` 掉到 `226 checked · 14 判负` ✗）与
  **G-42**（见下"试了 5 次全撤"）⇒ **B2 仍被 G-42 挡住** ✗。
- **判据**：课程门禁 **36/328/99/0**（逐项不变 ✓）· front **744** / LSP **164** ✓ ·
  `notation-lint` 84 文件 ✓ · `docs-lint` ✓ · `gap.py check` ✓ ·
  **CI 确认绿** ✓（纪律④）：`36254524752` 与 **`36255846648`（P4）** ⇒ 都 **completed success**（唯一非 success 的 `fast-fail`
  是 **skipped**，按设计 ✓），覆盖 **P1/P2/P3/P6 + 路线③ + AGENTS 四条纪律** ✓。
- **P4（概览尺 + 整行装饰）** ✓：编译期间给当前文档加整行装饰 ⇒ 右侧**概览尺**（Right 道）与
  行背景同时亮 ✓；**零资源**、**成对**（`end` 必须清空 ✗ 否则高亮永远留着）；反向验证 ✓（37/37 ✓）。
- **Infoview 字号** ✓（`b210974`）：`0.78em` ⇒ 1em、去掉**双重压暗**的 opacity、行高 1.5、
  新设置 `sokonanoda.infoview.fontScale`；CSS 契约判据 + 反向验证 ✓。
- **编译进度 P1/P2/P3主机侧/P6** ✓（`deeaf8b`+`a26bd33`+`b182972`）：LSP **成对**报 `$/progress`
  （令牌按 uri ✓）· 状态栏"编译中"态（P2）· Infoview **3 行**进度区（P3，判据钉"正好 3 行 /
  就地更新 / end 清干净"✓）· 节流 `sokonanoda.progress.throttleMs`（只节流 `report` ✓）。
- **B3 第二刀（路线③）** ✓：**只有隐式 binder 的常量**被应用时（`Set.univ x`）富余实参落到
  **结果类型**上、参数从富余实参的类型解出 ⇒ **G-41 收口**（判据先判红 + 反向验证 ✓；内核零
  改动 ✓）。⚠ 第一版放宽到"任意富余实参"**当场打红 prelude** ⇒ 收紧到 `explicit_layers == 0`
  （那一档**没有**旧写法歧义 ✓）；放宽要先能判定"旧写法是否良型"（Lean 用元变量 ✗）。
- ⚠ **本轮的工作方式教训（耗时账 / CI 被顶掉 / clippy 被本地门禁抓到）** ⇒ 见下面「未决项」✓

## 未决项

- ⚠ **工作方式四条纪律**（用户 2026-09-26 拍板 ⇒ **已写进 `AGENTS.md` 新节** ✓）：
  ① 后台**落盘 + `timeout` + 轮询**（禁止阻塞干等 ✗）② **同一个编译只跑一次**
  ③ 慢要**量出来**（`--timings`）④ 每条交付**带耗时账**、且**推送后确认那一轮跑绿** ✓。
- **耗时账**（纪律④的实测数字）：`cargo test -p sokonanoda-front --lib` ⇒ **115s** 墙钟 /
  **1.58s** 执行（**98.6% 是编译+链接** ✗）· `-p sokonanoda-lsp --lib` ⇒ **127s** / **37.3s** ·
  `-p sokonanoda-cli` 全量 ⇒ **>25 分钟**（被我自己 `timeout 1500` 砍掉；**12 套件 ok / 0 failed**，
  余下的交给集成那一轮）✗ · `ci-local --fast` ⇒ **197s** ✓ · `clippy --workspace` ⇒ ~4s ✓ ·
  `--no-run --timings` ⇒ 38.9s 而 **Fresh 109 / Dirty 1** ⇒ 慢在**链接**不在编译 ✗
  （⇒ "少跑测试"省不出来，只能"少触发重编译" ✓）。
- ⚠ **CI 被自己顶掉**：编译进度那批连续 push ⇒ `8070557`/`208ce07`/`b9d85f6`/`42888a0`
  **四轮全 cancelled** ✗ ⇒ 最后一次成功停在 `b210974`（**早于**那批 ✗）⇒ 处置：**推完停下等绿** ✓。
- ⚠ **clippy 是本地门禁抓到的**（不是 CI ✓）：`args.len() > 0` ⇒ `clippy::len_zero`
  （本仓 `-D warnings`）⇒ 修完重跑 `ci-local --fast` **30 ✅ / 0 ❌**（197s ✓）才推 ✓。
- **B 组**：B3 两刀已落（G-40/G-41 收口 ✓）· **G-42 本轮试了 5 次、全撤** ✗（如实记）：
  **第一半（`resolve_known` 查规范名）是好的** ✓（复现件与判据都从红转绿 ✓），**卡在第二半**
  —— 钩子真被触发后 `#check some Nat` 变 `Option Type 0` ✗，而"实参是否逐位贴合层域"这条
  入口**一直返回 false**，两个原因都很隐蔽：① **`Expr` 的 `PartialEq` 含 `span`** ✗
  （两边只差 offset 就判不等）② **`Type` 与 `Sort 1` 是同一个东西但 pp 不同** ✗
  （`{A : Type 0}` 渲染 `Type`、源码 `Nat : Type` 的源级类型渲染 `Sort 1` ⇒ 文本比假阴）。
  ⇒ 正解要**同时**"过 `parse_expr_text` 归一 + 比归一后的 pp 文本"；按纪律**停手撤回**
  （front **744/0 恢复全绿** ✓），实测入参（`head=some k=1 layers=2 args=1 explicit_arity=1`）
  与另两条发现（构造子注释引用的守卫 `layers.len() < k + args.len()` **代码里没有** ✗）都在台账 ✓。
  · S1 地图的模式 B/D 未动 → **B2 仍被 G-42 挡** ✗。
- **待办（不急 ✓）**：`scripts/ci-yml-lint.py` 缺 `pyyaml` 时 `exit=2` 与真红同形 ⇒ 将来分档为
  「环境异常」✓ · `scripts/check-site.py` 未接进 CI（要先解决"CI 里怎么拿最新 tag" ✓）。

## 硬事实（接手先读这 5 条 ✓）

- **硬规则**：内核可改，唯一红线是**判定正确性不变** ✓（`REQUIREMENTS.md` §2 ✓ · `docs/architecture.md` §6/§8 ✓）
- **批次制**：一个批次**只 push 一次** ✓（`AGENTS.md` §CI 节奏 ✓）
- **等待流水线不用轮询** ✓：`gh run watch <id> --exit-status` ✓（`docs/CI-FAILURES.md` "轮询不是工作" ✓）
- **性能门禁只跟同宿主的记录比** ✓（`AGENTS.md` §性能回归门禁 ✓）
- **文档预算** ✓：`python3 scripts/docs-lint.py`（活文档 ≤3.0 MB · 入口 ≤800 行 ·
  新设计 ≤150 行 / 既有冻结 · 归档必须被索引点名 ✓ —— `docs/design/docs-diet.md` ✓）
- **本文件受 lint 约束** ✓：`python3 scripts/status-lint.py` ✓（≤200 行 · 禁词 0 · 每段 ≤30 ✓）

---

## 第 477 轮（2026-09-26）：CI 实验结论 —— `e2e-ledger` 为何被静默跳过 ✓

- **变了什么** ✓：`e2e-ledger` 的 `if` 加 **`!cancelled()`** ✓（`.github/workflows/ci.yml` ✓）。
- **机制** ✓（**实验证实** ✓）：**skip 沿"依赖链"传播** ✓ —— `gates-fast` 被 skip ✗ ⇒
  `e2e` 靠**它自己的** `!cancelled()` 跑起来了 ✓，**但那个 skip 仍污染链条** ✗ ⇒
  `e2e-ledger` 的 `if` 里**没有状态函数** ⇒ **被静默跳过** ✗
  （整轮还报 `success` ✗）。
- **实验（唯一变量 ✓）**：`6e19f23` 只改 `editor/**` + `if` 无状态函数 ⇒ **skipped** ✗；
  `34a2814` 同样只改 `editor/**` + `if` 带 `!cancelled()` ⇒ **success** ✓✓。
- **纪律** ✓：**凡 `needs` 里可能有 skipped 的 job，`if` 都要带状态函数** ✓
  （否则它会静默消失 ✗）。

## 第 476 轮（2026-09-26）：**文档瘦身 + 判据守护** ✓（用户要求：「文档太重了」✓）

- **变了什么** ✓（三类分治，全部落成机制 ✓）：
  * **垃圾** ✓：**48 个 / 1.54 MB 删净**（`.tmpdir` 6 个 + 产物残留 `*.tmp-<pid>` 26 个
    + 编辑器残留）✓；`.gitignore` 补 `.*.tmp` / `*.tmp-*` / `*.orig` / `*.rej` / `*~` ✓。
  * **活规范瘦身** ✓：`REQUIREMENTS.md` **3112 → 214 行** ✓（§9 历史**逐字**进
    `docs/archive/REQUIREMENTS-ARCHIVE.md` ✓）· `e2-plan` **3105 → 379 行** ✓
    （`plan.py check/list/bumps` 仍全绿 ✓）· E1 计划 **4375 → 240 行** ✓ ·
    `TESTING` / `HANDOVER` / `imports-and-projects` / `course-lean-style` /
    `duplication-audit` 各**只留契约** ✓。
  * **过程记录归档** ✓（`docs/archive/` + 索引 ✓，**归档≠销毁** ✓）：`STATUS-ARCHIVE`
    （7392 → 1666 行）· `CI-FAILURES`（1823 → 446 行）· e2e 台账（253 → 50 条 + 140 日志）·
    调研笔记 20 篇 · 站点重构 17 篇 · 缺口工作单 13 篇 · 自述已废弃设计 5 篇 ✓。
  * **判据** ✓：`scripts/docs-lint.py` **六条** ✓ + `--selftest` ✓，接进
    `scripts/soko gate` / `ci-local.sh` / CI 的**独立 `docs-lint` job** ✓（不设 `if:` ⇒ 永远跑 ✓）。
- **现在的状态** ✓：`docs-lint` **绿** ✓（活文档 2.90 MB ≤ 3.0 · 归档 2.15 MB ≤ 2.5 ·
  垃圾 0 · 归档索引齐 ✓）· `status-lint` / `plan.py check` / `ci-yml-lint` /
  `e2e-merge --check` 全绿 ✓ · 站点 `check-site.py` **9/9** ✓。
- **未决** ✓：**是否现在 push**（见下）。
- ⚠ **两条顺带查出的既有缺陷** ✓（都不是本轮造成的 ✗）：
  ① `site/data/site.json` 停在 **0.66.0** ✗（最新 tag 是 v0.72.0）—— 真因是
  **`scripts/check-site.py` 既不在 CI 也不在 `ci-local.sh`** ✗ ⇒ 从 0.66.0 起没人拦；
  已按它给的补救命令**重生成** ✓（diff 只有 3 行 ✓，点检回到 9/9 ✓）。
  ② `crates/front/src/compile/check/mod.rsY3mLag`（49 KB）是**被 git 跟踪的畸形残留** ✗
  （`mod.rs` 的旧快照、零引用、不参与编译）⇒ **已删** ✓。
- ⚠ **并发写者** ✗（本轮**唯一不能自行收口**的点）：`crates/front/src/display.rs` 在
  10:59 被**另一个会话**改了 139 行（注释写着「2026-09-26 用户报告第 1 条」= A1 箭头折叠）✗
  ⇒ 它新增的两个测试（`arrows_fold_to_the_unicode_arrow` / `set_literals_fold_back_to_braces`）
  当前**红** ✗，但**不在 HEAD 里** ✓ ⇒ 与本轮**零关系**（本轮**零 Rust 改动** ✓）⇒ 本轮**不 push** ✓。

## 第 475 轮（2026-09-26）：🎉 **e2e 通过 —— `28 passed / 0 failed`** ✓✓（T-U12 面 #3 闭环 ✓）

- **变了什么** ✓：改用 `axiom` 后 **`e2e exit=0`** ✓ · **`28 passed / 0 failed`** ✓✓
  ⇒ **T-U12 面 #3 的两层都闭环** ✓：**front 侧**（`hover_text_is_folded_like_the_lsp_does` ✓）
  + **e2e 侧**（**真宿主 + 真 hover** ✓）。
- **状态** ✓：`docs/e2e/ledger.jsonl` 已追加 ✓ · `docs/e2e/latest.json` 已更新 ✓。
- ⚠ **五次尝试、四个夹具 bug** ✗ —— **每一个都是从失败文本里读出来的** ✓：
  ① **`waitFor` gate 在断言上** ✗（**只报超时** ✗）⇒ 改成"等非空" ✓；
  ② **`A`/`B` 不在作用域** ✗；③ **`Set` 未声明** ✗ + **元数不匹配** ✗（3 参 vs 2 参 ✗）；
  ④ **体的定义相等** ✗（`True` vs `Set.subset A B` ✗）⇒ **改 `axiom`** ✓。

## 第 474 轮（2026-09-26）：失败文本**前进了** ✓ —— `def usesWeird` ⇒ `def Weird` ✓

- **变了什么** ✓：夹具修好 `Set` + 元数后仍红 ✗，而**失败文本前进了一格** ✓：
  现在是 **`def Weird`** ✓ ⇒ **卡在 `Weird` 的体** ✗ —— `def … := True` 要求 `True` 与
  `Set.subset A B` **定义相等** ✗，而 elaborate 不展开它 ✗ ⇒ **改用 `axiom`** ✓。
- ⚠ **"失败文本前进"是好信号** ✓：**它说明前面的错都被修掉了** ✓ ——
  这一面的调试**每一步都有可读的下一步** ✓。
