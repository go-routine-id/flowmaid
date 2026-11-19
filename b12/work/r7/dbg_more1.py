import os
R=os.environ['DBGTREE']+'/'
def sub1(p,old,new):
    s=open(R+p).read()
    assert s.count(old)==1,(p,old[:60],s.count(old))
    open(R+p,'w').write(s.replace(old,new))
sub1('src/ortho.rs',"""    place_tracks(&mut jogs, &kept, cx.blocks);
    uncross(&mut jogs, &kept_lines);
""","""    if std::env::var("FM_XXJOG").is_ok() { eprintln!("XXJOG gaps {:?}", gaps.iter().map(|g| format!("{:.0}-{:.0}", g.0, g.1)).collect::<Vec<_>>()); } // XXDBG
    place_tracks(&mut jogs, &kept, cx.blocks);
    if std::env::var("FM_XXJOG").is_ok() { for j in &jogs { let e = &sc.edges[j.edge]; eprintln!("XXJOG placed {}->{} gap {} at {:.0}..{:.0} room {:.1}..{:.1} want {:.1} at {:.1} label {}", e.from, e.to, j.gap, j.lo_at, j.hi_at, j.room.0, j.room.1, j.want, j.at, j.above_label); } } // XXDBG
    uncross(&mut jogs, &kept_lines);
    if std::env::var("FM_XXJOG").is_ok() { for j in &jogs { let e = &sc.edges[j.edge]; eprintln!("XXJOG uncrossed {}->{} gap {} at {:.1}", e.from, e.to, j.gap, j.at); } } // XXDBG
""")
print("jog dbg on")
