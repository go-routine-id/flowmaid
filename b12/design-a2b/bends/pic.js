// usage: node pic.js <out.png> <svg> <struct-dir> <edge> <title> <hand-count> [zoom x0,y0,x1,y1]
// One picture: the edge as drawn, its straight runs of 8 px or more coloured (each run its own colour,
// runs under 8 px thin grey), every counted bend numbered where one run ends and the next begins,
// for the live yardstick and the corrected counter side by side; optionally a zoom on one corner with
// the run boundaries marked; and the whole diagram with the edge in red.
const b = require(__dirname + '/fixed/blockmetrics');
const fs = require('fs'), path = require('path');
const { chromium } = require('playwright');
const [out, svgf, sd, want, title, hand, zoom] = process.argv.slice(2);
const name = path.basename(svgf).replace(/\.mermaid\.svg$|\.svg$/, '');
const stf = path.join(sd, name + '.struct.json');
const r = b.analyse(svgf, { struct: fs.existsSync(stf) ? stf : null });
const e = r.model.edges.find(x => x.name === want);
if (!e) { console.error('no edge', want); process.exit(1); }
const segs = e.segs;
const PAL = ['#1f77b4', '#e6550d', '#2ca02c', '#d62728', '#9467bd', '#8c564b', '#e377c2', '#17becf', '#bcbd22', '#393b79', '#637939', '#843c39', '#7b4173', '#3182bd'];
const live = b.anchoredRuns(segs);
const fixAll = b.straightRuns(segs);
const fixTurn = b.turnRuns(segs);
// corrected: runs that count are the turnRuns; a corner piece taken out by stage 3 is drawn hatched-grey
const keyOf = rr => rr.k0 + ':' + rr.k1;
function panel(runs, counted, W, H, box, label, opts = {}) {
  const [x0, y0, x1, y1] = box;
  const sc = Math.min((W - 40) / (x1 - x0), (H - 70) / (y1 - y0));
  const ox = 20 + ((W - 40) - sc * (x1 - x0)) / 2, oy = 50 + ((H - 70) - sc * (y1 - y0)) / 2;
  const P = (x, y) => [ox + (x - x0) * sc, oy + (y - y0) * sc];
  const pl = (k0, k1) => { const pts = [P(segs[k0].ax, segs[k0].ay)]; for (let k = k0; k <= k1; k++) pts.push(P(segs[k].bx, segs[k].by)); return pts.map(p => p[0].toFixed(1) + ',' + p[1].toFixed(1)).join(' '); };
  let s = `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" style="background:#fff;border:1px solid #ccc">`;
  s += `<text x="10" y="20" font-family="sans-serif" font-size="14" font-weight="bold">${label}</text>`;
  s += `<clipPath id="c${opts.id}"><rect x="0" y="40" width="${W}" height="${H - 40}"/></clipPath><g clip-path="url(#c${opts.id})">`;
  s += `<polyline points="${pl(0, segs.length - 1)}" fill="none" stroke="#e4e4e4" stroke-width="${opts.base || 9}" stroke-linejoin="round"/>`;
  let ci = 0;
  const cset = new Set(counted.map(keyOf));
  for (const rr of runs) {
    if (rr.len >= b.BEND_RUN_MIN && cset.has(keyOf(rr))) { s += `<polyline points="${pl(rr.k0, rr.k1)}" fill="none" stroke="${PAL[ci % PAL.length]}" stroke-width="${opts.w || 4}" stroke-linecap="butt"/>`; ci++; }
    else if (rr.len >= b.BEND_RUN_MIN) s += `<polyline points="${pl(rr.k0, rr.k1)}" fill="none" stroke="#999" stroke-width="${opts.w || 4}" stroke-dasharray="3 3"/>`;
    else s += `<polyline points="${pl(rr.k0, rr.k1)}" fill="none" stroke="#777" stroke-width="1.2"/>`;
  }
  // run ends of the counted runs
  for (const rr of counted) for (const [x, y] of [[rr.ax, rr.ay], [rr.bx, rr.by]]) { const p = P(x, y); s += `<circle cx="${p[0]}" cy="${p[1]}" r="2.6" fill="#fff" stroke="#000" stroke-width="1"/>`; }
  // numbered bends between consecutive counted runs
  for (let i = 1; i < counted.length; i++) {
    const a = P(counted[i - 1].bx, counted[i - 1].by), c = P(counted[i].ax, counted[i].ay);
    const m = [(a[0] + c[0]) / 2, (a[1] + c[1]) / 2];
    const nx = m[0] + (opts.dx || 13), ny = m[1] - (opts.dy || 13);
    s += `<line x1="${m[0]}" y1="${m[1]}" x2="${nx}" y2="${ny}" stroke="#000" stroke-width="0.8"/>`;
    s += `<circle cx="${nx}" cy="${ny}" r="9" fill="#fff8c4" stroke="#000" stroke-width="1"/><text x="${nx}" y="${ny + 4}" text-anchor="middle" font-family="sans-serif" font-size="11" font-weight="bold">${i}</text>`;
  }
  s += '</g></svg>';
  return s;
}
const bb = e.bbox, pad = Math.max(30, 0.06 * Math.max(bb.w, bb.h));
const box = [bb.x - pad, bb.y - pad, bb.x + bb.w + pad, bb.y + bb.h + pad];
const liveCounted = live.filter(x => x.len >= b.BEND_RUN_MIN);
const W = 560, H = 560;
let html = `<html><body style="margin:0;background:#fff;font:13px sans-serif"><div id="w" style="display:inline-block;padding:10px">`;
html += `<div style="font-size:16px;font-weight:bold;margin-bottom:4px">${title}</div>`;
html += `<div style="margin-bottom:8px">live yardstick: <b>${Math.max(0, liveCounted.length - 1)}</b> bends &nbsp;|&nbsp; corrected counter: <b>${Math.max(0, fixTurn.length - 1)}</b> &nbsp;|&nbsp; counted by hand from the path: <b>${hand}</b>. Coloured = a straight run of 8 px or more (one colour per run); thin grey = runs under 8 px; dashed grey = a corner piece the corrected counter takes out (stage 3); yellow numbers = the bends each counter counts.</div>`;
html += `<div style="display:flex;gap:10px;align-items:flex-start">`;
html += panel(live, liveCounted, W, H, box, 'live: run line = its first segment', { id: 1 });
html += panel(fixAll, fixTurn, W, H, box, 'corrected: run line = its longest segment', { id: 2 });
html += `</div>`;
if (zoom) {
  const z = zoom.split(',').map(Number);
  html += `<div style="display:flex;gap:10px;margin-top:10px;align-items:flex-start">`;
  html += panel(live, liveCounted, W, 300, z, `zoom (${z.join(', ')}): live`, { id: 3, w: 6, base: 14, dx: 0, dy: 22 });
  html += panel(fixAll, fixTurn, W, 300, z, `zoom: corrected`, { id: 4, w: 6, base: 14, dx: 0, dy: 22 });
  html += `</div>`;
}
// context: the whole diagram with the edge in red
const src = fs.readFileSync(svgf, 'utf8');
const vb = (src.match(/viewBox="([^"]+)"/) || [])[1];
if (vb) {
  const [vx, vy, vw, vh] = vb.split(/[ ,]+/).map(Number);
  const cw = 1130, ch = Math.min(700, cw * vh / vw), s2 = Math.min(cw / vw, ch / vh);
  const pts = [[segs[0].ax, segs[0].ay], ...segs.map(g => [g.bx, g.by])].map(p => ((p[0] - vx) * s2).toFixed(1) + ',' + ((p[1] - vy) * s2).toFixed(1)).join(' ');
  const inner = src.replace(/^<\?xml[^>]*>/, '').replace(/<svg([^>]*?)\swidth="[^"]*"/, '<svg$1').replace(/<svg([^>]*?)\sheight="[^"]*"/, '<svg$1').replace('<svg', `<svg width="${(vw * s2).toFixed(0)}" height="${(vh * s2).toFixed(0)}"`);
  html += `<div style="margin-top:10px"><b>the whole drawing, the edge in red</b><div style="position:relative;width:${(vw * s2).toFixed(0)}px;height:${(vh * s2).toFixed(0)}px;border:1px solid #ccc">${inner}<svg style="position:absolute;left:0;top:0" width="${(vw * s2).toFixed(0)}" height="${(vh * s2).toFixed(0)}"><polyline points="${pts}" fill="none" stroke="#e00" stroke-width="4" opacity="0.8"/></svg></div></div>`;
}
html += `</div></body></html>`;
(async () => {
  const br = await chromium.launch(); const p = await br.newPage({ viewport: { width: 1200, height: 900 } });
  await p.setContent(html); await (await p.$('#w')).screenshot({ path: out });
  await br.close();
  fs.writeFileSync(out.replace(/\.png$/, '.html'), html);
  console.log('wrote', out);
})();
