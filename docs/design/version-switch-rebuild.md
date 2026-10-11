# 版本切换 ⇒ 自动全量预热（VS Code 扩展侧）

> 2026-10-11 用户要求，**最小改动**：
> 「**最小编译策略**平时是可以的，但是请增加版本检测功能：如果是项目，并且项目 cache 的
> 编译器是老版本，相比当前 vscode 插件（或者 cli 对应的编译器版本）就不能用这个最小编译
> 策略，而是 rebuild 策略。」
> 同日补充：「cli 不用自动，vscode 才需要」；「面板 rebuild 按钮也不需要了」。
>
> 结论：**只改扩展**（`editor/vscode/`），CLI 行为一个字不动；**没有**新设置项、
> **没有**新协议字段、**没有**新命令。

## 1. 现状（改前的行为，逐条实测）

| 环节 | 改前 |
|---|---|
| 打开一份项目文件（有 `import`） | LSP 只编**这一份的 import 闭包**（最小策略）——`crates/lsp/src/lib.rs` 的 `set_text`：`project_cache::load_at` 未命中 ⇒ 编闭包 |
| 命中判定 | 条目键里带**编译器版本 + 构建戳**（`compile::cache::key_parts`）⇒ 升级后旧条目**一条都命中不了** |
| 整项目预热 | 只有**手动**两条路：命令面板 `Sokonanoda: Build` / `Rebuild`（`alt+b` / `alt+shift+b`），或 CLI `sokonanoda build <目录>` |
| 面板提示 | 「产物：N 条 · X · 由编译器 A（n）· B（m）写入 · 当前编译器 C ⇒ 建议 Rebuild」——**只是提示**，要用户自己动手 |

⇒ 用户升级插件后第一次打开文件：闭包白编一遍、面板一直提示 Rebuild、照做还得自己找快捷键。

## 2. 契约（改后）

1. **触发点**：`loadProject()` 拿到 `soko/project` 的答案之后（切文档 / 诊断事件 / 激活
   都会走它）——**模块根就在答案里**（`project.root`），零额外取数。
2. **判据**：`<模块根>/.sokonanoda/compiled/` 里**有任一条目是别的编译器版本写的** ⇒ 旧。
   - 依据是**条目文件名**（`<编译器>+<构建戳>+<键>.json`，`crates/front/src/project/cache.rs::entry_stem`），
     **不是** `meta.json.compiler` —— 后者会被任何一次写入刷成当前版本（G-106 的谎），
     照它判会漏判"247 条旧 + 1 条新"这种现场。
   - **只看版本，不看构建戳**（与面板 `artifacts.stale` 的**有意差异**）：同版本换构建是
     G-112，不是"版本切换"⇒ 不触发。
3. **动作**：跑 `sokonanoda rebuild --json <模块根>` —— 与 `alt+shift+b` **同一条命令、
   同一个目标**（模块根，不是活动文件、不是工作区根）。
   - 为什么是 `rebuild`（带 clean）而不是 `build`：用户要的就是"Rebuild 策略"；且版本
     切换后**整个全局缓存**（所有项目）都作废了，clean 是顺带清理，不是额外损失。
4. **对齐闸**（G-112 的防线）：写产物的是 **CLI**、读产物的是 **LSP**。跑一次
   `sokonanoda --version`，取出版本 token 与**服务器自述**（`soko/version`）比；
   不一致 ⇒ **不动手**（否则每次开档都判定成旧版本 ⇒ 反复 22 秒全量预热 ✗）。
5. **每个模块根、每个会话只自动跑一次**（`autoRebuiltRoots`）。没有它就是死循环级事故：
   预热引发诊断事件 ⇒ `loadProject` ⇒ 再判定（预热期间旧条目仍在）⇒ 再预热 ✗。
6. **纪律**：不阻塞（`void`，绝不拖慢"打开文档 ⇒ 看到目标"）、不弹窗、不报错
   （进度进 `sokonanoda build` 输出通道；失败只是没预热，`sokonanoda: doctor` 能查）。

## 3. 落点

| 文件 | 改了什么 |
|---|---|
| `editor/vscode/server.js` | 新增纯函数 `entryWrittenByOtherVersion` + `projectArtifactsWrittenByOtherVersion`（只读一次目录，**不创建、不写入**）；导出两条 |
| `editor/vscode/extension.js` | `loadProject()` 尾部 `void autoRebuildOnVersionChange(answer)`；新增该函数 + `serverCompilerVersion()`（`soko/version` 只问一次、重启服务器时失效）+ `autoRebuiltRoots` |
| `editor/vscode/test-server.js` | 检测的单元判据（2 条，含"同版本不同戳不算"） |
| `editor/vscode/test-extension-host.js` | 宿主层 4 条：旧版本⇒起 `rebuild --json <模块根>` / 当前版本⇒一次都不起 / 每根只一次 / CLI skew⇒不起 |

**没有改**：`crates/**`（内核、front、CLI、LSP 一个字没动）、协议、设置项、命令表、webview。

## 4. 判据（都实跑过，含反向验证）

| 判据 | 读数 |
|---|---|
| `node editor/vscode/test-server.js` | 20/20 ✓ |
| `node editor/vscode/test-extension-host.js` | 63/63 ✓ |
| `node editor/vscode/test-webview.js` | 35/35 ✓ |
| `cargo test -p sokonanoda-cli --test extension` | 41/41 ✓ |
| **反向验证 ①**：撤掉检测（恒 `false`） | `test-server.js` 当场判红 ✓（19/20） |
| **反向验证 ②**：撤掉对齐闸 | `test-extension-host.js` 当场判红 ✓（62/63） |

**代价（实测，release，课程 251 个文件）**：一次全量预热 **22.3 秒**
（`target/release/sokonanoda rebuild --json <课程副本>` ⇒ `files 251 · compiled 248 ·
failed 2 · hit 1`；那 2 个 failed 是 `gaps/` 里"故意必须判红"的负例）。

## 5. 边界（如实记）

- **同版本、不同构建**（debug/release 各写一份，G-112）**不触发** —— 用户 2026-10-11
  明确只要"版本切换"这一条；那条的候选修法在台账 G-112。
- **只覆盖编辑器**：CLI 侧保持"你让它 build 才 build"（用户同日拍板）。
- **不是"把旧条目删干净"**：`rebuild` 会清该模块根 + 全局缓存；但若用户在预热**中途
  取消**，目录会停在混合状态（面板照实显示），下次开档**不再自动**重试（每根一次）
  ⇒ 这时用手动 `Rebuild`。这是有意的取舍：宁可少打扰，也不让它在后台反复跑 22 秒。