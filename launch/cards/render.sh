#!/usr/bin/env bash
# Render the fixed-size launch cards with headless Chrome.
#
#   ./render.sh
#
# Requires: Google Chrome (or set CHROME_BIN), ffmpeg (for the goal-panel crop),
# and macOS `sips` for the size assertion. No npm dependencies.
#
# Every card is a fixed viewport; the script fails if Chrome returns a different
# pixel size, so a layout regression cannot silently ship a wrong-sized asset.
set -euo pipefail
cd "$(dirname "$0")"

CHROME="${CHROME_BIN:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
if [ ! -x "$CHROME" ]; then
  echo "render.sh: Chrome not found at $CHROME (set CHROME_BIN)" >&2
  exit 2
fi

PROFILE="$(mktemp -d "${TMPDIR:-/tmp}/soko-cards.XXXXXX")"
trap 'rm -rf "$PROFILE"' EXIT

size() {
  sips -g pixelWidth -g pixelHeight "$1" 2>/dev/null |
    awk '/pixelWidth/{w=$2} /pixelHeight/{h=$2} END{print w"x"h}'
}

render() {
  local html="$1" out="$2" w="$3" h="$4"
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars \
    --force-device-scale-factor=1 --no-first-run --no-default-browser-check \
    --user-data-dir="$PROFILE" --window-size="$w,$h" --virtual-time-budget=3000 \
    --screenshot="../assets/$out" "file://$PWD/$html" >/dev/null 2>&1
  local got; got="$(size "../assets/$out")"
  if [ "$got" != "${w}x${h}" ]; then
    echo "render.sh: $out is $got, expected ${w}x${h}" >&2
    exit 1
  fi
  echo "  $out  ${w}x${h}"
}

echo "sync the demo loop from the site"
cp ../../site/assets/demo-vscode.gif ../assets/demo-vscode.gif

echo "crop goal panel from the refreshed hero"
ffmpeg -v error -y -i ../../site/assets/hero-vscode.png \
  -vf "crop=703:720:1320:0" ../assets/goal-panel.png

echo "render cards"
render og.html          og-card.png          1200 630
# The site's og:image is the same file: keep one render, two consumers.
cp ../assets/og-card.png ../../site/assets/og-card.png
render ph-1.html        ph-1-hero.png        1270 760
render ph-2.html        ph-2-kernel.png      1270 760
render ph-3.html        ph-3-courses.png     1270 760
render zh-cover.html    zh-cover.png         1200 630
render portrait.html    portrait.png         1080 1350
render icon.html        icon-240.png          240 240

echo "done"
