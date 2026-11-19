#!/usr/bin/env bash
# on.sh <label>: local acceptance static + judge A1 with the current target-on CLI (short output)
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
cd $B/local && ./run.sh fm $B/target-on/release/flowmaid out/$1 b12-$1 2>&1 | tail -1
./run.sh judge out/$1 --due A1 > out/$1/judge-a1.txt 2>&1
grep "MUST FAIL" out/$1/judge-a1.txt | cut -c1-200; tail -1 out/$1/judge-a1.txt
python3 - out/$1 <<'PY'
import csv,sys
rows={r['name']:r for r in csv.DictReader(open(sys.argv[1]+'/metrics.tsv'), delimiter='\t')}
names=['infra-overview','infra-overview-v2','perm-a-edges-reversed','perm-b-subgraphs-reversed','perm-c-actors-reversed','perm-d-edges-rotated','perm-e-one-more-inner-edge','blocks-overview','blocks-overview-edges-reversed','blocks-overview-edges-rotated','blocks-overview-actors-reversed','blocks-overview-subgraphs-reversed','bench-75','dense-lr','dense-td','hub5','cycle2']
print('  crossings:', ' '.join(rows[n]['crossings'] for n in names))
print('  must     : 20 17 20 23 20 20 22 7 7 7 7 7 325 51 74')
print('  breadth  :', ' '.join(rows[n]['W'] for n in names[:12]))
print('  xframe   :', ' '.join(n+'='+rows[n]['xframe_runs'] for n in rows if rows[n]['xframe_runs'] not in ('0','NA','')))
PY
