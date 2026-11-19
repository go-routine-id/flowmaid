#!/usr/bin/env bash
# svgs.sh <flowmaid-bin> <outdir> <dir-or-files...> : render every .mmd to <outdir>/<name>.svg (static)
BIN=$1; O=$2; shift 2; mkdir -p $O
for a in "$@"; do
  if [ -d "$a" ]; then fs=$(ls $a/*.mmd); else fs=$a; fi
  for f in $fs; do n=$(basename $f .mmd); $BIN $f > $O/$n.svg 2>/dev/null || true; done
done
