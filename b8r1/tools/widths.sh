#!/usr/bin/env bash
# usage: widths.sh <bin> files... -> prints name w h, then min/max width spread
B=$1; shift
for f in "$@"; do vb=$($B $f 2>/dev/null | grep -o 'viewBox="[^"]*"' | head -1 | tr -d '"' | cut -d' ' -f3,4); echo "$(basename $f .mmd) $vb"; done | awk '{print; w[NR]=$2; if(min==""||$2<min)min=$2; if($2>max)max=$2; if(hmin==""||$3<hmin)hmin=$3; if($3>hmax)hmax=$3} END {printf "WIDTH %d..%d (+%.0f%%)  HEIGHT %d..%d (+%.0f%%)\n", min, max, (max/min-1)*100, hmin, hmax, (hmax/hmin-1)*100}'
