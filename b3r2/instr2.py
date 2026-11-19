# Debug instrumentation applied to the scratch copy only (cwd = copy root).
import re
p='src/router.rs'; s=open(p).read()
dbg='let dbg = |i: usize| std::env::var("FM_DBG").map_or(false, |v| v == "*" || v == format!("{}->{}", sc.edges[i].from, sc.edges[i].to));\n'
def rep(old,new):
    global s
    assert s.count(old)==1, old[:60]
    s=s.replace(old,new)
rep("    let mut bad: Vec<(usize, usize)> = Vec::new(); // (class, edge): 0 hard, 1 soft\n","    let mut bad: Vec<(usize, usize)> = Vec::new(); // (class, edge): 0 hard, 1 soft\n    "+dbg)
rep("        alone[i] = w.score(info, i, &polys[i], None);\n","        alone[i] = w.score(info, i, &polys[i], None);\n        if dbg(i) { eprintln!(\"DBG {}->{} mode_static={} alone={:?}\", sc.edges[i].from, sc.edges[i].to, mode==Mode::Static, alone[i]); }\n")
rep("            if s.near > 0 || soft {\n                bad.push((1, i));\n            }",
 "            if dbg(i) { eprintln!(\"DBG {}->{} scored {:?} soft={}\", sc.edges[i].from, sc.edges[i].to, s, soft); }\n            if s.near > 0 || soft {\n                bad.push((1, i));\n            }")
rep("        if work.left == 0 && (class == 1 || alone[i].hard() == 0 || detour_work == 0) {\n",
 "        if dbg(i) { eprintln!(\"DBG {}->{} bad class={} work.left={}\", sc.edges[i].from, sc.edges[i].to, class, work.left); }\n        if work.left == 0 && (class == 1 || alone[i].hard() == 0 || detour_work == 0) {\n")
rep("            let s = w.score(info, i, &pts, Some(&mut accepted));\n            let _ = work.take(w.spent());\n            let clean",
 "            let s = w.score(info, i, &pts, Some(&mut accepted));\n            if dbg(i) { eprintln!(\"DBG {}->{} cand lat={} sw={} soft={} s={:?} corners={:?}\", sc.edges[i].from, sc.edges[i].to, t.lat, t.soft_walls, t.soft, s, corners); }\n            let _ = work.take(w.spent());\n            let clean")
rep("        match best {\n            Some((s, corners, pts)) if better(&s, &pts) => {",
 "        if dbg(i) { eprintln!(\"DBG {}->{} final base={:?} best={:?} better={:?} decided={}\", sc.edges[i].from, sc.edges[i].to, base, best.as_ref().map(|b| b.0), best.as_ref().map(|b| better(&b.0, &b.2)), decided); }\n        match best {\n            Some((s, corners, pts)) if better(&s, &pts) => {")
open(p,'w').write(s)
p='src/quality.rs'; s=open(p).read()
rep("                m.min_node_gap = m.min_node_gap.min(gap);\n","                m.min_node_gap = m.min_node_gap.min(gap);\n                if gap < 6.0 { m.details.push(format!(\"{}->{} gapnode {} {gap:.2}\", me.e.from, me.e.to, n.id)); }\n")
open(p,'w').write(s)
p='src/quality.rs'; s=open(p).read()
rep("            if j != i && me.pts.windows(2).any(|s| r.hits_seg(s[0], s[1])) {\n                m.lbl_edge += 1;\n",
    "            if j != i && me.pts.windows(2).any(|s| r.hits_seg(s[0], s[1])) {\n                m.lbl_edge += 1;\n                m.details.push(format!(\"{owner} lbl_edge {}->{}\", me.e.from, me.e.to));\n")
open(p,'w').write(s)
p='src/edgepass.rs'; s=open(p).read()
dbgf='''    let show = |sc: &Scene, stage: &str| {
        if let Ok(v) = std::env::var("FM_DBG") {
            for e in sc.edges.iter().filter(|e| format!("{}->{}", e.from, e.to) == v) {
                let w: Vec<(f64, f64)> = e.waypoints.iter().map(|p| ((p.0 * 10.0).round() / 10.0, (p.1 * 10.0).round() / 10.0)).collect();
                eprintln!("STAGE {:?} {stage}: n={} last={:?} bez={:?}", mode, w.len(), &w[w.len().saturating_sub(8)..], e.bezier);
            }
        }
    };
'''
rep("    let mut moved = vec![false; sc.edges.len()];\n    crate::ports::assign(g, sc, &cx, mode, &mut moved);\n    crate::router::repair(g, sc, &cx, mode, &mut moved);\n",
    "    let mut moved = vec![false; sc.edges.len()];\n"+dbgf+"    show(sc, \"built\");\n    crate::ports::assign(g, sc, &cx, mode, &mut moved);\n    show(sc, \"assign\");\n    crate::router::repair(g, sc, &cx, mode, &mut moved);\n    show(sc, \"repair\");\n")
rep("    crate::ports::respace(g, sc, &cx, mode, &mut moved);\n","    crate::ports::respace(g, sc, &cx, mode, &mut moved);\n    show(sc, \"respace\");\n")
rep("    crate::router::nudge(g, sc, &cx, mode, &mut moved);\n    glyph_stubs(sc, &cx, &mut moved);\n","    crate::router::nudge(g, sc, &cx, mode, &mut moved);\n    show(sc, \"nudge\");\n    glyph_stubs(sc, &cx, &mut moved);\n    show(sc, \"stubs\");\n")
open(p,'w').write(s)
