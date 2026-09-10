---
description: 跑本仓库与 CI 一致的本地门禁（贡献者；需要 Rust），报告真实退出码
agent: build
---

从**任意目录**运行（脚本自己处理门禁内容与失败输出）：

```bash
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
bash "$ROOT/scripts/soko.sh" gate; echo "EXIT=$?"
```

- 门禁 = 教学 crates fmt → workspace clippy → workspace 测试 → playground
  `--json` 锚点；任一步失败脚本停下并保留原始输出；
- 不许用管道/grep 掩膜退出码（见 `skills/sokonanoda-ci`）；
- 本命令是**贡献者**路径；用户/agent 的零 cargo 判卷用
  `/sokonanoda/setup` + `/sokonanoda/check`，不要用它做教学判卷。
