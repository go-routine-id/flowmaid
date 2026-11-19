import os
R=os.environ['DBGTREE']+'/'
def sub1(p,old,new):
    s=open(R+p).read()
    assert s.count(old)==1,(p,old[:60],s.count(old))
    open(R+p,'w').write(s.replace(old,new))
sub1('src/ortho.rs',"""                out.cols.entry((i, k)).or_insert(col);""","""                out.cols.entry((i, k)).or_insert(col);
                if std::env::var("FM_XXTITLE").is_ok() { eprintln!("XXTITLE {}->{} frame {} col {:.0} side {:?}", sc.edges[i].from, sc.edges[i].to, sc.clusters[k].id, col, out.sides[k]); } // XXDBG""")
print("title dbg on")
