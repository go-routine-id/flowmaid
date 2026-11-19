#!/usr/bin/env bash
# Render corpus (or given files) in every theme -> $S/b5/th/<label>/<name>.<theme>.svg/png and <name>.grid.png
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
L=$1; shift; O=$S/b5/th/$L; mkdir -p $O
N=/home/user/flowmaid/target/release/flowmaid
files="$@"; [ -z "$files" ] && files=$(ls $S/cmp/corpus/*.mmd)
for f in $files; do n=$(basename $f .mmd); for t in flowmaid default neutral dark forest; do $N --theme $t $f > $O/$n.$t.svg 2>/dev/null; done; done
NODE_PATH=$(npm root -g) node -e '
const { chromium } = require("playwright"); const fs=require("fs"),path=require("path");
(async()=>{const [o,...f]=process.argv.slice(1);const b=await chromium.launch();const p=await b.newPage({deviceScaleFactor:1,viewport:{width:2600,height:1000}});
for(const x of f){const n=path.basename(x).replace(/\.mmd$/,"");
 let html="<html><body style=\"margin:0;background:#888\"><div id=w style=\"display:flex;flex-wrap:wrap;gap:8px;padding:8px;align-items:flex-start;max-width:2560px\">";
 for(const t of ["flowmaid","default","neutral","dark","forest"]){const s=path.join(o,n+"."+t+".svg"); if(fs.existsSync(s)) html+="<div>"+fs.readFileSync(s,"utf8")+"</div>";}
 html+="</div></body></html>"; await p.setContent(html); await (await p.$("#w")).screenshot({path:path.join(o,n+".grid.png")});}
await b.close();})();' $O $files
echo "-> $O"
