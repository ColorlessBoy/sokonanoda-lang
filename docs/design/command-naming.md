# 设计：VS Code 命令名标准化（R-4，2026-09-24）

> **用户原话**（`REQUIREMENTS.md` §9 R-4）：
>
> > `vscode` 关于 `sokonanoda` 的命令都应该标准化一点，名字统一一点，
> > `Sokonanoda: <命令>(说明)`，每个开头都大写：`Sokonanoda: Infoview (目标面板)`
> > `Sokonanoda: Restart Server(重启服务器)` 啥的。

**目标规则**：命令面板里看到的每一行都是

```
Sokonanoda: <Command> (说明)
```

* **前缀固定** `Sokonanoda: `（大写 S；不是 `sokonanoda:`、不是 `doctor:`）；
* `<Command>` 是**命令词**、**首字母大写**（`Infoview` / `Restart Server` / `Build`）；
* 括号里是**中文说明**（全角括号或半角括号皆可，但**同一个仓库只用一种** ⇒ 统一**半角 `( )`**）。

本文是 **T-B1 的交付物**：先把现状盘清（含两条会被改动碰到的静态契约），
T-B2 照"目标"两列改，T-B3 走四份同步。

---

## 1. 现状盘点（`editor/vscode/package.json` 的 `contributes.commands`，共 15 条）

**关键事实：现状是两套机制混用** ✗ —— 9 条用 `category: "sokonanoda"`（VS Code 自动把
面板行渲染成 `sokonanoda: <title>`），另 6 条把前缀**写进 `title`**、`category` 为空。

| # | command | 现状 `title` | 现状 `category` | 命令面板实际显示（`category: title`） | 目标 `category` | 目标 `title` |
|---|---|---|---|---|---|---|
| 1 | `sokonanoda.showStatus` | `sokonanoda: show exercise status` | — | `sokonanoda: show exercise status` | `Sokonanoda` | `Show Exercise Status (练习状态)` |
| 2 | `sokonanoda.status` | `sokonanoda: show declaration status` | — | `sokonanoda: show declaration status` | `Sokonanoda` | `Show Declaration Status (声明状态)` |
| 3 | `sokonanoda.nextHole` | `sokonanoda: go to next hole` | — | `sokonanoda: go to next hole` | `Sokonanoda` | `Next Hole (下一个洞)` |
| 4 | `sokonanoda.previousHole` | `sokonanoda: go to previous hole` | — | `sokonanoda: go to previous hole` | `Sokonanoda` | `Previous Hole (上一个洞)` |
| 5 | `sokonanoda.goals.refresh` | `sokonanoda: refresh exercise panel` | — | `sokonanoda: refresh exercise panel` | `Sokonanoda` | `Refresh Exercise Panel (刷新练习面板)` |
| 6 | `sokonanoda.courseRefresh` | `sokonanoda: 刷新课程地图` | — | `sokonanoda: 刷新课程地图` | `Sokonanoda` | `Refresh Course Map (刷新课程地图)` |
| 7 | `sokonanoda.revealRange` | `sokonanoda: reveal range` | — | `sokonanoda: reveal range` | `Sokonanoda` | `Reveal Range (跳到该范围)` |
| 8 | `sokonanoda.revealHint` | `揭示下一条提示` | `sokonanoda` | `sokonanoda: 揭示下一条提示` | `Sokonanoda` | `Reveal Hint (揭示下一条提示)` |
| 9 | `sokonanoda.restartServer` | `restart server` | `sokonanoda` | `sokonanoda: restart server` | `Sokonanoda` | `Restart Server (重启服务器)` |
| 10 | `sokonanoda.openInfoview` | `sokonanoda: 打开目标面板 (Infoview)` | `sokonanoda` | `sokonanoda: sokonanoda: 打开目标面板 (Infoview)` ✗ **前缀双写** | `Sokonanoda` | `Infoview (目标面板)` |
| 11 | `sokonanoda.doctor` | `doctor: 诊断服务器与版本` | `sokonanoda` | `sokonanoda: doctor: 诊断服务器与版本` ✗ | `Sokonanoda` | `Doctor (诊断服务器与版本)` |
| 12 | `sokonanoda.build` | `build（编译当前文件/工作区，预热缓存）` | `sokonanoda` | `sokonanoda: build（编译当前文件/工作区，预热缓存）` | `Sokonanoda` | `Build (编译当前文件/工作区，预热缓存)` |
| 13 | `sokonanoda.rebuild` | `rebuild（清空编译缓存后重编译）` | `sokonanoda` | `sokonanoda: rebuild（清空编译缓存后重编译）` | `Sokonanoda` | `Rebuild (清空编译缓存后重编译)` |
| 14 | `sokonanoda.project.refresh` | `sokonanoda: refresh project view` | — | `sokonanoda: refresh project view` | `Sokonanoda` | `Refresh Project View (刷新项目视图)` |
| 15 | `sokonanoda.input.replaceAbbreviation` | `替换记法缩写（\and → ∧）` | `sokonanoda` | `sokonanoda: 替换记法缩写（\and → ∧）` | `Sokonanoda` | `Replace Notation Abbreviation (替换记法缩写：\and → ∧)` |

**盘点发现的三类毛病**：
1. **前缀双写**（#10/#11）：`category` 已经提供 `sokonanoda:`，标题里又写一遍
   （`sokonanoda: 打开目标面板 (Infoview)`、`doctor: 诊断服务器与版本`）
   ⇒ 面板里是 `sokonanoda: sokonanoda: …` / `sokonanoda: doctor: …` ✗；
2. **语言混用**（#1–#7、#14 全英文；#6、#8、#15 全中文）——R-4 要的是
   "英文命令词 + 中文说明"；
3. **括号不统一**（#12/#13/#15 用全角 `（）`，#10 用半角 `( )`）⇒ 统一半角。

## 2. 机制取舍：`category` 还是把前缀写进 `title`

| | (A) **`category: "Sokonanoda"`**（推荐） | (B) 前缀写进 `title` |
|---|---|---|
| 面板显示 | `Sokonanoda: <title>` ✓（VS Code 用 `category: title` 渲染） | 与 `title` 逐字相同 ✓ |
| 分组 | 命令面板里**按类别分组**、可过滤 ✓ | 不分组 ✗ |
| 前缀写几处 | **一处**（15 条 `category`） | 15 条 `title` 各写一遍 |
| 视图标题栏按钮的 tooltip | 只显示 `title`（**不带前缀**）—— 更短 | 带前缀 |
| 现状改动量 | 9 条已经是对的（只改大小写 `s`→`S`）+ 6 条去掉 title 里的前缀 | 9 条要去掉 `category`，15 条 title 都要改 |

**推荐 (A)**：它是 VS Code 的原生机制（`category` 就是为这个存在的），前缀只有一处、
面板还会分组；代价是"视图标题栏按钮的 tooltip 不带前缀"——那正是 tooltip 该有的样子
（短）。本表按 (A) 给出"目标"两列；若你更想要 (B)，只需把 §1 表里的 `category` 列留空、
把 `Sokonanoda: ` 加到每条 `title` 前面即可（其余结论不变）。

## 3. 会被这次改名碰到的静态契约（T-B2 必须先看这里）

`crates/cli/tests/extension.rs`：

| 测试 | 断言 | 改名后要做什么 |
|---|---|---|
| `build_and_rebuild_commands_warm_the_compile_cache` | `title.contains("build")` / `contains("rebuild")`（**小写、大小写敏感**） | 目标标题是 `Build (…)`/`Rebuild (…)` ⇒ 断言要改成**大小写不敏感**（`to_lowercase().contains("build")`），否则必红 ✗ |
| `manifest_declares_commands_that_extension_registers` 等 4 处 | 只用 `command` **id**（不许改 id） | 无需改：**id 一个都不动**（改名只动显示名） |
| 键位表（`keybindings`） | 指向已声明 id | 无需改 |

**红线**：只改**显示名**（`title`/`category`）——`command` id 是**协议**（键位、菜单、
`executeCommand`、文档、技能都在用），一条都不许动 ✓。

## 4. 判据（T-B1 自己怎么算做完）

1. **表覆盖全部命令**：`crates/cli/tests/extension.rs::command_naming_inventory_covers_every_contributed_command`
   —— 把 `contributes.commands` 的 id 集合与本文 §1 表第一列的 id 集合做**双向相等**断言
   （少了 = 有命令没盘点 ✗；多了 = 表里写了不存在的命令 ✗）；
2. **每条都有目标名**：表里 `目标 title` 列非空（人读 + 上面的测试保证行数一致）；
3. **红线写下来了**：§3 明确"id 不动、只动显示名"。

## 5. 交给 T-B2 / T-B3

**T-B2**：按 §1 的"目标"两列改 `editor/vscode/package.json`；同步把 §3 的
`build`/`rebuild` needle 断言改成大小写不敏感；跑
`cargo test -p sokonanoda-cli --test extension` + `node editor/vscode/test-extension-host.js`
（stub 宿主里有按标题断言的用例）+ 真宿主 e2e 的相关用例。

**T-B3**（四份同步）：`editor/vscode/README.md`（命令名出现处）·
`editor/vscode/CHANGELOG.md`（一个 Added/Changed 条目）· `skills/` 三个技能里
出现的命令名（`sokonanoda-teacher` 的「打开目标面板 (Infoview)」等）·
`AGENTS.md` 的命令表 · 本文（改完把"目标"列合并进"现状"列，保留盘点历史）。
`crates/cli/tests/skill.rs` / `dsh.rs` 不许漂移 ✓。
