# 为什么不直接编一个跨平台 Rust 二进制，而要 `soko.sh`

> 调研（2026-09-11，用户提问）：能不能像现在一样用 `soko.sh` 这么麻烦，
> 直接把 Rust 编成一个跨平台二进制？结论：**跨 OS 的单一二进制在技术上
> 不存在**；Rust 的编译单位是 target triple，跨平台 = 每个平台各出一份
> 二进制 + 一个"选对平台"的引导器。`soko.sh` 就是那个引导器，不是二进制的
> 替代品。

## 1. Rust 的"跨平台"跨的是什么

- 编译期就绑定 target triple（OS + libc/ABI + CPU 架构），例如
  `x86_64-unknown-linux-gnu`、`aarch64-apple-darwin`、
  `x86_64-pc-windows-msvc`。不同 triple 产出不同的
  **可执行格式**（ELF / Mach-O / PE）与 **系统调用接口**（Linux syscall、
  macOS Mach、Windows Win32），彼此不能直接互换执行。
- 唯一存在的"合体"是**同一 OS 内多架构**：macOS universal binary
  （x86_64 + arm64），Windows ARM64X 之类的跨架构合并。**跨 OS 没有**。
  例证：`cargo-binstall` 的发行物里只有 `universal-apple-darwin` 是 universal，
  其余全是 per-target。
- 所以"跨平台"在工程上只能是：**每个平台一个二进制**，安装时挑对的。
  本仓库正是这么做的——release 出 8 个 target 的
  `sokonanoda-cli-<triple>.tar.gz` / `sokonanoda-lsp-<triple>.tar.gz`，
  VSIX 里按平台内嵌。LSP/CLI 本身是 Rust，就已经是"跨平台二进制"，
  只是"多份"。

## 2. `soko.sh` 到底在干什么

它不参与编译，是 **bootstrap（引导器）**：

- 读 `Cargo.toml` 的版本 → 拼 release URL（锁定 `v<version>`，禁用 `latest`）；
- 按 `uname` 选 target triple → 下载对应二进制 → 放缓存 → 写版本标记；
- `setup` / `update` / `version` / `doctor` 都围绕"选对并维护这一份缓存"。

这些事发生在"系统上还没有任何二进制"的阶段，而做这件事的东西**不能依赖
那个二进制**（鸡生蛋）。rustup 就是同一模式：它最终是"一个二进制"，但官方
安装入口是 `rustup-init.sh`（`curl | sh`），由脚本挑并下载对应平台的
`rustup-init`；`rustup` 自身的代码注释也写明"安装基本上就是把二进制拷到位"。
`cargo-dist` / Homebrew / npm 的 shell installer 同理。

## 3. 为什么引导器不用 Rust 写

- 用 Rust 写引导器，也得**每个平台发一个引导器二进制**，再解决"用户怎么拿到
  引导器"——回到原点；仍然需要脚本 / 包管理器 / 编辑器插件迈第一步。
- 引导逻辑很小（选平台、下载、解压、PATH）。macOS/Linux 上 bash 现成；
  不用 bash 的那条路也已经有了——**opencode 插件**用 Node `fetch` + `tar`
  完成同样的 provisioning（零 bash、带 `.exe` 与 Alpine 判断），覆盖
  Windows/Alpine。
- 安全：`curl | sh` 有信任负担，换 Rust 二进制一样要做签名/校验，不省。

## 4. 现状盘点（已经是三条腿）

| 场景 | 引导方式 |
|---|---|
| macOS/Linux 终端、贡献者、CI | `scripts/soko.sh`（bash） |
| opencode / 任意 Node 运行时 | `.opencode/plugins/sokonanoda.ts`（Node，零 bash） |
| VS Code 用户 | VSIX 自带 per-target 二进制（无需引导） |

三条路最终都消费同一批 per-target Rust 二进制。

## 5. 能怎么把麻烦变小（建议，未实施）

1. **自检命令下放 CLI**：`sokonanoda version` / `doctor` / `self update`——
   一旦二进制到位就跨平台可用；但"首次获取"仍必须有引导器。
2. **引导脚本更标准**：改 POSIX `sh`（Alpine/BusyBox 可跑）或直接用 Node
   插件（已有）；避免 bash-only。
3. **用 `cargo-dist` / `cargo-binstall`** 生成 Homebrew / PowerShell / MSI /
   shell installer，并出 `SHA256SUMS` + artifact attestation，减少手写脚本。
4. **Windows 不要指望 bash**：走 VSIX / 插件（现状）。

## 6. 结论

"直接用 Rust 编一个跨平台二进制"在跨 OS 意义上不成立；成立的只是"每个平台
一个二进制"。`soko.sh` 不是替代二进制，而是"在还没有二进制时把它们弄到位"
的那一层。值得做的是把这层做薄、做标准（cargo-dist / POSIX sh / 自检命令
下放到 CLI），而不是消灭它。

## 参考

- rustup：`rustup-init.sh` 引导 + 单二进制分发 / 自更新
  （`rustup/src/cli/self_update.rs` 注释）。
- cargo-binstall：per-target 二进制安装；仅 macOS 提供
  `universal-apple-darwin`。
- cargo-dist：构建 per-target 二进制 + 多种安装器。
- Rust Forge《Other Installation Methods》：standalone installer 只有
  平台专属形态（tar.xz / .msi / .pkg）。

## 7. 后续更新（2026-09-11）

本调研之后，用户要求彻底去掉 `soko.sh`：环境能力（`version`/`doctor`/
`setup`/`update`/`grade`/`gate`/`lsp`）已全部搬进 `sokonanoda` 二进制
（内嵌 `ureq`(rustls/ring) + `flate2` + `tar` 下载器），`scripts/soko.sh`
删除。**首次获取**二进制仍由 opencode 插件（Node，跨平台）或 VSIX 自带完成；
插件是跨平台 bootstrap 的定位不变。设计见 `docs/design/binary-cli.md`。
第 5 节的"自检命令下放 CLI"已实现；引导器从脚本变成了 Node 插件。
