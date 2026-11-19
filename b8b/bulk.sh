#!/usr/bin/env bash
# usage: bulk.sh <label> <flowmaid-binary> <quality-binary>
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
L=$1; BIN=$2; Q=$3; V=$S/verify/b8-layering
O=$S/b8b/bulk/$L; rm -rf $O; mkdir -p $O/svg $O/chg
cd $V && cat $S/b8b/list.txt | xargs -P4 -I{} sh -c 'n=$(basename {} .mmd); timeout 60 '$BIN' {} > '$O'/svg/$n.svg 2>/dev/null; echo "$? $n"' > $O/status
for f in $O/svg/*.svg; do n=$(basename $f .svg); cmp -s $f $S/b8b/bulk/o035/$n.svg || echo $n; done > $O/changed.txt
wc -l < $O/changed.txt
# copy changed flowchart/state inputs for quality
while read n; do f=$S/b8b/bulkin/$n.mmd; head -c 400 $f | grep -qiE '^\s*(flowchart|graph|stateDiagram)' && cp $f $O/chg/; done < $O/changed.txt
ls $O/chg | wc -l
$Q $O/chg > $O/q.new.tsv 2>/dev/null
$S/b8b/tgt035/release/examples/quality $O/chg > $O/q.035.tsv 2>/dev/null
$S/b8b/tgt034/release/examples/quality $O/chg > $O/q.034.tsv 2>/dev/null
