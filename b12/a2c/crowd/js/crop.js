const { chromium } = require('/opt/node-tools/node_modules/playwright');
const fs = require('fs');
const [out, x, y, w, h, ...svgs] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 4000, height: 2000 } });
  const cells = svgs.map(f => {
    let s = fs.readFileSync(f, 'utf8');
    s = s.replace(/<svg([^>]*?)viewBox="[^"]*"/, `<svg$1 viewBox="${x} ${y} ${w} ${h}"`).replace(/<svg([^>]*?) width="[^"]*"/, `<svg$1`).replace(/<svg([^>]*?) height="[^"]*"/, `<svg$1`);
    s = s.replace('<svg', `<svg width="${w}" height="${h}"`);
    return `<td style="vertical-align:top;border:1px solid #ccc"><div style="font:12px sans-serif">${f.split('/').slice(-4).join('/')}</div>${s}</td>`;
  });
  await p.setContent(`<html><body style="margin:0;background:#fff"><table id=t><tr>${cells.join('')}</tr></table></body></html>`);
  await (await p.$('#t')).screenshot({ path: out });
  await b.close();
})();
