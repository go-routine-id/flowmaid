// usage: node unbridged.js <blockmetrics copy dir> <struct-dir> file.svg ... : edges whose drawn hops (arc pairs) exceed the bridged count
const b = require(process.argv[2] + '/blockmetrics');
const fs = require('fs'), path = require('path');
const [, , mod, sd, ...files] = process.argv;
for (const f of files) {
  const name = path.basename(f).replace(/\.svg$/, '');
  const st = path.join(sd, name + '.struct.json');
  const r = b.analyse(f, { struct: fs.existsSync(st) ? st : null });
  for (const e of r.model.edges) {
    const arcs = (e.d.match(/A /g) || []).length / 2;
    if (arcs > e.hops) {
      // context of each arc pair: the 2 commands before and after
      const tok = e.d.split(/(?=[MLCA])/);
      const ctx = [];
      tok.forEach((t, i) => { if (t[0] === 'A' && tok[i - 1] && tok[i - 1][0] !== 'A') ctx.push(tok.slice(Math.max(0, i - 1), i + 3).join('').replace(/\s+/g, ' ').trim()); });
      console.log(`${name}\t${e.name}\tdrawn ${arcs} bridged ${e.hops}\t${ctx.join(' || ')}`);
    }
  }
}
