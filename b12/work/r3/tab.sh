#!/usr/bin/env bash
# tab.sh <quality-binary> [max-starts]: static x_drawn per forced start count, then granted + scene_ms
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
Q=$1; MAX=${2:-9}
T=$(mktemp -d)
for k in $(seq 1 $MAX) g; do
  if [ $k = g ]; then "$Q" $B/work/tab > $T/$k.tsv 2>/dev/null; else "$Q" $B/work/tab --starts $k > $T/$k.tsv 2>/dev/null; fi
done
python3 - $T $MAX <<'PY'
import csv,sys
T=sys.argv[1]; MAX=int(sys.argv[2])
tabs={}
for k in list(map(str,range(1,MAX+1)))+['g']:
    rows=list(csv.reader(open(f'{T}/{k}.tsv'),delimiter='\t'))
    h=rows[0]; xi=h.index('x_drawn'); si=h.index('scene_ms')
    tabs[k]={r[0]:(r[xi],r[si]) for r in rows[1:] if r[1]=='static'}
names=[n for n in tabs['g'] if n!='TOTAL']
print('%-38s'%'input',' '.join('s%-3s'%k for k in range(1,MAX+1)),'granted scene_ms')
for n in names:
    print('%-38s'%n,' '.join('%-4s'%tabs[str(k)][n][0] for k in range(1,MAX+1)),'%-7s'%tabs['g'][n][0],tabs['g'][n][1])
PY
rm -rf $T
