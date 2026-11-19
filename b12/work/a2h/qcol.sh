#!/usr/bin/env bash
# qcol.sh <quality-bin> <dir> <col>...: print file, mode and the named columns
Q=$1; D=$2; shift 2
$Q $D 2>/dev/null | awk -v cols="$*" 'BEGIN{n=split(cols,want," ")} NR==1{for(i=1;i<=NF;i++) idx[$i]=i; next} {s=$1"\t"$2; for(k=1;k<=n;k++) s=s"\t"want[k]"="$(idx[want[k]]); print s}'
