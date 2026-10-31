#!/bin/sh
# 用法：fallback-one.sh <binary> <file>  ⇒ 一行 JSON：strict 开关开时的退出码、诊断数、KERNEL_APP 计数
bin="$1"; f="$2"
c=$(mktemp -d /tmp/fb-cache.XXXXXX)
out=$(mktemp /tmp/fb-out.XXXXXX); err=$(mktemp /tmp/fb-err.XXXXXX)
env -u SOKO_STRICT_AT SOKONANODA_NO_PROJECT_ARTIFACTS=1 SOKONANODA_NO_MODULE_ARTIFACTS=1 \
  SOKONANODA_CACHE_DIR="$c" SOKO_JUDGE_STATS=1 SOKO_STRICT_KERNEL_APP=1 \
  perl -e 'alarm shift; exec @ARGV' 900 "$bin" grade --dialect strict "$f" --json >"$out" 2>"$err"
rc=$?
python3 -I - "$f" "$rc" "$out" "$err" <<'PY'
import json,re,sys
f,rc,out,err=sys.argv[1:]
diags=0
for l in open(out,errors="replace"):
    l=l.strip()
    if l.startswith("{"):
        try:
            if json.loads(l).get("type")=="diagnostic": diags+=1
        except Exception: pass
ka={}
for l in open(err,errors="replace"):
    if l.startswith("KERNEL_APP ") or l.startswith("KERNEL_APP_MISS "):
        tag=l.split()[0]
        for k,v in re.findall(r"(\w+)=(\d+)",l): ka[tag+"."+k]=ka.get(tag+"."+k,0)+int(v)
print(json.dumps({"file":f,"rc":int(rc) if rc.isdigit() else rc,"diags":diags,**ka}))
PY
rm -rf "$c" "$out" "$err"
