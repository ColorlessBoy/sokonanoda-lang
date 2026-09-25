# 设计：`.sokonanoda/` 项目产物目录（R-3 前半，2026-09-24，T-B4）

> **用户原话**（`REQUIREMENTS.md` §9 R-3）：
> 「在 project 模式下，`sokonanoda.toml` 所在的根目录下，应该有 `build` 的文件才对，
> vscode 和 code agent 都应该在这里取编译后的数据，避免重复计算。比如创建一个
> `.sokonanoda` 文件夹，把编译、以及以后的依赖啥的都放到这个文件夹下。」

**本环节要回答的 6 个问题**（`e2-plan.md` §13 T-B4）：放什么 · 命名 · 清理策略 ·
`--clean` 语义 · 与现有缓存的关系 · **模块根产物 vs 全局缓存的分工**。
最后一条是**阶段 B 的刹车点**（`e2-plan.md` §3：分工说不清就只做 R-4、R-3 另出设计）——
本文的核心就是把它说清。

---

## 0. 先纠正一处事实：仓库里有**两个**"缓存"，此前文档把它们混为一谈

| | **二进制缓存** | **编译产物缓存** |
|---|---|---|
| 路径 | `~/.local/share/sokonanoda/bin/`（`sokonanoda` · `sokonanoda-lsp` · `*.version` 标记） | macOS `~/Library/Caches/sokonanoda/compiled/`；Linux `$XDG_CACHE_HOME\|~/.cache/sokonanoda`；Windows `%LOCALAPPDATA%\sokonanoda` |
| 谁维护 | `scripts/soko` 启动器 / `sokonanoda setup`·`update` | 编译器本身（CLI 与 LSP 共用同一份实现） |
| 覆盖变量 | `SOKONANODA_CACHE_DIR`（启动器的缓存根） | `SOKONANODA_CACHE_DIR`（**同一个变量名**，但语义不同）· `SOKONANODA_NO_CACHE=1` |
| 内容 | 平台二进制 + 版本标记 | `<key>.json`（`CachedCompile`：报告 / 事件流 / 项目报告） |
| 证据 | `scripts/soko` 的 `cacheDir()/markerPath()` | `crates/front/src/compile/cache.rs:67`（`root()`）/`:80`（`compiled_dir()`） |

**坑**：两个缓存**同名不同物** —— `SOKONANODA_CACHE_DIR` 对两者都生效，但默认根
不同（`scripts/soko:115-119` 给的是 `~/.local/share/sokonanoda/bin`；
`compile/cache.rs:67-79` 给的是平台缓存目录）。`scripts/vscode-e2e.sh` 与部分
gap repro 正是靠它把两者一起隔离到临时目录 ✓。

**R-3 说的是后者**（编译产物）。`REQUIREMENTS.md` §9 与 `e2-plan.md` 里把它写成
`~/.local/share/sokonanoda` ✗ —— 那是二进制缓存；本文按代码事实纠正，
并**不改**用户可见语义（用户要的是"产物在模块根下"）。

## 1. 现状（都实测/读到代码）

### 1.1 键与作用域：**单文件按内容、项目按位置**

| 条目 | 键 = | 实测（隔离缓存） |
|---|---|---|
| 单文件 | FNV-1a over (`CACHE_FORMAT`, `CARGO_PKG_VERSION`, build stamp, prelude 模式, **源文本**) —— `compile/cache.rs:139`(`key`) | 两份**逐字相同**但目录不同的文件 ⇒ **1 个条目**（共享 ✓） |
| 项目闭包 | `ProjectPlan::digest(options)` = 同一批 + **入口的绝对路径** + 每模块名/源/import 边 —— `project/mod.rs:156-172`（T-A06：内容相同不代表位置相同） | 两份逐字相同的项目（只差绝对路径）⇒ **2 个条目**；同目录再跑 ⇒ **命中** ✓ |

⚠ **取证实测的反例（T-B4 取证 B，已修）**：摘要里含的是**入口绝对路径**，
**不含模块根**（`project/mod.rs:172` 只 mix `digest_path(&self.entry)`）⇒
"**同一入口 + 两个内容逐字相同的根**"会算出**同一个摘要**：`build --root rootA`
之后 `build --root rootB` 直接 **hit**，`query project --root rootB` 报出 **rootA**
的根与模块路径（definition/references 会跳到别的根下的文件）✗。
⇒ T-B5 在 `load_at` 的**全局兜底**那条加了守卫：`ProjectReport::root` 与当前根
不一致 ⇒ 当 miss 重编（项目目录那条路天然安全，目录就是根）；
判据 `crates/cli/tests/artifacts.rs::a_project_entry_is_never_replayed_across_module_roots`
（抽掉守卫实测 **exit 101** ✓）。

⇒ **项目条目是按"入口位置"隔离的**（本节结论按上一条修正）：同根的入口不会被
别的目录命中，
**放进模块根不会串台，也不损失任何跨项目复用**。
（单文件条目反过来：键里没有位置信息 —— `compile::cache::key(src, options)`
**连路径参数都没有** ⇒ 放哪儿都是同一份，留给全局缓存才能跨项目共享。）
这条"项目条目按位置隔离"是**分工决定的地基**，已固化成判据：
`project::tests::closure_digest_is_scoped_to_the_entry_location`（T-B4 新增）。

### 1.2 落盘与原子性
`compiled/<key>.json`，写入 = **临时文件 + rename**（`compile/cache.rs:206-213`），
best-effort（写失败绝不让调用方失败）；读失败/损坏/`format` 不符 ⇒ 当 miss
（`load_in`）。`CACHE_FORMAT` 变更会让所有旧条目作废 ✓。

### 1.3 `--clean` 现状：**全局**
`build --clean` → `cache::clean()` → 清全局缓存的 `compiled/`
（`crates/cli/src/build.rs:22`、`compile/cache.rs:216`），**不分项目**；
事件 `{"type":"build.clean","removed":N}`。
两条实测细节（取证 B）：① `clean_in` 删该目录下**所有文件**（含写了一半的
`*.tmp-*`、含非 `.json`），只是**只对 `.json` 计数**；
② 它**只扫一层**，所以 `.sokonanoda/` 根下的 `meta.json`/`.gitignore`
（`compiled/` 的**同级**）天然不会被碰到 ✓——这也是把它们放在根而不是
`compiled/` 里的原因。
另有一条实测 bug（T-B5 顺手修）：`SOKONANODA_NO_CACHE=1` 时 `root()` 返回 `None`
⇒ `clean()` 恒 `removed 0`，用户"关掉缓存"之后**再也清不掉**已写下的条目 ✗；
现在 `clean()` 走不看 `NO_CACHE` 的目录解析。

### 1.4 谁在读写（改动要同时照顾）
* CLI：`build`（`build.rs:126/137/153/159`）、`check`、`query`、`course`；
* LSP：读 `lsp/src/lib.rs:230`、写 `lsp/src/lib.rs:278`（项目）与 `:283`（单文件）；
* 扩展/agent：**不直接碰**编译产物缓存（只经 LSP/CLI）；
  `SOKONANODA_CACHE_DIR` 在 `scripts/vscode-e2e.sh` 与部分 gap repro 里被用来**隔离**。

## 2. 分工决定（本环节的核心）

**一句话**：**产物按项目落盘，全局缓存退居"跨项目共享的后备"；
项目条目只认模块根，单文件条目只认全局。**

| | 单文件条目（内容键） | 项目闭包条目（内容 + 入口绝对路径） |
|---|---|---|
| 写 | 全局缓存（现状不变） | **`<模块根>/.sokonanoda/compiled/`** |
| 读顺序 | 全局 | **项目目录 → 全局**（老条目仍能命中 ⇒ 升级平滑） |
| 为什么 | 键里没有位置 ⇒ 放项目里也没有额外收益，反而丢掉跨项目共享 | 键里本来就有位置 ⇒ 天然项目作用域；放项目里 = **vscode 与 agent 一处取用**（用户动机）、可随项目删/归档、可被项目级清理 |

**刹车点解除**：`e2-plan.md` §3 的"分工说不清就停"对应这条规则——
它有**可执行判据**（§7）：项目条目在项目目录、单文件条目不在。

**项目目录的不变量（新增，T-B5 实现）**：
**只放"磁盘状态的产物"** —— 即 `overlay`（编辑器未落盘文本）为空时算出的摘要。
带未落盘编辑的条目**仍进全局缓存**（它与磁盘无关、是瞬时的）。
理由：项目目录是给人和 agent 看的（"这里放着这个项目编出来的东西"），
塞进瞬时条目会让它变成噪声；而全局缓存本来就是可丢的。

## 3. `.sokonanoda/` 的契约

### 3.1 目录树（v1）
```
<模块根>/.sokonanoda/
├── .gitignore          # 内容恰好一行 `*`（自忽略，实测见 §3.8）
├── meta.json           # 产物目录自述（§3.2）
└── compiled/
    └── <key>.json      # 与全局缓存**同格式同键**（不造第二套）
```
**预留**（本阶段不建、只写进契约）：
* `prefix/` —— 阶段 D 的 **T-D11**（把"已验证的前缀环境"跨会话持久化）；
* `deps/` —— 将来的依赖（用户原话里的"以及以后的依赖啥的"）。

**不建**：`build/`（用户口语里的"`build` 的文件"就是这个目录的内容，
再套一层 `build/` 只会产生 `build/compiled/` 这种绕口路径）。

### 3.2 `meta.json`（人可读、机器可判）
```json
{ "schema": "soko.artifacts/1",
  "compiler": "0.67.0",
  "build_stamp": "…hex…",
  "platform": "macos/aarch64",
  "created": "2026-09-24T…Z",
  "written": "2026-09-24T…Z" }
```
* `schema` 不符 ⇒ **整目录当不存在**（读方忽略、按需要重建）——与 `CACHE_FORMAT`
  同一条纪律：宁可重算，绝不误用；
* `compiler`/`build_stamp` 只作**诊断与提示**；真正的作废靠键里的版本 + stamp ✓
  （两次保险不需要，键已经够了）。

### 3.3 红线：产物的**扩展名不许是 `.sokonanoda`**（两条独立证据）

1. `build <dir>` 的 `collect_files` **会递归进隐藏目录**、只按扩展名收集
   （`crates/cli/src/build.rs:172-191`）⇒ `<root>/.sokonanoda/` 里若有
   `*.sokonanoda`，会被当成源文件再编一遍（自我吞掉）；
2. VS Code 监视的是 `**/*.sokonanoda`（`editor/vscode/extension.js:1981`），
   服务端有 `didChangeWatchedFiles` handler（`crates/lsp/src/lib.rs:1491`）
   ⇒ 命中该 glob 的产物会引发"写 → 重编 → 再写"的回声。

本契约的产物一律 **`<key>.json`** ✓（既有格式），两条红线都不会碰。
（另：`scripts/notation-lint.py`、`scripts/verify-decl-panel.py` 也是
`rglob("*.sokonanoda")` 不跳隐藏目录 ⇒ T-B5 实现时若想改这三处的遍历，必须先有判据。）

### 3.4 命名与键：**沿用**，不造第二套
`compiled/<key>.json` 与全局缓存**逐字节同格式**（同一个 `CachedCompile` /
`CacheFile` / `CACHE_FORMAT`）⇒ ① 拷贝即迁移；② 两处读同一份实现；
③ 新测试可以"把项目目录的条目挪到全局缓存里，行为不变"。

### 3.5 原子性与并发
沿用 §1.2 的 temp+rename ✓。同一键并发写 = 后者覆盖，而**同一键 ⇒ 同一内容**
（键是内容的函数）⇒ 安全。`--clean` 与写入并发 = best-effort（与现状同级）。
多开编辑器/agent 同时跑同一模块根：安全（见上），不需要锁 ✓。

### 3.6 清理策略
* **不自动清** —— 项目目录跟着项目走，用户删项目即删产物；
* 编译升级后旧条目**自然 miss**（键含版本 + build stamp）⇒ 只是占空间；
* 体积：**纠正**（取证 A 实测）项目条目是**整份 `ProjectReport`**（含事件流），
  **0.6–5.9 MB 一条**（`courses/set-theory` 的解答在 4.78 MB 一档；本机全局缓存
  383 条 / 117 MB，p50 ≈ 50 KB、p90 ≈ 966 KB、max 4.6 MB）——只有**单文件**条目
  才是几十 KB 量级。本文早先写的"单条几十 KB"是错的 ✗；
* **上限 = 32 条/模块根，超了按 mtime 淘汰最旧**（`project::cache::MAX_ENTRIES`）：
  没有上限时"每次编辑 = 新摘要 = 新条目"会只增不减（一个课程根可到上百 MB ✗）。
  淘汰**不影响正确性**（被淘汰的条目只是下次重编）；
* 提供 `--clean`（§3.7）；
* **沿用既有的 `is_clean` 门槛**（错误/诊断才不写缓存）——`ProjectReport::is_clean`
  **已经不把 `requires` 漂移算作"不干净"**（`project/report.rs:163-177`，G-24 已修
  ✓ 注：`docs/design/compile-cache.md` 里那段"漂移 ⇒ 永不写缓存"的散文**已过期**，
  以代码为准）。本环节**不收紧也不放宽**这条门槛 ✓。

### 3.7 `--clean` 语义（**改**：两处都清）
`build --clean` 从"清全局"变成"**清全局 + 当前模块根的 `.sokonanoda/compiled/`**"：
* 为什么必须改 ✗：`rebuild`（= `build --clean`）的语义是"清空编译缓存后重编译"；
  若项目目录里的条目不清，`rebuild` 会**命中项目条目 ⇒ 什么都没重编**（用户可见的假动作）；
* 事件 additive：`{"type":"build.clean","removed":N,"global":G,"project":P}`，
  `removed == G + P`（老消费者读 `removed` 语义不变 ✓）；
* 解不出模块根时（`build --clean` 无参数）只清全局，并在输出里说明 ✓。

### 3.8 `.gitignore`：**自忽略**（实测）
首次创建目录时写 `<模块根>/.sokonanoda/.gitignore`，内容**恰好一行 `*`**：
```
$ git status --porcelain      # 目录内 .gitignore = `*` ⇒ 输出为空（完全看不见 ✓）
```
* **不用**变体 `*` + `!.gitignore`：那会让 git 提示 `?? .sokonanoda/`（实测 ✗）；
* 代价：将来若要**提交**产物（例如依赖），得 `git add -f` —— 可接受，写进文档；
* **不自动改用户仓库根的 `.gitignore`**（只在自己目录里放一个），
  并在 `docs/`/技能里给一行指引（"想显式忽略可加 `.sokonanoda/`"）。

### 3.9 安全：只跳过重复劳动，永不推断
产物只是"内核已经算过的结果"，命中只省时间；读不到/损坏/版本不符 ⇒ 照常重编。
**判定永远走内核**（硬规则 4）——产物里没有任何被"信任"的判定结论。

## 4. 接口面（谁改哪里）

| 环节 | 改什么 |
|---|---|
| **T-B5**（CLI）✅ 已落地 | `front::project::cache`：`load_at`/`store_at`/`store_if_clean_at`/`clean_at`/`artifacts_dir` + `.gitignore`/`meta.json` 初始化 + 32 条上限；CLI 四处（`build`/`check`(grade)/`query`/`course`）传 `plan.root`；`--clean` 两处都清（事件 additive 加 `global`/`project`）；逃生门 `SOKONANODA_NO_PROJECT_ARTIFACTS=1`。**外加两件"不留红"的**：① LSP 的**读**路径也走 `load_at`（否则 CLI 预热不再帮到编辑器 = 性能退化 ✗，见 §2）；② VS Code e2e 的 `cacheStamp()` 改扫两处（它原来直接读 `<cacheDir>/compiled`，产物挪窝后会红 ✗），顺带成为 R-3 在 e2e 层的断言。`overlay` 非空 ⇒ 只写全局（§2 的不变量）——LSP 的**写**路径仍在全局，留给 T-C6 |
| **T-B6**（判据） | ① 同模块连跑两次 `build`：第二次 `hit == files`；② `query project` 增**只读派生**字段 `artifacts{dir, entries, bytes}`（不重跑编译，守 `project-view.md` 的纪律）；③ 体积数字进 `docs/perf/ledger.jsonl`；④ `.gitignore` 指引 |
| **T-C6**（LSP/agent 接入） | `lsp/src/lib.rs:191` 现在把 plan 丢掉（`let (_, digest)`）⇒ 保留 `plan.root` 并走 `load_at`；`:278` 的写同理 |
| **T-D11** | 在 `.sokonanoda/prefix/` 里落"已验证前缀环境"（本文只预留目录名） |

**协议**：`build.clean` 加两个字段、`query project` 加一个对象 —— 都是 **additive**
（`docs/protocol.md` 的"只加不删"）；`docs/protocol.md` 同一轮更新。

## 5. 兼容与回滚
* **升级平滑**：读顺序 项目 → 全局 ⇒ 升级前写进全局的项目条目**仍然命中** ✓
  （不必一次性迁移；全局里那些条目此后不再新增）；
* **逃生门**：`SOKONANODA_NO_PROJECT_ARTIFACTS=1` ⇒ 完全退回今天的行为
  （只写全局）。默认**开**（这是用户要的功能，有明确收益 ✓；
  与阶段 C/D 的"中性前置改动默认关"不冲突 —— 那条约束针对**无收益**的改动）；
* **回滚**：删 `.sokonanoda/` 即回滚到"只有全局缓存"的状态 ✓（无迁移、无破坏）。

## 6. 未决问题（明确留白，不当成已解决）
1. **磁盘上限 / LRU**：项目目录与全局缓存都没有上限；大课程长期使用会累积
   （键含版本 ⇒ 每次升级都会新增一批）。要不要在 `meta.json` 记用量并在超限时提示？
2. **依赖目录（`deps/`）的格式**：用户提到"以后的依赖"，但包管理还没设计 ⇒ 只预留名字；
3. **嵌套模块根**：子目录里再有一个 `sokonanoda.toml` 时，产物跟最近的根 ✓（现状的
   发现规则）；父根的产物**不含**子根的内容 —— 需要实测确认没有交叉（T-B6 补一条）；
3a. **入口路径的归一化不一致**（T-B5 实测暴露的**历史遗留**，不属于 R-3）：闭包加载
   会把入口 `canonicalize`（`course.json` 里是相对路径），而 CLI 的 `build`/`query`
   用**命令行原样路径**算摘要 ⇒ 在**符号链接路径**下两者算出**不同摘要**、共用失效 ✗。
   实测（同一份 fixtures）：路径规范化过 ⇒ `course` 写的条目被 `build` **命中**；
   用 `/var/...` 原样路径 ⇒ `build` 另写一条（miss）。
   ⇒ 影响：`course` 预热过的东西，`build` 在 `/tmp`、`/var` 这类路径下仍会重编
   （只是慢，不是错）。修法要统一入口路径的归一化（并会让旧条目作废），**排在 R-3 之后**；
   判据应做成一条真进程用例（`course` → `build` 命中，两种路径写法都试）。

3b. **模块根归一化**：零配置时模块根 = 入口目录，而 macOS 上 `/var/…` 与
   `/private/var/…` 是同一目录的两种写法（G-12 踩过）⇒ 同一个项目可能写出**两个**
   产物目录。T-B5 是否要 `canonicalize` 根路径，要在实现时定（不动键，只动目录选择）；
4. **`query project` 的 `artifacts.bytes`**：要不要真的遍历目录（IO）还是靠 `meta.json`
   记数？前者诚实、后者便宜 —— T-B6 定；
5. **`.gitignore` 变体**：若将来要**提交**部分产物（依赖），自忽略策略要重设计。

### 6b. 测试与复现件的卫生（T-B5 顺带处理）
产物落模块根之后，**跑在仓库夹具上的测试会把 `.sokonanoda/` 写进工作区**（虽然被
自忽略 + 根 `.gitignore` 双重忽略，看不见，但会累积、并可能让"冷开"用例意外变热）：
* 只测缓存**隔离**的用例（`course_project.rs`、`course_manifest.rs`）现在显式带
  `SOKONANODA_NO_PROJECT_ARTIFACTS=1`（= 退回旧语义，与它们本来的意图一致）；
* 真正测**产物布局**的判据集中在 `crates/cli/tests/artifacts.rs`，且一律在**临时目录**
  里跑（不进仓库）；
* `scripts/vscode-e2e.sh` 每次跑之前清掉夹具里的 `.sokonanoda/`（否则第二次跑就不是
  "冷开"了）。
* 本机还有一条**与代码无关**的限制（记下来免得下次误判成 bug）：某些受限环境会拦
  **仓库内的 `rename`** ⇒ 产物条目写不成（只剩 `*.tmp-*`）。判据放临时目录即可绕开 ✓。

## 7. 判据
**本环节（T-B4，文档级）**：本文必须回答 §13 列出的 6 个问题（§0–§3 逐条对应 ✓），
且每条决定都写了"替代方案与为什么不选"（§2 表格、§3.7、§3.8）。
**⚠ T-B6 的判据别押在 `build <dir>` 上** ✗：`build <dir>` 今天**每文件各编一份完整
闭包**（`build.rs:172-191` + `compile-cache.md:69-73`），它的 O(文件数×闭包) 要到
**阶段 C**（T-K30 的正解）才修 ⇒ "第二次显著更快"要挑**单入口 / 单模块**路径来量，
否则量到的是尚未修的病灶（`e2-plan.md:185`）。

**`grade` 也写产物，不是新增的副作用类别** ✓：今天 `check`（= `grade` 的判卷通道）、
`course`、`query`、`build`、LSP 五处都走 `store_if_clean` 写**全局**缓存
（`check.rs:82`、`course/mod.rs:429`、`query.rs:174`、`build.rs:137`、
`lsp/lib.rs:278`）⇒ 本契约只改**落盘位置**，不改"哪些命令会写" ✓。

**T-B5 已落地的判据**（`crates/cli/tests/artifacts.rs`，5 条，全部走真进程 + 隔离
`SOKONANODA_CACHE_DIR`）：① 项目 build 后产物在模块根、格式同源、`.gitignore`=`*`、
`meta.json` schema 正确、全局无项目条目、第二次 **hit**；② 单文件不产生产物目录；
③ 逃生门 ⇒ 不建目录 + 退回全局 + 仍能命中；④ `--clean` 两处都清（
`removed == global + project`）且保留元数据；⑤ **跨根不回放**。
**修前判红**：把 `store_at` 改回只写全局 ⇒ ① **exit 101**；抽掉跨根守卫 ⇒ ⑤
**exit 101**；恢复后 5/5 绿 ✓。

**后续判据**（T-B6 补）：
1. 项目条目落在 `<模块根>/.sokonanoda/compiled/`，单文件条目**不在**那里；
2. 同模块连跑两次 `build` ⇒ 第二次 `hit == files`（数字进台账）；
3. `query project` 的 `artifacts.dir/entries/bytes` 与实际目录一致；
4. `build --clean` 后项目目录与全局缓存的条目都被清（`removed == global + project`）；
5. `.gitignore` = `*` ⇒ `git status --porcelain` 为空；
6. `SOKONANODA_NO_PROJECT_ARTIFACTS=1` ⇒ 行为与本文之前**逐字节相同**（逃生门）。

## 8. 别重踩（取自 T-B4 取证 C 的既有结论，逐条带出处）
* **影子彩排**（`SOKO_SHADOW_CHECK=1`）是实验工具、恒 `一致=false` ⇒ 不要拿它当安全网
  （`E2-HANDOVER.md:113,120-122`）；
* **`&ArenaRef` 存进 `ExportFile` 走不通**（`stumpalo::ArenaRef` 非 Send/Sync，而
  `ExportFile: Sync` 是并行检查的硬要求）⇒ T-D11 的"前缀环境持久化"**形态未定**，
  本文只预留 `prefix/` 目录名，**不在 0.67.0 承诺二进制格式** ✓
  （`vscode-editor-feedback-plan.md:1664-1670`；`architecture.md:580`）；
* **独立环境 `add_declar` 会改写共享 `decl_idx` 槽位**（T-K12c 的死因）
  ⇒ 任何"导入已编译环境"的方案都要先过这道墙（`architecture.md:543-545`）；
* **T-K30 在现有 API 下做不到**（`plan_project` 只加载单入口闭包）⇒ 阶段 B 不要顺手
  去改 `build <dir>` 的分组逻辑，那是阶段 C 的活（`e2-plan.md:185,256`）；
* **`--text` / stdin 永不写产物**：占位路径（`root.join("Main.sokonanoda")`）写进产物
  会让 `query project` 与 definition/references 指向**假文件**；`--text` 今天根本不
  进缓存（`compile-cache.md:145-167`）⇒ 本契约维持"不写" ✓。
