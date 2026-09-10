---
description: 下载与仓库版本锁定的 sokonanoda CLI + LSP 二进制（零 cargo，agent/headless 环境搭建）
agent: build
---

把与当前仓库版本锁定的两个二进制下载到缓存——它们就是可直接执行的
程序（**不需要 Rust/cargo、不需要 VS Code 扩展**）：

```bash
V=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)   TARGET=aarch64-apple-darwin ;;
  Darwin-x86_64)  TARGET=x86_64-apple-darwin ;;
  Linux-x86_64)   TARGET=x86_64-unknown-linux-gnu ;;
  Linux-aarch64)  TARGET=aarch64-unknown-linux-gnu ;;
  *) TARGET=x86_64-pc-windows-msvc ;;
esac
if [ -f /etc/alpine-release ]; then
  case "$(uname -m)" in
    aarch64) TARGET=aarch64-unknown-linux-musl ;;
    *)       TARGET=x86_64-unknown-linux-musl ;;
  esac
fi
BIN="$HOME/.local/share/sokonanoda/bin"; mkdir -p "$BIN"
for pkg in sokonanoda-cli sokonanoda-lsp; do
  curl -fsSL "https://github.com/ColorlessBoy/sokonanoda-lang/releases/download/v${V}/${pkg}-${TARGET}.tar.gz" \
    | tar xz -C "$BIN"
done
"$BIN/sokonanoda" --version
```

- 不要用 `releases/latest`：URL 必须锁定 `v${V}`（版本错配是明确要避免的故障）；
- opencode 的 LSP launcher 会自动复用 `$BIN/sokonanoda-lsp`；
- 离线或自建场景（贡献者）见 `skills/sokonanoda-dev`。
