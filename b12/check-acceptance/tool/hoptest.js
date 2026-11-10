const fs=require('fs'),path=require('path'),vm=require('vm');
const bm=process.argv[2];
let src=fs.readFileSync(bm,'utf8').replace(/^#!.*\n/,'').replace(/\nmain\(\);\s*$/,'\nmodule.exports={flattenPath,toSegs,dehop,straightRuns,segsLength,crossingsBetween};\n');
const mod={exports:{}};
vm.runInThisContext('(function(require,module,exports,__filename,__dirname){'+src+'\n})')(require,mod,mod.exports,bm,path.dirname(bm));
const {flattenPath,toSegs,dehop,straightRuns,segsLength,crossingsBetween}=mod.exports;
const t=(label,d)=>{const r=dehop(toSegs(flattenPath(d,null)));const runs=straightRuns(r.segs).filter(x=>x.len>=8);console.log(label.padEnd(58),'hops',r.hops,'len',segsLength(r.segs).toFixed(1),'straight runs>=8px',runs.length,'longest',Math.max(...runs.map(x=>x.len)).toFixed(1));};
// B11 shapes (src/hops.rs): quarter circle up r, level over the bundle, quarter circle down
t('single hop r=5 over one line (span 10)','M 0 50 L 95 50 A 5 5 0 0 1 100 45 A 5 5 0 0 1 105 50 L 200 50');
t('bridge over 2 tracks 12.5 apart (span 22.5)','M 0 50 L 35 50 A 5 5 0 0 1 40 45 L 52.5 45 A 5 5 0 0 1 57.5 50 L 200 50');
t('bridge over 3 tracks 12.5 apart (span 35, B11 unit test)','M 0 50 L 35 50 A 5 5 0 0 1 40 45 L 65 45 A 5 5 0 0 1 70 50 L 200 50');
t('bridge over 6 lanes at 12 px (span 70)','M 0 50 L 35 50 A 5 5 0 0 1 40 45 L 100 45 A 5 5 0 0 1 105 50 L 200 50');
t('wide-stroke hop r=9 over 2 tracks (span 30.5)','M 0 50 L 31 50 A 9 9 0 0 1 40 41 L 52.5 41 A 9 9 0 0 1 61.5 50 L 200 50');
