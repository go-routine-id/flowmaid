const path = require('path'), fs = require('fs');
const [bmf, sd, svgf, want] = process.argv.slice(2);
const b = require(bmf);
const name = path.basename(svgf).replace(/\.svg$/, '');
const stf = path.join(sd, name + '.struct.json');
const r = b.analyse(svgf, { struct: fs.existsSync(stf) ? stf : null });
const e = r.model.edges.find(x => x.name === want);
const d = b.turnRuns(e.segs).filter(x => x.len >= b.BEND_RUN_MIN).map(x => { const dx = x.bx - x.ax, dy = x.by - x.ay; return Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? 'right' : 'left') : (dy > 0 ? 'down' : 'up'); });
console.log(want, d.join(' '));
