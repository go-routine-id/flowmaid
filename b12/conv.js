const { chromium } = require('playwright'); const fs = require('fs');
(async () => { const b = await chromium.launch();
  for (const [i, o] of [['1.webp','user-shot-flowmaid038.png'],['2.webp','user-shot-mermaid122.png']]) {
    const data = fs.readFileSync('/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/images/' + i).toString('base64');
    const p = await b.newPage({ viewport: { width: 2000, height: 1300 }, deviceScaleFactor: 1 });
    await p.setContent(`<html><body style="margin:0"><img id=i src="data:image/webp;base64,${data}"></body></html>`);
    await p.waitForFunction(() => document.getElementById('i').complete && document.getElementById('i').naturalWidth > 0);
    await (await p.$('#i')).screenshot({ path: 'private/' + o }); await p.close(); console.log('ok', o);
  } await b.close(); })();
