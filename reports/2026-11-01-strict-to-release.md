# 全局计划：**strict 彻底推上线**（2026-11-01 定，主会话只做拆解/审/验收，开发交子代理）

> 上游：`reports/2026-10-29-strict-replan.md` §4（阶段 P0…P4 与 P2 的 13 刀）、
> `reports/2026-11-01-p212-s4b-census.md`（回落清单）、`reports/2026-11-01-craft-notes.md`（怎么干）。
> 本文回答一个上一棒没答的问题：**「上线」到底要交什么** —— 不只是代码判据，还有发版面。

## 0. 目标态（用户可见，这就是"成功"的定义）

| # | 上线后用户看到什么 | 谁定 |
|---|---|---|
| 1 | **strict 是默认档**（打开编辑器/跑 CLI 不用设任何环境变量），`--dialect loose` 仍可显式选 | **用户拍板**（默认档由用户定，`REQUIREMENTS §9`） |
| 2 | strict 档解不出时给**人话**：点名是哪个 binder、给可照抄的出路（`@g <A> 0`），**永不**出现 `\0soko_m…` / `?1073741823` | 设计稿 Q3（S2 已实现，S3 接线） |
| 3 | strict **不慢于** loose（计数判据，不是墙钟） | replan §3 |
| 4 | `git grep SOKO_STRICT_KERNEL_APP` = **0**；`audit-strict-paths` strict 专属项 = **0** | P3 判据 |
| 5 | 发版闭环：bump → CI 绿 → auto-tag → release 资产齐 → 站点写的是**已发布版本** | `docs/RELEASE.md` §3/§5 |

## 1. 从现在到上线的整条链（**六批**，每批一个可 push 的批次边界）

| 批 | 内容 | 判据（可执行） | 谁做 |
|---|---|---|---|
| **1（在跑）** | S3 并入（`SOKO_STRICT_NO_FALLBACK` + 走 S2 报错出口）+ S4b 根因报告 | 开关关 714 行 0 差异 · c1 报错可读且 span 非 0:0 · `Fallback` 四例外与 S2 对齐 · S4b 有 Q1–Q5 | 子代理写，主会话审 |
| 2 | **S4b 实现**：`Set.Le`（`A ≼ B`）一族，`old_ok` **185→64** | 只降不升 · rc/诊断不变 · 动内核才跑两对语料 · 复现件修前红 | 子代理，主会话定分叉 |
| 3 | **S4c**（余 9 次：`unsolved` 4 / `argmismatch` 2+2 / `expected` 2）+ **S5**（CLI + LSP hover 判据） | 四种例外逐条有测试；c1/c5/R1 文字测试 | 子代理 |
| 4 | **S6 翻默认**（`effective = dialect.strict()`、删 `env_on`、重写 `kernel_app_keys.rs`）+ **P2-13**（砍共享代码里的 strict 分支） | P2 总判据四件（lib 17/17 · 整门课 strict 全过 · 计数 ≤ loose · loose 逐字节） | 子代理写，主会话跑两对全语料 |
| 5 | **P3 清理** + **发版面**（见 §2） | `audit-strict-paths` 0 · grep 开关 0 · docs-lint 绿 · skills/编辑器/文档同轮同步 | 主会话 + 子代理 |
| 6 | **P4 发版**（bump → push main → CI → auto-tag → release 核对） | `release-preflight.sh` 全绿 + `gh release list` 资产齐 | **主会话，且 push main 前问用户** |

## 2. 发版面（P4 之前必须同轮做完的"用户可见"清单）

这一块上一棒没动过，但**不做的后果是发出去的版本和文档/编辑器不一致**（AGENTS「收尾义务」）：

1. **`REQUIREMENTS §9` 追加一条**：strict 默认档 + 两档差异（设计稿 Q3 的"有意不同"：我们的 unsolved 在应用末尾就报、
   Lean 留到声明末尾）+ `SOKO_STRICT_AT` 仍是逃生门。
2. **skills 三件**：`sokonanoda-dev`（`skills/sokonanoda-dev/SKILL.md:73` 还写着 `SOKO_STRICT_AT=1` 与"默认 Loose"）、
   `sokonanoda-teacher`（`:429` 提到 `--dialect strict`）、`sokonanoda-ci`；`.agents/skills/` 入口不动（`crates/cli/tests/dsh.rs` 守）。
3. **VS Code**：设置项 `sokonanoda.dialect.strictImplicitArgs` 已存在（`editor/vscode/package.json:307`）⇒ 要看它是否还描述
   "实验/需要环境变量"；README / CHANGELOG / package.json 同轮改；设置项文案算用户可见改动。
4. **接手文档**：`AGENTS.md` 硬规则 7（strict = 绑定策略）要补"**默认已开**"；`docs/ONBOARDING.md` 的开档说明；
   `docs/architecture.md` §6 内核台账补 S0b/S4a 两条（陈旧 `Value` 的第三、四个补丁点）。
5. **站点**：`scripts/gen-site-data.py` 自动出数据（**不手写版本号**）；`python3 scripts/check-site.py` 10 项 +
   `--browser` 两项。
6. **台账**：`docs/e2e/ledger.jsonl` 按批次记一条；`docs/perf/ledger.jsonl` 若内核有性能改动则记。
7. **发版机制**：`python3 scripts/bump.py <x.y.z>`（写 Cargo.toml + package.json + Cargo.lock + 清单 requires + site 数据）
   → `scripts/release-preflight.sh` → push main（**问用户**）→ `ci.yml` auto-tag → release → `gh release list` 核对。

## 3. 用户决策点（只有三个，其余我全包）

| # | 决策 | 现在需要吗 |
|---|---|---|
| **D1** | **默认档**：strict 直接默认开，还是先给一个"设置项默认开、环境变量仍可关"的过渡版本 | 批 4 之前 |
| **D2** | **S4b 分叉**（若根因是内核 P1 限制）：(a) 内核补一阶近似 / (b) 课程改点名 `Set.Le α β A B` / (c) 记为已知差异进 §9.39 | 批 1 的 Q2 出来之后**立即**要 |
| **D3** | **push main / 发版**（批次收尾的推送不必问；merge 到 main 与打 tag 必须问） | 批 6 |

## 4. 分工与"100% 成功"的机制

- **主会话只做四件事**：写实施单（每刀的"怎么做 + 判据 + 边界"）、审子代理产出、跑重判据（两对全语料 / 全课程普查 /
  发版预检）、发版与对用户的汇报。**不亲自写判定代码**（用户 2026-11-01 明确）。
- **子代理只做**：单一刀的实现或只读调研；prompt 里必须带硬纪律摘要 + 可执行判据 + "不许动"边界（见 craft-notes §1–8）。
- **每刀两笔提交**（代码 + `docs(strict): …已并入（sha）…读数`），计划文档永不落后（craft-notes §1）。
- **失败的处理**：中间失败允许，但**必须**归因到"拆解/理解"并**当场改拆法**（用户 2026-11-01：「中间的失败都是任务没拆解好、理解好」），
  不许把它记成"问题本身很难"。判据：同一步连红 3 次 ⇒ 换拆法（拆小、换层、或先补只读调研）而不是重试。
