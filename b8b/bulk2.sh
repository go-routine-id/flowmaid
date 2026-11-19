#!/usr/bin/env bash
# usage: bulk2.sh <label> <flowmaid-binary> <quality-binary>
# renders all bulk inputs, finds those changed vs 0.35.0, quality for new (parallel) + cached 034/035 rows
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
L=$1; BIN=$2; Q=$3; V=$S/verify/b8-layering; C=$S/b8b/qcache
O=$S/b8b/bulk/$L; rm -rf $O; mkdir -p $O/svg $O/chg $C/034 $C/035
cd $V && cat $S/b8b/list.txt | xargs -P4 -I{} sh -c 'n=$(basename {} .mmd); timeout 60 '$BIN' {} > '$O'/svg/$n.svg 2>/dev/null; echo "$? $n"' > $O/status
for f in $O/svg/*.svg; do n=$(basename $f .svg); cmp -s $f $S/b8b/bulk/o035/$n.svg || echo $n; done > $O/changed.txt
echo "changed: $(wc -l < $O/changed.txt)"
while read n; do f=$S/b8b/bulkin/$n.mmd; head -c 400 $f | grep -qiE '^\s*(flowchart|graph|stateDiagram)' && cp $f $O/chg/; done < $O/changed.txt
echo "flow/state: $(ls $O/chg | wc -l)"
# split into 4 chunks
for i in 0 1 2 3; do mkdir -p $O/c$i; done
k=0; for f in $O/chg/*.mmd; do cp $f $O/c$((k%4))/; k=$((k+1)); done
need() { for f in $O/chg/*.mmd; do n=$(basename $f .mmd); [ -s $C/$1/$n.tsv ] || echo $f; done; }
for v in 034 035; do mkdir -p $O/n$v; for f in $(need $v); do cp $f $O/n$v/; done; done
for i in 0 1 2 3; do $Q $O/c$i > $O/q.new.$i.tsv 2>/dev/null & done
wait
for v in 034 035; do
  if [ -n "$(ls $O/n$v)" ]; then
    for i in 0 1 2 3; do mkdir -p $O/n$v/p$i; done; k=0; for f in $O/n$v/*.mmd; do mv $f $O/n$v/p$((k%4))/; k=$((k+1)); done
    for i in 0 1 2 3; do $S/b8b/tgt$v/release/examples/quality $O/n$v/p$i > $O/q.$v.$i.tsv 2>/dev/null & done; wait
    for i in 0 1 2 3; do python3 - $O/q.$v.$i.tsv $C/$v <<'PY'
import sys,csv
rows=list(csv.reader(open(sys.argv[1]),delimiter='\t'))
if rows:
    h=rows[0]
    by={}
    for r in rows[1:]:
        if len(r)==len(h) and r[0]!='TOTAL': by.setdefault(r[0],[]).append(r)
    for n,rs in by.items():
        with open(f"{sys.argv[2]}/{n}.tsv",'w') as f:
            f.write('\t'.join(h)+'\n'); [f.write('\t'.join(r)+'\n') for r in rs]
PY
    done
  fi
  python3 - $O $C/$v $O/q.$v.tsv <<'PY'
import sys,os,glob
O,C,out=sys.argv[1:]
h=None; lines=[]
for f in sorted(glob.glob(O+'/chg/*.mmd')):
    n=os.path.basename(f)[:-4]; p=f"{C}/{n}.tsv"
    if not os.path.exists(p): continue
    L=open(p).read().splitlines(); h=L[0]; lines+=L[1:]
open(out,'w').write((h+'\n' if h else '')+'\n'.join(lines)+'\n')
PY
done
head -1 $O/q.new.0.tsv > $O/q.new.tsv; for i in 0 1 2 3; do tail -n +2 $O/q.new.$i.tsv | grep -v '^TOTAL' >> $O/q.new.tsv; done
echo done
