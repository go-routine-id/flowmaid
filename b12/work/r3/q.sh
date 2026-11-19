#!/usr/bin/env bash
# q.sh <label> [quality-binary]: harness rows over the fixture dirs and the local inputs -> $B/work/r3/q/<label>.tsv
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
Q=${2:-$B/target-on/release/examples/quality}; mkdir -p $B/work/r3/q; out=$B/work/r3/q/$1.tsv; : > $out
for d in /home/user/flowmaid/tests/fixtures/quality /home/user/flowmaid/tests/fixtures/notes /home/user/flowmaid/tests/fixtures/blocks /home/user/flowmaid/examples $B/local/in; do
  "$Q" $d 2>/dev/null | awk -v d=$(basename $d) 'NR==1{print "dir\t" $0; next} $1!="TOTAL"{print d "\t" $0}' >> $out
done
wc -l < $out
