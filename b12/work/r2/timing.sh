#!/usr/bin/env bash
# timing.sh <quality-binary> : scene_ms per file, two runs
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
for run in 1 2; do
"$1" $S/b12/work/r2/perfin 2>/dev/null | python3 -c "
import sys,csv
rows=list(csv.reader(sys.stdin, delimiter='\t'))
h=rows[0]; i=h.index('scene_ms'); j=h.index('x_drawn'); r=h.index('route_ms')
print(' '.join('%s=%s(x%s,r%s)'%(row[0][:10],row[i],row[j],row[r]) for row in rows[1:] if row[1]=='static' and row[0]!='TOTAL'))
print('   routed: '+' '.join('%s=r%s'%(row[0][:10],row[r]) for row in rows[1:] if row[1]=='routed' and row[0]!='TOTAL'))
"
done
