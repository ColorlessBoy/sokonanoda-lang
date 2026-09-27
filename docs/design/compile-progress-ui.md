# 编译进度的 UI 方案（E23 / E29）

> 2026-09-27。用户要求：「**调查一下，找个能反馈编译进度的 UI 方案**」。
> 下面是查官方指南 + 同类扩展**生产源码**后的结论（含出处 ✓，不是凭印象 ✗）。

## 1. VS Code 给的原生进度 UI（**官方立场是"两级"，不是二选一**）

官方 *Extension Guidelines* 原文：
> 「需要**低调的后台进度** ⇒ 用**状态栏 item + loading 图标**（可加 spin 动画）。
> **需要吸引用户注意** ⇒ 升级成**进度通知（notification）**。」

**⭐ 关键发现 · `location` 的真实类型是 `ProgressLocation | { viewId: string }`**
出处：**redhat 的 Java 扩展生产源码** `vscode-java-debug/src/progressImpl.ts`：
```ts
private _progressLocation: ProgressLocation | { viewId: string };
window.withProgress({ location: this._progressLocation,
  title: this._jobName ? `[${this._jobName}](command:${STATUS_COMMAND})` : undefined,
  cancellable: this._cancellable }, (progress, token) => {
  progress.report({ message: this._message, increment: this._increment }); ... });
```
三点可借用：
1. **`{viewId}` 能把进度条挂到指定视图的标题栏** —— 这就是「**在 Infoview 里显示进度**」的
   **原生做法，零自绘** ✓；
2. **`title` 里能嵌命令链接**（`[名字](command:xxx)`）⇒ 点标题跳详细状态 ✓；
3. 它还提供 `silentNotification` 开关，把通知降级成状态栏 ✓。

**对标 Lean 4（最相关）**：① `LeanTaskGutter` **两级橙条** —— 右侧=整文件进度、
左侧=当前可见行进度，且**随处理推进而缩短**（我们的 P4 只有**开/关**，是**静态的** ✗）；
② `lean4.project.build` 带进度跟踪；③ 状态栏出现 `1 of 1 problem` 计数。

## 2. 方案：**三通道**（手动 build/rebuild 与 LSP 编译走**同一套**，只是数据源不同）

**通道 1（主角）· 原生真进度条 + 可取消** —— 对 `Build`/`Rebuild` 包 `withProgress`，
**两个落点都上**：
- `ProgressLocation.Notification` ⇒ 右下角**真进度条 + 取消按钮**；
- **`{ viewId: "sokonanoda.infoview" }`** ⇒ **进度条出现在 Infoview 标题栏**（原生渲染）。

```js
withProgress({ location: { viewId: "sokonanoda.infoview" }, title: "编译项目", cancellable: true },
  async (progress, token) => progress.report({ message: "[3/13] lib/Set.sokonanoda", increment: 100/13 }))
```
数据源现成 ✓：`crates/cli/src/build.rs` 逐文件发 **`build.file`**；
`build.summary` 有 `files/compiled/hit/failed`。

**顺带修一个真缺陷** ✗✓：**现在 `runBuild` 完全不能取消**（只能干等 `BUILD_TIMEOUT_MS`）——
`cancellable: true` + `token.onCancellationRequested(() => child.kill())` 就顺手修了 ✓。

> ⚠ **先做 30 分钟探针**：确认 `{viewId}` 对 **WebviewView**（Infoview 是 webview view）是否生效 ——
> Java 扩展用的是 **TreeView**，类型不同 ⇒ **不能想当然** ✗。
> **探针不绿就退回通道 2 + 3，不要硬上** ✗；**探针结果要如实写进提交说明** ✓。

**通道 2 · 状态栏 item**（已有，**扩到 build 路径**）
`$(sync~spin) Sokonanoda: 3/13 · lib/Set.sokonanoda`，**加 `command`**（点击开输出面板）+
**加 `tooltip`** 放明细。依据：官方指南「后台低调进度用状态栏」✓。

**通道 3 · Infoview 三行进度区（= E29）**
复用现有自绘链路（P1–P4），把 `build.file` 事件流接进来；
节流复用 `sokonanoda.progress.throttleMs`（默认 250），**`begin`/`end` 不节流** ✓。

**通道 4（可选、本轮不做）· Task 化**：注册成 VS Code Task ⇒ 免费得终止按钮 / 退出码 /
`ProblemMatcher` → Problems 面板。但**不在 Infoview 里**、属另一套 UX ⇒ 记为**后续增强** ✓。

**顺带**：输出面板**不该是唯一出口、也不该只滚 JSON** ✗ —— 把 `build --json` 的 stdout
渲染成人话行（`[3/13] lib/Set.sokonanoda ✓ 12ms`），原始 JSON 留给手动 `sokonanoda build --json` ✓。

## 3. 反假绿纪律（**三条，判据里必须写**）

1. **三处同时、数字同一份** ✓；**反向验证：撤掉任一通道 ⇒ 必须判红** ✓。
2. ⚠ **必须断言"中间态发生过"**：**只断言 `begin`/`end` 出现 ⇒ 判红** ✗；
   要断言**至少一次 `increment > 0` 的 `report`** ✓
   （**只判首尾 = 现在 LSP 路径的形态，等于没进度** ✗）。
3. ⚠ **不许**拿「输出面板有内容」当「有进度」的证据 ✗ —— 同型假绿在
   `editor/vscode/src/test/extension.test.ts:1083` 有过一次
   （`revealRange` 把光标停在 range 末尾 ⇒ 被当成"跳转成功" ✗）。

## 4. 实施顺序（**探针优先**）

**1** 探针（`{viewId}` × **WebviewView**）→ **2** 通道 1（通知 + viewId 原生进度条 + **可取消**）
→ **3** 通道 2（状态栏计数 + 可点）→ **4** 通道 3 / **E29** → **5** 输出面板 JSON→人话
→ **6**（可选）Task 化。

**归属 v0.74.0**（与 E22/E23 同版，同一条 build 链路 ✓）。

## 5. As-built（E23/E29，2026-09-27）—— **做了一半的 `{viewId}`，如实记账**

**做了什么**：通道 **1（`ProgressLocation.Notification`，`cancellable: true`）** +
通道 **2（状态栏逐文件 `3/13 文件 · 文件名`）** + 通道 **3（Infoview 三行进度区）** +
概览尺/整行背景（`setCompileDecorations`）+ 输出面板 JSON→人话。数据源 = CLI 的
`build.begin`（**E23 新加的 additive 事件**，带总数 ⇒ 才报得出 `3/13`；`docs/protocol.md`
已同步）+ `build.file` + `build.summary`，**流式**消费（`runBuildProcess` 的
`onLine`，行缓冲跨 `data` 块）。

**⚠ 没做 `{viewId: "sokonanoda.infoview"}`**（设计 §2 通道 1 的第二个落点）——
**探针没做**：它的效果是"Infoview **标题栏**出现原生进度条"，而**扩展宿主看不见
webview 所在的那个视图 chrome** ⇒ 写了也**没有判据能咬住** ✗（同 R-1/R-2 的
"数据对了 ≠ 用户看见了"，但这次连"数据"都不可观测）。而 Infoview **面板内**
已经有通道 3 的进度区（同一份数字 ✓）⇒ 标题栏那条是**冗余的第二块**，先不做。
**要做的条件**：有一条能断言"标题栏真的出现了进度条"的判据（例如 e2e 侧的可视化
断言）；在那之前它是**已知未做项**，不是"已做 ✓"。

**判据（三处同一份，撤任一处判红 ✓ 实测）**：`editor/vscode/test-extension-host.js::build
streams per-file progress to the status bar and the Infoview (E23)` —— ① 状态栏逐帧
`0/3 → 1/3 → 2/3 → 3/3`；② Infoview 载荷同序同数（百分比 0/33/67/100）；③ 原生
`report` 里**至少一次 `increment > 0`**（只报首尾 = 没进度 ✗，设计 §3 第 ② 条）；
④ 取消令牌真的 kill 子进程。**反向验证**：拿掉逐文件更新 ⇒ ①红（只剩 `0/3`）；
只拿掉 Infoview 那一推 ⇒ ②红（= E29 的"只留状态栏必须判红" ✓）。
