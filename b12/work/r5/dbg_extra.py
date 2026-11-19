import os
R=os.environ['DBGTREE']+'/'
def sub1(p,old,new):
    s=open(R+p).read()
    assert s.count(old)==1,(p,old[:60],s.count(old))
    open(R+p,'w').write(s.replace(old,new))
sub1('src/router.rs',"""            if let Some((new, p)) = nudged_i {
                let s = w.score(info, i, &p, Some(&mut *acc));""","""            if let Some((new, p)) = nudged_i {
                let s = w.score(info, i, &p, Some(&mut *acc));
                if std::env::var("FM_XXN").ok().as_deref() == Some(&format!("{}->{}", e.from, e.to)) { eprintln!("XXN cur {:?}\\n    new {:?}\\n s_cur {:?}\\n s_new {:?}", cur, new, s_cur, s); } // XXDBG""")
