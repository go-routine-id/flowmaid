const { chromium } = require('playwright'); const fs = require('fs');
const rows = process.argv.slice(2);
const titles = {subgraph: 'Contoh subgraph di README (subgraph.mmd)', fixture: 'Contoh publik ber-subgraph (blocks-overview)', analog: 'Analog diagram infrastruktur Anda (nama netral, bukan data asli)', 'dense-td': 'dense-td (diagram uji padat)'};
const svg = f => fs.readFileSync(f, 'utf8');
const png = f => `<img src="data:image/png;base64,${fs.readFileSync(f).toString('base64')}">`;
let html = `<html><body style="margin:0;background:#fff;font-family:Inter,Arial,sans-serif"><div id="w" style="display:inline-block;padding:22px">
<style>h2{font-size:19px;margin:22px 0 8px}.row{display:flex;gap:22px;align-items:flex-start}.col{border:1px solid #d8dbe2;border-radius:10px;padding:12px;background:#fff}
.cap{font-weight:700;font-size:14px;margin-bottom:8px}.col svg,.col img{width:520px;height:auto;display:block}</style>`;
for (const n of rows) html += `<h2>${titles[n]}</h2><div class="row">
<div class="col"><div class="cap">Hari ini (0.40.0)</div>${svg(n + '.040.svg')}</div>
<div class="col"><div class="cap">Blok, langkah 1 selesai (ukuran &amp; posisi: langkah 2)</div>${svg(n + '.blk.svg')}</div>
<div class="col"><div class="cap">mermaid 12.2</div>${png(n + '.mm.png')}</div></div>`;
html += '</div></body></html>';
(async () => { const b = await chromium.launch(); const p = await b.newPage({ deviceScaleFactor: 1.5, viewport: { width: 1800, height: 900 } });
  await p.setContent(html); await (await p.$('#w')).screenshot({ path: process.env.OUT }); await b.close(); })();
