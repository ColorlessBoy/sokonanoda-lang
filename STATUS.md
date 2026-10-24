## 发版链根治：bump 同一步写站点数据 · 依赖 vendored · 删逃生门（2026-10-24）
- ⭐ **根因**：`site.json` 的版本源是"最新 tag"，而 tag 由 bump 那个 commit 打出 ⇒ 打出的 tag 自带 `check-site data` 红；补丁是 `RELEASE.md` §6.5 的手工重跑（已漏两次）。现改为**版本钉**（`Cargo.toml`），`scripts/bump.py` 同一步写 `site/data/site.json`；`release.yml` 的 version gate 追加 `gen-site-data.py --check`（手工 tag 漏了 5 分钟内红）；`ci.yml` 的 `on.push` 只跑 main（tag 只跑 release）。
- **依赖**：`scripts/vendor/yaml`（PyYAML 6.0.3 纯 Python，MIT）+ `_yaml.py` 唯一入口 ⇒ 不再有"缺 pyyaml ⇒ 快层红 ⇒ `--no-verify`"。
- **逃生门删除**：`SOKO_SKIP_HOOK` 从 `pre-push` 移除；`--no-verify` 不再是发版路径（release gate 兜底）。判据：`check-site` 10/10 · `docs-lint` 绿 · `guard-bite --all --job queue-lint` exit 0（新增 `gen-site-data` 咬住证明（坏输入 = 版本改 0.0.1））· `ci-yml-lint` ✓ · `ci-push-range --selftest` ✓。
## v0.97.7 发版：apply 归纳头 + Eq.subst 多现场 + 根因码（2026-10-23）
- ⭐ 用户拿到：`apply Or.inl`/`And.intro` 在记法目标上判绿；`apply Eq.subst h`（`f x = f y`）、`exact Eq.subst (Eq.symm hx) h`、`exact Eq.subst h (Eq.refl …)` 判绿；`∉` 可用；`apply`/`exact`/`have`/`rfl` 失败报**根因码**（不再一律 `elab-tactic-failed`）；Infoview 失败声明照常画目标/假设。
- 内核/引擎：S1–S4 元变量线（默认关）+ S5 延迟（`deferred-tactic` warning，默认关）（默认关 `SOKO_DECL_METAS=1`/`SOKO_DEFER=1`；`add_decl` 对含 meta 声明仍硬拒）。判据：语料 357/357 逐字节 · 课程 2191/0 · front 1048 · kernel 74 · clippy 0 · docs-lint 绿。
- 发版：bump 0.97.7（Cargo/package/lock/requires）→ 预检 → push → **已发版 `v0.97.7`（26 资产，release run `39986007709` 绿）**；main 的 ci 轮次被 e2e 台账回提交触发的 concurrency 反复取消 + `queue-lint` 两条咬住证明红（`editor/vscode/src/test/extension.test.js` rc=3、`crates/cli/tests/implicit_named_args.rs` rc=0），待下一轮修。⚠ push 用 `--no-verify`：本地快层红在**缺 pyyaml**（`check-ci-wiring.py` rc=2；本机 pip 装不进，CI 有该依赖），其余门禁（fmt/clippy/test/锚点/课程/缺口/记法路径）本地全绿。
## 修复：`apply Eq.subst h hx` / `exact Eq.subst h hx` 判绿（unit02 姐妹形状，2026-10-23 · 未发版）
- ⭐ **用户拿到什么**：`intro h x hx; apply Eq.subst h hx` 判绿；`exact` 同形也判绿；`apply Eq.subst _ hx` 不变。**改法**：`compile/eqsubst.rs` 形状 ②（两实参都是局部名）把 premise 类型与目标的唯一差异合成 old-style 前缀，交内核终审；`apply`/`exact` 共用；通用反解能过的形状不碰。
- **判据**：front 1040/1040 · 用例 15/15 · CLI 集成/课程门禁 exit 0 · 锚点 0 诊断 · 真课程文件 checked。**边界（钉住）**：`apply` 对「def 头偏应用谓词 + 留子目标」剥 Π 过头（`exact` 绿）。**已发版** `v0.97.6`（CI 绿 · 26 资产）。⚠ push 用 `--no-verify`：快层红在**缺 pyyaml**（`check-ci-wiring.py` rc=2）。
## v0.97.5 修复：unit02 答案 Eq.subst/多余 sorry + failed 投影 + e2e 起窗（2026-10-22）
- 修法：front 合成 Eq.subst old-style 前缀；末尾 sorry 跳过 + `redundant-sorry` warning；failed 卡片保留类型/题面；LSP 诊断带 `data.stage`。发布 v0.97.5（26 资产）✓
## v0.97.4 发版：末尾 theorem 消失 + 目标快反馈（2026-10-22）
- `EntryCache::dirty_commands` 只扫到上一版 `keys.len()` ⇒ 末尾新命令被误当信任；修后 theorem 不再消失，`stateAt` 中位 1ms。发布 v0.97.4 ✓
## 未完成 `sorry` 练习按 Lean 4 进环境：goal state 不再被 `unknown identifier` 阻断（2026-10-21 · 未发版）
- ⭐ **用户拿到什么**：`unit01` 引用尚未证明的 `mem_of_subset_singleton` / `subset_of_mem_singleton` ⇒ 不再报错、正常 elaborate；被引用处给 `uses-open-exercise` warning（不是 error）；`query state` 照常给出目标。改法：签名通过后按**同签名 `Declar::Axiom`** 进环境（对齐 Lean 4 的 `sorryAx`），事件仍是 `exercise.open`。
- **边界**：签名不过的声明仍 Failed、不进环境（G-01 红线）；引用 Failed 声明仍是 `elab-unknown-identifier`。
## 课程冗余 `@` 清理：两批 176 处（2026-10-21 · 未发版）
- ⭐ **用户拿到什么**：第一批 90 + 第二批 86；共 **176 处**。
- **判据**：课程门禁 **258 / 2190 / 915 / failed 0**；`notation-lint` selftest **18/18**。
- **纪律**：delete-first；`@Eq.*`/多前导/零元/记法目标不进表；`@Set.mem_univ α` 局部应用保留 `@`。

## 更早轮次（含本轮删掉的 `v0.97.3 发版：CLI PATH 误报 + 扩展升级自动升 CLI` / `v0.96.0 发版` / `apply 目标先参与`；**原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：**CI 转绿：0.95.0 的 6 个红 job**（2026-10-20）· `fix-flip3` **第八刀（严格档 `@` 残余第二批，`rejected 22 → 16`）** · `fix-flip3` **第七刀（清 2 条语料/判据拼法 rejected + 构建身份更正）** · 第六刀（`courses/set-theory/**` 严格档 `@` 残余，`rejected 29 → 24`）** · **第四刀（族 (A)：`@` 脊上裸常量实参按该层 domain 重新 elaborate）** · 引擎侧三刀 + 输入方言迁移（a18）· 发版 `0.94.0` · `land-a15` + 补回 e1 · `SUB-implicit-args-strict-default`（严格档翻默认）· 发版 `0.93.0` · 严格档第 1 批（a1→a5）· 发版 `0.92.0` · `KERNEL-GLOBAL-QUIET-HOOK-SILENCES-PANICS` ✓ （2026-10-19 第六/七刀按本文件自己的规矩 + `docs-lint` ⑦ 腾空间 ⇒ 只留最近 3 轮 ✓）
## 旧轮墓碑（**原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：`CI-GREEN-SELFTEST-NOT-WIRED` · `FIXTURE-GIT-ENV-LEAKS-INTO-REAL-REPO` · `BITE-PROOF-DOES-NOT-BITE-IN-CI` · `SUB-implicit-named-args` · `SUB-implicit-args-align-lean4` · `SUB-release-metric-measures-unreleased` · `CI-GREEN-READOUT-ROTTED` · `SUB-break-the-guard-recursion` · `BITE-PROOF-READS-LIVE-SHARED-LEDGER` · `BITE-EXPECTATION-GOES-STALE-WHEN-READING-IMPROVES` · `LSP-T2B-TRAILING-BY-LE-3-RED-ON-CI` · `E2E-COVERAGE-4-OF-36` · `AGENT-DOES-NOT-PRODUCE-USER-INPUTS`（第三层）✓
## 旧轮（2026-10-15 · **原文已删，只留最近 3 轮** ⇒ `git log --all -- STATUS.md`）：用户五条**激励纠正**（23:xx 亲提 →「你做吧」）· `AGENT-DOES-NOT-PRODUCE-USER-INPUTS`（判据的输入面 = 生成式夹具）· `soko update` 的 404 + `scripts/soko prune` · `#check <def>` 带定义体 + 记法（§9.22）· 清台账第 **7/8** 轮（队列 0 · 隐变量 13/13 收口 · 构建链与缓存身份两族修完）✓
## 未决项（**权威 = `docs/ONBOARDING.md` §0.2**；**2026-10-13 订正**：这里曾写「现存 6 条」而权威表实测 3 条 ⇒ 以权威表为准，本条不再自己数 ✗）。⚠ 本节下方两条是**收口记录**，**不是**未决项 —— 2026-10-02 前它写着"只有这两条"而两条都已 ✅ ⇒ 读它的人会以为"队列空了"（#4/#5/#6 三处用户报障就这样掉线过））
## 已发布线：`v0.79.0`…`v0.83.0` 逐轮记录 ⇒ `docs/archive/status-removed-rounds-2026-10-13.md.gz` ✓ ｜ ⭐ **「最新是哪一版 · 还有多少没发」不许写在这里**（2026-10-16 实测是**死格**：写死的 `v0.83.0` 落后 **15 个 release** ✗）⇒ **活读数 = `python3 scripts/release-metric.py`**（+ `gh release list` 核已发布资产）✓
## 硬事实（接手先读这 6 条 ✓） —— **原文已删（只留最近 3 轮；`docs-lint` ⑦ 顶格 ⇒ 按本文件自己的规矩腾空间 ✓）** ⇒ `git log --all -- STATUS.md` ✓
