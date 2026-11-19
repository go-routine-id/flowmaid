#!/usr/bin/env bash
# usage: cfg.sh <label> <flowmaid-bin> <quality-bin> [ENV=VAL ...]
# hashes outputs of bulkfc/gen/gen2 + fixtures/corpus, quality on changed files only vs head
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; cd $S/b8r1
L=$1; FB=$2; QB=$3; shift 3
rm -rf chg/$L; mkdir -p chg/$L/in svg/$L
for f in bulkfc/*.mmd gen/*.mmd gen2/*.mmd; do n=$(basename $(dirname $f))_$(basename $f .mmd); h=$(env "$@" timeout 20 $FB $f 2>/dev/null | md5sum | cut -c1-32); echo $h > svg/$L/$n; [ "$h" != "$(cat svg/head/$n)" ] && cp $f chg/$L/in/$n.mmd; done
for f in /home/user/flowmaid/tests/fixtures/quality/*.mmd $S/flowcorpus/*.mmd /home/user/flowmaid/examples/*.mmd $S/cmp/corpus/*.mmd; do a=$(bin/flowmaid-head $f 2>/dev/null | md5sum); b=$(env "$@" $FB $f 2>/dev/null | md5sum); [ "$a" != "$b" ] && echo "CORPUS-CHANGED $f"; done | sort -u > chg/$L/corpus.txt
echo "changed: $(ls chg/$L/in | wc -l)"; cat chg/$L/corpus.txt
( bin/quality-head chg/$L/in > chg/$L/q.head.tsv 2>/dev/null & env "$@" $QB chg/$L/in > chg/$L/q.new.tsv 2>/dev/null & wait )
python3 sumq.py chg/$L/q.head.tsv chg/$L/q.new.tsv defects,x_drawn,edge_frame_x,edge_node_x,area_per_node
