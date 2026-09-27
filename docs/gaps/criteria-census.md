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
| 2 | `courses/set-theory/tools/check.py:791-850` | ③ | **`--selftest` 从不调用 `evaluate()`**（唯一调用点在 `run()` 里）⇒ G1/G2/G3/G4/G5 **零负例**；patch 成 raise 仍 PASS | **部分修** `90ab11e`（`evaluate()` **第一次进自检** ✓ —— 先补 **G4** 正/负两条；**G1/G2/G3/G5 仍未补** ✗） |
| 3 | 五个门禁脚本（`--x in argv` 子串判模式） | ④ | **错拼参数被静默忽略、回落全量检查并 exit 0** ⇒「自检没跑，退出码却是绿的」 | **已修 ✓** `acbb88e` |
| 4 | `crates/cli/tests/query.rs:822,940,1033` | ④ | LSP 二进制不存在 ⇒ `eprintln+return` ⇒ **三个 CLI≡LSP 一致性用例整条变绿**（防"两套真相"的唯一端到端守卫可静默消失） | **待修** |
| 5 | `crates/kernel/tests/arena.rs:163-167` | ④ | `LEAN_KERNEL_ARENA` 未设 ⇒ 打印+return ⇒ **accept/reject 语料对拍整层绿** | **待修** |
| 6 | `crates/front/tests/module_batch.rs:186` | ④ | `#[ignore]` 掉**真课程等价性**用例（夹具绿，而同文件另有断言"真课程形状不等价"） | **待修** |
| 7 | `crates/cli/tests/launcher.rs:42-58` | ④ | node 不在 PATH ⇒ 打印+return ⇒ **唯一跑启动器的 5 条用例绿** | **待修** |
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
