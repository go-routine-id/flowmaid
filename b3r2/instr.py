import sys,re
p=sys.argv[1]; s=open(p).read()
dbg='let dbg = |i: usize| std::env::var("FM_DBG").map_or(false, |v| v == format!("{}->{}", sc.edges[i].from, sc.edges[i].to));\n'
# after `let mut bad: Vec<(usize, usize)> = Vec::new();`
s=s.replace("    let mut bad: Vec<(usize, usize)> = Vec::new(); // (class, edge): 0 hard, 1 soft\n","    let mut bad: Vec<(usize, usize)> = Vec::new(); // (class, edge): 0 hard, 1 soft\n    "+dbg,1)
s=s.replace("            if s.near > 0 || soft {\n                bad.push((1, i));\n            }",
 "            if dbg(i) { eprintln!(\"DBG scored {:?} soft={} alone={:?} mode={:?}\", s, soft, alone[i], mode==Mode::Static); }\n            if s.near > 0 || soft {\n                bad.push((1, i));\n            }",1)
s=s.replace("        if work.left == 0 && (class == 1 || alone[i].hard() == 0 || detour_work == 0) {\n            continue; // keeps its geometry (already accepted)\n        }",
 "        if dbg(i) { eprintln!(\"DBG bad class={} work.left={}\", class, work.left); }\n        if work.left == 0 && (class == 1 || alone[i].hard() == 0 || detour_work == 0) {\n            continue; // keeps its geometry (already accepted)\n        }",1)
s=s.replace("            n_found += 1;\n            let pts = drawn_with(e, &smooth(&corners, CORNER_R));\n            // Charged by what the scoring looked at, even when that\n            // overdraws: the search is paid for, so the candidate is\n            // still scored, and the next check stops the repair. One\n            // candidate's scoring is the most a repair goes over its\n            // budget.\n            let s = w.score(info, i, &pts, Some(&mut accepted));",
 "            n_found += 1;\n            let pts = drawn_with(e, &smooth(&corners, CORNER_R));\n            let s = w.score(info, i, &pts, Some(&mut accepted));\n            if dbg(i) { eprintln!(\"DBG cand try lat={} soft_walls={} soft={} s={:?} corners={:?}\", t.lat, t.soft_walls, t.soft, s, corners); }",1)
s=s.replace("        match best {\n            Some((s, corners, pts)) if better(&s, &pts) => {",
 "        if dbg(i) { eprintln!(\"DBG final base={:?} best={:?} better={:?} decided={}\", base, best.as_ref().map(|b| b.0), best.as_ref().map(|b| better(&b.0, &b.2)), decided); }\n        match best {\n            Some((s, corners, pts)) if better(&s, &pts) => {",1)
open(p,'w').write(s)
print(s.count('dbg(i)'))
