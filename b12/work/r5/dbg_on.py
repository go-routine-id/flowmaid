import re,sys
import os
R=os.environ.get('DBGTREE','/nonexistent')+'/'
def sub1(p,old,new):
    s=open(R+p).read()
    assert s.count(old)==1,(p,old[:60],s.count(old))
    open(R+p,'w').write(s.replace(old,new))
sub1('src/layout.rs',"""        order.run(
            starts,
            seed.map(|s| (s, &element as ElementOf<'_>)),
            &mut layers,
            &mut pos,
        );
    }
""","""        order.run(
            starts,
            seed.map(|s| (s, &element as ElementOf<'_>)),
            &mut layers,
            &mut pos,
        );
        if std::env::var("FM_XXDBG").is_ok() { // XXDBG
            eprintln!("XXDBG order starts {} seed {} na {} links {} layered {}", starts, seed.is_some(), na, links, count_crossings(&layers, &succs, &alayer, nlayers)); // XXDBG
            for (l, lv) in layers.iter().enumerate() { // XXDBG
                let mut s = String::new(); // XXDBG
                let mut prevp: Vec<usize> = Vec::new(); // XXDBG
                for &v in lv { // XXDBG
                    let p = &apath[v]; // XXDBG
                    if *p != prevp { s.push_str(&format!(" |{}|", p.iter().map(|&c| g.subgraphs[c].id.clone()).collect::<Vec<_>>().join("/"))); prevp = p.clone(); } // XXDBG
                    if v < n { s.push_str(&format!(" {}", g.nodes[v].id)); } // XXDBG
                    else if dummy_edge[v] != usize::MAX { let e = &g.edges[dummy_edge[v]]; s.push_str(&format!(" ({}>{})", g.nodes[e.from].id, g.nodes[e.to].id)); } // XXDBG
                    else { s.push_str(if border_side[v] == 1 { " [" } else { " ]" }); } // XXDBG
                } // XXDBG
                eprintln!("XXDBG L{l}:{s}"); // XXDBG
            } // XXDBG
        } // XXDBG
    }
""")
sub1('src/layout.rs',"""            let c = self.settle(layers, pos);
            if best.as_ref().map_or(true, |b| c < b.0) {
                best = Some((c, layers.clone()));
            }
""","""            let c = self.settle(layers, pos);
            if std::env::var("FM_XXDBG2").is_ok() { eprintln!("XXDBG start {k} layered {c}"); } // XXDBG
            if best.as_ref().map_or(true, |b| c < b.0) {
                best = Some((c, layers.clone()));
            }
""")
sub1('src/quality.rs',"            m.x_drawn += hits.len();\n","""            m.x_drawn += hits.len();
            if std::env::var("FM_XXDBG6").is_ok() { for &p in &hits { m.details.push(format!("XX {}->{} x {}->{} at ({:.0},{:.0})", a.e.from, a.e.to, b.e.from, b.e.to, p.0, p.1)); } } // XXDBG
""")
sub1('src/router.rs',"""    if bad.is_empty() {
        return;
    }
    // A broken edge stays where it is until its turn comes (and for good
""","""    if std::env::var("FM_XXDBG7").is_ok() { for &(class, i) in &bad { let e = &sc.edges[i]; eprintln!("XXDBG bad class {class} {}->{} alone {:?} crossed {}", e.from, e.to, alone[i], crossed(i)); } } // XXDBG
    if bad.is_empty() {
        return;
    }
    // A broken edge stays where it is until its turn comes (and for good
""")
p='src/edgepass.rs'
s=open(R+p).read()
i=s.index("fn stages(g: &Graph, sc: &mut Scene, mut cx: Ctx, mode: Mode) {")
body=s[i:]
j=body.index("    refit(sc, mode, g.edge_paint_width.is_none());\n}")
blk=body[:j]
dbg='    let xxdbg = |tag: &str, sc: &Scene| { if std::env::var("FM_XXDBG4").is_ok() { let m = crate::quality::measure(g, sc); eprintln!("XXDBG stage {tag}: x_drawn {} fan_x {} edge_node {} edge_frame {} own_frame {} hard {}", m.x_drawn, m.fan_x, m.edge_node_x, m.edge_frame_x, m.own_frame_x, m.edge_node_x + m.shared_ports + m.arrow_on_start + m.wiggles + m.frame_side + m.node_overlap + m.node_frame + m.frame_overlap); if let Ok(d) = std::env::var("FM_XXDBG6") { let mut xs: Vec<&String> = m.details.iter().filter(|l| l.starts_with("XX ")).collect(); xs.sort(); std::fs::write(format!("{d}/{tag}.txt"), xs.iter().map(|l| format!("{l}\\n")).collect::<String>()).ok(); } } }; // XXDBG\n'
stop=lambda tag: f'    xxdbg("{tag}", sc); // XXDBG\n    if std::env::var("FM_XXSTOP").ok().as_deref() == Some("{tag}") {{ refit(sc, mode, g.edge_paint_width.is_none()); return; }} // XXDBG\n'
blk=blk.replace("    let mut moved = vec![false; sc.edges.len()];\n","    let mut moved = vec![false; sc.edges.len()];\n"+dbg+stop("builder"),1)
for call,tag in [("    crate::ports::assign(g, sc, &cx, mode, &mut moved);\n","ports"),("    crate::ortho::orthogonalize(g, sc, &mut cx, mode);\n","ortho"),("    crate::router::repair(g, sc, &cx, mode, &mut moved);\n","repair"),("    crate::ports::respace(g, sc, &cx, mode, &mut moved);\n","respace"),("    crate::router::nudge(g, sc, &cx, mode, &mut moved);\n","nudge"),("    crate::router::spread_close(g, sc, &cx, mode, &mut moved);\n","spread"),("    crate::comb::comb(sc, &cx, &mut moved);\n","comb"),("    crate::labels::relabel(g, sc, &cx, &moved, budget);\n","relabel")]:
    assert call in blk, call
    blk=blk.replace(call,call+stop(tag),1)
open(R+p,'w').write(s[:i]+blk+body[j:])
print('debug on')





sub1('src/layout.rs',"""                    cursor = cursor.max(at - before);
                }
            }
            prev_after = after;
""","""                    let at = if std::env::var("FM_XXNOG1").is_ok() { f64::NEG_INFINITY } else { at }; // XXDBG
                    cursor = cursor.max(at - before);
                }
            }
            prev_after = after;
""")


