#!/usr/bin/env bash
# rpair.sh <label> <files.mmd...>: head (9f51f77 suite-on) | now (target-on) static pair pictures -> pic/<label>/
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2i
L=$1; shift; mkdir -p $W/pic/$L/head $W/pic/$L/now
names=()
for f in "$@"; do n=$(basename $f .mmd); names+=($n)
  $W/fm-on-g3r $f -o $W/pic/$L/head/$n.svg >/dev/null 2>&1
  $W/../../target-on/release/flowmaid $f -o $W/pic/$L/now/$n.svg >/dev/null 2>&1
done
cd $W && NODE_PATH=$(npm root -g) node pair.js pic/$L pic/$L/head "before" pic/$L/now "now" "${names[@]}"
