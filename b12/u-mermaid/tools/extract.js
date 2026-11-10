// Extract geometry from mermaid SVGs: cluster rects, node boxes, edge polylines, edge label boxes.
// usage: node extract.js <out.json> <file.svg>...
// All coordinates are px from the top-left of the SVG's viewBox (1 user unit = 1 px).
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path');
(async () => {
  const [out, ...files] = process.argv.slice(2);
  const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1, viewport: { width: 1600, height: 900 } });
  const res = {};
  for (const f of files) {
    const svg = fs.readFileSync(f, 'utf8');
    await p.setContent('<html><body style="margin:0;background:#fff"><div id=w style="display:inline-block"></div></body></html>');
    const r = await p.evaluate(s => {
      const w = document.getElementById('w'); w.innerHTML = s; const e = w.querySelector('svg');
      const vb = e.viewBox.baseVal; e.setAttribute('width', vb.width); e.setAttribute('height', vb.height); e.style.maxWidth = 'none';
      const o = e.getBoundingClientRect();
      const rel = q => { const r = q.getBoundingClientRect(); return { x: +(r.left - o.left).toFixed(1), y: +(r.top - o.top).toFixed(1), w: +r.width.toFixed(1), h: +r.height.toFixed(1) }; };
      const strip = id => id.replace(/^m\d+-/, '').replace(/^flowchart-/, '').replace(/^state-/, '').replace(/-\d+$/, '');
      const clusters = [...e.querySelectorAll('g.cluster, g.statediagram-cluster')].map(g => {
        const shape = g.querySelector(':scope > rect, :scope > g > rect, :scope > path, :scope > g > path, rect');
        const lab = g.querySelector('.cluster-label, .nodeLabel');
        return { id: g.id.replace(/^m\d+-/, ''), cls: g.getAttribute('class'), ...rel(shape || g), title: (lab ? lab.textContent : '').trim().replace(/\s+/g, ' ').slice(0, 60), titleBox: lab ? rel(lab) : null };
      });
      const nodes = [...e.querySelectorAll('g.node')].map(g => {
        const shape = g.querySelector('.label-container') || g;
        return { id: strip(g.id), raw: g.id.replace(/^m\d+-/, ''), cls: g.getAttribute('class'), ...rel(shape), text: g.textContent.trim().replace(/\s+/g, ' ').slice(0, 40) };
      });
      const edges = [...e.querySelectorAll('path[data-edge="true"], g.edgePaths > path, g.edgePaths path.flowchart-link, g.edgePaths path.transition')].filter((v, i, a) => a.indexOf(v) === i).map(pth => {
        let pts = null;
        try { pts = JSON.parse(atob(pth.getAttribute('data-points'))); } catch (err) { pts = null; }
        const m = pth.getCTM();
        const map = q => ({ x: +(m.a * q.x + m.c * q.y + m.e).toFixed(1), y: +(m.b * q.x + m.d * q.y + m.f).toFixed(1) });
        // also sample the drawn path (dense polyline) for through-block tests
        const L = pth.getTotalLength(); const n = Math.max(2, Math.ceil(L / 6)); const samp = [];
        for (let i = 0; i <= n; i++) { const q = pth.getPointAtLength(L * i / n); samp.push(map(q)); }
        return { id: (pth.getAttribute('data-id') || pth.id).replace(/^m\d+-/, ''), cls: pth.getAttribute('class'), len: +L.toFixed(1), d: pth.getAttribute('d').length, points: pts ? pts.map(map) : null, samp };
      });
      const labels = [...e.querySelectorAll('g.edgeLabel')].map(g => {
        const inner = g.querySelector('[data-id]'); const r = rel(g);
        return { id: inner ? inner.getAttribute('data-id') : '', ...r, text: g.textContent.trim().replace(/\s+/g, ' ').slice(0, 60) };
      }).filter(l => l.w > 1 && l.h > 1 && l.text);
      return { width: vb.width, height: vb.height, clusters, nodes, edges, labels };
    }, svg);
    res[path.basename(f)] = r;
    console.log(path.basename(f), 'clusters', r.clusters.length, 'nodes', r.nodes.length, 'edges', r.edges.length, 'labels', r.labels.length, 'size', Math.round(r.width) + 'x' + Math.round(r.height));
  }
  fs.writeFileSync(out, JSON.stringify(res));
  await b.close();
})();
