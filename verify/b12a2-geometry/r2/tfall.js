const { chromium } = require('playwright'); const fs = require('fs');
(async () => { const b = await chromium.launch(); const p = await b.newPage();
 for (const f of process.argv.slice(2)) { await p.setContent(`<html><body style="margin:0">${fs.readFileSync(f,'utf8')}</body></html>`);
  const r = await p.evaluate(() => { const rects=[...document.querySelectorAll('rect[rx="8"]')].map(r=>({x:+r.getAttribute('x'),y:+r.getAttribute('y'),w:+r.getAttribute('width')}));
   return [...document.querySelectorAll('text[font-weight="bold"]')].map(t=>{const bb=t.getBBox();const tx=+t.getAttribute('x');const fr=rects.filter(r=>tx>=r.x&&tx<=r.x+r.w).sort((a,b)=>Math.abs(a.x+10-tx)-Math.abs(b.x+10-tx))[0];return t.textContent+' right_slack='+(fr.x+fr.w-(bb.x+bb.width)).toFixed(1);}); });
  console.log(f, r.join(' | ')); } await b.close(); })();
