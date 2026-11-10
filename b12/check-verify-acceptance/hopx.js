const fs=require('fs'),path=require('path'),vm=require('vm');
const bm=process.argv[2];
let src=fs.readFileSync(bm,'utf8').replace(/^#!.*\n/,'').replace(/\nmain\(\);\s*$/,'\nmodule.exports={flattenPath,toSegs,dehop,straightRuns,segsLength,crossingsBetween};\n');
const mod={exports:{}};
vm.runInThisContext('(function(require,module,exports,__filename,__dirname){'+src+'\n})')(require,mod,mod.exports,bm,path.dirname(bm));
const {flattenPath,toSegs,dehop,crossingsBetween}=mod.exports;
const mk=d=>{const segs=dehop(toSegs(flattenPath(d,null))).segs; let x0=1e9,y0=1e9,x1=-1e9,y1=-1e9; for(const g of segs){x0=Math.min(x0,g.ax,g.bx);y0=Math.min(y0,g.ay,g.by);x1=Math.max(x1,g.ax,g.bx);y1=Math.max(y1,g.ay,g.by);} return {segs,bbox:{x:x0,y:y0,w:x1-x0,h:y1-y0}};};
const bridge=mk('M 0 50 L 35 50 A 5 5 0 0 1 40 45 L 65 45 A 5 5 0 0 1 70 50 L 200 50');
let n=0;
for(const x of [40,52.5,65]){ const v=mk(`M ${x} 0 L ${x} 100`); try{ const r=crossingsBetween(bridge,v); n+= (Array.isArray(r)?r.length:(typeof r==='number'?r:(r&&r.n)||0)); console.log('x',x,JSON.stringify(r).slice(0,120)); }catch(e){console.log('err',e.message)} }
console.log('crossings counted over a 3-track bridge:',n);
