# 交接 prompt（复制给接手的 code agent）

> 用法：把下面 ``` 之间的整段复制给下一个 code agent 即可 ✓。
> 它是**自包含**的（对方可以从零开始 ✓），并指向仓库里已有的三份权威文档 ✓。

```
你是 sokonanoda-lang 的接手 code agent。请严格按下面的顺序做。

## 第一步（别跳）
1. cd 到仓库根；
2. 读 AGENTS.md（项目入口 + 硬规则速记）；
3. 读 docs/E2-HANDOVER.md（一页交接书：状态 / 开工方式 / 硬规则 / 陷阱 / 验收命令）；
4. 读 docs/design/e2-plan.md（当前计划正文：5 阶段 38 环节，每阶段 = 一个可发布版本）。

## 你的任务
按 E2 计划逐条推进，用这三条命令取活与记进度：
    python3 scripts/plan.py next      # 取当前环节（含完整规格）
    python3 scripts/plan.py list      # 看进度（现在应为 19/38）
    python3 scripts/plan.py done <ID> # 做完勾掉
**当前状态（2026-09-25）**：**19/38**。阶段 A 已发 0.66.0 ✓；阶段 B（R-4 命令名 +
R-3 产物目录）**代码全完**（T-B1..T-B6 ✓）、版本已 bump **0.67.0**，**但 release 未走完** ✗；
阶段 C 已按实测**重新定位并收口**（T-C1..T-C6 ✓、T-C4/T-C5 按刹车、**不发空的 0.68.0**）；
**下一步是 T-D1**（阶段 D 第一刀：纯重构抽出 `check_then_add_one`，执行方案与判据都已写死在
`e2e-plan.md` 的 T-D1 下）。

**⚡ 先解决 0.67.0 的 release**：`auto-tag` **只在 push 事件上运行**
（`if: github.event_name == 'push'`）⇒ `gh run rerun` 的重跑**永远不会打 tag** ✗。
要看 tag 必须往 `main` **push** 一次；绿了就会 auto-tag + dispatch release，
之后核对 `gh release list` 并把 **T-B7** 勾上 ✓。

已完成、别重做：T-A1..T-A7（R-1 的 `def` 值行、R-2 的 `=` 掉色真因 + 声明卡片目标行、
三层判据、0.66.0 闭环）；T-B1..T-B6（命令面板 15 条统一成 `Sokonanoda: <Command> (说明)`；
`<模块根>/.sokonanoda/` 产物目录 + 自忽略 + 每入口只留最新一条 + `query project` 的
只读 `artifacts` 字段；LSP 读写都落模块根）；T-C1..T-C3（`plan_module` API + 等价性判据，
**负结论**：平坦批编对课程形状不等价且慢 3× ⇒ 刹车）；T-C6（重新界定为 LSP 写路径）。
**开工前先读** `docs/E2-HANDOVER.md` §5 的陷阱清单（本机特有的 6 条：缓存内容比
marker 旧、`scripts/soko update` 下载 404 ⇒ `SOKONANODA_RELEASE_BASE` 本地镜像、
`NODE_NO_WARNINGS=1`、仓库 `target/` 写不进去 ⇒ `CARGO_TARGET_DIR` + 就地覆盖、
`git rebase`/`merge` 要 unlink 被拒 ⇒ **合 CI 台账回写用 plumbing**、
`DEVELOPER_DIR` 的正确取值）。

## 每个环节的完成定义
复现判红 -> 最小改动 -> 判据 -> e2e 转绿 -> 性能无退化 -> 文档 -> commit。
"复现判红"是硬要求：先写/跑出能证明问题存在的复现件，再动手。

## 硬规则（违反会被 CI 或用户打回）

> **两条 2026-09-24 新增的纪律**（都写进了 `AGENTS.md`，务必读那两节）：
> * **验证设计纪律**：每条用户可见改动必须先回答"**屏幕上会多/少什么？那条断言在
>   哪一层？**"；三层各司其职（front=真相 / LSP=**wire 字段存在性** / **e2e=看得见
>   的结果**）；**接缝要有 A∖B 守卫**（`python3 scripts/audit-wire-fields.py`）；
>   **「有就渲染」是反模式**。**新增 e2e 断言前先在本地跑一次那条用例**
>   （`SOKO_E2E_GREP=<用例名> npx vscode-test`）—— 别把第一次运行留给 CI。
> * **并行与 subagent 纪律**：subagent 只做可并行/只读/边界清晰的活；**并发 2–4**；
>   每个 prompt 自带"规则摘要 + 可执行判据 + 不许改什么"；产出**验证后才并入**；
>   结论**回写文档**；判定与发布**留主线**。
1. 判定红线：kernel 可以改，但同一批输入必须"接受/拒绝不变、事件计数不变、
   golden 与 --json 逐字节不变"。改内核必须跑 scripts/kernel-check.sh（五步）。
2. 提交粒度（用户明确）：一个环节一个 commit；一个阶段可以有很多 commit；
   阶段收尾才 push 一次（跑一轮 CI）。BUMP 必须闭环：bump 两处版本 ->
   push -> auto-tag -> release -> gh release list 核对。
3. 性能只升不降：有收益的改动才默认打开；前置/中性改动默认关（开关）或零调用
   （惰性 API）；每阶段复量三个基准（见 e2e 计划 §0）。
4. 课程一律写记法（a ∈ A 而不是 Set.mem α a A）；python3 scripts/notation-lint.py 会判红。
5. 改 VS Code 扩展要同一轮同步四份：editor/vscode/（README/CHANGELOG/package.json）
   + skills/ 三个技能 + AGENTS.md + docs/vscode-dev-guide.md。
6. 用户新要求先追加进 REQUIREMENTS.md §9（注明日期），再动手。

## 环境（本机特有，务必照做）
- 每条 cargo 命令都带 DEVELOPER_DIR=/Library/CommandLineTools；
- 本地构建用 CARGO_TARGET_DIR=/tmp/soko-target（仓库 target/ 里有删不掉的陈旧文件）；
- git checkout -- <file> 在本机不可用（沙箱拒绝 unlink）=> 回退请用 python 从 git
  读原文件再写回；
- git merge-tree --write-tree 冲突时退出码非 0 且输出带冲突标记的树 =>
  必须查退出码，非 0 就手工解，绝不 `| head -1`；
- bash 的 grep 在本机会静默返回空 => 用工具级 grep；find ~ / 全仓递归 grep 极慢；
- **受限沙箱会拒绝仓库内的 rename 与删除** ⇒ 在仓库内跑项目构建时，产物条目落不了盘
  （`*.tmp-*` 残留、下次不命中）⇒ 本地跑 gate/基准时加 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`
  退回旧语义即可（CI 不需要）；
- `scripts/soko` 用的缓存二进制必须与仓库版本**一致**（现在 0.67.0），否则 gate 直接
  exit 3 ⇒ 重建后把二进制落到 `target/debug/`（`cargo build -p sokonanoda-cli -p sokonanoda-lsp`
  再复制过去）；
- **慢 ≠ 卡住**：`test` job 正常 25–35 分钟（本机 CI 等价 `--test-threads=2` 实测 15m05s），
  **进行中的 job 取不到日志**（取消后才可取）⇒ 先看 step 级状态再决定要不要取消。

## 已经做完的（别重做）
- R-1（Infoview 的 def 卡片缺 := 值行）已修：crates/lsp/src/query_map.rs 的
  decl_info() 漏映射 value/value_runs，已补 + 判据在 crates/lsp/src/tests/goals.rs；
- 三个内核原语（EnvBuilder::with_env / snapshot() / interner+Dag 的 Clone）已就位，
  各有判据，已进 docs/architecture.md §6 台账；
- 别重试 T-K31（TcCache 的 4MB 复用池）：实测无收益，结论在 docs/perf/ledger.jsonl；
- 别指望 SOKO_SHADOW_CHECK=1 的"影子彩排"：三个假设全被实测否证，它现在恒报
  一致=false，是实验工具而不是产品故障。

## 阶段收尾清单
    scripts/soko gate                                    # fmt+clippy+test+锚点+课程门禁+缺口台账
    python3 courses/set-theory/tools/check.py            # 计数必须逐项不变
    scripts/kernel-check.sh                              # 改过内核才需要
    SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh   # 批次收尾跑一次
然后：一次 push -> CI 绿 -> bump 两处版本 -> auto-tag -> release -> gh release list 核对。
发版前必跑（专抓本地看不见的坏数据）：
    git show origin/main:docs/e2e/ledger.jsonl | python3 -c "import sys,json;[json.loads(l) for l in sys.stdin if l.strip()]"

## 现在开始
先跑 python3 scripts/plan.py next，把"这一环节你打算怎么改、判据是什么"讲清楚，
再动手。改完一个环节就 commit（一个环节一个 commit），阶段收尾才 push。
遇到判断不了的路，先看 docs/design/vscode-editor-feedback-plan.md 里
T-K12c / T-K13 / T-K30 / T-K31 四段（那里把"哪些路走不通、为什么"记到了代码行）。
```
