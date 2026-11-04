// Exit (printing the verdict) when a new verify:* result lands in the workflow journal.
// usage: node waitverify.js <journal.jsonl>
const fs = require('fs'); const f = process.argv[2];
const read = () => { const lab = {}; const res = [];
  for (const l of fs.readFileSync(f, 'utf8').trim().split('\n')) { let e; try { e = JSON.parse(l) } catch { continue }
    if (e.type === 'started') lab[e.key] = e.label;
    if (e.type === 'result' && (lab[e.key] || '').startsWith('verify:')) res.push(lab[e.key] + ' -> ' + (e.result && e.result.verdict)); }
  return res; };
const seen = read().length;
const t = setInterval(() => { const r = read(); if (r.length > seen) { console.log(r.slice(seen).join('\n')); clearInterval(t); } }, 15000);
