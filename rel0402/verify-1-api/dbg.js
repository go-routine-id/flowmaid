const { chromium } = require('playwright'); const fs = require('fs');
(async () => { const b = await chromium.launch(); const p = await b.newPage(); await p.setContent('<div id=w></div>');
const J = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const r = await p.evaluate((J) => { const host = document.getElementById('w'); host.innerHTML = J.parts[0].svg; const svg = host.querySelector('svg');
 return { head: J.parts[0].svg.slice(0, 600), els: [...svg.querySelectorAll('*')].map(e => { let bb=null; try { const q=e.getBBox(); bb=[q.x,q.y,q.width,q.height]; } catch(err){} return e.tagName + ' ' + JSON.stringify(bb) + ' fill=' + e.getAttribute('fill') + ' inFill(0.6,0.4)=' + (e.isPointInFill ? e.isPointInFill(new DOMPoint(0.61,0.37)) : '-'); }) }; }, J);
console.log(JSON.stringify(r, null, 1)); await b.close(); })();
