#!/usr/bin/env bash
# 本地性能报告（与 CI 的 "Performance report" 步骤同一口径）。
#
# 用法：scripts/perf-report.sh [输出文件]
#   缺省输出 perf-report-v<version>-<sha>.txt
#
# 阈值断言（缩放比/延迟）由测试本身强制执行——本脚本只负责把实测值
# 连同版本与 commit 留档，便于跨版本对比「哪个改动引入了回退」。
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
sha=$(git rev-parse --short HEAD)
out="${1:-perf-report-v${version}-${sha}.txt}"

{
  echo "# sokonanoda perf report"
  echo "version: v${version}  commit: $(git rev-parse HEAD)  date: $(date -u +%FT%TZ)"
  echo "# 阈值断言（scaling ratio / latency）由 cargo test 强制执行；"
  echo "# 下面是本轮实测值（front = 编译器内核路径，lsp = 编辑器交互路径）。"
  echo ""
  echo "## compiler (crates/front/tests/perf.rs)"
  cargo test -p sokonanoda-front --test perf -- --nocapture 2>&1 | grep "^PERF" || true
  echo ""
  echo "## lsp interaction (crates/lsp/src/lib.rs perf_*)"
  cargo test -p sokonanoda-lsp --lib -- perf_ --nocapture 2>&1 | grep "^PERF" || true
} | tee "$out"

echo ""
echo "report written: $out"
