#!/usr/bin/env bash
# rs.sh <bindir> <outdir> <files.mmd...>: suite-on static SVGs via the CLI, routed via quality --svg
set -uo pipefail
bin=$1; out=$2; shift 2; mkdir -p $out
for f in "$@"; do n=$(basename $f .mmd); $bin/flowmaid $f > $out/$n.svg 2>/dev/null; done
