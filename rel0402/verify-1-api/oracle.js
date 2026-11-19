// SVG-geometry oracle for the #112 pick methods: isPointInFill/isPointInStroke
// on the painter's own SVG of each isolated (text-stripped) element.
const { chromium } = require('playwright');
const fs = require('fs');
const files = process.argv.slice(2);
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  await page.setContent('<html><body><div id=w></div></body></html>');
  const summary = [];
  for (const f of files) {
    const J = JSON.parse(fs.readFileSync(f, 'utf8'));
    const res = await page.evaluate((J) => {
      const pts = J.pts; const N = pts.length / 2;
      const ty = J.type;
      const host = document.getElementById('w');
      const skip = new Set(['text','tspan','title','desc','defs','pattern','linearGradient','stop','g','svg','radialGradient']);
      const parseId = (s) => s;
      const rankOf = (id, el, idx, nEls) => {
        const m = id.match(/^([A-Za-z]+)\((\d+)\)$/);
        const num = (k) => parseInt(k);
        if (ty === 'mindmap' || ty === 'sankey') return m[1] === 'Link' ? [0, num(m[2])] : [1, num(m[2])];
        if (ty === 'pie') {
          if (id.startsWith('Slice')) return [0, num(m[2])];
          if (id.startsWith('Label')) return [1, parseInt(id.match(/label: (\d+)/)[1])];
          return [2, num(m[2])];
        }
        if (ty === 'journey') { const g = {Section:0, Task:1, Legend:2}[m[1]]; return [g, num(m[2])]; }
        if (ty === 'timeline') {
          if (id.startsWith('Section')) { const s = num(m[2]); return el.tagName === 'rect' ? [s, 0, 0] : [s, 1, 0]; }
          const s = parseInt(id.match(/section: (\d+)/)[1]), k = parseInt(id.match(/period: (\d+)/)[1]);
          return el.tagName === 'circle' ? [s, 2 + k, 1] : [s, 2 + k, 0];
        }
        if (ty === 'seq') {
          const k = num(m[2]);
          switch (m[1]) {
            case 'Frame': if (el.tagName === 'line') return [2, k]; if (el.tagName === 'rect' && el.getAttribute('fill') === 'none') return [0, k]; return [3, k];
            case 'Lifeline': return [1, k]; case 'Activation': return [4, k]; case 'Note': return [6, k];
            case 'Message': return [7, k]; case 'Participant': return [8, k]; case 'Footer': return [9, k];
          }
        }
        throw new Error('rank ' + ty + ' ' + id);
      };
      // which children of an isolated svg belong to the part
      const keep = (id, el) => {
        if (ty === 'timeline' && id.startsWith('Period')) {
          if (el.tagName === 'circle') return true;
          if (el.tagName === 'line' && el.getAttribute('stroke-dasharray')) return true;
          return false;
        }
        if (ty === 'pie' && el.tagName === 'line') return false; // separators: cut-outs
        if (el.tagName === 'rect' && !el.hasAttribute('x') && !el.hasAttribute('y')) return false; // canvas background
        if (el.tagName === 'rect' && (parseFloat(el.getAttribute('width')) === 0 || parseFloat(el.getAttribute('height')) === 0)) return false;
        return true;
      };
      const parts = [];
      for (const P of J.parts) {
        host.innerHTML = P.svg;
        const svg = host.querySelector('svg');
        const els = [...svg.querySelectorAll('*')].filter(e => !skip.has(e.tagName) && !e.closest('defs') && keep(P.id, e));
        const geo = [];
        for (const el of els) {
          const fill = el.getAttribute('fill');
          const filled = ['rect','circle','path','polygon','ellipse','polyline'].includes(el.tagName) && fill !== 'none';
          const stroke = el.getAttribute('stroke');
          const stroked = stroke && stroke !== 'none';
          const sw = parseFloat(el.getAttribute('stroke-width') || '1');
          let bb; try { bb = el.getBBox(); } catch (e) { continue; }
          geo.push({ el, filled, stroked, sw, bb, rank: rankOf(P.id, el), inside: (ty === 'seq' && P.id.startsWith('Frame') && el.tagName === 'rect' && fill === 'none') });
        }
        parts.push({ P, geo, svg: svg.cloneNode(true) });
      }
      // membership of a point in a part's geometry at tolerance tol; returns best rank or null
      const memb = (part, x, y, tol, strokeOnFilled) => {
        let best = null;
        const pt = new DOMPoint(x, y);
        for (const g of part.geo) {
          const pad = g.sw / 2 + tol + 1.5;
          if (x < g.bb.x - pad || x > g.bb.x + g.bb.width + pad || y < g.bb.y - pad || y > g.bb.y + g.bb.height + pad) continue;
          let on = false;
          if (g.filled && g.el.isPointInFill(pt)) on = true;
          if (!on && g.stroked && (!g.filled || strokeOnFilled)) {
            if (!g._w || g._w.tol !== tol) {
              const c = g.el.cloneNode(true);
              c.setAttribute('stroke-width', String(g.sw + 2 * (g.filled ? 0 : tol)));
              c.setAttribute('stroke-linecap', 'round');
              c.setAttribute('stroke-linejoin', 'round');
              c.removeAttribute('stroke-dasharray');
              g.el.parentNode.appendChild(c);
              g._w = { tol, c };
            }
            if (g._w.c.isPointInStroke(pt)) on = true;
          }
          if (on && (!best || cmp(g.rank, best) > 0)) best = g.rank;
        }
        return best;
      };
      const cmp = (a, b) => { for (let i = 0; i < Math.max(a.length, b.length); i++) { const d = (a[i] ?? -1) - (b[i] ?? -1); if (d) return d; } return 0; };
      // re-attach each part's svg so cloned stroke elements live in a document
      host.innerHTML = '';
      for (const part of parts) { host.appendChild(part.svg); const els = [...part.svg.querySelectorAll('*')].filter(e => !skip.has(e.tagName) && !e.closest('defs') && keep(part.P.id, e)); let i = 0; for (const g of part.geo) { g.el = els[i++]; g._w = null; } }
      const out = { file: J.file, type: ty, parts: J.parts.length, iso: { checked0: 0, bad0: 0, checked4: 0, bad4: 0, amb: 0, outline: 0, samples: [] }, full: { checked: 0, bad: 0, amb: 0, samples: [] }, extra: J.extra.filter(e => e[1] !== e[2] && e[2] !== null).map(e => e.join(' | ')) };
      const near = [[0.15,0],[-0.15,0],[0,0.15],[0,-0.15],[0.11,0.11],[-0.11,0.11],[0.11,-0.11],[-0.11,-0.11]];
      // iso comparisons
      for (const part of parts) {
        const set0 = new Set(part.P.h0), set4 = new Set(part.P.h4);
        let minx = Infinity, miny = Infinity, maxx = -Infinity, maxy = -Infinity;
        for (const g of part.geo) { minx = Math.min(minx, g.bb.x); miny = Math.min(miny, g.bb.y); maxx = Math.max(maxx, g.bb.x + g.bb.width); maxy = Math.max(maxy, g.bb.y + g.bb.height); }
        const pad = 10;
        const cand = new Set([...set0, ...set4]);
        for (let i = 0; i < N; i++) { const x = pts[2*i], y = pts[2*i+1]; if (x >= minx - pad && x <= maxx + pad && y >= miny - pad && y <= maxy + pad) cand.add(i); }
        for (const i of cand) {
          const x = pts[2*i], y = pts[2*i+1];
          for (const [tol, set, key] of [[0, set0, '0'], [4, set4, '4']]) {
            const b = memb(part, x, y, tol, false) !== null;
            out.iso['checked' + key]++;
            if (b !== set.has(i)) {
              const amb = near.some(([dx, dy]) => (memb(part, x + dx, y + dy, tol, false) !== null) !== b);
              if (amb) { out.iso.amb++; continue; }
              if (!b && set.has(i) && memb(part, x, y, tol, true) !== null) { out.iso.outline++; }
              out.iso['bad' + key]++;
              if (out.iso.samples.length < 12) out.iso.samples.push(`${part.P.id} tol${tol} (${x.toFixed(2)},${y.toFixed(2)}) rust=${set.has(i)} svg=${b}`);
            }
          }
        }
      }
      // full-scene z-order at tol 0
      const partBoxes = parts.map(part => { let a = [Infinity, Infinity, -Infinity, -Infinity]; for (const g of part.geo) { a = [Math.min(a[0], g.bb.x), Math.min(a[1], g.bb.y), Math.max(a[2], g.bb.x + g.bb.width), Math.max(a[3], g.bb.y + g.bb.height)]; } return a; });
      const expectAt = (x, y) => {
        let best = null, bestId = null;
        parts.forEach((part, pi) => {
          const bx = partBoxes[pi];
          if (x < bx[0] - 8 || x > bx[2] + 8 || y < bx[1] - 8 || y > bx[3] + 8) return;
          const r = memb(part, x, y, 0, false);
          let id = part.P.id;
          let rr = r;
          if (r === null && ty === 'seq' && id.startsWith('Frame')) {
            const g = part.geo.find(g => g.inside);
            if (g && g.el.isPointInFill(new DOMPoint(x, y))) { rr = [-1, parseInt(id.match(/\d+/)[0])]; id = id.replace('Frame', 'FrameInside'); }
          }
          if (rr !== null && (!best || cmp(rr, best) > 0)) { best = rr; bestId = id; }
        });
        return bestId;
      };
      for (let i = 0; i < N; i += (N > 60000 ? 2 : 1)) {
        const x = pts[2*i], y = pts[2*i+1];
        const e = expectAt(x, y);
        out.full.checked++;
        if (e !== J.full0[i]) {
          if (near.some(([dx, dy]) => expectAt(x + dx, y + dy) !== e)) { out.full.amb++; continue; }
          out.full.bad++;
          if (out.full.samples.length < 12) out.full.samples.push(`(${x.toFixed(2)},${y.toFixed(2)}) rust=${J.full0[i]} svg=${e}`);
        }
      }
      return out;
    }, J);
    summary.push(res);
    console.log(JSON.stringify(res));
  }
  await browser.close();
})();
