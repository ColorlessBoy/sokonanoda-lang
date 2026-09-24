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

> **完成（2026-09-21）**：实现落在 `crates/front/src/project/cache.rs`，
> CLI 的 `project_cache.rs` 变成 6 行 thin re-export。
> **二进制对拍**：改动前后两个 CLI 对 49 组输入（10 个代表文件 × 5 个 op +
> stdin + `--root`/`--no-project`）stdout **逐字节相同、退出码相同**。
> （全量 180 文件 × 8 组 = 1440 次调用、每次都要编闭包 ⇒ 10 分钟跑不完，
> 取的是有代表性的样本：单文件 / 项目入口 / 项目依赖 / 坏依赖 / 解析失败 /
> stdin / `--root` / `--no-project`。）
> `cargo test --workspace --locked` 全绿（673 front · 152 lsp · CLI 各套件）。

- **改什么**：新建 `crates/front/src/project/cache.rs`，把
  `crates/cli/src/project_cache.rs` 内容搬过去；CLI 保留 thin re-export（公开 API 不变）。
- **判据**：`cargo test --workspace --locked` 全绿 + `scripts/soko gate` exit 0 +
  **二进制对拍**（改动前后两个 CLI 对全部 `*.sokonanoda` + `--root`/`--no-project` +
  stdin + `query check|goals|holes` 的 stdout 逐字节相同）。
- **风险**：纯搬迁，**不许夹带语义改动**。

#### T-A02 缓存键去掉"可执行文件 mtime"这个不稳定的量

> **完成（2026-09-21）**：`build_stamp()` 换成**编译期常量**
> （`cfg!(debug_assertions)` + `std::env::consts::{OS, ARCH}` +
> 可选 `SOKO_BUILD_LABEL`），`CACHE_FORMAT` **2 → 3**（旧条目作废）。
> `env!("PROFILE")`/`env!("TARGET")` **在库 crate 里取不到**（那是 build script
> 的环境变量，`env!` 直接编译失败）——实测踩到，换成了上面那组。
> G-27 复现 **exit 0 → exit 1**（同一份二进制、两个 mtime 的副本，第二次 `1 hit`），
> 已关账；`cargo test -p sokonanoda-front --lib cache::` 4 passed。

- **改什么**：按 T-009 的结论改 `build_stamp()`（例如换成编译期常量
  `CACHE_PKG_VERSION` + profile + 目标三元组，或干脆去掉），并 bump `CACHE_FORMAT`。
- **判据**：
  ```bash
  # 两个 mtime 不同秒的二进制之间：CLI 预热 → LSP 命中
  bash docs/gaps/repro/G27-cli-warm-lsp-hit.sh ; echo "exit=$?"
  ```
  （新缺口 G-27，修前判红。）
- **依赖**：T-009。
- **风险**：**这是线 A 的地基**——不修则"编辑器变快"可能完全落空。

#### T-A03 缓存条目 v2：按模块存（对齐设计 §4.8）

> **完成（2026-09-21）**：`CachedCompile` / `CacheFile` 多了
> `project: Option<ProjectReport>`（`#[serde(default)]` 让旧条目仍读得进来），
> `ProjectReport` 一族补上 `Serialize/Deserialize`；
> **`set_cached_entry` 现在把整份报告一起回放**。
>
> 顺手修掉一个**既有缺陷**（`crates/cli/tests/imports.rs` 的老注释记着它）：
> 项目缓存**热命中**时 `query project` 答 `project:null / reason:"no-path"`，
> 而冷跑答得好好的——同一个项目的答案不该取决于缓存热不热。新用例
> `query_project_answers_after_a_cache_hit` 钉住它，**回滚修法即红**
> （报错正是 `{"project":null,"reason":"no-path"}`，已验证）。
>
> 判据：`cargo test --workspace --locked` exit 0 · `cargo test -p sokonanoda-front
> --lib cache::` 6 passed（新增「整份往返逐字段相等」与「单文件条目形状不变」）·
> 二进制对拍 49 组 0 差异。

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

> **完成（2026-09-21）**。脚本名从计划里的 `check-manifests.py` 改成
> **`scripts/bump.py`**：**写和查必须共用一份实现**——分成两个脚本就会有两份
> "版本写在哪几个地方"的清单，那正是漂移的成因本身。用法
> `python3 scripts/bump.py <x.y.z>`（写）· `python3 scripts/bump.py --check`（查）。
>
> 它写四处：`Cargo.toml` · `editor/vscode/package.json` · `Cargo.lock` ·
> **仓库里每个声明了 `requires` 的 `sokonanoda.toml`**（`git ls-files` 得到，
> 所以 `.cache/` 里那些不会被碰）。`CHANGELOG.md` **不**由它写——那是人写的。
>
> 修掉两处实际漂移：`course/shared` **0.57**、`courses/set-theory` **0.61**
> （仓库 0.63.3）。
>
> **实测踩到的陷阱**：第一版正则 `name = "sokonanoda[^"]*"` 把 **kernel** 也改了
> ——内核在 `Cargo.lock` 里就叫 `sokonanoda`，而它的版本是**独立**的
> （`0.5.0`，跟它自己那条线），`cargo test --locked` 立刻拒绝。改成**精确名字表**
> （front/cli/lsp 三个）。
>
> 接进 `scripts/soko gate`（缺脚本 exit 3，绝不静默跳过）与 `ci.yml`。
> 判别性已验证：人为把 `requires` 改回 `0.61` ⇒ **exit 1** 并指名文件。

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

> **完成（2026-09-21）**，顺带发现并修掉一条**真问题**。
>
> ① 补断言 `a_dependency_edit_forces_the_entry_to_recompile`：看 `build` 的
> **hit/compiled 计数**（既有那条看 `#reduce` 的值——值对了但计数没动，说明
> "碰巧算对了"，不是缓存真的失效）。**判别性已验证**：把模块源从摘要里去掉，
> 入口立刻变成 `hit:1, compiled:1`，测试红。
>
> ② **顺带发现**：`ProjectPlan::digest` 里**没有入口路径**。报告里的
> `entry`/`root`/`manifest` 与每个模块的 `path` 都是绝对路径 ⇒ 两个
> **内容逐字相同但在不同目录**的项目共用一个键，第二个回放到的是第一个的路径
> ——`query project` 报错模块根、LSP 的 definition/references **跳到别的目录的
> 文件**。新用例 `two_identical_projects_in_different_directories_do_not_share_a_key`
> 钉住它，**回滚即红**（实测报错就是 `b` 拿到了 `a` 的路径）。
>
> 加的时候踩了一个坑：直接用原串会让 `grade Main.sokonanoda` 与 `build .`
> （收集到 `./Main.sokonanoda`）算出**两个键**——G-12 的纪律是"词法绝对化之后
> 不解析 `.`/`..`"，所以加了个 `digest_path()` **只去掉 `.` 组件**。
> `soko.project-iface/1` → `/2`。

- **改什么**：确认并补断言：摘要按拓扑序含每模块源文本（`ProjectPlan::digest`）。
- **判据**：`cargo test -p sokonanoda-cli --test imports -- --nocapture` 既有判别性
  测试绿（改依赖后 `#reduce` 必须给出新值）+ 新增"改 lib 一行 ⇒ 入口必重编"。

#### T-A07 `--text` 中间态的处置

- **改什么**：测量 `--text`（内存中间态）在编辑器路径上是否出现；若出现，
  决定"不缓存"是否仍成立，把结论写进设计文档。
- **判据**：结论 + 数字进 `docs/design/compile-cache.md`。

### 5.2 A-II LSP 接线

#### T-A10 LSP 读项目缓存（命中即回放）

> **完成（2026-09-21）——"打开变快"的开关，实测 126×**。
>
> `Doc::set_text` 对有 `import` 的文档走 `project_cache::plan` → `load`，
> 命中即 `set_cached_entry`（连 T-A03 的整份报告一起回放）。
>
> **`overlay` 必须参与摘要**：依赖的未落盘编辑会改变这份文档的闭包结果，
> 不折进键里就会**错命中**（回放出一份按旧依赖算的报告）。为此
> `project_cache::plan` 加了 `overlay` 参数（CLI 传 `&[]`）。
>
> **实测（真课程 unit08，4 个 import）**：冷开 **15.2s** → 热开 **48ms**
> （spawned debug LSP，含进程启动）。G-25 复现已关账。
>
> **两个教训（都是探针自己错，不是代码错）**：
> 1. **探针的前提错了**：第一版先用 CLI 预热、再开两次 ⇒ 两次都是热的，
>    比值恒 ~1×，**无论 LSP 读不读缓存**。改成"冷（全新缓存）→ 预热 → 热"。
> 2. **夹具要够大**：缓存省掉的是**内核检查**，而**算摘要本身仍要读 + 解析
>    整个闭包**。夹具太小时解析占满全部时间（10ms 冷热一样）。调到
>    24 + 12 条声明：冷 1517ms / 热 12ms = **126×**，整个复现 3.2s
>    （160 + 80 时判别力也够，但要 85s，门禁跑不起）。

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

> **✅ 2026-09-21 完成（as-built 见 `docs/design/lsp-edit-concurrency.md` §7）。**
>
> **判据**（`crates/lsp/tests/lsp_edit_concurrency.rs`，两条都进 CI）：
> ① 一次 ~1.2s 编译进行中的 `soko/stateAt` **1277ms → <1ms**；
> ② 打开 + 连打 5 个键的**编译趟数** 6 → **≤3**（编辑被合并）。
> 代价明写在 `docs/PERF.md`：重建慢的文件下一次编辑等 120ms 静默期。
>
> 下面那段是**修之前**的记录，留着当"为什么这么做"的现场：
>
> 缺口成立且很严重：冷编译 unit12（8.9s）期间，第一个 `soko/stateAt`
> 等了 **8907ms**（另一次 unit12-solution 是 **24393ms**）——整个 LSP 冻结。
>
> **试过"把编译挪出锁"：不够**。LSP 的**消息循环是串行的**（`did_change` 的
> handler `await` 着 `refresh`），后面的请求等的是 handler，不是锁。
> 真正的修法是**把编译 spawn 出去**。
>
> 试的过程中撞到两件必须先解决的事（都已回退，树是绿的）：
> ① 短路（T-A21）与"依赖在磁盘上变了"纠缠：锁外编译必须**带着当前状态**，
> 而 `Session` 不可 `Clone` ⇒ 要么给它 `Clone`，要么把短路判据换成**摘要**
> （摘要含依赖磁盘内容 + 覆盖，这才是正确判据）；
> ② 并发正确性：spawn 后两次编辑会并发编译，写回要靠版本校验，而第一版用
> "文本相等"做判据时**首次打开**会被丢弃（18 个测试红）。
>
> **安排**：与 T-K01/K02 的护栏一起做（先有语料对拍再动并发），或与 K1 一起做
> （编译变便宜，冻结的绝对时长也跟着降）。数字见 `docs/PERF.md`。

- **根因**：A9。
- **改什么**：把编译移出锁（先在锁内取快照 → 锁外编译 → 再取锁写回，带版本校验），
  或至少让只读请求（`stateAt`/`project`/`hover`）用 `RwLock` 读侧并发。
- **判据**：新增 LSP 测试「一次长编译进行中，`soko/stateAt` 仍能在 <100ms 内应答」。
- **风险**：并发正确性；必须先有 T-A21/T-A10 的缓存兜底，否则收益有限。
- **依赖**：T-A10。

### 5.5 A-V 真宿主 e2e（矩阵用例 #3/#4/#5）

#### T-A60 缓存与扇出的 e2e 断言

> **✅ 完成（2026-09-21，0.64.1）。** 三条都进了真宿主套件（`21 passing / 3 failing`，
> 失败的三条正是批次 3/4 的记法用例 #6/#7/#8）。实测数字进了 `docs/e2e/logs/`：
> `PERF e2e cache: cold=436ms warm=58ms`（**7.5×**）、
> `PERF e2e fanout: entry diagnostics publishes=1`。
>
> **写这三条踩到的四个坑**（都记在用例的注释里，因为它们会**假绿**）：
> ① `vscode.languages.getDiagnostics(uri)` **不是"刚发来"的信号**——VS Code 按
> URI 留着上一次的结果，也不会因为 `didClose` 清掉 ⇒ "非空"会让打开立刻满足条件、
> 量到 0ms，而那次**根本没编译**；要监听 `onDidChangeDiagnostics`。
> ② **监听器要早于那次发布挂上**：`restartServer` 会把打开中的文档重新同步一遍，
> 发布就发生在重启过程里。
> ③ **`workbench.action.closeAllEditors` 不等于 `didClose`**：`openTextDocument`
> 返回的 `TextDocument` 只要还被引用，客户端就不发 `didClose`，服务端那份 `Doc`
> 还活着 ⇒ 重开"什么都没发生"。所以冷开用一份**从没编译过的文件**（`units/u02`），
> 不去猜任何一方的状态机。
> ④ **时间断言要让被测那一段占主导**：夹具只有 3 条声明时编译只占 ~30ms，冷/热都
> 被"重启 + 往返"的固定开销（~60ms）淹没（实测 89ms vs 63ms，比例断言成了噪声）
> ⇒ 冷开的夹具**故意做大**（+120 条用库记法的定理）。
>
> **保存那条为什么不是 `workbench.action.files.save`**：VS Code 对**干净缓冲区**
> 的保存是 no-op（不发 `didSave`），从扩展宿主里测不到服务端那条短路。用例走
> **同一条服务端路径的另一半**（磁盘上重写成**同样的字节** ⇒
> `did_change_watched_files`），短路判据完全相同；真 `didSave` 那条由进程内用例
> `perf_course_save_same_text_is_recorded` 钉着。

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

### 5.5b 线 L：**解答改项风格**（2026-09-21 用户拍板；零内核风险，与大计划并行）

> 用户原话：「tactic 先不动是我写的，但是现在看性能差太多了，solution 没必要
> 损失性能。solutions 保持项的方式，做提示。用户在做题目的时候，llm 应该能根据
> 提示给出 tactic 的方式。」

**为什么单独成线**：它的收益 **8–25×**（`unit12-solution` 冷跑 120s 里 68% 是
`by` 判定），而风险是**零**——只改课程内容，不碰编译器和内核。它比线 K 的
K1（68% 的上界、要动内核）更划算，所以**并行做、优先级不低于 K1**。

| 环节 | 内容 | 判据 |
|---|---|---|
| **T-L01** | `unit01-solution` 试点转项风格 | **完成**：5.37s → 3.34s，6 条 checked、0 诊断 |
| **T-L02** | 规则入 `courses/set-theory/AGENTS.md` + 教师技能 §5 的翻译对照表 | **完成** |
| **T-L03** | 记 G-30（期望类型不传播到嵌套实参）+ 复现 | **完成** |
| **T-L04** | 剩余 12 份 `solutions/` 全部转项风格（逐份判绿） | 每份 `grade` exit 0、`decl.checked` 不变、`:= by` 归零 |
| **T-L05** | 门禁加一条：`solutions/*.sokonanoda` 里 `:= by` 出现次数必须为 0 | 进了 `courses/set-theory/tools/check.py` 或 `notation-lint.py` |
| **T-L06** | 全课程门禁 + 性能台账：转换前/后的 `course` 计时 | 36 目标 · 328 checked · 99 open · 0 判负**逐项不变** |

**与画布的关系**：`units/*.sokonanoda`（学习者看的画布）里的 tactic 块**不动**
——学习者练的就是 tactic。只有 `solutions/` 改。

### ⛔ 5.5a **红线级**：`by` 的判定方案违背热编译的设计初衷（G-31，2026-09-21）

> 用户原话：
> > 我还是很疑惑，lean 的 by 风格有这么耗时吗？是不是我们的 by 的实现方案有问题呢？
>
> > 那要插入方案进计划里，这个违背我们的红线，也违背我们的热编译的设计初衷。

**质疑成立，而且是我们自己的架构问题**（不是 `by` 语义、也不是内核）：

`crates/front/src/judge.rs::judge_pairs_uncached` 的最后一步是

```rust
let report = check_document_with(
    &FolFile { commands, src: full_prefix },   // 整份前缀 + 合成的判定声明
    options,
);
```

而 `check_document_with` → `run(&[SourceUnit::single("", file)], …)` **从头跑完整
流水线**，零复用。实测（`unit12-solution`，25 个 `by` 块）：

| | |
|---|---|
| `check_document_with` 调用次数 | **25**（每个 `by` 块一次） |
| 前缀字节 | 13,286 → 83,187 **单调增长** |
| 合计处理 | **1,064,669 字节 ≈ 文件本身的 13 倍** |
| 单次最贵 | **28.6 秒** |
| 判定占整个文件墙钟 | **68%**（81.9s / 120.4s） |

**Lean 不是这样**：`by` 只是一个普通命令，按顺序 elaborate **一次**，tactic 逐步改
goal state，**前面的声明不会被重新 elaborate**（`Lean.Server.FileWorker`："elaboration
is executed in a chain of tasks, where each task corresponds to the elaboration of
one command"）。⇒ 25 个 `by` 块的文件在 Lean 里与 25 个项风格证明**同量级**。

**为什么当初这么写**：前端没有"往已有环境里再 elaborate 一条声明"的入口，唯一入口
`check_document_with(FolFile)` 只能整份重跑。而"复用已判定的环境"在
`docs/design/by-tactics.md` §13 里被明确记成**死路**——`EnvBuilder` 字段私有、
`new(arena, config)` 是唯一构造入口、`finish(self)` 消费自身，且
`NamePtr::decl_idx` 绑定在造它的 builder 上。

⚠ **但那是内核冻结时写的**。2026-09-21 内核解冻（硬规则 1 修订）+ 用户明确授权
"如果是内核的性能问题，内核也可以列计划修改" ⇒ **那条死路现在是通的**。

#### 还有一层：重跑前缀会**连带重跑前缀里每个 `by` 块**

采样（`unit12-solution`）里 `run_by` 的 inclusive 占比 **47%**、`elab_expr` **28%**——
也就是说 `check_document_with(prefix + judges)` 不只是"重新 elaborate 一遍声明"，
它还会**把前缀里每一个 `by` 块重新跑一遍 tactic 引擎**（每个 `by` 触发的子判定
再由 `flush_batch` 汇总）。

⇒ 代价是 **O(`by` 块数²)**，不是 O(`by` 块数)。所以：

* **只做内核侧的 `with_env` 不够**——它省掉的是"重新内核检查"，但前端的
  **重新 elaborate + 重跑 tactic 引擎**还在；
* 真正对得上 Lean 的做法是**一趟走完**：elaborate 到 `by` 块时，**在当下的环境里**
  跑 tactic 引擎拿证明项，继续往下——而不是"先收对、再合成一份文档重判"。
  这正是计划里 **T-K20（`closure-incremental.md`，命令层检查点）** 要设计的东西，
  现在它从"锦上添花"变成**红线级任务的设计前置**。

#### 计划（**优先于线 K 的其余部分**）

| 环节 | 内容 | 判据 |
|---|---|---|
| **T-K12′** | `EnvBuilder::with_env`：让判定在**已 elaborate 过的环境**上继续，而不是重跑前缀 | `JUDGE_STATS` 的 `total_ms` 降到与"主 pass 一次"同量级；`unit12-solution` 冷跑从 120s 降到 **10s 量级** |
| **T-K13′** | 若 `with_env` 做不成：`EnvBuilder::snapshot()` 克隆式检查点（陷阱：`conv.rs:169` 按**指针**比较 `NatLit`，快照必须与新建声明同一 arena） | 同上 |
> **⬆ 排期决定（2026-09-21，用户拍板）**：**先收完线 C 的 4 条**
> （T-C32 / T-C50 / T-C40 / T-C41 含 patch bump），**然后立刻插 T-K20′**
> ——不再按 §13 的原顺序等它排到第 128 项附近。
>
> **依据**：实测 unit12 的**冷编译 9.8 s（release）**，正是用户最初那条
> 「编译很慢」；已查明其中一部分是 **G-34/G-31**（judge 每批合成文档 + 整前缀
> 重跑，实测 126k 次调用、363 次 miss）。用户此前已授权
> 「内核性能问题计划优先级可以往前挪」。**收完线 C 再插**是为了不让线 C 的
> bump 与验收被打断。

| **T-K20′** | **设计**：一趟走完（elaborate 到 `by` 就在当下环境跑引擎）vs 收对重判——给出取舍与工作量。**2026-09-21 补充**：设施必须**同时**覆盖 `judge_pairs`（`by` 判定，G-31）与 `judge_infer`/`judge_type_of`（记法消解与冗余 `sorry` 探针，G-34）——两处是同一个病（合成文档 + 整前缀重跑），只是入口不同 | 设计文档进 `docs/design/`，含实测的分阶段预算；两处的"重编前缀"计数都落到 0 |
| **T-K01/K02** | 护栏（`kernel-diff.sh` 已就位；还差 `kernel-check.sh` 与 `arena.rs` 收集逻辑） | 对拍零差异 |
| **回滚线 L** | K1 落地且 `by` 变便宜之后，**重新评估** `solutions/` 要不要用回 tactic | 届时按性能数字决定 |

**在那之前**：线 L（解答改项风格）仍要做——它是**零内核风险**的 8–25×，
而且不依赖 K1 的成败。但它是**绕开**，不是修复；`by` 贵这件事必须在计划里
按红线级跟踪，不能因为"解答改项风格了"就算解决。

### 5.6 A-VI 线 K：内核解冻后的性能根治（**批次 5 → 提前**）

> ### ⚠ 2026-09-21 用户改优先级：**速度是生命线**
>
> 用户原话：「我每次修改一下文件，就要编译 6807ms 吗？那我要你 build 有什么用？
> ……速度和性能是生命线！」
>
> **缺口 G-29**（已入台账 + 复现）：项目缓存只让**打开**变快，
> **编辑一次仍然重编整条闭包**：
>
> | | unit08（4 个 import） |
> |---|---|
> | 冷开 | 4970ms |
> | 热开（缓存命中） | 8ms |
> | **改一行** | **3039ms** |
> | ├ 依赖 4 个模块 | 2.16s |
> | └ 入口自身 | 2.7s |
>
> **一个会改变选刀顺序的实测**：把 unit08 的 9 个 `by` 块全换成 `sorry`，
> 只从 4.96s 降到 4.60s（**7%**）⇒ **瓶颈是 elaboration（建环境、elaborate
> 签名与项），不是证明检查**。原计划把 T-K11（`by` 块判定的前缀复用）排第一，
> 那是针对**证明检查**的刀——对课程单元这种"签名多、证明少"的语料几乎无效。
>
> **新的顺序**（在批次 2 之后立刻执行）：
> 1. **T-K01 / T-K02** 护栏（语料逐字节对拍 + 内核改动验收清单）——动内核之前必须先有；
> 2. **T-K03** 分阶段 profile（把"elaboration 里具体是哪一段"量出来；
>    本节上面那条 7% 是它的第一块拼图）；
> 3. **T-K12**（`EnvBuilder::with_env`）/ **T-K13**（`snapshot`）——复用依赖已编译
>    好的环境，这是"编辑不再重编依赖"的唯一办法；
> 4. T-K11 顺延（它治的是 `by` 块密集的语料，例如 36.1s 那个合成用例）。
>
> 细节与数字见 `docs/design/compile-cache.md` §4（**已更正**：原来把这条写成
> "命中缓存后第一次按键慢 1.56×、差的是进程预热"，避重就轻）。



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

> **完成（2026-09-21）**：`scripts/kernel-diff.sh`。
>
> * 对拍 `grade --json` + `query check|goals|holes|project`，**stdout 逐字节 +
>   退出码**；全量模式另比**课程门禁计数**；
> * `SOKONANODA_NO_CACHE=1` **强制**关缓存——否则条目命中会跳过编译，
>   对拍就测不到内核了（而且两个二进制版本相同时会互相命中）；
> * `--fast`（3 文件 × 2 op）**1.9 秒**，每环节能用；全量含课程门禁约 6 分钟；
> * `--self-test` **2/2**：同一二进制零差异 + 人为注入一个字节必须被抓到。
>
> **踩到的三个坑**（都写进了注释）：
> ① `mapfile` 在 macOS 自带 bash 3.2 里**不存在**——它报错之后 `FILES` 是空的，
> 脚本还会"零差异"地**绿过去**（最坏的那种失败）；改成 `while read` 循环。
> ② `--fast` 第一版含课程门禁 ⇒ 6 分钟（门禁要 grade 36 目标 ×2 二进制），
> 把"每环节能用"拖成"不敢跑"；挪到全量模式。
> ③ self-test 的"poison 包装器"第一版用了 `exec` ⇒ 它替换掉 shell，后面那句
> `echo` 永远不执行 ⇒ 注入的差异根本没出现，self-test 报"没发现差异"。

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

> **完成（2026-09-21）**。`unit12-solution` 冷跑 **120.4s**，拆开：
> **`by` 块判定 81.9s（68%）** + 主 pass 38.5s（32%）。
> 判定是 25 次 `check_document_with` **重跑整份前缀**，平均 3.3s/次。
>
> **两把刀的收益上界**：T-K11（前缀复用）**68%**；只省内核检查的刀 **≤27%**
> （采样里没有独立的内核帧，被内联进前端）。
>
> **还证实了一件事：profile 随文件形状变化极大** —— `unit08` 的 `by` 判定只占
> **7%**（把 9 个 `by` 全换成 `sorry` 只省 4.96→4.60s），而 `unit12-solution`
> 占 **68%**。⇒ **不能只按一个文件选刀**；课程里 `solutions/` 那些 tactic 风格
> 的解答才是最坏样本。
>
> 加了一个**常驻**开关 `SOKO_JUDGE_STATS=1`（`crates/front/src/judge.rs` 的
> `stats` 模块），把"一次性探针"变成随时可重量的口径。数字见 `docs/PERF.md`。

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

> **✅ 完成（2026-09-24）**：文档已写 —— `docs/design/by-prefix-reuse.md`。
> 四部分齐全：① 现状调用链（`session.rs:265/276` → `run_incremental`
> (`check/mod.rs:500`) → `walk.rs` → `kernel_phase`）；② **两条纠正**；
> ③ 三个候选的三栏对照（侵入面/解锁收益/正确性风险）+ **为什么 K1-b 优于 K1-c**；
> ④ 每个候选的风险与验收（共同红线 = `scripts/kernel-check.sh` 五步）。
>
> **写文档时实测核对过引用**（调研稿里有几处行号漂了 ✗，例如
> `judge_pairs_uncached` 实际在 `judge.rs:399` 而不是 418）：
> * `walk.rs:137` 的 `trusted` ✓、`:330 lower_value` ✓、`:378/:388 build_def` ✓
>   ⇒ **纠正 ① 属实**：TrustPlan 只省"内核重查"，elaborate 与 `Declar` 构造照跑 ✓；
> * `builder.rs:244` 往 **interned NameNode** 上写 `decl_idx` ✓、`env.rs:273/:297`
>   读且**不校验名字** ✓ ⇒ **纠正 ② 属实**；
> * `tc.rs:210 check_all_declars_par` + `:223 thread::scope` ✓ ⇒ K1-c 那条
>   `ExportFile: Sync` vs `ArenaRef` 非 `Send`/`Sync` 的论证成立 ✓。

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

> **⚠ 实测结论（2026-09-24）：机制已落地且正确，但在这批语料上"不触发" ⇒ 不勾。**
>
> 实现（`435907e`）：线程局部"已核前缀"上下文 + judge 三处合成文档入口统一走
> `check_synthesized`（条件满足走 `run_incremental`，否则回退 `check_document_with`）
> + 开关 `SOKO_JUDGE_ENV_REUSE=0/1` + `REUSED` 计数。
>
> **正确性判据：过 ✓** —— 开关两态在 **27 个课程/示例文件**上 `--json`
> **逐字节相同、0 差异**（数字进了 `docs/perf/ledger.jsonl`）。
>
> **性能判据：没过 ✗（如实记）**：
> * `by_block_did_open`（冷开 `unit12-solution`）：off **12448ms** / on **12371ms**；
> * `keystroke`（`unit08`）：off 614/611ms / on 589/606ms —— 全在噪声内；
> * `SOKO_JUDGE_REUSE_STATS=1` 显示命中数 = **0** ✗ ⇒ **不是收益小，是根本没触发**。
>
> **根因（有证据）**：keystroke 只改**后段**命令 ⇒ 前缀未变 ⇒ **judge 自己的缓存
> 全命中**（贵的那 89 次 miss 不在增量路径上 ✗），而冷开路径的前缀**确实没有人
> 担保** ⇒ 守门条件（外层必须担保 `before` 覆盖全部前缀）**正确地**拒绝复用 ✓。
> ⇒ 把"前缀不重查"寄托在 `TrustPlan` 上，对**冷开**（= 课程最重的场景）无效。
> **真正能省的是另一件事**：judge 合成文档的前缀检查与**外层 pass 的检查共享**
> （一次检查两处用），那是 T-K12/K13 的范畴。
>
> 代码**保留**（正确、零风险、且是上面那件事的挂点），但**不勾这一条**：
> 它的判据（"耗时下降的实测数字"）**没有成立**。

> **🚧 分析（2026-09-24）：两处实测判断。**两条实测判断（下一轮直接照这个做）：
>
> 1. **只改 cache miss 那条路**：`judge_infer`/`judge_type_of`/`judge_pairs_with`
>    的缓存键**含整段前缀**（`judge.rs:839` 一带），而 §13 探针的统计是
>    `hits=20764 misses=89 judge_time=83.5s` ⇒ **83.5s 几乎全在 89 次 miss 上**
>    （每次 ~938ms 重查整份前缀）。⇒ K1-a 的收益 = 把 **miss 那次** 的
>    "前缀内核检查"拿掉，而不是去动 20764 次命中。
> 2. **别逐点穿参**：`judge_*` 的调用点全仓 **111 处**（`elab.rs`/`by.rs` 为主），
>    逐点加 `prefix_failures` 参数会把改动摊到 111 个地方 ✗。仓库里已有先例
>    —— `PreludeInstallGuard`（`compile/elab.rs:181` 一带）用**线程局部**标记
>    "正在安装 prelude"。K1-a 应当照同一条路：一个**线程局部的"当前前缀失败表"**
>    上下文，由外层 pass 设置、judge 的 miss 路径读取；**没设置就回退到
>    `check_document_with`**（= guard (i)：只有外层能担保前缀时才复用）。
> 3. 三处守死（原文照旧）：trusted 前缀不产 `DeclState`/事件 ⇒ `judgement_of`
>    只读 `_soko_judge_k`，**必须加断言**；`skip` 语义与 pass 2 的 check-then-add
>    **逐字一致**；开关 `SOKO_JUDGE_ENV_REUSE=0/1` 两态下全语料 `--json` 逐字节相同。

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

> **🚧 Stage 1 完成（2026-09-24，commit `694d75f`）**：内核侧 `EnvBuilder::with_env`
> 已落地 —— 把 `dag/declars/notations/mutual_block_sizes` **借**给一个临时
> `ExportFile`（`name_cache` 由 `dag.mk_name_cache(anon)` 现造、借完丢弃），
> 回调结束后**原样装回** ⇒ intern 表指针恒等式不变、合成声明**从不**进环境。
> 判据：`crates/kernel/tests/memory_api.rs::with_env_lends_the_intern_tables_and_takes_them_back`
> （① 借出时看到的是同一份表 ✓ ② `declaration_count` 不变 ✓ ③ 装回后还能继续
> `add_declar` + `finish()` ✓）。内核全套 **60 条通过**；五步 ①②③ 已过。
>
> **⚠ Stage 2 的真实体量（实测后修正，比原文写的更大）**：原文说"把
> `&mut EnvBuilder` 从 walk 穿到 `lower_value`→`run_by`→judge" —— 但
> **walk 现在根本不持有 builder** ✗：walk 只产出 `PendingOp`，`ExportFile` 是在
> **之后**的 `kernel_phase` 才建的（`compile/check/kernel_phase.rs`）。
> ⇒ Stage 2 不是"加个参数"，而是**把环境构建搬进 walk**（或让 walk 与
> kernel_phase 共享一个 builder）—— 这是**编译流水线的结构性调整**，
> 必须单独一轮做，且要盯死两件事：
>   * `PendingOp` 的 check-then-add 语义（部分应用/遮蔽/prelude 决策）不能变；
>   * `Judgement` 三态与**文案**逐字段不变（`Error.code` 来自 `refine_kernel_kind`
>     这个 front 分类器 ⇒ 新路径**必须复用同一个分类器**）。
>
> **📊 定刀取证（2026-09-24，冷开 `unit12-solution` ≈ 12.4s）**：
> `JUDGE_INFER calls=51156 total=**11183ms（90%）**`，其中 `misses=247 ≈ **9.6s = 77%**`
> （每次 miss 重查整份前缀）；`hits=50909` 的 `key_ms=787`（每次哈希 170KB 前缀）+ 
> `hit_ms=793`；`JUDGE_STATS`（pairs 路径）只有 `calls=11 total=1203ms`。
> ⇒ **刀落在 `judge_infer` 的 miss 上**（不是 pairs 路径）；K1-a 覆盖不到它
> （冷开 `before=0`，复用条件正确地不成立）。数字进 `docs/perf/ledger.jsonl`。
>
> **Stage 2 的实现设计（避开"重构 kernel_phase"这个更大的动作）**：
> 让 **walk 自己持有一个"影子环境"**（一个 `EnvBuilder`）——walk 每 elaborate 出
> 一个声明就 `add_declar` 进影子环境（`Declar` 本来就在手上，**零额外 elaborate**），
> 于是**任意时刻影子环境 == 当前前缀**。judge 的 miss 路径拿到这个 builder 后
> 走 K1-b 形态：`build_def(&mut builder, …)` 建合成声明 →
> `builder.with_env(|env| env.try_check_declar_at(&d, EnvLimit::ByIndex(k)))`。
> **`kernel_phase` 一个字不用改**（它照旧按 `PendingOp` 建自己的环境 ✗ 影子环境
> 只是给 judge 用的 ⇒ 语义零风险，代价是一份额外的环境内存）。
>
> 判据复用**已有**的两条（不必新写骨架）：
> * `crates/front/src/judge.rs:1626` 那条"批处理与逐条判定必须逐字相同"❌→ 已有 ✓；
> * 全语料 `--json` 两态逐字节相同（照 K1-a 的 27 文件做法）。
>
> 原建议"先写对拍骨架"**不必**：批处理/逐条等价测试已经在了。
>
> **🎯 为什么这一刀值（决定性推论，2026-09-24）**：T-K03 的旧 profile 写死了两条
> ——「**parse 只占 0.2%**」「**只省内核检查的刀 ≤27%**」⇒ miss 那次 38ms 里
> **大头是 elaborate** ✗。而 **K1-b 恰好三样全省**（环境里已经有那些声明 ⇒
> 不重 parse、不重 elab、不重 check）⇒ 它吃掉的是**几乎全部**的
> **9.5s / 12.4s**（≈ **77%**），落点 12.4s → **~3s** ✓。
> 对照：**K1-a（T-K11）只省 check 那一段**，且冷开下复用条件不成立 ⇒
> 实测**零收益** ✗（命中 0）——两者不矛盾，是"能不能碰到 elab"的区别 ✓。
>
> **⛔ 一条会破红线的约束（2026-09-24 挖到，必须先遵守）**：
> `kernel_phase.rs:60` 是 `let mut env = builder.finish();` —— kernel 阶段**消费**
> `run_pass` 里那个 builder，而且全程严格 **check-then-add**（先 `try_check_declar`
> 再 `add_declar`）。而 walk 发生在**它之前** ✗。
> ⇒ **"walk 边 elaborate 边 `add_declar` 进影子环境、judge 直接用它"会错** ✗：
> 影子环境里会出现**尚未通过内核检查**的声明 ✗ ⇒ judge 拿它当"已核前缀"用，
> 会给出与真实环境**不同**的判定（例如前缀本该被内核拒绝 ⇒ 旧路径报 Error，
> 新路径却判 Match）——**这正是 REQUIREMENTS §2 第 1 条的红线** ✗。
>
> **修正后的设计（可行且语义安全）**：影子环境必须**边生长边被内核验证**：
>   1. walk 每 elaborate 出一个声明 ⇒ **立刻** `try_check_declar_at(&d, ByIndex(k))`
>      （增量、O(1) 每次 ✓）⇒ 通过才 `add_declar`，失败就记进**失败表**；
>   2. 于是任意时刻影子环境 == **已被内核验证过的前缀** ✓（与 kernel 阶段的语义
>      逐条一致：同顺序、同 `EnvLimit::ByIndex`、同失败表）；
>   3. judge 的 miss 路径再用 `builder.with_env(...)` 查合成声明 ✓；
>   4. **`kernel_phase` 一个字不改** ✓ —— 它照旧 check-then-add 一遍。代价是
>      内核检查做**两遍**，但两遍都是**增量 O(n)**，而不是现在那 247 次
>      **整份前缀重查** ✗ ⇒ 这正是赚的那 9.5s ✓。
>
> **判据要点**：两遍检查必须给出**同一份**失败表（否则 judge 的可见前缀会与
> 真实环境分叉）⇒ 对拍时除了 `--json` 逐字节，还要比"前缀失败表"。
>
> **另一个实现选项（更省一遍检查，但动 kernel_phase）**：让 walk 的增量检查
> **成为** kernel 阶段的那一遍（kernel 阶段不再重查已核的）——省掉重复，但要改
> `finish_pass` 的 PendingOp 消费顺序 ⇒ **先做上面那个两遍版**（零语义风险），
> 拿到数字后再评估要不要合。
>
> **✅ 更正一条我先前写错的判断（2026-09-24，动手时实测）**：
> 本文件前面写过"**walk 根本不持有 builder**" ✗ —— **错了**。
> `walk.rs:36-40`：`pub(super) struct Walk<'arena> { …, builder: EnvBuilder<'arena>, … }`，
> 而 arm 里用的就是 `&mut self.builder`（`walk.rs:388`）；它最后经
> `kernel_phase::finish_pass(kernel_phase::Walked { … })`（`mod.rs:772`）被
> `builder.finish()` **消费**（`kernel_phase.rs:60`）。
> 我当时的推理是"env 在之后的 kernel 阶段才建" ⇒ 误推 walk 没有 builder ✗。
> **更正后的影响**：配方**更简单**了 —— 影子环境可以直接作为 `Walk` 的**新字段**
> （同 `'arena` 寿命 ✓），不需要另外想办法把 builder 送进 judge 的深处；
> 只是它仍然需要**自己那份 prelude**（因为 `self.builder` 最终要被 `finish()` 消费、
> 且 walk 阶段**不往里 add** 文件声明 ✗）⇒ 共用的 prelude 助手照样要用 ✓。
>
> **✂ 拆成三个可独立验收的子环节（2026-09-24 拍板；原条目体量超出"小环节"）**
>
> 动手三轮后的事实：T-K12 不是"加个参数"，而是**一次真实的编译流水线重构** ✓。
> 为了保持"一小步一验收"的节奏，拆成：
>
> * **T-K12a（✅ 已完成）**：内核 `EnvBuilder::with_env`（`694d75f` ✓ + 判据 ✓）
>   + `install_all_preludes` 唯一实现（`d763e78` ✓）。
>   **验收**：内核 60 条测试 + 往返回归判据 ✓。
> * **T-K12b（🚧 骨架已落地，2026-09-24）**：`Walk` 新增 `shadow: EnvBuilder<'arena>`
>   / `shadow_upto` / `shadow_failed: Vec<usize>`（**按 `ops` 下标**记失败 ——
>   `PendingOp::Decl` 自带 `cmd: usize`，正是 kernel 失败表的键 ✓）；`run_pass`
>   建 `shadow_arena` + 用**同一个** `install_all_preludes` 装 prelude ✓；
>   `Walk::shadow_env()` 从 `ops` **惰性重放**（`Decl`/`InductiveBlock` 逐条
>   `with_env(|env| env.try_check_declar(&d))` ⇒ 过则 `add_declar`，不过记下标 ✓）。
>   `SOKO_SHADOW_CHECK=1` 打印规模供对照 ✓。
>   **首次观测（外层真编译）**：`unit01-sets-membership` → `decls=72 failed=3`；
>   `unit12-solution` → `decls=85 failed=27`；同一文件 pass1/pass2 两次一致 ✓。
>   ⚠ **注意 `--json` 会触发大量嵌套编译**（judge 合成的文档也会走 `run_pass` ✓，
>   那些只有 prelude 的 12 条 ✗）⇒ 观测要看**最大的那条** ✓。
>   **下一步（b 的验收）**：把这两个数与**内核阶段**的结果对照 —— `failed=27`
>   偏高 ✗，必须查清是"影子检查得更严"还是"镜像不准" ✗（**这正是 b 要回答的
>   问题**，答不上就不进 c ✓）。 —— `Walk` 新字段 `shadow: EnvBuilder<'arena>`
>   + 从 `self.ops` **惰性重放**（`Decl`/`InductiveBlock` 逐条
>   `try_check_declar`，**与 `kernel_phase` 的主路径逐条同款**：
>   主声明用 `ByName` 形式（`kernel_phase.rs:251`）、签名探针才用 `ByIndex`
>   （`:164`/`:548`）、归纳块的记账一并镜像 ✓）+ 影子失败表。
>   **验收（不碰 judge）**：单测"影子环境的声明数与失败表 == 内核阶段那一份" ✓。
> * **T-K12c（其后）**：**judge 接线** —— 合成声明要在影子环境里 elaborate，
>   所以要么把前端表（`known` 等）穿下去、要么把"合成"搬到 walk 里 ✗
>   （见下面 (b)）⇒ 这一步含真实重构 ✓；完成后才有性能数字与 **⬆ bump minor** ✓。
>
> **为什么这么拆**：b 的验收**完全独立**于 c ✓（影子对不对，不需要 judge 参与 ✓）；
> 而 c 是唯一有性能风险的一步 ✓。b 做完若发现影子无法与内核阶段逐条一致 ✗，
> 就在 b 就停下 —— **不会把风险带到 judge** ✓。
>
> **🧱 动手时又挖到两个结构事实（2026-09-24，都会改变工作量估计）**：
>
> **(a) 声明不用自己收集** ✓：`PendingOp::Decl { declar, .. }` /
> `PendingOp::InductiveBlock { declars, .. }`（`mod.rs:26-40`）⇒ walk **本来就按序**
> 把已 elaborate 的声明收在 `self.ops` 里 ⇒ 影子环境可以**直接从 `ops` 惰性构建**
> （不必在 6 个 arm 里各插一行 ✓）。
>
> **(b) 真正的障碍：judge 的合成声明要"用前端表 elaborate"** ✗。
> judge 现在把候选术语合成 `Command::Def` 文本，交给 `check_document_with` ⇒
> 里面会 **elaborate**（要 `&KnownTable` 等前端表）。而这些表在 **walk** 手里 ✗
> ⇒ 要"在影子环境里建合成声明"，judge 必须先拿到这些表 ⇒ **要么把表也穿下去，
> 要么把合成这一步搬到 walk 里** ✗。**规格估的 front 150–300 行偏乐观** ✓
> （实际要 400+ 且含一次真实重构 ✓）。
> ⇒ 落地顺序改为：**先**把影子环境建起来并**证明它与内核环境一致**
> （可用 `ops` 惰性构建 + 单测 ✓，不依赖 judge），**再**单独一轮解决 (b)。
>
> **🔧 实现配方（2026-09-24 定稿）** —— 影子环境必须**自带 prelude**
> （否则一条声明都检查不了 ✗），而 kernel 阶段的 builder 会被 `finish()` **消费** ✗
> ⇒ 需要**第二个 builder**，且两边 prelude **必须逐条同款**（条件分叉 = 判定义分叉 ✗）：
>
> 1. 把 `run_pass` 里那段 prelude 安装（`PreludeMode::Full` 分支：
>    `prelude_shape` 预扫描 + `install_prelude`/`install_bool_prelude`/
>    `install_eq_prelude`/`install_l1_prelude`）抽成**一个共用助手**
>    `install_all_preludes(builder, known, inductives, defs, units, options)`
>    —— 两处调用**同一个函数** ⇒ 条件不可能分叉 ✓；
> 2. `run_pass` 再建 **arena₂ + `shadow: EnvBuilder`**，用**一次性**的 front 表
>    （`known₂`/`inductives₂`/`defs₂`，用完即弃）调同一个助手 ✓，然后把它作为
>    **`Walk` 的新字段**（`shadow: EnvBuilder<'arena>`）传进去 ✓（同寿命 ⇒
>    不需要任何 unsafe / thread-local ✓）；
> 3. walk 拿到 `shadow: &mut EnvBuilder`：**每 elaborate 出一个声明**就
>    `shadow.try_check_declar_at(&d, EnvLimit::ByIndex(k))`（k = `shadow.declaration_count()`）
>    ⇒ `Ok` 才 `add_declar`，`Err` 记进**影子失败表**（与 kernel 阶段同语义 ✓）；
> 4. judge 侧：`run_by`/`by.rs`/`judge_infer` 加 `Option<&mut EnvBuilder>`（老签名
>    委派给 `_with_env` 变体，调用点零改动 ✓）；拿到 env 时走
>    `builder.with_env(|env| env.try_check_declar_at(&synth, ByIndex(k)))` ✓，
>    拿不到就**回退**旧路径 ✓（回退是默认 ✓）；
> 5. 判据：`SOKO_JUDGE_STATS`（`JUDGE_INFER` 的 miss 成本应塌下来 ✓）+ 全语料
>    两态 `--json` 逐字节 + **前缀失败表相同** + 五步 ✓ + 本条的 **⬆ bump minor** ✓。
>
> **为什么值得这么费事**：靶心是 247 次 miss ≈ **9.5s / 12.4s = 77%** ✓，
> 而 K1-b 三样全省（parse/elab/check ✓）。
>
> **实现上唯一的硬骨头**：judge 深在 walk 内部，而 `EnvBuilder` 借 `&'a ArenaRef`
> ⇒ "arena + builder"是自引用结构（`docs/architecture.md` §8 的 arena 生命周期
> gotcha）**不能存进 `Walk` 自己的字段** ✗。⇒ 必须由**外层作用域**（`run_pass`）
> 持有 arena+builder，再把它**送进** judge —— 两种走法：
> (a) 沿调用链穿参（`walk.rs` 的方法 + `lower_value` + `by.rs` + judge 三入口）；
> (b) 作用域内的 thread-local（生命周期由 guard 保证，但需要 `unsafe`）。
> **下一轮先做 (a) 的最小切片**：只打通 `by` 块那条（`run_by` → `judge_pairs_with`），
> 用 `SOKO_JUDGE_STATS` 验证 miss 成本塌下来，再推广到 `judge_infer`。

> ~~建议下一轮先写"新路径 vs 旧路径逐条 `Judgement` 全字段相等"的**对拍测试骨架**~~
> （它同时是 Stage 2 的判据），再动流水线。

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

##### T-K22 **K1-d：记法消解的类型查询走局部书写类型（零内核调用）**

**已完成（2026-09-21，缺口 G-34）。** 与 T-K11/T-K12 **同一个病、不同入口**：
`elab_notation` 解前导参数时要问内核"这个操作数的类型是什么"
（`infer_type_text` → `judge::judge_infer`），而 `judge_infer` 的缓存键
**含整段前缀** ⇒ 前缀每长一条声明就换一个键，未命中就把**整段前缀重编译一趟
pass**。定位它靠两个新的常驻诊断开关：`SOKO_PASS_TRACE=<n>`（第 n 趟
`run_pass` 的调用栈）与 `SOKO_INFER_TRACE=<n>|all`（每次未命中的查询 + 栈）。

- **改什么**：`crates/front/src/compile/elab.rs` 新增 `operand_type_expr`——
  **局部变量先取 `ElabScope::source_type_of`（书写类型，零内核调用）**，
  拿不到才退回 `infer_type_text`。判据与 `implicit.rs` 已有的那条路**完全相同**
  （见 `elab.rs` 里 `arg_tys` 的注释：书写类型不但零调用，还比内核 pp 更准——
  pp 会丢掉嵌套常量的隐式实参，`Eq.{1} Nat 1 1` pp 成 `Eq 1 1`）。
  三个入口同时换：`solve_prefix_args`（主）、`guarded_binder_type`、`arg_tys`。
- **实测（release，`unit12-solution`，冷缓存）**：

  | 指标 | 改前 | 改后 |
  |---|---|---|
  | `judge_infer` 调用 | 126,105 | **51,156** |
  | 未命中（= 整段前缀重编一趟） | 363 | **247** |
  | `judge_infer` 累计 | 12.3s | **6.9s** |
  | **墙钟** | **11.4–12.0s** | **7.5–8.4s** |

  命中侧本来就不贵（50,909 次命中 744ms、键构造 764ms）⇒ 优化必须打**未命中**
  （即"别问内核"），不是打哈希。
- **判据**：`scripts/kernel-diff.sh`（全语料逐字节对拍，**零差异**）+
  `cargo test --workspace` + 课程门禁计数不变 + 缺口复现
  `docs/gaps/repro/G34-notation-type-query-recompiles-prefix.sh`。
- **剩下的（不许当成已根治）**：247 次未命中来自**别的入口**——冗余 `sorry`
  探针的 `fun (__soko_render : T) => __soko_render`、`And.intro` 这类**裸常量
  头**、inductive 安装、闭包里的记法。它们**没有局部类型可拿** ⇒ 只能靠
  T-K20′ 的「就地拿当前 pass 的环境」。本刀是缓解，不是根治。

#### 5.6.2 第二刀：闭包的跨模块增量 / 入口间共享

> **注意**：本刀的"零内核改动版"就是**线 A（T-A10…T-A23）**——
> 调研确认它是低风险纯接线（`crates/cli/tests/project_features.rs:164/194/201`
> 已有冷/热逐字节对拍），**但它只解决"第二次打开"，不解决"首次打开新文件"**。
> 首次打开必须靠下面这两条。

##### T-K20 设计文档 `docs/design/closure-incremental.md` + spike

> **✅ 完成（2026-09-21）。** 设计文档 + spike 脚本 + **计划要求的脚本化守卫**。
>
> **spike 实测**（`scripts/spike-closure-incremental.py`，只量不改；release、
> 每个入口一个全新缓存目录）：
>
> | 入口 | 闭包模块 | 冷编译 | 只编新增 | 合法前缀 |
> |---|---|---|---|---|
> | `unit01-sets-membership` | 3 | 1848ms | 1848ms | ✓ |
> | `unit08-images-preimages` | 5 | 4125ms | **2277ms** | ✓ |
> | `unit12-synthesis` | 8 | 8886ms | **4761ms** | ✓ |
>
> **依次打开三个入口：14859ms → 8886ms（−40.2%）**，与计划记的目标
> （15.06s → ≈8.5s）一致 ✓。三个前缀全部 **downward-closed** ✓——这是 K2-b
> 的硬条件（把已编集合当前缀用，它必须满足"每个已编模块的依赖也在里面"）。
>
> **守卫落地了（不只是写在文档里）**：`compile::prelude_shape(units)` 返回
> `{explicit_nat, explicit_bool, shadowed}`，**`run_pass` 的安装判据现在就走它**
> ——安装与守卫是**同一个函数**，不会漂。`shadowed` 用"与 `PRELUDE_NAMES` 的
> 撞车集"而不是整个 `taken`（后者会把"用户名字不同"误判成形状不同）。
> **脚本化验证**：`crates/front/tests/prelude_shape.rs`——课程里**每一个**
> `.sokonanoda` 的闭包形状必须一模一样且不撞 prelude 名字（全绿 ✓）。
> 计划里说"调研说是、但不是代码保证"，这条测试把它变成了代码保证。
>
> 文档内容：§1 现状（一个 Arena + 一个 builder 跑整个闭包，含 `file:line`）·
> §2 两个障碍（**O7** prelude 按闭包决定 ⇒ 会静默改变判卷；**O8** 七样每轮状态
> 必须一起提升，含 `ns` 的单元边界 `reset` 语义）· §2.3 报告归因（`split_report`
> 走命令下标、不用 span ⇒ 已编模块的报告也要留住）· §3 候选（K2-b 窄版先落地、
> K2-a 结构正解）· §4 spike 实测与**读法**（上界估计，别过度解读）·
> §5 实现切片 · §6 风险（高）。

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

> **✅ 完成（2026-09-21）。** `crates/kernel/tests/pretty_printer.rs`（**5 条**）钉住
> `pp_expr` 的文本输出：`->` / `forall` / `{}` 隐式 / binder 未使用就折成箭头 /
> 匿名 Pi 套具名 Pi 要括号 / `Prop` 与 `Type 0` / 应用左结合与参数括号 /
> 匿名 binder 的空转义 `«»` / 层级实参默认不打印。
>
> **它们是特征化测试**（钉现状，不是钉"正确"）：**变异检查**做过——把
> `f (g x)` 的期望改成 `f g x` 必须红（实测红了），证明它抓得住变化。
>
> 两个建夹具的坑（写进了文件注释）：`Config::default()` 的 `proofs = false` 会让
> pp 对**开项**跑 `is_proof` 推断 ⇒ `infer: loose bvar` panic（要按前端
> `finish_pass` 那样设 `pp_options.proofs = true`）；表达式里用到的常量**必须真的
> 声明**，否则 `const_head_type: unknown const` panic。

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

> **✅ 完成（2026-09-21）**：`docs/design/notation-aware-printing.md` §1。
> 逐格实测（命令模板也写在文档里，每格可重跑），三条结论：
> ① **光标在不在 tactic 上**决定走哪一支——学习者的光标就在 tactic 上，所以他看到的
> 就是内核 pp 的点名形式（**这就是用户的抱怨**）；
> ② "`by` 步进保留记法"**只对结构型 tactic 成立**：`apply Set.ext` 之后是
> `(x : α) -> Iff (A x) (B x)`——`∈` 与 `↔` 一起消失（子目标来自被应用引理的
> **内核 pp 望远镜**）；
> ③ **同一份声明在两个 surface 上文本不同**（`mem_of_subset` 的 `goal` 是
> `(a ∈ A) -> a ∈ B`，`sorry` 行是 `forall (α : Type 0) …, Set.mem α a A -> …`）
> ⇒ **要改的是 #1（根状态）与 #3（声明卡片）**，#2 已经是对的。

- **改什么**：写 `docs/design/notation-aware-printing.md` 的第一节：对 unit01/08/12
  与对应解答，逐 `sorry`/逐 `by` 步记录"这个 surface 的文本有没有记法"，
  用 §2.5 的四个生产者归类。
- **判据**：表里每个格子都有实测输出；明确"用户说的'goal 没记法'是哪一个 surface"。
- **依赖**：无。**这一步先做，避免修错 surface。**

#### T-C02 消费者审计（**不然会静默改坏判卷**）

> **✅ 完成（2026-09-21）**：`docs/design/notation-aware-printing.md` §2（五个字段 ×
> 消费者，逐条 `file:line`，分"给人看"/"回读"）。**关键结论是一条不对称**：
>
> * **`ty_text` 只有"给人看"的消费者**（`query/mod.rs:640/651`、`query/state.rs:54`、
>   `lsp/src/lib.rs:1597/1881`）⇒ **可以就地改**，这是最便宜的一刀（T-C20）；
> * **`goal` / `binders[].ty` / `sub_goals[].ty` 同时是 judge 的输入**
>   （`suggest.rs:412/418/425`、`goals.rs:461-462/499`、`by.rs:501-506`）⇒
>   **不能就地改**，要改只能在**显示出口**（`query_map.rs` 组 wire 处 / CLI 打印处）
>   做重写，让"回读拿到的"与"人看到的"分成两份。
>
> 另加 §2.3：线 C 会让 `kernel-diff.sh` 报差异，那是**预期的**——判定正确性要看
> 课程门禁计数逐项不变 + 接受/拒绝集合不变，显示文本的有意更新归 T-C40。

- **改什么**：把 `DeclState.ty_text` / `DeclState.goal` / `DeclInfo.ty` /
  `GoalBinder.ty` / `ByGoal.ty` 的**全部**消费者列出来，分两类：
  **给人看**（Infoview、hover、树 tooltip）与**回读**（`judge_terms`、
  `judge.rs::fold_declared:1284`、`wrap_binders:1327`、`proof::parse_expr_text_with:53`、
  `canonical_goal_with_spec`）。
- **判据**：每条都有 `file:line`；**回读类一个都不许被改到**。

#### T-C03 设计文档：**把已有的 `printback-feasibility.md` §4 升格为权威设计**

> **✅ 完成（2026-09-21）**：升格进 `docs/design/notation-aware-printing.md` **§3**
> （notes 稿加了"已升格"抬头，留作调研现场）。三件事都做了：
>
> ① **§3.1 为什么不走内核 pp**——**发现 A**：内核的记法打印是**死代码**
> （`ExportFile.notations` 全仓库无一处 insert；`pp_app` 的记法分支永不触发），
> 且就算填表也不命中（`pp_app` 要 `args.len()` 恰好 1/2，而 `∈` 展开成 3 个实参的
> `Set.mem α a A`；零元记法走 `pp_const`；已有 Infix 分支**取操作数顺序是反的**
> 且零覆盖、`priority - 1` 在 0 时下溢）。**发现 B**：`pp_expr` 同时是
> `#check`/`#reduce`/`#print` 的出口 ⇒ 改它就动 `--json` 字节；`render_expr` 同理
> （产物同时是 judge 的回读输入）。⇒ **记法绝不能从内核 pp 走**。
> ② **§3.2 arity 硬规则**：只有 `spine.len() == arity` 才是记法实例
> （`Set.mem α a` 是部分应用，不许回显成 `α ∈ a`），arity 来源两级（闭包声明 AST +
> prelude 源码 parse 一次缓存 / 兜底 `judge_type_of`）。
> ③ **两处失效理由就地作废**：`notation-subset.md:554` 与 `course-lean-style.md:1124`
> （N-3）的"内核冻结"——都改成"理由作废 + 真正的理由 + 指向新设计"。
>
> §3 另含：落点表与**明确不落**的红线（`compile/**` 一字不改、
> `CheckEvent::TypeChecked/Reduced` 绝不碰）、`DisplayText` 编译期护栏
> （让 `parse_expr_text(&display_text)` **编译不过**）。

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

> **✅ 完成（2026-09-21）**：`crates/front/src/display.rs`——`DisplayText(String)`
> 无 `Deref` / 无 `as_str` / 无 `Into<String>`，唯一读法 `as_display_str()`。
>
> **判据用 `compile_fail` doctest 钉住**（Rust 自带，零依赖），而且**对变异敏感**
> ——两条都做过变异检查：
>
> | doctest | 钉什么 | 变异检查 |
> |---|---|---|
> | `parse_expr_text(&shown)` 编译不过 | 没有 `Deref<Target = str>` | 加 `Deref` ⇒ **红** ✓ |
> | `shown.as_str()` 编译不过 | 没有"顺手拿回 `&str`"的口子 | 加 `as_str()` ⇒ **红** ✓ |
>
> 第三条是**正向** doctest（`as_display_str()` / `Display` 必须能用），防止把护栏
> 做成"谁都读不出来"。**为什么这条必须先于任何折叠代码**：折叠一旦落地，护栏是
> 唯一能保证"显示文本没被喂回 judge"的机制。

- **改什么**：定义 `struct DisplayText(String)`——**无 `Deref`、无 `as_str`、
  无 `Into<String>`**，只提供 `as_display_str()`。目的是让
  `parse_expr_text(&display_text)` / `judge_terms(…, &display_text)` **编译不过**
  （`printback-feasibility.md` §4 的核心护栏）。
- **判据**：故意写一行 `parse_expr_text(&display_text)` → **编译失败**
  （这条"反向测试"就是护栏的验收）。
- **依赖**：T-C03。

#### T-C04 记法表复用 `judge.rs` 的重建（提成公共函数）

> **✅ 完成（2026-09-21）**：`crates/front/src/notation.rs::notation_table(&[Command])
> -> Vec<NotationDecl>`——**逐字**从 `judge.rs` 提出来（行为不变；取 `&[Command]`
> 而不是 `&str` 是因为判定路径的前缀**本来就已解析**，零额外开销）。
> 4 条单测：声明顺序、`scoped` 未 open 时被滤掉、普通记法一直生效、
> 以及下面那条陷阱。
>
> ⚠ **提出来的时候发现一个真陷阱（已记台账 G-35）**：这个函数是**扫一遍**而不是
> 两遍——`open scoped Foo` 写在 `scoped infix` **之后**（正常写法）时收不到那条
> 记法；注释里写的却是"取前缀结束时生效的那些"（= 两遍扫描的意图）。
> **没有顺手改**（T-C04 是纯提取，行为变更要自己的复现与验收），而是：
> ① 特征化测试钉住当前行为（`open_scoped_after_the_notation_does_not_bring_it_back`）；
> ② 台账 G-35 + 复现件。**线 C 会复用同一张表**，所以要么在 T-C11/T-C12 之前修掉，
> 要么保证两边同口径——"一起漏"一致，"一边漏一边不漏"才是灾难。

- **改什么**：把 `crates/front/src/judge.rs:322-355` 的表重建提成
  `pub(crate) fn notation_table(prefix_src: &str, options: &CompileOptions) -> Vec<NotationDecl>`
  （或等价物），两处共用。
- **判据**：`cargo test -p sokonanoda-front -- --nocapture` 全绿（**行为不变**，
  纯提取）。

### 6.2 C-II 折叠层

#### T-C10 折叠函数第一刀：只做二元 infix 族

> **✅ 完成（2026-09-21）**：`crates/front/src/display.rs` 的
> `DisplayNotations{table, arity}` + `print_back(text, &dn) -> DisplayText`。
> **8 条单测**（判据要的 5 类 + 3 条护栏），`cargo test -p sokonanoda-front display`
> 全绿。第一刀只做二元 infix 族（一元前缀/后缀与 binder 记法的操作数位不同，
> 一起做会把这一刀撑大）。
>
> **做的时候定下两条性质**（都成了测试）：
> ① **一处都没折 ⇒ 逐字节原样返回**——**这条比"折对了"更要紧**：`print_back` 的
> 最后一步是 `render_expr`，而它会把 `forall (a b : T), …` **拆成箭头链**、
> 把 `Type 0` 重排成 `Sort 1`（它的产物是回读通道的输入，故意的）。无条件重渲染
> 会让**每一条不带记法的类型都跟着改样子**；有了这条性质，显示漂移被限制在
> "真的折过"的文本里。
> ② **命中不了就原样**（不猜）：表里没有 / 元数对不上（部分应用）/ 含松散变量
> `$N`（F3）/ 解析不了——全部逐字节返回。
>
> **留给 T-C20 的决定**：既然"折过就重渲染"，**含记法的**类型文本会连带换一种
> binder 写法（`forall (α : Type 0) (A B : Set α), A ⊆ B` →
> `(α : Sort 1) -> (A : Set α) -> (B : Set α) -> A ⊆ B`）。接进生产者那一环拍板：
> 接受，还是让显示出口做**源保留拼接**（只替换折过的子树）。

- **改什么**：`App(App(f, a), b)` 且 `f` 命中表 ⇒ `a ∈ b`；处理结合性与优先级
  所需的括号；**剥掉隐式前导实参**；命中不了就**回退点名**（不猜）。
- **判据**：`cargo test -p sokonanoda-front display -- --nocapture`
  新增 5 条：左结合 / 右结合 / 优先级加括号 / 嵌套 / 不命中回退点名。

#### T-C11 arity 的来源

> **✅ 完成（2026-09-21）**：`display::arities_in_sources(&[源文本])` 与
> `arities_with_prelude(&[源文本])`——parse 每段源，按 `namespace` 累积的**全名**
> 记下「声明 → telescope 层数」。prelude 的 `PRELUDE_EQ_SRC`/`PRELUDE_L1_SRC`
> 已并进后者（`And`/`Or`/`Not`/`Iff`/`Eq`/`Exists` 住在那里）。
> **5 条新单测**（共 13 条）。
>
> **判据实测**：`infix:50 " ∈ " => Set.mem` 的 telescope = **3**（`α`/`a`/`A`），
> 操作数 = 2 ⇒ **前导参数 1 个**（那个 `α`）。两个口径在文档里写清了：
> **telescope**（= `spine.len()`）vs **操作数个数**。
>
> 三个踩到的点：① `def f (a : T) (b : T) : U` 是**一个 `Forall` 带两个 binder**
> ⇒ 数 binder 不数节点；② **parser 已经把名字限定好了**（`namespace Foo` 里的
> `def bar` ⇒ `Foo.bar`）⇒ 自己再拼一次会得到 `Foo.Foo.bar`（踩过）；
> ③ 归纳类型要数 `params` + 类型上的 binder。
>
> 顺带把 `PRELUDE_EQ_SRC` / `PRELUDE_L1_SRC` 从 `pub(crate)` 提到 `pub`
> （`arities_with_prelude` 要用；线 C 的四个生产者都会经过它）。

- **改什么**：给折叠层提供"记法吃几个显式参数"。来源二选一并写进设计：
  ① 目标声明的**源级签名**里非隐式参数个数（`KnownName` 已存源级签名，
  `crates/front/src/compile/elab.rs:515/557`）；② `report.decls[i].ty_text`/`signature`。
- **判据**：单测：`infix:50 " ∈ " => Set.mem` 的 arity = 2（`α` 是隐式前导）。

#### T-C12 `scoped` 的保真度

> **✅ 完成（2026-09-21）——决定 + 修掉 G-35。**
>
> **决定：不做位置精确，但要两遍扫描。** 读回表**复用** `notation_table`
> （T-C04 的唯一实现），语义定为「**这段文本结束时**生效的那些」。理由：
> parser 的 `opened_scopes` 是**逐命令**推进的（`open scoped` 只影响它**之后**
> 的代码），那是**编译期**的关切；读回通道只有一段前缀、不关心"用在哪一行"，
> "结束时生效"才是它要的答案。
>
> **但"结束时生效"必须真的按结束时算**：一遍扫描会让「先 `scoped infix` 声明、
> 后 `open scoped`」（**正常写法**：声明在库里、`open` 在使用处）漏掉那条记法
> ——那就是 T-C04 提取时发现的 **G-35**。本轮**修掉**（两遍：先收齐
> `open scoped`，再按声明顺序过滤），台账已关账 `fixed_in = 0.64.2`。
> 主通道 parser 两个方向都对（`activate_scope` 的 pending 表 + 记住作用域），
> 读回表对齐的就是这个语义。
>
> 判据：`notation::tests::open_scoped_after_the_notation_is_collected`
> （两个方向都断言）+ G-35 的复现件（现在 exit 1 = 已修）。

- **改什么**：`judge.rs:332-355` 的近似是"前缀里出现过 `open scoped` 就算生效"。
  折叠层要不要位置精确（对齐 parser 的 `opened_scopes`，`parser.rs:643-680`）？
  决定并写进设计；若要，加测试。
- **判据**：决定 + 测试。

#### T-C13 重载（一个符号 → N 个目标）的处置

> **✅ 完成（2026-09-21）——决定 + 2 条测试。**
>
> **两个方向分开看**：
> * **同一符号、N 个 target**（`⊕` => `AddA` 与 `AddB`）：**不是歧义**。反向折叠的
>   判据是 **head 名字** ⇒ 两个 head 各自折成同一个符号，都对。前向那条路
>   （符号 → 候选目标）才需要按期望类型挑，与折叠层无关。
> * **同一 target、两个符号**（`∈` 与 `∊` 都 => `Set.mem`）：这是反向**唯一残留的
>   歧义** ⇒ **取声明顺序第一个**（`Vec` 顺序）。测试把声明顺序倒过来，折出的符号
>   跟着变——证明判据确实是顺序本身。
> * 折叠出来的 `Expr::Notation` 的 `alternatives` 一律**空**：target 已知且唯一，
>   候选列表是前向路径的东西。
>
> 判据：`display::tests::one_target_with_two_symbols_takes_the_first_declared`、
> `one_symbol_with_two_targets_folds_by_head_without_ambiguity`（`display` 共 15 条）。

- **改什么**：决定折叠层遇到重载时"折叠成哪一个"或"回退点名"。
- **判据**：决定 + 测试。

#### T-C14 折叠层的损失护栏

> **✅ 完成（2026-09-21）——护栏分三层，从强到弱**（设计 §3.4b）：
>
> | 层 | 护栏 | 判据 |
> |---|---|---|
> | **类型**（最强） | `DisplayText` 无 `Deref`/`as_str`/`Into<String>` | `compile_fail` doctest（T-C03b） |
> | **幂等** | 折过的文本再折一次**一个字节不变** | `display::tests::folding_is_idempotent` |
> | **可解析** | 折叠产物必须能**重新解析** | `display::tests::folded_text_reparses` |
>
> **"可解析"不是"逐字节回读等价"**（这条容易混）：折过的文本重新解析得到的是
> `Expr::Notation` 节点，**结构上不等于**展开后的 `App`——那正是记法的定义。
> 真正的结构性保证是**类型那一层**。
>
> **为什么没复用 `is_rereadable` / `keep_if_lossless`**：那两个管的是**另一条**路
> （`by` 引擎的 pp→parse 往返），折叠层不经过它。`render_expr_round_trips`
> （`crates/front/src/compile/tests.rs`）本轮**一字未动**（实测仍绿）。

- **改什么**：折叠不得让文本变得**不可回读**（若折叠结果会进入任何回读通道）。
  复用 `is_rereadable` / `keep_if_lossless` 的既有判据。
- **判据**：`cargo test -p sokonanoda-front -- --nocapture` 绿；
  `render_expr_round_trips`（`crates/front/src/compile/tests.rs:1474`）**必须不动**。

### 6.3 C-III 接进四个生产者

#### T-C20 生产者 1+3：根状态与声明列表的 `ty_text`

> **✅ 完成（2026-09-21）——用户报的那条（G-26）关账。**
>
> **做法**：`finish_pass` 里**建一次** `DisplayNotations`，两处 `ty_text`
> （开练习分支 + 普通声明分支）各过一遍 `print_back`。表从**闭包各单元的已解析
> 命令**收（零额外解析）+ 内建记法；元数从源级签名 + prelude（`OnceLock` 缓存）。
> **只动 `ty_text`**（T-C02 的审计：它只有给人看的消费者）；`goal` /
> `binders[].ty` / `sub_goals[].ty` **一个字节没动**。
>
> **判据实测**：
> ```
> demo_subset_def | forall (α : Type 0) (A B : Set α), (A ⊆ B) ↔ ((x : α) -> A x -> B x)
> mem_of_subset   | forall (α : Type 0) (A B : Set α), A ⊆ B -> (forall (a : α), a ∈ A -> a ∈ B)
> 根状态（L45）    | 同上（学习者的光标就在 tactic 上，看到的就是它）
> ```
> **`⊆` ✓ `↔` ✓ `∈` ✓，而 binder 分组、`Type 0`、折行全部原样。**
> 缺口 **G-26 关账**（`fixed_in = 0.64.2`），它的复现件转绿。
>
> **接进生产者时撞到的两件事**（设计里没写、实测才知道）：
> ① **必须按 span 拼接，不能重渲染整棵树**——重渲染会把折过之外的东西也改样
> （`forall (a b : T),` 拆成箭头链、`Type 0` 重排成 `Sort 1`）。改成把每处折叠记成
> `(span, 文本)`、**只替换那几段**（取最外层、从右往左）。两处细节：
> `parse_expr_text_with` 的 span 多一个 `"#check "` 前缀（**头部反推**，不硬编码）；
> 解析器给**带括号的原子**的 span **不含括号** ⇒ 替换范围要**按括号配平**。
> ② **内建记法要自己补**：`↔`/`∧`/`∨`/`¬`/`=`/`≠` 不在任何源文本里（parser 硬编码）
> ⇒ `notation_table` 收不到，`Iff` 永远折不成 `↔`。
>
> **顺带修正 arity 的口径**：**元数 = 显式 binder 的个数**（内核 pp **省略隐式
> 参数**）：`Eq {α : Sort u} (a b : α)` ⇒ 元数 **2**（pp 是 `Eq A B`）、
> `Ne (α : Sort u) (a b : α)` ⇒ **3**。用 telescope 层数会让 `=` 永远折不出来。
>
> **判据**：`cargo test -p sokonanoda-front display` **20 条** · 课程门禁计数
> **逐项不变**（36 目标 · 328 checked · 99 open · 0 判负）· `perf-check
> --case perf_course` **无退化**（±2.3% 内）· 更新的 golden 两处
> （`query::tests::state_at_root_before_any_tactic`、LSP 的两条 state 用例）
> ——都是**预期的**可见变化，注释里写明是线 C 的效果。

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

> **✅ 完成（2026-09-21）——补了 3 条守护。**
>
> 这条路本来就好（T-C01 的实测表：不带 `by` 的开练习，`goal` 走 `render_expr`
> ⇒ 记法保留），但**此前零测试**（断言里的 `∈`/`⊆` grep 命中 0）——线 C 的其它
> 环节都在动显示文本，这是"随时可能被改坏而没人发现"的状态。补的：
>
> | 层 | 测试 | 钉什么 |
> |---|---|---|
> | front（真相层） | `query::tests::an_open_exercise_without_by_keeps_notation_in_its_goal` | `decl.goal` 含 `⊆`/`∈`，**且不含点名**（`Set.subset`/`Set.mem`） |
> | front（顺带） | `query::tests::a_declarations_ty_is_notation_folded_too` | `decl.ty` 也带记法（证明 T-C20 的折叠真的接到了 `ty_text` 上） |
> | LSP（wire） | `tests::goals::goals_keep_notation_in_the_goal_text` | 过了 `query_map` 之后**还在**（判据原话就是 `soko/goals` 的 `decl.goal`） |
>
> 夹具要点：**不带 `by`**（带 `by` 的走生产者 1 = 内核 pp，那是另一条路，
> T-C20 才修）。

- **改什么**：T-C01 的实测若显示这一路已经保留记法（§2.5 的 unit01 例就是），
  则**补测试钉住**（现在零覆盖：断言里的 `∈`/`⊆` grep 命中 0）。
- **判据**：新增测试：`soko/goals` 的 `decl.goal` 含记法。

#### T-C22 生产者 4：`by` 步进里被 pp 化的四处

> **✅ 完成（2026-09-21）——展示副本折叠，判定输入一个字节没动。**
>
> **做法**：表**整趟建一次**（`run_pass` 的 `display_notations(units)`），
> `Walk` 与 `finish_pass` **共用同一份**；折叠点选在 **`by_step_states`**
> （`check/mod.rs`）——它把引擎的 `ByGoal` 转成报告层 `ByStepState`，
> **那就是展示边界**。引擎手里的 `nodes[id].ty` 一个字节没动。
>
> **实测**（`query state` 在 `exact` 行上，unit04）：`apply Set.ext` 之后
> `(x : α) -> Iff (A x) (B x)` → **`(x : α) -> (A x) ↔ (B x)`** ✓
>
> **判据**（`query::tests::by_step_display_is_folded_but_the_judge_input_is_not`）
> **两面都要**（这条是计划点名的"最容易出错的地方"）：
> * **展示**：含 `↔`/`∈`，**不含** `Iff`/`Set.mem`；
> * **判定**：同一个 `by` 块后面的 `exact h` 仍然判过（`status == "checked"`）
>   ——折叠若误伤判定输入，子目标回读会失败、这条声明就判红。
>
> 消费者核对：`by_steps` 的读者里 `session.rs` 只做 span 平移、
> `query/mod.rs` 的 `DeclInfo.goals` 与 `query/state.rs` 都是展示、
> `suggest.rs` 读的是**另一组**字段（`DeclState.goal`/`sub_goals[].ty`，没动）。

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

> **✅ 完成（2026-09-21）——判据已满足，补守护。**
>
> 实测：`query state` 的 `binders[].ty` **本来就带记法**——
> `h : A ⊆ B`（`⊆` 在）。原因：binder 的类型来自**源里写的**类型
> （`fun (h : A ⊆ B) => …`），走 `render_expr` 的源级渲染。
> `by` 那条路（by-step 的 binder）由 **T-C22** 折过 ✓，两条都覆盖到了。
>
> 补的守护：`query::tests::state_binders_keep_notation_in_their_types`——
> 夹具刻意用**不带 `by`** 的开练习（那条走 `DeclState.binders` 那份，与带 `by`
> 的 by-step 那份是**两条路**，两条都要有人守），断言假设行含 `⊆` **且不含点名**。

- **判据**：`query state` 的 `binders[].ty` 含记法。

#### T-C24 逐 surface 的判别性测试

> **✅ 完成（2026-09-21）——四条测试 + 一条机械判据 + bump minor 到 0.65.0。**
>
> 四条 surface 测试（都已在前面各环节落地）：
>
> | # | surface | 测试 |
> |---|---|---|
> | 1 | 根状态 | `query::tests::state_at_root_before_any_tactic`（断言 `∧`） |
> | 2 | 无 `by` 的开练习 | `query::tests::an_open_exercise_without_by_keeps_notation_in_its_goal` |
> | 3 | 声明 `ty` | `query::tests::a_declarations_ty_is_notation_folded_too` |
> | 4 | `by` 步进 | `query::tests::by_step_display_is_folded_but_the_judge_input_is_not` |
>
> **判别性**（"折叠被关掉时必须红"）——加了开关
> **`SOKO_NO_NOTATION_FOLD=1`**（`display_notations` 直接返回空表，仿
> `SOKO_NO_JUDGE_BATCH`），实测关掉后：
>
> | surface | 关掉后 | 读法 |
> |---|---|---|
> | 1 根状态 | **红** | 记法是折叠给的 |
> | 3 声明 `ty` | **红** | 同上 |
> | 4 `by` 步进 | **红** | 同上 |
> | 2 无 `by` 的开练习 | 仍绿 | 它的记法来自 `render_expr` 的**源级渲染**，不是折叠 ⇒ 这条是**守护**（T-C21） |
> | 假设行 `binders[].ty` | 仍绿 | binder 类型是**源里写的** ⇒ 守护（T-C23） |
>
> 机械判据**两条，缺一不可**：
>
> * **折叠层**：`display::tests::with_the_fold_off_every_foldable_surface_is_pointwise`
>   ——空表 ⇒ 一律点名（证明"记法确实是折叠给的"）；
> * **整条路**：`crates/cli/tests/notation_fold.rs` 三条真二进制 A/B（开/关
>   `SOKO_NO_NOTATION_FOLD`），把上表**逐行**变成断言：
>   ① `the_fold_switch_turns_every_foldable_surface_pointwise`（三个可折 surface
>   关掉 ⇒ 回到点名）· ② `source_rendered_surfaces_ignore_the_fold_switch`
>   （两个源级 surface 关掉 ⇒ **一个字节不变**）·
>   ③ `the_fold_switch_never_changes_the_judge`（`grade --json` **逐字节相同**）。
>
> **为什么非要第二条**：那条单测用的是 `DisplayNotations::default()`，它**碰不到
> 开关本身**（`display_notations` 里那个 `if`）——开关被删、或有哪条路绕过了
> `display_notations`，单测照样绿。第 ② 条同时是**行程开关**：谁把生产者 2 改成
> 走内核 pp 折叠，它就会红，逼他回来更新上面那张表。
>
> **⬆ BUMP minor → 0.65.0**：goal / 假设 / 声明类型**第一次**显示记法（§0.2 的
> bump 动作"用户可感知的能力落地"）。

- **改什么**：四个生产者**各一条**测试，断言"该 surface 含记法"；
  并在折叠层被关掉时**必须红**（判别性）。
- **判据**：`cargo test -p sokonanoda-front -- --nocapture` +
  `cargo test -p sokonanoda-cli --test notation_fold`（真二进制 A/B）。

#### T-C25 边界：命中不了就回退

> **✅ 完成（2026-09-21）——5 种边界各一条测试 + 修掉一个真 bug。**
>
> **边界**（只做二元 infix 族，其余**回退点名不猜**；写法都取课程库的真实例子）：
> `prefix:100 " 𝒫 "` / `postfix:100 " ᶜ "` / `notation "∅"` /
> `binder_notation "∃"` / 元数对不上（`Set.mem α a`）。测试
> `display::tests::only_binary_infix_folds_and_the_rest_fall_back` +
> `partial_and_over_application_fall_back`（后者带一条**对照**：同一夹具里的
> 二元 infix 照折，证明表确实建起来了、前面四条不是"空表造成的假绿"）。
>
> **顺带修掉一个真 bug**：折过的子树**被应用**时就地替换会**改变语义**——
> `(Set.mem α a A) B` 折成 `a ∈ A B`，重新解析是 `Set.mem α a (A B)`。
> 规则改成"**上提到应用脊根**，括号交给 `render_expr`" ⇒ `(a ∈ A) B` ✓。
>
> **踩到的排序坑**（隐蔽）：记法节点取被折那段的 span，而"上提到脊根"那一处
> **起点相同**；只按起点稳定排序会让**内层排前面**，"取最外层"规则反而丢掉真正的
> 外层（`(A ∪ B) ∪ C` 退化成 `Set.union α (A ∪ B) C`）。改成按
> `(起点, 终点倒序)` 排——**起点相同时长的在前**。
>
> 判据：`display` **24 条**全绿；用户可见输出复测不变
> （`(A ⊆ B) ↔ ((x : α) -> A x -> B x)` 等）。

- **改什么**：`prefix` / `postfix` / 零元 `notation` / binder 记法 / 重载歧义
  ⇒ 回退点名，不猜。每种一条测试。
- **判据**：`cargo test -p sokonanoda-front -- --nocapture`。

### 6.4 C-IV 语义着色（独立缺陷）

#### T-C30 `semantic::tag_runs` 填 `Names::notations`

> **✅ 完成（2026-09-21）——实测撞到三件事，比计划写的多两件。**
>
> 计划只写了"补 `notations`"，实测发现补齐它**还不够**：
> ① 符号表要**扫整个闭包**（`∈`/`⊆` 声明在 `lib/Set.sokonanoda`，入口只 `import`
> 了它），而且**不能 parse**——"使用库记法的文件单文件 parse 必然失败"（记法随
> `import` 传播）⇒ 用**词法级扫描** `token::scan_notation_decls`（省掉"每次查询
> parse 一遍闭包"的钱）；
> ② 还要把符号**喂给词法**：`↔`/`¬`/`≠` 不在数学符号码点类里，不喂就切成 `Ident`
> ⇒ `unknown_ident`（比"没有 kind"更糟）⇒ 改用
> `tokenize_with_symbols(text, notations)`；
> ③ **内建也要算**（`∧`/`∨`/`↔`/`¬`/`=`/`≠` 不在任何源文本里）。
>
> 判据实测：`query goals` 的 `ty_runs` 里 `⊆`/`↔` 都是 `{"kind":"keyword"}` ✓
> 且 runs 能逐字重建文本 ✓。测试
> `semantic::tests::tag_runs_marks_notation_symbols_as_keywords`（带一条**反向**
> 断言：不喂符号表时 `↔` 掉成 `unknown_ident`——修之前的样子）。
> 顺带把 T-C22 里弱化的 LSP 断言**加强回来**（`goal_kinds` 现在含 `keyword`）。

- **根因**：§2.7 第 2 条。
- **改什么**：`crates/front/src/semantic.rs:240` 建 `Names` 时补 `notations`
  （对齐 `classify` 在 `:544` 的做法）。
- **判据**：wire 上 `∈` 的 run 有 `kind`（不再是裸 `{"text":"∈"}`）；
  `cargo test -p sokonanoda-front semantic -- --nocapture` 新增断言。

#### T-C31 目标文本里的**导入名**不再标 `unknown_ident`

> **✅ 完成（2026-09-21）。**
>
> **根因**：`decl_kinds()` 只看入口文件 ⇒ 项目文件里 `Set`/`Set.mem` 全被标成
> `unknown_ident`（线 C 之后 goal 里全是这些名字，一眼就看得出来）。
>
> **做法**：闭包级声明表在**编译期算一次**（`QueryDoc::compute_closure_decls`，
> 每个模块一次 `parse`——编译本来就在解析它们），并进 runs 计算。
> **不是每次查询算一遍**：`state_at` 是**光标一动就问一次**的，12 个模块 × 每次
> 光标移动的 parse 是白烧。剥 `import` 行用 `importless_source`（与
> `judge_prefix` 同一手法——被导入的模块自己也有 `import`）。
>
> **判据实测**（`query goals` unit01 的 `ty_runs`）：`Set` → **`def_use`** ✓
> （修前是 `unknown_ident`）。测试
> `crates/cli/tests/query.rs::query_goals_classifies_imported_names_and_notation_symbols`
> ——一个**真项目**（lib + 入口），断言导入名有 kind、`∈` 是 `keyword`、
> 且 `unknown` 里不再有导入名。
>
> **已知剩余**（不在本环节）：签名**自己的** binder 名（`forall (α : Type 0)
> (A B : Set α), …` 里的 `α`/`A`/`B`）仍是 `unknown_ident`——它们不在
> `DeclState.binders` 里（那是 **goal 的** binder 列表；没有 lambda 前缀的
> `:= sorry` 声明它是空的），而 `ty_text` 的 binder 只存在于文本里。
> 要修得让 `tag_runs` 认文本里的 `(name :` 形态，属于另一件事。

- **根因**：`decl_kinds()` = `declaration_kinds(&self.text)`（`query/mod.rs:274`）
  只看入口文件 ⇒ 项目文件里 `Set.mem`/`Set`/`α` 全是 `unknown_ident`。
- **改什么**：把闭包级声明表喂进 runs 计算。
- **判据**：unit 文件的 `goal_runs` 里 `Set.mem` 有正确 `kind`。

#### T-C32 着色在 Infoview 里可见

> **✅ 完成（2026-09-21）——两条 webview 断言（判据给的二选一里的那条便宜的）。**
>
> **先说结论：渲染这一侧本来就是通的**——`infoview.js` 把每个 run 渲染成
> `tok-<kind>` 的 span，CSS 的 `.tok-*` 规则读 `--soko-<category>`，四个主题里
> `--soko-keyword` 都有定义。**服务端给对了**（T-C30）与**用户看得见**是两件事，
> 缺的是**测试**：既有用例只覆盖 `axiom_use`/`unknown_ident`/`sort`，没有
> **记法符号**（`keyword`）。
>
> 补的两条（`editor/vscode/test-webview.js`）：
> * `state: notation symbols render as tok-keyword spans`——目标行
>   `A ⊆ B -> (A ↔ B)` 的 runs 逐字重建文本，且 `⊆`/`↔` 各是一个
>   `tok-keyword` span；
> * `decls: notation symbols render as tok-keyword spans in the type line`——
>   同一个东西在**声明卡片**那一侧（两个 surface 都要有）。
>
> **转发链路核对**（不需要新测试，读代码即可）：扩展 `_pushState` 是
> `Object.assign({ type: "state", uri }, state)`——**全字段透传**，
> `goal_runs`/`ty_runs` 不会在中途被挑拣掉 ✓。
>
> 判据：`node editor/vscode/test-webview.js` **13/13** ·
> `node editor/vscode/test-extension-host.js` **34/34**。

- **判据**：真宿主 e2e 或 `test-webview.js` 的 runs 断言。

#### T-C50 真宿主 e2e：goal 文本用记法（矩阵用例 #6）

> **✅ 完成（2026-09-21）——用例本来就在（T-015..T-017 时写的），本轮确认它转绿。**
>
> `editor/vscode/src/test/extension.test.js` 的
> `test("goal text uses the file's notation")` 就是判据点名的用例 #6 ✓：
> 光标落在夹具 `units/u01.sokonanoda` 的 `sorry` **行内**（不是行后——注释里写了
> 那次假绿的教训：`revealRange` 把光标停在 range 末尾，正好落到"无 by 的声明级
> 目标"那一支 ⇒ 记法本来就在 ⇒ 假绿），断言 `lastState().goal` 含 `⊆` 或 `∈`。
>
> **实测**：`scripts/vscode-e2e.sh --grep "goal text uses the file's notation"
> --profile debug` → **1 passed / 0 failed** ✓（线 C 之前它是 known-red 的三条之一）。
> 台账条目如实记了 `grep` 字段（filtered run 不会被误读成全量）。

- **改什么**：新增用例 `goal text uses the file's notation`：把光标放到夹具
  `units/u01.sokonanoda` 的 `sorry` 上，断言 `infoview.lastState().goal`
  含 `∈` 或 `⊆`（修复前必须是红的）。
- **判据**：`scripts/vscode-e2e.sh --grep "goal text uses the file's notation" --profile debug --no-build`
  → 1 passing。
- **依赖**：T-015、T-016、T-017、T-C24。

### 6.5 C-V 收尾

#### T-C40 断言与 golden 更新（**计数中性**）

> **✅ 完成（2026-09-21）——审计：只动了 5 处，全部按实测重钉。**
>
> **做法**：不按计划里给的行号审（那些行号早被前面的环节挪走了），改成审
> **`git diff 7874dd4..HEAD` 里测试文件的每一条 golden 改动**——这才是"逐条按实测
> 重钉、不做机械替换"的可核对形式。
>
> **结果**：线 C 全程只重钉了 **5 处**，全是 `And` → `∧`（折叠给的），每一处都是
> 先跑测试、读**实际输出**再改的：
>
> | 文件 | 改动 | 属于 |
> |---|---|---|
> | `crates/front/src/query/tests.rs` | `forall (a b : Prop), And a b -> And b a` → `… a ∧ b -> b ∧ a` | 生产者 1（根状态） |
> | `crates/front/src/query/tests.rs` | `state.goals[0].goal`: `And b a` → `b ∧ a` | 生产者 4（by 步进） |
> | `crates/front/src/compile/tests.rs` | `s0.goals[0].ty`: `And a a -> a` → `a ∧ a -> a` | 生产者 4 |
> | `crates/front/src/compile/tests.rs` | `s1.goals[0].binders[1].ty`: `And a a` → `a ∧ a` | 生产者 4（假设行） |
> | `crates/cli/tests/query.rs` + `crates/lsp/src/tests/state.rs` | 同上两条的 wire 版本 | CLI / LSP |
>
> **没动的**（也核过）：`crates/lsp/src/tests/goals.rs`、`crates/front/src/proof.rs`、
> 以及 `crates/kernel/tests/pretty_printer.rs`（内核 pp **一个字节没改**——T-C03 的
> 红线：它是 `#check`/`#reduce`/`#print` 的出口）。**空断言扫描**（`contains("")`
> / `assert!(true)` 之类）无命中。
>
> **判据**：`cargo test --workspace --locked` → **exit 0**（39 个 suite 全绿，0 failed）·
> `python3 courses/set-theory/tools/check.py` → **36 目标 · 328 checked · 99 open ·
> 0 判负**（**逐项不变**）· `scripts/soko gate` → **exit 0**。

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

### ✅ 检查点 CP-C（批次 3，minor 版本）——**已过（2026-09-21）**

- [x] `bash scripts/verify-editor-issues.sh` 第 5 条转「已修」
      → 实测 **已修 6 · 缺口仍在 1 · 环境异常 0**（第 5 条 = G-26 **已修**；
      剩的第 6 条 = G-23 记法导航/hover，属**线 D**）
- [x] 四个生产者的判别性测试全绿
      → `display` 24 条 + `query`/`goals` 的 5 条 surface 守护 + `semantic` 26 条；
      关掉折叠（`SOKO_NO_NOTATION_FOLD=1`）后**生产者 1/3/4 红、2 与假设行仍绿**
      ——正是设计里的分工
- [x] 课程门禁计数**逐项不变**
      → **36 目标 · 328 checked · 99 open · 0 判负**
- [x] `cargo test --workspace --locked` 全绿
      → **exit 0**（39 个 suite，0 failed）
- [x] **性能无退化**：`python3 scripts/perf-compare.py --since c74c0046` **exit 0**
      （"没有退化，也没有哨兵消失"）
      → ⚠ 一处**在阈值内但未解释**的：`lsp-course/did_open` +13.6%/+15.6%/+18.4%
      （<25% 阈值）。已排除折叠与防抖、已确认那是 **debug 构建**的度量，
      排查记录在 `docs/PERF.md` 的"待查"一节
- [x] **真宿主 e2e**：用例 #6 `goal text uses the file's notation` 转绿
      （全量 **23 passed / 2 failed**，剩的两条是线 D 的 #7/#8）
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

> **✅ 完成（2026-09-21）——比计划写的多一件事：`import` 来的记法够不着目标。**
>
> 计划说这是"全计划最便宜的一刀（一个 `pub` API 调用 + 一行文本）"。
> 实测撞到一条**闭包**问题（与 T-C30/T-C31 同族）：
> `notation_input::symbol_at` 解析展开目标时只看**本文件**的声明 + 内建
> ⇒ **`import` 来的记法（`∈`/`⊆`）目标永远是 `None`**（声明在别的文件里）
> ⇒ 连"展开成什么"那一行都没有，更谈不上签名。
>
> **修法**：加 `symbol_at_with_sources(text, offset, closure_srcs)`——目标解析
> 多一条**闭包前缀**的回退（LSP 侧就是 `judge_prefix` 那段）。签名走
> `judge_type_of_constant(prefix, options, target)`（自带**常量键**缓存）。
>
> **实测 hover**（G-23 的复现件）：
> ```
> `∈` —— 记法符号
> 展开成 `Set.mem`
> `Set.mem : forall (α : Type 0), α -> Set α -> Prop`     ← 原始类型 ✓
> 输入：`\in`（别名 `\mem`）
> `a ∈ A : Prop`
> ```
> 与计划给的期望文本**逐字一致** ✓。
>
> **那一行故意不折记法**：它叫"**原始**类型"，要回答的就是"底下站着什么"
> ——折成 `A ⊆ B` 反而把它要回答的问题盖掉了。
>
> **请求路径成本**（计划要求按 `spine-meta-a.md` 的纪律测一次）：
> `lsp-project/request_latency` 的 `hover_ms = 0ms`（交互预算 50ms）✓。
>
> 判据：新增 `crates/lsp/src/tests/hover.rs::hover_on_an_imported_notation_symbol_shows_the_raw_type`
> （**真项目**：lib + 入口，断言"展开成 `Set.mem`"与签名那一行）。

- **改什么**：`notation_symbol_hover`（`crates/lsp/src/lib.rs:809-858`）在
  `target` 已知时，用 `judge::judge_type_of_constant(query.judge_prefix(offset),
  query.mode, &target)` 取签名并渲染一行。
- **判据**：hover 含 `` `Set.mem : forall (α : Type 0), α -> Set α -> Prop` ``（文本以实测为准）。
- **成本**：**这是全计划最便宜的一刀**（一个 `pub` API 调用 + 一行文本）。
- **风险**：`judge_type_of_constant` 走内核 ⇒ 请求路径成本；它自带常量键缓存
  （`judge.rs:596-608`），但要按 `docs/design/spine-meta-a.md` 的请求路径纪律测一次。

#### T-D03 hover 的"原始类型"只对**能解析出 target** 的符号显示

> **✅ 完成（2026-09-21）——三种形态各一条测试；反向那条钉在函数层。**
>
> | 形态 | 测试 | 断言 |
> |---|---|---|
> | **本文件声明** | `hover_on_a_locally_declared_notation_symbol_explains_it` | `myop : Prop -> Prop -> Prop` |
> | **语言内建** | `hover_on_a_builtin_notation_symbol_shows_the_raw_type` | `展开成 \`And\`` + `And : …` |
> | **`import` 来的** | `hover_on_an_imported_notation_symbol_shows_the_raw_type` | `展开成 \`Set.mem\`` + 签名 |
>
> **"解析不出就不显示"钉在函数层**（`notation_input::target_resolution_tests::
> a_symbol_nobody_declares_has_no_target`），不是 hover 层——因为**在能编译的
> 文件里这条不可达**：凡是认得出来的记法符号都必有 target（本文件声明的 ✓ /
> 内建的 ✓ / `import` 来的 ✓）。它是**防御性**的（半成品文件、输入法表里有但
> 没人声明的符号）。hover 那侧靠"那一行写在 `if let Some(target)` 里"
> **结构性**保证——想加一条 LSP 级的反向用例时撞到过这一点（构造不出一个
> "能编译 + 符号无 target"的文件），所以如实记在这里而不是硬凑一条假用例。
>
> **⬆ BUMP patch → 0.65.2**（§0.2："用户可感知的能力落地"——hover 显示记法的
> 原始类型，全计划最便宜的一刀）。

- **改什么**：本文件声明 / 内建 / **import 来的**三种都要能解析出 target
  （import 那种依赖 T-D10）。解析不出就**不显示**这一行（不编）。
- **判据**：三种各一条 hover 测试（`crates/lsp/src/tests/hover.rs`）。

### 7.2 D-II 记法解析走闭包

#### T-D10 `NotationDecl` re-export + 补 `span`/`module`

> **✅ 完成（2026-09-21）。** `NotationDecl` 加 `span`（声明那一行的 span，
> 在**声明它的模块**的坐标里）+ `module: Option<String>`（声明它的模块名；
> `None` = 语言内建的记法，不在任何源文本里、无处可跳）。两个构造点都填了：
> `Command::notation_decl()` 填 `span`、`absorb_notations` 填 `module`
> （命令自己不知道自己在哪个模块里，加载层知道）。顺带给 `NotationDecl`/
> `NotationAssoc` 加 `Serialize`/`Deserialize`——`ProjectReport` 要过项目缓存，
> 记法表是报告的一部分，缓存命中时它必须一起回来（否则"跳转"在热路径上会
> 突然失效）。
>
> 判据：`notation::span_tests::a_notation_decl_carries_its_own_span`（span **逐字**
> 等于 `infix:50 " ∈ " => Set.mem` 那一行）+ `builtin_notations_have_no_declaration_site`。

- **改什么**：① `crates/front/src/lib.rs` re-export `NotationDecl`
  （以及需要的话 `parse_with_inherited`）；② `NotationDecl` 增加声明点信息
  （`span` + 所属模块）。**注意它是跨模块传播的纯数据，加字段要考虑
  `project/graph.rs` 的 `absorb_notations`（`:119-134`）与
  `Command::notation_decl()`（`ast.rs:649`）两处构造点。**
- **判据**：单测：`infix:50 " ∈ " => Set.mem` 的 span 逐字等于那一行。

#### T-D11 闭包级记法表进 `ProjectReport`/`QueryDoc`

> **✅ 完成（2026-09-21）。** 加载期算好的入口记法表（`exports` 是局部量、
> 以前算完就丢）现在搬到 `Closure.notations` → `ProjectReport.notations`。
> 继承来的那份**保留它原来的模块名**——那才是它的声明点。
>
> 判据：`crates/front/tests/prelude_shape.rs::the_entry_notation_table_points_at_the_declaring_module`
> ——unit01 的记法表里 `∈` → `Set.mem`、`module = "lib.Set"`，且 `span` 切出来的
> 文本含 `infix:50` 与 `Set.mem`（**声明点在库里，不在入口**）。

- **改什么**：闭包编译时收集每模块的记法声明（符号 → 声明点 + 模块），
  存进报告层（现在 `graph.rs:92` 算了又丢）。
- **判据**：单测：unit01 的记法表里 `∈` 指向 `lib/Set.sokonanoda` 的第 N 行。

#### T-D12 解析 API：`notation_resolve(text, table, offset)`

> **✅ 完成（2026-09-21）——落地成 `QueryDoc::notation_at`。**
>
> 计划想要的是"比 `symbol_at` 的 `(String, Option<String>)` 更完整"的 API。
> 实测发现**分工**更清楚：`notation_input::symbol_at` 是**词法**的（只看本文件 +
> 内建，不依赖 parse 成功），而"声明点"必须来自**闭包记法表**（T-D11）——
> 所以新的入口放在 `QueryDoc` 上：
>
> ```rust
> QueryDoc::notation_at(text, offset) -> Option<(符号, 目标, 模块名, 声明点 span)>
> QueryDoc::module_path(name)        -> Option<PathBuf>   // 模块名 → 文件路径
> ```
>
> `symbol_at` **保留原样**（hover 那条路用它 + `symbol_at_with_sources`）；
> 它不需要"声明点"，硬塞进去只会让词法层背上闭包。
>
> 判据：G-23 的复现件（它同时走 hover 与 definition 两条路）。

- **改什么**：新 API 返回 `{ symbol, target, decl_span, module, fixity, precedence,
  candidates }`（比 `symbol_at` 的 `(String, Option<String>)` 完整）。
  保留 `symbol_at` 给不需要这么重的调用方，或让它变成薄包装。
- **判据**：单测：import 来的 `∈` 解析出 `target = "Set.mem"` + 声明点；
  重载符号返回全部候选。

#### T-D13 hover 的"展开成"（import 来的记法）

> **✅ 完成（2026-09-21）——T-D02 顺带做掉了。**
>
> `symbol_at_with_sources`（闭包前缀回退）让 hover 对 `import` 来的记法也说得出
> "展开成 `Set.mem`" ✓。判据：`crates/lsp/src/tests/hover.rs::hover_on_an_imported_notation_symbol_shows_the_raw_type`
> 断言 `展开成 \`Set.mem\`` 与原始类型那一行。

- **判据**：hover 含 `` 展开成 `Set.mem` ``。

### 7.3 D-III go-to-definition

#### T-D14 parser 保留记法符号 token 的 span

> **✅ 完成（2026-09-21）。AST 变更落地，六处构造点全填。**
>
> `Expr::Notation` 增加 **`symbol_span: Span`**（只覆盖那个符号，不是整段节点）；
> `bump_operator` 改为**返回 `Token`**（以前 `bump()` 的返回值被丢掉）；
> `notation_node` 加参数。填的位置：**infix 族**（用 `op_tok.span`）、**prefix**
> （`tok.span`）、**postfix**（`tok.span`）、**binder**（`tok.span`）、
> **零元**（节点 span 本来就只覆盖符号 ⇒ 两者相同）。
>
> **为什么要这个字段**：节点 span 覆盖整段（`a ∈ A` 三个 token），而编辑器要问的是
> "光标是不是正好压在这个**符号**上"——`notation_at` 以前只能靠**词法重新扫一遍
> 文本**回答；现在 AST 侧直接有答案（表达式内部的记法跳转、以及"点哪一段算点了
> 记法"都不必再扫文本）。
>
> **判据**：`parser::tests::a_notation_nodes_symbol_span_covers_only_the_symbol`
> —— `∈` 的 `symbol_span` 正好 3 个字节、且是节点 span 的**真子区间**。
> 另按本条的"风险"提示，给 `compile::tests::
> notation_records_a_hover_row_covering_the_whole_notation` **补了断言**：
> hover 行覆盖整段与 `symbol_span` 只覆盖符号**不矛盾**——两者回答不同的问题
> （"这段表达式是什么类型" vs "光标是不是压在符号上"）。
>
> **AST 变更的连带面**（编译器全指出来了，逐个改并**每次核对行数**）：
> `by.rs`（恢复）、`compile/elab.rs`（展开路径用不到 ⇒ 模式里写 `..` 并注明）、
> `compile/goals.rs` ×2（重建）、`display.rs` ×2（折叠产物是**渲染产物**、
> 没有源里的符号 token ⇒ 退化成节点 span）、`spine.rs` ×4（beta 归约 /
> 改名 / 替换）。
>
> **⚠ 过程事故（第三次同类）**：这轮的一次 `str.replace` 把 `elab.rs`
> **写少了 4852 行**（`git diff --stat` 立刻暴露）。处置：`git checkout --` 恢复，
> 之后**每处改动都先 `count(old)` 断言唯一、再核对行数增减**。
> 教训写进 `skills/sokonanoda-dev`：**批量文本替换前后必须核对行数**，
> 看到 `git diff --stat` 的删除行数远大于预期就立刻停下。

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

> **✅ 完成（2026-09-21）。记法符号现在是"一等目标"。**
>
> `ResolvedTarget` 增加 **`Notation { symbol, span, module }`**；`elab.rs` 的记法
> 展开路径把 `resolution: None` 换成该变体，`span` 取 **T-D14 的 `symbol_span`**
> （使用处**那个符号自己**，不是整段节点）。
>
> **D5 的坑已确认 + 已加测试**：装配那一步
> （`kernel_phase.rs`）只对 `Declaration` 回填声明 span：
> ```ignore
> if let Some(ResolvedTarget::Declaration { name, .. }) = &node.resolution { … }
> ```
> ⇒ 新变体**天然**不受影响——但"天然"不是判据，所以钉了
> `compile::tests::a_notation_hover_row_keeps_its_notation_resolution_after_assembly`：
> 报告装配**之后**那条记法行的 `resolution` 仍是 `Notation`，且**不等于**
> `Set.mem` 的声明 span。
>
> **两处连带（编译器指出来）**：
> * `render.rs::definition_name_span` —— 记法**不是名字**、没有可改名的 token
>   ⇒ 返回 `None`（`rename` 因此被拒）。**T-D30 那条"记法上 rename 不得改 `h`"
>   现在由变体本身保证**，不再只靠"光标在符号上"的守卫；
> * `lib.rs::definition` 的跨文件分支 —— 记法的跨模块跳转走 T-D10 的记法分支
>   （在 `definition_at` 之前就返回），走到那里的是本文件内的记法 ⇒ `None`。
>
> **与计划措辞的一处偏差（如实记）**：计划写的是"`resolution` 仍是**记法声明点**"，
> 而 elaborator **拿不到记法表**（它在 parser 手里）⇒ 这里填的是**使用处符号自己**
> 的 span，**声明点**由 LSP 侧的闭包记法表解析（`QueryDoc::notation_at`，
> T-D10/T-D16）。判据的**实质**（"没有被改写成 `def Set.mem` 的 span"）一字不差地
> 钉住了；`module` 字段先留 `None`，等 T-D16 需要跨模块时再填。

- **改什么**：`ResolvedTarget`（`crates/front/src/compile/report.rs:128-134`）
  增加 `Notation { symbol, span, module }`；`elab.rs:2914` 把
  `None` 换成该变体（至少对**符号 span 命中**的情形）；
  **确认 `kernel_phase.rs:419-429` 的覆写只作用于 `Declaration` 变体**
  （这是 D5 的坑，必须加测试）。
- **判据**：单测：记法 hover 行的 `resolution` 在报告装配**之后**仍是记法声明点，
  没有被改写成 `def Set.mem` 的 span。

#### T-D16 LSP `definition` 处理记法变体

> **✅ 完成（2026-09-21）。判据两条都实跑过。**
>
> * `bash docs/gaps/repro/G23-notation-navigation.sh` ⇒ **exit 1**（已修），
>   台账里 G-23 是 `fixed` + `行为已变`，`gap.py check` 全绿；
> * `cargo test -p sokonanoda-lsp --lib tests::navigation` ⇒ 绿，
>   含本轮补的 `goto_definition_on_a_notation_symbol_lands_on_its_declaration`
>   （`∈` 跳到**声明它的模块**那一行）。
>
> **实现落在两处**（计划里写的 `module` + `span` → `Location` 由它们完成）：
> 1. **T-D10 的记法分支**：`goto_definition` 在 `definition_at` **之前**先试
>    `query.notation_at(text, offset)`（词法，闭包记法表）⇒
>    `query.module_path(module)` → `Location{uri, range_of(span)}`。
>    这条路覆盖**任意位置**的记法符号（含表达式内部），也是 e2e 用例 #7 走的；
> 2. **T-D15 的 `ResolvedTarget::Notation`**：`definition_at` 那条通用路现在
>    也能拿到记法目标。它的跨文件分支对记法返回 `None`（**有意的**）——
>    跨模块的记法跳转由上面第 1 条负责，走到那里的是**本文件内**声明的记法。
>
> 也就是说：本条**没有新增产品代码**，是 T-D10 + T-D15 合起来交付的；
> 这一轮做的是**把两条判据实跑一遍并钉住**（免得以后有人以为它没做）。

- **改什么**：`crates/lsp/src/lib.rs:1334-1364` 的 definition handler 认识新变体；
  跨模块时用 T-D11 的表把 `module` + `span` 变成 `Location`。
- **判据**：`bash docs/gaps/repro/G23-notation-navigation.sh` → `1`；
  `cargo test -p sokonanoda-lsp navigation -- --nocapture` 新增用例绿。

#### T-D17 hover 的 `range` 收窄到符号本身

> **✅ 完成（2026-09-21）。**
>
> `notation_symbol_hover` 的 `range` 从 `None` 换成
> `notation_input::symbol_span_at(text, offset).map(range_of)`。
>
> **为什么较真**：`None` 时客户端按"光标词"自己高亮——而客户端的分词规则与我们
> 的词法**不是一回事**，`⁻¹'`/`×ˢ` 这种多字符符号、`𝒫` 这种星平面符号都会歪。
>
> **实现要点**：`notation_input` 新增 `symbol_span_at`，并且把 `symbol_at` 与它
> 共用的那一步抽成 `symbol_token_at` ⇒ **"认得出来"与"给出范围"永远一致**，
> 不会两边漂移（这是这一刀最容易写歪的地方）。
>
> **判据**：`crates/lsp/src/tests/hover.rs::
> a_notation_hover_range_covers_only_the_symbol` —— 光标停在 `⁻¹'` 的**中间**
> （第二个字符上，正是"按光标词"最容易歪的位置），断言 range 从符号第一个字符
> 起、**正好覆盖 3 个字符**。
> **踩到的坑**：夹具里 `src.rfind("⁻¹'") + 1` 会切在 `⁻`（3 字节）**中间**，
> `lsp_pos` 当场 panic ⇒ 必须落在**字符边界**上（`+ "⁻".len()`）。
>
> **回归**：LSP 全量 **158 通过 / 0 失败**；真宿主 e2e
> `--grep "notation symbol"` ⇒ **2 passed / 0 failed**（台账已记）。

- **改什么**：现在 `notation_symbol_hover` 返回 `range: None`
  （`lib.rs:854-856`）⇒ 客户端按"光标词"高亮。有了 `symbol_span` 后可以给精确 range。
- **判据**：hover 返回的 range 只覆盖符号（测试断言）。

### 7.4 D-IV 边界与一致性

#### T-D20 内建/ prelude 目标的定义跳转怎么办

> **✅ 完成（2026-09-24）**：决定 = **`definition` 返回 `null`，hover 说明原因**
> （「内建记法（内核 prelude）：没有源码声明，`F12` 无处可跳」）。
> 判据：`hover_on_a_builtin_notation_symbol_shows_the_raw_type` 里加了这一行断言；
> `cargo test -p sokonanoda-lsp --lib` **158 通过**。设计决定写进
> `docs/design/notation-subset.md`（含"内建在闭包表里查不到 ⇒ 判据不能写成
> `module.is_none()`"这个坑）。

- **问题**：`∧`→`And`、`=`→`Eq` 是内核 prelude 名，**没有源码声明**
  （`top_level_def_spans` 按构造排除 prelude，`check/mod.rs:668`）。
- **改什么**：决定并落地：跳到记法声明点（如果有）/ 返回 `null` / 跳到别的地方。
- **判据**：决定写进设计 + 测试。

#### T-D21 跨文件记法的"定义"是哪个

> **✅ 完成（2026-09-24）**：决定 = **库的那一行 `infix`**（声明点唯一；
> `import` 只说明传播）。**实现早就在**（T-D12/T-D13 的 `notation_at` 给出
> 模块 + 那一行的 span），**测试也早就有**：
> `goto_definition_on_a_notation_symbol_lands_on_its_declaration` 建真临时项目
> （`SetLib` 声明 `∈` + `Canvas` import）并断言 `uri == lib_uri`。
> 本轮把**决定**补写进 `docs/design/notation-subset.md`（这才是这条清单项缺的东西）。

- **问题**：`∈` 声明在 `lib/Set`，用在 unit。定义 = 库的 `infix` 行，
  还是入口的 `import` 行？（设计 §10.3 只说记法随 import 传播，没说定义在哪。）
- **改什么**：决定（推荐库的 `infix` 行）+ 测试。

#### T-D22 `scoped` 记法的导航是否尊重作用域

> **✅ 完成（2026-09-24）**：决定 = **导航跟作用域（闭包表 + 本文件 + 内建），
> 输入提示刻意不跟**（输入法是全局的）——偏差是**有意的**，写进
> `docs/design/notation-subset.md`。判据：
> `a_notation_symbol_out_of_scope_is_not_resolved`（真临时项目：`SetLib` 声明 `∈`、
> 入口不 import ⇒ `definition` 必须 `null`）。实跑 **3 passed**。

- **问题**：`notation_input` 刻意忽略 scoping（`docs/design/notation-input.md` §10 偏差 3）
  ⇒ 不在作用域的符号也会被当成在作用域。导航要不要跟？
- **改什么**：决定 + 测试。

#### T-D23 重载：一个 `Location` 还是 N 个

> **✅ 完成（2026-09-24）**：决定 = **一个** `Location`，取闭包记法表里**第一个**
> （与展示层折叠规则同一条约定）。**顺带挖出并修掉真 bug G-39**：
> `notation_at` 原来用只看"输入表 + 本文件声明"的 `symbol_at` ⇒ **import 进来的
> 用户自定义符号**（`⊗`）在使用处认不出来、导航全 `null`；
> 既有跨文件用例用的 `∈` 恰好在输入表里，把这条路遮住了。
> 修：改用闭包感知的 `symbol_at_with_sources`。判据：单元测试
> `an_imported_user_notation_symbol_resolves_into_its_module`（**必须用 URI 感知的
> `did_open_at`**，单文件版没有项目闭包）+ 真 LSP 探针
> `docs/gaps/repro/G39-…js`（⇒ exit 1）。LSP **160 通过**。

- **改什么**：决定 + 测试（LSP 的 `definition` 可以返回数组）。

#### T-D24 `documentHighlight`/`references`/`rename` 覆盖记法符号

> **✅ 完成（2026-09-24）**：**做** `documentHighlight`（文件内）—— 符号上给出
> **它自己的每一处**（前端 `notation_input::symbol_occurrences`，词法、非子串匹配）；
> **不做** `references`（项目级扫描与"性能是生命线"冲突；文件内需求已被
> `documentHighlight` 覆盖）与 `rename`（符号是源级糖、可能多模块各声明一次；
> 今天**明确拒绝** `InvalidParams`，安全且诚实）。决定与理由写进
> `docs/design/notation-subset.md`。**T-D30 的意图一条没破**：断言从
> "符号上必须为空"改成语义化的"每段点亮的文本必须就是符号本身"。
> 实跑：`cargo test -p sokonanoda-lsp --lib` **160 通过**。

- **改什么**：实现或明确写"不做"（给出理由）。
- **判据**：决定写进 `docs/design/notation-subset.md`；若做，加测试。

#### T-D30 记法符号不再误解析到外层 binder（**独立正确性 bug**）

> **✅ 完成（2026-09-21）——两处回退都加了守卫，含一条"别修坏定义点"的对照。**
>
> **病**：`render.rs::highlight_uses`（LSP `documentHighlight`）与
> `references::resolve_at`（front，`rename`/`references` 共用）都有"回退到包含光标
> 的任意 hover 行"的逻辑，而 **binder 的 span 覆盖整段类型标注**（`(h : a ∈ A)`）
> ⇒ 光标在 `∈` 上会被解析成外层 binder `h`，`rename` 会去改 `h`。
> （`definition` 反而因为**没有**回退而返回 `null`——四个 handler 行为不一致，是
> 构造性的。）
>
> **修法**：两处都加同一条守卫——**光标落在记法符号上就直接答"没有名字"**
> （判据走词法 `notation_input::symbol_at`：本文件声明的 / 内建的 / 输入法表里的
> 符号都认得出来）。`resolve_at` 因此多一个 `doc: &str` 参数（它没有文本就判不了）。
>
> **判据**（两侧都有，且都带**对照**）：
> * front：`references::tests::resolve_at_does_not_mistake_a_notation_symbol_for_the_enclosing_binder`
>   —— `∈` 上答 `None`，**而同一个 binder 的 `h` 本身仍解析得到**（别把定义点
>   那一支修坏）；
> * LSP：`tests::navigation::a_notation_symbol_does_not_resolve_to_the_enclosing_binder`
>   —— `documentHighlight` 在 `∈` 上返回空、`rename` 被拒（`没有可以改名的名字`）。
>
> **踩到的坑（值得记）**：`character` 是**字符**计数（LSP 口径），而 Rust 的
> `str::find` 给的是**字节**下标——行里有 `α`/`∈`，两者不等。我第一版 `offset_of`
> 按字节算 ⇒ offset 落到 `) :` 上 ⇒ 守卫不触发、测试照红。
> **星平面符号的偏差是 T-D31 的独立缺口**（LSP 要的是 UTF-16 码元），两侧一起改。

- **根因**：§2.7 第 1 条。
- **改什么**：`render.rs:327-331` 与 `references.rs:57-60` 的"包含光标即命中"回退
  必须排除"光标落在记法符号上"的情形（或改成"命中最近的最小 span"）。
- **判据**：单测：光标在 `(h : a ∈ A)` 的 `∈` 上时
  `documentHighlight`/`references`/`rename` **不得**返回 `h`；
  `rename` 不得改 `h`。
- **风险**：这条修的是**现有错误行为**，可能让某些 rename 用例变红——那正是要找的。

#### T-D31 `position_to_offset` 的 UTF-16 语义（**独立缺口**）

> **✅ 完成（2026-09-21，按计划只做"立台账 + 判定实验"，不做修复）。**
>
> * **台账**：`docs/gaps/ledger.jsonl` 新增 **G-36**（`status: open`，
>   `repro_expect: script` ⇒ 复现件 exit 0 = 缺口仍在）；`gap.py check` 全绿。
> * **判定实验**：`docs/gaps/repro/G36-utf16-position-mapping.sh`（+ `.js`）——
>   起**真 LSP**、说 JSON-RPC。夹具
>   `theorem demo (α : Type) (A : Set α) (h : 𝒫 A = 𝒫 A) : 𝒫 A = 𝒫 A := h`
>   （`soko grade` 退出 0、零诊断）里 `𝒫 A` 的 `A` 在 **UTF-16 列 44**：
>   服务端给的 hover 是**外层表达式** `𝒫 A = 𝒫 A : Prop`，而**列 43** 的 hover
>   才是 `A : Set α` ⇒ 差的就是那 1 个码元。
> * **两条纪律（都踩过）**：① 夹具必须**编译干净**，否则 hover 退化成"未通过，
>   见诊断"的错误卡片，红的原因就不是位置映射了；② 判据**不能只看 range**
>   （落偏时服务端回退成"整行表达式"，range 照样覆盖光标 ⇒ 恒真、量不出缺口），
>   要看**文本**，并且**必须带对照**（列号减一 ⇒ 命中 `A`），否则排除不掉
>   "`A` 本来就没有 hover"。形状不对 ⇒ exit 2。
> * **为什么仓库内测试抓不到**：`crates/lsp/src/testutil.rs` 的 `lsp_pos` 刻意
>   **镜像**了服务端的数法 ⇒ 单测里两边一起偏、永远一致。只有像本实验这样走
>   **真 JSON-RPC** 才看得见。
> * **真修单独立项**：改成 UTF-16 计数会动**所有**位置映射与夹具
>   （含 front 的 `references::offset_of`，T-D30 里有意与它同口径），要一起改。

- **根因**：§2.7 第 3 条（`crates/lsp/src/lib.rs:615-629`；`docs/protocol.md:806-811`
  已记为独立缺口）。
- **改什么**：本轮**只立台账 + 写判定实验**（用 `𝒫` 造一个偏离用例）；
  真正修（改成 UTF-16 计数）单独立项，因为它会动所有位置映射与测试夹具。
- **判据**：`docs/gaps/ledger.jsonl` 新增一条；实验脚本能稳定复现偏移。

### 7.5 D-V 三层测试与文档

#### T-D40 三层测试（矩阵用例 #7/#8）

> **✅ 完成（2026-09-21）。三层齐，且每一层都有"前提守卫"。**
>
> | 层 | 用例 | 钉住什么 |
> |---|---|---|
> | **front** | `query::tests::notation_folding_does_not_clobber_a_use_points_resolution` | 线 C 的折叠只动**显示副本**，点名使用点的 `resolution` 仍在；**前提守卫**：这条夹具确实折了记法（`h.ty` 含 `⊆`），否则断言证明不了什么 |
> | **LSP** | `hover_on_{a_locally_declared,a_builtin,an_imported}_notation_symbol_*`（三条）· `goto_definition_on_a_notation_symbol_lands_on_its_declaration` · `a_notation_symbol_does_not_resolve_to_the_enclosing_binder` | hover 的三种形态 + 跳转到**声明它的模块**那一行 + highlight/rename 不误命中 binder |
> | **真宿主 e2e** | 矩阵 #7 `go to definition on a notation symbol lands on its declaration` · #8 `hover on a notation symbol shows the target's signature` | 真 VS Code 的 `executeDefinitionProvider`/`executeHoverProvider` |
>
> **判据实跑**（T-D40 验收这一轮）：
> ```
> SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh --grep "notation symbol" --profile debug
>   ⇒ e2e: 2 passed / 0 failed（46s；台账进 docs/e2e/ledger.jsonl）
> ```
> **LSP 的 definition 用例是这轮补的**（T-D10 的记法分支此前只有 e2e 覆盖，
> 单元层缺一条）；**front 那条也是这轮补的**——原来三层里 front 层是空的。
>
> **一处如实记录**：front 那条最初想断言"记法符号自己的 hover 行没有
> `resolution`"，实测**这个夹具里没有正好落在 `⊆` 上的 hover 行**（hover 行按
> 表达式/名字给），所以那半条是**空断言**——删掉，换成"前提守卫 + resolution 仍在"
> 两条真能失败的前提。**不凑数**。

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

> **✅ 完成（2026-09-21），并 ⬆ BUMP patch → 0.65.3。**
>
> | 文档 | 改了什么 |
> |---|---|
> | `docs/design/notation-subset.md` | §4 的 hover 行不再说"`resolution` v1 留空"；**新增 §4.1「编辑器支持（as-built）」**——五条能力（跳转 / hover 展开成 / hover 精确范围 / highlight+rename / `resolution` 不被覆写）各配**判据** |
> | `docs/TESTING.md` | 守护表新增一行「记法编辑器导航（0.65.3；线 D 收口，缺口 G-23 关账）」：六条契约 + 全部用例名 + 两条可跑命令 |
> | `editor/vscode/README.md` | 「Notation input」那条升级为**两条**：输入 + **可导航**（hover 三件事、框住符号本身、F12 跨 `import` 跳转） |
> | `editor/vscode/CHANGELOG.md` | 0.65.3 条目（用户可感知：hover 精确范围；内部：记法是一等目标） |
> | `docs/protocol.md` | 位置一节点名 **G-36**（UTF-16 缺口 + 复现件）；上一轮已随 T-D31 落 |
>
> **判据**：`cargo test -p sokonanoda-cli --test skill` ⇒ **4 passed**；
> 完整 `gate` ⇒ PASS（课程计数逐项不变）。
> **bump 闭环**：推 main → CI 绿 → auto-tag → release → `gh release list` 核对
> （REQUIREMENTS §9 的硬要求）。

- **改什么**：`docs/design/notation-subset.md` 的"编辑器支持"段 as-built
  （更新 §4/§13 与 `:208` 的"留空"话术）；`docs/protocol.md`（若新增请求/字段）；
  `docs/TESTING.md` 的守护表；`editor/vscode/README.md` + `CHANGELOG.md`；
  `skills/` 三个技能同轮。
- **判据**：`cargo test -p sokonanoda-cli --test skill` 绿。

### 7.6 用户第 7 条反馈（2026-09-23 现场取证后新增）

> **机制已查明（两条都查到根因，不是猜的）。** 现场文件
> `courses/set-theory/lib/Set.sokonanoda`（记法声明在 115–128 行）。
>
> #### 机制 A：声明栏的 `forall` = **折叠层只认 infix 族**
>
> `crates/front/src/display.rs::fold_spine` 里明写：
> ```ignore
> // **第一刀只做二元 infix 族**（Infix/Infixl/Infixr）——一元前缀/后缀与
> // binder 记法的折叠留给后续环节
> if !matches!(decl.assoc, NotationAssoc::Infix | Infixl | Infixr) { return None; }
> ```
> **实测**（`query goals`）：`t_infix` 的 `ty` =
> `forall (α : Type 0) (a : α) (A : Set α), a ∈ A -> a ∈ A`
> —— `∈` **折了**、`forall` **没折**。⇒ 用户说的"**丢了一批符号**"成立：
> **prefix（`𝒫`）、postfix（`ᶜ`）、binder（`∀`/`∃`）、零元（`∅`）全都漏折**。
>
> **而且是三层叠加**（只改过滤器不够——这是查完才敢下的结论）：
> 1. `fold_spine` 只放行 infix 族（上面那段代码）；
> 2. **`∀`/`∃` 根本不在内建记法表里**（`BUILTIN_NOTATIONS` 只有
>    `∧ ∨ ↔ ¬ = ≠` 等）——它们是 **parser 级 binder 语法**，不在表里就永远
>    折不出来；
> 3. 声明栏那个 `forall` 是**内核 pp 打的 telescope**
>    （`forall (α : Type 0) (a : α) …,`），**不是源码里的 `∀`** ⇒ 折叠要认的是
>    `forall (x : T), body` 这个**形状**，而不是"查表找符号"。
>    （回读通道 `render_expr` 正是把内核 pp 重新 parse 回来 ⇒ 它给出的
>    `forall` 落在 AST 的哪个节点上，是 T-D51 第一件要查清的事。）
>
> #### 机制 B：记法声明行的目标名**从来不是使用点**
>
> 真 LSP 探针（`textDocument/{definition,hover,documentHighlight}`）打在
> 115–128 行每一条的**目标名**上：**五条全是 `null`** ——
> `Set.mem` / `Set.powerset` / `Set.compl` / `Set.image` / `Set.preimage` /
> `Set.prod` 无一例外。⇒ **ctrl+点击不能跳转**是**共性问题**，与符号种类无关。
>
> **为什么"高亮"只坏了三条**（用户观察到的差异）——语义 token 的**类型号**不同：
>
> | 行 | 目标名 | token 类型 | 说明 |
> |---|---|---|---|
> | 115–125 | `Set.mem` / `Set.subset` / `Set.union` / … / `Set.powerset` / `Set.compl` | **4 = `FUNCTION`** | 名字**在本文件里声明** ⇒ 作用域查得到 |
> | 126–128 | `Set.image` / `Set.preimage` / `Set.prod` | **5 = `VARIABLE`** | 名字**不在本文件作用域**（在 `lib/Image.sokonanoda` / 单元⑤ 的画布里）⇒ 落成 `SemanticKind::UnknownIdent` |
>
> ⇒ 三条"没高亮"的**直接原因**是它们被当成**未知标识符**（`variable.other`），
> 而不是"记法目标"。**根因仍是机制 B**：目标名不是一个**已知引用**，
> 于是只能退回作用域查找；查不到就只好说"不知道这是什么"。

#### T-D50 记法声明的**目标名**是使用点（**修机制 B，一条修两个症状**）

> **✅ 完成（2026-09-24）。判据实跑：G-37 复现件 ⇒ exit 1。**
>
> **两处实现**：
> 1. **着色**（`semantic::tag_runs_with_notations`）：把记法声明的**目标名**
>    按**已知引用**登记（判据走词法 `scan_notation_decls`，与 parser 同源），
>    并用 `or_insert` —— 名字**真在本文件里**时保留它**真实**的种类。
>    ⇒ 不再落 `UnknownIdent`（`variable.other`），用户看到的"三条没高亮"消失。
> 2. **跳转 + hover**（LSP）：新增词法助手
>    `notation_input::notation_target_at`（找记法命令关键字 → 跳过符号字符串 →
>    取**第一个标识符** = 目标名；不需要认 `=>` 这个 token，词法里它是 `=` + `>`），
>     `definition` 与 `hover` 各加一条分支，走**同一条闭包通道**
>    （`project_definition`）。
>
> **判据实跑**（真 LSP，`lib/Set.sokonanoda` 124–128 行）：
> * **hover 五条全部答得上**（"`Set.image` —— 记法的目标"）；
> * **definition 在闭包里的两条**（`Set.powerset`/`Set.compl`）**跳转成功**；
> * **不在闭包里的三条**（`Set.image`/`Set.preimage`/`Set.prod`，声明在别的模块）
>   **诚实为 null** —— 这份文件没 import 声明它们的模块，没有可跳的目标
>   （计划原文的边界：`resolution` 诚实为 `None`，**但着色必须仍是已知引用** ✓）。
> * `documentHighlight` 本轮**不做**（要 use→def 映射，属于另一条线）。
> * front 判据：`a_notation_target_is_a_known_reference_not_an_unknown_ident`
>   （含"真未知标识符仍是 `UnknownIdent`"的对照）。
>
> **⚠ 这条缺口的第一次取证（0.65.2）是错的（如实记）**：复现件用
> `positionOf(SRC, "Set.powerset")` 取**第一次出现**——它在文件更早的**注释**里
> ⇒ 光标一直落在**注释**上，三种请求当然全 null。修法是"行首偏移 + 行内偏移"。
> **教训：夹具位置必须定位到"那一行里的那个 token"**，用全局第一次出现是最容易
> 骗过自己的写法（与 G-36 的"因为错的原因为真"是同一类）。

- **改什么**：解析/编译记法声明时，把 `=>` 后面的**目标名 token** 登记成
  **使用点**：① 语义分类给**已知引用**（`FUNCTION` 族，不是 `UnknownIdent`）；
  ② 记录 `resolution`（目标在闭包里就指向它的声明；不在就**仍然着色为已知引用**，
  因为**记法声明就是这个名字作为词汇的引入处**——课程注释原话：
  "`Set.prod` 是**单元⑤ 给出的词汇**"）；③ hover 说清"这是记法 `''` 的目标"；
  ④ `definition` 走闭包跳到声明（跨模块，复用 T-D10/T-D11 的表）。
- **判据**：
  1. 单测（front `semantic`）：记法声明行的目标名 token 的 kind **不是**
     `UnknownIdent`；
  2. LSP：`definition` 打在 `infix:50 " ∈ " => Set.mem` 的 `Set.mem` 上返回非空；
  3. **e2e**（真宿主，矩阵新用例 #9）：在 `lib/Set.sokonanoda` 的
     `Set.image` 上 `vscode.executeDefinitionProvider` 非空 + 该处语义 token
     不是 `variable`。
- **风险**：`Set.image`/`Set.preimage`/`Set.prod` **在本文件里根本不在作用域**
  ⇒ 它们的 `resolution` 只能"诚实地说找不到声明"（`None`），**但着色必须仍按
  "已知引用"**——否则用户看到的就是今天这个"没高亮"。这条要在测试里钉住，
  免得下一刀又把"查不到声明"退化成"未知标识符"。

#### T-D51 折叠层扩到 prefix / postfix / binder / 零元 + 给 `∀`/`∃` 补表（**修机制 A**）

> **✅ 完成（2026-09-24）。判据实跑：G-38 复现件 ⇒ exit 1。**
>
> **两层修法**：
> 1. **四种记法全折**：`fold_spine` 去掉"只放行 infix 族"的限制，按每种记法
>    自己的**操作数位**折——Infix 族 2 个（左、右）· Prefix/Binder 1 个（在**右**）·
>    Postfix 1 个（在**左**）· 零元 0 个（照 parser 的 `notation_node` 构造形状）。
> 2. **`forall` → `∀`**：`∀` 是 parser **关键字**、不在 `BUILTIN_NOTATIONS` 里，
>    而声明栏那个 `forall` 是**内核 pp 打的 telescope** ⇒ 只能**认形状**
>    （`Expr::Forall`）。**只替换关键字那 6 个字节**（顶层时逐字节保真），
>    并**同时**返回折好的记法节点（外层 App 重渲染时也带 `∀`）。
>    顺带：binder 渲染**带类型**（以前多 binder 只打名字 ⇒ 折了反而丢信息）。
>
> **判据**：`docs/gaps/repro/G38-folding-only-infix.sh` ⇒ **exit 1**；
> `ty` = `∀ (α : Type 0) (a : α) (A : Set α), a ∈ A -> a ∈ A`；
> 测试从 `only_binary_infix_folds_and_the_rest_fall_back` 改写成
> **`every_notation_kind_folds`**（五条"点名 → 记法" + 元数不匹配回退）；
> `cargo test --workspace` **exit 0**（40 个 suite）。
>
> **两个坑（都写进代码注释）**：
> * 第一版把整个 `Forall` **重渲染** ⇒ binder 分组被拆（`(A B : Set α)` →
>   `(A : Set α) (B : Set α)`）、`Type 0` 变 `Sort 1`——**信息失真**，
>   被 `only_the_folded_spans_change` 逮住 ⇒ 改成**关键字级替换**；
> * parser 把 `(x : α) -> …` **也**解析成 `Expr::Forall`（匿名 binder）⇒ 只看 AST
>   会在 `(x : α` 那 6 个字节上写 `∀`、括号配不平 ⇒ `splice` **整体放弃**、
>   展示副本退回**完全不折**。判据必须是**源文本**，为此把 `src`/`base` 传进折叠层。
>
> **⚠ 开发期的坑（值得记）**：改完折叠后 `query_state_agrees_with_the_lsp_state_at_request`
> 红了一条（CLI 给 `∀`、LSP 给 `forall`）——**不是代码不一致**，而是 **LSP 的编译
> 缓存**里还存着改动前的报告（**版本号没变 ⇒ 缓存键没变**）。
> 用 `SOKONANODA_CACHE_DIR=$(mktemp -d)` 一跑就绿。⇒ **开发期验证一律用全新缓存
> 目录**，别被陈旧报告骗。

- **改什么**：`display.rs::fold_spine` 去掉"只放行 infix 族"的限制，按**每种记法
  自己的形状**折：一元 prefix（`𝒫 A` ⇒ 操作数 1 个、在**右**）、一元 postfix
  （`Aᶜ` ⇒ 操作数 1 个、在**左**）、binder（`∀ x, p x` ⇒ 操作数 2 个、第 2 个是
  lambda）、零元（`∅` ⇒ 无操作数，`span` 就等于符号）。四种的 `arity`/操作数位
  都不同，`fold_spine` 现在硬编码 `args[arity - 2..]` 只对 infix 成立。
- **判据**：单测四组（每种记法一条）：声明栏的 `ty` 里出现 `∀` / `𝒫` / `ᶜ` / `∅`；
  **且** `print_back` 的**回读**仍能解析（折过的文本必须还能被 parser 吃回去——
  T-C20 那条 `folded_text_reparses` 的守护要扩到这四种）。
- **风险**：`render_expr` 是**回读通道**的输入 ⇒ 折出来的文本必须**可解析**且
  **语义等价**；binder 记法的回读最容易踩坑（`∀ x, p x` vs `forall x, p x`）。
  另：**判负/事件计数不得变化**（折叠只动显示副本，红线）。

#### T-D52 `def` 的声明多一行"真正定义"（`:=` 之后的东西）

> 用户第 8 条（2026-09-23）原话：「def 的符号，再声明里要多一行内容，对应它们的
> `:=` 之后的那个真正定义，只是它们的类型已经提供不了足够的信息了。比如
> `Set.mem` 的类型完全看不出它的本质是什么」。

> **🚧 数据层已完成（2026-09-24），编辑器那一行待做。**
>
> 已完成：内核 `Declar::value()`（纯读访问器 ✓ 不碰判定）· front
> `DeclState.val_text`（与 `ty_text` 同一形状：`pp_expr` + 线 C 折叠）·
> `DeclInfo.value`/`value_runs` 透出（`query goals` / `soko/goals` 都带）·
> 判据 `query::tests::a_def_carries_its_value_but_a_theorem_does_not`。
>
> **实测**（`query goals`）：`Set.mem` 的 `value` =
> `fun (α : Type 0) (a : α) (A : Set α) => A a` ✓（用户举的那个例子）；
> `Set` = `fun (α : Type 0) => α -> Prop`；`axiom`/`theorem` = `None` ✓。
>
> **性能（计划要求"必须先量"）**：冷缓存 A/B（`grade courses/set-theory/lib/
> Set.sokonanoda`，各 3 轮）旧（0.65.3）**1.74 / 1.70 / 1.51s**、新（0.65.4）
> **1.91 / 1.66 / 1.61s** ⇒ **中位数 1.70 → 1.66s，无退化**（在噪声内）。
> 报告的每次构建多算一遍 `pp_expr(value)`，在这份"很多 def"的文件上量不出成本。
>
> **编辑器那一行也做完了**：Infoview 声明卡片的类型行下面多一行
> `:= <值>`（`media/infoview.js` 的 `renderDecls` + `infoview.css` 的
> `.decl-val-line`/`.decl-val-label`/`.decl-val`——值比类型**亮一档**，因为用户
> 要它正是"类型看不出本质"）；树里的声明条目把值放进 **tooltip**
> （树的行高固定，多一行会打散"一行一声明"的节奏）。
> 客户端契约：`soko/goals` 的每个声明多 `value` / `value_runs` 两个字段
> （`docs/protocol.md` 同步）。stub 宿主 **34/34** ✓。

- **为什么**：`Set.mem` 的类型是
  `forall (α : Type 0), α -> Set α -> Prop` —— 看了**不知道它是什么**；
  而它的 `:=` 之后是 `fun (α : Type 0) (a : α) (A : Set α) => A a`
  （"这个元素属不属于这个集合"），一眼就懂。**类型不是定义**。
- **机制（已查清，数据都在手边）**：
  * 内核**手里就有** value：`Declar::Definition { info, val, hint }`
    （`crates/kernel/src/env.rs:58`）——但 `Declar::info()`（`:174`）只暴露
    `name/uparams/ty`，**没有 value 的访问器** ⇒ 补一个
    `Declar::value() -> Option<ExprPtr<'a>>`（`Definition`/`Opaque` 有、
    其余 `None`）。**纯访问器，不碰判定**（红线不变）。
  * front 侧 `ty_text` 的算法就在
    `crates/front/src/compile/check/kernel_phase.rs:222`：
    `tc.with_pp(|pp| pp.pp_expr(ty))` + 线 C 折叠 ⇒ `val_text` 用**同一个
    形状**算一遍 `pp_expr(val)` 即可。
  * `DeclState`（`report.rs:77`）加 `val_text: Option<String>`；
    `QueryDoc`/`soko/goals` 透出 `value`；Infoview 的"声明"栏在类型下多一行。
- **判据**：
  1. front 单测：`def Set.mem … := A a` 的 `val_text` 含 `fun` 与 `A a`，
     且 `theorem`/`axiom`/`inductive` 的 `val_text` 是 `None`；
  2. e2e（真宿主，矩阵新用例 #10）：`Set.mem` 那一行的面板里出现第二行
     （`:=` / `fun … => A a`）；
  3. **判负/事件计数逐字节不变**（纯显示字段，红线）。
- **风险 / 性能（生命线）**：
  * **报告是每次编译都构建的** ⇒ 给每个 `def` 多算一次 `pp_expr` 是**新增成本**。
    必须先量（`perf_course` / `perf_project` 的前后对比 + 记
    `docs/perf/ledger.jsonl`），**不能默认"pp 很便宜"**；超预算就改成
    **惰性**（只在客户端真的要那一行时算，例如放进 `soko/goals` 的
    `--with-values` 或 hover 通道）。
  * body 可能很长（`by` 块、`match`）⇒ 面板只显示**一行/截断**，
    完整值留给 hover；
  * `theorem` 的证明**不显示**（用户要的是 `def` 的"本质"；证明是另一件事）。

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
                                │                                     ├─ T-K22（K1-d，✅ 局部书写类型，G-34）
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
- [x] `T-A25` `build <目录>` 的 O(文件数 × 闭包) 如实记账
- [x] `T-A01` 项目缓存下沉到 `front`
- [x] `T-A02` 缓存键去掉"可执行文件 mtime"这个不稳定的量
- [x] `T-A03` 缓存条目 v2：按模块存（对齐设计 §4.8）
- [x] `T-A04` 冷/热 `--json` 逐字节一致（带 `sorry` 的项目）
- [x] `T-A05` `requires` 漂移不再静默关掉缓存 + 两条写缓存路径规则一致
- [x] `T-A08` `requires` 的单一来源 + 漂移门禁（**用户判定的根因**）
- [x] `T-A06` 依赖改动仍必 miss
- [x] `T-A07` `--text` 中间态的处置
- [x] `T-A10` LSP 读项目缓存（命中即回放）
- [x] `T-A11` LSP 写项目缓存
- [x] `T-A12` LSP 侧测试：两次独立进程打开，诊断逐字节一致
- [x] `T-A13` 跨文件能力在缓存命中后仍然工作
- [x] `T-A14` 实测数字
  - ⬆ **BUMP**：`minor` —— LSP 接入项目缓存：重启编辑器后重开同一文件不再等 1.8–8.5s
- [x] `T-A15` 命中缓存后 Session 快照的处置
- [x] `T-A23` 扇出：改一个依赖不重编所有打开文档
- [x] `T-A30` 编译不再独占 `Mutex<Docs>`
- [x] `T-A60` 缓存与扇出的 e2e 断言
- [x] `T-A50` 设计文档 as-built
- [x] `T-A51` 性能台账收口
  - ⬆ **BUMP**：`patch` —— 批次 2 收尾（性能台账与文档）
- [x] `T-A52` （可选，用户拍板）打开工作区时预热缓存

#### 批次 3 · 线 C：goal 用记法（minor）

- [x] `T-K32` `pretty_printer.rs` 零单测
- [x] `T-C01` 四个生产者 × 真实文件的实测表
- [x] `T-C02` 消费者审计（**不然会静默改坏判卷**）
- [x] `T-C03` 设计文档：**把已有的 `printback-feasibility.md` §4 升格为权威设计**
- [x] `T-C03b` `DisplayText` 护栏（**先于任何折叠代码**）
- [x] `T-C04` 记法表复用 `judge.rs` 的重建（提成公共函数）
- [x] `T-C10` 折叠函数第一刀：只做二元 infix 族
- [x] `T-C11` arity 的来源
- [x] `T-C12` `scoped` 的保真度
- [x] `T-C13` 重载（一个符号 → N 个目标）的处置
- [x] `T-C14` 折叠层的损失护栏
- [x] `T-C20` 生产者 1+3：根状态与声明列表的 `ty_text`
- [x] `T-C21` 生产者 2：无 `by` 的开练习（应当已经好，补守护）
- [x] `T-C22` 生产者 4：`by` 步进里被 pp 化的四处
- [x] `T-C23` binder ty（假设行的类型）
- [x] `T-C24` 逐 surface 的判别性测试
  - ⬆ **BUMP**：`minor` —— goal / 假设 / 声明类型第一次显示记法
- [x] `T-C25` 边界：命中不了就回退
- [x] `T-C30` `semantic::tag_runs` 填 `Names::notations`
- [x] `T-C31` 目标文本里的**导入名**不再标 `unknown_ident`
- [x] `T-C32` 着色在 Infoview 里可见
- [x] `T-C50` 真宿主 e2e：goal 文本用记法（矩阵用例 #6）
- [x] `T-C40` 断言与 golden 更新（**计数中性**）
- [x] `T-C41` 文档 + CHANGELOG
  - ⬆ **BUMP**：`patch` —— 批次 3 收尾（着色 + golden 重钉）

#### 批次 4 · 线 D：记法跳转 + hover（minor）

- [x] `T-K20` 设计文档 `docs/design/closure-incremental.md` + spike
      （**⬆ 提前**：2026-09-21 用户拍板"先收完线 C 的 4 条，再插 T-K20′"——依据是 unit12 冷编译 9.8s、其中一部分是 G-31/G-34）
- [x] `T-D01` 复现脚本
- [x] `T-D02` hover 增加"原始类型"行
- [x] `T-D03` hover 的"原始类型"只对**能解析出 target** 的符号显示
  - ⬆ **BUMP**：`patch` —— hover 显示记法的原始类型（全计划最便宜的一刀）
- [x] `T-D30` 记法符号不再误解析到外层 binder（**独立正确性 bug**）
- [x] `T-D31` `position_to_offset` 的 UTF-16 语义（**独立缺口**）
- [x] `T-D10` `NotationDecl` re-export + 补 `span`/`module`
- [x] `T-D11` 闭包级记法表进 `ProjectReport`/`QueryDoc`
- [x] `T-D12` 解析 API：`notation_resolve(text, table, offset)`
- [x] `T-D13` hover 的"展开成"（import 来的记法）
- [x] `T-D14` parser 保留记法符号 token 的 span
- [x] `T-D15` 新增 `ResolvedTarget` 变体并绕开覆写
- [x] `T-D16` LSP `definition` 处理记法变体
- [x] `T-D17` hover 的 `range` 收窄到符号本身
  - ⬆ **BUMP**：`minor` —— F12 在记法符号上能跳到声明
- [x] `T-D20` 内建/ prelude 目标的定义跳转怎么办
- [x] `T-D21` 跨文件记法的"定义"是哪个
- [x] `T-D22` `scoped` 记法的导航是否尊重作用域
- [x] `T-D23` 重载：一个 `Location` 还是 N 个
- [x] `T-D24` `documentHighlight`/`references`/`rename` 覆盖记法符号
- [x] `T-D40` 三层测试（矩阵用例 #7/#8）
- [x] `T-D41` 文档同步
- [x] `T-D50` 记法声明的**目标名**是使用点（着色 + 跳转，一条修两个症状）
- [x] `T-D51` 折叠层扩到 prefix / postfix / binder / 零元（`forall` → `∀` 等一批符号）
- [x] `T-D52` `def` 的声明多一行"真正定义"（`:=` 之后的 body）
  - ⬆ **BUMP**：`patch` —— 批次 4 收尾（含 rename/highlight 不再误伤 binder）

#### 批次 5 · 线 K：内核提速（minor）

- [x] `T-K01` 全语料逐字节对拍工具
- [x] `T-K02` 内核改动的验收清单 + **修语料对拍的收集逻辑**
- [x] `T-K03` `36.1s` 花在哪：分阶段 profile
- [x] `T-K10` 设计文档 `docs/design/by-prefix-reuse.md` + 三个候选的定稿
- [ ] `T-K11` **K1-a：纯 front 的 judge TrustPlan 复用（零内核改动，先做）**
  - ⬆ **BUMP**：`minor` —— by 判定的内核检查那一段拿掉（K1-a）
- [ ] `T-K12` **K1-b：`EnvBuilder::with_env`（单次内核改动里性价比最高）**
  - ⬆ **BUMP**：`minor` —— 36.1s → 个位数秒（K1-b，本轮最大的一刀）
- [ ] `T-K13` **K1-c（备选）：`EnvBuilder::snapshot()` 克隆式检查点**
- [x] `T-K22` **K1-d：记法消解的类型查询走局部书写类型（G-34；本刀是缓解，根治在 T-K20′）**
  - ⬆ **BUMP**：`patch` —— 用户可感知的提速（`unit12-solution` 11.4s → 7.5s），无新能力
- [ ] `T-K30` `build <dir>` 不再逐文件各编一份闭包
- [ ] `T-K31` `TcCache::new` 每次 `with_ctx` 清 4MB
- [ ] `T-K40` 内核改动台账 + 文档
- [ ] `T-K41` `STATUS.md` 的"根因未修"话术更新
  - ⬆ **BUMP**：`patch` —— 批次 5 收尾（跨模块增量 + build 目录）

> `T-K21…`（K2-b / K2-a 的实现子步骤）**不是一条独立环节**：它要等 T-K20 的设计定稿后，按选中的候选方案拆成 3–6 个环节再补进本清单。
