#!/usr/bin/env bash
# pic.sh <tag> <set/name> [labels...]: side by side of s-<label>/<set>/svg/<name>.svg
F=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12a2b/fr4
tag=$1; n=$2; shift 2; labs=${*:-base head B}
set=${n%%/*}; name=${n#*/}; args=()
for l in $labs; do args+=($F/s-$l/$set/svg/$name.svg); done
NODE_PATH=$(npm root -g) node $F/three.js $F/pic/$tag-$name.png "${args[@]}"
echo $F/pic/$tag-$name.png
