// usage: node render.js <outdir> [t0] [t1] [step]   (seconds; default: whole video at 30 fps)
// Also writes build/events.json (every sound cue, from the same timeline as the picture).
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path');
const S = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad';
const out = process.argv[2]; fs.mkdirSync(out, { recursive: true });
const FPS = 30;
const CLIPCFG = JSON.parse(fs.readFileSync(path.join(__dirname, 'clips.json'), 'utf8'));
function frames(dir) {
  return fs.readdirSync(dir).filter(f => /^f\d+\.png$/.test(f)).sort().map(f => 'file://' + path.join(dir, f));
}
function manifest(dir) { return JSON.parse(fs.readFileSync(path.join(dir, 'manifest.json'), 'utf8')); }
const svgdir = path.join(S, 'fvm/svg');
const TH = [['Flowchart','advanced'],['Sequence','sequence'],['Class','class'],['State','state'],
  ['Entity-relationship','er'],['Pie','pie'],['User journey','journey'],['Git graph','gitgraph'],
  ['Mindmap',null],['Timeline','timeline'],['Sankey','sankey'],['Architecture','architecture']];
const thumbs = TH.map(([label, f]) => ({ label,
  src: 'file://' + (f ? path.join(svgdir, f + '.svg') : path.join(S, 'hero/svg/mindmap.light.svg')) }));
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1080, height: 1350 }, deviceScaleFactor: 1 });
  await p.goto('file://' + path.join(__dirname, 'compose.html'));
  await p.evaluate(() => document.fonts.ready);
  const dirs = {}; for (const k of ['web', 'flowcli', 'dbx']) dirs[k] = path.join(S, 'vid/clips', CLIPCFG.dirs[k]);
  const mw = manifest(dirs.web), mc = manifest(dirs.flowcli), md = manifest(dirs.dbx);
  const cfg = Object.assign({}, CLIPCFG, {
    svg: fs.readFileSync(path.join(__dirname, 'hero.svg'), 'utf8'), thumbs,
    clips: { web: { frames: frames(dirs.web) }, flowcli: { frames: frames(dirs.flowcli) }, dbx: { frames: frames(dirs.dbx) } },
    webW: mw.width, webH: mw.height, cliW: mc.width, cliH: mc.height, dbxW: md.width, dbxH: md.height,
  });
  await p.evaluate(c => window.setup(c), cfg);
  fs.writeFileSync(path.join(__dirname, 'events.json'), JSON.stringify(await p.evaluate(() => window.events())));
  const dur = await p.evaluate(() => window.DURATION);
  const t0 = +(process.argv[3] ?? 0), t1 = +(process.argv[4] ?? dur), step = +(process.argv[5] ?? 1 / FPS);
  let i = 0;
  for (let k = 0; ; k++) {
    const t = t0 + k * step; if (t > t1 - 1e-9) break;
    await p.evaluate(t => window.render(t), t);
    await p.evaluate(() => window.ready());
    await p.screenshot({ path: path.join(out, String(i++).padStart(5, '0') + '.png') });
  }
  console.log('frames', i, 'dur', dur);
  await b.close();
})();
