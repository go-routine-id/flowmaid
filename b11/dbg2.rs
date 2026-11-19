    #[test]
    fn zz_dbg_marquee() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/quality/dense-td.mmd")).unwrap();
        let s = scene(&parse(&src).unwrap());
        let hops = crate::hops::find(&s);
        let e = &s.edges[4];
        eprintln!("hops {:?}", hops[4]);
        let (start, segs) = drawn_segs(e, |p| p);
        eprintln!("d = {}", hopped_d(start, &segs, &hops[4]));
        let (ps, pg) = plain_segs(e, |p| p);
        eprintln!("plain = {}", hopped_d(ps, &pg, &hops[4]));
        let r = [3674.539722619832, 1050.6149201968499, 3674.6397226198324, 1050.7149201968498];
        let m = PickRect { x0: r[0], y0: r[1], x1: r[2], y1: r[3] };
        let mut hp = HopPick::new(&s);
        eprintln!("dist {} plain {}", hp.distance(4, e, m, 1.0), edge_distance(e, m, 10.0));
    }
