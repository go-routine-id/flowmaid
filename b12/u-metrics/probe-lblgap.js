// probe: per label, distance from the chip box to the nearest foreign edge path
const { execFileSync } = require('child_process');
const fs = require('fs');
for (const f of process.argv.slice(2)) {
  const out = JSON.parse(execFileSync('./blockmetrics', ['--struct-dir', 'struct', '--json', f], { maxBuffer: 1 << 28 }).toString());
  const m = out[0];
  console.log(f, m.detail.labels.map(l => l.gap === undefined ? '?' : l.gap.toFixed(0)).join(' '));
}
