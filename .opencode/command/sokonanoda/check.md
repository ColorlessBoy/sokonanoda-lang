---
description: 用真实内核判卷一个 .sokonanoda 文件并汇总 JSON 事件（默认 playground.sokonanoda；零 cargo）
agent: build
---

从**任意目录**用单一入口判卷 `$ARGUMENTS`（未给参数则用
`$ROOT/playground.sokonanoda`）：

```bash
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
bash "$ROOT/scripts/soko.sh" grade "$ROOT/playground.sokonanoda"
```

该入口在缺二进制时会自动按版本补齐（等价于 `setup`），然后在
`~/.local/share/sokonanoda/bin/sokonanoda` 上执行 `--json`。

汇总并报告：

- 各事件计数（`decl.checked` / `exercise.open` / `expr.typed` / `expr.reduced` / `diagnostic`）；
- 每个开放练习的名字与剩余目标；
- 每条诊断的 `line:col code message hint`。

纪律：

- 开放练习（`sorry`）是**合法状态**（exit 0），不是错误；
- 判定永远走 kernel，禁止文本比对或「看起来对」；
- 用户/agent 路径零工具链依赖（`REQUIREMENTS.md` §2 第 9 条），
  禁止改用源码构建命令。
