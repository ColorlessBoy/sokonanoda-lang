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
    python3 scripts/plan.py list      # 看进度（现在应为 3/38）
    python3 scripts/plan.py done <ID> # 做完勾掉
当前应从 **T-A5（R-2 修复）** 开始。已完成、别重做：T-A1..T-A4。
**T-A5 的核心已经落地**（`crates/front/src/semantic.rs`：不再把 `=` 喂给词法符号表
—— 修前含 λ 的 goal/类型文本会整段降级成 1 个无 kind 的 run，webview 就不上色；
实测 `flawed_equalities_refuted` 1→313 段、`project_chain` 1→216 段、
对照组 `project_chain_cardinal` 94 段不变）。**还剩**：`goal_runs` 的父子补齐
（front types.rs → LSP protocol.rs/query_map.rs → infoview.js + CSS）+ 一条
**"看得见"的 e2e 断言**。

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
- bash 的 grep 在本机会静默返回空 => 用工具级 grep；find ~ / 全仓递归 grep 极慢。

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
