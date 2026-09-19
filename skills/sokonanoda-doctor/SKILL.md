---
name: sokonanoda-doctor
description: Diagnose whether the version-pinned sokonanoda CLI + LSP are ready to run, and report the exact next command when they are not. Use at the start of a session, when the launcher refuses to run, or when grading fails in a way the file cannot explain.
---

# sokonanoda-doctor：环境就绪诊断

## 命令（只读；从仓库根或任意目录）

```bash
scripts/soko doctor --json; echo "EXIT=$?"
```

## 退出码

- `0` = **就绪**，可以判卷；
- `3` = **未就绪**（缓存缺失或版本不符）→ 下一步 `scripts/soko setup`；跑完
  仍不就绪就 `scripts/soko update`（见 `skills/sokonanoda-update/SKILL.md`）；
- 其他非 0 = 用法或启动器自身错误：把原文贴出来，**不要猜**。

## 汇报字段

`ready` / `version` / `version_source` / `version_constraint` / `version_error` /
`target` / `rust_target` / `cache` / `offline` / `launcher` / `cli` / `lsp`；后两者
各含 `path` / `present` / `ready` / `marker`。

`version` 来自**版本钉源链**（`SOKONANODA_VERSION` → `sokonanoda-version.txt` →
`sokonanoda.toml` 的 `requires` → `Cargo.toml`），`version_source` 指名中的哪个；
`version_error` 非空（`version` 为 `null`）= 源缺失/冲突，此时 `ready` 必为
`false`——**解析不出期望版本就绝不 exec 缓存**（不是"再试一次"能好的事）。

`marker` 形如 `<version> <target>`，必须与 `version` 一致；不等就是
缓存过期。`present: true` 但 `ready: false` 正是"文件在、版本不对"这一种情形。

## 纪律

- **只读**：不要顺手 `setup` / `update` / 删缓存。先报告实测值，再按用户
  意图走下一步。
- 不得要求用户安装 Rust/cargo（`REQUIREMENTS.md` §2 第 9 条）。
- 在 DeepSeek Harness 里**不要等编辑器诊断**：服务端 `publishDiagnostics`
  不进 agent，判卷一律走 CLI（`dsh/README.md`）。
