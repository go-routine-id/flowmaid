// usage: node measure.js <titles.txt> : rendered bold 12px width (Chromium, Helvetica -> Liberation Sans) per line
const { chromium } = require('playwright'); const fs = require('fs');
(async () => {
  const lines = fs.readFileSync(process.argv[2], 'utf8').split('\n').filter(l => l.length);
  const b = await chromium.launch(); const p = await b.newPage();
  const esc = s => s.replace(/&/g,'&amp;').replace(/</g,'&lt;');
  const body = lines.map((l, i) => `<text id="t${i}" x="10" y="${20 + i * 20}" font-family="Helvetica, Arial, sans-serif" font-size="12" font-weight="bold">${esc(l)}</text>`).join('');
  await p.setContent(`<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="${lines.length * 20 + 40}">${body}</svg>`);
  const w = await p.evaluate(n => [...Array(n).keys()].map(i => document.getElementById('t' + i).getBBox().width), lines.length);
  lines.forEach((l, i) => console.log(`${w[i].toFixed(2)}\t${l}`));
  await b.close();
})();
