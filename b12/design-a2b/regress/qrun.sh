#!/usr/bin/env bash
# qrun.sh <cfg>: quality TSVs of one configuration over every family -> m/<fam>-<cfg>.tsv
# cfg: pre (9c142b3) | t78 (7892fb7) | on (7122a53, probe build) | r1 (split by plain room) | r1t1 (+ title-side nesting) | t1
REG=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/regress
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a2-geometry/r6
cfg=$1
case $cfg in
  pre) Q=$V/pre-on-target/release/examples/quality; E="";;
  t78) Q=$V/t78-target/release/examples/quality; E="";;
  on) Q=$REG/bin/q-probe; E="";;
  r1) Q=$REG/bin/q-probe; E="FM_RFIX=1";;
  r1t1) Q=$REG/bin/q-probe; E="FM_RFIX=1 FM_TFIX=1";;
  t1) Q=$REG/bin/q-probe; E="FM_TFIX=1";;
  proto) Q=$REG/bin/q-proto; E="";;
esac
for fam in bulkst:$S/b8r1/bulkst bulkfc:$S/b8r1/bulkfc all:$S/b3r4/all dirs:$S/b12/work/a2i/dirs dirsx:$S/b12/work/a2i/dirsx stress:$V/stress; do
  n=${fam%%:*}; d=${fam#*:}
  env $E $Q $d > $REG/m/$n-$cfg.tsv 2> $REG/m/$n-$cfg.err
done
echo done $cfg
