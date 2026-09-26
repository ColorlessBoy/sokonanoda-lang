# AGENTS.md — sokonanoda-lang

给任何 code agent 的项目入口（DeepSeek Harness / opencode / Claude Code 等原生
读取本文件）。按序读完再动手：

1. `REQUIREMENTS.md` —— 用户全部要求的**权威总账**（硬规则、新要求追加到 §9）；
2. `docs/HANDOVER.md` —— **交接汇总**（现在在哪、还剩什么、怎么继续）；
   **接手新计划请看 `docs/E2-HANDOVER.md`** ✓（E2 计划的一页交接书：
   当前状态、怎么开工、硬规则、陷阱清单、验收命令）；
3. `STATUS.md` —— 当前进度（最新一轮在最上）；
4. `ROADMAP.md` §10 —— 待办与验收标准；
5. `docs/architecture.md` —— 流水线与内核 gotchas（§8 必读）。

文档已分层：入口/权威在仓库根（`README.md`/`AGENTS.md`/`ROADMAP.md`/
`REQUIREMENTS.md`/`STATUS.md`），开发者参考在 `docs/` 顶层，设计与调研笔记在
`docs/design/`、`docs/notes/`；完整地图见 **`docs/README.md`**。对外官网在
`site/`（数据由 `scripts/gen-site-data.py` 生成，永不手写版本号）。
harness 适配（各 harness 能用什么、缺什么）见 **`docs/design/deepseek-harness.md`**。

**改站点之前先读 `docs/design/site-single-page.md`**（2026-09-21 简化：官网是
**一个页面**，只讲 是什么 / 怎么安装 / 核心特点 / 未来的计划）。它取代了
2026-09-20 的 28 页重构（**已归档** ⇒ `docs/archive/site-rebuild-2026-09-26/`，
含 `site.md.gz`；**不要照着它们新建页面** ✗）。一条命令验收：

```bash
python3 scripts/check-site.py            # 10 项：结构 + 链接 + 版本 + 元数据 + 体积 + 已发布版本一致
python3 scripts/check-site.py --browser  # 额外跑真 Chrome（资源零 404 + 版本号已回填）
```

**站点写的是「已发布版本」的事实。** 本仓库常有并行开发，`crates/` 与 `courses/`
的未提交改动会让 `scripts/soko` 量到**未发布代码**（它优先解析仓库构建）。
量内核行为前先钉发布产物，或确认 `git status --short crates/ courses/` 干净 ——
详见 `docs/design/site-single-page.md` §4 与 `docs/archive/site-rebuild-2026-09-26/STATE.md.gz` §5
（后者是"实测与文档不符"的原始清单，仍然有效 ✓；`gunzip -c … | less` ✓）。

**第二大课（卷 I 集合论）已建在 `courses/set-theory/`**：入口
`courses/set-theory/README.md`；写课程内容前先读**硬规则 10**（课程标准库三层分界，
`REQUIREMENTS.md` §2）与 `docs/design/course-stdlib.md`；判卷一条命令
`python3 courses/set-theory/tools/check.py`（它的内部用绝对路径 + `grade` 退出码——
原因见台账 G-12；G-10 已修：`query check` 现在也带 parse 诊断并 exit 1，与 `grade` 同口径）。
判据是 **G1–G6**（每个目标 `grade` 退出码 0 / 目标存在 / 解答 0 open 且 checked>0 /
解答覆盖画布每个具名练习 / lib+Demo 0 open / **G6 清单自洽**：卷章 id 唯一、unit 恰好一章、
`prereqs` 不悬空——配额差额只报告不判红），**与规模无关、不锁计数**；
`--selftest` 自检判据通道（故意坏的单元必须被拒），`--json` 出计数，
`--only "<标签>" --bisect` 二分到第一个判红的声明（不依赖诊断 span——G-15）。
这门课的判卷已接进 `scripts/soko gate` 与 CI（设计 `docs/design/course-gate-in-ci.md`）。

## Setup（用户/agent 零 cargo；一条命令，任何 harness 都能用）

```bash
scripts/soko setup                        # 版本锁定的 CLI + LSP → 缓存（幂等）
scripts/soko doctor --json                # 就绪诊断；0=就绪 3=未就绪
scripts/soko grade playground.sokonanoda  # 判卷（--json 事件流）
scripts/soko grade course/unit11-project/Exercises.sokonanoda  # 多文件项目（import 闭包）
scripts/soko grade --root <模块根> <入口.sokonanoda>  # 显式模块根（默认：最近 sokonanoda.toml，否则入口目录）
scripts/soko grade --no-project <文件>    # 忽略 sokonanoda.toml，模块根 = 入口目录
scripts/soko course "$PWD/courses/set-theory/course.json" --json  # 整门课进度（有 import 的单元走同一份闭包，failed 与 grade 同判）
scripts/soko query check --file playground.sokonanoda   # 同一判卷的单 JSON 摘要
scripts/soko query state --file playground.sokonanoda --line 327 --col 4
scripts/soko query project --file course/unit11-project/Exercises.sokonanoda  # 项目闭包状态（根/清单/模块）
scripts/soko version --json               # 仓库版本 + 解析来源 + 缓存标记
scripts/soko update                       # 刷新缓存；0=写成了 3=没写成（stderr 给 download 原因）
```

- **判卷有两个视图，同一份真相**：`grade --json` = 全量事件流（既有消费者不变），
  `query <op>` = 计数/目标/洞/项目状态的**单 JSON 对象**（`check`/`state`/`goals`/
  `holes`/`hints`/`reduce`/`project`）。要问"某处还差什么"就用 `query state`，
  要问"哪个模块拖坏了入口"就用 `query project`，别自己扫事件流。
  契约见 `docs/protocol.md`；计数一致性由 `crates/cli/tests/query.rs` 钉死。
  `ok:false` **不是**空结果；退出码 0=答上了 / 1=有拒绝（内核拒绝**或**解析失败）/
  2=用法错误。
- **DeepSeek Harness** 里那七个查询还包成 MCP 工具
  （`mcp__sokonanoda__{check,state,goals,holes,hints,reduce,project}`，需
  `dsh web --patch ./dsh/cordis.patch.yml`）：有工具就直接调，别绕 shell。

- **`scripts/soko` 是 harness 中立的启动器**（零依赖 Node，跨平台、无 bash）：
  解析顺序 = `$SOKONANODA_BIN` → 版本**匹配**的仓库构建 → 缓存（标记
  `<version> <target>` 必须与**版本钉**一致）→ VS Code 扩展自带 →
  按版本钉锁定下载；其余子命令原样转发给 `sokonanoda` CLI。
  **版本钉的源链**（课程仓没有 `Cargo.toml` 也能自钉）= `$SOKONANODA_VERSION` →
  `<repo>/sokonanoda-version.txt` → `<repo>/sokonanoda.toml` 的 `requires`（完整
  `x.y.z` 才算钉，`0.58` 只是约束）→ `<repo>/Cargo.toml`；出现多个源必须一致，
  不一致就指名文件报错。**解析不出期望版本就绝不 exec**（缓存/仓库构建都拒绝，
  exit 3 + 人话）；缓存**过期就拒绝运行并提示**（它是历史上最常见的故障源）。
- 已经有 `sokonanoda` 在 PATH 上时，上表的 `scripts/soko …` 可换成
  `sokonanoda …`（等价）；DSH 里没有项目级 PATH 注入，所以文档一律先给启动器形式。
- 环境能力本身是 `sokonanoda` 二进制的子命令（内嵌下载器，跨平台；旧的
  `scripts/soko.sh` 已删除）。
- opencode 额外有：`/sokonanoda/setup` `/sokonanoda/update`
  `/sokonanoda/version` `/sokonanoda/doctor` `/sokonanoda/check`
  `/sokonanoda/gate`，以及启动插件自动 provision。
- DeepSeek Harness 额外有：技能目录自动发现（`.agents/skills/`），技能名即
  `/sokonanoda-teacher` 等命令；两个**人工**运维命令
  `/sokonanoda-update`（刷新缓存）与 `/sokonanoda-doctor`（就绪诊断）也已上架
  （`disable-model-invocation`，不进模型目录）；编辑器 LSP 需显式
  `dsh web --patch ./dsh/cordis.patch.yml`（详见 `dsh/README.md`）。
  DSH 的斜杠命令文法不允许 `/`，所以 opencode 的 `/sokonanoda/update` 在 DSH
  侧只能拼成 `/sokonanoda-update`。
- 贡献者（需要 Rust）：`scripts/soko gate`（= fmt + clippy + test + playground
  锚点 **+ 课程门禁** `courses/set-theory/tools/check.py` **+ 缺口台账门禁**
  `scripts/gap.py check`；后两步要 python3，探不到就 **exit 3**、绝不静默跳过）
  或 `cargo build/test`（见 `skills/sokonanoda-dev`）。
- 网络受限时设代理（`HTTPS_PROXY=http://127.0.0.1:7890` 之类），启动器会把它
  交给 `curl` 下载。
- 禁止：`releases/latest`、为使用仓库安装 Rust/cargo（REQUIREMENTS §2 第 9 条）。

## 角色技能（Agent Skills）

- **当老师（产品主循环）**：加载 `sokonanoda-teacher`——画布
  `playground.sokonanoda` 出题/判卷/决策的完整操作手册（正文
  `skills/sokonanoda-teacher/SKILL.md`）。DeepSeek Harness 里直接输入
  `/sokonanoda-teacher`。
- **做开发**：加载 `sokonanoda-dev`——内核可改但**正确性不变**、TDD 三层、文档先行。
- **推代码/发布/查 CI**：加载 `sokonanoda-ci`——本地验证纪律
  （退出码、无 grep 掩膜）、workflow 陷阱、`gh` 排错三板斧、失败必录。
- **运维（人工命令）**：`sokonanoda-update`（把缓存的 CLI + LSP 刷到仓库
  版本）与 `sokonanoda-doctor`（只读就绪诊断）——它们**只给人用**
  （DSH 入口带 `disable-model-invocation: true`，模型目录里没有）；模型侧
  等价能力已经写在本文与三个角色技能里，不属于需要"加载"的技能。

> 技能在 DeepSeek Harness 下经 `.agents/skills/<name>/SKILL.md` 被自动发现
> （薄入口，正文仍以 `skills/<name>/SKILL.md` 为唯一源）；`crates/cli/tests/dsh.rs`
> 守住两边不漂移。

## 硬规则速记（全文见 REQUIREMENTS §2/§3）

1. **kernel 可以改**（2026-09-21 用户解冻：含热路径与内部表示，目的可以是**提速**）。
   唯一红线是**判定正确性不变**：同一批输入**接受/拒绝不变、事件计数不变、
   golden 与 `--json` 逐字节不变**。每次内核改动必须带三层回归
   （kernel `tests/` + front 单测 + CLI e2e）与语料对拍；性能改动另记
   `docs/perf/ledger.jsonl`。内核相对上游的改动台账在 `docs/architecture.md` §6，
   **改内核前先读它 + §8 gotchas（arena 生命周期、panic→Result、`quiet_catch` 不可嵌套）**；
2. 不调用官方 Lean 工具链（lean/lake/lean4export/leanc/elan）——opencode 由
   `opencode.json` 权限 deny 强制；**DSH 用 `dsh/hooks/hooks.json` 的
   `PreToolUse` 拦截（需在 profile 插一行启用，见 `dsh/README.md`），未启用时
   退回文档纪律**；
3. 教学语法是真实 Lean 4 的子集；新增语法 = 课程 + 测试 + 白名单三件套；
   **签名也受检**（G-01/0.59.0）：值位是 `sorry` 不免检签名——签名 elaborate 不了、
   不是类型、或 `theorem` 的不是 Prop ⇒ 一条 diagnostic + 声明 Failed + **不发**
   `exercise.open`。判卷只认 `decl.checked` 与 `diagnostic`，别拿 `exercise.open`
   计数当"签名没坏"的证据；
   **记法是例外面**（G-04/0.59.0）：`infix:N`/`infixl:N`/`infixr:N`/`notation`
   是用户自定义的源级糖——不引入新语义、不产生事件，点名形式永久可用且两种写法
   判卷一致；边界（文件内作用域、补前导类型参数、第二刀未做项）见
   `docs/design/notation-subset.md`；
   **课程一律写记法**（2026-09-21 用户拍板，脚本判红）：`Eq.{1} T a b` → `a = b`、
   `Set.mem α a A` → `a ∈ A`、`And X Y` → `X ∧ Y`、基础类型省前导隐式实参
   （`And.left h` / `Or.inl h` / `Exists.intro w hw`）；判据
   `python3 scripts/notation-lint.py`（已进 `scripts/soko gate` 与 CI），
   细则 `docs/notes/course-lean-style/notation-rewrite-brief.md`；
4. 判定永远走 kernel——**禁止文本比对**（tactic 判定范例：`front::judge`）；
5. 模块化：文件接近 ~500 行即拆分；公开 API 用 re-export 保持稳定；
6. **用户/agent 路径零工具链依赖**：获取与运行只用 Release 二进制或平台
   插件，不把 cargo/Rust 当使用前提（cargo 仅贡献者开发需要；见
   REQUIREMENTS §2 第 9 条）。

## 命令（贡献者：需要 Rust；用户/agent 用 `scripts/soko` / `sokonanoda` 子命令）

```bash
scripts/soko gate --fast   # **迭代内环**（~30s）：fmt + clippy + **改动过的 crate 的单测**
                    #   + 锚点 + 课程门禁（走持久缓存）；跳过缺口台账门禁与集成测试
scripts/soko gate   # = CI 门禁（**提交/推送前**跑这条）：fmt + clippy + test + playground 锚点
                    #   + 课程门禁（卷 I）+ 缺口台账门禁（python3）
python3 courses/set-theory/tools/check.py --selftest   # 课程判据通道自检（故意坏文件必须被拒）
python3 courses/set-theory/tools/check.py --only "单元 5" --bisect   # 二分到第一个判红的声明
python3 scripts/gap.py selftest   # 台账判据自检（judge() 的期望推导 / repro_expect / 非法值）
python3 scripts/gap.py check      # 台账契约：每条缺口的复现必须与 status 一致（gate 已含，单跑用这条）
scripts/perf-ledger.sh   # 性能台账：跑全部 perf 套件 → docs/perf/ledger.jsonl（提交它）
SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh
#   真 VS Code 集成测试（构建 release → stage → npm test）→ docs/e2e/ledger.jsonl（提交它）；手册 docs/E2E.md
# 注意：gate 的 anchor 用**运行中二进制**的内嵌编译器；若它与仓库版本不一致
# （旧缓存/旧构建），gate 会直接 exit 3 —— 先 `scripts/soko update`，或用
# `cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda`。
# 或手动：
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
# 禁止 `cargo fmt --all`：kernel 的 rustfmt.toml 需要 nightly，`--all` 会重排整个内核
# （噪声巨大、掩盖真实 diff）。只 fmt 教学 crates，或直接 `scripts/soko gate`。
cargo clippy --workspace --all-targets
cargo test --workspace --locked
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda
```

编辑器/agent 反馈通道：`.sokonanoda` 文件的 LSP 诊断由完整 kernel 判定。
**编译缓存**：**项目**闭包产物落**模块根** `<模块根>/.sokonanoda/compiled/`
（同格式同键、自忽略；`--clean` **两处都清**；逃生门
`SOKONANODA_NO_PROJECT_ARTIFACTS=1`；设计 `docs/design/project-artifacts.md`）。
`sokonanoda build [--clean] [<file>|<dir>]`（CLI）与编辑器里的
`Sokonanoda: Build (编译当前文件/工作区，预热缓存)`（`alt+b`）/ `Sokonanoda: Rebuild (清空编译缓存后重编译)`（`alt+shift+b`，先清缓存）
是同一条路——第一次按键慢、或在编辑器外改了依赖后面板像"没反应"，先 rebuild。
两种接线：

- **opencode**：启动插件自动接线（解析原生 `sokonanoda-lsp`——仓库构建 /
  VS Code 扩展自带 / 缓存 / 版本锁定下载（`fetch`+`tar`，跨平台、零 bash）
  ——并改写 `lsp.command`；`shell.env` 注入 PATH；`opencode.json` 不含 lsp
  命令）；另有 `skills/` 自动加载、`/sokonanoda/*` 命令、`teacher` 主 agent、
  Lean 工具链命令 deny。
- **DeepSeek Harness**：技能与 `/sokonanoda-*` 命令自动可用
  （`.agents/skills/`），但 LSP 需显式启用
  `dsh web --patch ./dsh/cordis.patch.yml`；且**服务端诊断不会被投递给
  agent**——判卷一律走 CLI `--json`（`dsh/README.md`、
  `docs/design/deepseek-harness.md`）。
- 其他 harness 可用 `.opencode/lsp/sokonanoda-lsp.sh` shim →
  `scripts/soko lsp`。

goal 视图走自定义请求
`soko/goals` / `soko/hints` / `soko/nextHole` / `soko/stateAt` /
`soko/project`（项目闭包状态：根/清单来源/模块表/每模块状态）/
`soko/version`（服务器自述 {version,pid}，重启命令用）
（`docs/protocol.md`）——**目前只有 VS Code 扩展与 opencode 消费它们**，
DSH 侧无消费者（属于 `docs/design/deepseek-harness.md` 的 H5 backlog）。
每个 release 仍正常产出各平台
`sokonanoda-lsp-<triple>.tar.gz` 与 `sokonanoda-cli-<triple>.tar.gz`
（各 8 个；CLI tarball 里就是可直接执行的二进制，agent 无需 cargo）与
VSIX（9 个，平台包内嵌 LSP 与 CLI），供自动下载与 headless 手动安装；
**下载一律按仓库版本锁定，禁用 `latest`**。
**发版已全自动**：bump 两处版本 → push main → `ci.yml` 的 auto-tag 自动打
tag 并 dispatch release（见 `docs/RELEASE.md`；手动推 tag 仅应急）。

## VS Code 扩展改动

改 `editor/vscode/` 下的任何文件前，先读 `docs/vscode-dev-guide.md`
（版本纪律 / 测试三层 / 常见坑）。**版本号 = 发布触发器**：`ci.yml` 的 auto-tag
只在"版本号对应的 tag 还不存在"时发版 ⇒ 不动版本号推 main 只跑 CI、不发版；
CI 强制的只有 `Cargo.toml` 与 `package.json` **相等**，且版本号只能是纯 `x.y.z`
（带后缀会让 `scripts/soko` 按 G-16 拒绝运行）。开发期与发布期的 bump 时机见
`docs/vscode-dev-guide.md` §2。
扩展改动后的例行三层：`scripts/soko gate`（Rust/契约）→
`node editor/vscode/test-extension-host.js`（stub 宿主）→ `scripts/vscode-e2e.sh`
（**真 VS Code**，结果记进 `docs/e2e/`；手册 `docs/E2E.md`）。
单环节快速反馈：`scripts/dev-loop.sh lsp` + 命令面板 `Sokonanoda: Restart Server (重启服务器)`
（需 `sokonanoda.serverOverride`），或 `SOKO_E2E_GREP=<用例名> npx vscode-test`
只跑一个 e2e 用例。

## CI 节奏：**批次制**（2026-09-24 用户拍板，长期工作方式）

**默认：一个批次只 push 一次、只跑一轮 CI。**

1. **同一批次内的多个环节，先在本地连续改完**（**一个环节一个 commit** ✓；
   **批次内可以有很多 commit** ✓ —— 提交粒度细、推送粒度粗 ✓）；每个环节该跑的本地判据
   （`cargo test -p …` / 该环节的验收命令 / `scripts/soko gate --fast`）**照常跑**
   ——**不要每改一个就 push 等 CI**。
2. 一批（或一批紧密相关的环节）**全部改完、本地验证通过**后，**才 push 一次**，
   统一跑一轮 CI。（三平台矩阵一轮 33–35 分钟，这是要省的成本。）
3. **例外：诊断性 CI**——只有当**本地复现不了、怀疑是平台差异**时（例如只在
   ubuntu runner 上出现的时序问题），才为定位**单独**跑一次 CI。
   这类 CI **必须在 `STATUS.md` 写明原因**（"为什么本地复现不了"），
   **不许变成默认动作**。
4. **e2e 台账按批次记一条**：`SOKO_VSCODE_TEST_VERSION=<ver> scripts/vscode-e2e.sh`
   在**批次收尾**时跑一次、记进 `docs/e2e/ledger.jsonl`——不要每个环节一条。
5. **BUMP 点仍然闭环**（REQUIREMENTS §9）：bump 是"批次收尾"的一部分——
   批次改完 → 本地全绿 → **一次** push → CI 绿 → auto-tag → release →
   `gh release list` 核对。**闭环用的就是批次那一次 CI**，不额外多跑。

> 为什么这么定：单环节 push 的边际信息很小（本地判据已经覆盖了绝大多数回归），
> 而每轮 CI 要 33–35 分钟；把 CI 用在**批次边界**上，才能既保住"线上一定绿"
> 又保住迭代速度。

## 验证设计纪律（2026-09-24 用户拍板：**验证环节没设计正确就是白做功**）

**起因（真实事故，两次同形）**：
1. T-D52 给 Infoview 的 `def` 卡片加了"第二行 `:= <值>`"，**e2e 全绿**却**用户看不见**
   —— LSP 的 `decl_info()` **漏映射** `value`/`value_runs`，而扩展是「**有就渲染**」的
   宽容实现 ⇒ **静默降级**。三层测试全没抓到：front 单测验"真相"（数据算对了）、
   LSP 单验"值"（**没人断言字段必须在 wire 里**）、e2e 验"声明存在"
   （**没人断言屏幕上看得见什么**）。
2. R-2 的"目标不高亮"同理：真因是 `crates/front/src/semantic.rs:308` 把**内建记法表**
   （含 `"="`）喂给词法 ⇒ `fun (x : Nat) => z` 里 `=>` 的 `=` 被当声明符号吃掉 ⇒
   整段**降级成 1 个无 kind 的 run** ⇒ webview 只画纯文本。**而三层测试全绿**：
   每层都只验了自己那一段，**没人验"整条链的用户可见结果"**。

**⇒ 病根**：**真相**与**显示**是两条路，bug 活在**接缝**里。**数据对了 ≠ 用户看见了**。

**四条硬规则**：

1. **每条用户可见的改动，必须先回答一句**：**"屏幕上会多/少什么？那条断言在哪一层？"**
   答不上来 ⇒ **验收不完整**，不许勾环节。
2. **三层各司其职，缺一层就是洞**：
   * front / CLI 单测 = **真相**（算得对）；
   * LSP 单测 = **wire 契约**（**字段存在性**，不只是值）；
   * **e2e = 用户看到的东西**（**渲染结果**，不只是"数据在"）。
3. **接缝要有专门的守卫**：凡是"A 层产出、B 层消费"的字段，都要一条 **A∖B 对账**
   —— 已有：`python3 scripts/audit-wire-fields.py`（扩展读了 / LSP 从不发的字段，
   已进 `scripts/soko gate` 与 CI）。
   **反向验证是硬要求**：守卫必须能**咬住已知的历史 bug**（内存回退 R-1 时它必须报
   `value_runs`）—— 咬不住的守卫等于没有。
4. **「有就渲染」是反模式**：宽容消费者会**掩盖契约破坏**。出路：
   ① 契约测试覆盖它（首选）；② 消费者在字段缺失时给**可见信号**（至少 `console.warn`）。

> 一句话：**验收必须断言"用户可见的结果"，接缝必须有能咬住历史 bug 的守卫。**

## 并行与 subagent 纪律（2026-09-24 用户拍板：吸取两次教训）

**两次教训**：上一版计划"**太少 subagent**" ⇒ 全程串行、**太慢** ✗；
更早一版"**太多 subagent**" ⇒ 烧 token ✗、**经验不共享** ✗、**错误百出** ✗。
下面是取两者中间的规则 ✓：

1. **subagent 只做可并行、只读或边界清晰的工作** ✓：调研、逐文件审计、
   多角度验证、写复现件、量基准数字 ✓。
   **判定相关代码与发布闭环走主会话关键路径** ✓（内核改动、bump/release、
   跨模块重构 ✗ 不外包）。
2. **并发上限 2–4 个** ✓；**写操作串行** ✗ —— 同一文件/同一模块同一时间
   只允许一个写者，避免互相覆盖 ✓。
3. **每个 subagent 的 prompt 必须自带三样** ✗（缺一样就会重复踩坑、结论不可用）：
   * `docs/E2-HANDOVER.md` 的**规则与陷阱摘要** ✓（尤其 §5 陷阱清单 ✓）；
   * 它要交付的**判据**（**可执行的验收命令** ✓，不是"看看对不对" ✗）；
   * 明确的**"不许改什么"边界** ✓（例如"只读，不许改 `crates/kernel/`" ✓）。
4. **产出必须验证后才并入** ✓：复跑它给的判据 ✓、抽查结论 ✓
   （"错误百出"那次就是没人验 ✗）。
5. **发现要回写** ✓：subagent 的结论写进 `docs/` 或计划 ✓
   ⇒ 下一个 subagent/下一轮能复用 ✓（这就是"经验共享"的机制 ✓）。
6. **提交粒度不变** ✓：一个环节一个 commit ✓、一阶段多 commit ✓、
   **阶段收尾才 push** ✓。

> 一句话：**把"可并行的取证与验证"外包，把"判定与发布"留在主线，
> 每条外包都带判据、都要验、结论都要回写。**

## 长命令的工作方式（2026-09-26 用户拍板：**四条硬纪律**）

**起因（实测）**：一次后台 `cargo test` 被干等了 30 分钟、全程黑箱；同一个 crate 的全量
测试为两种 grep **跑了两遍**；连续 4 次 push 各自把上一轮 CI 顶掉（**四轮全 cancelled** ✗），
于是"推上去的代码一次都没真过门禁"。

1. **后台一律落盘 + 自带超时，禁止阻塞干等。**
   ```bash
   timeout 1500 <cmd> > /tmp/<有意义的名字>.log 2>&1 &
   # 之后用 `tail -n 5 /tmp/<名字>.log` 轮询（外部可读、可观测）
   ```
   别用"等 30 分钟的阻塞读"：框架的后台输出进**内存 ring 缓冲**、不落盘，
   而且**后台没有执行超时** ⇒ 卡死了外面也看不出来 ✓。
2. **同一个编译只跑一次。** 跑一次 → 存文件 → 想 grep 几次就 grep 几次 ✓
   （不同 grep 各跑一遍 = 白付一次完整编译）。
3. **慢要能定位，不许只喊"慢"。** `cargo test -p X --no-run --timings`（或
   `cargo build --timings`）出 HTML 耗时报告（`target/cargo-timings/`）；把**最慢的
   5 个 crate** 与实测秒数记进 `STATUS.md` ✓。
4. **每条交付带"耗时账"，并且必须确认 CI 真跑绿。**
   * 本次改动让哪条命令变快/变慢 ⇒ **给数字**（秒/分钟）✓；
   * **推送后等那一轮 CI 跑完**，确认绿 ✓ —— 不许"连推几批让它们互相 cancel"就算完 ✗；
     总要被 cancel 就**停下来等一轮**再推下一批 ✓（批次制的本意就是这个 ✓）。

> 一句话：**长命令要留痕（落盘 + 超时 + 轮询），编译只付一次，慢要量出来，
> 推完必须看见绿。**

## 性能回归门禁（`perf-gate`，T-E1，2026-09-25 ✓）

CI 的第 15 个 job ✓（**快层** ✓）：**每次 rust 改动的 push** 跑一组 **smoke** ✓
（合计 **~1 秒** ✓），与 `docs/perf/ledger.jsonl` 的**上一次同名记录**比较 ✓，
**大幅退化就红** ✗。**它不是"再快一点"✗，而是"以后慢下来会被发现"** ✓ ——
**D-1/D-2 那两刀收益量不出** ✗，但它们的**热路径**（`did_open_same_session`，**134ms** ✓）
从此有守卫 ✓。

- **本地跑** ✓：`scripts/perf-check.sh --case judge_prefix_with_imported --threshold 50` ✓
  （`--list` 看台账里有哪些 `(scope, case)` ✓；`scripts/perf-ledger.sh` 跑全量 ✓）；
- ⚠ **`--case` 是"测试名子串"** ✗ —— **不是**台账里的 `(scope, case)` 名 ✗
  （实测：拿台账名 ⇒ **5 个里 3 个 `exit=2`** ✗ = "没跑到任何 case" ⇒ **守卫空转** ✗✓）。
  **新增过滤器必须先 `git grep` 出真实测试名 ✓，再逐个验证 `exit != 2`** ✓
  —— **咬不住的守卫等于没有** ✓；
- ⚠ **"CI runner 比本地吵"这个诊断是错的** ✗✓（2026-09-25 round 430 **实测推翻** ✓）：
  `0.72.0` 的发版轮里 `perf-gate` 报 **`+585%` ~ `+1189%`** ✗（**四条 case** ✓），
  而台账的 `host` 是 **`{system: Darwin, machine: arm64}`** ✓ —— **本机 Mac** ✗，
  **CI 是 `ubuntu-24.04` 的 2 核 runner** ✗ ⇒ ⇒ **那是"跨机器比"，不是噪声** ✗✓。
  **⇒ 真因** ✓：`perf-compare.py:98-108` 的 `comparable()` **本来就比 `system`/`machine`** ✓，
  **而 CI 走的是 `perf-check.sh` 的内联比较** ✗ —— **它只按 `(scope, case, entry)` 取基线** ✗
  ⇒ ⇒ **已修** ✓（round 432 ✓）：**两处都带上宿主** ✓（`_h.get("system")` / `platform.system()` ✓）
  ⇒ **本地仍能比** ✓（实测 `-0.8%` ✓）、**CI 变成"（无基线）"** ✓ ⇒ **不再假红** ✓。
  ⚠ **所以 `--threshold` 是治症状** ✗ —— **调到多大都追不上 7.5× 的机器差** ✗
  （**同一套件：本机 37.37s vs CI 278.58s** ✓）；**正解是"只跟同宿主的记录比"** ✓；
- **第一轮 `continue-on-error: true`** ✓（只报不拦 ✓）⇒ **CI 会绿** ✗
  ⇒ **读它必须 job 级** ✓（`scripts/ci-watch.sh` ✓ / `gh run view <id> --json jobs` ✓），
  **不能看整轮结论** ✗（**批次制第 d 条** ✓）；
- **详见** `docs/PERF.md` 的"性能回归**门禁**"一节 ✓ 与 `docs/design/ci-parallelism.md` ✓。

## CI 失败记录

每次 CI 红了，在 `docs/CI-FAILURES.md` 追加一条（原因/修复/预防）。
同一类失败不犯第二次。

## 收尾义务

- 落 commit 前更新 `STATUS.md`（只保留最近 3 轮，旧轮归档
  `docs/STATUS-ARCHIVE.md`）；
- **文档预算**（用户 2026-09-26：「文档太重了，还没实现多少东西文档先爆炸了」）：
  `python3 scripts/docs-lint.py`（已进 `scripts/soko gate` 与 CI）—— 活文档 ≤3.0 MB ·
  单文件 ≤2000 行 · 入口文件 ≤800 行 · 新设计文档 ≤150 行（**既有按
  `scripts/docs-budget.json` 冻结：只许减不许增**）· `docs/**` 禁 `.tmp`/`.tmpdir` ·
  **归档必须被 `docs/archive/README.md` 点名**（归档≠销毁）。**设计先行只写契约不写过程**
  （过程进 commit message 与 `STATUS.md`）；要放宽预算 ⇒ **手改那份 JSON**（评审可见）。
  设计与判据：`docs/design/docs-diet.md`。
- 用户新要求追加进 `REQUIREMENTS.md` §9 并注明日期（冲突以该文件为准）；
- 设计先行：新功能先写设计进 `docs/`，再动手；多用 subagent 并行调研。
- **VS Code + skills 同步**：任何用户可见改动（命令/键位/视图/反馈/语法/协议/发布形态）
  必须**同一轮**更新 `editor/vscode/`（README/CHANGELOG/package.json）**与** `skills/`
  三个技能 + 本文 + `docs/vscode-dev-guide.md`；测试见 `crates/cli/tests/skill.rs`。
- **harness 同步（opencode 与 DeepSeek Harness 都算一等公民）**：改 `skills/` 正文后
  确认 `.agents/skills/` 入口仍指向它（`crates/cli/tests/dsh.rs` 会挡住漂移）；
  改环境/判卷命令后用 **`scripts/soko`** 形式（harness 中立），opencode 专属
  （插件/`/sokonanoda/*`/teacher agent）与 DSH 专属（`dsh/cordis.patch.yml`）
  各自在同一轮同步；能力差异以 `docs/design/deepseek-harness.md` 为准。
- **code agent 适配是一等公民**：每个开发计划先问「agent 怎么用/怎么验证」——提供
  `--json` 结构化输出、把能力写进 skills、命令可直接执行、`docs/HANDOVER.md` 同步。
- **skill 写法**：命令用**确切可执行的一条命令**（`scripts/soko version --json`），
  少用 token、少用"你应该考虑…"式散文；skill 是给 agent 执行的操作手册。
