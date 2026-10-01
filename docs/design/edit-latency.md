# 编辑响应延迟（2026-10-01 用户反馈：一闪一闪 / 一卡一卡）

> 本文是**编辑 → 反馈**这条链的**唯一权威**：实测分解、修了什么、**明确不做什么**。
> 症状原话：「在 VS Code 里编辑 .sokonanoda 文件太卡，输入后过很久才有反应
> （诊断/目标面板更新），与 Lean4 不可比」+ 补充四条：①「一闪一闪」②「一卡一卡」
> ③「分不清是插件事件太多还是编译真的慢」④ 参考基准「Lean4 编辑无闪烁、响应 <1s」。

## 1 实测分解（**先诊断，别猜**）

三把独立的尺子，量**同一件事的三段**。命令都在 `docs/PERF.md` 的「怎么重量」一节。

| 段 | 怎么量 | 实测（Apple Silicon，debug LSP，warm 缓存） | 结论 |
| --- | --- | --- | --- |
| **服务端编译** | 真进程 stdio，逐键 `didChange`→`publishDiagnostics` | `playground.sokonanoda`（405 行/29 声明）**0.1ms 中位**，24 键 24 次编译**每次 0ms** | **不是瓶颈** |
| **VS Code 自动发的每编辑请求** | `semanticTokens/full` · `codeLens` · `inlayHint` · `documentSymbol` | 1.6 / 0.2 / 0.6 / 0.2 ms（playground）；3.1 / 0.1 / 2.2 / 0.1 ms（`unit12-solution`） | **不是瓶颈** |
| **扩展自己发的请求** | `soko/goals`（带内核探针）· `soko/project` · `soko/stateAt` | 4.5–5.9 / 0.1–0.7 / ~0.3 ms | **不是瓶颈** |
| **冷开**（进程内首次编译） | 同上，空缓存 | `unit12-solution`（503 行）**7794–8407ms**；第二次 **33.7ms** | 真慢，但**一次性** |
| **服务端事件密度** | 逐键清点 `$/progress` | **2.0 条/键**（24 键 → 48 条：24 `begin` + 24 `end`） | ⚠ **闪烁源** |
| **端到端**（真 VS Code） | `editor.edit` 逐键 → 诊断事件 / 面板内容变化 | **诊断 122ms · 面板 268ms**；8 键亮 **16** 次 | 延迟合格，**闪烁不合格** |

### 1.1 结论：用户问的「插件 vs 编译」，答案是**插件**

编译本身（warm）是**亚毫秒**的；用户感到的「卡」来自**客户端每个键做的一串可见动作**：

```
每敲一个键
├─ 服务端编一次（~0ms）→ 发 1 条 publishDiagnostics
│   └─ 扩展 150ms 去抖 → soko/goals(探针) + soko/project + soko/stateAt
│                        + 目标树 fire + 项目树 refresh + Infoview 重建
└─ 服务端发 $/progress begin + end        ← 立刻生效，**不去抖**
    ├─ setCompileDecorations(true/false)：**整份文档**的背景装饰 + 概览尺
    ├─ 状态栏切成「编译中…」再切回
    └─ Infoview 插一块进度再删掉
```

`begin`/`end` 这一对**没有防抖也没有节流**（P6 只节流 `report`），而服务端**每个键
都编一次** ⇒ **敲一个字，整份文档亮灭一次**。真宿主实测：**8 个键亮 16 次**
（`flicker_frames=16`，`lights_per_keystroke=2.00`）。这就是「一闪一闪」。
`document.getText()`（整份文档拷成字符串）也在这一对里，每键一次。

### 1.2 排除掉的（**有数字，不是"看着没问题"**）

* **hint 阶梯**：`soko/hints` 只在展开「提示」节点时按需请求，**不在击键路径上** ✓。
* **进度节流不够**：`report` 节流本来就是对的（5 条 `report` 只挂 1 个定时器 ✓）；
  漏的是 `begin`/`end` **根本不进节流**。
* **树刷新频率**：`provider.refresh()` 只有诊断事件那一条路（150ms 去抖后一次），
  不是每键 ✓。

## 2 修的刀：P7 展示延迟（2026-10-01）

**改法**（clangd / rust-analyzer / VS Code 自己的 `withProgress` 同款）：`begin` 只
**挂一个定时器**，到点才真的亮；`end` 先到就把定时器撤掉 —— **一次界面都没碰**。
教学规模的编译（~0ms）因此完全无痕；真的慢的编译（冷开 7.8s）照旧亮起 ⇒
P1/P3/P4 的能力**一个不少**。

**闸门放在 `onCompileProgress`（LSP 那条路），不放 `applyProgress`**：
`applyProgress` 还供 `Sokonanoda: Build/Rebuild` 用，那条路**必须**从第一帧就报
进度（E23/P2「rebuild 中途必须看到非 0 百分比」是用户的硬需求）⇒ 加在那儿会
**倒掉真进度** ✗。闪烁是**击键**特有的，闸门就只该拦击键那条路。

配置 `sokonanoda.progress.showDelayMs`（默认 **300**，`0` = 旧行为）。
另修一个真 bug：`compileShown` 是模块级的，重新激活时不清 ⇒ 残留的"已经亮着"
会让编译提示**此后再也不出现**；现在 `activate()` 调 `resetCompileProgressState()`。

### 2.1 前后对比（同一文件、同一操作、同一台机器）

| 量 | 前 | 后 | 判据 |
| --- | --- | --- | --- |
| **闪烁**（8 键亮几次，真 VS Code） | **16** | **0** | e2e `flicker_frames`；断言 `frames < keystrokes` |
| 每键亮灯数 | 2.00 | **0.00** | 同上 |
| 诊断延迟（键 → 诊断事件） | 122ms | 99ms | `perfNote`（只记不断言） |
| 面板延迟（键 → 面板内容变） | 268ms | 263ms | 同上 |
| 整文档装饰/状态栏/进度块的**每键**触碰 | 各 2 次 | **0 次** | stub 宿主 `P7:` 两条用例（确定性） |

**面板那 263ms 拆开**：≈100ms 是 **VS Code 自己的诊断管线**（服务端同夹具实测
0.2ms/键 ⇒ 与我们无关）+ **150ms 是扩展的诊断去抖** + ~15ms 取数与渲染。
两条都在 <1s 内，**不构成"数秒"**；用户感到的"慢"主要是闪烁造成的**卡顿感**。

## 3 明确不做的（**收益小或风险高**，不为改而改）

1. **不缩短 150ms 诊断去抖**。它合并的是"项目模式一次编辑发多份文档诊断"那个
   真问题；缩短只省 ~70ms，却会让项目模式下多花几倍取数。
2. **不改 `$/progress` 的服务端契约**（不为快编译跳过 `begin`/`end`）。服务端**事前
   不知道**这次快不快；"要不要画"是**客户端**的决定（clangd/rust-analyzer 同此）。
3. **不动 `crates/`**。实测编译不是瓶颈（warm 0.2ms/键），动编译管线既无收益，
   又要与并行会话抢 `crates/front/src/compile/*` 的文件占用。
4. **不删概览尺装饰**。它是 P4 的"在哪编"信号，且现在只在**真的慢**时才亮 ⇒
   不再有整份文档的亮灭。删它才是真回归。

## 4 判据（三层，缺一层就是洞）

| 层 | 判据 | 咬什么 |
| --- | --- | --- |
| stub 宿主（**计数**，噪声免疫） | `P7: a compile that finishes inside the show delay never touches the UI` | 快编译碰了装饰/状态栏/Infoview 就红；`end` 漏撤定时器也红 |
| stub 宿主 | `P7: a compile that outlives the show delay still lights up, and lights up once` | 把 P1/P3/P4 改没了就红；连续慢编译"亮—灭—亮"也红 |
| **真 VS Code**（用户看得见的） | `edit responsiveness: last keystroke reaches diagnostics and the goal panel` | 末键没引起诊断/面板更新就红；`flicker_frames ≥ keystrokes` 就红 |

**反向验证**：e2e 那条在修前实测 **红**（`8 个键亮了 16 次`），修后绿 —— 咬得住 ✓。
