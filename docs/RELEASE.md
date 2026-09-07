# 发布手册（RELEASE.md）

发布流水线：`.github/workflows/release.yml`（业内标准 tag 触发式发布）。
发布 = 推一个 `v*` tag，其余全自动。

## 1. 版本号在哪几处

| 位置 | 说明 |
| --- | --- |
| `Cargo.toml` → `[workspace.package].version` | **Rust 侧单一来源**。四个 crate（kernel/front/cli/lsp）均 `version.workspace = true`，改这一处即可。 |
| `editor/vscode/package.json` → `version` | 扩展版本，与 Rust 版本保持一致（人工同步）。 |
| `editor/vscode/CHANGELOG.md` | 扩展的变更记录，发布时补一条对应版本条目。 |

注意：**根目录没有 CHANGELOG.md**。因此 release 工作流不使用
`taiki-e/create-gh-release-action` 的 changelog 参数，GitHub Release 的说明由
`generate_release_notes: true` 自动生成（基于上个 tag 以来的 commit/PR）。
若日后想改为 changelog 驱动，先在根目录建 CHANGELOG.md 再改工作流。

## 2. 流水线概览

```
push tag v* ──► job build (ubuntu-latest)
                 1. cargo build --release --locked -p sokonanoda-cli -p sokonanoda-lsp
                 2. softprops/actions-gh-release 建 Release（名 = tag，自动生成说明）
                 3. taiki-e/upload-rust-binary-action 上传
                    sokonanoda,sokonanoda-lsp → sokonanoda-x86_64-unknown-linux-gnu.tar.gz
                    + .sha256 校验和
               ─► job vsix (needs: build)
                 1. node 22 + npm ci（editor/vscode）
                 2. npx @vscode/vsce package --out sokonanoda.vsix
                 3. upload-artifact 挂 vsix
                 4. gh release upload 把 vsix 附到同一 Release
```

权限最小化：workflow 顶层 `contents: read`；仅需要写权限的 job 声明
`contents: write`（build 的建 Release/传资产、vsix 的 `gh release upload`）。

## 3. 发布步骤

1. **改版本**：`Cargo.toml` 的 `[workspace.package].version` +
   `editor/vscode/package.json` 的 `version`，两处一致（如 `0.2.0`）。
2. **补 CHANGELOG**：`editor/vscode/CHANGELOG.md` 加对应条目。
3. **跑校验清单**（见 §5），全绿后 commit。
4. **打 tag 并推送**：
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
5. **看 CI**：Actions → `release` workflow；`build` 与 `vsix` 都绿后，
   到 GitHub Releases 确认资产齐全：
   - `sokonanoda-x86_64-unknown-linux-gnu.tar.gz`
   - `sokonanoda-x86_64-unknown-linux-gnu.tar.gz.sha256`
   - `sokonanoda.vsix`

tag 推错只需删 tag 重推：`git push origin :refs/tags/vX.Y.Z`（Release 若已建，
删 tag 后删 Release 再来）。

## 4. dry-run（无 tag 测试）

Actions → `release` → **Run workflow**（`workflow_dispatch`）。该模式：

- 跳过 Release 创建与 `gh release upload`；
- taiki-e action 以 `dry-run: true` 运行（构建 + 打 tar.gz，不上传）；
- 二进制 tar.gz 与 vsix 都只作为 workflow artifact 上传，可在 run 页面下载验证。

## 5. 发布前校验清单

```bash
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
cargo clippy --workspace --all-targets
cargo test --workspace --locked
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda
```

VSIX 本地预打包（应输出 `DONE  Packaged: sokonanoda.vsix`）：

```bash
cd editor/vscode
npm ci
npx @vscode/vsce package --out sokonanoda.vsix
```

## 6. 已知限制与风险

- **【高优先】VSIX 缺运行时依赖**：`extension.js:10` 运行时
  `require("vscode-languageclient/node")`，但 `editor/vscode/.vscodeignore`
  排除了 `node_modules/**`，实测 vsce 打包结果不含 node_modules（9 个文件，
  13.54 KB）——装上后会报 "Cannot find module"。修复（需另改文件，本手册
  无权限）二选一：
  1. `.vscodeignore` 删除 `node_modules/**` 一行（vsce 会自动包含
     package.json `dependencies` 的生产依赖），最小改动；
  2. 引入 esbuild/webpack 打包为单文件（依赖 devDependencies，改动更大）。
- vsce 未列入 devDependencies（`package.json` 冻结），CI 里
  `npx @vscode/vsce` 每次解析最新版 → 存在版本漂移/供应链风险；可用
  `npx @vscode/vsce@<pin>` 收敛。
- `publisher` 是占位符 `sokonanoda-lang`，未注册 marketplace：只把 vsix
  挂 GitHub Release，不发布商店。
- 目前仅 linux x86_64 二进制；需要 mac/windows 时在 build job 加
  target matrix（taiki-e action 原生支持）。
- 以下属 GitHub 环境特有，**需 CI 首跑验证**：taiki-e 多 bin +
  `package` 映射行为、softprops 建 Release、`gh release upload`、
  setup-node 的 npm cache。
