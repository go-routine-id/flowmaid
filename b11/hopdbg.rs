    #[test]
    fn zz_dbg_hops() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality/bench-75.mmd")).unwrap();
        let sc = scene(&parse(&src).unwrap());
        let hops = find(&sc);
        for i in [1usize, 13] {
            let e = &sc.edges[i];
            eprintln!("E{i} {}->{} corners {:?}", e.from, e.to, corners_of(e));
            eprintln!("   hops {:?}", hops[i]);
        }
    }
