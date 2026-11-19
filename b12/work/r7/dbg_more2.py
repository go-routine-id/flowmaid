import os
R=os.environ['DBGTREE']+'/'
def sub1(p,old,new):
    s=open(R+p).read()
    assert s.count(old)==1,(p,old[:60],s.count(old))
    open(R+p,'w').write(s.replace(old,new))
sub1('src/router.rs',"""                s.hug += side_run(r, simple, &own).round() as usize;""","""                let xxh = side_run(r, simple, &own).round() as usize; // XXDBG
                if xxh > 0 && std::env::var("FM_XXHUG").ok().as_deref() == Some(&me.to_string()) { eprintln!("XXHUG edge {me} r {:?} run {xxh} simple {:?}", r, simple.iter().map(|p| format!("({:.1},{:.1})", p.0, p.1)).collect::<Vec<_>>()); } // XXDBG
                s.hug += xxh; // XXDBG""")
print("hug dbg on")
