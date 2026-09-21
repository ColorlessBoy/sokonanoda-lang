#!/usr/bin/env bash
# **环节循环**：把「改完 Rust → 在真 VS Code 里看到效果」压成一条命令。
#
# 为什么需要它：计划（`docs/design/vscode-editor-feedback-plan.md` §0.4）把每环节的
# 反馈分成 5 层，其中 L4「真 VS Code 单用例 e2e」与"肉眼看效果"都要先把二进制换掉。
# 换二进制有两条路，各有各的坑，本脚本只做**编排 + 提示**，不发明新机制。
#
# 用法：
#   scripts/dev-loop.sh lsp            # 编 debug 的 LSP + CLI，并打印在 VS Code 里怎么让它生效
#   scripts/dev-loop.sh stage-debug    # 把 debug 构建 stage 进 editor/vscode/bin/<target>/
#   scripts/dev-loop.sh ext            # 只改扩展代码时：打印 F5 开发宿主的步骤
#   scripts/dev-loop.sh check          # 只检查当前环境（二进制在不在、serverOverride 怎么设）
#
# 退出码：0 = 编排成功；2 = 用法错误或前置缺失（cargo 不在等）。
#
# 两条路（细节见 docs/vscode-dev-guide.md「环节循环」一节）：
#
#   A. **serverOverride 回路**（Rust 侧改动，推荐）
#      用户设置里一次性开 `sokonanoda.serverOverride: true` +
#      `sokonanoda.serverPath` 指向本仓 `target/debug/sokonanoda-lsp`；
#      之后每环节只要 `scripts/dev-loop.sh lsp` + 命令面板 `sokonanoda: restart server`。
#      **限制**：这只换服务器二进制，**换不了扩展代码** ⇒ 客户端改动看不到。
#
#   B. **F5 开发宿主**（扩展侧改动）
#      `editor/vscode` 里按 F5；`bin/` 不存在时解析落到 `target/debug`。
#      改 `extension.js` 按重载按钮。要回到这条路先 `npm run clean:lsp`。

set -u
cd "$(dirname "$0")/.." || exit 2

REPO="$PWD"
EXT="$REPO/editor/vscode"

# macOS 上如果 Xcode 许可没同意，`cc` 找不到 SDK、**任何** Rust 链接都会失败
# （`error: linking with cc failed: exit status: 69`）。CommandLineTools 不需要那个
# 许可，指过去即可。这里只在**真的坏了**的时候提示，不静默改环境。
sdk_hint() {
  if [ "$(uname -s)" != "Darwin" ]; then return 0; fi
  if xcrun --show-sdk-path >/dev/null 2>&1; then return 0; fi
  if [ -d /Library/Developer/CommandLineTools ]; then
    echo "⚠ macOS SDK 不可用（多半是 Xcode 许可没同意）——本次构建会失败。两种解法：" >&2
    echo "    a) 正解：sudo xcodebuild -license accept" >&2
    echo "    b) 绕过：export DEVELOPER_DIR=/Library/Developer/CommandLineTools" >&2
    echo "  （b 只影响本 shell；本脚本不会替你改环境）" >&2
  fi
}

target() {
  case "$(uname -s)/$(uname -m)" in
    Darwin/arm64) echo "darwin-arm64" ;;
    Darwin/x86_64) echo "darwin-x64" ;;
    Linux/x86_64) echo "linux-x64" ;;
    Linux/aarch64 | Linux/arm64) echo "linux-arm64" ;;
    MINGW* | MSYS* | CYGWIN*) echo "win32-x64" ;;
    *) echo "" ;;
  esac
}

report_paths() {
  echo
  echo "二进制："
  for f in target/debug/sokonanoda-lsp target/debug/sokonanoda; do
    if [ -f "$f" ]; then
      printf '  ✓ %-34s %s\n' "$f" "$(stat -f '%Sm' -t '%Y-%m-%d %H:%M:%S' "$f" 2>/dev/null || stat -c '%y' "$f" 2>/dev/null || echo '?')"
    else
      printf '  ✗ %-34s （还没编）\n' "$f"
    fi
  done
  tgt="$(target)"
  if [ -n "$tgt" ] && [ -f "$EXT/bin/$tgt/sokonanoda-lsp" ]; then
    echo "  ⚠ editor/vscode/bin/$tgt/ 里有 staged 的二进制——F5 会优先用它（要回到 target/debug 先 npm run clean:lsp）"
  fi
}

case "${1:-}" in
  lsp)
    command -v cargo >/dev/null 2>&1 || { echo "error: 需要 cargo（贡献者路径见 skills/sokonanoda-dev）" >&2; exit 2; }
    sdk_hint
    echo "+ cargo build -p sokonanoda-lsp -p sokonanoda-cli（debug）"
    cargo build -p sokonanoda-lsp -p sokonanoda-cli || exit 2
    report_paths
    cat <<'EOF'

在 VS Code 里让它生效（**不用重装 VSIX、不用重载窗口**）：
  1) 一次性设置（用户设置 JSON）：
       "sokonanoda.serverOverride": true,
       "sokonanoda.serverPath": "<本仓绝对路径>/target/debug/sokonanoda-lsp"
     （不设 serverOverride 的话，扩展会优先用它自带的 bundled 服务器，这份 debug 构建被忽略）
  2) 每个环节：命令面板 → `sokonanoda: restart server`
     （它走与激活同一条解析链，回执会显示重启前后的版本与 pid）

看不到效果的两种情况：
  * 改的是 `editor/vscode/extension.js` 等扩展代码 ⇒ 这条路换不了扩展代码，用 `scripts/dev-loop.sh ext`
  * `editor/vscode/bin/<target>/` 里有 staged 的旧二进制 ⇒ `cd editor/vscode && npm run clean:lsp`
EOF
    ;;
  stage-debug)
    [ -d "$EXT/node_modules" ] || { echo "error: 先 (cd editor/vscode && npm install)" >&2; exit 2; }
    echo "+ node editor/vscode/scripts/stage-lsp.js --profile debug"
    (cd "$EXT" && node scripts/stage-lsp.js --profile debug) || exit 2
    report_paths
    echo
    echo "已 stage。真宿主 e2e（scripts/vscode-e2e.sh）用的就是 bin/ 里这份。"
    ;;
  ext)
    cat <<'EOF'
扩展侧改动的回路（F5 开发宿主）：
  1) cd editor/vscode && npm run clean:lsp      # 让解析落到 target/debug（否则 bin/ 优先）
  2) editor/vscode 里按 F5（Run Extension）——开发宿主窗口里 extensionPath 是本仓
  3) 改 extension.js 后按开发宿主的**重载**按钮；改 Rust 后先 scripts/dev-loop.sh lsp，
     再在开发宿主里 `sokonanoda: restart server`
  4) 秒级回归：node editor/vscode/test-extension-host.js（stub 宿主）
EOF
    report_paths
    ;;
  check)
    sdk_hint
    report_paths
    echo
    echo "当前解析来源（只读）："
    scripts/soko version --json 2>/dev/null | python3 -c "
import json,sys
try:
    d=json.load(sys.stdin)
except Exception:
    print('  （scripts/soko version --json 跑不出来）'); raise SystemExit(0)
for k in ('cli','lsp'):
    v=d.get(k) or {}
    print(f\"  {k:4s}: {v.get('source','?'):12s} {v.get('path','?')}\")
"
    ;;
  -h | --help | "")
    sed -n '2,30p' "$0"
    [ -n "${1:-}" ] || exit 2
    ;;
  *)
    echo "error: unknown command: $1（lsp / stage-debug / ext / check）" >&2
    exit 2
    ;;
esac
