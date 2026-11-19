// usage: node scenetruth.js <struct-dir> <svg-dir> [names...]
// Independent check: the scene's own polyline (struct.scene.edges[i].points, the SAME build as the SVG)
// gives each routed edge's axis turns exactly (no painting, no sampling). Compare live and corrected bends.
// scene_turns = direction changes of the polyline; scene_turns8 = the turns left when a straight piece
// whose painted straight part (piece minus the rounding the painter takes off at its corners, read from
// the SVG's own corner cubics is not available here, so: piece minus 2 x the shorter neighbouring
// rounding leg) is under BEND_RUN_MIN is dropped: the yardstick's own rule.
const b = require(__dirname + '/fixed2/blockmetrics');
const fs = require('fs'), path = require('path');
const [sd, dir, ...only] = process.argv.slice(2);
const files = (only.length ? only : fs.readdirSync(dir).filter(f => f.endsWith('.svg')).map(f => f.slice(0, -4)));
const tally = { edges: 0, routed: 0, liveEq: 0, fixEq: 0, fixEq8: 0, liveOver: 0, fixOver: 0, fixUnder: 0 };
const rows = [];
for (const n of files) {
  const stf = path.join(sd, n + '.struct.json');
  if (!fs.existsSync(stf)) continue;
  const st = JSON.parse(fs.readFileSync(stf, 'utf8'));
  const r = b.analyse(path.join(dir, n + '.svg'), { struct: stf });
  const sc = st.scene.edges;
  if (sc.length !== r.model.edges.length) { rows.push(`${n}\tSKIP scene ${sc.length} vs svg ${r.model.edges.length}`); continue; }
  r.model.edges.forEach((e, i) => {
    tally.edges++;
    const se = sc[i];
    if (!se.routed || !e.segs || !e.segs.length) return;
    tally.routed++;
    // maximal collinear pieces of the scene polyline
    const P = se.points.filter((p, k, a) => k === 0 || Math.hypot(p[0] - a[k - 1][0], p[1] - a[k - 1][1]) > 1e-6);
    const pieces = [];
    for (let k = 1; k < P.length; k++) {
      const dx = P[k][0] - P[k - 1][0], dy = P[k][1] - P[k - 1][1], L = Math.hypot(dx, dy);
      const last = pieces[pieces.length - 1];
      if (last && Math.abs(last.ux * dy - last.uy * dx) < 1e-3 * (1 + L) && last.ux * dx + last.uy * dy > 0) { last.len += L; last.segs.push(L); continue; }
      pieces.push({ ux: dx / L, uy: dy / L, len: L, segs: [L] });
    }
    const turns = Math.max(0, pieces.length - 1);
    const live = Math.max(0, b.anchoredRuns(e.segs).filter(x => x.len >= b.BEND_RUN_MIN).length - 1);
    const fix = Math.max(0, b.turnRuns(e.segs).length - 1);
    if (live === turns) tally.liveEq++; if (fix === turns) tally.fixEq++;
    if (live > turns) tally.liveOver++; if (fix > turns) tally.fixOver++; if (fix < turns) tally.fixUnder++;
    const shortest = Math.min(...pieces.map(p => p.len));
    rows.push([n, e.name, `scene ${turns}`, `live ${live}`, `fixed ${fix}`, `shortest piece ${shortest.toFixed(1)}`, `hops ${e.hops}`, `arcs ${(e.d.match(/A /g) || []).length}`].join('\t'));
  });
}
fs.writeFileSync(path.join(__dirname, 'data', 'scenetruth2-' + path.basename(dir) + '.tsv'), rows.join('\n') + '\n');
console.log(JSON.stringify(tally));
