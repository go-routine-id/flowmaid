    // Group the legs by (commit, side). Legs that share one run side
    // by side, ordered so none crosses another where it turns: those
    // whose run comes from earlier commits (turning back) first, the
    // longer one outside; then those running on to later commits, the
    // longer one outside again.
    let owner_of = |l: &Link| match l.leg {
        Leg::None => None,
        Leg::AtFrom => Some((l.from, l.to)),
        Leg::AtTo => Some((l.to, l.from)),
    };
    let mut groups: Vec<(usize, bool, usize)> = links
        .iter()
        .enumerate()
        .filter_map(|(k, l)| owner_of(l).map(|(o, t)| (o, lane[t] > lane[o], k)))
        .collect();
    groups.sort_by_key(|&(o, high, k)| {
        let l = &links[k];
        let reach = lane[l.from].abs_diff(lane[l.to]) as isize;
        let order = match l.leg {
            Leg::AtTo => (0, reach),
            _ => (1, -reach),
        };
        (o, high, order, k)
    });
    let mut start = 0;
    while start < groups.len() {
        let (o, high, _) = groups[start];
        let k = groups[start..]
            .iter()
            .take_while(|&&(o2, h2, _)| (o2, h2) == (o, high))
            .count();
        if k > 1 {
            // Stay inside the dot, however many share it.
            let pitch = LEG_PITCH.min((2.0 * R - EDGE_W) / (k - 1) as f64);
            for (j, &(_, _, li)) in groups[start..start + k].iter().enumerate() {
                links[li].off = (j as f64 - (k - 1) as f64 / 2.0) * pitch;
            }
        }
        start += k;
    }

    let mut leg_high = vec![false; n];
    let mut leg_low = vec![false; n];
    let mut offs_high = vec![(0.0f64, 0.0f64); n];
    let mut offs_low = vec![(0.0f64, 0.0f64); n];
    let mut legs_at = vec![Vec::new(); n];
    for l in &links {
        let Some((owner, other)) = owner_of(l) else {
            continue;
        };
        let (a, b) = (lane[owner], lane[other]);
        let (flag, offs) = if b > a {
            (&mut leg_high[owner], &mut offs_high[owner])
        } else {
            (&mut leg_low[owner], &mut offs_low[owner])
        };
        *flag = true;
        *offs = (offs.0.min(l.off), offs.1.max(l.off));
        legs_at[col[owner]].push((a.min(b), a.max(b), l.off));
    }
