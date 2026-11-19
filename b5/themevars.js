const { chromium } = require('playwright');
const mm = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/mm/node_modules/mermaid/dist/mermaid.min.js';
(async () => {
  const b = await chromium.launch(); const p = await b.newPage();
  await p.setContent('<html><body></body></html>');
  await p.addScriptTag({ path: mm });
  const out = {};
  for (const t of ['default','neutral','dark','forest','base']) {
    out[t] = await p.evaluate(async (t) => {
      mermaid.initialize({ startOnLoad: false, theme: t });
      // render something to force theme computing
      await mermaid.render('x'+Math.floor(Math.random()*1e9), 'flowchart TD\nA-->B');
      const c = mermaid.mermaidAPI.getConfig();
      return c.themeVariables;
    }, t);
  }
  console.log(JSON.stringify(out, null, 1));
  await b.close();
})();
