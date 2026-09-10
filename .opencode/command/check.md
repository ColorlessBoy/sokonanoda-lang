---
description: 用真实内核判卷一个 .sokonanoda 文件并汇总 JSON 事件（默认 playground.sokonanoda）
agent: build
---

用真实内核判卷 `$ARGUMENTS`（未给参数则用 `playground.sokonanoda`）：

```bash
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json <file>
```

汇总并报告：

- 各事件计数（`decl.checked` / `exercise.open` / `expr.typed` / `expr.reduced` / `diagnostic`）；
- 每个开放练习的名字与剩余目标；
- 每条诊断的 `line:col code message hint`。

纪律：

- 开放练习（`sorry`）是**合法状态**（exit 0），不是错误；
- 判定永远走 kernel，禁止文本比对或"看起来对"；
- 需要逐洞判定/建议时用 `--json` 的事件词表（见 `docs/protocol.md`），不要自造输出。
