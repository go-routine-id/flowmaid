#!/usr/bin/env bash
# tcmp.sh <quality-bin> <dir>: routed route_ms / drag_ms per file (default build), with --drag
$1 $2 --drag 2>/dev/null | awk 'NR==1{for(i=1;i<=NF;i++) c[$i]=i; next} $2=="routed"{printf "%s r=%s d=%s  ", $1, $(c["route_ms"]), $(c["drag_ms"])} END{print ""}'
