# 计划：VS Code 项目模式（`set-theory` 为根）六条反馈的分环节修复

> **状态：只做计划，未改任何产品代码**（2026-09-21）。
> 本文每一条根因都有**实测命令 + 实测输出**（§2），不是推断；无法证实的条目
> 一律标成「未证实」并给出判定实验。
>
> **交付方式**：一个环节 = 一个可独立验收的小改动。用户逐环节验收，
> 每环节自带**一条可粘贴的判据命令**与期望输出（§0.3）。
>
> 关联：`REQUIREMENTS.md` §9（本轮要求）、`ROADMAP.md` §10（指针）、
> `docs/gaps/`（缺口台账：本计划每条反馈都落成一条缺口 + 复现脚本）。

---

## 0. 怎么用这份计划

### 0.1 用户的验收流程（每一环节都长这样）

```bash
# 1) 缺口复现脚本：0 = 缺口仍在，1 = 已修，2 = 环境异常
bash docs/gaps/repro/G22-lsp-goals-empty-for-import-entry.sh ; echo "exit=$?"

# 2) 本环节声明的「判据」命令，对期望输出（见各任务）

# 3) 本环节对应的**真 VS Code e2e 用例**（功能真的修好了没有）
scripts/vscode-e2e.sh --grep "<用例名>" --profile debug --no-build

# 4) 全貌：六条反馈各是什么状态，一条命令
bash scripts/verify-editor-issues.sh

# 5) 批次收尾才跑全量门禁
scripts/soko gate
node editor/vscode/test-extension-host.js
scripts/vscode-e2e.sh                      # 全量 e2e，结果进 docs/e2e/ledger.jsonl
```

**粒度原则**：一个环节只做一件事；判据必须能**一条命令**跑出来；
凡是改了行为的环节都必须**先有判红的复现脚本**（仓库既有纪律：缺口即测试，
`docs/gaps/README.md`）。

### 0.2 版本与发版：**版本号不动 = 零 release**（实测过 CI 逻辑）

`ci.yml` 的 `auto-tag`（`.github/workflows/ci.yml:39-71`）在 push main 且
lint/test/e2e/e2e-macos 全绿后：

```
cargo_version = Cargo.toml 的 version
ext_version   = editor/vscode/package.json 的 version
两者必须相等（不等直接 exit 1）
tag = "v${cargo_version}"
若 origin 上已有该 tag  →  "tag already exists — nothing to release"，exit 0
否则 打 tag + push + gh workflow run release.yml
```

⇒ **只要不动版本号，推多少次 main 都不会发版**（只跑 CI）。
`docs/vscode-dev-guide.md` §2 的"每次 commit 涉及 `editor/vscode/` 必须 bump"
是**约定**，不是门禁；唯一被 CI 强制的是"Cargo == package.json"。

所以"环节数"和"版本号"是**两件可以解耦的事**。

#### 定案（2026-09-21 用户拍板）：**到一定程度就 bump，不等批次**

> 用户原话：「不行，到一定程度就 bump 一下」。⇒ 不是"整个批次一个版本"，
> 而是**沿途按阈值 bump**，这样每个阶段都能在 Marketplace 装到、都能自己先试。

**bump 触发（命中任一就 bump，不必等批次收尾）：**

1. **一条用户可感知的能力落地**（不是内部重构）——例如"声明栏不再为空"、
   "goal 显示记法"、"打开不再等 5 秒"；
2. **距上次 bump 已过 ≥ 8 个环节**（防止小步拖太久）；
3. **用户要拿去测**。

**版本类型**：新能力 = `minor`；只是修好/更快 = `patch`（`docs/vscode-dev-guide.md` §2 的口诀：
"学习者能不能做一件之前做不了的事"）。

**bump 动作（两处必须相等，否则 CI 直接 exit 1）：**
`Cargo.toml` 的 workspace version + `editor/vscode/package.json` 的 version
→ push main → `ci.yml` 的 auto-tag 打 tag 并 dispatch release → Marketplace
（索引延迟约 5 分钟）。发版前 CI 要全绿（lint / test / e2e / e2e-macos）。

**bump 点已经标进 §13 的线性清单**（`⬆ **BUMP**` 行，共 **13 个**，
平均每 ~9 个环节一次）：

| 批次 | bump 点 | 类型 | 理由 |
|---|---|---|---|
| 0 | **无** | — | 批次 0 的产物是判据、e2e 基建与性能基建，**零用户可见改动** ⇒ 不发版 |
| 1 | T-B05 / T-B14 | patch ×2 | 声明栏服务端修好；客户端四个缺陷 + e2e 证明 |
| 2 | T-A22 / T-A14 / T-A51 | patch / minor / patch | 清浪费；LSP 接入缓存（重开不再等）；收尾 |
| 3 | T-C24 / T-C41 | minor / patch | goal 第一次显示记法；着色 + golden 重钉 |
| 4 | T-D03 / T-D17 / T-D41 | patch / minor / patch | hover 原始类型；F12 跳转；收尾 |
| 5 | T-K11 / T-K12 / T-K41 | minor ×2 / patch | K1-a；K1-b（36.1s → 个位数秒）；收尾 |

**bump 的两个已知代价（接受即可）**：
① **编译缓存全失效**（缓存键含 `CARGO_PKG_VERSION`）⇒ bump 后第一次打开会重编一遍，
之后恢复；
② 每次 release 要跑完整流水线（8 LSP tarball + 8 CLI tarball + 9 VSIX +
SHA256SUMS + provenance + Marketplace 发布），历史上 gallery 会间歇超时
（`docs/CI-FAILURES.md`）。

**怎么知道该 bump 了**：`python3 scripts/plan.py next` 会在下一条做完要 bump 时
直接打出来；`python3 scripts/plan.py bumps` 列出全部 13 个 bump 点。

#### 本地测试版本号：**只能用纯 `x.y.z`**

`scripts/soko` 的版本解析正则是 `^v?(\d+)\.(\d+)(?:\.(\d+))?$`
（`scripts/soko:110`）⇒ `0.63.0-local` / `0.63.1-dev.1` 这类**带后缀的版本号会让
启动器解析不出"期望版本"，按 G-16 直接拒绝运行**（`scripts/soko:504-524` 的
`binaryVersion`/`versionSatisfied` 同理）。要支持带后缀的本地版本，得先改启动器的
解析链（并同步 G-11/G-16 的复现与测试）——**不划算**。

本地"这是哪份构建"的标识，仓库里已经有更好的手段，不需要动版本号：
`scripts/soko version --json` 的 `source`（`repo-build`/`cache`/…）、
`sokonanoda: doctor` 的 `source=` 与 `server-version` 行、
`docs/e2e/ledger.jsonl` 里的 `dirty` 与 `lsp_sha256_16`。

#### 批次表（**批次 = 主题分组，不是发版边界**；发版按上面的 bump 点）

| 批次 | 内容 | 环节数 | bump 点 | 主要收益 |
|---|---|---|---|---|
| 批次 0 | 阶段 0（判据、**e2e 基建**、性能基建、回路与文档） | 23 | **0**（零用户可见改动） | 后面每一步都可判红/判绿、可快速看 |
| 批次 1 | 线 B（声明栏 + `nextHole`） | 15 | 2（patch ×2） | 面板立刻可用，最便宜 |
| 批次 2 | 线 A（缓存 + 清浪费） | 24 | 3（patch/minor/patch） | 打开/按键从秒级降到毫秒级 |
| 批次 3 | 线 C（goal 用记法） | 23 | 2（minor/patch） | 目标/类型行可读 |
| 批次 4 | 线 D（记法跳转 + hover 原始类型） | 20 | 3（patch/minor/patch） | 编辑器导航闭环 |
| 批次 5 | 线 K（**内核解冻后的性能根治**，见 §5.6） | 12 | 3（minor ×2/patch） | `by` 块 36s 与闭包重复编译的真正大头 |

> 批次之间的**检查点**（CP0…CP-K）仍然要跑全量门禁（`scripts/soko gate` +
> 全量 e2e + `perf-compare`）；但**发版不必等到检查点**——按 bump 点走。

### 0.3 每一环节的**完成定义（DoD）**——贴给实现者

```
1. 先写/跑复现脚本，确认判红（exit 0 = 缺口仍在）；跑 `python3 scripts/gap.py check`（必须仍绿）
2. 最小改动（不夹带重构；要重构单独立环节）
3. 跑本环节的「判据」命令，把**实际输出**贴进 commit message
4. **跑本环节对应的 e2e 用例，确认转绿**（§0.5 的矩阵；
   先在改动前跑一次确认它是**红的**——否则这条 e2e 证明不了任何东西）
5. **跑性能检测**（§0.6）：`scripts/perf-check.sh --case <本环节相关场景>` 把数字贴进
   commit message；再跑 `python3 scripts/perf-compare.py --since <上一检查点>`，
   **任何 case 的 `best_ms` 退化 > 25% 必须解释或修回**
6. 更新对应设计文档的 as-built 段
7. 一条 commit；message 末尾写 `判据：<命令> → <结果>` + `e2e：<用例名> → pass`
   + `perf：<case> → <数字>（Δ <±x>%）`
```

> **三条机械判据缺一不算完成**：复现脚本证明"缺口没了"，e2e 证明"真编辑器里能用"，
> 性能检测证明"没把别的地方弄慢"。前两条防漏，第三条防**回退**——
> 109 个环节里"修 A 弄慢 B"是必然会发生的，而它**只有机械判据能拦住**。

### 0.4 快速反馈回路：每环节 5 层，越靠前越快（**这是"每次少做点"的落地**）

| 层 | 命令 | 耗时 | 用在哪一类环节 |
|---|---|---|---|
| **L1** LSP 直探（不开 VS Code） | `bash docs/gaps/repro/G2x-*.sh` | 秒级 | 线 A/B/C/D 的绝大多数（判据就是它） |
| **L2** 扩展 stub 宿主 | `node editor/vscode/test-extension-host.js` | 秒级 | 只改 `extension.js` 的环节（T-B08…T-B11） |
| **L3** Rust 单测过滤 | `cargo test -p sokonanoda-front <filter>` / `-p sokonanoda-lsp <filter>` | 十秒级 | 每个 Rust 环节 |
| **L4** **真 VS Code 单用例 e2e** | `scripts/vscode-e2e.sh --grep "<用例名>" --profile debug --no-build` | **十几秒**（debug 二进制 + 只跑一个用例） | **每个修复环节（DoD 第 4 步，必跑）** |
| **L5** 检查点全量 | `scripts/soko gate` + `scripts/vscode-e2e.sh`（全量） | 分钟级 | 只在 CP0…CP-K |

> **环境前置（2026-09-21 实测踩到）**：macOS 上若 Xcode 许可没同意，
> `xcrun --show-sdk-path` 失败 ⇒ **任何 Rust 链接都报**
> `error: linking with cc failed: exit status: 69`（不是代码问题）。
> 解法：`sudo xcodebuild -license accept`（正解），或
> `export DEVELOPER_DIR=/Library/Developer/CommandLineTools`（绕过，只影响本 shell）。
> `scripts/dev-loop.sh` 会检测并打印这条提示（不替你改环境）。

L4 的"十几秒"要靠三件事（都在 §0.5 的 T-015…T-017 里做）：
`--grep` 只跑一个用例、`--profile debug` 用 debug 二进制（不跑 release 构建）、
`--no-build` 在二进制比源码新时跳过构建与 stage。

### 0.5 本地 e2e：六条反馈的用例矩阵（**用户要求：确保功能真的修好了**）

#### 现状（盲区，实测）

- `editor/vscode/src/test/extension.test.js` 有 **18 个用例**，
  **没有一个读 Infoview 的声明栏 / 目标文本**（`:408-428` 只 smoke
  "命令存在且不抛"）；`grep -c 声明卡片` = 0。
- fixture 工作区 `src/test/fixtures/workspace/` **只有一个 README.md**——
  所有用例都把内联文本写进去，**没有任何 `import` 项目夹具** ⇒
  线 B/A/C 的核心场景（项目文件）在 e2e 里**根本无法出现**。
- `testApi`（`extension.js:1834-1838`）只暴露
  `{ goals, project, course }`，**不含 Infoview provider** ⇒ 测试拿不到
  "面板收到了什么"。
- `scripts/vscode-e2e.sh` 每次都 `cargo build --release` + stage + **跑全量**，
  分钟级 ⇒ 不可能每环节跑。

#### T-015 项目夹具：一个最小的 import 项目

- **改什么**：新增 `editor/vscode/src/test/fixtures/workspace/`
  - `sokonanoda.toml`（模块根）
  - `lib/Set.sokonanoda`：`def Set (α : Type) : Type := α -> Prop`、
    `def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a`、
    `infix:50 " ∈ " => Set.mem`、`def Set.subset …`、`infix:50 " ⊆ " => Set.subset`
  - `units/u01.sokonanoda`：`import lib.Set` + 一个 `theorem … := by sorry`
    （带记法的目标）+ 一个已 checked 的声明
- **判据**：`bash docs/gaps/repro/G22-…sh` 把入口换成这个夹具也判红
  （即夹具真的复现了"项目入口 `soko/goals` 为空"）。
- **为什么**：不依赖 `courses/set-theory`（它会变、且慢），e2e 才能稳定。

#### T-016 测试可见的 Infoview 载荷（`testApi` 扩面）

- **改什么**：`extension.js:1834-1838` 的 `testApi` 增加 `infoview: infoviewProvider`；
  给 `InfoviewProvider` 加两个**只读**测试访问器：`lastDecls()`（最后一次 post 的
  `decls` 数组）与 `lastState()`（最后一次 post 的 `state` 载荷）。
  **生产路径零行为变化**（只是把已有字段读出来）。
- **判据**：`cargo test -p sokonanoda-cli --test extension` 的静态契约仍绿
  （若契约里断言了 `testApi` 的形状，同步更新）；
  `node editor/vscode/test-extension-host.js` 全绿。
- **为什么不能直接读 DOM**：VS Code 的测试 API 拿不到 webview DOM。
  **分工**：e2e 断言**载荷**（真 LSP → 真扩展），`test-webview.js` 断言**渲染**
  （载荷 → DOM）。两者合起来才是"面板真的显示了"。

#### T-017 `scripts/vscode-e2e.sh` 加三个开关（L4 层的前提）

- **改什么**：
  - `--grep <用例名>` → 透传 `SOKO_E2E_GREP`，`.vscode-test.mjs` 的 `mocha` 块加
    `grep: process.env.SOKO_E2E_GREP || undefined`
    （`@vscode/test-cli` 把 config 的 `mocha` 原样交给 Mocha，
    `node_modules/@vscode/test-cli/out/runner.cjs:12-17`）；
  - `--profile debug|release`（默认 release）→ `cargo build [-p …]` 不带/带 `--release`
    + `node scripts/stage-lsp.js --profile <p>`（`scripts/stage-lsp.js:52-75` 已支持）；
  - `--no-build` → 若 `bin/<target>/sokonanoda-lsp` 比 `crates/` 下任何 `.rs` 新，
    跳过构建与 stage（否则打印原因并照常构建）。
- **判据**：
  ```bash
  scripts/vscode-e2e.sh --grep "<既有用例名>" --profile debug --no-build
  # 期望：只跑 1 个用例、通过、总耗时十几秒；台账照常追加一条
  scripts/vscode-e2e.sh --grep "<不存在的名字>"
  # 期望：0 个用例、exit 0（或明确报"没有匹配用例"），不静默全跑
  ```
- **风险**：`--no-build` 可能让你测到旧二进制 ⇒ 台账里已有 `lsp_sha256_16`
  与 `dirty`，**判读时必须看这两列**；`--no-build` 命中时要打印
  `bin/` 里那份二进制的 mtime 与 hash 前 16 位。

#### T-018 六条反馈的 e2e 用例（矩阵）

> **范围说明**：矩阵九条里，#1/#2 在 T-B13/T-B15、#6 在 T-C50、#7/#8 在 T-D40、
> #3/#4/#5 在 T-A60、#9 在 CP-K——**本条只负责把它们写成 e2e 并确认"修复前是红的"**，
> 各条的详细规格在它自己那一节。已落地的五条（#1/#2/#6/#7/#8）实测**全部判红**，
> 失败原因都正确（不是夹具/环境问题）。

每个用例都写在 `editor/vscode/src/test/extension.test.js`，
用 T-015 的夹具，断言走 T-016 的载荷或标准 VS Code API：

| # | 用例名 | 断言（真 VS Code + 真 LSP + 真扩展） | 属于 |
|---|---|---|---|
| 1 | `declarations panel lists a project unit's declarations` | 打开 `units/u01`（含 `import` + 库记法）⇒ `infoview.lastDecls().length > 0`，且与 `goals.declItems.length` 一致 | 线 B（T-B13） |
| 2 | `next hole jumps inside a project unit` | `sokonanoda.nextHole`（`alt+n`）后活动编辑器选区落在洞的 range 内 | 线 B（T-B06） |
| 3 | `reopening a project unit hits the compile cache` | ① 冷开记 `t1`；② `sokonanoda: restart server` 后重开同一文件记 `t2`；断言 `t2 < t1 / 3`，并把 `t1`/`t2` 写进 e2e 台账 | 线 A（T-A14） |
| 4 | `saving an unchanged project unit does not recompile` | 保存后 `infoview.lastDecls()` 与保存前逐字节相同，且**没有**新的编译发生（用 T-016 的编译计数或 cache 目录条目数不变） | 线 A（T-A22） |
| 5 | `editing a dependency refreshes the open unit once` | 改 `lib/Set` 一行 ⇒ 打开的 `units/u01` 诊断更新，且只发一份 | 线 A（T-A23） |
| 6 | `goal text uses the file's notation` | 光标落在 `sorry` 上 ⇒ `infoview.lastState().goal` 含 `∈` 或 `⊆` | 线 C（T-C24） |
| 7 | `go to definition on a notation symbol lands on its declaration` | `vscode.executeDefinitionProvider` 在 `∈` 上返回非空，uri 指向 `lib/Set.sokonanoda` | 线 D（T-D40） |
| 8 | `hover on a notation symbol shows the target's signature` | `vscode.executeHoverProvider` 的 markdown 含 `Set.mem` 的签名 | 线 D（T-D40） |
| 9 | `kernel speedup keeps the course gate counts` | （批次 5）`courses/set-theory` 的一个单元打开后诊断与计数不变；耗时对比写台账 | 线 K（CP-K） |

- **判据**：每个用例都**先在修复前跑一次确认是红的**（写进 commit message），
  修复后转绿；全量 `scripts/vscode-e2e.sh` 从 18 → 27 个用例全绿。
- **注意**：用例 3/4 是**时间/计数**断言，按 `docs/PERF.md` 的纪律标定
  （best-of-N、阈值取"最慢受支持 runner 的实测分布"再留余量；
  数字同时进台账供人判读，不只看 pass/fail）。

#### T-019 e2e 进 CI 与台账

- **改什么**：`ci.yml` 的 `e2e` job 已经跑全量；新增用例自动纳入。
  `docs/E2E.md` 补一节「单用例快跑（`--grep --profile debug --no-build`）」；
  `docs/vscode-dev-guide.md` §3 的测试三层表把 L4 写进去。
- **判据**：`ci.yml` 未改（新增用例自动被 `files: "src/test/**/*.test.js"` 收走）；
  `docs/E2E.md` 与 `docs/vscode-dev-guide.md` 同步。

### 0.6 性能检测：每步都要有数字，且**退化必须被机械拦住**

#### 能复用什么（现状，实测）

| 设施 | 位置 | 作用 |
|---|---|---|
| `PERFJSON {…}` 记录 | 各 perf 测试 `println!`（schema `soko.perf/1`，字段 `case`/`scope`/`ms`/`best_ms`/`worst_ms`） | 机器可读的实测值 |
| 台账 | `scripts/perf-ledger.sh` → `docs/perf/ledger.jsonl`（`soko.perf-ledger/1`，带 `version`/`commit`/`dirty`/`cli_profile`/`host`/`records`） | 跨版本/跨机器对比 |
| 人读报告 | `scripts/perf-report.sh`（CI 的 "Performance report" 步骤同口径） | 这一版多少 |
| 阈值断言 | **写在测试内部**（`crates/front/tests/perf.rs`、`perf_project.rs`、`crates/lsp/src/tests/perf.rs`、`crates/cli/tests/perf_project.rs`） | 只抓**算法级**回归（O(n²)、意外重复编译） |
| 测量纪律 | `docs/PERF.md`（串行 `--test-threads=1`、best-of-N、阈值按**最慢受支持 runner 的实测分布**标定、优先比 `best_ms`、±25% 内算同档、先 `warm_up()` 打掉 macOS 首次 spawn 的 ~425ms） | 数字可比 |

#### 五个缺口（实测，都要补）

1. **没有回归比较器**：`docs/PERF.md` 只写了人肉规则（"优先比 `best_ms`、±25% 内算同档"），
   **没有任何脚本读 ledger 判"这一版哪条退化了"** ⇒ 109 个环节里"修 A 弄慢 B"
   没有任何机械拦截。**这是本节最值钱的一条。**
2. **夹具比真实课程小 1–2 个数量级**：ledger 的 `scope` 只有
   `front-project` / `lsp-project` / `cli-project`，**没有 editor-host、没有真实课程闭包**；
   实测 unit01 **1.8s** / unit08 **4.6s** / unit12 **8.5s** / unit12 解答 **36.1s**，
   而仓库既有数字是 12–14ms（§2.1）。
3. **`by` 块判定没有独立场景**：36.1s 的大头（`judge.rs:257-263`）在现有套件里量不到。
4. **`soko/project` 不在记录里**：`docs/PERF.md:145` 只记了 goals + stateAt
   （0.58.0 新增的 `soko/project` 漏了）。
5. **没有"单场景快跑"入口**：`perf-ledger.sh` 跑全部四个套件（分钟级），不能每环节跑。

#### T-020 真实课程 + 编辑器路径的 perf 套件（**升级 T-007**）

> **进度（2026-09-21）**：已落地 `crates/lsp/src/tests/perf_course.rs`，
> 现有 **4 个可测 case**：`did_open`（unit01/08/12 三条记录）、
> `did_open_same_session`、`keystroke`、`by_block_did_open`（慢例，opt-in）。
> **还差两条**，它们量的就是修复本身，所以归到各自的环节：
> `did_open_warm_cache`（要 T-A10 先把 LSP 接上缓存）与
> `save_no_recompile`（要 T-A22 先有"文本未变即短路"）；
> `lsp_project_request` 已在 `perf.rs` 的 `perf_project_requests_are_interactive` 里覆盖。

- **改什么**：新增 `crates/lsp/tests/perf_course.rs`（或 `crates/lsp/src/tests/perf_course.rs`），
  用**真实课程闭包**做夹具（把 `courses/set-theory/units/unit01|unit08|unit12` +
  它们的 `lib/*` 复制成临时目录，或直接以仓库为根），量这些场景并打 `PERFJSON`：
  | case | 量什么 | 修前基线 |
  |---|---|---|
  | `course_did_open` | 冷开一个项目单元（`didOpen → 诊断`） | 1.8s / 4.6s / 8.5s |
  | `course_did_open_warm_cache` | **独立进程**冷开同一文件（缓存热） | 待测（目标 <100ms） |
  | `course_keystroke` | 一次按键 | 121–476ms |
  | `course_by_block` | 含 25 个 `by` 的解答编译 | **36.1s** |
  | `course_save_no_recompile` | 保存同一文本 | 待测（目标 ≈0） |
  | `course_fanout_dependency_edit` | 打开 5 个文档后改 `lib` 一行的扇出总耗时 | 待测 |
  | `lsp_project_request` | `soko/project` 一次请求（补缺口 4） | 待测 |
- **阈值**：按 `docs/PERF.md` 纪律标定——**只抓算法级回归**，绝对阈值宽松
  （CI runner 有 4× 实例方差，见 `docs/PERF.md` 的 300ms→800ms 教训），
  串行 + best-of-N。
- **判据**：
  ```bash
  scripts/perf-ledger.sh
  python3 -c "import json;[print(r['case'],r.get('best_ms',r.get('ms'))) for r in json.load(open('docs/perf/latest.json'))['records'] if r['scope']=='lsp-course']"
  # 期望：7 条新记录，且 course_by_block ≈ 36.1s（修前基线）
  ```
- **依赖**：T-015（夹具可复用）。

#### T-021 `scripts/perf-check.sh --case <name>`（单场景快跑，**每环节用**）

- **改什么**：跑**一个** `case`（`--case` 过滤 + `--test-threads=1` + best-of-N），
  打印 `case / best_ms / worst_ms / 上次台账的 best_ms / 变化%`。
- **判据**：
  ```bash
  scripts/perf-check.sh --case course_did_open
  # 期望：十几秒内出数字 + 与台账上一版的对比；--case 不存在时明确报错、不静默全跑
  ```
- **为什么**：这是"每次少做点快速反馈"在性能侧的对应物——改完立刻知道快了没有、
  以及有没有把别的拖慢。

#### T-022 `scripts/perf-compare.py`（**回归比较器**，本节最值钱的一条）

- **改什么**：读 `docs/perf/ledger.jsonl`，取两条记录（`--since <commit|版本|条目序号>`，
  默认"上一条"），按 `(scope, case)` 配对，输出一张表并按规则判红：
  - `best_ms` 退化 **> 25%** ⇒ **红**（`docs/PERF.md` 的 ±25% 规则，从人肉变机械）；
  - 新增的 case ⇒ 提示"无基线"，不算红；
  - 消失的 case ⇒ 红（"悄悄删掉一个哨兵"是最隐蔽的回归）；
  - 宿主不同（`host.system`/`machine`/`cli_profile` 不同）⇒ 只提示不可比，不算红。
- **判据**：
  ```bash
  python3 scripts/perf-compare.py --self-test    # 用造的台账验证四条判定规则
  python3 scripts/perf-compare.py                # 对真实台账跑，exit 0/1
  ```
- **风险**：**别把它做成噪声机器**。阈值 25% 是 `docs/PERF.md` 已有的经验值；
  若某 case 在 CI 上反复假红，按纪律"先做采样口径、再做二进制对拍，两步都排除掉
  再谈放宽阈值"（`docs/PERF.md` 的 800ms 教训）。
- **依赖**：T-020。

#### T-023 性能进 DoD 与 CI

- **改什么**：
  - **DoD 加一步**（§0.3 第 7 步）：跑本环节相关场景的 `scripts/perf-check.sh --case …`，
    数字贴进 commit message；跑 `python3 scripts/perf-compare.py --since <上一检查点>`，
    **任何 case 退化 > 25% 必须解释或修回**；
  - **纯性能环节**（线 K）把"数字下降 X%"直接写成判据；
  - `scripts/perf-ledger.sh` 纳入 T-020 的新套件；CI 的 "Performance report" 步骤
    自动带上（同一脚本）；
  - `docs/PERF.md` 新增「性能回归怎么判」一节，把 T-022 的规则写成判据。
- **判据**：`scripts/perf-ledger.sh` 一次跑完包含新套件；
  `docs/PERF.md` 里能查到 T-022 的四条规则；CI 的 perf artifact 里出现新 case。

#### 性能的"修前基线"（已实测，见 §2.1）

| 场景 | 修前 |
|---|---|
| `lib/Set`（2 模块） | 1.98s |
| `units/unit08`（5 模块） | 4.59s |
| `units/unit12`（8 模块） | 8.49s |
| `units/solutions/unit12-solution`（8 模块、25 个 `by`） | **36.13s** |
| 无 `import` 的课程文件（走单文件缓存） | ~0.10s |

> 这张表就是批次 2/5 的"修前"，也是 CP-A/CP-K 的对照物。

### 0.7 肉眼看效果（用户自己的 VS Code 窗口，**不用重装 VSIX**）

- **Rust 侧改动（线 A/C/D/K 的绝大多数）**：一次性在用户设置里开
  `sokonanoda.serverOverride: true` + `sokonanoda.serverPath` 指向仓库的
  `target/debug/sokonanoda-lsp`（`editor/vscode/extension.js:105-121` 的解析链：
  override 打开时 `setting` → `env` 优先，bundled 被绕过）。之后每个环节只要：

  ```bash
  scripts/dev-loop.sh lsp        # cargo build -p sokonanoda-lsp -p sokonanoda-cli（debug）
  # VS Code 命令面板 → sokonanoda: restart server   # 走与激活同一条解析链，不重载窗口
  ```

  **限制**：这只换服务器二进制，**换不了扩展代码** ⇒ 线 B 的客户端部分
  （T-B08…T-B11）在这个回路里看不到，要用 F5。
- **扩展侧改动**：`editor/vscode` 按 **F5**（开发宿主；`bin/` 不存在时落到
  `target/debug`），改 `extension.js` 按重载按钮。要回到这条路先
  `npm run clean:lsp`。
- **要装成真扩展**（只在检查点）：`npm run package:host` +
  `code --install-extension sokonanoda.vsix --force` + Reload Window。

---

## 1. 六条反馈 → 根因（一句话版）

| # | 用户反馈 | 根因（实测，见 §2） | 所在层 |
|---|---|---|---|
| 1 | 编译很慢 | 项目文件在 LSP 上**每次动作都从零编译整个 import 闭包**：release 实测 unit01 **1.8s** / unit08 **4.6s** / unit12 **8.5s** / unit12 解答 **36.1s**；编译期间**整把文档锁被占住**，hover/goals/stateAt 全部排队 | LSP + front |
| 2 | 没有实现编译后的文件加速 VS Code | **LSP 对含 `import` 的文档既不读也不写任何缓存**（`crates/lsp/src/lib.rs:152-156`，`grep digest crates/lsp` = 0）；项目缓存只活在 CLI crate（`crates/cli/src/project_cache.rs`），且形状是「一闭包一条、只存入口模块」（`crates/front/src/query/mod.rs:215` 拿到它就把 `project` 置 `None`）——LSP 需要的**逐模块报告**根本不在里面 | LSP + CLI |
| 3 | 新打开一个文件就有临时编译 | ① 每份 `Doc` 各编一份闭包，文档之间零共享（打开 `lib/Set` 编 {Logic,Set}，打开 unit08 又编 {Logic,Exists,Set,Image,unit08}）；② 项目模式**先白编一遍入口单文件**再编闭包，结果被丢弃（`crates/front/src/query/mod.rs:117` vs `:135`/`:144-151`）；③ `Doc::set_text` **没有"文本没变就返回"的短路** ⇒ 保存 / 编辑器外改动会重编同一文本；④ 项目缓存**永不写**：只写"完全干净"的项目（`is_clean` 要求每个模块 `warnings.is_empty()`），而 `sorry` 是 WARNING | LSP + front |
| 4 | Infoview「声明」栏经常失效，unit 教学文件没有 | `QueryDoc::goals` 被 `parsable()` 挡住（`crates/front/src/query/mod.rs:481` → `:403-408`）；项目入口单文件 parse **必然**失败（记法来自 `import`）⇒ `Err(NotParsable)` ⇒ LSP `.unwrap_or_default()`（`crates/lsp/src/lib.rs:495`）⇒ `decls: []`。**同一条 G-20 补丁 `check()` 与 `Doc::set_text` 都打了，只有 `goals` 漏了** ⇒ 目标栏好、声明栏空（正是用户看到的不对称）。同族：`holes`/`nextHole`（`alt+n` 跳洞）全死 | front + LSP |
| 5 | goal 展现没有用 notation | **goal 文本有四个生产者**，不是一个：根状态与声明列表走**内核 pp**（必然点名）、无 `by` 的开练习走 `render_expr`（**保留**记法）、`by` 步进走 `render_expr` 但一旦经 `canonical_goal*`/`apply`/`cases` 就换成 pp 文本再 parse 回来（丢）。实测同一份数据 `goal = "(a ∈ A) -> a ∈ B"` 而 `ty = "… Set.mem α a A …"` | front |
| 6 | 记法不能跳转、hover 没有原始类型 | 记法使用处在 elab 里**硬编码** `resolution: None`（`crates/front/src/compile/elab.rs:2914`），而 `definition_at` **就是** `hover_type_at(…).resolution`（`crates/lsp/src/render.rs:306-314`），没有第二条路 ⇒ `definition` = `null`；hover 的 `target` 只认**本文件**声明的记法（`notation_input.rs:261-267`）⇒ import 来的 `∈` 连"展开成"都没有；而 `Set.mem` 的签名其实**已经算过又被丢掉**（`elab.rs:1228` 的 `judge::judge_type_of_constant`，**该函数是 `pub`**） | front + LSP |

**一句话**：1/2/3 是**同一个缓存故事**；4 是**一个判据漏打补丁**；
5 是**渲染层按生产者分裂**；6 是**记法表没有对编辑器暴露**。

---

## 2. 实测底账（全部可复现；这是"修前"基线）

环境：Apple Silicon，`target/release/sokonanoda-lsp` 与
`target/release/sokonanoda`（`[profile.release]` opt-level 3 + fat LTO，
**与扩展内置的那份同 profile**），仓库 `0.63.0`，`git status` 干净。

### 2.1 LSP：打开文件的分阶段耗时（探针手法见 T-006）

| 文件 | import | spawn+init | **didOpen → 诊断** | 同文本再 didChange |
|---|---|---|---|---|
| `courses/set-theory/units/unit01-sets-membership.sokonanoda` | 2 | 40ms | **1781ms** | 121ms |
| `courses/set-theory/units/unit08-images-preimages.sokonanoda` | 4 | 38ms | **4599ms** | 360ms |
| `courses/set-theory/units/unit12-synthesis.sokonanoda` | 7 | 43ms | **8465ms** | 476ms |
| `course/unit1-propositions-proofs.sokonanoda`（无 import） | 0 | — | **~100ms**（整轮进程+打开） | — |

同一条 `QueryDoc` 路径的 CLI 口径（`SOKONANODA_NO_CACHE=1`，release）：

| 文件 | 闭包模块数 | 编译 |
|---|---|---|
| `courses/set-theory/lib/Set.sokonanoda` | 2 | **1.98s** |
| `units/unit08-images-preimages.sokonanoda` | 5 | **4.59s** |
| `units/unit12-synthesis.sokonanoda` | 8 | **8.49s** |
| `units/solutions/unit12-solution.sokonanoda`（520 行、25 个 `by`） | 8 | **36.13s** |

`query goals` / `query goals --probe` / `query state` 在同一文件上都是 4.59–4.60s
⇒ **编译主导**，查询与探针本身测不出增量。

> 同一份文档在同一 LSP 会话里再改（内容相同）只要 121–476ms，说明 `Session`
> 的"内容未变零重编译"是好的；**贵的是"第一次打开一份新文档"**——正是用户说的
> 「新打开一个文件就有临时编译」。
>
> **为什么仓库既有 perf 数字是 12ms**：`docs/PERF.md` 的夹具是 2–5 模块 ×
> 10–12 声明，比 `courses/set-theory` 小 **1–2 个数量级**
> （`docs/perf/ledger.jsonl` 的 scope 只有 `front-project`/`lsp-project`/`cli-project`，
> **没有任何 editor-host 或真实课程闭包的条目**）。⇒ T-006 必须补真实课程夹具。

### 2.2 缓存：项目缓存对教学文件**完全不生效**（根因已实测更正）

```bash
$ ./target/debug/sokonanoda build courses/set-theory/units/unit08-images-preimages.sokonanoda
built 1 file(s) — 0 hit, 1 compiled, 0 failed     # 再跑一遍仍是 0 hit

$ ./target/debug/sokonanoda query check --file courses/set-theory/units/unit08-images-preimages.sokonanoda
real 4.873s                                        # 与冷跑一样慢 ⇒ 根本没写进缓存
```

**根因（2026-09-21 实测更正 + 用户判定）**：
**`courses/set-theory/sokonanoda.toml` 的 `requires = "0.61"` 与二进制 0.63.0 漂移**
——而**开发过程没有自动提升项目清单的 `requires`**；这门课与语言仓**同仓共同开发**，
版本本来应当一致。

机制：`version_warning`（`crates/front/src/project/manifest.rs:135-144`，只比
**major.minor**）⇒ `ProjectPlan.requires_warning = Some(…)`
（`crates/front/src/project/mod.rs:235`）⇒ `ProjectReport::is_clean()`
（`crates/front/src/project/report.rs:158-166`）为假 ⇒ `store_if_clean`
（`crates/cli/src/project_cache.rs:33-41`）**永不写缓存**。

实测对照（同一份 3 行项目、隔离缓存、`build` 连跑两次）：

| 清单 | 第二次 build |
|---|---|
| `requires = "0.61"` | **0 hit**（漂移） |
| `requires = "0.1"` | **0 hit**（漂移） |
| `requires = "0.63"` | **1 hit**（匹配） |
| 无清单 | **1 hit** |

> **最初写在这一节的根因（"`sorry` 是 WARNING ⇒ 永不入缓存"）是错的**：
> `sorry` 只体现为 `exercise_open` 计数，**不在** `module.report.warnings` 里；
> 实测"入口带 `sorry`、无清单"的项目**能正常入缓存**。G-24 的复现脚本把这条
> 更正钉死了（含对照组）。

**顺带发现（同一片代码）**：两条写缓存路径**规则不一致**——`build`/`check` 走
`store_if_clean(…, project.is_clean())`（`crates/cli/src/build.rs:136`、
`crates/cli/src/check.rs:83`），而 `query` 走 `set_cached_entry`
（`crates/front/src/query/mod.rs:214`）**不看 `is_clean`** ⇒ 命不命中取决于
先跑了哪条命令。

**另一条仍成立的设计分叉**：设计 §4.8（`docs/design/imports-and-projects.md:594-595`）
写的是"按模块存"，实现是"一闭包一条、只存入口"——LSP 需要的逐模块报告不在里面。

### 2.3 三个"疑似键/形状不对"的点（**需要实验，见 §11 问题 5**）

1. **`build_stamp()` = `current_exe()` 的 mtime（秒级）被折进缓存键**
   （`crates/front/src/compile/cache.rs:67-79,88-96`）。CLI 与 LSP 是**两个不同的
   可执行文件** ⇒ `sokonanoda build` 预热的条目能否被 LSP 命中，取决于两个二进制
   的 mtime 是否落在**同一秒**。仓库里 staged 的那对恰好同秒，是"同一次 build +
   同一次 stage"的巧合，**不是设计保证**（release 走 upload/download-artifact +
   两份 tarball）。**这条不解决，线 A 的收益可能为 0。**
2. **`CachedCompile` 只含入口模块** ⇒ 即使接上，跨文件 `definition`/`references`/
   `rename`、`soko/project` 的模块表、扇出判定（都读 `project_modules()`）仍然没数据。
3. **`--text` 中间态不缓存**（`docs/design/compile-cache.md` §5 有意为之）。

### 2.4 `soko/goals`（声明栏唯一数据源）：项目入口恒为空

LSP 探针（`initialize(rootUri=仓库根)` → `didOpen` → `soko/goals` + `documentSymbol`）：

| 文件 | 诊断 | `soko/goals` 的 decls | `documentSymbol` |
|---|---|---|---|
| `courses/set-theory/units/unit01-sets-membership.sokonanoda` | 6 × `sorry` | **0** | 8 |
| `courses/set-theory/units/unit08-images-preimages.sokonanoda` | 9 × `sorry` | **0** | 27 |
| `courses/set-theory/units/notation-cheatsheet.sokonanoda` | 3 × `sorry` | **0** | 23 |
| `course/unit1-propositions-proofs.sokonanoda`（无 import） | 6 × `sorry` | **13** | 13 |

报告本身是好的（`documentSymbol` 有 8/27/23 项），只是 `goals` 这条出口被判死。
`soko/stateAt` **正常**（实测 unit01 第 45 行 `sorry` 处 `decl.name = "mem_of_subset"`）
⇒ **目标栏好、声明栏空**，与用户描述完全一致。

### 2.5 goal 文本：四个生产者，两种行为

| 生产者 | 谁在用 | 文本来源 | 记法 | 实测 |
|---|---|---|---|---|
| **根状态**（`step:-1`，光标在 `by`/`sorry`） | `soko/stateAt` 目标栏 | `DeclState.ty_text` = **内核 pp**（`crates/front/src/compile/check/kernel_phase.rs:170-178`/`:207-213` → `pp.pp_expr`） | **丢** | unit01:45 → `forall (α : Type 0) (A B : Set α), Set.subset α A B -> …` |
| **无 `by` 的开练习** | `soko/goals` 的 `decl.goal` | `render_expr(ty)`（`crates/front/src/compile/goals.rs:1088/1230/1350/1452`） | **保留** | unit01 同一声明 → `"(a ∈ A) -> a ∈ B"` |
| **声明列表的 `ty`** | 声明卡片 | 同根状态（内核 pp） | **丢** | `ty = "forall (α : Type 0) …"` |
| **`by` 步进** | 目标栏进度 | `ByGoal.ty = render_expr(&nodes[id].ty)`（`crates/front/src/by.rs:1162`，`have` 在 `:1858`） | 多数保留 | 解答 `:10`（`intro ha` 后）→ `a ∈ B`；`:21`（`apply h` 后）→ `Set.singleton α a a`（**丢**） |

> **光标位置决定走哪一支**（2026-09-21 在真 VS Code 里逐列量过，见 G-26 的 `today`）：
> 对 `theorem … := by` + `sorry` 的声明——
> **光标在 tactic 内**（半开区间 `start <= cursor < end`）⇒ **根状态**（内核 pp，**点名**）；
> **光标在 tactic 之后** ⇒ 走"无 `by` 的声明级目标"那一支（`render_expr`，**记法保留**）；
> `by` 行内 ⇒ 根状态 ⇒ 点名。
> ⇒ 学习者写证明时光标就在 tactic 上，**看到的就是点名形式**——这正是用户的抱怨。
> **写 e2e 时注意**：`revealRange` 把光标停在 range **末尾**（= 之后）⇒ 会假绿；
> 必须显式设 `editor.selection`（`extension.test.js` 的用例 #6 就是这么写的，并踩过一次）。

**`by` 引擎里把源 AST 换成 pp 文本再 parse 回来的位置**（设计文档没记这条）：

- 根目标规范化（**只在单元用了 `namespace`/`open`/`export`/`open … in` 时**）：
  `by.rs:760-763` → `canonical_goal_type`（`by.rs:497`）→ `judge::judge_render_type`
  （`crates/front/src/judge.rs:897`）；开关 `CmdCtx.canonical_goal`
  （`walk.rs:100-115`/`:162`）。
- `intro` 遇到 def-headed 目标（`A ⊆ B`、`¬ A`、`∅ ⊆ A`）：`by.rs:839` →
  `canonical_goal_with_spec`（`by.rs:553`），剥出来的 `rest` 成为新目标（`by.rs:901`）。
- `apply`：`f_ty` 来自 `judge_infer` 的 pp 文本（`by.rs:1277`），子目标是这份
  pp 望远镜上的 `substitute`（`by.rs:1413`）。
- `cases`：scrutinee 类型来自 `judge_infer` 的 pp（`by.rs:1450+`）。

### 2.6 记法 hover / 跳转

`textDocument/hover` 在 unit01 第 32 行的 `∈` 上：

```
`∈` —— 记法符号

输入：`\in`（别名 `\mem`）

`a ∈ A : Prop`
```

* **有**输入法提示；**没有**"展开成 `Set.mem`"；**没有** `Set.mem` 的原始类型；
* `textDocument/definition` = **`null`**；`documentHighlight` = **`null`**。

### 2.7 顺带发现的四条（不在用户六条里，但同一片代码）

1. **`documentHighlight`/`references`/`rename` 在记法符号上会误解析到外层 binder**：
   记法行的 `resolution` 是 `None`，这三个 handler 有"回退到包含光标的任意 hover 行"
   的逻辑（`crates/lsp/src/render.rs:327-331`、`crates/front/src/references.rs:57-60`），
   而 binder 的 span 覆盖整段类型标注（`parser.rs:2950-2966`）⇒ 光标在 `(h : a ∈ A)`
   的 `∈` 上会被解析成 `h`，`rename` 可能改掉 `h`。**`definition` 反而因为"没有回退"
   而返回 `null`**——四个 handler 行为不一致，是构造性的。→ T-D30。
2. **`semantic::tag_runs` 从不填 `Names::notations`**（`crates/front/src/semantic.rs:240`，
   而 `semantic::classify` 在 `:544` 是填的）⇒ ① 目标文本里的 `∈` 那一 run
   **没有 `kind`**（不着色）；② `Set.mem`/`Set`/`α` 全被标成 `unknown_ident`，
   因为 `decl_kinds()` = `declaration_kinds(&self.text)`（`query/mod.rs:274`）
   **只看入口文件**。→ T-C30。
3. **`position_to_offset` 按 `char` 计数，不是 LSP 要求的 UTF-16 码元**
   （`crates/lsp/src/lib.rs:615-629`，`docs/protocol.md:806-811` 已记为独立缺口）。
   课程里唯一星平面符号是 `𝒫`（U+1D4AB）⇒ `𝒫` 之后同一行的 hover 会偏。
   测试夹具刻意镜像了服务端的数法（`testutil.rs:38-47`），**仓库内测试抓不到**。→ T-D31。
4. **`soko/project` 的 `project_view_reason()` 每次重新 parse 整份文本**
   （`crates/front/src/query/project.rs:36`），而扩展每轮诊断都问一次。→ T-A24。

---

## 3. 阶段 0：先建判据（不改产品代码）

> 目的：让后面每个环节都"可判红、可判绿"，也让用户有一条命令看全貌。
> **零产品代码改动**，可以立刻做、立刻验收。

### T-001 六条反馈（+2 条顺带发现）落缺口台账

- **改什么**：`docs/gaps/ledger.jsonl` 追加 6 条（顺延 `G-22`…`G-27`），每条带
  `today`（§2 实测行为）、`repro`、`repro_expect`、`owner`。
- **判据**：`python3 scripts/gap.py list | tail -8` 看到 6 条；`python3 scripts/gap.py check; echo $?` → `0`。
- **测试层**：`scripts/gap.py selftest`。

### T-002 复现脚本：声明栏（LSP 探针）

- **改什么**：`docs/gaps/repro/G22-lsp-goals-empty-for-import-entry.{sh,js}` ——
  起 LSP、`didOpen` `units/unit08-images-preimages.sokonanoda`、发 `soko/goals`，
  断言 `decls.length > 0` 且与 `documentSymbol` 条数一致。外壳只认 `.sh`，
  `exec node` 到隔壁 `.js`（样板 `G20-lsp-drops-rescued-report.{sh,js}`）。
- **判据**：`bash docs/gaps/repro/G22-…sh ; echo "exit=$?"` → 修前 `0`。

### T-003 复现脚本：记法跳转 + hover 原始类型（LSP 探针）

- **改什么**：`docs/gaps/repro/G23-notation-navigation.{sh,js}` —— 在 unit01 的 `∈` 上断言
  ① `definition` 非 `null`；② hover 含 `Set.mem` 的签名行。

### T-004 复现脚本：`requires` 漂移关掉项目编译缓存（CLI）

- **改什么**：`docs/gaps/repro/G24-project-cache-never-warms.sh` —— 用**独立
  `SOKONANODA_CACHE_DIR`**（隔离；样板 `crates/cli/tests/perf_project.rs`）：
  ① 冷跑 `query check` 记 `t_cold`；② `build`；③ 再 `query check` 记 `t_warm`；
  断言 `t_warm < t_cold / 3` 且 `build` 输出 `hit ≥ 1`。
- **注意**：复现一律在中性环境跑（`gap.py` 会剔除 `SOKONANODA_BIN`/`_LSP_BIN`），
  缓存目录自己 inline 指定，别用开发者的真实缓存。

### T-005 复现脚本：LSP 每次打开都重编（两个独立 LSP 进程）

- **改什么**：`docs/gaps/repro/G25-lsp-recompiles-on-every-open.{sh,js}` ——
  **两个独立 LSP 进程**分别 `didOpen` 同一文件，记 `t1`/`t2`，断言 `t2 < t1 / 3`。
- **为什么必须两个进程**：同一进程里 `Doc`/`Session` 会保留，`t2` 本来就快
  （实测 121–476ms），测不出问题；"重启编辑器后重开文件"才是用户场景。
- **未证实**：`didClose` 之后 `Doc` 是否被移除——写脚本时先验证。

### T-006 复现脚本：goal 文本丢记法（CLI）

- **改什么**：`docs/gaps/repro/G26-goal-text-loses-notation.sh` —— 对 unit01 的
  `sorry` 处 `query state`，断言 goal 含 `∈` 或 `⊆`；并断言 `query goals` 的
  `ty` 含 `⊆`/`↔`（两个 surface 分开断言，见 §2.5）。

### T-007 编辑器路径性能台账（把 §2.1 变成可重复的哨兵）

> **完整版见 §0.6 的 T-020**（七条 case，含 `course_by_block` 与 `soko/project`）；
> 本条是它的最小内核：先让 `didOpen → 诊断` 有一个可重复的哨兵。
> **与 T-021/T-022 一起做**——只有哨兵没有比较器，等于没有回归拦截。

- **改什么**：新增 `crates/lsp/tests/perf_course.rs`：**真实课程夹具**——
  把 `courses/set-theory/units/unit01|unit08|unit12` 的**闭包**（含 `lib/*`）
  复制成临时目录夹具，量 `didOpen → 诊断`，打 `PERFJSON`（`scope: "lsp-course"`）。
  阈值按 `docs/PERF.md` 纪律标定（串行 + best-of-N + "最慢受支持 runner 的实测分布"）。
- **判据**：
  ```bash
  scripts/perf-ledger.sh && tail -5 docs/perf/ledger.jsonl
  ```
  `docs/PERF.md` 新增「编辑器：真实课程闭包」小节，贴修前数字（§2.1）。

### T-008 一条人工验收命令

- **改什么**：`scripts/verify-editor-issues.sh` —— 顺序跑 T-002…T-006 的复现
  + 六条 e2e 用例（`--grep`）+ `python3 scripts/perf-compare.py`，
  打印一张状态表（缺口在 / 已修 / 环境异常 / 性能退化），exit 0 = 全部已修。

### T-009 实验：缓存键的 `build_stamp` 到底会不会让 CLI 预热对 LSP 失效

- **改什么**：不改代码。写 `docs/notes/cache-key-build-stamp.md` 记录实验：
  ① `stat -f %m` 比较仓库 `target/release/sokonanoda` 与 `sokonanoda-lsp`
  的 mtime；② 造一对 mtime 不同秒的二进制，跑 `build` 再跑 LSP `didOpen`，
  看是否命中；③ 检查 release 产物（tarball / VSIX 内嵌）是否同秒。
- **判据**：文档里给出"是/否失效"的结论 + 复现命令。
- **为什么先做**：**它决定线 A 是否白干**（§2.3 第 1 条）。

### T-010 文档与要求入账

- **改什么**：本文档进仓；`REQUIREMENTS.md` §9 追加本轮要求（2026-09-21）；
  `ROADMAP.md` §10 挂指针。

### T-011 环节循环脚本 `scripts/dev-loop.sh`

- **改什么**：一条命令完成"重建 → 告诉我在 VS Code 里按什么"：
  ```bash
  scripts/dev-loop.sh lsp          # cargo build -p sokonanoda-lsp -p sokonanoda-cli（debug）
                                   # + 打印：命令面板 → sokonanoda: restart server
  scripts/dev-loop.sh ext          # 打印 F5 开发宿主的步骤（bin/ 存在时提示 clean:lsp）
  scripts/dev-loop.sh stage-debug  # node editor/vscode/scripts/stage-lsp.js --profile debug
                                   # （打包/真宿主路径用；--profile debug 已支持，见脚本 :52-75）
  ```
  脚本只做**编排 + 提示**，不发明新机制。
- **判据**：`bash scripts/dev-loop.sh lsp` 跑完 exit 0，且打印的路径真实存在
  （`target/debug/sokonanoda-lsp`、`target/debug/sokonanoda`）。

### T-012 真 VS Code **单用例**跑法（L4 层的前提）

> 完整版见 §0.5 的 **T-017**（`--grep` / `--profile debug` / `--no-build` 三个开关）；
> 本条是它的最小内核：`.vscode-test.mjs` 的 `mocha` 块加一行
> `grep: process.env.SOKO_E2E_GREP || undefined`（`@vscode/test-cli` 把 config 的
> `mocha` 原样交给 Mocha，`node_modules/@vscode/test-cli/out/runner.cjs:12-17`）。
> **验收与其余两个开关一起在 T-017 里做**（分开做会出现"能 grep 但每次仍跑
> release 构建"的半成品）。

### T-013 版本纪律修订：bump = **发布边界**，不是 commit 边界

- **改什么**：`docs/vscode-dev-guide.md` §2 把"每次 commit 涉及 `editor/vscode/`
  必须 bump"改成：
  > **bump 发生在发布边界（批次收尾）**，不是每个 commit。开发期版本号保持不变
  > ⇒ 推 main 只跑 CI、不发版（`ci.yml:61-65` 的 `tag already exists` 分支）。
  > 唯一被 CI 强制的是 `Cargo.toml` 与 `package.json` **相等**；
  > 版本号只能是纯 `x.y.z`（`scripts/soko:110` 的解析正则；带后缀会让启动器按
  > G-16 拒绝运行）。
- **判据**：文档改完，`grep -n "bump" docs/vscode-dev-guide.md` 的说法与
  `ci.yml` 的实际行为一致；`crates/cli/tests/extension.rs` 的版本契约测试仍绿。
- **风险**：这条改动会**放宽**一条既有纪律 ⇒ 必须同时写清"什么时候必须 bump"
  （批次收尾、发 tag 前、以及"改了协议/事件计数"时）。

### T-014 把"肉眼看效果"的回路写进文档

- **改什么**：`docs/vscode-dev-guide.md` 新增一节「环节循环（快速反馈）」，
  写清 §0.4 的三条路（serverOverride 回路 / F5 开发宿主 / 打包安装）、
  各自的**能看到什么、看不到什么**（尤其"override 只换服务器，换不了扩展代码"），
  以及 `SOKO_E2E_GREP`。
  **不改 `editor/vscode/README.md`**：它是 Marketplace 门面（安装/功能/agent 卖点），
  没有开发向章节，往里塞开发回路是噪声——已核对（`README.md` 的章节列表里只有
  用户可见内容）。
- **判据**：按文档从头走一遍能复现（用户按文档操作能立刻看到 LSP 改动生效）。

### 性能检测基建（T-020…T-023，规格在 §0.6）

> 四条任务的完整规格在 **§0.6**，与 e2e 基建同属批次 0 的"判据与量具"：
> **T-020** 真实课程 + 编辑器路径的 perf 套件（七条 case）·
> **T-021** `scripts/perf-check.sh --case`（单场景快跑，每环节用）·
> **T-022** `scripts/perf-compare.py`（**回归比较器**，`best_ms` 退化 > 25% 判红）·
> **T-023** 性能进 DoD 与 CI。
> **没有 T-022 就不算有性能检测**——哨兵只告诉你"这一版多少"，
> 比较器才告诉你"是不是被上一个环节弄慢了"。

### ✅ 检查点 CP0

- [ ] `python3 scripts/gap.py check` exit 0
- [ ] `bash scripts/verify-editor-issues.sh` 打印 6 条全部「缺口在」
- [ ] `docs/perf/ledger.jsonl` 有真实课程闭包的"修前"行
- [ ] T-009 有明确结论
- [ ] **环节循环可用**：`scripts/dev-loop.sh lsp` + `restart server` 能让一个
      LSP 改动在用户自己的 VS Code 里立刻生效
- [ ] **e2e 基建就绪（§0.5）**：项目夹具（T-015）能被 T-002 的复现脚本判红；
      `testApi.infoview.lastDecls()` 可读（T-016）；
      `scripts/vscode-e2e.sh --grep … --profile debug --no-build` **十几秒**跑完一个用例（T-017）；
      九条新用例**全部先跑一次确认是红的**（T-018）
- [ ] **性能检测就绪（§0.6）**：`scripts/perf-check.sh --case course_by_block`
      出数字（≈36.1s）；`python3 scripts/perf-compare.py --self-test` 四条判定规则全过；
      `scripts/perf-ledger.sh` 含新套件；CI perf artifact 里有 `lsp-course` 的记录
- [ ] **用户验收**：确认 6 条缺口描述与自己的感受一致；确认"慢"主要发生在
      **打开新文件**还是**每次按键**还是**保存**（对应不同根因，见 §11 问题 2）

---

## 4. 线 B：Infoview「声明」栏 + `nextHole`（反馈 4）

> **为什么排第一**：最便宜、最显眼、根因最确定（一条判据漏打补丁），
> 顺带修好 `alt+n` 跳洞。

### 根因

- **B1** `QueryDoc::goals`（`crates/front/src/query/mod.rs:480`）第一行是
  `self.parsable()?`（`:481`），`parsable` 只看 `parse_error.is_some()`（`:403-408`）。
  项目入口单独 parse **必然**失败（记法来自 `import`）⇒ `Err(NotParsable)`。
- **B2** LSP 侧 `.unwrap_or_default()`（`crates/lsp/src/lib.rs:495`）把 `Err` 吞成 `decls: []`。
- **B3** **同一条判据，三个出口修了两个**：
  `QueryDoc::check()` 有例外（`.filter(|_| !self.project_entry_compiled())`，`:353-357`）；
  `Doc::set_text` 有例外（`crates/lsp/src/lib.rs:170-182`，G-20）；
  `goals` **没有**。判据本体 = `project_entry_compiled()`（`:246-251`）。
- **B4** 同族：`holes()`（`:621`）与 `next_hole()`（`:649`）都经由 `goals(true)`
  ⇒ **`soko/nextHole` / `alt+n` 在项目文件里全不可用**。`state_at()`（`:415`）用
  `report`，**正常**；`hints` 正常。
- **B5** 客户端另有 4 个独立缺陷（E4/E5/E6/E8，见 T-B10…T-B13）。
- **B6** 测试盲区：`crates/lsp/src/tests/project.rs:693-762` 的 G-20 回归夹具
  `NOTATION_CANVAS` **恰好复现本 bug**，但只断言诊断/hover/documentSymbol，
  **缺 `soko/goals` 那一条**；`editor/vscode/test-webview.js` 对空列表零覆盖
  （`暂无` 命中 0 次）；真宿主 e2e 从不读 webview DOM。
- **B7** CLI 看不见这个 bug：`crates/cli/src/query.rs:149-173` 走 `set_cached_entry`，
  而 `crates/front/src/query/mod.rs:214` 把 `parse_error = None` ⇒ 缓存热时**偶然正常**。

### T-B01 实测表：哪些文件坏、哪些好

- **改什么**：新增只读量具 `scripts/verify-decl-panel.py`；结果写进
  `docs/design/goal-list.md` as-built：13 `units/*` + 13 `units/solutions/*` +
  9 `lib/*` + 15 `course/*`，每个文件三列（单文件可 parse / `goals` 条数 /
  `documentSymbol` 条数）。
- **判据**：修前表里"单文件 parse 失败"的那些，`goals` 列全 0。

### T-B02 实测：`nextHole` / `stateAt` / `hints` 在项目入口的现状

- **改什么**：同一张表加 4 列。
- **判据**：明确写出"哪些坏"（预期 `nextHole`/`holes` 坏，`stateAt`/`hints` 好）。

### T-B03 收敛成单一判据 `QueryDoc::usable()`

- **改什么**：`crates/front/src/query/mod.rs` 新增

  ```rust
  /// 这份文档的**真相层**是否可用（与 `check()` / `Doc::set_text` 同一判据）：
  /// 单独 parse 失败但 import 闭包编译成功 ⇒ 可用（G-20 的语义）。
  fn usable(&self) -> bool {
      self.report.is_some() && (self.parse_error.is_none() || self.project_entry_compiled())
  }
  ```

  `goals` / `holes` / `next_hole` 改用它；`check()` 与 `set_text` 的重复判据也收敛到它
  （**一处判据、三处引用**，杜绝再次漏打）。
- **判据**：
  ```bash
  cargo test -p sokonanoda-front query:: -- --nocapture
  # 新增两条绿：入口 parse 失败 + 闭包成功 ⇒ goals 非空；闭包也失败 ⇒ 仍 NotParsable
  ```
- **风险**：**必须保住 G-17 契约**——"真的解析不了"仍要 `Err(NotParsable)`。
  两个分支都要有测试。

### T-B04 补 LSP 断言（夹具已经在了）

- **改什么**：`crates/lsp/src/tests/project.rs` 的
  `imported_notation_keeps_the_report_and_the_diagnostics_honest` 增加第 ④ 条：
  对 `NOTATION_CANVAS` 发 `soko/goals`，`decls` 非空且条数 = `documentSymbol` 条数。
- **判据**：`cargo test -p sokonanoda-lsp project:: -- --nocapture` 绿；
  **把 T-B03 回滚则该断言必须红**（判别性）。

### T-B05 复现转绿

- **判据**：`bash docs/gaps/repro/G22-…sh` → `1`；`python3 scripts/gap.py check` → `0`。

### T-B06 `alt+n` 跳洞在项目文件里可用

- **改什么**：T-B03 之后 `next_hole` 自然恢复；补一条 LSP 测试钉住。
- **判据**：`cargo test -p sokonanoda-lsp -- --nocapture` 新增用例绿。

### T-B07 CLI 侧的假绿也钉住

- **改什么**：`crates/cli/tests/query.rs` 补一条**冷缓存**用例（现在只有热缓存
  偶然正常）：对项目入口跑 `query goals`，`data` 非空。
- **判据**：新用例绿；在 T-B03 之前必须红。

### T-B08 客户端 E4：请求期间切文档 ⇒ 新文档必须补取数

- **根因**：`extension.js:362` `if (requestedUri !== this.uri) return;` —— 提前返回，
  既不建 `declItems` 也不推 `onDecls`，面板停在上一份文档。
- **改什么**：过期答案丢弃之后**为新文档补一次取数**。
- **判据**：`node editor/vscode/test-extension-host.js` 新增用例绿
  （「A 的请求在飞时切到 B ⇒ B 最终被取数并推给 Infoview」）。

### T-B09 客户端 E5：`_pendingDecls` 合并把新文档那次取数吃掉

- **根因**：`extension.js:344-348` 的合并按 provider 而不是按 URI；A 在飞时
  `trackEditor(B)` 拿回 A 的 promise，A 随后在 `:362` 提前返回 ⇒ **B 的取数从未发生**。
  A 是慢编译的项目文件时必中。现有测试把"合并"当**特性**钉住
  （`test-extension-host.js:574-590`），`:592-633` 只断言树里没有 stale 行，
  **从不断言新文档被取数**。
- **改什么**：合并按 URI 分键（`_pendingDecls: Map<uri, Promise>`），或合并后检测
  URI 变化并重查。
- **判据**：stub 宿主新增「A 慢、B 快 ⇒ B 不被 A 的 promise 吃掉」。

### T-B10 客户端 E6：空数组是真值 ⇒ `ensureDeclarations()` 永久空转

- **根因**：`if (!this.declItems)`（`extension.js:334`）+ 无条件
  `this.declItems = decls.map(...)`（`:366`）；`decls = []` 时 `![] === false`
  ⇒ 之后 `:1879`/`:260`/`:699` 三处 `ensureDeclarations()` 全部空转。
  **触发**：激活时活动编辑器已是 `.sokonanoda`——`trackEditor` 在 `:1805` 跑，
  `client` 直到 `:1858` 才创建 ⇒ `requestGoals` 立即 `undefined` ⇒ 面板
  "暂无声明。"、状态卡在"编译中…"。
- **改什么**：用 `_declsUri` 记录"已经为哪个 URI 取过数"，取代 `!declItems` 的真值判断。
- **判据**：stub 宿主新增「激活时活动编辑器已是 `.sokonanoda` ⇒ 服务器起来后必须
  补取一次并推 `ready`」。

### T-B11 客户端 E8：指纹吞掉"只改了类型"的更新

- **根因**：`declsFingerprint`（`extension.js:407-415`）只含
  `name/kind/status/holes[0].id`，改类型文本不变指纹 ⇒ 卡片 `ty` 与 `L<n>` 停在旧值。
- **改什么**：指纹纳入 `ty`（或 `ty_runs` 长度 + 首尾文本）。
- **判据**：stub 宿主新增「只改类型文本 ⇒ 卡片更新」。

### T-B12 Webview 空态三态化

- **改什么**：`editor/vscode/media/infoview.js:229-232` 现在无论什么原因都是
  「暂无声明。」——改成 **真的 0 条** / **服务器尚未编译完成** / **读取失败**。
- **判据**：`node editor/vscode/test-webview.js` 新增 3 个分支断言（现在 0 覆盖）。

### T-B13 真宿主 e2e：声明栏真的有卡片（矩阵用例 #1）

- **改什么**：`editor/vscode/src/test/extension.test.js` 增加用例
  `declarations panel lists a project unit's declarations`：打开 T-015 夹具的
  `units/u01.sokonanoda`（含 `import` + 库记法），断言
  `infoview.lastDecls().length > 0` 且与 `goals.declItems.length` 一致
  （现在 e2e 只 smoke 命令存在，**从不看声明栏**）。
- **判据**：
  ```bash
  scripts/vscode-e2e.sh --grep "declarations panel lists a project unit" --profile debug --no-build
  # 期望：1 passing；**改动前必须先跑一次确认 0 decls（红）**
  ```
- **依赖**：T-015、T-016、T-017。

### T-B15 真宿主 e2e：`alt+n` 跳洞（矩阵用例 #2）

- **改什么**：新增用例 `next hole jumps inside a project unit`：在夹具的
  `units/u01.sokonanoda` 里执行 `sokonanoda.nextHole`，断言活动编辑器选区落在
  洞的 range 内（T-B06 修复前必须红）。
- **判据**：`scripts/vscode-e2e.sh --grep "next hole jumps" --profile debug --no-build`
  → 1 passing。

### T-B16 顺带修复 G-28：`→` 右边直接跟 `∀` 不解析

- **来源**：写 G-26 复现夹具时顺带发现，已立账（`docs/gaps/ledger.jsonl` 的 G-28）。
- **根因**：parser 的箭头右侧表达式入口不接受 `forall`/`∀` 作为起点。
  实测：`A -> forall (x : Prop), x -> x` 报 `unexpected-token`
  「expected an expression, found Forall」；`A -> (forall …)` 与 `A ↔ ∀ x, P` 都正常。
- **为什么值得单列**：硬规则 3（教学语法是真实 Lean 4 的子集）的违例；
  课程正文里 `A ⊆ B → ∀ a, a ∈ A → a ∈ B` 这种形状很常见，今天得手写括号绕开。
- **改什么**：让箭头 RHS 接受 `∀`/`forall`（与 `↔` 的 RHS 走同一入口）。
- **判据**：
  ```bash
  scripts/soko grade docs/gaps/repro/G28-arrow-forall.sokonanoda ; echo "exit=$?"   # 期望 0
  python3 scripts/gap.py check ; echo $?                                            # 期望 0
  # 加 `gap.py close G-28 --version <版本>` 关账
  cargo test -p sokonanoda-front -- --nocapture        # 新增解析回归
  ```
- **测试层**：front 解析单测 + CLI e2e + 课程语料（G-28 的 `.sokonanoda` 复现件）。

### T-B14 文档同步

- **改什么**：`docs/protocol.md`（`soko/goals` 在 `NotParsable` 下的语义澄清）、
  `docs/design/goal-list.md` as-built、`docs/vscode-dev-guide.md` 新增坑条目
  （"同一个'能不能用'的判据有多个出口 ⇒ 收敛成 `usable()`"）、
  `editor/vscode/CHANGELOG.md`。
- **判据**：`cargo test -p sokonanoda-cli --test extension` 绿。

### ✅ 检查点 CP-B（批次 1，patch 版本）

- [ ] `bash scripts/verify-editor-issues.sh` 第 4 条转「已修」
- [ ] 13 unit + 13 solution 的声明栏全部非空（T-B01 表两列相等）
- [ ] `alt+n` 跳洞在 unit 文件里可用（T-B02 表更新）
- [ ] `node editor/vscode/test-extension-host.js` 全绿；`scripts/vscode-e2e.sh` 全绿
- [ ] `scripts/soko gate` exit 0
- [ ] **性能无退化**：`python3 scripts/perf-compare.py --since <上一检查点>` exit 0
      （任何 `best_ms` 退化 > 25% 必须解释或修回；新 case 提示无基线不算红）
- [ ] **用户验收**：在 VS Code 里打开几个 unit 文件，声明栏都有内容

---

## 5. 线 A：编译缓存 + 清除浪费（反馈 1/2/3）

> **收益最大**（1.8–36s → 期望 <100ms），但要动缓存键与条目形状，
> 是最容易出隐蔽错误的线。**T-009 的结论是这条线的前置。**

### 根因

- **A1** LSP 对含 `import` 的文档**既不读也不写任何缓存**：
  `crates/lsp/src/lib.rs:148` 算 `has_imports`，`:152-156`
  `let cached = if cfg!(test) || has_imports { None } else { cache::load(...) }`；
  `:183-194` 的 `cache::store` 也被 `!has_imports` 门住。`grep digest crates/lsp` = **0**。
  文档自认共用（`docs/design/compile-cache.md:16-17`、`crates/lsp/src/lib.rs:131-133`
  「`build`/`check` 预热过的画布对编辑器同样有效」）——**对项目文件不成立**。
- **A2** 项目缓存住在 **CLI crate**（`crates/cli/src/project_cache.rs`），LSP 依赖不到。
- **A3** 条目形状不对：只存**入口模块**的 report + events
  （`crates/cli/src/project_cache.rs:42-49`），`QueryDoc::set_cached_entry` 拿到它
  直接把 `project` 置 `None`（`crates/front/src/query/mod.rs:215`）。LSP 需要的
  **逐模块报告**（跨文件 `definition`/`references`/`rename`、`soko/project` 模块表、
  扇出判定都读 `project_modules()`）不在里面。**设计 §4.8 写的是"按模块存"
  （`docs/design/imports-and-projects.md:594-595`），实现已经分叉。**
- **A4** 项目缓存**永不写**：只写"完全干净"的项目（`store_if_clean`
  `crates/cli/src/project_cache.rs:33-41`；`is_clean`
  `crates/front/src/project/report.rs:158-166` 要求每个模块 `warnings.is_empty()`），
  而 `sorry` 是 WARNING（§2.2 实测）。
- **A5** 闭包编译**没有按模块增量**：`compile_plan`
  （`crates/front/src/project/mod.rs:272-295`）→ `compile_all_units`
  （`crates/front/src/compile/units.rs:113-118`）→ `run`（`check/mod.rs:402-430`）
  **从零**；增量入口 `run_incremental`（`check/mod.rs:443`）只服务单文件 `Session`。
  连读盘 + parse 也每次重做（`project/graph.rs:221,376,450-459`）。
  设计 §1.3 与 P7 明确 v1 不做。
- **A6** 项目模式**先白编一遍入口单文件**再编闭包：`crates/front/src/query/mod.rs:117`
  `self.session.update(text, version)`，`:135` `self.project = self.project_compile(text)`，
  `:144-151` 用闭包入口报告**覆盖** `self.report` ⇒ 那次单文件编译的报告/事件被丢弃。
- **A7** `Doc::set_text` **没有"文本没变就返回"的短路**（`crates/lsp/src/lib.rs:136-195`）
  ⇒ 保存（`did_save` 是死代码：FULL sync + `includeText:false` ⇒ `if let Some(text)`
  永不成立）与编辑器外改动（`did_change_watched_files`，`:1094-1121`，
  客户端 watcher 不过滤已打开文档）会**重编内容完全相同的文本**。
- **A8** 扇出 = N 次闭包编译：`crates/lsp/src/lib.rs:420-450` 遍历所有打开文档，
  命中就 `set_text_at`（`:257-271`）。N 个课程文件同时打开（VS Code 重启会恢复
  所有编辑器）时，改 `lib/Logic` 一行 = N 次闭包编译，且**串行在同一把
  `Mutex<Docs>` 里**。
- **A9** 编译期间**整把文档锁被占住**（`:381-466`，注释 `:378-379`「锁内同步编译
  可接受」）。单次 8–36s 时，hover / goals / stateAt / 诊断刷新**全部堵在后面**——
  这是"慢"被感知成"卡死"的直接原因。
- **A10** `build <目录>` = O(文件数 × 闭包)：`crates/cli/src/build.rs:40-57` 递归收集
  全部 `*.sokonanoda`，逐个 `build_one`（`:117-148`）。对 `courses/set-theory`
  （33 个文件）就是 33 次闭包编译。
- **A11** `by` 块判定**每步重跑整份文档前缀**（`crates/front/src/judge.rs:257-263`；
  批处理 `judge_pairs_with` `:270-294` 只把"一个 `by` 块内"合成一次；缓存按前缀
  文本失效 `:80-81`）。`STATUS.md:152-156` 明写「**根因未修，不许当成已修**」。
  真正的修法是给未改动的前缀做缓存 ⇒ 需要**内核侧的环境复用/检查点**
  （2026-09-21 内核解冻后转正为 §5.6 线 K 的 T-K10…）。
- **A12** 缓存键含 `build_stamp()` = `current_exe()` 的 mtime（秒级）
  （`crates/front/src/compile/cache.rs:67-79,88-96`）⇒ CLI 与 LSP 是两个二进制，
  预热能否命中取决于 mtime 同秒。**见 T-009。**

### 5.1 A-I 缓存地基（先把"键"和"形状"做对）

#### T-A01 项目缓存下沉到 `front`

- **改什么**：新建 `crates/front/src/project/cache.rs`，把
  `crates/cli/src/project_cache.rs` 内容搬过去；CLI 保留 thin re-export（公开 API 不变）。
- **判据**：`cargo test --workspace --locked` 全绿 + `scripts/soko gate` exit 0 +
  **二进制对拍**（改动前后两个 CLI 对全部 `*.sokonanoda` + `--root`/`--no-project` +
  stdin + `query check|goals|holes` 的 stdout 逐字节相同）。
- **风险**：纯搬迁，**不许夹带语义改动**。

#### T-A02 缓存键去掉"可执行文件 mtime"这个不稳定的量

- **改什么**：按 T-009 的结论改 `build_stamp()`（例如换成编译期常量
  `CARGO_PKG_VERSION` + profile + 目标三元组，或干脆去掉），并 bump `CACHE_FORMAT`。
- **判据**：
  ```bash
  # 两个 mtime 不同秒的二进制之间：CLI 预热 → LSP 命中
  bash docs/gaps/repro/G27-cli-warm-lsp-hit.sh ; echo "exit=$?"
  ```
  （新缺口 G-27，修前判红。）
- **依赖**：T-009。
- **风险**：**这是线 A 的地基**——不修则"编辑器变快"可能完全落空。

#### T-A03 缓存条目 v2：按模块存（对齐设计 §4.8）

- **改什么**：项目条目改为存**每个模块**的 report + events + 归因
  （`ProjectReport` 的模块表 + 入口），使 `set_cached_entry` 能重建
  `project_modules()`（跨文件导航、`soko/project`、扇出判定都要它）。
  单文件条目形状不动（保住"无 import 的文件逐字节不变"的不变量）。
- **判据**：
  ```bash
  cargo test -p sokonanoda-front cache:: -- --nocapture
  # 新增：v2 往返逐字段相等；既有：单文件 key 与旧格式逐字节相同
  ```

#### T-A04 冷/热 `--json` 逐字节一致（带 `sorry` 的项目）

- **改什么**：新增测试：对一个**带 open exercise** 的课程单元，冷跑与热跑的
  `--json` 事件流**逐字节相同**（这是 A4 那条 v1 边界的正解）。
- **判据**：`cargo test -p sokonanoda-cli --test imports -- --nocapture` 绿；
  **故意破坏**（热跑丢一条 warning）时该测试必须红 ← 判别性检查。

#### T-A05 `requires` 漂移不再静默关掉缓存 + 两条写缓存路径规则一致

> **根因已在 T-001 实测更正**（§2.2）：不是 `sorry`，是**清单 `requires` 与二进制
> 版本漂移**（`courses/set-theory` 写 `0.61`，二进制 0.63.0）。`sorry` 不在
> `module.report.warnings` 里，**不阻止缓存**。

- **改什么**：
  ① `store_if_clean` 的判据不再被 `requires_warning` 一票否决——漂移是**可回放的
     确定性事实**，应当随条目一起存/回放，而不是关掉整个缓存；
  ② **漂移要可见**：`query check`（JSON 视图，编辑器走的就是这条）里能查到它
     （形态由实现定：进 `warnings[]` 或加一个 `project` 块）；
  ③ **两条写缓存路径规则统一**：`build`/`check` 走 `store_if_clean(is_clean())`，
     `query` 走 `set_cached_entry`（不看 `is_clean`）——同一个项目的缓存命不命中
     不该取决于先跑了哪条命令。
- **判据**：
  ```bash
  bash docs/gaps/repro/G24-project-cache-never-warms.sh ; echo "exit=$?"   # 期望 1
  python3 scripts/gap.py check ; echo $?                                   # 期望 0
  ```
- **风险**：**"热跑诊断 ≠ 冷跑诊断"是最坏的失败模式** ⇒ T-A04 是前置，不许跳过。
- **注**：课程清单本身的版本漂移由 **T-A08** 从根上消除。

#### T-A08 `requires` 的单一来源 + 漂移门禁（**用户判定的根因**）

- **根因**：开发过程**没有自动提升**项目清单的 `requires`。`courses/set-theory`
  与语言仓**同仓共同开发**，版本本来应当一致，却手写着 `"0.61"`；
  `course/shared/sokonanoda.toml` 写 `"0.57"`，同样漂移。
- **改什么**（两条一起，缺一不可）：
  ① **单一来源**：清单的版本不再手写——或由脚本在 bump 时同步，或清单干脆
     声明"跟随仓库版本"（`sokonanoda.toml` 允许 `requires = "workspace"` 之类，
     具体形态由实现定，但**不能靠人记得改**）；
  ② **门禁**：`scripts/soko gate` 与 CI 增加一步"清单版本 == 仓库版本"的检查
     （零依赖 python3，与课程门禁/台账门禁同一层），漂移直接判红。
- **判据**：
  ```bash
  # 门禁能抓住漂移
  python3 scripts/check-manifests.py ; echo "exit=$?"      # 期望 0（当前清单已同步）
  # 故意把某个清单改旧 ⇒ 必须判红
  sed -i '' 's/requires = "0\.[0-9]*"/requires = "0.1"/' courses/set-theory/sokonanoda.toml
  python3 scripts/check-manifests.py ; echo "exit=$?"      # 期望 1
  # 改回
  git checkout courses/set-theory/sokonanoda.toml
  ```
- **为什么排在这里**：它是**唯一**能让"卷 I 的缓存真的开始工作"的修法；
  不修它，批次 2 的 LSP 接线（T-A10…）做完也仍然全是 miss。

#### T-A06 依赖改动仍必 miss

- **改什么**：确认并补断言：摘要按拓扑序含每模块源文本（`ProjectPlan::digest`）。
- **判据**：`cargo test -p sokonanoda-cli --test imports -- --nocapture` 既有判别性
  测试绿（改依赖后 `#reduce` 必须给出新值）+ 新增"改 lib 一行 ⇒ 入口必重编"。

#### T-A07 `--text` 中间态的处置

- **改什么**：测量 `--text`（内存中间态）在编辑器路径上是否出现；若出现，
  决定"不缓存"是否仍成立，把结论写进设计文档。
- **判据**：结论 + 数字进 `docs/design/compile-cache.md`。

### 5.2 A-II LSP 接线

#### T-A10 LSP 读项目缓存（命中即回放）

- **改什么**：`Doc::set_text`（`crates/lsp/src/lib.rs:136-195`）的 `has_imports`
  分支改为：`ProjectPlan::digest(options)` → `cache::load` → 命中即回放
  （诊断 + 逐模块 report + 事件）；`cfg!(test)` 仍不碰真实缓存。
- **判据**：`bash docs/gaps/repro/G25-lsp-recompiles-on-every-open.sh` → `1`。

#### T-A11 LSP 写项目缓存

- **改什么**：未命中时编译完按 T-A05 的判据 store。
- **判据**：`bash docs/gaps/repro/G24-…sh` → `1`；`G25-…sh` → `1`。

#### T-A12 LSP 侧测试：两次独立进程打开，诊断逐字节一致

- **判据**：`cargo test -p sokonanoda-lsp -- --nocapture` 新增用例绿。

#### T-A13 跨文件能力在缓存命中后仍然工作

- **改什么**：补测试：命中缓存后，`textDocument/definition`（跨文件）、
  `references`、`rename`、`soko/project` 的模块表**都还正确**
  （这正是 T-A03 按模块存的验收）。
- **判据**：`cargo test -p sokonanoda-lsp project:: -- --nocapture` 绿。

#### T-A14 实测数字

- **判据**（§2.1 同款探针）：unit01/08/12 与 unit12 解答的 `didOpen → 诊断`
  降到与"同文本再 didChange"同量级；把表贴进 `docs/PERF.md`。

#### T-A15 命中缓存后 Session 快照的处置

- **改什么**：二选一，**用数字决定**：① 命中时也把报告喂给 `Session` 建立快照；
  ② 明确不做，把退化成本写进设计文档（`docs/design/compile-cache.md` §4）。
- **判据**：`scripts/perf-ledger.sh && tail -5 docs/perf/ledger.jsonl`
  ——「缓存命中后第一次按键」有一行明确数字（做或不做都要有数字）。

### 5.3 A-III 清除浪费（**不碰缓存也能立刻变快**）

#### T-A20 项目模式不再白编一遍入口单文件

> **实测结果（2026-09-21，做完之后补记）**：收益**远小于预期**——课程入口
> 单独 parse 就失败（记法来自 import），那次"白编"在 parse 阶段就退出了，
> 很便宜。实测 unit01/08/12 的 `didOpen` 只快了 **0.6% / 0.9% / 1.1%**，
> 合成项目（入口能单独 parse）也是 **0%**。仍然保留这个改动（它确实去掉了
> 一次冗余编译、且让"报告从哪来"更清楚），但**别把它算进性能收益**。
> 真正的收益在 T-A21（356ms → 0ms）与 T-A10（接缓存）。

- **根因**：A6（`crates/front/src/query/mod.rs:117` vs `:135`/`:144-151`）。
- **改什么**：项目模式下跳过那次单文件 `session.update`（或让它只为
  `parse_error` 服务而不跑完整流水线）。
- **判据**：unit08 的 `didOpen → 诊断` 明显下降（贴前后数字）；
  `cargo test --workspace --locked` 全绿。

#### T-A21 `set_text` 文本未变即短路

> **实测（2026-09-21）**：unit08（8 模块闭包）上一次"同文本 didChange"
> **356ms → 0ms**。保存、`git checkout`、别的工具写文件都会走这条路，
> 而它们的内容往往和缓冲区一模一样。哨兵 =
> `crates/lsp/src/tests/perf_course.rs::perf_course_save_same_text_is_recorded`
> （`case: "save_same_text"`，阈值 500ms）。

- **根因**：A7。
- **改什么**：`Doc::set_text` 开头比较 `self.doc.text == text`（且版本/模式相同）
  ⇒ 直接返回（仍按需发诊断，但**不重编**）。
- **判据**：新增 LSP 测试「同文本 didChange 不重编」；
  `bash docs/gaps/repro/G25-…sh` 仍 `1`。

#### T-A22 保存 / 编辑器外改动不再重编同一文本

> **实测（2026-09-21）**：**服务端已被 T-A21 的短路覆盖**——
> `workspace/didChangeWatchedFiles` 在内容没变时 **361ms → 0ms**
> （哨兵 `perf_course_watched_unchanged_file_is_recorded`，阈值 200ms；
> 回滚 T-A21 即红，已验证）。
>
> **决定：不加客户端 watcher 过滤**。原计划写的是"客户端 watcher 过滤已打开
> 且未落盘的文档"，但服务端短路之后那条路已经是 0ms，再加一层客户端过滤
> 只是多一处可能与服务端判断不一致的地方（而且 VS Code 的 watcher 事件不带
> "这个文档脏不脏"的信息，客户端要自己查）。**一处判据就够了**——这条纪律
> 在 G-22 上已经付过学费。

- **根因**：A7（watched-files 无过滤 + `did_save` 死代码）。
- **改什么**：① 客户端 watcher 过滤已打开且未落盘的文档；② 或服务端在
  `did_change_watched_files` 里比对缓冲区内容后再决定是否 refresh。
- **判据**：stub 宿主 / LSP 测试「保存已打开文件 ⇒ 不产生第二轮闭包编译」。
- **依赖**：T-A21。

#### T-A23 扇出：改一个依赖不重编所有打开文档

- **根因**：A8。
- **改什么**：先测量真实倍数（打开 5 个课程文件，改 `lib/Logic` 一行）；
  再决定：① 只对**闭包内**文档重编（已经是）；② 未受影响的文档直接复用上次报告
  而不重编（现在已经在做"重发上次诊断"）；③ 真正的问题是**受影响的** N 份各编一遍
  ⇒ 用 T-A03 的按模块条目 + 摘要前缀复用减少重复。
- **判据**：扇出总耗时数字进 `docs/PERF.md`；`crates/lsp/src/tests/perf.rs` 的
  `perf_project_dependency_edit_refreshes_dependents` 不退化。
- **依赖**：T-A10。

#### T-A24 `project_view_reason()` 不再每次 parse 整份文本

- **根因**：§2.7 第 4 条（`crates/front/src/query/project.rs:36`）。
- **改什么**：缓存这份判定（按文本/摘要）。
- **判据**：`cargo test -p sokonanoda-lsp perf_project_requests_are_interactive` 不退化。

#### T-A25 `build <目录>` 的 O(文件数 × 闭包) 如实记账

- **改什么**：不改行为。测 `build courses/set-theory` 冷/热总时长，写进
  `docs/PERF.md` 与 `docs/design/compile-cache.md`，并在扩展的 `alt+b` 提示里
  说明"目录 build 是逐文件各自闭包"。
- **判据**：数字进文档；`editor/vscode/README.md` 同步。

### 5.4 A-IV 交互性与锁

#### T-A30 编译不再独占 `Mutex<Docs>`

- **根因**：A9。
- **改什么**：把编译移出锁（先在锁内取快照 → 锁外编译 → 再取锁写回，带版本校验），
  或至少让只读请求（`stateAt`/`project`/`hover`）用 `RwLock` 读侧并发。
- **判据**：新增 LSP 测试「一次长编译进行中，`soko/stateAt` 仍能在 <100ms 内应答」。
- **风险**：并发正确性；必须先有 T-A21/T-A10 的缓存兜底，否则收益有限。
- **依赖**：T-A10。

### 5.5 A-V 真宿主 e2e（矩阵用例 #3/#4/#5）

#### T-A60 缓存与扇出的 e2e 断言

- **改什么**：`editor/vscode/src/test/extension.test.js` 新增三条：
  1. `reopening a project unit hits the compile cache` —— 冷开夹具 `units/u01`
     记 `t1`；`sokonanoda: restart server` 后重开同一文件记 `t2`；
     断言 `t2 < t1 / 3`，并把 `t1`/`t2` 写进 e2e 台账（人判读用）。
  2. `saving an unchanged project unit does not recompile` —— 保存后
     `infoview.lastDecls()` 与保存前逐字节相同，且**没有新的编译**
     （T-016 暴露的编译计数或 cache 目录条目数不变）。
  3. `editing a dependency refreshes the open unit once` —— 改夹具 `lib/Set` 一行
     ⇒ 打开的 `units/u01` 诊断更新，且只发一份。
- **判据**：
  ```bash
  scripts/vscode-e2e.sh --grep "reopening a project unit" --profile debug --no-build
  scripts/vscode-e2e.sh --grep "saving an unchanged project unit" --profile debug --no-build
  scripts/vscode-e2e.sh --grep "editing a dependency refreshes" --profile debug --no-build
  # 每条在修复前必须先跑一次确认是红的
  ```
- **风险**：用例 1 是**时间断言**，按 `docs/PERF.md` 纪律标定（best-of-N +
  按最慢受支持 runner 的分布留余量），并**同时记数字进台账**——
  pass/fail 之外还要能看趋势。
- **依赖**：T-015、T-016、T-017、T-A10。

### 5.6 A-VI 线 K：内核解冻后的性能根治（**批次 5**）

> 「跨模块增量 / 入口间共享编译产物」与「`by` 块前缀重判根治」这两条
> **原本因为"碰内核冻结边界"被列为不做**；2026-09-21 用户解冻内核
> ⇒ 转正，作为独立批次（批次 5）。
>
> **可行性调研已完成**（2026-09-21，只读，逐行核对），三条结论直接改写本节：
> ① **收益上界已量出**：`unit12-solution` 有 **≈20 次前缀重编译**
> （9 个声明值位 `by` + 闭包 lib 里 11 个 `:= by`），36.13s ÷ 20 ≈ 1.8s/遍，
> 而 term 风格基线是 3.6s ⇒ **环境复用的天花板 = 36.1s → ~3.6–5s（8–10×）**，
> 正好是 CP-K 要的"个位数秒"。
> ② **最划算的第一刀是纯 front、零内核风险**（K1-a，见 T-K11）。
> ③ **单次内核改动里性价比最高的是 `EnvBuilder::with_env`**（K1-b，内核 ~60–120 行，
> `ExportFile`/`TcCtx`/`eval`/`conv` 零改动），而且它是第二刀的基座。

#### 5.6.0 先建护栏（**没有这个不许动内核**）

##### T-K01 全语料逐字节对拍工具

- **改什么**：`scripts/kernel-diff.sh <before-bin> <after-bin>` —— 对全部
  `*.sokonanoda`（`courses/` + `course/` + `playground` + `examples/` + `docs/gaps/repro/`）
  跑 `grade --json`，**stdout 逐字节比较**；再跑 `query check|goals|holes`
  的摘要比较；**外加**课程门禁计数逐项比较
  （`python3 courses/set-theory/tools/check.py --json`）。
- **判据**：`bash scripts/kernel-diff.sh <旧二进制> <新二进制>` → 全绿（零差异）；
  故意改坏一处（例如给某条 warning 改一个字）→ 必须报差异并 exit 1。
- **为什么**：这是内核解冻后**唯一**能证明"正确性不变"的机械判据。
  仓库既有先例：批次 3 拆 `run_pass` 用的就是"二进制对拍"。
- **现状**：**脚本不存在**（`ls scripts/` 里没有）。

##### T-K02 内核改动的验收清单 + **修语料对拍的收集逻辑**

- **改什么**：
  ① `docs/architecture.md` §6 补一节「改内核的验收五步」；
  ② `scripts/kernel-check.sh` 一条命令跑完五步
  （`cargo test --workspace --locked` → `kernel-diff.sh` → 课程门禁计数 →
  `perf-ledger.sh` → 语料对拍）；
  ③ **修 `crates/kernel/tests/arena.rs` 的收集逻辑**（调研实测的三个缺陷）：
  `:30` 只 `read_dir(root/_build/tests)` **不递归**、`:38` 跳过 >64MB、
  `:19-27` 只认平铺 `tests/<stem>.yaml` ⇒ 本机**只有 `init-prelude.ndjson`
  （3.7MB、1777 条）被收集**，`init`(309MB)/`std`(526MB) 被体积跳过，
  14 个 `perf/*` 与 15 个 `corner-cases/*` 因在子目录**完全不可见**；
  `:134-138` 对 `outcome: reject` **把 parse error 当通过**（`Either` 永不失败）。
- **判据**：
  ```bash
  LEAN_KERNEL_ARENA=/path/to/lean-kernel-arena scripts/kernel-check.sh
  # 期望：收集到的用例数从 1 → 显著更多（递归 + 子目录 yaml）；
  #       reject 用例的 parse error 不再算通过
  ```
- **为什么必须现在修**：**现在的"语料对拍"只有一个用例，是安慰剂**。
  而且 CI 里没有 `LEAN_KERNEL_ARENA`（`.github/` 零命中）⇒
  `cargo test --workspace` 里它**静默空过**（`arena.rs:112-115`）。
  要么把它接进 CI（opt-in job），要么在文档里明说"本地可选"——**不能假装有**。
- **附**：内核单测的真实分布（调研实测）——`crates/kernel/src/*.rs` 里只有
  `builder.rs:317-329` 一个 `#[cfg(test)] mod tests`；真正的在内核
  `src/tests/{level,natlit,util}.rs`（40 条）+ `parser.rs` 的 `semver_tests`（2 条）；
  **`pretty_printer.rs` 零单测**（这条对线 C 重要）。最强语义钉子在
  `crates/kernel/tests/memory_api.rs`（8 条）。

##### T-K03 `36.1s` 花在哪：分阶段 profile

- **改什么**：对 `courses/set-theory/units/solutions/unit12-solution.sokonanoda`
  做分阶段 profile，把 36.1s 拆成「读盘+parse / elab / **内核检查** / `by` 判定 / pp」。
  **已有现成探针可抄**：`docs/design/by-tactics.md` §13 的仪器化实测
  （`hits=20764 misses=89 judge_time=83.5s avg=938.6ms prefix_bytes=5075591`；
  `parse=0.2s check_document=82.3s commands_total=6162`）——
  **99.6% 花在 `check_document_with` 重跑前缀，parse 只占 0.2%**。
- **判据**：`docs/PERF.md` 新增一张分阶段表，数字加起来 ≈ 36s；
  **明确"内核检查占 X%、elab 占 Y%"**——它直接决定 T-K11（只省内核检查）
  的收益上界。
- **附**：实测 `unit12-solution` 的实际 pass 数（调研静态数出 ≈20）要在这里证实。

#### 5.6.1 第一刀：`by` 块判定的前缀复用

##### T-K10 设计文档 `docs/design/by-prefix-reuse.md` + 三个候选的定稿

- **改什么**：写清 ① 现状调用链；② **TrustPlan 不是环境复用**（见下）；
  ③ 三个候选与推荐顺序；④ 每个候选的正确性风险与验收。
- **必须写进文档的两条纠正**（调研发现，与直觉相反）：
  1. **`front::session` 的 TrustPlan 只省"内核重查"，不省 elaborate**：
     `walk.rs:134` 的 `trusted` 分支在**跑完 `lower_value`（含 `by` 判定）与
     `build_def`/`build_theorem` 之后**才 return（`walk.rs:375-407` 等），
     只是不 push `PendingOp`。`docs/architecture.md:548` 已写死：
     *"Session 每次 update 都开新 arena——跨 update 只复用渲染后的快照，不复用内核对象"*。
  2. **真正的障碍不是 arena 借用，而是 `NameNode::decl_idx`**——
     一个**挂在被 intern 的 NameNode 上的全局可变槽位表**
     （`builder.rs:244` 写、`env.rs:280-287` 读且**不做名字校验**）。
     用 builder A 造的 `Declar` 去查 builder B 的环境会**静默取到别人的声明**。
     另一条：`ExportFile: Sync` 被 `check_all_declars_par`（`tc.rs:208-240`，
     `thread::scope`）需要，而 `stumpalo::ArenaRef` **既非 Send 也非 Sync**
     ⇒ **把 `&ArenaRef` 存进 `ExportFile` 这条路走不通**（列为"被否方案"）。
- **判据**：文档 review 通过；三个候选各自的"侵入面 / 解锁收益 / 正确性风险"
  三栏齐全，且**写明为什么 K1-b 优于 K1-c**。

##### T-K11 **K1-a：纯 front 的 judge TrustPlan 复用（零内核改动，先做）**

- **改什么**：`judge_pairs_uncached`（`judge.rs:418`）把
  `check_document_with(&file, options)` 换成
  `run_incremental(&file, options, &TrustPlan{ before: 前缀命令数, … }, &prefix_failures)`；
  `prefix_failures` 由外层 walk 的失败表映射到合成文件的下标传入。
  `judge_infer` / `judge_type_of` 同法（它们的 `#check` 在 `before` 处，
  `trusted=false`，照常执行）。
- **侵入面**：**front 约 60–150 行**（judge 三处入口 + 把失败表从 `run_pass`
  穿到 `lower_value`/`run_by`/judge——**穿参是主要工作量**）。内核 **0 行**。
- **解锁**：每个 pass 省掉"前缀所有声明的**内核检查**"。收益 = 内核检查在
  prefix pass 里的占比 × ~19 遍（由 T-K03 定）。按 §13 探针，
  若内核检查占一半 ⇒ 36.1s → ~18s；占 80% ⇒ → ~8s。
- **为什么先做**：**复用的是已经上线、已有逐字节对拍测试的机制**
  （`crates/cli/tests/judge_batch.rs:55-64` 的 `assert_same_both_ways`），
  内核一行不动 ⇒ 零内核风险。
- **判据**：加开关 `SOKO_JUDGE_ENV_REUSE=0/1`（仿 `SOKO_NO_JUDGE_BATCH`，
  `judge.rs:461-474`），两态下全语料 `--json` **逐字节相同**；
  `unit12-solution` 耗时下降的实测数字进 `docs/perf/ledger.jsonl`。
- **三处必须守死**：(i) 只有外层 pass 对前缀的判定可担保时才允许复用；
  (ii) trusted 前缀**不产 `DeclState`/事件**（`walk.rs:375-407`）⇒
  `judgement_of` 只读 `_soko_judge_k`，**必须加断言**防止报告缺状态被误用；
  (iii) `skip` 语义必须与 pass 2 的 check-then-add 逐字一致。
- **依赖**：T-K01、T-K02、T-K03。

##### T-K12 **K1-b：`EnvBuilder::with_env`（单次内核改动里性价比最高）**

- **改什么**：给内核加

  ```rust
  impl<'a> EnvBuilder<'a> {
      /// 把 builder 的字段临时装进一个 `ExportFile<'a>` 交给回调，
      /// 回调结束后装回。`ExportFile` 结构体本身**一字不改**。
      pub fn with_env<R>(&mut self, f: impl FnOnce(&mut ExportFile<'a>) -> R) -> R
  }
  ```

  实现：把 `dag/anon/zero/declars/notations/config/mutual_block_sizes` 收进
  一个 `struct BuilderState<'a>` 用 `Option` 持有（或 `mem::replace` +
  `Dag::empty`）。**`ExportFile`/`TcCtx`/`eval`/`conv`/`infer` 零改动**
  ⇒ 不碰 O5 的 `Sync` 问题，也不碰指针恒等式。
- **front 调用时序（顺序是关键）**：
  1. `let k = builder.declaration_count();`
  2. `let d = build_def(&mut builder, …)` —— 合成声明**先**用 builder 建
     （此时所有 name/string/bignum/level 进 builder 的**活表**）；
  3. `builder.with_env(|env| env.try_check_declar_at(&d, EnvLimit::ByIndex(k)))`
     —— 检查器看到的 intern 表与 builder **完全同一份**，指针恒等式天然成立；
  4. 合成声明**从不 `add_declar`**，环境不被污染，`decl_idx` 不变。
- **侵入面**：**内核 ~60–120 行**（builder 状态拆分 + `with_env`）；
  **front ~150–300 行**（把 `&mut EnvBuilder` 从 walk 穿到
  `lower_value`→`run_by`→judge；把"合成文档"换成"合成 `Declar` + 直接查"）。
- **解锁**：**36.1s → ~4–6s**（19/20 次前缀重编译消失，只剩外层一次全量
  ≈ term 基线 3.6s）；`unit12-synthesis` 8.49s → ~1–2s；闭包内 `lib/Set`
  的 10 个 `by` 一并受益。**并且它是第二刀（T-K20…）的基座。**
- **最容易翻车的地方**：必须逐字保持 `Judgement` 的三态与文案——
  `judgement_of` 现在从**报告**里取 `Mismatch{expected,actual}` /
  `Error{code,message}`，而 `Error.code` 是 front 的稳定错误码
  （来自 `refine_kernel_kind`）⇒ **新路径要复用同一个分类器**，否则诊断码会变。
  逐条对拍，别只看 pass/fail。
- **另需守**：`quiet_catch` **不可嵌套**（`docs/architecture.md:535-537`；
  `try_check_declar_at` 自己 take/set 全局 panic hook，`util.rs:670-673`）。
- **判据**：`scripts/kernel-check.sh` 全绿；`unit12-solution` 到个位数秒；
  新增单测"新路径与旧路径逐条 `Judgement` 相等（Match / Mismatch 文案 /
  Error code **全字段**）"。
- **依赖**：T-K11（先拿掉内核检查那一段，再上环境复用，两刀收益可分辨）。

##### T-K13 **K1-c（备选）：`EnvBuilder::snapshot()` 克隆式检查点**

- **只在 K1-b 的状态拆分被判太侵入时用**。改什么：给 `interner!` 宏与三个手写
  interner 加 `Clone`（**~10 行**，`util.rs:324-459`），`Dag: Clone`，
  `EnvBuilder::snapshot() -> ExportFile<'a>`（**~25 行**）。
- **陷阱（必须写进设计）**：**快照必须在合成声明建好之后取**。内核多处依赖
  "同一字面量/名字 ⇒ 同一指针"：`conv.rs:169` `(NatLit{ptr:px}, NatLit{ptr:py})
  => px == py`（**指针相等**）、`eval.rs:369`/`:1072` 用 `ptr.get_hash()`（= 地址）
  做内容哈希、`NameInterner::get` 比较 `StringPtr` 的**地址**。
  快照若早于合成声明里新出现的字面量 ⇒ 检查器为同一个值造出第二个指针
  ⇒ **本该判过的 def_eq 假失败**。
- **成本**：每 by-site 一次 O(表大小) memcpy（数 MB × 20 次 = 几十 ms，可忽略）。
- **解锁**：与 K1-b 相同，但**只解决本刀**（快照是 builder 的副本，
  不构成跨模块复用的基座）。
- **判据**：同 T-K12。

#### 5.6.2 第二刀：闭包的跨模块增量 / 入口间共享

> **注意**：本刀的"零内核改动版"就是**线 A（T-A10…T-A23）**——
> 调研确认它是低风险纯接线（`crates/cli/tests/project_features.rs:164/194/201`
> 已有冷/热逐字节对拍），**但它只解决"第二次打开"，不解决"首次打开新文件"**。
> 首次打开必须靠下面这两条。

##### T-K20 设计文档 `docs/design/closure-incremental.md` + spike

- **改什么**：写清 ① 现状（一个 `Arena` + 一个 `EnvBuilder` 跑**整个闭包**：
  `project/mod.rs:272-295` → `compile/units.rs:113-118` → `check/mod.rs:402/497/504-505`）；
  ② **两个本刀独有的障碍**（不能靠"再克隆一次"绕过）：
     - **O7 prelude 决策是"按闭包"的**（`check/mod.rs:515-541`：
       `explicit_nat`/`explicit_bool`/`taken` 从 `units` 全集算）⇒
       共享环境对某个入口可能装了它闭包里不存在的 `Nat`/`Bool`/`Eq`
       ⇒ **语义漂移**。必须有"prelude 形状相同才复用"的守卫，
       并**脚本化验证 `courses/set-theory` 每个入口的 prelude 形状是否相同**
       （调研说"是"，但那是实测结论、不是代码保证）。
     - **O8 front 每轮状态**：`known`/`inductives`/`defs`/`ns`/`exports`/
       `GoalTemplates`/`closure_prefixes` 都是 `run_pass` 的局部量
       （`check/mod.rs:506-509`/`:611-626`/`:562-605`），要跨调用复用必须一起
       提升为会话状态。
  ③ 候选：**K2-b（窄版，先落地）**——只缓存最近一次编译的
  `(闭包模块集合, Arena+Env)`，新入口的闭包若是旧集合的超集且 prelude 形状相同，
  则只编新增模块；否则整编。**K2-a（结构正解）**——per-module-root 的
  `ProjectSession`：一次编译、入口间共享 + 全局拓扑序 + 模块级缓存。
  ④ 报告归因不能漂：`split_report` 依赖 `error_cmds`/`warning_cmds` 的
  **命令下标**（`units.rs:44-50`）。
- **判据**：设计文档 + spike 量出"打开 unit01 再打开 unit08，第二次只编新增模块"。
- **依赖**：T-K12（需要"跨调用持有 builder"的能力）。

##### T-K21… 实现（**K2-b 先，K2-a 后**，各拆 3–6 个环节）

- **判据（本刀总）**：同进程依次打开 `lib/Set`(1.98s) / `unit08`(4.59s) /
  `unit12`(8.49s) 的总耗时从 15.06s 降到接近**一次最大闭包**（≈8.5s）；
  `soko/project` 的模块表与跨文件导航**逐字节不变**；
  `scripts/kernel-check.sh` 全绿。
- **风险：高**。prelude 形状、`decl_idx` 槽位、front 每轮状态、报告归因
  四者任一漂移都会**静默改变判卷**。

#### 5.6.3 第三刀：`build <目录>` 编一次全项目

##### T-K30 `build <dir>` 不再逐文件各编一份闭包

- **改什么**：`crates/cli/src/build.rs:40-57,117-148` 现在对目录里每个文件
  各跑一次 `plan_project` + `compile_plan`（`courses/set-theory` = 33 次闭包编译）。
  改成按**模块根分组**、每个模块只编一次。
- **判据**：`time ./target/release/sokonanoda build courses/set-theory` 从
  `O(文件数 × 闭包)` 降到接近 `O(闭包)`；`build.summary` 的 hit/compiled/failed
  语义不变（`crates/cli/tests/build.rs` 全绿）。
- **依赖**：T-K21…（K2-b/K2-a 的实现环节）。

#### 5.6.4 顺带发现：两个独立的低风险内核提速点（**可选，不阻塞**）

##### T-K31 `TcCache::new` 每次 `with_ctx` 清 4MB

- **根因**：`util.rs:1218` 每次 `with_ctx` 都 `vec![0u8; 1<<22]`
  （`WHNF_ADMIT_LEN=1<<22`，`:1569`）+ 约 15 张 4096–8192 容量的 map
  （`session_fx_hash_map`，`:515-517`）。今天 `try_check_declar` **逐声明**一次
  `with_ctx` ⇒ **每声明清 4MB**。
- **改什么**：复用同一份缓存（按会话重置而不是每次 new），或按需增长。
- **判据**：`scripts/kernel-check.sh` 全绿 + `perf-check` 数字下降；
  **T-K12 之后这个点会更显眼**（"每步一次小检查"变多）。

##### T-K32 `pretty_printer.rs` 零单测

- **改什么**：在动任何 pp 相关代码**之前**补基础单测（现在只有
  `memory_api.rs:196` 一条 `assert_eq!(printed, "Prop -> Prop")` 钉着）。
- **判据**：新增 pp 单测覆盖 `->`/`forall`/应用/括号/宇宙；
  **这是线 C 的前置**（见 §6.0）。

#### 5.6.5 收尾

##### T-K40 内核改动台账 + 文档

- **改什么**：`docs/architecture.md` §6 追加本次所有内核改动行；
  `docs/PERF.md` 对比表；`docs/design/imports-and-projects.md` 的 P7 段从
  "未做"改成 as-built；`docs/design/by-tactics.md` §13 的"根因未修"改成 as-built
  并写明用的是哪个方案（K1-a / K1-b）。
- **判据**：文档里的每个数字都能用本计划的命令复现。

##### T-K41 `STATUS.md` 的"根因未修"话术更新

- **改什么**：`STATUS.md:152-156` 现在写着「**根因未修，不许当成已修**」——
  做完要改成 as-built。

### 5.7 A-VII 线 A 的文档与台账收口

#### T-A50 设计文档 as-built

- **改什么**：`docs/design/compile-cache.md` §7 重写（键去 mtime、条目 v2 按模块、
  `is_clean` 判据变更、LSP 接入、跨入口不共享的边界）；
  `docs/architecture.md` §4.5 补一句。
- **判据**：文档里每个数字都能用本计划的命令复现。

#### T-A51 性能台账收口

- **改什么**：`docs/PERF.md` 新增「编辑器：真实课程闭包」小节，贴修前（§2.1）与修后两张表；
  `docs/perf/ledger.jsonl` 追加修后条目。
- **判据**：`scripts/perf-ledger.sh` 跑通；冷/热差距 ≥ 5×。

#### T-A52（可选，用户拍板）打开工作区时预热缓存

- **改什么**：设置项 `sokonanoda.warmCacheOnOpen`（**默认关**），打开工作区时后台
  跑一次 `sokonanoda build <项目根>`。
- **判据**：stub 宿主测试 + 真宿主 e2e 一例。
- **风险**：默认开会让"打开编辑器"变慢（正是用户抱怨的另一面）⇒ 默认必须关。

### ✅ 检查点 CP-A（批次 2，minor 版本）

- [ ] `bash scripts/verify-editor-issues.sh` 第 1/2/3 条转「已修」
- [ ] unit01/08/12 + unit12 解答的 `didOpen → 诊断` 降到同量级（表更新）
- [ ] 冷/热 `--json` 逐字节一致（带 `sorry` 的项目）
- [ ] 跨文件 definition/references/rename/`soko/project` 在命中后仍正确
- [ ] `scripts/perf-ledger.sh` 记录完成，`docs/PERF.md` 有对比表
- [ ] `scripts/soko gate` exit 0
- [ ] **性能无退化**：`python3 scripts/perf-compare.py --since <上一检查点>` exit 0
      （任何 `best_ms` 退化 > 25% 必须解释或修回；新 case 提示无基线不算红）
- [ ] **用户验收**：重启 VS Code 后打开 3 个 unit 文件，第一次也应明显变快

---

### ✅ 检查点 CP-K（批次 5，minor 版本）

- [ ] `scripts/kernel-check.sh` 全绿（含全语料逐字节对拍）
- [ ] `unit12-solution` 编译 **36.1s → 个位数秒**（数字进 `docs/perf/ledger.jsonl`）
- [ ] 打开 unit01→unit08→unit12 的总耗时接近"公共库只编一次"
- [ ] `build courses/set-theory` 不再 O(文件数 × 闭包)
- [ ] 课程门禁计数**逐项不变**；`cargo test --workspace --locked` 全绿
- [ ] **性能无退化**：`python3 scripts/perf-compare.py --since <上一检查点>` exit 0
      （任何 `best_ms` 退化 > 25% 必须解释或修回；新 case 提示无基线不算红）
- [ ] **用户验收**：在 VS Code 里打开 unit12 解答，不再等半分钟

## 6. 线 C：goal / 类型行用记法（反馈 5）

> **两条红线**（第二条是内核解冻后调研新发现的，推翻了"改内核 pp 更干净"的直觉）：
> 1. **`render_expr`（`crates/front/src/proof.rs:226`）不能改**——它的产物
>    **同时是 judge 的回读输入**（`proof.rs:262` 注释明写；`render_expr_round_trips`
>    在 `crates/front/src/compile/tests.rs:1474` 钉住）。
> 2. **`pp_expr`（内核）也不能改**——它**同时是 `#check`/`#reduce`/`#print` 的出口**
>    （`kernel_phase.rs:337-343/360-366/383`）⇒ 直接进 `--json` 的
>    `expr.typed`/`expr.reduced`/`decl.printed`。**改它就会动 `--json` 字节**，
>    与"判定正确性不变（golden 与 `--json` 逐字节不变）"直接冲突。
>
> ⇒ **结论：记法绝不能从内核 pp 走。** 唯一安全的缝是**显示出口之后**做重写
> （见 §6.0）。

### 6.0 C-0 前置与已定方案（调研结论）

#### 发现 A：内核的记法打印是**死代码**（所以"填个表就行"是幻觉）

- `ExportFile.notations`（`util.rs:626`）只在 `builder.rs:53`、`util.rs:649`、
  `parser.rs:727` 被 `new_fx_hash_map()` 初始化，**全仓库无一处 insert**；
  `Notation::new_prefix/new_infix/new_postfix`（`env.rs:196-208`）**零调用者**
  ⇒ `pp_app` 的记法分支（`pretty_printer.rs:652-683`）**永不触发**。
- 就算填表也不命中：`pp_app` 要求 `args.len()` **恰好 1/2**，而 front 的 `∈`
  展开成 `Set.mem α a A`（**3 个实参**：`elab.rs:1296-1316` 先补 `prefix_args`
  再补操作数）；零元记法（`∅`）走 `pp_const` 不走 `pp_app`。
- 已有的 Infix 分支**取操作数顺序是反的**（`:670-678` 取 `lhs=args[len-1]`/
  `rhs=args[len-2]`，与 `unfold_apps_pp`（`:639-650`）的自然顺序相反）
  且**零测试覆盖**；`priority - 1`（`:662/669/677`）在 `priority=0` 时 usize 下溢。
- 结论：**内核那条路是"看起来现成、实际全是坑"**。真要走内核，最小形态是
  T-K32 之后再加 `Notation.leading_args` + 一个**只接显示出口**的
  `pp_expr_with_notation`（`#check`/`#reduce`/`#print` 继续走 `pp_expr`）——
  **不推荐**，因为它把显示逻辑劈成两半。

#### 已定方案 K3-a：**front 侧的显示边界重写**（零内核改动）

- **设计已经写好了**：`docs/notes/course-lean-style/printback-feasibility.md` §4 ——
  5 个落点（`query/mod.rs` 的 `ty`/`goal`/`reduce`、`lsp/render.rs:178` 的
  `expr_hover`、`lsp/lib.rs:1145/1429-1433` 的签名与补全），
  **明确不落** `compile/**`、`judge.rs`/`by.rs`/`proof.rs`/`spine.rs`/`suggest.rs`、
  以及 `CheckEvent::TypeChecked/Reduced`；数据结构 `DisplayNotations{table, arity}`；
  护栏是 `struct DisplayText(String)`（**无 `Deref`/`as_str`**，
  让 `parse_expr_text(&display_text)` **编译不过**）。
  量级 **250–400 行 / 5 文件**。
- `docs/design/course-lean-style.md:605`（SP2）当年已判"**有条件做，且推荐做**"；
  唯一的反对理由是"内核冻结"（`notation-subset.md:554`、`course-lean-style.md:1124`），
  **该理由已于 2026-09-21 失效**。
- **核心难点 = arity**：只有 `spine.len() == arity` 才是记法实例，
  否则 `Set.mem α a` 会被误打成 `α ∈ a`。arity 来源见 C5/T-C11。
- **前置**：T-K32（`pretty_printer.rs` 零单测）——因为 pp 文本的形状是这条路的输入。

### 根因

见 §2.5 的四个生产者表，以及：

- **C1** 记法在 elab 期被**源到源**展开成 `App`；elaborate 后的 AST 里没有记法。
- **C2** `Expr::Notation { symbol, target, assoc, lhs, rhs, alternatives, span }`
  （`crates/front/src/ast.rs:95-105`）**只活在解析期 AST**。
- **C3** **两条红线**同 §6 抬头（`render_expr` 与 `pp_expr` 都不能改）。
- **C4** 记法表**不需要新建**：`crates/front/src/judge.rs:322-355` **已经**会从
  `prefix_src` 重建 `Vec<NotationDecl>`（并按 `Command::Open{scoped:true}` 过滤
  `scoped`）；`prefix_src` 在 `run_by`/`apply_tactic` 全程在作用域里
  （`check/mod.rs:592-605` → `walk.rs:143-160`），check 阶段
  `units[*].file.commands` 也还带着 `Command::Notation`。
  ⇒ **复用这一处**（提成公共函数），不要另造一套表。
- **C5** `NotationDecl`（`ast.rs:168-180`）**没有 arity**：把 `Set.mem α a A` 折回
  `a ∈ A` 必须知道"丢掉前导隐式 `α`"。前端有 `leading_implicit_prefix` /
  `implicit::telescope`，但没挂在 `NotationDecl` 上；现成匹配器是 `judge.rs` 的
  `unify_extract` / `spine::peel_pi`（吃**源级**签名）。
- **C6** pp 是**有损**的（`Eq.{1} (Set α) A B` → `Eq A B`），既有护栏是
  `by.rs:577-640`（`is_rereadable`、`restore_universe_levels`）与
  `by.rs:534-548`（`keep_if_lossless`）⇒ 折叠层必须尊重它们。
- **C7** **没有任何测试钉住"goal/type 文本里有记法"**（grep 断言里的 `∈`/`⊆` = 0），
  所以记法保留路径（生产者 2）**目前无守护**。
- **C8**（调研补充）**内核 pp 文本会被 front 回读**：`judge.rs:909`
  （`judge_render_type` 用**不带记法表**的 `parse_expr_text`）、`by.rs:511-520`、
  `by.rs:565` ⇒ pp 一打印记法，这些回读点会**静默退化**（返回 `None` 退回源 AST）。
  这是"改内核 pp"的又一条否决理由，也是 `DisplayText` 护栏要存在的原因。

### 6.1 C-I 量具与审计

#### T-C01 四个生产者 × 真实文件的实测表

- **改什么**：写 `docs/design/notation-aware-printing.md` 的第一节：对 unit01/08/12
  与对应解答，逐 `sorry`/逐 `by` 步记录"这个 surface 的文本有没有记法"，
  用 §2.5 的四个生产者归类。
- **判据**：表里每个格子都有实测输出；明确"用户说的'goal 没记法'是哪一个 surface"。
- **依赖**：无。**这一步先做，避免修错 surface。**

#### T-C02 消费者审计（**不然会静默改坏判卷**）

- **改什么**：把 `DeclState.ty_text` / `DeclState.goal` / `DeclInfo.ty` /
  `GoalBinder.ty` / `ByGoal.ty` 的**全部**消费者列出来，分两类：
  **给人看**（Infoview、hover、树 tooltip）与**回读**（`judge_terms`、
  `judge.rs::fold_declared:1284`、`wrap_binders:1327`、`proof::parse_expr_text_with:53`、
  `canonical_goal_with_spec`）。
- **判据**：每条都有 `file:line`；**回读类一个都不许被改到**。

#### T-C03 设计文档：**把已有的 `printback-feasibility.md` §4 升格为权威设计**

- **改什么**：**不要另写一份新设计**——`docs/notes/course-lean-style/printback-feasibility.md`
  §4 已经写好了方案（5 个落点、`DisplayNotations{table, arity}`、`DisplayText` 护栏、
  "明确不落"清单）。本环节做三件事：
  ① 把它从 `docs/notes/` 升格到 `docs/design/notation-aware-printing.md`
  （或直接在 `docs/design/notation-subset.md` 加一节并链接），
  **更新掉两处已失效的反对理由**（`notation-subset.md:554`、
  `course-lean-style.md:1124` 的"内核冻结"）；
  ② 把 §6.0 的**发现 A**（内核记法打印是死代码 + 5 个坑）与**发现 B**
  （`pp_expr` 同时是 `#check`/`#reduce` 出口 ⇒ 动 `--json` 字节）
  写成"**为什么不走内核 pp**"一节——**这是防止后人再走一遍弯路的记录**；
  ③ 把 **arity 判据**写成硬规则：只有 `spine.len() == arity` 才是记法实例。
- **判据**：文档 review 通过；`docs/design/notation-subset.md` 里能查到
  "显示重写不走内核 pp"的结论与两条发现。
- **依赖**：T-C01、T-C02、**T-K32**（pp 零单测，pp 文本形状是这条路的输入）。

#### T-C03b `DisplayText` 护栏（**先于任何折叠代码**）

- **改什么**：定义 `struct DisplayText(String)`——**无 `Deref`、无 `as_str`、
  无 `Into<String>`**，只提供 `as_display_str()`。目的是让
  `parse_expr_text(&display_text)` / `judge_terms(…, &display_text)` **编译不过**
  （`printback-feasibility.md` §4 的核心护栏）。
- **判据**：故意写一行 `parse_expr_text(&display_text)` → **编译失败**
  （这条"反向测试"就是护栏的验收）。
- **依赖**：T-C03。

#### T-C04 记法表复用 `judge.rs` 的重建（提成公共函数）

- **改什么**：把 `crates/front/src/judge.rs:322-355` 的表重建提成
  `pub(crate) fn notation_table(prefix_src: &str, options: &CompileOptions) -> Vec<NotationDecl>`
  （或等价物），两处共用。
- **判据**：`cargo test -p sokonanoda-front -- --nocapture` 全绿（**行为不变**，
  纯提取）。

### 6.2 C-II 折叠层

#### T-C10 折叠函数第一刀：只做二元 infix 族

- **改什么**：`App(App(f, a), b)` 且 `f` 命中表 ⇒ `a ∈ b`；处理结合性与优先级
  所需的括号；**剥掉隐式前导实参**；命中不了就**回退点名**（不猜）。
- **判据**：`cargo test -p sokonanoda-front display -- --nocapture`
  新增 5 条：左结合 / 右结合 / 优先级加括号 / 嵌套 / 不命中回退点名。

#### T-C11 arity 的来源

- **改什么**：给折叠层提供"记法吃几个显式参数"。来源二选一并写进设计：
  ① 目标声明的**源级签名**里非隐式参数个数（`KnownName` 已存源级签名，
  `crates/front/src/compile/elab.rs:515/557`）；② `report.decls[i].ty_text`/`signature`。
- **判据**：单测：`infix:50 " ∈ " => Set.mem` 的 arity = 2（`α` 是隐式前导）。

#### T-C12 `scoped` 的保真度

- **改什么**：`judge.rs:332-355` 的近似是"前缀里出现过 `open scoped` 就算生效"。
  折叠层要不要位置精确（对齐 parser 的 `opened_scopes`，`parser.rs:643-680`）？
  决定并写进设计；若要，加测试。
- **判据**：决定 + 测试。

#### T-C13 重载（一个符号 → N 个目标）的处置

- **改什么**：决定折叠层遇到重载时"折叠成哪一个"或"回退点名"。
- **判据**：决定 + 测试。

#### T-C14 折叠层的损失护栏

- **改什么**：折叠不得让文本变得**不可回读**（若折叠结果会进入任何回读通道）。
  复用 `is_rereadable` / `keep_if_lossless` 的既有判据。
- **判据**：`cargo test -p sokonanoda-front -- --nocapture` 绿；
  `render_expr_round_trips`（`crates/front/src/compile/tests.rs:1474`）**必须不动**。

### 6.3 C-III 接进四个生产者

#### T-C20 生产者 1+3：根状态与声明列表的 `ty_text`

- **根因**：`ty_text` 来自内核 pp（`kernel_phase.rs:170-178`/`:207-213`）。
- **改什么**：在 `ty_text` 出口做折叠（**只影响展示字段**，或新增
  `ty_display` 字段并让消费者切换——由 T-C02 的审计决定）。
- **判据**：
  ```bash
  ./target/debug/sokonanoda query goals --file courses/set-theory/units/unit01-sets-membership.sokonanoda \
    | python3 -c "import json,sys; d=json.load(sys.stdin)['data']; print([x['ty'] for x in d][:2])"
  # 期望：含 `⊆`、`↔`
  ```
- **风险**：`docs/protocol.md:337-339`/`:434` 把 `ty` 定义为"内核渲染的类型"，
  是**契约**；若新增字段而非改 `ty`，契约不动（推荐）。

#### T-C21 生产者 2：无 `by` 的开练习（应当已经好，补守护）

- **改什么**：T-C01 的实测若显示这一路已经保留记法（§2.5 的 unit01 例就是），
  则**补测试钉住**（现在零覆盖：断言里的 `∈`/`⊆` grep 命中 0）。
- **判据**：新增测试：`soko/goals` 的 `decl.goal` 含记法。

#### T-C22 生产者 4：`by` 步进里被 pp 化的四处

- **改什么**：对 `canonical_goal_type`（`by.rs:497`）、`canonical_goal_with_spec`
  （`by.rs:553`）、`apply`（`by.rs:1277`/`:1413`）、`cases`（`by.rs:1450+`）
  的**出口文本**做折叠。
- **判据**：
  ```bash
  bash docs/gaps/repro/G26-goal-text-loses-notation.sh ; echo "exit=$?"   # 期望 1
  ```
- **风险**：这四处同时是**判定输入**（子目标要回读）⇒ 折叠只能作用在
  **展示副本**上，绝不能改判定用的那一份。**这是本条最容易出错的地方，
  必须单独立测试。**

#### T-C23 binder ty（假设行的类型）

- **判据**：`query state` 的 `binders[].ty` 含记法。

#### T-C24 逐 surface 的判别性测试

- **改什么**：四个生产者**各一条**测试，断言"该 surface 含记法"；
  并在折叠层被关掉时**必须红**（判别性）。
- **判据**：`cargo test -p sokonanoda-front -- --nocapture`。

#### T-C25 边界：命中不了就回退

- **改什么**：`prefix` / `postfix` / 零元 `notation` / binder 记法 / 重载歧义
  ⇒ 回退点名，不猜。每种一条测试。
- **判据**：`cargo test -p sokonanoda-front -- --nocapture`。

### 6.4 C-IV 语义着色（独立缺陷）

#### T-C30 `semantic::tag_runs` 填 `Names::notations`

- **根因**：§2.7 第 2 条。
- **改什么**：`crates/front/src/semantic.rs:240` 建 `Names` 时补 `notations`
  （对齐 `classify` 在 `:544` 的做法）。
- **判据**：wire 上 `∈` 的 run 有 `kind`（不再是裸 `{"text":"∈"}`）；
  `cargo test -p sokonanoda-front semantic -- --nocapture` 新增断言。

#### T-C31 目标文本里的**导入名**不再标 `unknown_ident`

- **根因**：`decl_kinds()` = `declaration_kinds(&self.text)`（`query/mod.rs:274`）
  只看入口文件 ⇒ 项目文件里 `Set.mem`/`Set`/`α` 全是 `unknown_ident`。
- **改什么**：把闭包级声明表喂进 runs 计算。
- **判据**：unit 文件的 `goal_runs` 里 `Set.mem` 有正确 `kind`。

#### T-C32 着色在 Infoview 里可见

- **判据**：真宿主 e2e 或 `test-webview.js` 的 runs 断言。

#### T-C50 真宿主 e2e：goal 文本用记法（矩阵用例 #6）

- **改什么**：新增用例 `goal text uses the file's notation`：把光标放到夹具
  `units/u01.sokonanoda` 的 `sorry` 上，断言 `infoview.lastState().goal`
  含 `∈` 或 `⊆`（修复前必须是红的）。
- **判据**：`scripts/vscode-e2e.sh --grep "goal text uses the file's notation" --profile debug --no-build`
  → 1 passing。
- **依赖**：T-015、T-016、T-017、T-C24。

### 6.5 C-V 收尾

#### T-C40 断言与 golden 更新（**计数中性**）

- **改什么**：列出所有断言 goal/ty 文本的测试（front / lsp / cli / 课程门禁），
  逐条按**实测**重钉（不做机械替换——仓库纪律）。已知清单（来自调研）：
  `crates/lsp/src/tests/state.rs`（`:20`/`:120`/`:150-154`/`:221-224`/`:242`/`:258`）、
  `crates/lsp/src/tests/goals.rs`（`:18`/`:21-22`/`:133-136`/`:165-167`/`:194`）、
  `crates/front/src/query/tests.rs`（`:46-48`/`:83`/`:124`/`:152`）、
  `crates/cli/tests/query.rs`（`:159`/`:943`/`:967-973`/`:1118`/`:1140`）、
  `crates/front/src/compile/tests.rs`（20+ 处，含 by-step 与 sub-goal 两组）、
  `crates/front/src/proof.rs`（`:644-780`）。
- **判据**：
  ```bash
  cargo test --workspace --locked                      # 全绿
  python3 courses/set-theory/tools/check.py            # 计数逐项不变：36 目标 · 328 checked · 99 open · 0 判负
  scripts/soko gate                                    # exit 0
  ```
- **风险**：**显示层改动不得改变判卷**。课程门禁计数必须逐项不变。

#### T-C41 文档 + CHANGELOG

- **改什么**：`docs/design/goal-rendering.md` as-built；
  `docs/design/notation-subset.md` 补"渲染"一段（N7 的边界要更新）；
  `editor/vscode/CHANGELOG.md`。

### ✅ 检查点 CP-C（批次 3，minor 版本）

- [ ] `bash scripts/verify-editor-issues.sh` 第 5 条转「已修」
- [ ] 四个生产者的判别性测试全绿
- [ ] 课程门禁计数**逐项不变**
- [ ] `cargo test --workspace --locked` 全绿
- [ ] **性能无退化**：`python3 scripts/perf-compare.py --since <上一检查点>` exit 0
      （任何 `best_ms` 退化 > 25% 必须解释或修回；新 case 提示无基线不算红）
- [ ] **用户验收**：Infoview 里的目标/假设/声明类型与自己写的一致

---

## 7. 线 D：记法跳转 + hover 原始类型（反馈 6）

### 根因

- **D1** 记法使用处在 elab 里**硬编码** `resolution: None`
  （`crates/front/src/compile/elab.rs:2914`：`record_hover(hovers, scope, *span, out, None)`）。
- **D2** `definition_at` **就是** `hover_type_at(…).resolution`，**没有回退**
  （`crates/lsp/src/render.rs:306-314`）⇒ `Ok(None)`。设计自己早就写了这一刀不做：
  `docs/design/notation-subset.md:208`「`resolution`（转到定义）v1 留空」、
  `docs/gaps/WO-011-notation.md:199,201`「转到定义留第二刀」——第二刀/第三刀/课程
  Lean 化都没捡起来，台账里也没有对应缺口（G-04 已 `fixed`）。
- **D3** 记法**符号 token 的 span 在 parser 里就被丢掉**：
  `bump_operator`（`parser.rs:2248-2251`）字面就是 `let _ = op; self.bump();`；
  `notation_node`（`parser.rs:2260-2282`）根本没有能接收符号 span 的参数。
  `Expr::Notation` 只有复合 span（`Span::new(lhs.start, rhs.end)`，`parser.rs:2141`）。
- **D4** 记法**声明的 span 只活在 AST 里**：`Command::Notation.span`
  （`ast.rs:510-518`，parser 在 `:735/772/806/840` 设为整条命令）；
  `NotationDecl`（跨模块那份）**明确"不含 span"**（`ast.rs:163`）；
  `compile_closure` 只把 `module.file.src` 拷进 `ModuleReport.source`
  （`project/mod.rs:330`/`:340`），AST 丢掉。`Session`/`QueryDoc`/`DocumentReport`
  都没有记法字段。`project/graph.rs:92` 的 `exports: HashMap<String, Vec<NotationDecl>>`
  在 `:107-116` 建 `Closure` 时被丢弃。
- **D5** **`ResolvedTarget` 的陷阱**：报告装配时**每一条**
  `ResolvedTarget::Declaration` 都会被 name→def-span 表**覆写**
  （`crates/front/src/compile/check/kernel_phase.rs:419-429`），而
  `top_level_def_spans` **不含记法命令**（`check/mod.rs:710`）。
  ⇒ 记法行若复用 `Declaration` 变体，会被**静默改写**成 `def Set.mem` 的 span
  （或 prelude 目标变成 `None`）。**必须新增一个变体**（如
  `ResolvedTarget::Notation { symbol, span, module }`）。
- **D6** hover 的 `target` 只认**本文件**声明的记法
  （`notation_input.rs:261-267`；`symbol_at` 返回 `(String, Option<String>)`，
  无 span、无 fixity、无候选表；`token.rs:679` 还按符号去重 ⇒ **重载只暴露第一个**）。
- **D7** `Set.mem` 的签名**已经算过又被丢掉**：`elab.rs:1228`
  `judge::judge_type_of_constant(ctx.prefix_src, ctx.options, &canonical)`
  （用在 `:1242`/`:1261`），而该函数是 **`pub`**（`crates/front/src/judge.rs:621-642`）
  ⇒ **hover 想要的那行文本只差一次公开 API 调用**（外加 `QueryDoc::judge_prefix`
  `query/mod.rs:184-198` 与 `QueryDoc.mode` `query/mod.rs:47`，都在手边）。
- **D8** `NotationDecl` **没有从 `sokonanoda_front` re-export**
  （`crates/front/src/lib.rs:39-44`），`parse_with_inherited` 也没有 ⇒
  任何"把记法表交给 LSP"的方案都得先补 re-export。
- **D9** hover 在**声明行本身**（`infix:50 " ∈ " => Set.mem`）上没有反应：
  那里是 `Str` token（`notation_input.rs:255`），且命令没有 hover 行（`walk.rs:223`）。

### 7.1 D-I 最便宜的一刀：hover 原始类型

#### T-D01 复现脚本（= T-003，已建）

- **判据**：`bash docs/gaps/repro/G23-notation-navigation.sh ; echo "exit=$?"` → 修前 `0`。

#### T-D02 hover 增加"原始类型"行

- **改什么**：`notation_symbol_hover`（`crates/lsp/src/lib.rs:809-858`）在
  `target` 已知时，用 `judge::judge_type_of_constant(query.judge_prefix(offset),
  query.mode, &target)` 取签名并渲染一行。
- **判据**：hover 含 `` `Set.mem : forall (α : Type 0), α -> Set α -> Prop` ``（文本以实测为准）。
- **成本**：**这是全计划最便宜的一刀**（一个 `pub` API 调用 + 一行文本）。
- **风险**：`judge_type_of_constant` 走内核 ⇒ 请求路径成本；它自带常量键缓存
  （`judge.rs:596-608`），但要按 `docs/design/spine-meta-a.md` 的请求路径纪律测一次。

#### T-D03 hover 的"原始类型"只对**能解析出 target** 的符号显示

- **改什么**：本文件声明 / 内建 / **import 来的**三种都要能解析出 target
  （import 那种依赖 T-D10）。解析不出就**不显示**这一行（不编）。
- **判据**：三种各一条 hover 测试（`crates/lsp/src/tests/hover.rs`）。

### 7.2 D-II 记法解析走闭包

#### T-D10 `NotationDecl` re-export + 补 `span`/`module`

- **改什么**：① `crates/front/src/lib.rs` re-export `NotationDecl`
  （以及需要的话 `parse_with_inherited`）；② `NotationDecl` 增加声明点信息
  （`span` + 所属模块）。**注意它是跨模块传播的纯数据，加字段要考虑
  `project/graph.rs` 的 `absorb_notations`（`:119-134`）与
  `Command::notation_decl()`（`ast.rs:649`）两处构造点。**
- **判据**：单测：`infix:50 " ∈ " => Set.mem` 的 span 逐字等于那一行。

#### T-D11 闭包级记法表进 `ProjectReport`/`QueryDoc`

- **改什么**：闭包编译时收集每模块的记法声明（符号 → 声明点 + 模块），
  存进报告层（现在 `graph.rs:92` 算了又丢）。
- **判据**：单测：unit01 的记法表里 `∈` 指向 `lib/Set.sokonanoda` 的第 N 行。

#### T-D12 解析 API：`notation_resolve(text, table, offset)`

- **改什么**：新 API 返回 `{ symbol, target, decl_span, module, fixity, precedence,
  candidates }`（比 `symbol_at` 的 `(String, Option<String>)` 完整）。
  保留 `symbol_at` 给不需要这么重的调用方，或让它变成薄包装。
- **判据**：单测：import 来的 `∈` 解析出 `target = "Set.mem"` + 声明点；
  重载符号返回全部候选。

#### T-D13 hover 的"展开成"（import 来的记法）

- **判据**：hover 含 `` 展开成 `Set.mem` ``。

### 7.3 D-III go-to-definition

#### T-D14 parser 保留记法符号 token 的 span

- **改什么**：`Expr::Notation` 增加 `symbol_span: Span`；
  `bump_operator`（`parser.rs:2248-2251`）改为返回 `Token`；
  `notation_node`（`parser.rs:2260-2282`）加参数；infix 族（`:2141`）、
  prefix（`:2351`）、postfix（`:2082`）、binder（`:2875-2911`）四处都填。
  零元记法（`:2254-2256`）本来就等于节点 span。
- **判据**：单测：`a ∈ A` 的 `Expr::Notation.symbol_span` 只覆盖 `∈` 三个字节。
- **风险**：**AST 变更**；`render_expr` 的 `Expr::Notation` 分支
  （`proof.rs:363-385`）与所有 `match` 到该变体的地方都要跟着改
  （编译器会全指出来）；`crates/front/src/compile/tests.rs:6658`
  `notation_records_a_hover_row_covering_the_whole_notation` 要补断言。

#### T-D15 新增 `ResolvedTarget` 变体并绕开覆写

- **改什么**：`ResolvedTarget`（`crates/front/src/compile/report.rs:128-134`）
  增加 `Notation { symbol, span, module }`；`elab.rs:2914` 把
  `None` 换成该变体（至少对**符号 span 命中**的情形）；
  **确认 `kernel_phase.rs:419-429` 的覆写只作用于 `Declaration` 变体**
  （这是 D5 的坑，必须加测试）。
- **判据**：单测：记法 hover 行的 `resolution` 在报告装配**之后**仍是记法声明点，
  没有被改写成 `def Set.mem` 的 span。

#### T-D16 LSP `definition` 处理记法变体

- **改什么**：`crates/lsp/src/lib.rs:1334-1364` 的 definition handler 认识新变体；
  跨模块时用 T-D11 的表把 `module` + `span` 变成 `Location`。
- **判据**：`bash docs/gaps/repro/G23-notation-navigation.sh` → `1`；
  `cargo test -p sokonanoda-lsp navigation -- --nocapture` 新增用例绿。

#### T-D17 hover 的 `range` 收窄到符号本身

- **改什么**：现在 `notation_symbol_hover` 返回 `range: None`
  （`lib.rs:854-856`）⇒ 客户端按"光标词"高亮。有了 `symbol_span` 后可以给精确 range。
- **判据**：hover 返回的 range 只覆盖符号（测试断言）。

### 7.4 D-IV 边界与一致性

#### T-D20 内建/ prelude 目标的定义跳转怎么办

- **问题**：`∧`→`And`、`=`→`Eq` 是内核 prelude 名，**没有源码声明**
  （`top_level_def_spans` 按构造排除 prelude，`check/mod.rs:668`）。
- **改什么**：决定并落地：跳到记法声明点（如果有）/ 返回 `null` / 跳到别的地方。
- **判据**：决定写进设计 + 测试。

#### T-D21 跨文件记法的"定义"是哪个

- **问题**：`∈` 声明在 `lib/Set`，用在 unit。定义 = 库的 `infix` 行，
  还是入口的 `import` 行？（设计 §10.3 只说记法随 import 传播，没说定义在哪。）
- **改什么**：决定（推荐库的 `infix` 行）+ 测试。

#### T-D22 `scoped` 记法的导航是否尊重作用域

- **问题**：`notation_input` 刻意忽略 scoping（`docs/design/notation-input.md` §10 偏差 3）
  ⇒ 不在作用域的符号也会被当成在作用域。导航要不要跟？
- **改什么**：决定 + 测试。

#### T-D23 重载：一个 `Location` 还是 N 个

- **改什么**：决定 + 测试（LSP 的 `definition` 可以返回数组）。

#### T-D24 `documentHighlight`/`references`/`rename` 覆盖记法符号

- **改什么**：实现或明确写"不做"（给出理由）。
- **判据**：决定写进 `docs/design/notation-subset.md`；若做，加测试。

#### T-D30 记法符号不再误解析到外层 binder（**独立正确性 bug**）

- **根因**：§2.7 第 1 条。
- **改什么**：`render.rs:327-331` 与 `references.rs:57-60` 的"包含光标即命中"回退
  必须排除"光标落在记法符号上"的情形（或改成"命中最近的最小 span"）。
- **判据**：单测：光标在 `(h : a ∈ A)` 的 `∈` 上时
  `documentHighlight`/`references`/`rename` **不得**返回 `h`；
  `rename` 不得改 `h`。
- **风险**：这条修的是**现有错误行为**，可能让某些 rename 用例变红——那正是要找的。

#### T-D31 `position_to_offset` 的 UTF-16 语义（**独立缺口**）

- **根因**：§2.7 第 3 条（`crates/lsp/src/lib.rs:615-629`；`docs/protocol.md:806-811`
  已记为独立缺口）。
- **改什么**：本轮**只立台账 + 写判定实验**（用 `𝒫` 造一个偏离用例）；
  真正修（改成 UTF-16 计数）单独立项，因为它会动所有位置映射与测试夹具。
- **判据**：`docs/gaps/ledger.jsonl` 新增一条；实验脚本能稳定复现偏移。

### 7.5 D-V 三层测试与文档

#### T-D40 三层测试（矩阵用例 #7/#8）

- **改什么**：front（`resolution` 不被覆写）、LSP（hover/definition/highlight）、
  真宿主 e2e 两条：
  7. `go to definition on a notation symbol lands on its declaration` ——
     `vscode.executeDefinitionProvider` 在夹具 `units/u01` 的 `∈` 上返回非空，
     uri 指向 `lib/Set.sokonanoda`；
  8. `hover on a notation symbol shows the target's signature` ——
     `vscode.executeHoverProvider` 的 markdown 含 `Set.mem` 的签名。
- **判据**：
  ```bash
  scripts/vscode-e2e.sh --grep "go to definition on a notation symbol" --profile debug --no-build
  scripts/vscode-e2e.sh --grep "hover on a notation symbol" --profile debug --no-build
  # 两条在修复前都必须先跑一次确认是红的
  ```

#### T-D41 文档同步

- **改什么**：`docs/design/notation-subset.md` 的"编辑器支持"段 as-built
  （更新 §4/§13 与 `:208` 的"留空"话术）；`docs/protocol.md`（若新增请求/字段）；
  `docs/TESTING.md` 的守护表；`editor/vscode/README.md` + `CHANGELOG.md`；
  `skills/` 三个技能同轮。
- **判据**：`cargo test -p sokonanoda-cli --test skill` 绿。

### ✅ 检查点 CP-D（批次 4，minor 版本）

- [ ] `bash scripts/verify-editor-issues.sh` 第 6 条转「已修」
- [ ] 真宿主 e2e 全绿（含新用例）
- [ ] `rename`/`references` 不再误伤外层 binder（T-D30）
- [ ] `scripts/soko gate` exit 0
- [ ] **性能无退化**：`python3 scripts/perf-compare.py --since <上一检查点>` exit 0
      （任何 `best_ms` 退化 > 25% 必须解释或修回；新 case 提示无基线不算红）
- [ ] **用户验收**：在 `∈` 上 F12 跳转、悬停看到 `Set.mem` 的签名

---

## 8. 顺序、依赖与检查点总表

```
阶段 0   T-001 ─┬─ T-002 ─┐
                ├─ T-003 ─┤
                ├─ T-004 ─┼─ T-008（验收命令）
                ├─ T-005 ─┤
                ├─ T-006 ─┘
                ├─ T-007 ─ T-020（真实课程 perf 套件）
                ├─ T-009（缓存键实验）──► 决定 T-A02 是否必要
                ├─ T-010
                ├─ e2e 基建  T-015 ─ T-016 ─ T-017 ─ T-018 ─ T-019
                └─ 性能基建  T-020 ─┬─ T-021（单场景快跑）
                                    └─ T-022（回归比较器）─ T-023（DoD + CI）

线 B     T-B01 ─ T-B02 ─ T-B03 ─┬─ T-B04 ─ T-B05 ─┬─ T-B06 ─ T-B15
                                │                 ├─ T-B07
                                │                 └─ T-B14
                                ├─ T-B08 ─ T-B09 ─ T-B10 ─ T-B12 ─ T-B13
                                └─ T-B11

线 A     T-A01 ─ T-A02 ─ T-A03 ─ T-A04 ─ T-A05 ─ T-A06 ─ T-A07
         T-A10 ─┬─ T-A11 ─ T-A12 ─ T-A13 ─ T-A14 ─ T-A15
                ├─ T-A23
                └─ T-A30
         T-A20 / T-A21 ─ T-A22 / T-A24 / T-A25        （与缓存并行）
         T-A60（缓存/扇出的 e2e）
         T-A50 ─ T-A51    T-A52（可选）

线 C     T-C01 ─ T-C02 ─ T-C03 ─┬─ T-C03b（DisplayText 护栏）─ T-C04 ─ T-C10 ─┬─ T-C11..T-C14
                                │                 （前置：T-K32）            └─ T-C20..T-C25
                                └─ T-C30 ─ T-C31 ─ T-C32
         T-C40 ─ T-C41    T-C50（goal 记法的 e2e）

线 D     T-D01 ─ T-D02 ─ T-D03                       （最便宜，先做）
         T-D10 ─ T-D11 ─ T-D12 ─┬─ T-D13
                                ├─ T-D14 ─ T-D15 ─ T-D16 ─ T-D17
                                └─ T-D20..T-D24
         T-D30 / T-D31（独立）    T-D40（含 e2e #7/#8）─ T-D41

线 K     T-K01 ─ T-K02 ─ T-K03 ─┬─ T-K10 ─ T-K11（K1-a，纯 front，零内核风险）
                                │            └─ T-K12（K1-b，with_env）─┬─ T-K13（K1-c 备选）
                                │                                     └─ T-K20 ─ T-K21… ─ T-K30
                                └─ T-K31（TcCache 4MB，独立）   T-K32（pp 单测，线 C 前置）
         T-K40 ─ T-K41
```

**建议执行顺序**（按"收益 ÷ 风险"排，来自调研结论）：
`CP0` → 线 B（批次 1）→ 线 A（批次 2）→ 线 C（批次 3）→ 线 D（批次 4）→ **线 K（批次 5）**。

线 K 内部的顺序是**硬约束**，不要打乱：
1. **T-K01/T-K02/T-K03**（护栏 + profile）——没有护栏不许动内核；
   而且**现在的语料对拍只有 1 个用例**，先把 `arena.rs` 的收集逻辑修了再谈对拍。
2. **T-K11（K1-a）**——纯 front、零内核风险、复用已有逐字节对拍机制，
   先拿掉"内核检查"那一段。
3. **T-K12（K1-b）**——单次内核改动里性价比最高，直接把 36.1s 推到 term 基线，
   **并且是 T-K20 的基座**。
4. **T-K20…**（K2-b 先、K2-a 后）；**T-K31 可随时并行**（独立低风险）。
5. 线 C 的 **T-K32** 是它自己的前置（pp 零单测）。

理由：线 B 最便宜且立刻消除最刺眼的问题；线 A 收益最大但最需要判据与 T-009 的结论；
线 C 风险最高（碰回读通道边界）；线 D 的 **T-D02（hover 原始类型）可以随时插队**
——它是全计划最便宜的一刀。

**可并行**：线 B 与阶段 0 的收尾；线 A 的 A-III（清浪费）与 A-I/A-II（缓存）；
线 D 的 T-D01..T-D03 与线 C 并行；T-D30/T-D31 独立于其它一切。

---

## 9. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| 改到 `render_expr`（回读通道） | **判卷静默改变**，最坏的失败模式 | T-C02 审计先行；只**新增**折叠层；T-C14 复用 `is_rereadable`；T-C40 要求课程门禁计数逐项不变 |
| 折叠层改到"判定用的那一份"文本（`by` 的四处） | 子目标回读变化 ⇒ 判卷漂移 | T-C22 明确只作用于展示副本；T-C24 判别性测试 |
| 缓存回放不一致 | 同一文件两次打开诊断不同 | T-A04 冷/热逐字节一致（**故意破坏必须判红**）；`CACHE_FORMAT` bump |
| 缓存键含二进制 mtime | **CLI 预热对 LSP 完全无效 ⇒ 线 A 白干** | T-009 先做实验；T-A02 修键 + 独立缺口 G-27 |
| `ResolvedTarget` 覆写陷阱（D5） | 记法跳转被静默改指到 `def Set.mem` | T-D15 新增变体 + 专门的"装配后仍正确"测试 |
| 旧缓存条目被新代码读 | 崩溃或错误回放 | 键前缀已含 `format` + `version`；bump 后旧条目自动 miss |
| 碰内核 | 违反硬规则 | **2026-09-21 内核已解冻**：可改热路径与内部表示，红线只有"判定正确性不变"。每次内核改动跑 `cargo test --workspace --locked` + 全部 `*.sokonanoda` 的 `--json` 逐字节对拍 + 课程门禁计数逐项不变；改前先读 `docs/architecture.md` §6/§8 |
| 项目缓存跨入口不共享（A5） | 用户仍会觉得"换一个文件又要等" | 批次 2 先诚实写进文档与 CHANGELOG（**不假装本轮解决**）；真正的解药是 §5.6 线 K（T-K20…，跨模块增量，内核解冻后转正） |
| `build <目录>` 仍是 O(文件×闭包) | `alt+b` 在大课程上很慢 | T-A25 如实记账 + 扩展文案说明 |
| 编译独占文档锁（A9） | 表现为"编辑器卡死"而非"慢" | T-A30；但排在缓存之后（否则收益有限） |
| 版本 bump 触发自动发版 | 小环节发一堆版本 | 批次制（§0.2） |
| 真宿主 e2e 慢/脆 | 验收成本高 | 单环节只跑 stub 宿主 + 复现脚本；e2e 只在检查点跑 |
| `position_to_offset` 的 UTF-16 偏差 | `𝒫` 之后的位置全偏 | T-D31 立台账；不在本轮修（会动所有位置映射） |

---

## 10. 明确不做的事（避免再次"改得很大却漏得多"）

> 2026-09-21 内核解冻后，原先列在这里的"跨模块增量"、"`by` 前缀重判根治"、
> "`build <目录>` 编一次全项目"**已转正**，进 §5.7 线 K（批次 5）。
> 下面这些仍然不做：

1. **诊断消息里的类型改用记法**：本轮只改 Infoview 的 goal / 类型行（§11 问题 6）。
2. **`position_to_offset` 的 UTF-16 修正**（T-D31）：立台账，单独立项
   （会动所有位置映射与测试夹具）。
3. **重构 `extension.js`（1906 行）/ `crates/lsp/src/lib.rs`（1700 行）**：
   结构债已登记（`docs/HANDOVER.md` §4），但"修 bug 顺带重构"正是上次出纰漏的模式；
   要拆单独开批次。
4. **换掉 `stumpalo` 或重写 arena 层**：线 K 只做"可检查点/可复用"这一刀，
   不换基础设施（换基础设施的风险与收益不成比例）。

---

## 11. 未决问题（需要用户拍板）

1. ~~**批次制 vs 每环节一个版本**~~ —— **2026-09-21 用户拍板：到一定程度就 bump，
   不等批次**（§0.2 的定案）。bump 触发：① 用户可感知的能力落地；② 距上次 bump
   ≥ 8 个环节；③ 用户要拿去测。**13 个 bump 点已标进 §13**
   （`python3 scripts/plan.py bumps`），批次 0 零用户可见改动 ⇒ 不发版。
2. **"慢"主要发生在哪个动作**：打开新文件 / 每次按键 / 保存 / 切文件，对应不同根因
   （A1+A6 / A1+A5 / A7 / A8）。T-009 与 T-007 会给数字，但用户的体感更权威。
3. **优先级**：推荐 `阶段0 → 线B → 线A → 线C → 线D → 线K`。若"慢"比"面板空"更痛，
   把线 A 提到线 B 之前（但那样第一批就碰缓存键与条目形状，风险更高）。
4. **线 K 的先后**：内核解冻后，`by` 前缀缓存与跨模块增量是**收益最大的两刀**
   （36.1s 那个数字的大头）。建议**紧接线 A**（批次 5），不要拖到最后——
   否则线 A 做完你仍会觉得"换一个文件又要等"。
5. **缓存键的 `build_stamp`（T-009）**：若结论是"CLI 预热确实到不了 LSP"，
   是否接受改键（会让现有缓存全部失效一次）？
6. **记法渲染的范围**：`printback-feasibility.md` §4 已经圈定 5 个落点
   （`ty`/`goal`/`reduce`/`expr_hover`/签名与补全），并明确**不落**
   `CheckEvent::TypeChecked/Reduced`。是否接受这个圈定？**诊断消息里的类型**
   （走 `debug_print`，不是 pp，安全）要不要一起改？
7. **记法导航的语义**（T-D20..T-D23）：内建目标（`∧`→`And`，无源码声明）怎么办？
   跨文件记法的"定义"是库的 `infix` 行还是入口的 `import` 行？重载返回 1 个还是 N 个？
8. **语料对拍要不要进 CI**（T-K02）：Lean Kernel Arena 现在**只有 1 个用例被收集**、
   且 CI 里没有 `LEAN_KERNEL_ARENA` ⇒ `cargo test --workspace` 里**静默空过**。
   修好收集逻辑后，是接进 CI（要下载/克隆外部语料、`init`/`std` 是 309MB/526MB），
   还是在文档里明说"本地可选、CI 不做"？

### 11b. 调研留下的实验清单（**做完才知道上界**）

调研给了 9 个必须用实验回答的问题，前 4 个直接决定线 K 的收益与风险：

1. **T-K03**：36.1s 里 parse / elab / **内核检查** / pp 各占多少？
   （决定 T-K11 的收益上界；探针可抄 `by-tactics.md` §13）
2. **T-K11 验收实验**：加 `SOKO_JUDGE_ENV_REUSE=0/1` 开关后，两态在全语料上
   `--json` 是否逐字节相同？
3. `unit12-solution` 实际发生多少次 `judge_pairs_uncached`？
   （调研静态数出 ≈20：9 个声明值位 + 闭包 11 个 `:= by`；用 `judge.rs:481-489`
   的 `pass_count()` 或 §13 的 prefix_bytes 探针实测）
4. **T-K12 验收实验**：`judge_infer`/`judge_type_of` 换成 `with_env` 路径后，
   文本是否与合成 `#check` **逐字相同**？（`EnvLimit` 从 `Empty` 变成
   `ByIndex(k)`；pp 内部自用 `PpUnlimited`，理论上无关，但**必须实测**）
5. `ExportFile` 加 `&ArenaRef` 是否真的让 `check_all_declars_par` 编译失败？
   （预期失败；确认后写进设计文档的"被否方案"）
6. `courses/set-theory` 每个入口闭包的 **prelude 形状是否完全相同**？
   （T-K20 复用守卫的前提；可脚本化验证）
7. LSP 里"每次动作都从零编闭包"是否仍成立？4.59s 里**入口 vs 各 lib 各占多少**？
   （per-module 计时）
8. 内核 Infix 记法的操作数顺序（`lhs=args[len-1]`）是上游约定还是 bug？
   —— 虽然**本计划不走内核 pp**，但若将来要填 `notations` 表，
   **必须先写一条 `And a b` → `a ∧ b` 的内核测试**。
9. `TcCache::new` 每次 `with_ctx` 清 4MB 在真实课程上占多少？（T-K31；
   独立于本任务的低风险提速点）

---

## 12. 收尾义务（每个批次都要做，仓库硬规则）

- [ ] `STATUS.md` 只保留最近 3 轮（旧轮归档 `docs/STATUS-ARCHIVE.md`）
- [ ] `REQUIREMENTS.md` §9 追加本轮要求（带日期）
- [ ] 用户可见改动同轮更新 `editor/vscode/`（README/CHANGELOG/package.json）
      **与** `skills/` 三个技能 + `AGENTS.md` + `docs/vscode-dev-guide.md`
- [ ] **`docs/e2e/ledger.jsonl` 留档并提交**（本批次的 e2e 用例名与数字；
      用例 3/4 的 `t1`/`t2` 也记进去）
- [ ] **`docs/perf/ledger.jsonl` 留档并提交** + `python3 scripts/perf-compare.py`
      对上一批次无 > 25% 退化
- [ ] 缺口台账 `docs/gaps/ledger.jsonl` 关账（`gap.py close` 会先复跑复现）
- [ ] `python3 scripts/notation-lint.py`（课程记法规则）仍绿
- [ ] **`docs/PERF.md` 的"修前/修后"表更新**（§2.1 那张表的修后列）

---

## 13. 执行清单（**线性顺序，逐条勾**）

> **这是唯一的执行入口。** §3–§7 是每个环节的详细规格（判据、依赖、风险），
> 本节是它们的**线性执行顺序**——按批次自上而下走，勾掉一条做下一条。
>
> ```bash
> python3 scripts/plan.py next            # 下一条要做的环节（含完整规格，可直接粘给实现者）
> python3 scripts/plan.py next --json     # 同上，机器可读
> python3 scripts/plan.py list            # 全部环节 + 进度（n/117）
> python3 scripts/plan.py done T-001      # 勾掉一条（写回本节）
> python3 scripts/plan.py check           # 清单与详细规格不漂移（可选：接进 gate 防腐烂）
> ```
>
> **文件地图（"所有计划列在哪"）**：
>
> | 要找什么 | 在哪 |
> |---|---|
> | **执行入口**（下一条做什么） | `python3 scripts/plan.py next` |
> | 每条环节的完整规格 | 本文 §3–§7（判据 / 依赖 / 风险 / 证据） |
> | 六条反馈的实测底账 | 本文 §2（**所有数字的来源**） |
> | 三条机械判据与 DoD | §0.3 / §0.5（e2e 矩阵）/ §0.6（性能检测） |
> | 快速反馈回路与版本策略 | §0.4 / §0.2 |
> | 每环节的缺口复现 | `docs/gaps/ledger.jsonl` + `docs/gaps/repro/G22…G27` |
> | 要求总账（用户原话 + 决定） | `REQUIREMENTS.md` §9 的 2026-09-21 两条 |
> | 路线图指针 | `ROADMAP.md` §10 的 **E1** 小节 |
> | 内核改动台账 + gotchas | `docs/architecture.md` §6 / §8 |
> | 性能台账 / e2e 台账 | `docs/perf/ledger.jsonl` / `docs/e2e/ledger.jsonl` |
> | 内核提速的调研底稿 | 本文 §5.6（结论已并入）+ §11b（9 个待做实验） |
>
> **一条环节的完成定义**见 §0.3（复现判红 → 最小改动 → 判据 → **e2e 转绿**
> → **性能无退化** → 文档 → commit）。**三条机械判据缺一不算完成。**
>
> 批次之间是**发布边界**（§0.2）：批次内不动版本号，批次收尾才 bump 一次。
> 每个批次收尾跑 `scripts/soko gate` + 全量 `scripts/vscode-e2e.sh` + `perf-compare`。


#### 批次 0 · 判据与量具（不动版本）

- [x] `T-001` 六条反馈落缺口台账
- [x] `T-002` 复现脚本：声明栏（LSP 探针）
- [x] `T-003` 复现脚本：记法跳转 + hover 原始类型（LSP 探针）
- [x] `T-004` 复现脚本：项目缓存对带 `sorry` 的单元不生效（CLI）
- [x] `T-005` 复现脚本：LSP 每次打开都重编（两个独立 LSP 进程）
- [x] `T-006` 复现脚本：goal 文本丢记法（CLI）
- [x] `T-008` 一条人工验收命令
- [x] `T-009` 实验：缓存键的 `build_stamp` 到底会不会让 CLI 预热对 LSP 失效
- [x] `T-011` 环节循环脚本 `scripts/dev-loop.sh`
- [x] `T-012` 真 VS Code **单用例**跑法（L4 层的前提）
- [x] `T-013` 版本纪律修订：bump = **发布边界**，不是 commit 边界
- [x] `T-014` 把"肉眼看效果"的回路写进文档
- [x] `T-015` 项目夹具：一个最小的 import 项目
- [x] `T-016` 测试可见的 Infoview 载荷（`testApi` 扩面）
- [x] `T-017` `scripts/vscode-e2e.sh` 加三个开关（L4 层的前提）
- [x] `T-018` 六条反馈的 e2e 用例（矩阵）
- [x] `T-019` e2e 进 CI 与台账
- [x] `T-007` 编辑器路径性能台账（把 §2.1 变成可重复的哨兵）
- [x] `T-020` 真实课程 + 编辑器路径的 perf 套件（七条 case）
- [x] `T-021` `scripts/perf-check.sh --case <name>`（单场景快跑，**每环节用**）
- [x] `T-022` `scripts/perf-compare.py`（**回归比较器**，本节最值钱的一条）
- [x] `T-023` 性能进 DoD 与 CI
- [x] `T-010` 文档与要求入账

#### 批次 1 · 线 B：声明栏 + nextHole（patch）

- [x] `T-B01` 实测表：哪些文件坏、哪些好
- [x] `T-B02` 实测：`nextHole` / `stateAt` / `hints` 在项目入口的现状
- [x] `T-B03` 收敛成单一判据 `QueryDoc::usable()`
- [x] `T-B04` 补 LSP 断言（夹具已经在了）
- [x] `T-B05` 复现转绿
  - ⬆ **BUMP**：`patch` —— 声明栏第一次真的能用（服务端修好，unit 文件不再空）
- [x] `T-B06` `alt+n` 跳洞在项目文件里可用
- [x] `T-B07` CLI 侧的假绿也钉住
- [x] `T-B08` 客户端 E4：请求期间切文档 ⇒ 新文档必须补取数
- [x] `T-B09` 客户端 E5：`_pendingDecls` 合并把新文档那次取数吃掉
- [x] `T-B10` 客户端 E6：空数组是真值 ⇒ `ensureDeclarations()` 永久空转
- [x] `T-B11` 客户端 E8：指纹吞掉"只改了类型"的更新
- [x] `T-B12` Webview 空态三态化
- [x] `T-B13` 真宿主 e2e：声明栏真的有卡片（矩阵用例 #1）
- [x] `T-B15` 真宿主 e2e：`alt+n` 跳洞（矩阵用例 #2）
- [x] `T-B16` 顺带修复 G-28：`→` 右边直接跟 `∀` 不解析
- [x] `T-B14` 文档同步
  - ⬆ **BUMP**：`patch` —— 客户端 E4/E5/E6/E8 修好 + 真宿主 e2e 证明面板有卡片（批次 1 收尾）

#### 批次 2 · 线 A：缓存 + 清浪费（minor）

- [x] `T-A20` 项目模式不再白编一遍入口单文件
- [x] `T-A21` `set_text` 文本未变即短路
- [x] `T-A22` 保存 / 编辑器外改动不再重编同一文本
  - ⬆ **BUMP**：`patch` —— 打开不再白编一遍、保存不再重编同一文本（立刻能感觉到的快）
- [x] `T-A24` `project_view_reason()` 不再每次 parse 整份文本
- [ ] `T-A25` `build <目录>` 的 O(文件数 × 闭包) 如实记账
- [ ] `T-A01` 项目缓存下沉到 `front`
- [ ] `T-A02` 缓存键去掉"可执行文件 mtime"这个不稳定的量
- [ ] `T-A03` 缓存条目 v2：按模块存（对齐设计 §4.8）
- [ ] `T-A04` 冷/热 `--json` 逐字节一致（带 `sorry` 的项目）
- [ ] `T-A05` `requires` 漂移不再静默关掉缓存 + 两条写缓存路径规则一致
- [ ] `T-A08` `requires` 的单一来源 + 漂移门禁（**用户判定的根因**）
- [ ] `T-A06` 依赖改动仍必 miss
- [ ] `T-A07` `--text` 中间态的处置
- [ ] `T-A10` LSP 读项目缓存（命中即回放）
- [ ] `T-A11` LSP 写项目缓存
- [ ] `T-A12` LSP 侧测试：两次独立进程打开，诊断逐字节一致
- [ ] `T-A13` 跨文件能力在缓存命中后仍然工作
- [ ] `T-A14` 实测数字
  - ⬆ **BUMP**：`minor` —— LSP 接入项目缓存：重启编辑器后重开同一文件不再等 1.8–8.5s
- [ ] `T-A15` 命中缓存后 Session 快照的处置
- [ ] `T-A23` 扇出：改一个依赖不重编所有打开文档
- [ ] `T-A30` 编译不再独占 `Mutex<Docs>`
- [ ] `T-A60` 缓存与扇出的 e2e 断言
- [ ] `T-A50` 设计文档 as-built
- [ ] `T-A51` 性能台账收口
  - ⬆ **BUMP**：`patch` —— 批次 2 收尾（性能台账与文档）
- [ ] `T-A52` （可选，用户拍板）打开工作区时预热缓存

#### 批次 3 · 线 C：goal 用记法（minor）

- [ ] `T-K32` `pretty_printer.rs` 零单测
- [ ] `T-C01` 四个生产者 × 真实文件的实测表
- [ ] `T-C02` 消费者审计（**不然会静默改坏判卷**）
- [ ] `T-C03` 设计文档：**把已有的 `printback-feasibility.md` §4 升格为权威设计**
- [ ] `T-C03b` `DisplayText` 护栏（**先于任何折叠代码**）
- [ ] `T-C04` 记法表复用 `judge.rs` 的重建（提成公共函数）
- [ ] `T-C10` 折叠函数第一刀：只做二元 infix 族
- [ ] `T-C11` arity 的来源
- [ ] `T-C12` `scoped` 的保真度
- [ ] `T-C13` 重载（一个符号 → N 个目标）的处置
- [ ] `T-C14` 折叠层的损失护栏
- [ ] `T-C20` 生产者 1+3：根状态与声明列表的 `ty_text`
- [ ] `T-C21` 生产者 2：无 `by` 的开练习（应当已经好，补守护）
- [ ] `T-C22` 生产者 4：`by` 步进里被 pp 化的四处
- [ ] `T-C23` binder ty（假设行的类型）
- [ ] `T-C24` 逐 surface 的判别性测试
  - ⬆ **BUMP**：`minor` —— goal / 假设 / 声明类型第一次显示记法
- [ ] `T-C25` 边界：命中不了就回退
- [ ] `T-C30` `semantic::tag_runs` 填 `Names::notations`
- [ ] `T-C31` 目标文本里的**导入名**不再标 `unknown_ident`
- [ ] `T-C32` 着色在 Infoview 里可见
- [ ] `T-C50` 真宿主 e2e：goal 文本用记法（矩阵用例 #6）
- [ ] `T-C40` 断言与 golden 更新（**计数中性**）
- [ ] `T-C41` 文档 + CHANGELOG
  - ⬆ **BUMP**：`patch` —— 批次 3 收尾（着色 + golden 重钉）

#### 批次 4 · 线 D：记法跳转 + hover（minor）

- [ ] `T-D01` 复现脚本
- [ ] `T-D02` hover 增加"原始类型"行
- [ ] `T-D03` hover 的"原始类型"只对**能解析出 target** 的符号显示
  - ⬆ **BUMP**：`patch` —— hover 显示记法的原始类型（全计划最便宜的一刀）
- [ ] `T-D30` 记法符号不再误解析到外层 binder（**独立正确性 bug**）
- [ ] `T-D31` `position_to_offset` 的 UTF-16 语义（**独立缺口**）
- [ ] `T-D10` `NotationDecl` re-export + 补 `span`/`module`
- [ ] `T-D11` 闭包级记法表进 `ProjectReport`/`QueryDoc`
- [ ] `T-D12` 解析 API：`notation_resolve(text, table, offset)`
- [ ] `T-D13` hover 的"展开成"（import 来的记法）
- [ ] `T-D14` parser 保留记法符号 token 的 span
- [ ] `T-D15` 新增 `ResolvedTarget` 变体并绕开覆写
- [ ] `T-D16` LSP `definition` 处理记法变体
- [ ] `T-D17` hover 的 `range` 收窄到符号本身
  - ⬆ **BUMP**：`minor` —— F12 在记法符号上能跳到声明
- [ ] `T-D20` 内建/ prelude 目标的定义跳转怎么办
- [ ] `T-D21` 跨文件记法的"定义"是哪个
- [ ] `T-D22` `scoped` 记法的导航是否尊重作用域
- [ ] `T-D23` 重载：一个 `Location` 还是 N 个
- [ ] `T-D24` `documentHighlight`/`references`/`rename` 覆盖记法符号
- [ ] `T-D40` 三层测试（矩阵用例 #7/#8）
- [ ] `T-D41` 文档同步
  - ⬆ **BUMP**：`patch` —— 批次 4 收尾（含 rename/highlight 不再误伤 binder）

#### 批次 5 · 线 K：内核提速（minor）

- [ ] `T-K01` 全语料逐字节对拍工具
- [ ] `T-K02` 内核改动的验收清单 + **修语料对拍的收集逻辑**
- [ ] `T-K03` `36.1s` 花在哪：分阶段 profile
- [ ] `T-K10` 设计文档 `docs/design/by-prefix-reuse.md` + 三个候选的定稿
- [ ] `T-K11` **K1-a：纯 front 的 judge TrustPlan 复用（零内核改动，先做）**
  - ⬆ **BUMP**：`minor` —— by 判定的内核检查那一段拿掉（K1-a）
- [ ] `T-K12` **K1-b：`EnvBuilder::with_env`（单次内核改动里性价比最高）**
  - ⬆ **BUMP**：`minor` —— 36.1s → 个位数秒（K1-b，本轮最大的一刀）
- [ ] `T-K13` **K1-c（备选）：`EnvBuilder::snapshot()` 克隆式检查点**
- [ ] `T-K20` 设计文档 `docs/design/closure-incremental.md` + spike
- [ ] `T-K30` `build <dir>` 不再逐文件各编一份闭包
- [ ] `T-K31` `TcCache::new` 每次 `with_ctx` 清 4MB
- [ ] `T-K40` 内核改动台账 + 文档
- [ ] `T-K41` `STATUS.md` 的"根因未修"话术更新
  - ⬆ **BUMP**：`patch` —— 批次 5 收尾（跨模块增量 + build 目录）

> `T-K21…`（K2-b / K2-a 的实现子步骤）**不是一条独立环节**：它要等 T-K20 的设计定稿后，按选中的候选方案拆成 3–6 个环节再补进本清单。
