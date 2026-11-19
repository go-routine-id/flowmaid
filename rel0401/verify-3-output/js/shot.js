// node shot.js <prefix> <dragged index> : renders <prefix>.{dev,head}.svg with rings at the given centres -> <prefix>.png (dev | head)
const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const [pre, k] = process.argv.slice(2);
  const given = JSON.parse(fs.readFileSync(pre + '.given.json', 'utf8'));
  const mark = s => {
    const rings = given.map((p, i) => `<circle cx="${p[0]}" cy="${p[1]}" r="${i == k ? 9 : 4}" fill="none" stroke="${i == k ? '#e00' : '#f80'}" stroke-width="${i == k ? 3 : 2}"/>` + (i == k ? `<path d="M ${p[0]-14} ${p[1]} L ${p[0]+14} ${p[1]} M ${p[0]} ${p[1]-14} L ${p[0]} ${p[1]+14}" stroke="#e00" stroke-width="1.5"/>` : '')).join('');
    return s.replace('</svg>', rings + '</svg>');
  };
  const dev = mark(fs.readFileSync(pre + '.dev.svg', 'utf8')), head = mark(fs.readFileSync(pre + '.head.svg', 'utf8'));
  const b = await chromium.launch(); const p = await b.newPage({ viewport: { width: 2400, height: 1600 } });
  await p.setContent(`<html><body style="margin:0;background:#ddd;font:14px sans-serif"><table><tr><td>origin/dev</td><td>HEAD</td></tr><tr><td style="vertical-align:top;background:#fff"><img src="data:image/svg+xml;base64,${Buffer.from(dev).toString('base64')}"></td><td style="vertical-align:top;background:#fff"><img src="data:image/svg+xml;base64,${Buffer.from(head).toString('base64')}"></td></tr></table></body></html>`);
  await (await p.$('table')).screenshot({ path: pre + '.png' });
  await b.close();
})();
