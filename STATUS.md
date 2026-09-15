# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-14（第五十六轮：VS Code webview Infoview goal 面板；0.30.0）
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-14，第五十六轮：VS Code webview Infoview）

> 续 TODO 清账（顺序 R57→R56→R55）：按
> `docs/design/webview-infoview.md` 落地方案 B（Lean Infoview 式 webview）。

1. **view/命令**：新增 `sokonanoda.infoview`（`type: webview`，与练习/课程
   并列）+ `sokonanoda.openInfoview`；树的「当前光标处」组保留为默认与兜底
   （webview 不可用时功能零回归）。
2. **协议**（`protocol:1`）：宿主→webview `state`（`soko/stateAt`）/`decls`
   （仅诊断·切文件）/`server`（`soko/version`）/`theme`；webview→宿主
   `ready`/`reveal`/`focusExercise`；按文档 `version` 丢弃过期 `state`。
3. **安全**：CSP `default-src 'none'` + 每次随机 nonce、`localResourceRoots`
   限 media；渲染只用 `textContent`（契约负断言禁 `innerHTML`/远程 URL/eval）。
4. **性能**（吸取 goal-list §2.4 教训）：光标移动只发轻量 `state`（去抖
   200ms），绝不触发 `soko/goals` 或整树重建；provider 缓存最后快照。
5. **测试**：`crates/cli/tests/extension.rs` 静态契约（view/命令一致、资源与
   CSP nonce、负断言）；`extension.test.js` 集成 smoke；`node test-server.js`
   18/18。
6. **验收**：`sokonanoda gate` PASS；版本 0.29.0 → **0.30.0**（新 view+命令）。

## 本轮进度（2026-09-14，第五十五轮：编译器服务事件流）

> 续 TODO 清账（用户确认顺序 R57→R56→R55）：按
> `docs/design/compiler-service-events.md` 落地 watch 服务事件流。

1. **规范名**：watch 开场事件 `file.changed` → **`file.didChange`**（payload
   不变，新增稳定 `file` 字段）；`file.changed` 保留一个 minor 的弃用别名
   （`WATCH_VOCABULARY` 接受、不再发射）。
2. **握手**：stdout 第一行恒为 `service.hello {protocol:1, engine, pid}`
   （对齐 LSP `soko/version`；原纯文本 banner 移出 stdout）。
3. **作用域**：`watch <file>` / `--doc <file>` / `--workspace <root>`（互斥）；
   workspace 递归发现 `*.sokonanoda`，每文件一个 `Session` 与独立版本号、
   事件带 `file`、跨文件无全序。
4. **背压**：每文件有界缓冲（64），溢出合并为最新版本并标
   `recompiled_from: 0`（协议注明可全量重同步）。
5. **测试**：`crates/cli/tests/watch.rs` 6 项（握手/规范名/`--doc`/workspace
   独立版本/闭词汇 + `file`/protocol.md 覆盖）+ watch.rs 单测（溢出合并）；
   CLI 套件 105 pass、`skill.rs` conformance 绿。
6. **文档**：`protocol.md` watch 小节 + `TESTING.md` + teacher `events.md` +
   `help.rs` 同步。
7. **验收**：`sokonanoda gate` PASS；版本 0.28.0 → **0.29.0**（协议/功能 minor）。

## 本轮进度（2026-09-14，第五十四轮：elaborator `let`（Phase 1））

> 续第五十三轮的 TODO 清账：按 `docs/design/elaborator-let-match.md` 的
> S1–S5 落地**值位 `let`**（Phase 1）。`match` 依设计推迟到 Phase 2。

1. **设计**：`docs/design/elaborator-let-match.md` §11 切片 S1–S5 + 本文 §12 as-built。
2. **front（S1–S3）**：`Expr::Let`；`parse_expr` 识别 `let`（`starts_atom` /
   `named_group_ahead` 排除）；elab 分支（外层类型 + 期望类型 + `mk_let`
   `nondep=false`，缺注解 `elab-untyped-binder`）；`spine`/`proof`/`semantic`/
   `goals` 同步；`open_goal` 支持值位/body 洞。+21 测试（含 zeta 等价契约）。
3. **课程 + CLI（S4–S5）**：unit3 新增「局部绑定 `let`」小节（zh/en/钥匙，
   `def`/`#reduce` 逐字节镜像 + 两道 sorry 练习）；golden `unit3 (1,4,1)→(2,6,2)`、
   汇总 `checked 47→48 / open 34→36`；CLI e2e +3；`architecture.md` §4.1/§8、
   `TESTING.md` §1 同步。
4. **验收**：`sokonanoda gate` PASS（front 287、cli 97、lsp 108）。
5. **版本** 0.27.1 → **0.28.0**（新语法 = minor，Cargo + VSIX + CHANGELOG Added）。

> TODO 余项：`match`（Phase 2）、无注解 `let`、spine-meta A 实现、webview
> Infoview、事件流、perf 基准 + I8 early-cutoff、SHA256SUMS/attest。
