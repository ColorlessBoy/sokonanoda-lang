---
name: sokonanoda-ci
description: Ship and monitor GitHub Actions for sokonanoda-lang without repeat failures - exact local-verification commands (exit codes, no grep masks), workflow pitfalls (download-artifact v4 layout, tag-pinned workflow files, gh release idempotency, vsce flags), run triage with gh CLI, and the CI-FAILURES.md ledger duty. Use when pushing, tagging, releasing, or debugging CI/Actions failures in this repo.
---

# sokonanoda-ci：GitHub Actions 推送 / 发布 / 排错纪律

> 背景：本仓库 CI 失败率曾偏高——根因几乎都不是代码本身，而是
> 「本地验证与 CI 不一致」和「对 Actions 机制想当然」。本 skill 把
> 踩过的每个坑固化成操作规程；失败台账在 `docs/CI-FAILURES.md`
> （同一类失败不犯第二次）。

## 0. 推送前的本地验证（必须逐条、必须看真退出码）

```bash
# 与 CI 完全一致的命令（.github/workflows/ci.yml），顺序执行：
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
cargo clippy --workspace --all-targets -q; echo "EXIT=$?"
cargo test --workspace --locked -q; echo "EXIT=$?"
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda
```

纪律：

1. **退出码必须直接看**：`echo $?` 或 `${PIPESTATUS[0]}`；
   绝不把管道末端（grep/head）的退出码当成 cargo 的。
2. **输出只许 tail/-A 上下文，不许 grep 掩膜**：`grep "^(warning|error)"`
   之类掩膜会吞掉真实警告（crate 路径在 `-->` 行，不在 warning 行）。
3. 教学 crates 的 `[lints.rust] warnings = "deny"` 会让**任何**
   rustc/clippy 警告变成 error——本地裸 clippy 看似只出 warning 的东西
   在 CI 是红。怀疑时不许自我辩解，用 CI 同款命令复现（历史教训：
   `int_plus_one` 假绿，见 CI-FAILURES 2026-09-09）。
4. 临时诊断代码（eprintln/debug 测试）**用完即删**，删完重跑 fmt+clippy。

## 1. Workflow 机制陷阱（全部踩过）

| 陷阱 | 事实 | 规程 |
|---|---|---|
| `download-artifact@v4` 不带 `name:` | 每个 artifact 下载进**同名目录**（`lsp-<target>/`、`sokonanoda-vsix/`），不是平铺 | 引用路径前先确认落盘布局；要平铺用 `pattern:` + `merge-multiple: true` |
| tag 触发的 workflow 用**哪个文件** | 用 **tag 指向的 commit** 上的文件，不是 main 最新 | 修 workflow 后要重跑 release：`git tag -f v<ver> <fix-commit> && git push -f origin v<ver>`；`gh run rerun` 只会重放旧文件 |
| `gh release create` 非幂等 | 已存在 → 422 | 永远 `|| true`（存在即跳过） |
| `gh release upload` 非幂等 | 同名 asset 已存在 → 422 → `bash -e` 全 job 死 | 永远 `--clobber`（重跑覆盖） |
| 强移 tag 前先想清楚 | release 上可能已有首跑传了一半的 asset | 上传步骤必须幂等后再强移 |
| `vsce package --no-dependencies` | 跳过生产依赖收集 → 12 文件/21KB 空壳 VSIX | 冒烟用与 release.yml 相同的命令（无该 flag）；平台包正确基线 ≈327 文件（含 `bin/<target>/`） |
| vsce publish Azure 超时 | `Request timeout: /_apis/gallery` 是**间歇性**网络问题 | 直接重跑该 job；连续两次超时再查代理 |
| action 名拼写 | 多个 s / 复数错名 → action 不存在 | 新 action 首次使用先验证存在 |
| run 步骤默认 `bash -e` | 任何一步非零退出 → 整个 step 死 | 想容忍的命令才加 `\|\| true`；别把 `bash -e` 当没有 |
| 平台包 exec 位 | VSIX 的 zip 记录 unix mode；**Windows 上 `vsce package` 会丢执行位**（vsce #152/#512） | 只在 Linux/macOS 打包；stage 时 `chmod 755`；冒烟用 python `zipfile` 断言 `mode & 0o111` |
| 平台包发布顺序 | Marketplace 同版本 universal + 多 target 并存；“Validating”窗口有装错 target 的竞态（vscode#141696） | 先 universal 后 target；每包独立重试；装错反馈先让用户卸载重装 |
| 版本门禁 | tag / `Cargo.toml` / `package.json` 三者不一致时 release 必须 fail（历史上靠人工） | release `package-vsix` 的 version gate + `cargo_and_extension_versions_match` 契约测试双保险；tag 前先 `cargo check` 更新 lock |

## 2. 触发与监控

```bash
git push origin main                      # → ci（main）
git tag v0.4.x && git push origin v0.4.x  # → ci（tag）+ release
gh run list --limit 5                     # 状态总览
gh run view <id> --json jobs --jq '.jobs[] | {name, conclusion}'
gh run view <id> --log-failed | tail -30  # 只看失败 step 的日志尾部
```

- **版本纪律先于 tag**：tag 之前确认 `Cargo.toml` 与
  `editor/vscode/package.json` 版本已 bump 且一致（feature→minor /
  fix→patch，见 `docs/vscode-dev-guide.md` §2）——release 的 version gate
  会直接 fail 不一致的 tag；发布形态（per-target VSIX + universal 回退包）
  与 dry-run 见 `docs/RELEASE.md`。
- **推送后必监控到终态**：`gh run list` 每 2–5 分钟一次；红 → 立即
  `--log-failed` 取证，不许"回头再看"。
- marketplace 没更新 = 先查 tag 是否真触发（`gh run list --workflow=release`），
  再查版本号是否与 tag 一致。

## 3. 排错三板斧

1. `gh run view <id> --json jobs` → 哪个 job 红；
2. `gh run view <id> --log-failed` → 失败 step 的**最后 30 行**（错误
   几乎总在尾部；警告墙会淹没中间）；
3. 区分三类：**代码问题**（修代码+测试）、**workflow 机制问题**
   （修 yml + 强移 tag）、**间歇性网络**（直接重跑 job）。

## 4. 失败必录

每次 CI 红了（本地推前发现的红也记），在 `docs/CI-FAILURES.md` 追加
原因 / 修复 / 预防；同一类失败第二次出现 = 流程没改进，返工。

补充（2026-09-10）：

| 陷阱 | 事实 | 规程 |
|---|---|---|
| `vscode-test` ETIMEDOUT | `Resolving version...` 后 `AggregateError [ETIMEDOUT]` 是连 `update.code.visualstudio.com` 下载 VS Code 失败，**间歇性** | 直接重跑 job；同提交下一轮绿即验证为网络问题；不要当代码回归查 |

| 陷阱 | 事实 | 规程 |
|---|---|---|
| Linux glibc 地板 | ubuntu-latest 原生构建会带 glibc 2.39 符号（VS Code 自身底线 2.28），老发行版装不上 | Linux 目标走 `cargo zigbuild` + `.2.28`；构建期 `readelf` 断言；musl（alpine）断言 `ldd` 静态；Zig/cargo-zigbuild 版本钉死 |
