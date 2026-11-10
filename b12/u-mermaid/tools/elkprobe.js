// direct elkjs probes (no mermaid): node elkprobe.js
const ELK = require('/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/mm/node_modules/elkjs/lib/elk.bundled.js');
const elk = new ELK();
const N = (id, w = 100, h = 40, extra = {}) => ({ id, width: w, height: h, ...extra });
const E = (id, s, t) => ({ id, sources: [s], targets: [t] });
const base = { 'elk.algorithm': 'elk.layered', 'elk.direction': 'DOWN', 'elk.hierarchyHandling': 'INCLUDE_CHILDREN', 'spacing.baseValue': 40 };
const flat = (n, ox = 0, oy = 0, out = []) => { for (const c of n.children || []) { out.push(`${c.id}: x=${Math.round(c.x + ox)} y=${Math.round(c.y + oy)} w=${Math.round(c.width)} h=${Math.round(c.height)}`); flat(c, ox + c.x, oy + c.y, out); } return out; };
(async () => {
  // 1. alignment inside a layer by port ratio: tall T, source-only S, sink-only K, through M, isolated I all in one layer
  let g = { id: 'root', layoutOptions: base, children: [N('top'), N('T', 100, 200), N('S'), N('K'), N('M'), N('bot')],
    edges: [E('e1', 'top', 'T'), E('e2', 'T', 'bot'), E('e3', 'top', 'K'), E('e4', 'S', 'bot'), E('e5', 'top', 'M'), E('e6', 'M', 'bot')] };
  let r = await elk.layout(g); console.log('1. port-ratio alignment (T is 200 tall):\n  ' + flat(r).join('\n  '));
  // 2. child direction under INCLUDE_CHILDREN vs SEPARATE_CHILDREN
  for (const hh of ['INCLUDE_CHILDREN', 'SEPARATE_CHILDREN']) {
    g = { id: 'root', layoutOptions: base, children: [N('x'), { id: 'B', layoutOptions: { 'elk.direction': 'RIGHT', 'elk.hierarchyHandling': hh, 'elk.algorithm': 'elk.layered' }, children: [N('a'), N('b'), N('c')], edges: [] }, N('y')],
      edges: [E('e0', 'x', 'B'), E('e1', 'a', 'b'), E('e2', 'b', 'c'), E('e3', 'B', 'y')] };
    r = await elk.layout(g); console.log(`2. child elk.direction=RIGHT with ${hh}, edges to the block only:\n  ` + flat(r).join('\n  '));
  }
  // 3. block is one layer entry; a long edge passes beside it
  g = { id: 'root', layoutOptions: base, children: [N('s'), { id: 'B', children: [N('b1'), N('b2'), N('b3')], edges: [] }, N('t')],
    edges: [E('e0', 's', 'b1'), E('e1', 'b1', 'b2'), E('e2', 'b2', 'b3'), E('e3', 'b3', 't'), E('long', 's', 't')] };
  r = await elk.layout(g); console.log('3. block as one layer; long edge s->t:\n  ' + flat(r).join('\n  '));
  console.log('  long edge: ' + JSON.stringify(r.edges.find(e => e.id === 'long').sections.map(s => [s.startPoint, ...(s.bendPoints || []), s.endPoint].map(p => [Math.round(p.x), Math.round(p.y)]))));
  // 4. back edge between blocks, not pre-reversed
  g = { id: 'root', layoutOptions: base, children: [{ id: 'A', children: [N('a1'), N('a2')], edges: [] }, { id: 'B', children: [N('b1'), N('b2')], edges: [] }],
    edges: [E('e0', 'a1', 'a2'), E('e1', 'b1', 'b2'), E('fwd', 'a2', 'b1'), E('back', 'b2', 'a1')] };
  r = await elk.layout(g); console.log('4. block cycle, ELK alone (no pre-reversal):\n  ' + flat(r).join('\n  '));
  for (const id of ['fwd', 'back']) { const e = r.edges.find(e => e.id === id) || (r.children.flatMap(c => c.edges || []).find(e => e.id === id)); console.log('  ' + id + ': container=' + (e && e.container) + ' ' + JSON.stringify(e && e.sections.map(s => [s.startPoint, ...(s.bendPoints || []), s.endPoint].map(p => [Math.round(p.x), Math.round(p.y)])))); }
})().catch(e => console.log('ERR', e));
