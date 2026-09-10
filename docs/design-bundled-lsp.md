# 插件自带 LSP 二进制（bundled VSIX）设计

> 日期：2026-09-10。触发：用户要求——像 coq/lean 的 VS Code 插件那样把
> bin 打包进插件，消掉「装完插件还要再下载 GitHub、还可能网络失败」的差体验。
> 本文只做调研 + 设计与计划；实现按 §8 分阶段执行。

## 0. 调研结论（先纠正一个前提）

- **官方 Lean 4 扩展不打包二进制**：`leanprover.lean4` v0.0.239 的 VSIX 只有
  ~5.3 MiB 纯 JS/webview，运行时用 PATH（+`~/.elan/bin`）里的 `lean --server` /
  `lake serve`；连 elan 都是首次使用时联网脚本装的。
- **VsCoq2 / Rocq 扩展也不打包**：要求用户先 `opam install
  vsrocq-language-server`（或装 Rocq Platform），扩展只 `which`/PATH 找；
  `coq-lsp` 扩展同理（唯一例外是浏览器版把 WASM worker 放进 VSIX）。
- 所以「照抄官方」不存在；但 **VS Code 官方机制完全支持我们要做的事：
  platform-specific VSIX**（`vsce package --target <t>`）。这条路是
  rust-analyzer/cpptools 等社区扩展的通用做法。
- 关键机制事实（实现依赖，均有官方文档/源码依据）：
  1. targets：`win32-x64` / `win32-arm64` / `linux-x64` / `linux-arm64` /
     `linux-armhf` / `alpine-x64` / `alpine-arm64` / `darwin-x64` /
     `darwin-arm64` / `web`；**不带 `--target` 的包 = universal fallback**；
  2. Marketplace 同一扩展 id 下 target 包与 universal 包共存；同版本时
     target 包优先，平台没有 target 包才回落 universal；
  3. **VSIX 能保留可执行位**：在 Linux/macOS 上打包时 zip 记录 unix mode，
     VS Code 解压按 mode 落盘（yauzl `modeFromEntry`）；**在 Windows 上打包
     会丢 exec bit**（vsce 已知问题）——所以必须在 CI 的 ubuntu 上打包；
  4. 体积：本仓库 release LSP 实测 **3.4 MB**（fat LTO，darwin-arm64）；
     每平台 VSIX ≈ 4 MB。Marketplace 历史默认单包上限 25 MiB（可申请提高），
     Open VSX 256 MiB——余量充足；
  5. `extensionKind: ["workspace"]` 已就位：远程/容器/WSL 场景二进制跑在
     workspace 侧，VS Code 会按远端平台选 target 包；
  6. 扩展目录可能只读（Nix/系统安装）；惯例是运行时 `X_OK` 检测 +
     best-effort `chmod`，写回缓存放 `context.globalStorageUri`。

## 0.5 现有发布/下载链路（as-is）与版本错配根因

### 发布流水线（`.github/workflows/release.yml`，tag `v*` 触发）

```text
build（matrix 4 平台，各自原生构建）
  ├─ ubuntu   x86_64-unknown-linux-gnu
  ├─ macos    aarch64-apple-darwin
  ├─ macos    x86_64-apple-darwin
  └─ windows  x86_64-pc-windows-msvc
  产物：artifact `lsp-<rust-target>`（裸二进制）
        │
vsix（ubuntu，与 build 并行）
  └─ npm ci + `vsce package`（**universal，无 bin**）→ artifact `sokonanoda-vsix`
        │
github-release（needs build+vsix，contents: write）
  ├─ `gh release create || true`
  ├─ 每个 lsp-<target>/ → `sokonanoda-lsp-<rust-target>.tar.gz`（--clobber 幂等）
  └─ 上传 universal VSIX
        │
marketplace-publish（needs vsix）
  └─ `vsce publish --skip-duplicate --packagePath sokonanoda.vsix`（Azure 超时重试 ×4）
```

### 扩展端下载（`editor/vscode/extension.js:101-169`）

- URL（第 113 行）：
  `https://github.com/ColorlessBoy/sokonanoda-lang/releases/**latest**/download/sokonanoda-lsp-${target}.tar.gz`
- 缓存：`~/.local/share/sokonanoda/bin/sokonanoda-lsp`（Windows 用 `%USERPROFILE%`）
  + `.version` 标记文件；`cachedServerIsCurrent` 把标记与**扩展版本**比较，
  决定是否重新下载。
- 目标映射：`darwin` 按 arch 映射 `aarch64/x86_64-apple-darwin`；
  `win32`→`x86_64-pc-windows-msvc`；其余→`x86_64-unknown-linux-gnu`。

### 版本错配根因（用户点名的恶心点）

1. **下载的是 `latest`，不是扩展对应版本**：`.version` 标记只回答
   「要不要重新下载」，不锁「下载哪个版本」。旧插件 + 新 Release 会拉到
   比插件新的 LSP；客户端/服务端协议（`soko/*` 自定义方法、诊断形状、
   `by_steps`/`stateAt` 等）静默漂移。
2. **扩展比 Release 新**（tag 已推、CI 挂了、资产缺失，或本地/开发 VSIX）：
   `latest` 指向旧 Release，拉到旧二进制，甚至 404。
3. **不可复现**：同一扩展版本在不同时间安装，拉到的二进制可能不同
   （取决于当时 latest 是什么）。
4. **没有任何版本一致性门禁**：tag、`Cargo.toml`、`package.json` 全靠
   `docs/RELEASE.md` 人工同步；`ci.yml` 也不检查三者的关系。
5. **LSP 无版本握手**：服务端没有 `--version`、`initialize` 也不回版本，
   协议不兼容时只能等某条请求炸掉，报错不指向版本问题。

### 新链路如何消灭它

- bundled 二进制与扩展**同一个 tag、同一次构建**产出 → 版本一致是结构性
  保证，不靠约定；
- 保留的 universal 下载回退把 URL 从 `latest` 改成
  `releases/download/v${extensionVersion}/...`（版本锁定、可复现、可回滚）；
- release 流水线加 tag↔版本校验；常规 CI 加两版本字段一致性的契约测试；
- （Phase 3 可选）给 LSP 加 `--version` / `initialize.serverInfo`，扩展
  启动时断言一致，报错直指版本。

## 1. 目标 / 非目标

**目标**：

- 受支持平台安装 VSIX 后**零网络**启动 LSP：不再有「正在下载语言服务器」；
- 内核二进制与扩展同 tag 同版本（版本一致性由 CI 保证）；
- 保留逃生通道：无内置二进制的平台仍回退到现有下载 / 自编译路径。

**非目标**：

- 不做 WASM/web 版内核（另立项）；
- 不做扩展内自动更新二进制（跟着 VS Code 更新扩展走）；
- 首期不覆盖所有平台（与 `release.yml` 现有 4 个 target 对齐）。

## 2. 方案取舍

| 方案 | 说明 | 结论 |
|---|---|---|
| A. 单 VSIX 内嵌全平台二进制，运行时释放到 globalStorage | 一个包，但 ~14MB、要管缓存/更新/清理，非标准做法 | 否 |
| B. **per-target VSIX（4 个 target）+ universal fallback** | 用户只下自己平台 ~4MB；标准机制；exec bit 可控 | **选定** |
| C. 保持激活时下载 | 用户点名的差体验 | 否 |
| D. 学 Lean/VsCoq 依赖系统安装 | 与「自包含、零 setup」产品定位冲突 | 否 |

## 3. 实现设计

### 3.1 打包布局

```text
editor/vscode/bin/<target>/sokonanoda-lsp[.exe]   # gitignored；打包前 stage
```

首期 target（与 release matrix 一致）：
`darwin-arm64` / `darwin-x64` / `linux-x64` / `win32-x64`。

### 3.2 运行时解析顺序（`extension.js`）

1. `sokonanoda.serverPath` 设置（显式覆盖；受限模式仍忽略）；
2. `SOKONANODA_LSP_BIN` 环境变量；
3. **bundled**：`<extensionPath>/bin/<target>/sokonanoda-lsp[.exe]`
   —— 存在即用；先 `X_OK` 检测，缺可执行位时 best-effort `chmod 0o755`
   （只读目录失败则记日志、继续走回退）；
4. 开发发现：workspace/仓库 checkout 的 `target/{debug,release}`（现有逻辑）；
5. 下载缓存（现有 version marker 机制）；
6. GitHub Release 下载（**URL 改为版本锁定**：
   `releases/download/v${extensionVersion}/sokonanoda-lsp-${rustTarget}.tar.gz`，
   不再用 `/latest/`；404 给可行动文案：该版本无此平台二进制 / 请升级插件 /
   用 `serverPath` 指定本地二进制）；
7. 失败文案区分「本平台无内置二进制（给出下载/自编译指引）」与
   「内置二进制不可执行（只读扩展目录）」。

设计理由：bundled 在 target/ 之前——产品目标是「装了就用」；开发者要覆盖
二进制时用 `serverPath`/env（写入文档）。F5 开发时 `bin/` 不存在
（gitignored），自然落回 `target/`。

**重构**：把解析与下载从 `extension.js` 抽到 `server.js`（不依赖 `vscode`，
路径/平台/fs 可注入），`extension.js` 只接线。顺带消灭现有隐患：
`test-download.js` 目前**复制了一份下载逻辑**（注释自认 "duplicated for
test isolation"），改为 require 真实现，杜绝漂移。

### 3.3 打包与发布（to-be）

**本机开发**：

- `editor/vscode/package.json` scripts：
  - `stage:lsp`：按 host（或参数 target）把 `target/release/sokonanoda-lsp`
    stage 到 `bin/<target>/` + `chmod +x`；
  - `package:host`：stage + `vsce package --target <host-target>`；
  - `package:universal`：不含 `bin/` 的 fallback 包。
- `.vscodeignore`：保留 `bin/**`，排除 staging 垃圾；逐 target 打包时先清
  `bin/` 只放当前平台（比 `--ignore-other-target-folders` 更少依赖版本行为）。

**发布流水线（release.yml 重排）**：

```text
build（matrix 4 平台，不变）
  └─ artifact：lsp-<rust-target>（裸二进制，同时继续做 tar.gz 供回退下载）
        │
package-vsix（ubuntu，需要 build；新增，替代原 vsix job）
  ├─ download-artifact 全部 lsp-*
  ├─ 版本门禁：GITHUB_REF_NAME == v$(Cargo.toml version) == v$(package.json version)
  ├─ 逐 target：清 bin/ → stage 二进制（rust 三连→vsce target 映射）→ chmod +x
  │             → `vsce package --target <t> --out dist/sokonanoda-<t>.vsix`
  ├─ universal：清 bin/ → `vsce package --out dist/sokonanoda-universal.vsix`
  └─ artifact：全部 5 个 VSIX
        │
github-release（needs package-vsix）
  ├─ 4 个 `sokonanoda-lsp-<rust-target>.tar.gz`（--clobber）
  └─ 5 个 VSIX（--clobber）
        │
marketplace-publish（needs package-vsix；保留 Azure 超时重试 ×4）
  └─ 先 universal、后逐 target：`vsce publish --skip-duplicate --packagePath …`
```

rust 三连 ↔ vsce target 映射（与 build matrix 一一对应）：

| rust target | vsce target | 二进制名 |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `linux-x64` | `sokonanoda-lsp` |
| `aarch64-apple-darwin` | `darwin-arm64` | `sokonanoda-lsp` |
| `x86_64-apple-darwin` | `darwin-x64` | `sokonanoda-lsp` |
| `x86_64-pc-windows-msvc` | `win32-x64` | `sokonanoda-lsp.exe` |

要点：

- **exec 位必须在 ubuntu 上打包**（research §B4：Windows 打包丢 unix mode）；
  stage 后 `chmod 755`，CI 用 `unzip -Z` 对 linux/darwin VSIX 断言；
- universal fallback 保留**版本锁定的下载器**（URL 见 §3.2 修改项），
  这样首期未覆盖的平台（alpine / arm64 linux / win32-arm64）仍可工作；
- 发布顺序：先 universal 后 target（research 提到 Marketplace
  "Validating" 窗口存在装错 target 的竞态，`microsoft/vscode#141696`）；
  tag 重跑靠 `--skip-duplicate` + `--clobber` 幂等；
- **版本纪律门禁**：
  - `package-vsix` 里断言 tag 与两处版本一致（`workflow_dispatch` 跳过 tag 项）；
  - `crates/cli/tests/extension.rs` 加契约测试：`Cargo.toml` workspace version
    == `editor/vscode/package.json` version（防日常漂移，tag 前就能发现）；
- 本功能为 minor：0.6.0 → **0.7.0**（Rust 与扩展同步 bump）。

### 3.4 文档 / 门面同步

`package.json` description（不再说 "the server downloads itself"）、
`editor/vscode/README.md` Quick start（装完即用 + universal 回退说明）、
`CHANGELOG.md`、`docs/vscode-dev-guide.md`（§4 开发循环加 stage、§5 加
exec-bit/打包平台坑）、`docs/RELEASE.md`（**已过期**，本轮一并重写为
per-target 流程）、`skills/sokonanoda-ci`（发布陷阱 +1）、`docs/STATUS.md`、
`docs/REQUIREMENTS.md` §9。

## 4. 测试

- **node 单测**（新 `test-server.js`；`server.js` 为纯模块）：解析顺序矩阵
  （设置 / env / bundled / 缓存）、平台→target 映射（含不支持平台）、
  X_OK 修复只在缺位时调用、只读写失败降级；
- **静态契约**（`crates/cli/tests/extension.rs`）：脚本消费 bundled 路径、
  含 target 映射、scripts 声明齐全、版本一致、description 不再承诺下载；
- **VS Code 集成**（`npm test`，CI xvfb）：测试 job 先把 LSP stage 到
  `editor/vscode/bin/linux-x64/` 再跑，证明「只靠 bundled，诊断/hover 能到」；
  另加无缓存激活用例（HOME 指向临时目录，确保不走下载缓存）；
- **发布 dry-run**（workflow_dispatch）：检查 5 个 VSIX + exec 位；
- **回退不回归**：现有 `test-download.js` 全绿（universal 下载路径）。

## 5. 边界与已知风险

- 平台覆盖 8 个：`linux-x64` / `linux-arm64` / `alpine-x64` / `alpine-arm64` /
  `darwin-arm64` / `darwin-x64` / `win32-x64` / `win32-arm64`（对齐 C# 8 平台，
  cpptools 为 9 含 linux-armhf）；linux-armhf 等仍未覆盖 → universal 下载兜底
  （版本锁定）。linux-x64 曾因 ubuntu-latest 原生构建引入 glibc 2.39 依赖，
  已改 cargo-zigbuild 显式 `.2.28` 地板（VS Code 自身的 Linux 最低要求）；
  Alpine 为静态 musl，运行时按 `/etc/alpine-release` 检测（与 VS Code 一致）。
- macOS quarantine：VSIX 解压通常不带 quarantine；若用户遇到 Gatekeeper，
  文档给 `xattr -d com.apple.quarantine` 指引；
- 只读扩展目录 chmod 失败 → 降级到下载缓存并提示；
- Marketplace 连发 5 个包的重试/幂等（沿用现有重试循环）；
- 远程/WSL：`extensionKind: workspace` 已就位，target 由 VS Code 选择；
  linux 远端需要 linux 包（首期有）。

## 6. 验收

- 无网环境安装 `sokonanoda-<host>.vsix` → 打开 `.sokonanoda` → **无任何下载
  提示**，诊断 / hover / 练习树正常；
- **版本一致性**：bundled 二进制与扩展同 tag；release job 的 tag↔版本门禁
  与 CI 契约测试生效；fallback URL 为 `v${extensionVersion}`（无 `latest`）；
- universal VSIX 仍可下载回退；unsupported 平台给出明确指引；
- release dry-run 产出 5 个 VSIX，linux/darwin 包 exec 位正确；
- 全部门禁（fmt / clippy / workspace tests）+ 扩展静态契约 + VS Code 集成
  测试全绿。

## 7. 影响文件清单（实现时）

| 文件 | 改动 |
|---|---|
| `editor/vscode/extension.js` | 接线 `server.js`；激活文案 |
| `editor/vscode/server.js` | **新增**：解析 + 下载（自 extension.js 迁出） |
| `editor/vscode/scripts/stage-lsp.js` | **新增**：staging |
| `editor/vscode/test-server.js` | **新增**：纯 node 单测 |
| `editor/vscode/test-download.js` | 改为 require `server.js`（去重复实现） |
| `editor/vscode/package.json` | scripts、description、version 0.7.0 |
| `editor/vscode/.vscodeignore` | `bin/**` 保留规则 |
| `editor/vscode/README.md` / `CHANGELOG.md` | 门面同步 |
| `editor/vscode/src/test/extension.test.js` | bundled 激活用例 |
| `.github/workflows/release.yml` | per-target 打包 + exec 位冒烟 + 发布循环 |
| `.github/workflows/ci.yml` | 测试 job stage bundled 后再跑集成测试 + 版本一致性契约测试 |
| `.gitignore` | `editor/vscode/bin/` |
| `crates/cli/tests/extension.rs` | 契约 +3（bundled 解析、scripts/描述、Cargo↔package.json 版本一致） |
| `docs/vscode-dev-guide.md` / `docs/RELEASE.md` / `skills/sokonanoda-ci` | 打包/发布纪律 |
| `docs/STATUS.md` / `docs/REQUIREMENTS.md` | 收尾 |

## 8. 分阶段计划

- **Phase 1（核心可用）✅ 2026-09-10**：`server.js` 重构 + 解析顺序（含
  bundled + chmod 守卫）+ `scripts/stage-lsp.js` + `.gitignore` +
  node 单测（15）+ 静态契约（+3）；本机 `package:host` 出 VSIX（1.96MB，
  zip mode 755）、universal 0.47MB 无 bin。
- **Phase 2（发布闭环）✅ 2026-09-10**：`release.yml` per-target VSIX +
  tag↔版本门禁 + exec-bit 冒烟 + Release 附件 + marketplace 逐平台发布；
  fallback URL 改版本锁定；`docs/RELEASE.md` / 开发指南 / CI skill 同步；
  **dry-run `workflow_dispatch` 全绿**（run 34460822423，5 个 VSIX）；版本
  bump 0.7.0 + 门面同步；两个只存在于 CI 的路径 bug 修掉并记台账。
- **Phase 3（硬化）✅ 2026-09-10（v0.8.0）**：CI 集成测试走 bundled 路径
  （fresh runner = 无缓存激活性实证）；平台矩阵 4 → 8：新增 linux-arm64 /
  alpine-x64 / alpine-arm64 / win32-arm64；Linux 目标改 cargo-zigbuild +
  显式 glibc 2.28 地板（顺带修掉 linux-x64 的 24.04 兼容隐患），musl 静态
  链接、win32-arm64 原生构建；release.yml 冒烟同步扩到 9 个 VSIX。
  剩余：linux-armhf（边缘平台，universal 兜底）与用户反馈收集。
