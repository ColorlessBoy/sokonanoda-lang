# 交接单 —— **v0.75.0 已收口**（下一会话从 v0.76.0 起）

> 写于 2026-09-27 深夜（第三个会话收尾）。**本文件只是起点快照**，不取代
> `docs/PLAN-0.74-0.79.md`（**唯一真相**：顶部「⚡ 执行索引」，环节与判据以当前发版点章节为准）
> 或 `docs/gaps/ledger.jsonl` ✓。

## 1. v0.75.0「跳转与高亮」= **4/4 全部 ✅ 并已发布**

| 环节 | 状态 | 证据 / commit |
|---|---|---|
| E05 记法目标名 F12 落点（Bug A） | ✅ | `3c6ac01`（`range` 用**定义那一处** span；真课程库 `Set.powerset` L125→**L81** · `Set.compl` L126→**L79**） |
| E06 G-37 判据升级 | ✅ | `3c6ac01`（复现件 **exit 0 → exit 1**；单测钉「落点 == `def` 那一行、不许自跳」；反向验证逐字一致） |
| E07 跨模块目标（Bug B）**定案** | ✅ | `eee6cc2`（落点与编辑器 F12 一致 ⇒ 闭包外**诚实 null**、不扩到全仓；高亮**实测已一致** ⇒ 两条防漂移判据；**G-54 wo-filed**） |
| E08 `documentHighlight` **定案：本轮不做** | ✅ | `eee6cc2`（三种备选语义不同 ⇒ 设计决定；**G-55 open** + 「断言当前行为」的防漂移判据） |

**发版**：`scripts/bump.py 0.75.0` ⇒ `79bbf4c` ⇒ auto-tag `v0.75.0` ⇒ release **2026-09-27T19:38:39Z** ✓
（`gh release list` 显示 **Latest** ✓）。

## 2. 实测验收数字（**接手先认这几个数**）

| 判据 | 结果 |
|---|---|
| 课程门禁 | **36 目标 · 327 checked · 99 open · 0 判负** ✓ |
| 本地全量 | `scripts/ci-local.sh` 全绿 ✓（fmt/clippy/workspace/gates/两条守卫自检/editor stub） |
| CI（发版那一推） | run **`36343726623`** ⇒ `python3 scripts/ci-green.py --run 36343726623` **exit 0**（重活 **10/10**、failure 0、skipped 0）✓ |
| CI（往轮） | `36342280193`（e2e 修后：editor + 三条 e2e 腿全绿 ✓）· `36340364435`（**红过**：macos e2e，已修） |
| e2e | 三平台 **35/35** ✓（`docs/e2e/ledger.jsonl`） |

## 3. ⚠ 本轮踩到并修掉的真红（**别再重复**）

- **run `36340364435` 的 `e2e (macos-latest · 1.138.0)`**：E30 那条 e2e **「等 A 断 B」** ✗✓ ——
  `waitFor` 等的是 `infoview.lastProject()`，最后一条断言读的却是 `extensionApi.project.answer`
  （**项目树**那份，另一条路填）⇒ 慢 runner 上 `undefined` ⇒ `TypeError` ✓。
  **按值守口径 rerun 一次仍红** ⇒ 取 artifact 定位真因并修（`8842f23`）✓。
  **教训**：① 把"后面要读的每一个异步状态"都列进 `waitFor` ✓；② **快机器绿、慢机器红 ⇒ 先怀疑判据，别叫 flake** ✗；
  ③ **rerun 只能证伪 flake、不能消红** ✓；④ 沙箱里取 CI 日志要 `XDG_CACHE_HOME=/tmp/ghcache` ✓
  （`gh` 要写 `~/.cache/gh` ✗）。已进 `docs/CI-FAILURES.md`（**压缩进 465 行冻结预算内，没抬上限** ✓）。

## 4. 剩余待办

1. **G-54（wo-filed）**：记法目标**定义在闭包外**时跳不到（真课程库 3 条：`Set.image`/`Set.preimage`/`Set.prod`）。
   正路：让 `project_definition` 在闭包查不到时去**模块根**下按名字补查（会把那份库拉进编译 ⇒
   要「按需编译一个模块」的能力与性能账）；**不许用文本扫描绕过**（硬规则 4）✗。
2. **G-55（open）**：`documentHighlight` 对目标名（E08 定案本轮不做）；三条备选写在台账里，
   倾向 ①（只高亮声明行那一个名字），前提是先把「目标名是使用点」在 front 定下来。
3. **G-53（open）**：Infoview 的**记法符号**点不动（wire 的 runs 无位置 ⇒ 要改 front+LSP 协议）。
4. **G-48 / G-49（open）**：`≈` 零元糖补不出论域；类型错误报裸 de Bruijn 编号。
5. **v0.76.0「prelude 显式化 + 两跳」（E09–E11）** —— **先读 PLAN 那一节再动手** ✗（别凭记忆）；
   既有设计先读 `docs/design/prelude-l1-proposal.md`（367 行）。

## 5. 交接时的仓库状态

- `HEAD` = 发布收尾提交（PLAN/STATUS 的发版点行）+ `79bbf4c`（bump）；**工作树干净** ✓；
- ⚠ **版本纪律**：bump 之后 pre-push 的课程门禁会因「判卷二进制与仓库版本不一致」exit 2 ✗ ⇒
  **先 `cargo build -p sokonanoda-cli --bin sokonanoda`**，再 `SOKONANODA_BIN=$PWD/target/debug/sokonanoda git push` ✓
  （不带它走 debug 全量：课程门禁实测 **626s** ✗）；
- ⚠ **CI 的 `e2e ledger` job 会往 main 回写** ⇒ 每次推都非 fast-forward ✗ ⇒
  `git fetch origin && git rebase origin/main` 再推（**禁止 force push** ✗）；台账冲突：
  `ledger.jsonl` **取并集**（按 `date` 排序去重）、`latest.json` **取 theirs** ✓。
