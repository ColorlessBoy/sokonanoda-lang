# 当前快照（2026-09-26）

- **进度**：**64/66**（E2 50/50 ✓ + **批次 N** 记法×隐式参数×产品交互 14/16 ✓）
- **已发布**：**`sokonanoda v0.72.0`** ✓（`gh release list` 显示 **Latest** ✓ · 2026-09-25T23:25:07Z ✓）
- **A 组（A0–A5）全部落地** ✓：显示层混合形态（`ty_text` ASCII `->` **227 → 0**）· `{a}` 折回 ·
  `{a}` 可跳转（根因：开放练习的签名**一条 hover 行都没有**）· prelude 可跳转（F12 落到前奏源文件
  的真 span）· `flawed_equalities_refuted` 去点名（标记 5 → 3）· 洞的期望类型**两半分开钉**
  （显示副本必折 / 真相字段一个字节都不许折）。A0 守卫基线 **68 → 59**（结论：**59 是地板**）。
- **真宿主 e2e**：A1/A2/A3/A4 四条 ✓（全量 **32/32**），反向验证撤修复 ⇒ **1 passed / 3 failed** ✓。
- **B3 第一刀** ✓（`04d863a`）：**裸常量**的隐式插入（G-40 收口）—— `∅` = 裸 `Set.empty` 以前停在
  那个 Pi 上、`rfl` 判不出来 ✗ ⇒ 走 `solve_prefix` 路线 ② 从期望类型补 ✓；判据 + 反向验证 ✓。
  新登记 **G-41**（记法路径 + 隐式 binder ⇒ 真库 `328/0` 掉到 `226 checked · 14 判负` ✗）与
  **G-42**（查签名表用裸名 ⇒ `namespace` 里裸名引用不触发；显然修法会回归 `#check some Nat` ✗）。
  **⇒ B2 仍被 G-41 挡住** ✗。
- **判据**：课程门禁 **36/328/99/0**（逐项不变 ✓）· front **743** / LSP **163** ✓ ·
  `notation-lint` 84 文件 ✓ · `docs-lint` ✓ · `gap.py check` ✓ · `ci-local --fast` 全绿 ✓。
- **未做（新顺序）**：**Infoview 字号 → 编译进度展示 P1–P6** → B2（被 G-41 挡）。

## 未决项

- **无阻塞** ✓（E2 50/50 ✓；A 组全部落地 ✓；推送钩子**全绿** ✓ —— 文档预算上限已按
  用户要求提到 **10 MB** ✓，「绕钩子」那条例外**已销账** ✓）。
- ⚠ **反向验证的手法教训（2026-09-26，如实记 ✓）**：B3-① 的反向验证**翻了三次**才跑通 ——
  前两次是**改法本身编译不过**（`return Ok(bare)` 早退 ⇒ `unused variable` 撞 `-D warnings`；
  `let bare = bare;` 同因），第三次换成"**反转闸门**"（`k == 0` ⇒ `k > 0`）才让判据如期
  **判红** ✓。**结论：判据本身是真的**（能咬住 ✓），但**手法是试出来的** ✗ ——
  下次改这类"闸门式"代码**一步到位用反转闸门**，别先试早退/占位 ✓。
- **B 组进度**：**B3 只落了第一刀** ✓（裸常量；G-40 收口）· **B2 仍被 G-41 挡住** ✗ ·
  G-42（`namespace` 裸名 + 构造子歧义）**已登记未修** ✗ —— 三条都在
  `docs/gaps/ledger.jsonl`（各带可执行复现 ✓，`python3 scripts/gap.py check` 绿 ✓）。
- **待办（不急 ✓）**：`scripts/ci-yml-lint.py` 缺 `pyyaml` 时 **`exit=2`** ✗ 与真红同形 ⇒
  将来分档为"**环境异常**" ✓。
- **遗留（文档线，不急 ✓）**：`scripts/check-site.py` **仍未接进 CI/gate** ✗ —— 接之前
  要先解决"CI 里怎么拿最新 tag"（`actions/checkout` 默认 `fetch-depth: 1` **不取 tag** ✗）；
  78 份**既有**设计文档**未逐份重写** ✗（改用**棘轮**：`docs/docs-budget.json` **只许减不许增** ✓）。

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
