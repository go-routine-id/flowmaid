const {execFileSync}=require('child_process');
const [bm,sd,...files]=process.argv.slice(2);
for(const f of files){
  const out=execFileSync('node',[bm,'--struct-dir',sd,'--json',f],{encoding:'utf8',stdio:['ignore','pipe','ignore'],maxBuffer:1<<28});
  const j=JSON.parse(out)[0]; const E=j.detail.edges;
  const b=E.map(e=>e.bends).sort((a,b)=>b-a);
  const over6=b.filter(x=>x>6).length;
  console.log([j.metrics.name,j.metrics.engine,'bends',j.metrics.bends,'max/edge',b[0],'edges>6',over6,'top5',b.slice(0,5).join(' '),'detach_max',(+j.metrics.lbl_detach_max).toFixed(1),'endpx_max',Math.round(j.metrics.lbl_endpx_max),'aspect',(+j.metrics.aspect).toFixed(2)].join('\t'));
}
