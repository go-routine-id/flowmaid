import os
R=os.environ.get('DBGTREE','/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/r7/dbgtree')+'/'
def sub1(p,old,new):
    s=open(R+p).read()
    assert s.count(old)==1,(p,old[:60],s.count(old))
    open(R+p,'w').write(s.replace(old,new))
sub1('src/layout.rs',"""            let least = pair_sep(u, d, t.n, t.absize, Some(t.apath), Some(t.border), None);
            let short = need - t.bpos[u] - least;""","""            let least = pair_sep(u, d, t.n, t.absize, Some(t.apath), Some(t.border), None);
            let short = need - t.bpos[u] - least;
            if std::env::var("FM_XXTR").is_ok() { eprintln!("XXTR block {c} d {d} bpos {:.1} u {u} bpos {:.1} least {least:.1} need {need:.1} chip {:.1}..{:.1} k {k} short {short:.1}", t.bpos[d], t.bpos[u], chip.lo, chip.hi); } // XXDBG""")
sub1('src/layout.rs',"""        let k = counts[c].0;
        let need = if k == 0 {""","""        let k = counts[c].0;
        if std::env::var("FM_XXTR").is_ok() { eprintln!("XXTR chip {c} row {} {:.1}..{:.1} k {k}", chip.row, chip.lo, chip.hi); } // XXDBG
        let need = if k == 0 {""")
print("r8 dbg on")
sub1('src/layout.rs',"""        if !under.is_empty() && order.clear_titles(&under, &bpos, &absize, &mut layers, &mut pos) {""","""        if std::env::var("FM_XXDTR").is_ok() { for &(d, hi) in &under { let e = &g.edges[dummy_edge[d]]; eprintln!("XXDTR under {}->{} at {:.1} chip_hi {hi:.1}", g.nodes[e.from].id, g.nodes[e.to].id, bpos[d]); } } // XXDBG
        if !under.is_empty() && std::env::var("FM_NOCLEAR").is_err() && order.clear_titles(&under, &bpos, &absize, &mut layers, &mut pos) {""")
sub1('src/layout.rs',"""        if !widen.is_empty() {
            bpos = breadth(&layers, &widen);""","""        if std::env::var("FM_XXDTR").is_ok() { for &(d, w) in &widen { let e = &g.edges[dummy_edge[d]]; eprintln!("XXDTR widen {}->{} by {w:.1}", g.nodes[e.from].id, g.nodes[e.to].id); } } // XXDBG
        if !widen.is_empty() && std::env::var("FM_NOROOM").is_err() {
            bpos = breadth(&layers, &widen);""")
sub1('src/layout.rs',"""                    if c <= cross && f <= fans {
                        (cross, fans, changed) = (c, f, true);""","""                    if c <= cross && f <= fans {
                        if std::env::var("FM_XXDTR").is_ok() { eprintln!("XXDTR kept chain {ch} d {d} x {x:.1} whole {} cross {cross}->{c} fans {fans}->{f}", moved.len() == chain.len() && moved.len() != run.len()); } // XXDBG
                        (cross, fans, changed) = (c, f, true);""")
print("r8 dbg2 on")
sub1('src/layout.rs',"""            seed_layers = Some(before);
        }""","""            seed_layers = Some(before);
            if std::env::var("FM_XXLAY").is_ok() { // XXDBG
                for (l, lv) in layers.iter().enumerate() { // XXDBG
                    let mut s = String::new(); // XXDBG
                    for &v in lv { if v < n { s.push_str(&format!(" {}", g.nodes[v].id)); } else if dummy_edge[v] != usize::MAX { let e = &g.edges[dummy_edge[v]]; s.push_str(&format!(" ({}>{})", g.nodes[e.from].id, g.nodes[e.to].id)); } else { s.push_str(if border_side[v] == 1 { " [" } else { " ]" }); } } // XXDBG
                    eprintln!("XXLAY L{l}:{s}"); // XXDBG
                } // XXDBG
            } // XXDBG
        }""")
