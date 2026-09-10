---
description: 跑本仓库与 CI 一致的本地门禁（贡献者；需要 Rust），逐步报告真实退出码
agent: build
---

```bash
bash scripts/soko.sh gate; echo "EXIT=$?"
```

门禁内容 = 教学 crates fmt → workspace clippy → workspace 测试 →
`playground.sokonanoda` 的 `--json` 锚点。任一步失败脚本会停下并保留原始
输出；不许用管道/grep 掩膜退出码（见 `skills/sokonanoda-ci`）。

用户/agent 的零 cargo 路径是 `/sokonanoda/setup` + `/sokonanoda/check`，
不要用本命令做教学判卷。
