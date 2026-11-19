// node adv.js: bold advance (per 1000 em) of code point ranges, Helvetica -> system face, via canvas measureText
const { chromium } = require('playwright');
(async () => {
  const b = await chromium.launch(); const p = await b.newPage();
  await p.setContent('<canvas id=c></canvas>');
  const ranges = [[0x20,0x7e],[0xa0,0x24f],[0x370,0x3ff],[0x400,0x52f],[0x1e00,0x1eff],[0x2010,0x2044],[0x20a0,0x20bf],[0x2122,0x2122]];
  const out = await p.evaluate((ranges) => {
    const ctx = document.getElementById('c').getContext('2d');
    ctx.font = 'bold 1000px Helvetica, Arial, sans-serif';
    const r = [];
    for (const [a, z] of ranges) for (let cp = a; cp <= z; cp++) {
      const s = String.fromCodePoint(cp); const m = ctx.measureText(s);
      r.push([cp, m.width, m.actualBoundingBoxRight, s.normalize('NFD')[0], s.toUpperCase() !== s.toLowerCase() ? (s === s.toUpperCase() ? 'U' : 'L') : '-']);
    }
    return r;
  }, ranges);
  for (const [cp, w, ink, base, cs] of out) console.log(`${cp.toString(16)}\t${String.fromCodePoint(cp)}\t${w.toFixed(0)}\t${ink.toFixed(0)}\t${base}\t${cs}`);
  await b.close();
})();
