#!/usr/bin/env bash
# qcol.sh <tsv>: key columns (file mode defects x_drawn fan_x compact_max compact_mean sib_stagger lateral_x spine_off min_edge_gap)
awk -F'\t' 'NR==1{for(i=1;i<=NF;i++)c[$i]=i; next}{printf "%-22s %-7s def %-3s x %-4s fan %-3s cmax %-6s cmean %-6s stag %-3s lat %-3s spine %-3s gap %s\n",$1,$2,$c["defects"],$c["x_drawn"],$c["fan_x"],$c["compact_max"],$c["compact_mean"],$c["sib_stagger"],$c["lateral_x"],$c["spine_off"],$c["min_edge_gap"]}' $1
