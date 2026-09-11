# 发布手册（RELEASE.md）

发布流水线：`.github/workflows/release.yml`（tag 触发 + `workflow_dispatch`
dry-run）。发布 = 推一个 `v*` tag，其余全自动。

> 设计依据：`docs/design/bundled-lsp.md`（插件自带 per-target VSIX + universal
> 回退包 + 版本锁定下载）。核心不变量：
> **tag == `Cargo.toml` == `package.json` == VSIX 内嵌的 LSP 二进制版本。**

## 1. 版本号在哪几处

| 位置 | 说明 |
| --- | --- |
| `Cargo.toml` → `[workspace.package].version` | **Rust 侧单一来源**。四个 crate 均 `version.workspace = true`，改这一处即可。 |
| `editor/vscode/package.json` → `version` | 扩展版本。必须与 Rust 一致，由契约测试 `cargo_and_extension_versions_match` 与 release 的 version gate 双重强制。 |
| `editor/vscode/CHANGELOG.md` | 扩展变更记录，发布时补对应版本条目。 |

注意：**根目录没有 CHANGELOG.md**。GitHub Release 的说明由
`generate_release_notes: true` 自动生成（基于上个 tag 以来的 commit/PR）。

## 2. 流水线概览

```text
push tag v* ──► job build（matrix：8 平台）
                  ├─ 原生：darwin-arm64 / darwin-x64（macos-latest）、
                  │        win32-x64 / win32-arm64（windows-latest）
                  └─ Linux：cargo-zigbuild + Zig（ubuntu-latest）
                     gnu 目标加 `.2.28` 地板（VS Code 的 Linux 最低要求），
                     musl（alpine-*）静态链接；构建后 readelf/ldd 断言
                  产物：artifact `lsp-<rust-target>/` 与 `cli-<rust-target>/`
                      （LSP 服务器 + `sokonanoda` CLI 裸二进制）
                    │
                job package-vsix（ubuntu；needs build）
                  1. version gate：tag == Cargo.toml == package.json
                  2. download 全部 lsp-*/cli-* artifact
                  3. 逐 target：stage-lsp.js（同时 stage 两者）→ vsce package --target
                     → 8 个平台包（linux-x64/arm64、alpine-x64/arm64、
                       darwin-arm64/x64、win32-x64/arm64，各内嵌 LSP+CLI）
                  4. clean bin/ → vsce package（无 target）
                     → sokonanoda-universal.vsix（回退包，无 bin）
                  5. 冒烟：python zipfile 断言每个平台包的两个 bin 路径、
                     大小 >1MB、linux/darwin exec 位、manifest TargetPlatform
                    │
                job github-release（needs build + package-vsix，contents: write）
                  ├─ 8 个 sokonanoda-lsp-<rust-target>.tar.gz（回退下载资产）
                  ├─ 8 个 sokonanoda-cli-<rust-target>.tar.gz（agent/headless）
                  └─ 9 个 .vsix
                    │
                job marketplace-publish（needs package-vsix）
                  └─ 先 universal、后 8 个平台包，逐包重试 4 次
                     （vsce publish --skip-duplicate --packagePath …）
```

- **Linux 二进制必须走 cargo-zigbuild 并显式 `.2.28`**：ubuntu-latest
  原生构建会带上 glibc 2.39 符号，Deacon/老发行版装不上；Zig 与
  cargo-zigbuild 版本在 workflow 里钉死（0.16.0 / 0.23.4）。musl 目标由
  Zig 静态链接（`ldd` 应为 "not a dynamic executable"）。
- **exec 位必须在 Ubuntu 上打包**：VSIX 的 zip 记录 unix mode，Windows 打包
  会丢（vsce 已知问题）。`scripts/stage-lsp.js` 在 stage 时 `chmod 755`，
  package-vsix 冒烟会断言。
- universal 包用于没有平台构建的用户（当前：Linux armhf 等），其下载 URL
  由扩展锁定到 `releases/download/v${extensionVersion}/…`，**不会**跟随
  latest。
- 发布顺序：universal 先、平台包后（Marketplace “Validating” 窗口有装错
  target 的竞态，见 `microsoft/vscode#141696`）；`--skip-duplicate` + `--clobber`
  保证 tag 重跑幂等。

## 3. 发布步骤

1. **改版本**：`Cargo.toml` 的 `[workspace.package].version` +
   `editor/vscode/package.json` 的 `version`，两处一致（如 `0.7.0`）；
   跑一次 `cargo check` 让 `Cargo.lock` 跟上。
2. **补 CHANGELOG**：`editor/vscode/CHANGELOG.md` 加对应条目；
   门面（README/description）与行为同步（`docs/vscode-dev-guide.md` §7）。
3. **跑校验清单**（见 §5），全绿后 commit + push。
4. **打 tag 并推送**：
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
5. **看 CI**：Actions → `release`。`build`、`package-vsix` 绿后，
   到 GitHub Releases 确认资产齐全（共 25 个）：
   - `sokonanoda-lsp-<rust-target>.tar.gz` ×8（回退下载）
   - `sokonanoda-cli-<rust-target>.tar.gz` ×8（agent/headless 直接执行）
   - `sokonanoda-{linux-x64,linux-arm64,alpine-x64,alpine-arm64,darwin-arm64,darwin-x64,win32-x64,win32-arm64}.vsix` ×8
   - `sokonanoda-universal.vsix`
6. **看 Marketplace**：版本、平台包与 universal 包都应在（`vsce show` 或
   网页端 Files 列表核对）。

tag 推错只需删 tag 重推：`git push origin :refs/tags/vX.Y.Z`（Release 若已建，
删 tag 后删 Release 再来）。

## 4. dry-run（无 tag 测试）

Actions → `release` → **Run workflow**（`workflow_dispatch`）。该模式：

- 跳过 version gate 的 tag 检查（仍校验 Cargo ↔ package.json 一致）；
- 跳过 Release 创建/上传与 Marketplace 发布；
- `build` + `package-vsix` 正常跑，5 个 VSIX 作为 workflow artifact 上传，
  可在 run 页面下载验证（含 exec 位冒烟）。

## 5. 发布前校验清单

```bash
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
cargo clippy --workspace --all-targets
cargo test --workspace --locked
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda

cd editor/vscode
npm ci
npm run test:unit        # server.js 解析/下载 + 重定向/解压单测
npm run package:host      # stage 本机二进制 + 打平台 VSIX
code --install-extension sokonanoda.vsix --force   # 手动验收（离线可用）
```

## 6. 已知限制与风险

- 平台覆盖：`linux-x64` / `linux-arm64` / `alpine-x64` / `alpine-arm64` /
  `darwin-arm64` / `darwin-x64` / `win32-x64` / `win32-arm64`（8 个，另有
  universal 回退包）。未覆盖的（如 linux-armhf）走 universal 的版本锁定
  下载；glibc 地板 2.28（与 VS Code 自身要求一致），Alpine 为静态 musl。
- Marketplace 平台包与 universal 包同版本并存；VS Code 的回落选择在历史上
  有过 bug（`microsoft/vscode#276673`），遇到装错 target 的反馈先让用户
  卸载重装。
- Azure gallery 端点间歇超时：marketplace-publish 每包重试 4 次 ×30s。
- macOS quarantine：VSIX 解压一般不带 quarantine；若用户被 Gatekeeper 拦，
  指引 `xattr -d com.apple.quarantine <extension>/bin/<target>/sokonanoda-lsp`。
- 只读扩展目录（Nix/系统安装）：chmod 修复会失败，扩展自动回退到
  workspace/缓存/下载路径并提示。
