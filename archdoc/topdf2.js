const { chromium } = require('playwright');
(async () => { const [inp, out] = process.argv.slice(2); const b = await chromium.launch(); const p = await b.newPage();
  await p.goto('file://' + inp, { waitUntil: 'networkidle' }); await p.evaluate(() => document.fonts.ready);
  await p.pdf({ path: out, format: 'A4', printBackground: true, preferCSSPageSize: true, displayHeaderFooter: true,
    headerTemplate: '<div></div>',
    footerTemplate: '<div style="width:100%;font-family:Inter,sans-serif;font-size:8px;color:#8a93a3;padding:0 20mm;display:flex;justify-content:space-between"><span>RFC-001 · Vibe coding di dalam chat</span><span><span class="pageNumber"></span> / <span class="totalPages"></span></span></div>' });
  await b.close(); })();
