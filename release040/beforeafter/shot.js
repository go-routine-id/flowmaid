const { chromium } = require('playwright'); const fs = require('fs');
const rows = [
  ['fan6', 'Enam class menuju satu class', 'dua garis terluar melengkung dan memotong tetangganya (4 persilangan)', 'keenam garis bersarang rapi (0 persilangan)'],
  ['loops', 'Self-loop di antara class bersaudara', 'loop dan labelnya bertemu dengan loop tetangga di celah antar-kotak', 'loop dibagi ke sisi-sisi yang lebih lega'],
  ['dotted', 'Nama class bertitik', 'error: tidak bisa digambar', 'digambar sebagai satu class per nama'],
];
const esc = s => s.replace(/&/g,'&amp;').replace(/</g,'&lt;');
const cell = (n, v) => { const svg = fs.readFileSync(`${n}.${v}.svg`, 'utf8'); if (svg.trim()) return svg;
  return `<div class="err">${esc(fs.readFileSync(`${n}.${v}.err`, 'utf8').trim())}</div>`; };
let html = `<html><body style="margin:0;background:#fff;font-family:Inter,Arial,sans-serif"><div id="w" style="display:inline-block;padding:24px">
<style>h2{font-size:20px;margin:26px 0 10px} .row{display:flex;gap:28px;align-items:flex-start}
.col{border:1px solid #d8dbe2;border-radius:10px;padding:14px;background:#fff} .cap{font-weight:700;font-size:15px;margin-bottom:4px}
.sub{font-size:13px;color:#444;margin-bottom:10px;max-width:520px} .err{font-family:monospace;font-size:13px;color:#a11;background:#fdf0f0;padding:16px;border-radius:6px;max-width:480px}
svg{max-width:560px;height:auto}</style>`;
for (const [n, title, b, a] of rows) html += `<h2>${title}</h2><div class="row">
<div class="col"><div class="cap">Sebelum — 0.39.0</div><div class="sub">${b}</div>${cell(n,'039')}</div>
<div class="col"><div class="cap">Sesudah — 0.40.0</div><div class="sub">${a}</div>${cell(n,'040')}</div></div>`;
html += '</div></body></html>';
(async () => { const br = await chromium.launch(); const p = await br.newPage({ deviceScaleFactor: 2, viewport: { width: 1500, height: 900 } });
  await p.setContent(html); await (await p.$('#w')).screenshot({ path: 'flowmaid-0.40.0-before-after.png' }); await br.close(); })();
