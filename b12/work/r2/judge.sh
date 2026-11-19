#!/usr/bin/env bash
# judge.sh <outdir-name> [env...] : run fm + judge (A1), print must-fails and the crossings row
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
cd $S/b12/local; out=out/$1; shift
env "$@" ./run.sh fm $S/b12/target-on/release/flowmaid $out b12-r2 2>&1 | tail -0
./run.sh judge $out --due A1 2>&1 | awk '{print $1,$2,$3,$4,$5,$6,$7,$8,$9}' | grep -v "target missed" | tail -14
python3 - $out <<'PY'
import csv,sys
rows={r['name']:r for r in csv.DictReader(open(sys.argv[1]+'/metrics.tsv'), delimiter='\t')}
names=['infra-overview','infra-overview-v2','perm-a-edges-reversed','perm-b-subgraphs-reversed','perm-c-actors-reversed','perm-d-edges-rotated','perm-e-one-more-inner-edge','blocks-overview','blocks-overview-edges-reversed','blocks-overview-edges-rotated','blocks-overview-actors-reversed','blocks-overview-subgraphs-reversed','bench-75','dense-lr','dense-td','hub5','cycle2']
print('  crossings:', ' '.join(rows[n]['crossings'] for n in names))
print('  breadth  :', ' '.join(rows[n]['W'] for n in names[:12]))
PY
