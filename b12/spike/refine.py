import sys
p='tree/src/scene.rs'
s=open(p).read()
i=s.index("    // Registered legs, so later paths take a free track.")
j=s.index("    let mut real: Vec<Option<Vec<(f64, f64)>>> = vec![None; g.edges.len()];")
new=open('build_new.rs').read()
s=s[:i]+new+s[j:]
def rep(a,b,cnt=1):
    global s
    assert s.count(a)==cnt,(a[:70],s.count(a)); s=s.replace(a,b)
rep("""            &chips.iter().map(|&c| flip(c)).collect::<Vec<_>>(),
            &mut vlegs,
            &mut hlegs,
        );""",
"""            &chips.iter().map(|&c| flip(c)).collect::<Vec<_>>(),
            m,
            (fa.0, fa.1),
            (fb.0, fb.1),
            &mut vlegs,
            &mut hlegs,
        );""")
rep("""            &[],
            &mut vlegs,
            &mut hlegs,
        );""",
"""            &chips.iter().map(|&c| flip(c)).collect::<Vec<_>>(),
            m,
            (nb.0, nb.1),
            (fb.0, fb.1),
            &mut vlegs,
            &mut hlegs,
        );""")
open(p,'w').write(s)
