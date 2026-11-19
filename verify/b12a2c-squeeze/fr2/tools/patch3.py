import sys
p=sys.argv[1]; s=open(p).read()
old='''    let mut used = vec![false; paths.len()];
    let mut miss = 0;'''
new='''    // routed scenes are painted shifted (view_shift): undo the most common start offset
    let mut votes: std::collections::BTreeMap<(i64, i64), usize> = std::collections::BTreeMap::new();
    {
        let mut used0 = vec![false; paths.len()];
        for e in sc.edges.iter() {
            let key = format!("{} \\u{2192} {}", e.from, e.to);
            if let Some(pi) = (0..paths.len()).find(|&j| !used0[j] && (paths[j].0 == key || paths[j].0.starts_with(&format!("{key}:")))) {
                used0[pi] = true;
                if let Some(&q) = paths[pi].1.first() {
                    *votes.entry((((q.0 - e.bezier[0].0) * 10.0).round() as i64, ((q.1 - e.bezier[0].1) * 10.0).round() as i64)).or_default() += 1;
                }
            }
        }
    }
    let shift = votes.iter().max_by_key(|(_, &c)| c).map(|(&(x, y), _)| (x as f64 / 10.0, y as f64 / 10.0)).unwrap_or((0.0, 0.0));
    let paths: Vec<(String, Vec<(f64, f64)>)> = paths.into_iter().map(|(t, q)| (t, q.into_iter().map(|p| (p.0 - shift.0, p.1 - shift.1)).collect())).collect();
    let mut used = vec![false; paths.len()];
    let mut miss = 0;'''
assert old in s
s=s.replace(old,new,1)
open(p,'w').write(s)
