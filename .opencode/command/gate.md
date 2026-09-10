---
description: 跑本仓库与 CI 完全一致的本地门禁（fmt / clippy / workspace 测试 / playground 锚点），逐步报告真实退出码
agent: build
---

从仓库根目录按顺序执行本仓库的 CI 门禁命令，并报告**每一条的真实退出码**（不许管道/grep 掩膜——见 `skills/sokonanoda-ci` 的教训台账）：

1. `cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check`
2. `cargo clippy --workspace --all-targets -q; echo "EXIT=$?"`
3. `cargo test --workspace --locked -q; echo "EXIT=$?"`
4. `cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda`
   （锚点应保持 `decl.checked=20 / exercise.open=9 / 0 diagnostic`）

规矩：

- 任一步失败就停下，贴出失败输出与完整命令，不要用"改测试/改 golden"来掩盖；
- kernel 冻结（`docs/architecture.md` §6）：fmt 不覆盖 kernel，clippy 对 kernel 只要求 warning 级；
- 最后给一行总结：`gate: PASS` 或 `gate: FAIL at step N（原因）`。
