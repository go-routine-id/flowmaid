#!/usr/bin/env bash
# hl.sh <in.svg> <out.png> "a → b" ...  : highlight edges (red, wide) and screenshot
set -euo pipefail
IN=$1; OUT=$2; shift 2
python3 - "$IN" "$OUT.svg" "$@" <<'PY'
import sys,re
src,dst,*names=sys.argv[1:]
s=open(src).read()
for n in names:
    pat=re.compile(r'(<path d="[^"]*" fill="none" stroke=")[^"]*(" stroke-width=")[^"]*("[^>]*><title>'+re.escape(n)+r'</title>)')
    s,k=pat.subn(r'\1#e8003a\2 4\3',s)
    if k==0: print('no match',n,file=sys.stderr)
open(dst,'w').write(s)
PY
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs");
(async()=>{const [s,o]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1});
await p.setContent(`<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block">${fs.readFileSync(s,"utf8")}</div></body></html>`);
await (await p.$("#w")).screenshot({path:o});await b.close();})();' "$OUT.svg" "$OUT"
