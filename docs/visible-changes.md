# 「看得见的变化」清单（批次 N 收尾 · 2026-09-26 专项）

> **这是什么** ✓：用户 2026-09-26 专项（记法 × 隐式参数 × 产品交互）的**验收面**——
> 每条 = **用户报的什么 → 屏幕上多/少什么 → 哪一层的哪条断言咬住它**。
> 规格登记在 `REQUIREMENTS.md` §9.5；缺口台账 `docs/gaps/ledger.jsonl`
> （G-43 / G-69 / G-70 = 隐式参数那三条前置，均已 `fixed` ✓）。
> ⚠ **够不到的面不假装** ✓：终端 stdout、状态栏、webview DOM 不是真宿主 e2e 能断言的 ⇒
> 钉「**载荷到达**」那一层（stub 宿主 / CLI / 前端），并在下表写明它**不在** e2e 层。

| 用户报的 | 屏幕上多/少什么 | 判据（断言名 · 层） |
|---|---|---|
| ① 终端每秒刷 `build.tick` · 「build 没有反应」 | **机器通道不变**：`--json` 的 stdout 心跳契约一字不动（终端不发、管道发）✓；**人看的通道改走 stderr**：**逐文件**一行 `… 42/240 · <相对路径>`（C1，旧的是每 10% ⇒ 每 24 个才一条 ✗）+ 人话心跳 `… still building (12s)`（≥1s，真事件顶掉）；`SOKO_BUILD_TICK_MS=1` 强制、`SOKO_BUILD_NO_TICK=1` 逃生门 | `cli_build_heartbeat_is_off_unless_asked_for`（`crates/cli/tests/cli.rs` · 四态含反向）+ **`scripts/check-progress-cadence.py`**（N=120 ⇒ 人看 ≥120 行；拿 C1 之前的二进制跑**必须判红** = 10 行 ✓，`--selftest` 反向） |
| ② Rebuild 面板恒 `0%` · 无参 rebuild 是假重编 | 进度**边编边走**（`build.progress` 每编完一个文件就报；`build.file` 的确定性不变）；**C2**：无参 `clean`/`rebuild` 也按**当前目录**解模块根、**两处都清**（旧的无参只清全局 ⇒ 紧接着的预热全 `hit` ✗） | e2e「Rebuild shows a non-zero percent while it is still running (②)」（**真宿主**）+ `cli_build_reports_file_progress_while_it_compiles`（CLI）+ **`a_no_argument_rebuild_also_clears_the_module_root_artifacts`**（`crates/cli/tests/artifacts.rs` · 反向验证已实跑：撤掉默认 ⇒ `clean.project == 0` 判红 ✓） |
| ③ Infoview 版本戳陈旧 | 版本戳**每次写产物刷新**，裸版本号补标签（「由编译器 X 写入」/「服务器」） | `editor/vscode/test-project-first-screen.js`（stub 宿主，已进 `npm run test:unit`）+ 前端判据（两层都反向验证过） |
| Q1 大项目编译超时 | `sokonanoda.build.timeoutMs`（默认 300000，**0 = 不限制**）+ 超时消息写明去哪改 | `editor/vscode/test-extension-host.js`（配置读取 + 传参 · stub 宿主） |
| Q2 没装 CLI | `sokonanoda.installCli`（**自带即装**、离线、不校 SHA256） | `test-extension-host.js`（命令注册）+ **值**判据：跑 `--version` 与插件版本比 |
| K1 改一行等很久 | 编辑后热编译变快（**前缀复用默认开**） | e2e「rewriting an unchanged project unit does not recompile」·「editing a dependency refreshes the open unit once」（**真宿主**）+ `crates/cli/tests/judge_env_reuse.rs`（两态等价） |
| **隐式参数**（B2/B3） | 课标库的前导类型参数**不用再手写**：`a ∈ A` · `f '' A` · `A ⊆ B` · `A ∩ B` · `(∅) x` | front 单测 **6 条**（`crates/front/src/compile/tests.rs`，逐条**反向验证** ✓）+ 课程门禁 **43 目标 · 377 checked · 99 open · 0 判负** |
| 记法铺面（A1–A5）· prelude 能不能改 | `->` 折成 `→` · 集合字面量显示 `{a}` · F12 跳 `{a}` / prelude / 记法声明 · 目标行不漏折；**E1（2026-10-08）**：prelude 的真相搬到 `prelude/{Eq,L1,Quot}.sokonanoda`（Rust 里只剩 `include_str!`）⇒ **改文件真的会改行为** ✓、F12 落点是**仓库里那份**（不再是缓存副本 ✗） | e2e「Infoview 的 ⊢ 用记法箭头 →」「Infoview 里集合字面量显示成 {a}」「go to definition on a `{a}` set literal lands on `Set.singleton`」「go to definition on a prelude name lands in the prelude source」「hover 的类型文本折成记法（T-U12 面 #3）」（**真宿主 36/36** ✓）+ E1：`the_three_source_files_are_the_truth_and_rust_holds_no_prelude_text`（三段拼接 == 视图 == `prelude_source()`；Rust 里哨兵 0 命中）+ **反向验证**（把 `prelude/L1.sokonanoda` 的 `True` 改名 ⇒ `True.intro` 探针判红 ✓）+ `tests::navigation::goto_definition_on_a_builtin_notation_lands_in_the_prelude` |

## 怎么跑（每一列一条命令）

```bash
SOKO_VSCODE_TEST_VERSION=1.138.0 scripts/vscode-e2e.sh   # 真宿主那一列 → docs/e2e/ledger.jsonl
node editor/vscode/test-extension-host.js                # stub 宿主（Q1/Q2/③）
cargo test -p sokonanoda-cli -p sokonanoda-front         # CLI / front 那一列
python3 courses/set-theory/tools/check.py                # 课程门禁（隐式参数那一行）
```

**遗留（如实记）**：③ 与 Q1/Q2 只有 **stub 宿主 + 值**判据（webview DOM 与状态栏在真宿主里
够不到 ⇒ 按 §9.2 的边界钉"载荷到达" ✓）；隐式参数那行的「**数量级下降**」只做到
「**不再必须写**」——点名叫法仍有一批带 `soko:notation-ok` 标记的调用点，逐站点迁移是后续工作；
`by` 块里对**隐式化之后的课标库**做 `cases`（`f '' (f ⁻¹' C)` 一类派生的假设）会撞**判定缝**
⇒ 台账 **G-71**（复现件 `docs/gaps/repro/G71-tactic-context-pp-form-not-rereadable.sokonanoda`，
自包含、判红 = 缺口仍在 ✓）。三条都详见 `docs/design/e2-plan.md` 的 T-N13 as-built「遗留」。
