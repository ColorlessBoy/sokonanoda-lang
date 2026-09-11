---
description: 跑本仓库与 CI 一致的本地门禁（贡献者；二进制会调用 cargo），报告真实退出码
agent: build
---

从**任意目录**运行：

```bash
sokonanoda gate; echo "EXIT=$?"
```

- 门禁 = 教学 crates fmt → workspace clippy → workspace 测试 → playground
  `--json` 锚点；任一步失败即停并保留原始输出；
- `sokonanoda gate` 内部调用 `cargo`（因此需要 Rust）；它用**当前这个
  `sokonanoda` 二进制**跑 playground 锚点——贡献者想要工作树最新代码，
  先构建出 `target/debug/sokonanoda`，再直接运行它的 `gate`；
- 不许用管道/grep 掩膜退出码（见 `skills/sokonanoda-ci`）；
- 本命令是**贡献者**路径；用户/agent 的零 cargo 判卷用
  `/sokonanoda/setup` + `/sokonanoda/check`，不要用它做教学判卷。
