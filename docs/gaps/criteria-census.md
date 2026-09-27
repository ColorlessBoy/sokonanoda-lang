# 判据强度普查 · 全量清单（E00）

> 2026-09-27。**方法**：按维度切片、**并行派 5 个只读 subagent** 逐文件审计（不是主会话串行读）。
> 每个 prompt 自带三样：**规则与陷阱指向**（`docs/E2-HANDOVER.md` + `AGENTS.md`）·
> **逐文件列全的清单**（不让子代自己 glob）· **固定输出格式**（`文件:行 | 类 | 弱在哪 | 升级后判据`）。
> 写操作一律留在主会话 ✓（子代只产出清单）。四类口径见 `docs/design/criteria-strength.md` §2。

## 规模与分布

| 片 | 范围 | 文件数 | 条数 | ① / ② / ③ / ④ |
|---|---|---|---|---|
| A1 | `docs/gaps/repro/*.js` | 9 | 14 | 9 / 1 / 4 / 0 |
| A2 | `docs/gaps/repro/*.sh` + 探针 | 29 | 21 | 5 / 3 / 11 / 2 |
| B | `scripts/*lint*.py` + `scripts/audit-*.py` | 6 | 18 | 5 / 1 / 4 / 8 |
| C | `courses/set-theory/tools/check.py` | 1 | 13 | 8 / 0 / 2 / 3 |
| D | `crates/*/tests/*.rs` | 34 | 45 | 25 / 10 / 6 / 4 |
| — | **合计** | **79** | **111** | **52 / 15 / 27 / 17** |

**读法**：④（把"没跑/跳过"当绿）虽然只有 17 条，但**全落在关键守卫上**（见 P0）；
①（只断言非空/非 null）最多，是"账面绿、实际坏"的主力形态。

## P0（必须修 —— 全是"守卫看着在、其实瞎了"或"已复现的假绿"）

| # | 位置 | 类 | 一句话 | 状态 |
|---|---|---|---|---|
| 1 | `scripts/audit-wire-fields.py:107-114` | ④ | 写死行域 `lo=150,hi=234` 把 binder/run 读取点**全排除**（实测在 236/240/241 与 54/55）⇒ 抹 `GoalBinderInfo.ty_runs` 仍印 `NONE ✓` exit 0 —— **全仓唯一能咬 R-1 的守卫** | **已修 ✓** `564918f` |
| 2 | `courses/set-theory/tools/check.py:791-850` | ③ | **`--selftest` 从不调用 `evaluate()`**（唯一调用点在 `run()` 里）⇒ G1/G2/G3/G4/G5 **零负例**；patch 成 raise 仍 PASS | **已修 ✓** `90ab11e` + `b18062b`（`evaluate()` 全面进自检：G4 · G1 · G3×2 · G5 + 两条正控制 ✓；**G2 不在该函数内**、覆盖不到，已注明 ✓。反向验证：`evaluate` 变 no-op ⇒ **一次报 5 条失效** ✓） |
| 3 | 五个门禁脚本（`--x in argv` 子串判模式） | ④ | **错拼参数被静默忽略、回落全量检查并 exit 0** ⇒「自检没跑，退出码却是绿的」 | **已修 ✓** `acbb88e` |
| 4 | `crates/cli/tests/query.rs:822,940,1033` | ④→**降级 P1** | ⚠ **复核后修正**：**不是静默跳过** ✗ —— 源码注释写明「**不静默跳过**，但也别让无关的测试套件红——**打印一条明确的提示**」✓，skip **会打印** ✓；缺的只是「CI 里缺前置 ⇒ 判红」这一层 | **降级**（见文末更正说明） |
| 5 | `crates/kernel/tests/arena.rs:163-167` | ④→**降级 P1** | ⚠ **复核后修正**：skip **会 `eprintln`** ✓（不是静默 ✗），且**同文件已被 T-K02 加固过**（注释：「原来『只收集到 1 条』是**完全静默**的」⇒ 已修 ✓）；缺的同样只是「CI 缺前置 ⇒ 判红」 | **降级** |
| 6 | `crates/front/tests/module_batch.rs:186` | ④→**降级 P1** | ⚠ **复核后修正**：`#[ignore]` **带成文理由** ✓（「整门课 35 个文件、每份报告 MB 量级，跑一次要分钟级；**CI 用夹具那条守回归**，这条用来回答『真实课程里等价性成不成立』」）；且**测试体本来就只拷切片**（N 个真单元）✗ ⇒ 子代建议的「切片进 CI」**它已经在做** | **降级** |
| 7 | `crates/cli/tests/launcher.rs:42-58` | ~~④~~ **判错** | ❌ **复核后推翻**：它有 `assert!(std::env::var_os(CI).is_none(), …)` ⇒ **CI 里缺 node 是硬失败** ✓（不是「绿」✗）；docstring 写明这是**故意的**：「a silent skip would hide the only behavioral pin on the launcher, so **the skip is printed, and under CI it is a hard failure**」✓ | **撤回**（不是缺口 ✓） |
| 8 | `docs/gaps/repro/G39-….js:81,85` | ① | definition/hover **只断言非 null** ⇒ **与 G-37 事故同形**（自跳、答错目标照样绿），而台账已写 `fixed` | **已修 ✓** `8af25ba`（**升级后行为仍对** ⇒ 台账不用改回 open ✓） |
| 9 | `docs/gaps/repro/G38-….js:45` | ③ | `JSON.parse(...).data` 从不查退出码/信封 `ok` ⇒ 解析失败时 `undefined.find` 抛异常 ⇒ node **exit 1** ⇒ 台账 `fixed` 恰好把「行为已变」读成**一致**（`gap.py:220`）⇒ **坏环境静默判绿** | **待修** |
| 10 | `courses/set-theory/tools/check.py:602-610` | ④ | `--only` 下画布被判红时 `continue` 的前提不成立 ⇒ **画布那次判红无人报告**，exit 0（**已实测复现**） | **待修** |
| 11 | `scripts/audit-notation-paths.py:186` | ② | 自检只断言"walk.rs 里 ≥1 处"，而 8 处**全是 render_expr** ⇒ 删掉另两个模式仍 exit 0；且 T-U5 迁完后会**假红** | **已修 ✓** `86ab324`（合成夹具 + 模式集合相等；**改独立字面量**后才咬得住） |
| 12 | `scripts/docs-lint.py:141,198-199,203` | ④ | **零扫描 = 全绿**：`cd scripts && python3 docs-lint.py` ⇒「活文档 0 个 ✓ · 归档 0 个 ✓」exit 0 | **已修 ✓** `2984527`（两条地板：`live==0` + 归档目录缺失） |
| 13 | `scripts/notation-lint.py:415-418` | ④ | `MARKER` 命中**丢掉且不计数**：271 个标记行静默豁免 **569 处**旧写法，门禁却印「零旧写法」、`--json` 的 `total=0` | **部分修** `2984527`（`scanned==0 ⇒ exit 2` + `PRELUDE_CALLS` 用例 ✓；**豁免计数仍未修** ✗） |
| 14 | `scripts/status-lint.py:42-48` | ④ | `except: return 0` 把「取不到 HEAD 基线」与「零增长」混为一谈 ⇒ 判据 ④ 静默失效、仍印「净增 0 ✓」 | **已修 ✓** `2984527`（`growth()` 取不到 ⇒ `None` ⇒ 判红） |
| 15 | `scripts/ci-yml-lint.py:63-73` | ① | `jobs:` 为空时循环 0 次 ⇒ exit 0 —— 而**该工具正是因"0 job、0 秒失败"而生** | **已修 ✓** `b313fef` |
| 16 | `docs/gaps/repro/G37-….js:119` | ① | definition 那半已升级，**hover 那半仍是 `!== 'null'`** —— 同一事故的另一半 | **已修 ✓** `a9def6d` |

## P1（该修，不阻塞）

- **接线级**：`audit-notation-paths --self-test` 只活在 pre-push（可 `--no-verify` 绕）；
  `audit-notation-paths`/`audit-wire-fields` 在**全部 workflow 0 处**；
  `scripts/soko` 的课程门禁那一步**没带 `--selftest`**（`:995`）⇒ 本地 gate 不跑。
- **`audit-wire-fields.py:23`**：`PT = "editor/vscode/project-tree.js"` **从不使用** ⇒ 7 处 wire 读取永远不被扫。
- **`notation-lint.py:438-447`**：7 个用例全打在 `scan_text` 上，`PRELUDE_CALLS`（18 条）**零覆盖**。
- **`status-lint.py:95`**：禁词只覆盖 1/7（删其余 6 个仍 7/7）；`growth()` 无自检。
- **`docs-lint.py:245…320`**：9 条自检全是 `any(...)`=≥1、不绑夹具（LESSONS.md 探针失效、别的文件替它红，仍 9/9）。
- **`ci-yml-lint.py:69-70`**：`isinstance(st, dict)` 分支零用例；`okc` 只断条数不断文案。
- ~~**`check.py:612`**：G4 的 `missing` 在 `open_names` 为空时恒空 ⇒ **无「画布必须有具名练习」的非空守卫**
  （旧测试 `crates/cli/tests/course.rs:228-231` 有、check.py 的"无计数版"丢了）。~~ ⇒ **已修 ✓** `90ab11e`（同口径守卫 + 正/负两条自检）
- **`check.py:611-614`**：G4 **只比名字不比命题** ⇒ 同名换命题照样绿。
- ~~**`check.py:785`**：自检只断言子串 `"99"`（差额算成 `+99` 或 `0` 也过）—— 与校准事故同形。~~ ⇒ **已修 ✓** `d9d84e7`（正则取三个数逐一断言 99/0/-99）
- **`check.py:683-685`**：`--bisect` 把「行首没扫到声明」印成「**整份文件判绿**」—— 方向性错误。
- **`check.py:203-208`**：走启动器时"二进制可信"只靠标签**子串黑名单**，而精确探针 `binary_version()` 只在 `--bin` 分支用。
- **`query.rs:169-172`**：`!goal_runs.is_empty()` —— **正是 R-2 事故的形状**（降级成 1 个无 kind run 也绿）。
- **`course_shared.rs:135-138`**：只断言存在 `expr.reduced` 事件，**注释却声称断言了归约值**。
- **`cli/perf_project.rs:145-162`**：只断言 `status.success()`（注释说"必 miss 且结果正确"）⇒ 错误命中旧报告仍绿。
- **`front/perf_project.rs:283-309`**：量了但**零阈值**（同文件 `:224` 有 `worst<2000`）。
- **`lsp_cache.rs:103-107,124`**：`written_entries()>=1` 分不清项目条目到底写没写。

## P2（"自检断言不够具体"，按类计数）

**共 35 条**（D 片 35 条中的其余 + A/B/C 的次要项），形态高度一致：
`is_some()`/`is_string()`/`contains(...)`/`>=1` 只钉"有"，不钉"是哪一个"。
它们**不是活洞**（当前行为是对的），但**一旦漂移不会红**。逐条位置见各片的完整清单
（本文件只固化 P0/P1；P2 建议随各自文件的下一轮改动顺带升级）。

## 明确判为**强**的（不为凑数压低）

- **性能片整体是硬的** ✓：`front/tests/perf.rs` 断言缩放比 <20.0 · 编辑中位数 <50ms · 最坏 <250ms；
  `perf_module_batch.rs` 断言 `reports.len()==files.len()` 且 `batch<=per_entry*1.5`；
  `cli/perf_project.rs` 断言 `warm<cold` · `warm<300ms` · `2×warm<cold`。**只有 P1 里那两条例外**。
- `notation.rs` · `notation_fold.rs` · `namespace.rs` · `judge_batch.rs` · `course.rs` ·
  `course_project.rs` · `skill.rs` · `protocol.rs`（除两点）· `kernel/pretty_printer.rs`（逐例精确 pp 串）·
  `memory_api.rs`（除 `:40`）· `arena.rs`（除跳过）· `lsp_edit_concurrency.rs` · `imports.rs` · `dsh.rs` · `watch.rs`。
- **`G3` 本身不弱** ✓（C 片实测纠正了我的预设）：`decl.checked > 0` 形式上像 ②，但"覆盖该覆盖的"
  由 **G4** 兜着，而 G4 断言**是哪几个名字**（比 ② 强）；G3 不可替代的作用是"解答不能是空文件/全灭"。
  **真正弱的是 G4 的输入集**（见 P1 两条）。
- 反空转前提（`!files.is_empty()` / `scanned>30` / `shapes.is_empty()`）**不算弱判据** ——
  `notation.rs:169` 注释写明那是**防 `0==0` 假绿**的；`#[cfg(unix)]` 也不算 ④。

## 场外发现（**不在本次普查范围内，建议立刻入台账**）

**`query reduce --file` 在含 `#reduce` 的文件上答错值**（D 片实测，可复现）：
```
./target/release/sokonanoda query reduce --file course/shared/Demo.sokonanoda \
    --expr "Nat.succ Nat.zero" --compact      →  {"value":"Nat.zero"}   ← 错（期望 Nat.succ Nat.zero）
# 同一表达式走 --text 同一份文本 ⇒ "1"（正确）；文件不含 #reduce 时正常。
```
⇒ 即「**文件里有 `#reduce` ⇒ `--expr` 被忽略/取到文件自己的第一个 reduced 值**」——
**这是 agent 判卷通道上的错答案**，而 `query.rs:265-278` 对 reduce 的覆盖只有 `--text` + `1+1` 一条
——**正是本文件要找的"判据太弱 = 等于没有判据"的活样本**。

---

## 剩余工作（**交给下一条会话**，2026-09-27 收尾时写）

**当前状态**：普查 **111 条** · **已落地 16 条修复**（各有独立 commit + 先判红 + 反向验证）·
`--selftest` 从「三条门禁没有自检入口」变成 **10/10 绿** ·
⚠ **38 个提交仍未推**（全部本地验证过，**一次 CI 都没跑过** ⇒ 「真绿」尚未被证明 ✗）。

### 一键复核「已落地的都还在」
```bash
python3 scripts/{ci-green,target-hygiene,status-lint,notation-lint,ci-yml-lint,docs-lint,audit-wire-fields}.py --selftest
python3 scripts/audit-notation-paths.py --self-test
python3 scripts/gap.py selftest
python3 courses/set-theory/tools/check.py --selftest
python3 courses/set-theory/tools/check.py          # 必须仍是 36 目标 · 328 checked · 99 open · 0 判负
```

### 仍待修 4 条（按「能不能安全动」排序）

| # | 位置 | 病灶 | 升级方案 | ⚠ 卡点 |
|---|---|---|---|---|
| 1 | `crates/cli/tests/query.rs:822,940,1033` · `arena.rs:163-167` · `module_batch.rs:186` | **降级为 P1** ⚠ —— **复核后修正**：三条**都会打印跳过** ✓、**都有成文理由** ✓（详见文末更正说明）；残余缺口只有「**本地缺前置时测试仍通过**」（CI 侧 `launcher` 已硬失败 ✓） | 可选加固：缺前置**且 `CI` 已设** ⇒ `panic!`；**动手前先读源码** ✓（别照抄本条 ✗） | 验证要 cargo |
| ~~1b~~ | ~~`launcher.rs:42-58`~~ | ❌ **判错、已撤回**（它 CI 里是硬失败 ✓） | — | — |
| 2 | `courses/set-theory/tools/check.py:611-614` | G4 **只比名字不比命题** ⇒ 同名换命题（`theorem prod_fst_mk : True := True.intro`）照样绿 | 让内核在 `decl.checked` 事件里带上**该声明 elaborate 后的类型**（或类型指纹），断言逐字相等 | **跨层**：要改内核/协议 ⇒ 属 v0.79 范围，别在 E00 里顺手做 |
| 3 | `scripts/docs-lint.py`（`--selftest` 的 9 条） | 自检全是 `any(...)` = 「≥1 条」、**不绑夹具** ⇒ 别的文件替它红也照样 9/9 | 每条绑到自己的夹具文件，断言**是哪一条**判据开火 | 编辑量中等；**改门禁判据前先补自检** |
| 4 | `courses/set-theory/tools/check.py:203-208` | 走启动器时「二进制可信」只靠标签**子串黑名单**（`"STALE"/"unknown"`），而精确探针 `binary_version()` 只用在 `--bin` 分支 | 启动器分支也跑 `binary_version()`，要求 `reported == report["version"]` 且 `pin is not None`，否则 exit 2 | 会给课程门禁**加一次子进程调用**（性能/时序），先量再改 |

### 本轮新立的判据模式：**第 ⑤ 类「自指期望」**
见 `docs/design/criteria-strength.md` §2.5（期望值从**被测对象**派生 ⇒ 恒成立 ✗；
与 `G29-….js:97` 的 `budget = 10 * warmOpen + 200` 同型）。
**自测方法**：改坏被测对象，若判据**照样绿** ⇒ **先怀疑期望本身是自指的**，而不是先怀疑反向验证没做对 ✓。
**实例**：修 `audit-notation-paths --self-test` 时，我第一版把夹具与期望**都从 `CALLS` 派生** ⇒
删一个模式时两边一起缩 ⇒ 自检恒过 ✗（**反向验证当场抓到** ✓，见 `86ab324`）。

### 已知但**未修**、也不要顺手修的
· `crates/cli/tests/*` 的 35 条 P2（「断言不够具体」）—— **不是活洞**（当前行为是对的），
  但一旦漂移不会红 ⇒ **随各自文件的下一轮改动顺带升级** ✓，不要为它们开专场 ✗。
· **X1 文档分层**：`docs/PLAN-0.74-0.79.md` **792/800 行** · `docs/design/criteria-strength.md` **136/150 行**
  ⇒ **下一次往这两份里加内容之前，必须先拆一段出去** ✗（`docs-lint` 判据 ③ 会在 801 行判红）。

---

## ⚠ 更正说明（2026-09-27，**我自己复核出来的**）

P0 表的第 **4/5/6/7** 条来自 **D 片 subagent**，我**当初直接写进 P0、没有逐条复核** ✗。
本轮抽这 4 条**去读源码**，结果：

| 原判 | 复核结论 |
|---|---|
| #4 `query.rs:822`「静默跳过」 | **不静默** ✓ —— 注释写明「**不静默跳过**…**打印一条明确的提示**」 |
| #5 `arena.rs:163`「打印+return」 | 说明里就写了「打印」✓，且**同文件已被 T-K02 加固过**（原「只收集到 1 条」才是完全静默的，已修 ✓） |
| #6 `module_batch.rs:186` `#[ignore]` | **带成文理由** ✓；子代建议的「切片进 CI」**它已经在做**（测试体只拷 N 个真单元） |
| #7 `launcher.rs:42` | **判错** ❌ —— 它有 `assert!(CI 未设)`，**CI 里缺 node 是硬失败** ✓ |

**⇒ 四条里 1 条判错、3 条把「有打印的、有理由的跳过」说成了「静默当绿」** ✗。
**残余的真实缺口只有一层**：本地缺前置时**测试仍然通过**（但**跳过会打印** ✓，CI 侧 `launcher` 已硬失败 ✓）
⇒ 应作 **P1**（可选加固：缺前置且 `CI` 已设 ⇒ panic），**不是 P0** ✓。

### 我违犯了哪条纪律（如实记，不粉饰）
`AGENTS.md` §并行与 subagent 纪律第 4 条：**「产出必须验证后才并入 ✓：复跑它给的判据 ✓、
抽查结论 ✓（『错误百出』那次就是没人验 ✗）」**。
我**没有抽查就并入** ✗ —— 把 5 片共 **111 条**照单写进清单，并把 D 片这 4 条直接标成 **P0** ✗。
**抽查 4 条 ⇒ 1 条错、3 条夸大** ⇒ **其余 107 条的可信度同样未知** ✗。

**⇒ 给下一条会话的告诫**：本清单每一条在动手前**都要先读源码确认** ✓。
**已修的那 18 条都是读过源码的** ✓，可信度高于未复核的条目 ✓。
**别把「子代说了」当成「已核实」** ✗ —— 这条我自己就犯了 ✓。

### 抽查续报（2026-09-27，本会话末轮）

上一节说「1 条判错、3 条夸大」⇒ 我又抽了 **D 片的 3 条**（都是具体的"断言形态"指控）去读源码：

| 条目 | 复核 |
|---|---|
| `crates/cli/tests/query.rs:169-172` ① 「只断言 `!goal_runs.is_empty()`，正是 **R-2 事故**的形状」 | **准确** ✓ —— 实测就是 `!is_empty()`；子代的升级建议（断言 `(text,kind)` 含 `("∧","keyword")`）**切题** ✓ |
| `crates/cli/tests/course_shared.rs:135-138` ① 「只断言存在 `expr.reduced`，**注释却声称**『⇒ zero / ⇒ 4』」 | **准确** ✓ —— 注释写「递归定义真的算出来了」，断言只查 `events.contains("expr.reduced")` ✗ |
| `crates/cli/tests/artifacts.rs:348,351` ② 「`entries>=1` / `bytes>0`」 | **准确** ✓ —— 字面如此 ✓ |

**⇒ 修正上一节的结论**（我第一次只看了 4 条 P0）：
**子代对「代码里写的是什么」的陈述基本可靠** ✓（3/3 + 前一节的 #4/#5/#6 都对）；
**错在「定级」** ✗ —— 把**有打印、有成文理由**的跳过叫成「④ 静默当绿」，
以及 **#7 漏看**了 `launcher.rs` **早就有** `assert!(CI 未设)` ✗。
**⇒ 给下一条会话**：本清单的**事实描述**可采信度较高 ✓，
但**「是 P0 还是 P1」必须自己判** ✗（子代倾向把"不完美"一律报成 P0 ✗）。

**另记一条新子模式（= ① 的变体，值得单列）**：
**「注释声称的强度 > 断言的实际强度」** —— `course_shared.rs:135` 的注释写「⇒ zero / ⇒ 4」，
而断言只证明"有 `expr.reduced` 事件" ✗。
它与校准事故 `summary !== 'null'` 同族：**读注释会以为验过了，读断言才知道没验** ✓
⇒ **读测试时先看断言、再看注释** ✓（注释是**意图**，断言才是**判据** ✗）。

**再记一条反面提醒**（免得下一条会话"整文件判弱"✗）：
同一文件常常**强弱混着** ✓ —— 例如 `query.rs:169` 的**紧邻上一行**就是
`assert_eq!(binders, vec!["a", "b", "h"])`（**点名具体值、很强** ✓），
`artifacts.rs:352` 紧接着断言 `compiler == env!("CARGO_PKG_VERSION")`（**具体** ✓）。
⇒ **按行判，别按文件判** ✗。

---

## 变异测试总账（2026-09-27，② 的交付项「报出有几条判据会红」）

**一条判据「会红」的证明 = 它的自检里注入了变异、并断言该变异被报出来。**
本仓现在有 **10 个自检入口**，逐脚本自报的变异数如下（`--selftest` 的 `X/Y` = 「咬得住 X / 共 Y」）：

| 自检入口 | 变异数 |
|---|---|
| `scripts/ci-green.py --selftest` | 6 |
| `scripts/target-hygiene.py --selftest` | 8 |
| `scripts/status-lint.py --selftest` | 8 |
| `scripts/notation-lint.py --selftest` | 8 |
| `scripts/ci-yml-lint.py --selftest` | 9 |
| `scripts/docs-lint.py --selftest` | 9 |
| `scripts/audit-notation-paths.py --self-test` | 3 |
| `scripts/audit-wire-fields.py --selftest` | 2 |
| `scripts/gap.py selftest` | 16 |
| `courses/set-theory/tools/check.py --selftest` | 15 |
| **合计** | **84** |

⇒ **变异测试结论：84 条判据会被证明「变异 ⇒ 判红」** ✓（全部 **exit 0**，即 84/84 都咬得住 ✓）。

⚠ **这 84 条里有一部分是本会话新增的**：`check.py` 的 15 条中 **10 条**是本会话加的
（`evaluate()` 的 7 条 + unit 整数的 3 条，见 `90ab11e` / `b18062b` / `66a424b`）；
另有**不在自检里、但各带一次性反向验证**的 8 条修复（G23/G37×2/G39/行域/拼法陷阱/零扫描×3/
豁免计数/`_rel`/二进制交叉核对 —— 逐条见各 commit 的「反向验证」节 ✓）。

⚠ **诚实的边界**（不把它说满 ✗）：
· 这 **84** 是**自检自称**的数，**不是**我逐条手工复核过的数
  （本会话抽查过 7 条 P0/P2 条目，**事实 6/7 准**，见上面的更正说明）；
· **判据会红 ≠ 判据断言得够具体** —— 一条只断言「非 null」的判据也会「在变异下判红」✓
  （所以 84 这个数**不能**用来证明清单已经升级完 ✗）；
· 它证明的是「**守卫咬得住已知的坏形状**」✓，这正是 `AGENTS.md` 要的那件事 ✓。

---

## ⚠ 推送前必读：`scripts/soko gate` 会因 **G-07** 而红（**不是本会话造成的**）

2026-09-27 实测（`python3 scripts/gap.py check`，跑了 ~10 分钟 —— 它会逐个跑复现件）：

```
1 条与台账不一致 —— 台账是契约：要么修好了（写 fixed_in），要么行为回退了。
G-07  fixed  script  缺口仍在  ← 台账写的是「行为已变」，请更新
      ｜复现件：→ G6 机器可读面（修后预期）：NO
      ｜stderr: 结论：缺口回来了（清单又变回扁平 / v2 字段没发出来 / v1 兼容破了 / 门禁没有 G6）
```
**⇒ exit 1** ⇒ `scripts/soko gate` 的「缺口台账门禁」这一步**会失败**，
CI 的 ledger job 大概率同样红 ✗ —— **推 44 个提交之前必须知道这一点** ✗。

**⚠ 但不是本会话造成的** ✓，证据：
1. **本会话碰过的两条台账条目实测都一致** ✓ ——
   `G-37  open  / 缺口仍在`（我把它的判据升级后**改回 open** ✓）·
   `G-39  fixed / 行为已变`（判据升级后**行为仍对**、台账不用改 ✓）；
2. 既有 commit `42be634` 已记「**与既有不一致，与清理无关**」✓ ⇒ **早于本会话**；
3. 本会话**从未改过 G-07 的复现件或台账行** ✓（`git log origin/main..HEAD -- docs/gaps/ledger.jsonl` 只动过 G-37/G-39 ✓）。

**⇒ 给下一条会话**：推之前**先判 G-07** —— 二选一 ✓：
· 复现件说的「缺口回来了」是**真的** ⇒ 那是**行为回退**，先查根因（别急着改台账 ✗）；
· 是**环境/形状**造成的假红 ⇒ 按 `docs/gaps/ledger.jsonl` 的契约用 `repro_expect` 说明，或修复现件 ✓。
**⚠ 不许**为了让 gate 变绿而**把 G-07 直接改成与复现件一致** ✗ —— 那是"改判据迁就现状"，
正是本会话在门禁里修了一路的那种病 ✗。
