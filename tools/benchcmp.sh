#!/usr/bin/env bash
# Per-file render benchmark: flowmaid (render_svg, release build) vs mermaid 12.2.0 (headless Chromium).
# usage: benchcmp.sh [repo=/home/user/flowmaid] [corpus=$S/cmp/corpus] [runs=30]
# Writes $S/runs/bench/{flowmaid.tsv,mermaid.tsv,compare.md}.
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
REPO=${1:-/home/user/flowmaid}
CORPUS=${2:-$S/cmp/corpus}
RUNS=${3:-30}
OUT=$S/runs/bench
mkdir -p "$OUT"
FILES=$(ls "$CORPUS"/*.mmd | LC_ALL=C sort)

(cd "$REPO" && cargo build --release --example bench -q)
"$REPO/target/release/examples/bench" files --runs "$RUNS" $FILES > "$OUT/flowmaid.tsv"
export NODE_PATH=$(npm root -g)
node "$S/tools/mmbench.js" "$RUNS" $FILES > "$OUT/mermaid.tsv"
CHROMIUM=$(node -e "require('playwright').chromium.launch().then(async b=>{console.log(b.version());await b.close()})")

CPU=$(grep -m1 'model name' /proc/cpuinfo | sed 's/.*: //')
NPROC=$(nproc)
RUSTC=$(cd "$REPO" && rustc --version)

python3 -I - "$OUT" "$RUNS" "$NPROC" "$CPU" "$RUSTC" "$CHROMIUM" <<'PY'
import sys, statistics
out, runs, nproc, cpu, rustc, chromium = sys.argv[1:]
def load(p):
    rows = {}
    with open(p) as f:
        hdr = f.readline().rstrip('\n').split('\t')
        for line in f:
            r = dict(zip(hdr, line.rstrip('\n').split('\t')))
            rows[r['name']] = r
    return rows
fm, mm = load(f'{out}/flowmaid.tsv'), load(f'{out}/mermaid.tsv')
def num(s):
    try: return float(s)
    except (TypeError, ValueError): return None
lines = ['| diagram | type | flowmaid ms | mermaid ms | speed-up × | flowmaid SVG KB | mermaid SVG KB |',
         '|---|---|--:|--:|--:|--:|--:|']
ups = []
for name, f in fm.items():
    m = mm.get(name, {})
    fms, mms = num(f.get('median_ms')), num(m.get('median_ms'))
    fkb, mkb = num(f.get('svg_bytes')), num(m.get('svg_bytes'))
    up = mms / fms if fms and mms else None
    if up: ups.append(up)
    c = lambda v, fmt: fmt.format(v) if v is not None else 'n/a'
    lines.append(f"| {name} | {f['type']} | {c(fms, '{:.2f}')} | {c(mms, '{:.1f}')} | "
                 f"{c(up, '{:.0f}×')} | {c(fkb and fkb / 1024, '{:.1f}')} | {c(mkb and mkb / 1024, '{:.1f}')} |")
geo = statistics.geometric_mean(ups) if ups else float('nan')
note = f"""
**Method.** Each cell is the median of {runs} timed renders after warm-ups (flowmaid 3, mermaid 2) of the same
`.mmd` source. flowmaid: `flowmaid::render_svg` (parse + layout + SVG string) in a release build
(`cargo run --release --example bench -- files`). mermaid: `mermaid.render()` 12.2.0 (parse + layout + SVG string,
including its in-browser text measurement) in headless Chromium {chromium} via Playwright. Speed-up = mermaid median
÷ flowmaid median; geometric mean over {len(ups)} diagrams: **{geo:.0f}×**. SVG KB = length of the returned SVG string (all-ASCII here, so chars = bytes).
Machine: {nproc} CPUs, {cpu}; {rustc}. One render at a time on both sides; timings on a shared box are noisy (±10–20 %).
"""
open(f'{out}/compare.md', 'w').write('\n'.join(lines) + '\n' + note)
print('\n'.join(lines) + '\n' + note)
PY
