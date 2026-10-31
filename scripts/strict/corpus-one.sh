#!/bin/sh
# 用法：corpus-one.sh <binary> <dialect> <file>  ⇒ 打印 "<dialect> <file> <stdout sha256 前 16 位>"
# 冷跑：两个 NO_*_ARTIFACTS + 每次新的全局缓存目录 + 去掉继承的 SOKO_STRICT_AT（同 scripts/strict-score.py）。
bin="$1"; d="$2"; f="$3"
c=$(mktemp -d /tmp/corpus-cache.XXXXXX)
h=$(env -u SOKO_STRICT_AT SOKONANODA_NO_PROJECT_ARTIFACTS=1 SOKONANODA_NO_MODULE_ARTIFACTS=1 \
  SOKONANODA_CACHE_DIR="$c" perl -e 'alarm shift; exec @ARGV' 900 "$bin" grade --dialect "$d" "$f" --json 2>/dev/null \
  | shasum -a 256 | cut -c1-16)
rm -rf "$c"
echo "$d $f $h"
