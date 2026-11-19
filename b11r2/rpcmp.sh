#!/usr/bin/env bash
# rpcmp.sh <runs> <binA> <binB> [files...]: interleaved runs of `tp rp`, per-frame min over runs; prints per-file worst and frames over 6 ms
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
R=$1; A=$2; B=$3; shift 3
FILES=${@:-"$S/flowcorpus/bench-75.mmd $S/flowcorpus/dense-td.mmd $S/flowcorpus/dense-lr.mmd"}
T=$(mktemp -d)
for r in $(seq $R); do
  DUMP=$T/a.tsv REPS=7 $A rp $FILES >/dev/null
  DUMP=$T/b.tsv REPS=7 $B rp $FILES >/dev/null
done
python3 - $T/a.tsv $T/b.tsv <<'PY'
import sys, collections
def load(p):
    d = collections.defaultdict(lambda: 1e9)
    for l in open(p):
        f, n, node, ms = l.rstrip("\n").split("\t")
        k = (f, int(n), node)
        d[k] = min(d[k], float(ms))
    return d
a, b = load(sys.argv[1]), load(sys.argv[2])
files = sorted({k[0] for k in a})
for f in files:
    ka = [k for k in a if k[0] == f]
    wa = max(ka, key=lambda k: a[k]); wb = max(ka, key=lambda k: b[k])
    ma = sum(a[k] for k in ka) / len(ka); mb = sum(b[k] for k in ka) / len(ka)
    oa = sum(1 for k in ka if a[k] > 6.0); ob = sum(1 for k in ka if b[k] > 6.0)
    top = sorted(ka, key=lambda k: -b[k])[:5]
    print(f"{f}: A worst {a[wa]:.2f} (frame {wa[1]} {wa[2]}) mean {ma:.3f} over6 {oa} | B worst {b[wb]:.2f} (frame {wb[1]} {wb[2]}) mean {mb:.3f} over6 {ob}")
    print("   B top frames: " + " ".join(f"{k[1]}:{k[2]} A={a[k]:.2f} B={b[k]:.2f}" for k in top))
PY
rm -rf $T
