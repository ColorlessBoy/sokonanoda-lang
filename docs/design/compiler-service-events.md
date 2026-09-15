# 设计：编译器服务事件流（L1/L3）—— `file.didChange` 与 agent 消费（2026-09-14）

> 触发：ROADMAP §10 L2/L3「compiler service 事件流（`file.didChange` 等，
> 见 protocol.md 未来事件名）、讲课 agent 消费同一文档状态自动出题」。
> 依据：`docs/protocol.md`「Watch stream (L1 CLI form)」与「Future
> structured event names」、`crates/front/src/session.rs`
> （`SessionEventKind` 闭词汇）、`crates/cli/src/watch.rs`（已实现）、
> `skills/sokonanoda-teacher`（agent 决策表）。**本文只定方案，不改码。**

## 1. 现状：watch 就是服务层的 CLI 形态

`sokonanoda watch <file>`（`crates/cli/src/watch.rs`）轮询 300ms，
`Session::update` 后逐行 JSON：

```json
{"type":"file.changed","version":3,"recompiled_from":0}
{"type":"exercise.solved","name":null,"version":3}
{"type":"diagnostic","stage":"elab","code":"…","message":"…","hint":"…","span":{…},"version":3}
```

闭词汇在 `crates/cli/tests/common/mod.rs::WATCH_VOCABULARY`（7 个）：
`file.changed` / `decl.checked` / `decl.failed` / `exercise.opened` /
`exercise.solved` / `exercise.failed` / `diagnostic`。语义：编辑命令 i 只
内核重查 `i..n`（`recompiled_from` 事实化，`stats.kernel_checks` 可验证）。

watch 的限制：**单文档、轮询、无订阅/背压、无生命周期协商**——这正是
服务层要补的，而不是重写。

## 2. 关系：服务 = 同一引擎 + 同一闭词汇

- 服务**复用** `front::session::Session` 与上面的事件名，不引入第二套
  编译器；`file.changed` 的字段（`version` / `recompiled_from`）原样保留。
- L1 关注传输（订阅、推送、连接生命周期）；L3 关注 agent 消费同一事件流。
- 编辑器路径仍走 LSP（`soko/*`）；服务事件流是 **agent 面**，两者共享
  `front` 的快照数据，不互相替代（`docs/protocol.md` 的「一种词汇，两种
  渲染」原则）。

## 3. 事件名与版本化

- **规范名 `file.didChange`**（与 protocol.md 未来名单一致）：服务按此
  发射，并**在一个 minor 周期内继续接受/发射 `file.changed`** 作为兼容
  别名；弃用期在 `docs/protocol.md` 明写。
- 其余 delta 名保持不变；`docs/protocol.md` 列的未来名逐步收敛：
  `decl.rejected` 若保留则作为 `decl.failed` 的别名（二者不可并存发射）。
- **信封**：每条事件 `{type, version, ...}`；诊断继续带
  `stage/code/message/hint/span/version`；delta 带 `name`（可为 null）。
  `version` 是**文档版本**，客户端据此丢弃乱序/过期。
- **握手**：服务自述 `{protocol:1, engine:"<CARGO_PKG_VERSION>", pid}`
  （对齐 LSP 的 `soko/version`），客户端据此判断协议兼容与重启。

## 4. 单文档 vs workspace 作用域

- `--doc <path>`：等价今天的 watch（加 `uri`/文件标识），单 session。
- `--workspace <root>`：每个 `*.sokonanoda` 一个 session，**事件带
  `file`（稳定 uri/path）**；版本号是**每文件**的，跨文件**无全序**
  （协议必须写明，客户端按文件聚合）。
- 跨文件聚合（课程进度）仍归 CLI `sokonanoda course`（现有 `course.unit`
  /`course.summary`）；服务是否转播留给后续，v1 不做。
- 增量语义不变：会话只重查受影响后缀；`recompiled_from` 是事实，不是
  义务。

## 5. agent 如何消费（教学回环）

1. agent 启动时订阅目标文件；服务**先发当前全量快照**（按 `decl.*`
   事件或一次性 snapshot），再发增量——重连即恢复，无需历史回放。
2. 用户（或 agent）改画布 → 服务推 `file.didChange` + delta + 诊断；
   agent 按 `skills/sokonanoda-teacher` §3 决策表动作：`decl.checked` 肯定
   并追加；`exercise.open` 持续指向提示阶梯；诊断 code（带期望/实际）
   给针对性反馈，然后写下一段。
3. **不再轮询 `--json`**：事件即接口；agent 读事件，不读 exit code。
4. **背压**：每文件有界缓冲；溢出时**合并**为最新版本并标
   `recompiled_from: 0`（全量重查提示，客户端必须视为可重同步），协议
   写明该情形。
5. **客户端→服务命令**（最小集）：`subscribe {file, since_version?}`、
   `unsubscribe`、`ping`；v1 服务**只读观察磁盘**，不写文件、不执行代码
   ——写画布仍是 agent/编辑器的事。
6. `setContent`（内存画布）不在 v1：避免引入第二条文档真相来源。

## 6. 测试与验收

- **CLI**（`crates/cli/tests/`）：保留 watch 测试；新增 `--doc` 别名与
  `--workspace` 冒烟（临时目录多文件 → 断言每文件版本独立、事件带
  `file`）；等待事件用轮询/超时而非固定 sleep（`docs/design/real-input-tests.md`
  的 flake 纪律）。
- **契约**（`common/mod.rs` + `protocol.rs` 风格）：服务闭词汇断言；
  `docs/protocol.md` 必须列出每个发射名（现有 `protocol_document_lists_…`
  测试模式扩展）。
- **skill conformance**（`crates/cli/tests/skill.rs`）：teacher 技能只宣传
  这些事件名，防止文档漂移。
- **版本化**：弃用期内 `file.changed` 与 `file.didChange` 均可被接受/
  发射的测试。
- **验收**：agent 能仅凭事件流完成一轮「讲课 → 用户作答 → 判卷 → 下一步」
  （无 `--json` 轮询）；闭词汇与 handshake 落实；`sokonanoda gate` 全绿；
  `docs/protocol.md`、`REQUIREMENTS.md §9`、`STATUS.md` 同步。

## 7. 明确不做

- 网络/多用户/远程协作；服务写文件或执行用户代码；
- 跨文件事件的全局顺序；替代 LSP（编辑器面仍 LSP）；
- 历史事件回放（只做订阅时全量快照 + 增量）；
- 在服务里再实现一遍 kernel/前端逻辑（复用 `front::session` 是硬约束）。

---

## 8. as-built（2026-09-14，0.29.0）

- **规范名**：watch 开场事件改为 `file.didChange`（payload 不变，新增稳定
  `file` 字段）；`file.changed` 保留为**一个 minor 的弃用别名**（
  `WATCH_VOCABULARY` 接受、实现不再发射）。
- **握手**：stdout 第一行恒为
  `{"type":"service.hello","protocol":1,"engine":"<ver>","pid":<pid>}`
  （原纯文本 banner 移出 stdout）。
- **参数**：`watch <file>` / `watch --doc <file>` / `watch --workspace <root>`
  （互斥）；workspace 递归发现 `*.sokonanoda`（跳过 target/.git/node_modules），
  每文件一个 `Session` 与独立版本号，事件带 `file`，跨文件无全序。
- **背压**：每文件有界缓冲（64）；溢出合并为最新版本并标 `recompiled_from: 0`
  （协议注明「客户端须视为可全量重同步」）。
- **测试**：`crates/cli/tests/watch.rs`（握手 / didChange / `--doc` /
  `--workspace` 独立版本 / 闭词汇 + `file` 字段 / protocol.md 覆盖）+
  watch.rs 单测（溢出合并）；CLI 套件 105 pass。
- **文档**：`docs/protocol.md` watch 小节、`docs/TESTING.md`、
  `skills/sokonanoda-teacher/references/events.md`、`help.rs` 同步。
- **版本** 0.28.0 → **0.29.0**（协议/功能 → minor）。

---

## 9. as-built 续：stdin 客户端命令（2026-09-14，0.37.0）

- **命令集**（stdin JSON Lines，边轮询边处理）：`ping {id}` → `pong {id, protocol, engine}`；
  `subscribe {file}` / `unsubscribe {file}` 过滤 `--workspace` 哪些文件发事件
  （首个 subscribe 收窄为白名单；默认全发，兼容旧行为）；未知/畸形行 → `error`
  事件且流不中断。
- **非阻塞**：后台线程 `stdin().lock().lines()` + `mpsc`，轮询每 300ms `try_recv`
  排空；stdin EOF 不杀 watch（继续监控）。
- **测试**：`crates/cli/tests/watch.rs` ping/subscribe/unsubscribe/malformed 4 项
  （用 ping→pong 同步，不 sleep）+ watch.rs 单测 2 项。
- **文档**：`docs/protocol.md` watch 小节、`docs/TESTING.md`。
- 版本 0.36.0 → **0.37.0**（新能力 minor）。
