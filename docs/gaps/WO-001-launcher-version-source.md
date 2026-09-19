# WO-001 启动器在非 Rust 仓库（课程仓）没有版本源（G-11）

> 来源：`docs/gaps/ledger.jsonl` 第 10 行（G-11，`kind:"tooling"`、`severity:"blocker"`、
> `status:"open"`、`wo_planned:"WO-001"`、`where.area = scripts/soko repoVersion()`）。
> 模板出处：`docs/design/teaching-project.md` §6.3（295–308 行）。
> 排期依据：同文件 §8 P1 第 1 条（375 行）"WO-001 G-11（启动器版本源——独立仓库的**入场券**）"；
> 风险表 459 行"独立仓库的版本钉不住 ⇒ G-11 就是它"。
> 用户定调：`REQUIREMENTS.md` 1427–1440（2026-09-18：D-1 落点独立仓库 + 生成器
> `scripts/new-course-repo.sh` 已实测 3 checked / 0 failed）。
>
> 一句话：**版本钉的"文件形态"在课程仓已经存在（`sokonanoda-version.txt`），启动器却只认
> `Cargo.toml`；而且在"无版本源"形态下它还会绕过自己的 STALE 守卫去 exec 陈旧缓存。**

## 用户可见症状 / 最小复现

- 复现命令（一条，可直接粘贴）：
  `bash docs/gaps/repro/G11-launcher-version-source.sh`
- 实测（本仓版本 0.58.0，脚本按约定 **exit 0 = 缺口仍在、与台账 `today` 一致**）：

```
== ① 空缓存 + 无版本源：启动器解析不出任何二进制 ==
  "cache": ".../tmp.XXXX/cache",
  "cli": {},
  "lsp": {}
   → cli 解析为空（预期）： True

== ② 用本机缓存（有别的版本标记）时的表现（信息性，不参与判定）==
   stderr: error: No such file or directory (os error 2)（台账记：No such file or directory (os error 2)）

== ③ 兜底：显式 SOKONANODA_BIN（可用，但绕过版本钉）==
   exit=0（0 = 兜底可用；要求机器上已有二进制，不能当课程仓常态）
```

- 同一夹具（`cp scripts/soko <tmp>/scripts/soko` + 一个 `.sokonanoda`，**无 Cargo.toml**）的
  补充实测，四条都跑过：
  1. 空缓存 + `node <tmp>/scripts/soko setup` → stdout `cli: unresolved` / `lsp: unresolved`，
     **exit 3**；stderr 关键行：
     `release: https://github.com/ColorlessBoy/sokonanoda-lang/releases/download/v?/`
     ——版本号位置是字面 `v?`，因为 `repoVersion()` 返回 `undefined`
     （`scripts/soko:93-100`；URL 拼装在 `download()`，`scripts/soko:249`）。
  2. 空缓存 + `scripts/soko grade <file>` → **exit 3**，stderr：
     `scripts/soko: no usable sokonanoda binary for v? (offline=no).` + `Run scripts/soko setup first.`
     ——而 `setup` 自己也是 exit 3（第 1 条）：**"下一步"是个死循环**。
  3. 空缓存 + `scripts/soko doctor --json` → `"ready": false`、`"cli": {"present": false, "ready": false}`，
     **JSON 里干脆没有 `version` 键**（`JSON.stringify` 丢掉 `undefined`，`scripts/soko:504`）。
  4. 本机默认缓存里是 0.55.0（marker `0.55.0 darwin-arm64`）时的现场：`version --json` 的
     `cli.source` 是 `cache(STALE: expected undefined darwin-arm64, found 0.55.0 darwin-arm64)`，
     但 `scripts/soko query check --file <存在的文件> --compact` **真的跑了那个 0.55.0 二进制**：
     stderr 只有一行 `error: No such file or directory (os error 2)`、**exit 1**
     （单独执行 `~/.local/share/sokonanoda/bin/sokonanoda query check … --compact` 得到**逐字相同**的
     输出与退出码 ⇒ 证实是陈旧 CLI 在报错，不是启动器在给人话）。
     原因：`resolve()` 在"没有版本"分支把 probe 的 `cache(STALE…)` 改写成
     `cache(unknown repo version)`（`scripts/soko:339-342`），而 default 分支的守卫**只认
     `cache(STALE` 前缀**（`scripts/soko:564`）⇒ 守卫被绕过。而对外承诺是"缓存**过期就拒绝运行
     并提示**"（`AGENTS.md:54`；设计同款 `docs/design/deepseek-harness.md:258-260`；代码里的判据
     本该是 `scripts/soko:114-115` 的"marker 必须等于 `<version> <target>`"）——这个承诺在
     "没有版本源"的形态下不成立。

- 兜底与代价（台账 `workaround` 原文，实测可用）：`SOKONANODA_BIN=<绝对路径>`（复现 ③ exit 0）。
  代价 = 放弃"课程仓钉住工具链"这一条；`scripts/new-course-repo.sh` 生成的门禁因此内建了
  "STALE/unknown 都不可信"的保守探测（`:187-218`），README/CI 也都写着兜底说明
  （`:170`、`:212-215`、`:302-306`、`:318-326`）。

## 期望行为

- **性质**：非 Lean 语义，是**工具链行为**。官方 Lean 4 的对应物是两个既有事实（出处
  `docs/design/imports-and-projects.md:175-179`）：**Lake 不向上找包**，而 **elan 向上找
  `lean-toolchain`**（"walking up through parent directories until a toolchain version is found"）
  ——"工具链是继承的，包根不是"。本仓已经决定把"根"与"版本钉"分开，本 WO 补的是**版本钉的
  仓库自述文件那一半**（今天只有语言仓的 `Cargo.toml` 能当源）。
- **版本源链**（台账 `expected_lean`，按优先级）：
  1. 环境变量 `SOKONANODA_VERSION`；
  2. `<repo>/sokonanoda-version.txt`（生成器今天就写它，`scripts/new-course-repo.sh:50`）；
  3. `<repo>/sokonanoda.toml` 的 `requires`（仅当写了完整 `x.y.z` 时能当**下载锚点**；
     `requires = "0.58"` 这种 major.minor 形态只能当**约束**，见"边界"）；
  4. `<repo>/Cargo.toml`（语言仓现状，回归不变）。
  锚点 `REPO` = **启动器自身所在仓库根**（`HERE`/`REPO`，`scripts/soko:45-46`），**不是 cwd**
  ——vendored 到课程仓 `scripts/soko` 后天然指向课程仓根，结果与调用位置无关。
  名字与形态要与既有约定对齐：`SOKONANODA_VERSION` 在 `scripts/install.sh:14-18,27` 已存在
  （release tag，**带不带 `v` 前缀都收**），启动器必须同形，不要造第三个约定。
- **多源一致性**：所有**出现**的源必须一致（`major.minor` 形态按 major.minor 比）；不一致就
  按 `docs/design/imports-and-projects.md:181` 的语气**指名文件**报错（"告诉用户改哪个文件"）。
  语言仓里的回归红线：`Cargo.toml` 仍是语言仓的唯一日常源（不能因为多了一个散落的
  `sokonanoda-version.txt` 就悄悄换钉）。
- **拒绝执行契约（本 WO 的第二个交付点）**：**只要解析不出期望版本，就绝不 exec 缓存/仓库构建的
  二进制**，而是 exit 3 + 人话（期望版本、来源、实际 marker、"该改哪个文件 / 跑哪条命令"）。
  今天 `cache(unknown repo version)` 这条静默通道必须消失（`scripts/soko:336-350`）。
- **可观测性**：`version --json` 的 `version` 必须有值，并新增来源字段
  （建议 `version_source: "env" | "version.txt" | "manifest.requires" | "Cargo.toml"`；
  字段名是实现细节，**行为**必须可断言）；`doctor --json` 的 `version`/`ready` 随之能报；
  `setup`/`update` 的下载 URL 必须带真实版本（不再出现 `v?`）。
- **本教学子集的边界（哪些不做）**：
  - **不做 semver 范围**：`>=0.57`、`^0.58`、`~0.58`、通配一律不支持——front 侧今天也只比
    major.minor（`crates/front/src/project/manifest.rs:123-131`），本 WO 不扩大版本文法；
  - **不做向上继承**：版本文件只在 `REPO` 根，不沿父目录找（与 Lake"不向上找包"同侧；
    显式不引入 elan 式继承，`docs/design/imports-and-projects.md:190-191` 的取舍不变，
    只是把"钉在 `requires`"改写成"钉在仓根的自述文件，`requires` 保留为清单契约"）；
  - **不改 `requires` 在 front 的语义**：仍然 warn-only、不阻断判卷；
  - **不做自动 bump / 自动改写**课程仓的版本文件（人改文件，工具只读）。

## 范围

- `scripts/soko`（唯一实现文件，584 行；改动点全部带行号）：
  - `repoVersion()`（93–100）→ 保留名字/返回形状（`string | undefined`），内部改走新的
    `versionSource()`（返回 `{version, source, constraint?}`），调用点
    `319`/`413`/`470`/`484`/`504`/`519`/`558` 一并接上；新增 `sokonanoda-version.txt` 的读取与
    `sokonanoda.toml` 的**最小单行解析**
    （只认顶层 `requires = "x.y.z"`；启动器是零依赖 Node，不能也不该 spawn 一个还没有的
    二进制去解析 TOML）。
  - `RELEASES` 常量（44）→ `releaseBase()`，读 `SOKONANODA_RELEASE_BASE`（默认值逐字不变）。
    理由：现在的 e2e 只能靠真网络；CLI 侧已有同名开关（`crates/cli/src/env/target.rs:83-84`，
    文档 `docs/design/binary-cli.md:27`），加上它才能把"按版本源下载"做成**无网络**测试
    （先例：`crates/cli/tests/opencode.rs:62-70` 的本地 HTTP server）。**待确认**：`install.sh:41`
    另有一个 `SOKONANODA_BASE_URL`，轮内是统一两个名字还是先只加 CLI 那个（建议先加
    `SOKONANODA_RELEASE_BASE`，并把统一问题记进 `docs/design/binary-cli.md`）。
  - `resolver()`（317–352）：probe 保留；`resolve()` 的 `!version` 分支（339–342）改成"拒绝"，
    default 分支的守卫（564）改成看**可信来源**而不是前缀字符串（`cache(STALE` / `cache(unknown`
    都要拦）；`setup`/`update` 的失败文案（419–429）补上"版本源在哪、期望什么、该改哪个文件"。
  - `version --json`（463–491）/`doctor --json`（494–527）：补来源字段，`version` 不再可能缺键。
  - 文件头注释的"解析链"描述（14–17）与 `--help` 文案（399–400）同步。
- `.opencode/plugins/sokonanoda.ts`：`findRepoRoot`（44，要求 `Cargo.toml` + 插件文件）与
  `readVersion`（84）是**同一条链的第二实现**；设计承诺"语义与插件的解析链完全一致"
  （`docs/design/deepseek-harness.md:255-256`，形状在 `crates/cli/tests/opencode.rs:103-113` 钉着）
  ⇒ 同轮同步（或在本 WO 明确写出"为什么本轮不同步"的理由；倾向同步，代价很小）。
- `crates/cli/tests/`：`launcher.rs`（真跑 node 的行为契约，先例 `:30-46` 的 require_node、
  `:111-161` 的"假仓库无 Cargo.toml"夹具）、`dsh.rs:387-414`（形状 needle）、
  `opencode.rs:103-113`（插件 needle）。
- 课程侧文案（兼容用，非语义）：`scripts/new-course-repo.sh:11-12`、`:170`、`:187-218`、
  `:212-215`、`:302-306`、`:318-326`。
- **是否动内核：预期否**。`crates/kernel/**` 一行不改（冻结快照）；连 `crates/front/**` 与
  `crates/cli/src/**` 都不需要改——CLI 自己的 `version/doctor/setup/update` 用的是编译进二进制
  的版本（`crates/cli/src/env/target.rs:9-11`），下载来的二进制自报版本，本 WO 不碰。

## 不做的事（明确排除，防顺手扩大）

- 不修 G-12（相对路径 / 模块根退化）、G-10（`query check` 假绿）、G-02（构造子命名空间）、
  G-03（Prop+Type 参数归纳）、G-15（内核错误 span 漂移）——各自有 WO，排期见
  `docs/design/teaching-project.md:375-381`。本 WO 的唯一产物 = 版本源链 + 拒绝执行守卫。
- 不新增用户可见的 CLI 子命令（不做 `sokonanoda pin`）；不改 `sokonanoda.toml` 的三键封闭
  （`name`/`requires`/`src`，`crates/front/src/project/manifest.rs:91`）。
- 不做自动升级/自改课程仓；不改 release 产物形态（tarball 名与 `v${version}` URL 结构照旧，
  **永不引入 `latest`**，`crates/cli/tests/dsh.rs:408-413` 会拦）。
- 不删任何既有解析链环节：`SOKONANODA_BIN`/`SOKONANODA_LSP_BIN` 兜底、`SOKONANODA_OFFLINE`、
  VS Code 扩展自带这三条必须继续有效（`launcher.rs` 现有三条用例不许改判据）。
- 不做"任意目录向上找版本文件"，也不把 cwd 卷进版本解析。

## 兼容策略（既有课程 / 既有课程仓 / 与 G-02 的边界）

- **语言仓内的课程（`courses/set-theory/`，12 单元）本轮零改动**：它用的是仓库根启动器
  （REPO = 语言仓 ⇒ 链上命中的仍是 `Cargo.toml`），`courses/set-theory/sokonanoda.toml:4` 的
  `requires = "0.58"` 保持 warn-only；生成器也**不需要**给仓内课程补 `sokonanoda-version.txt`。
- **独立课程仓今天就是新形态**：`scripts/new-course-repo.sh` 已经写 `sokonanoda-version.txt`
  （`:50`）与 `requires = "<完整版本>"`（`:145-149`）⇒ 缺的只是"启动器会读"。修好后同轮清理
  生成器里的 G-11 兜底文案与门禁的保守探测（`:187-218` 的 `resolve_soko()` 可以简化成
  "调 `version --json`，看 `version` 非空且来源可信"），并把 README/CI 里的
  `SOKONANODA_BIN` 说明撤掉（`:170`、`:212-215`、`:304`、`:325-326`）。
- **已经生成过的旧课程仓**：把 `scripts/soko` 换成修复版即可（vendored 文件是唯一需要更新的
  东西，`:152-153` 就是 `cp`）；没有 `sokonanoda-version.txt` 的仓回落到 `requires`（生成器写的是
  完整版本）⇒ 老仓也能被钉住，不需要人工造文件。
- **与 G-02 的边界（防撞车）**：本 WO **不**动构造子命名（`prod_mk`/`Prod.mk`）。G-02 修好时
  必须同轮改的清单（留给 WO-005，见 `docs/design/teaching-project.md:378-379` 与
  `courses/set-theory/lib/Prod.sokonanoda:15-25` 的"修好后机械改名"注释）：
  `courses/set-theory/lib/Prod.sokonanoda`（`prod_mk` 12 处，含 `:45` 的构造子声明与
  `:58,62,68-73` 的展开引理）、
  `courses/set-theory/units/unit05-pairs-products.sokonanoda`（22 处，如 `:44-45,61,156-158`）、
  `courses/set-theory/units/solutions/unit05-solution.sokonanoda`、
  以及 `course/`（入门课）与 `courses/set-theory/` 的 golden 计数。**本轮只登记，不执行。**

## 验收（三层）

- **front 单测**：本 WO 不碰 `crates/front` ⇒ 第一层落到既有的**启动器行为契约**
  `crates/cli/tests/launcher.rs`（它已经用 node 真跑 `scripts/soko`，是最贴近的一层）。新增（全部
  hermetic：`SOKONANODA_CACHE_DIR` 指向临时目录、`SOKONANODA_OFFLINE=1`、不依赖 `target/`）：
  1. 假课程仓（copy `scripts/soko`，**不写** `Cargo.toml`，写 `sokonanoda-version.txt`）+ 预置
     marker 匹配的假二进制 ⇒ `version --json`：`version` = 钉的版本、`version_source` = `version.txt`、
     `cli.source` = `cache`；`doctor --json` exit 0。
  2. 同上但 marker 是**别的**版本 ⇒ 转发命令必须**不执行**缓存二进制：exit 3，stderr 含期望版本 +
     文件名 + "refusing"语义；**断言 stderr 不出现 `os error 2`**（今天 ② 的反例）。
  3. 无任何版本源 + 有缓存（含 marker 陈旧）⇒ 同样拒绝，且文案指名去找过的文件（今天会被静默 exec）。
  4. 优先级：`SOKONANODA_VERSION` > `version.txt` > `manifest.requires` > `Cargo.toml`
     （临时目录里伪造四种文件两两组合断言；语言仓回归 = 本文件现有三条用例一字不改仍绿）。
  5. `requires = "0.58"`（major.minor）：不得当下载锚点，不得构造 `/v?/`，且必须给出人话
     （"这是约束不是钉，请写 `sokonanoda-version.txt` 或完整版本"）。
  6. 下载路径（加了 `SOKONANODA_RELEASE_BASE` 之后）：照 `opencode.rs:62-70` 起本地 HTTP server，
     喂一个含两个假二进制的 tar.gz，断言请求是 `/v<钉的版本>/sokonanoda-cli-<triple>.tar.gz`
     与 `/v<钉的版本>/sokonanoda-lsp-<triple>.tar.gz`、marker 写成 `<钉的版本> <target>`、
     无 `/latest/`。若最终不采纳该 env，退化为：`update` 离线时 stderr 的 `release: …/v<钉的版本>/`
     断言版本进得去 URL（弱一档，但仍不需要网络）。
- **CLI e2e**：
  - `bash docs/gaps/repro/G11-launcher-version-source.sh` → **exit 1**（行为已变）；
  - 生成器真跑：`scripts/new-course-repo.sh /tmp/soko-course-$$ --no-git`，然后
    `node /tmp/soko-course-$$/scripts/soko version --json`（`version` 非空、来源是 `version.txt`）
    与 `python3 /tmp/soko-course-$$/scripts/check-course.py` 绿——骨架基线 `3 checked / 0 failed`
    （`REQUIREMENTS.md:1432-1435` 记的实测），**且全程不需要 `SOKONANODA_BIN`**；
  - 语言仓自检不回退：`scripts/soko version --json` 的 `version` 仍是 `0.58.0`（来自 `Cargo.toml`）、
    `cli.source` 仍是 `repo-build`（本轮实测，见 `version` 的 probe 路径 `scripts/soko:463-491`）。
- **课程用例**（真文件、真练习）：
  - 总门禁：`python3 courses/set-theory/tools/check.py`（lib 自检 + 12 单元 + 12 解答，**绝对路径**、
    **只看 `grade` 退出码**——理由见 `courses/set-theory/README.md:17-19`）。本轮实测基线：
    **34 个目标 / 296 checked / 93 open / 0 个被判负 / exit 0**；修完必须逐字不变。
  - 单点抽查（`import` + 构造子名都能被这条练习覆盖）：
    `courses/set-theory/units/unit05-pairs-products.sokonanoda` 的**练习 1 `prod_fst_mk`（第 60 行，
    提示在第 56-59 行）**，它是项目模式（`:25-27` 有 `import lib.Logic` / `lib.Set` / `lib.Prod`）
    且用到 `prod_mk`（`:44-45,61`）；对应解答
    `courses/set-theory/units/solutions/unit05-solution.sokonanoda`（`prod_fst_mk` 在第 24 行）。
    判卷命令：`node scripts/soko grade "$PWD/courses/set-theory/units/solutions/unit05-solution.sokonanoda"`
    → 8 checked / 0 failed（本轮实测）。
- **影响面**：本 WO **不改编译器、不改事件流** ⇒ **事件计数不变**，
  `crates/cli/tests/course.rs:86` 与 `crates/cli/tests/course_status.rs:68` 的**双 GOLDEN 无需同步**；
  但 `cargo test --workspace --locked` 必须全绿，尤其 `launcher.rs` / `dsh.rs` / `opencode.rs`
  三条契约（后两条可能只需要把新关键词加进 needle 列表）。

## 文档同步清单

- `docs/design/deepseek-harness.md` H1 第 1 条与"版本守卫"段（253–260）：`Cargo.toml` → 版本源链；
- `docs/design/onboarding.md:155-159`（"解析链与 opencode 插件同语义"这句要补版本源）；
- `docs/design/binary-cli.md:27` 附近：登记 `SOKONANODA_VERSION`，以及启动器是否共用
  `SOKONANODA_RELEASE_BASE`（**待确认**：与 `install.sh:41` 的 `SOKONANODA_BASE_URL` 统一问题）；
- `docs/design/imports-and-projects.md:178-179、190-191、294、433`：把"版本钉 = `requires`"改写为
  "版本钉 = 仓根自述文件（`sokonanoda-version.txt`），`requires` = 清单契约（warn-only）"，
  并说明这是"**同目录、不向上找**"的钉文件（不是 elan 式继承）；
- `AGENTS.md:50-56`（Setup 的解析顺序与"`<version> <target>` 必须等于 `Cargo.toml` 版本"）；
- `skills/`：`skills/README.md:34`、`skills/sokonanoda-doctor/SKILL.md:23,27`、
  `skills/sokonanoda-update/SKILL.md:27,50`、`skills/sokonanoda-teacher/SKILL.md:80`、
  `skills/sokonanoda-dev/SKILL.md`（环境节）；改完确认 `.agents/skills/` 入口不漂移
  （`crates/cli/tests/dsh.rs` 会挡）；
- `editor/vscode/`：**待确认**——扩展自带 LSP/CLI、不解析仓库版本源；只有把 `version --json`
  的新字段接进 VS Code 状态展示时才需要同轮改（今天 `editor/vscode/README.md`/`CHANGELOG.md`
  只在文档里提到启动器）；
- `docs/HANDOVER.md`、`STATUS.md`（收尾义务）；`REQUIREMENTS.md` §9 在 2026-09-18 那条
  "独立仓库落点"后补一行"版本源链已落地"（用户可见契约变更）。

## 门禁

- `scripts/soko gate`（= fmt + clippy + test + playground 锚点；注意 gate 用**运行中二进制**的
  内嵌编译器，版本不一致会 exit 3 —— 先 `scripts/soko update`，或用
  `cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda`）；
- 全量 `cargo test --workspace --locked`（新增用例必须在**无网络**下也过：CI 无代理）；
- 额外：`python3 scripts/gap.py check`（关账前后各跑一次，见下）。

## 关账

1. 改 `scripts/soko` + `.opencode/plugins/sokonanoda.ts` + 三个契约测试；跑门禁。
2. **同轮改复现脚本** `docs/gaps/repro/G11-launcher-version-source.sh`：夹具从"只有启动器的裸目录"
   升级成**最小课程仓**（写 `sokonanoda-version.txt` + `sokonanoda.toml`，预置一个旧 marker 的假
   缓存），断言改成：① 版本从文件解析出来（`version --json` 的 `version` 非空 + 来源正确）；
   ② 陈旧缓存被**拒绝**（exit 3 + 人话，且没有裸 `os error 2`）；③ `SOKONANODA_BIN` 兜底仍可用；
   脚本按新契约翻成 **exit 1**。
   **为什么必须改**：`gap.py close` 会**先复跑复现**，exit 0 就拒绝关账
   （`scripts/gap.py:198-206`）；而今天 ① 的断言是"cli 解析为空"，修好后在一个**真没有任何版本源**
   的裸目录里仍然为空（那恰恰是正确行为）⇒ 不改夹具就永远关不了账。
3. 更新 `docs/gaps/ledger.jsonl` 的 G-11：`wo:"docs/gaps/WO-001-launcher-version-source.md"`、
   `status:"wo-filed"`（本轮派工），关账时 `status:"fixed"` + `fixed_in:"<新版本>"`，
   `today` 改写成新行为一句话。
   > 现状提醒：**本 WO 的派工只写了这张工作单本身**——`ledger.jsonl` 第 10 行今天仍是
   > `"wo":null` / `"wo_planned":"WO-001"`，上面这次字段更新还没做（按台账协议 §6.2，
   > WO 文件存在后才填 `wo`）。
4. `python3 scripts/gap.py close G-11 --version <新版本>`（复现翻 1 后它才会写 `fixed_in`）；
   随后 `python3 scripts/gap.py check` 应打印"全部与台账一致"；
5. bump 版本（`Cargo.toml` + `editor/vscode/package.json`）→ push main（auto-tag 自动发版，
   见 `docs/RELEASE.md`）；旧课程仓同步替换 vendored `scripts/soko`。
