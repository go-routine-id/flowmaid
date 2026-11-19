import re,shutil
B='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r3/fmprof/src/'
R='/home/user/flowmaid/src/'
import os
for f in os.listdir(R):
    if f.endswith('.rs'): shutil.copy(R+f, B+f)
s=open(B+'edgepass.rs').read()
s=s.replace('''pub(crate) fn finish_held(g: &Graph, sc: &mut Scene, mode: Mode, fixed: &[bool], held: &[bool]) {
    let mut cx = ctx(g, sc);''','''pub static STAGE_NS: [std::sync::atomic::AtomicU64; 16] = [const { std::sync::atomic::AtomicU64::new(0) }; 16];
pub static NUDGE_CNT: [std::sync::atomic::AtomicU64; 4] = [const { std::sync::atomic::AtomicU64::new(0) }; 4];
pub(crate) fn trace_edges(sc: &Scene, stage: &str) {
    let Ok(want) = std::env::var("FM_TRACE") else { return };
    for e in &sc.edges {
        if want.split(',').any(|w| w == format!("{}->{}", e.from, e.to)) {
            let w: Vec<String> = e.waypoints.iter().map(|p| format!("({:.2},{:.2})", p.0, p.1)).collect();
            eprintln!("[{stage}] {}->{} wps={} bez={:?}", e.from, e.to, w.join(" "), e.bezier);
        }
    }
}
fn tick(t: &mut std::time::Instant, k: usize) {
    let n = t.elapsed().as_nanos() as u64;
    STAGE_NS[k].fetch_add(n, std::sync::atomic::Ordering::Relaxed);
    *t = std::time::Instant::now();
}
pub(crate) fn finish_held(g: &Graph, sc: &mut Scene, mode: Mode, fixed: &[bool], held: &[bool]) {
    let mut tt = std::time::Instant::now();
    let mut cx = ctx(g, sc);''')
names=['ctx','ports','ortho','repair','respace','nudge','spread_close','stubs+snap','labels','refit']
pairs=[('    crate::ports::assign(g, sc, &cx, mode, &mut moved);',0,True),
 ('    crate::ports::assign(g, sc, &cx, mode, &mut moved);',1,False),
 ('    crate::ortho::orthogonalize(g, sc, &mut cx, mode);',2,False),
 ('    crate::router::repair(g, sc, &cx, mode, &mut moved);',3,False),
 ('    crate::ports::respace(g, sc, &cx, mode, &mut moved);',4,False),
 ('    crate::router::nudge(g, sc, &cx, mode, &mut moved);',5,False),
 ('    crate::router::spread_close(g, sc, &cx, mode, &mut moved);',6,False),
 ('    snap_ends(sc, &cx);',7,False),
 ('    crate::labels::relabel(g, sc, &cx, &moved, budget);',8,False)]
for line,k,before in pairs:
    t=f'    tick(&mut tt, {k});\n    trace_edges(sc, "{names[k]}");'
    assert line in s, line
    s=s.replace(line, (t+'\n'+line) if before else (line+'\n'+t),1)
s=s.replace('''    refit(sc, mode, g.edge_paint_width.is_none());
}''','''    refit(sc, mode, g.edge_paint_width.is_none());
    tick(&mut tt, 9);
}''',1)
open(B+'edgepass.rs','w').write(s)
l=open(B+'lib.rs').read()+'\n#[doc(hidden)]\npub use edgepass::STAGE_NS;\n#[doc(hidden)]\npub use edgepass::NUDGE_CNT;\n'
open(B+'lib.rs','w').write(l)
