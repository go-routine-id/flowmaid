//! Independent probe of #111 (route_partial for ER, class, mindmap).
use flowmaid::model::{ClassDiagram, Document, ErDiagram, Mindmap};
use flowmaid::parser::parse_document;
use flowmaid::{class, er, mindmap};
use std::collections::{HashMap, HashSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

static mut FAILS: usize = 0;
fn fail(msg: String) {
    unsafe {
        FAILS += 1;
        if FAILS < 200 {
            println!("FAIL: {msg}");
        }
    }
}
fn dbg<T: std::fmt::Debug>(t: &T) -> String {
    format!("{t:?}")
}

fn mmd_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut v: Vec<PathBuf> = rd.map(|e| e.unwrap().path()).collect();
    v.sort();
    for p in v {
        if p.is_dir() {
            mmd_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "mmd") {
            out.push(p);
        }
    }
}

fn docs() -> Vec<(String, Document)> {
    let root = Path::new("/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0402/verify-2-api/guard");
    let mut files = Vec::new();
    mmd_files(&root.join("examples"), &mut files);
    mmd_files(&root.join("tests/fixtures"), &mut files);
    let mut out: Vec<(String, Document)> = files
        .into_iter()
        .filter_map(|p| {
            let src = std::fs::read_to_string(&p).ok()?;
            parse_document(&src)
                .ok()
                .map(|d| (p.display().to_string(), d))
        })
        .collect();
    if let Ok(dir) = std::env::var("PROBE_CORPUS") {
        let step: usize = std::env::var("PROBE_STEP").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
        let mut more = Vec::new();
        mmd_files(Path::new(&dir), &mut more);
        for (k, p) in more.into_iter().enumerate() {
            if k % step != 0 { continue; }
            let Ok(src) = std::fs::read_to_string(&p) else { continue };
            if let Ok(d) = catch_unwind(AssertUnwindSafe(|| parse_document(&src))) {
                if let Ok(d) = d { out.push((p.display().to_string(), d)); }
            } else {
                println!("note: parser panics on {}", p.display());
            }
        }
    }
    for (n, s) in extra_sources() {
        match parse_document(&s) {
            Ok(d) => out.push((n.to_string(), d)),
            Err(e) => println!("note: extra {n} does not parse: {e:?}"),
        }
    }
    out
}

fn extra_sources() -> Vec<(&'static str, String)> {
    vec![
        ("er_mix", ER_MIX.to_string()),
        ("class_notes", CLASS_NOTES.to_string()),
        ("class_cards", CLASS_CARDS.to_string()),
        ("mind_styled", MIND.to_string()),
    ]
}

const ER_MIX: &str = "erDiagram
  CUSTOMER ||--o{ ORDER : places
  ORDER ||--|{ LINE : contains
  LINE }o--|| PRODUCT : of
  PRODUCT }|..|{ CATEGORY : in
  CATEGORY |o--o| CUSTOMER : likes
  CUSTOMER ||--o| CUSTOMER : refers
  ORDER }o--o{ PRODUCT : lists
  CUSTOMER {
    string name PK
    string email UK \"contact\"
  }
";

const CLASS_NOTES: &str = "classDiagram
  note \"free note\"
  class Animal {
    +String name
    +eat() void
  }
  class Duck
  class Fish
  Animal <|-- Duck
  Animal <|-- Fish
  Duck \"1\" --> \"*\" Egg : lays
  Fish ..> Water
  note for Duck \"can fly\"
  note for Fish \"swims\"
  Animal --> Animal : self
";

const CLASS_CARDS: &str = "classDiagram
  Hub \"1\" --> \"*\" A : a
  Hub \"1\" --> \"0..1\" B : b
  Hub \"many\" o-- \"1\" C
  C \"1\" *-- \"*\" D : has
  D \"1\" --> \"1\" E
  A <|-- B
  E ..|> F
";

const MIND: &str = "mindmap
  root((Root))
    A[Square]
      A1
      A2)cloud(
    B(Round)
      B1
        B11
    C{{Hex}}
      ::icon(fa fa-book)
    D
";

// ------------------------------------------------------------------ ER

fn counts(a: usize, b: usize) -> Vec<usize> {
    if b <= 40 {
        return (0..b + 6).collect();
    }
    let mut v: Vec<usize> = (0..4).chain(a.saturating_sub(2)..a + 3).chain(b.saturating_sub(2)..b + 6).collect();
    v.sort();
    v.dedup();
    v
}

fn pick_nodes(n: usize) -> Vec<usize> {
    let lim: usize = std::env::var("PROBE_NODES").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    if n <= lim { return (0..n).collect(); }
    let mut v: Vec<usize> = (0..lim).map(|k| k * n / lim).collect();
    v.dedup();
    v
}

fn er_at(es: &er::ErScene) -> Vec<(f64, f64)> {
    es.scene.nodes.iter().map(|n| (n.x, n.y)).collect()
}

fn er_frame_check(name: &str, d: &ErDiagram, base: &er::ErScene, at: &[(f64, f64)], to: &[(f64, f64)]) -> er::ErScene {
    let f = er::route_partial(d, to, base, at);
    let moved: Vec<bool> = to.iter().zip(at).map(|(a, b)| a != b).collect();
    if f.scene.nodes.len() != d.entities.len() {
        fail(format!("{name}: node count"));
    }
    for (k, n) in f.scene.nodes.iter().enumerate() {
        if (n.x, n.y) != to[k] {
            fail(format!("{name}: entity {k} at {:?} not {:?}", (n.x, n.y), to[k]));
        }
    }
    let full = er::route(d, to);
    if dbg(&f.tables) != dbg(&full.tables) || dbg(&f.cards) != dbg(&full.cards) {
        fail(format!("{name}: tables/cards differ from route"));
    }
    for (j, r) in d.relations.iter().enumerate() {
        if !moved[r.from] && !moved[r.to] && dbg(&f.scene.edges[j]) != dbg(&base.scene.edges[j]) {
            fail(format!("{name}: kept rel {j} changed"));
        }
        if moved[r.from] || moved[r.to] {
            let e = &f.scene.edges[j];
            if e.from != d.entities[r.from].name || e.to != d.entities[r.to].name {
                fail(format!("{name}: redrawn rel {j} ends {} {}", e.from, e.to));
            }
            if e.kind != full.scene.edges[j].kind || dbg(&e.ends) != dbg(&full.scene.edges[j].ends) {
                fail(format!("{name}: redrawn rel {j} style differs from route"));
            }
        }
    }
    // Canvas holds the tables.
    let (ox, oy) = f.scene.origin;
    for n in &f.scene.nodes {
        if n.x - n.w / 2.0 < ox - 1e-6
            || n.y - n.h / 2.0 < oy - 1e-6
            || n.x + n.w / 2.0 > ox + f.scene.width + 1e-6
            || n.y + n.h / 2.0 > oy + f.scene.height + 1e-6
        {
            fail(format!("{name}: table {} off canvas", n.id));
        }
    }
    if f.scene.origin.0 > base.scene.origin.0 || f.scene.origin.1 > base.scene.origin.1 {
        fail(format!("{name}: origin shrank"));
    }
    let _ = er::to_svg(&f);
    f
}

fn er_probe(all: &[(String, Document)]) {
    let mut frames = 0;
    for (name, doc) in all {
        let Document::Er(d) = doc else { continue };
        let ok = catch_unwind(AssertUnwindSafe(|| {
        let n = d.entities.len();
        let laid = er::scene(d);
        let at = er_at(&laid);
        let routed = er::route(d, &at);
        for (bn, base) in [("laid", &laid), ("routed", &routed)] {
            // no move = base
            if dbg(&er::route_partial(d, &at, base, &at)) != dbg(base) {
                fail(format!("{name}/{bn}: no-move != base"));
            }
            for i in pick_nodes(n) {
                let mut to = at.clone();
                to[i].0 += 37.0;
                to[i].1 -= 23.0;
                let f = er_frame_check(&format!("{name}/{bn}/{i}"), d, base, &at, &to);
                frames += 1;
                // chain with another node
                let mut to2 = to.clone();
                let k = (i + 1) % n;
                to2[k].0 -= 19.0;
                er_frame_check(&format!("{name}/{bn}/{i}+{k}"), d, &f, &to, &to2);
                // far top-left
                let mut far = at.clone();
                far[i] = (-420.0, -310.0);
                let ff = er_frame_check(&format!("{name}/{bn}/{i}/far"), d, base, &at, &far);
                if n > 0 && (ff.scene.origin.0 >= 0.0 || ff.scene.origin.1 >= 0.0) {
                    fail(format!("{name}: far drag origin {:?}", ff.scene.origin));
                }
                let svg = er::to_svg(&ff);
                if !svg.contains("viewBox=\"-") {
                    fail(format!("{name}: far drag svg viewBox not negative"));
                }
            }
            // counts
            for k in counts(n, n) {
                let c: Vec<(f64, f64)> = (0..k).map(|q| at.get(q).copied().unwrap_or((10.0 * q as f64, 5.0))).collect();
                let t = catch_unwind(AssertUnwindSafe(|| er::try_route_partial(d, &c, base, &at)));
                match t {
                    Err(_) => fail(format!("{name}: try_route_partial panicked at count {k}")),
                    Ok(v) => {
                        if v.is_some() != (k == n) {
                            fail(format!("{name}: try count {k} -> {}", v.is_some()));
                        }
                    }
                }
                let a = catch_unwind(AssertUnwindSafe(|| dbg(&er::route_partial(d, &c, base, &at)))).ok();
                let b = catch_unwind(AssertUnwindSafe(|| dbg(&er::route(d, &c)))).ok();
                if a.is_some() != b.is_some() {
                    fail(format!("{name}: count {k} panic-ness differs partial={} route={}", a.is_some(), b.is_some()));
                }
                if k != n {
                    if let (Some(a), Some(b)) = (&a, &b) {
                        if a != b {
                            fail(format!("{name}: count {k} differs from route"));
                        }
                    }
                }
                // base_centers count wrong
                if n > 0 {
                    let mut to = at.clone();
                    to[0].0 += 5.0;
                    let a = catch_unwind(AssertUnwindSafe(|| dbg(&er::route_partial(d, &to, base, &c)))).ok();
                    if k != n {
                        if a.as_deref() != Some(&dbg(&er::route(d, &to))) {
                            fail(format!("{name}: base count {k} not route"));
                        }
                    }
                }
            }
            // NaN / inf
            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 2e6, -2e6] {
                for i in 0..n.min(3) {
                    let mut c = at.clone();
                    c[i].0 = bad;
                    match catch_unwind(AssertUnwindSafe(|| er::try_route_partial(d, &c, base, &at))) {
                        Err(_) => fail(format!("{name}: try panics on {bad}")),
                        Ok(Some(_)) => fail(format!("{name}: try accepts {bad}")),
                        Ok(None) => {}
                    }
                    let mut to = at.clone();
                    to[i].1 += 3.0;
                    let mut bc = at.clone();
                    bc[i].1 = bad;
                    match catch_unwind(AssertUnwindSafe(|| er::try_route_partial(d, &to, base, &bc))) {
                        Err(_) => fail(format!("{name}: try panics on base {bad}")),
                        Ok(Some(s)) => {
                            if dbg(&s) != dbg(&er::route(d, &to)) {
                                fail(format!("{name}: bad base_centers {bad} not route"));
                            }
                        }
                        Ok(None) => fail(format!("{name}: try refuses good centers with bad base")),
                    }
                }
            }
        }
        }));
        if ok.is_err() {
            fail(format!("{name}: PANIC in probe body"));
        }
    }
    println!("er: {frames} frames checked");
    // empty ER
    if let Ok(Document::Er(d)) = parse_document("erDiagram\n") {
        let b = er::scene(&d);
        let r = catch_unwind(AssertUnwindSafe(|| er::route_partial(&d, &[], &b, &[])));
        match r {
            Err(_) => fail("empty er: route_partial panics".into()),
            Ok(s) => {
                if dbg(&s) != dbg(&er::route(&d, &[])) && dbg(&s) != dbg(&b) {
                    fail("empty er: neither route nor base".into())
                }
            }
        }
        let t = catch_unwind(AssertUnwindSafe(|| er::try_route_partial(&d, &[], &b, &[])));
        println!("empty er try: {:?}", t.map(|x| x.is_some()));
        let t = catch_unwind(AssertUnwindSafe(|| er::try_route_partial(&d, &[(1.0, 1.0)], &b, &[])));
        println!("empty er try 1 pos: {:?}", t.map(|x| x.is_some()));
    } else {
        println!("empty erDiagram does not parse");
    }
    // edits
    let d0 = er_of(ER_MIX);
    let base = er::scene(&d0);
    let at = er_at(&base);
    let routed = er::route(&d0, &at);
    let edits: Vec<(&str, String)> = vec![
        ("add entity+rel", format!("{ER_MIX}  SHIPPER ||--o{{ ORDER : ships\n")),
        ("add lone entity", format!("{ER_MIX}  LONELY {{\n    int x\n  }}\n")),
        ("remove rel", ER_MIX.replace("  ORDER }o--o{ PRODUCT : lists\n", "")),
        ("change card", ER_MIX.replace("LINE }o--|| PRODUCT", "LINE }|--|| PRODUCT")),
        ("identifying->non", ER_MIX.replace("ORDER ||--|{ LINE", "ORDER ||..|{ LINE")),
        ("label", ER_MIX.replace(": of", ": off")),
        ("attr type", ER_MIX.replace("string name PK", "int name PK")),
        ("attr key", ER_MIX.replace("string name PK", "string name FK")),
        ("attr comment", ER_MIX.replace("\"contact\"", "\"mail\"")),
        ("rename", ER_MIX.replace("CATEGORY", "KATEGORY")),
        ("swap rel order", ER_MIX.replace("  ORDER ||--|{ LINE : contains\n  LINE }o--|| PRODUCT : of\n", "  LINE }o--|| PRODUCT : of\n  ORDER ||--|{ LINE : contains\n")),
        ("reverse rel", ER_MIX.replace("LINE }o--|| PRODUCT : of", "PRODUCT ||--o{ LINE : of")),
    ];
    for (en, src) in &edits {
        let Ok(Document::Er(d2)) = parse_document(src) else {
            println!("edit {en} does not parse");
            continue;
        };
        let names0: Vec<&str> = d0.entities.iter().map(|e| e.name.as_str()).collect();
        // positions: map by name, new ones placed far right.
        let mut c2: Vec<(f64, f64)> = d2
            .entities
            .iter()
            .enumerate()
            .map(|(k, e)| names0.iter().position(|x| *x == e.name).map(|p| at[p]).unwrap_or((900.0 + 50.0 * k as f64, 100.0)))
            .collect();
        for (bn, b) in [("laid", &base), ("routed", &routed)] {
            // no move
            let a = er::route_partial(&d2, &c2, b, &at);
            if dbg(&a) != dbg(&er::route(&d2, &c2)) {
                fail(format!("er edit {en}/{bn} (no move): not route"));
            }
            c2[0].0 += 13.0;
            let a = er::route_partial(&d2, &c2, b, &at);
            if dbg(&a) != dbg(&er::route(&d2, &c2)) {
                fail(format!("er edit {en}/{bn} (moved): not route"));
            }
            c2[0].0 -= 13.0;
        }
    }
}

fn er_of(s: &str) -> ErDiagram {
    match parse_document(s) {
        Ok(Document::Er(d)) => d,
        e => panic!("{e:?}"),
    }
}
fn class_of(s: &str) -> ClassDiagram {
    match parse_document(s) {
        Ok(Document::Class(d)) => d,
        e => panic!("{e:?}"),
    }
}
fn mind_of(s: &str) -> Mindmap {
    match parse_document(s) {
        Ok(Document::Mindmap(d)) => d,
        e => panic!("{e:?}"),
    }
}

// --------------------------------------------------------------- class

fn cl_at(cs: &class::ClassScene) -> Vec<(f64, f64)> {
    cs.scene.nodes.iter().map(|n| (n.x, n.y)).collect()
}

fn class_frame_check(name: &str, d: &ClassDiagram, base: &class::ClassScene, bat: &[(f64, f64)], to: &[(f64, f64)]) -> class::ClassScene {
    let f = class::route_partial(d, to, base, bat);
    let full = class::route(d, to);
    let n = d.classes.len();
    // positions: as route places them
    let fpos = cl_at(&f);
    let rpos = cl_at(&full);
    if fpos != rpos {
        fail(format!("{name}: node positions differ from route's"));
    }
    for k in 0..to.len().min(fpos.len()) {
        if fpos[k] != to[k] {
            fail(format!("{name}: node {k} at {:?} not {:?}", fpos[k], to[k]));
        }
    }
    if dbg(&f.boxes) != dbg(&full.boxes) || dbg(&f.rels) != dbg(&full.rels) || dbg(&f.notes) != dbg(&full.notes) {
        fail(format!("{name}: boxes/rels/notes differ from route"));
    }
    // moved by node id, against base positions
    let bpos = cl_at(base);
    let moved: HashSet<String> = f
        .scene
        .nodes
        .iter()
        .enumerate()
        .filter(|(k, _)| bpos.get(*k) != Some(&fpos[*k]))
        .map(|(_, nd)| nd.id.clone())
        .collect();
    let bspots = class::card_spots(base);
    let fspots = class::card_spots(&f);
    for (j, e) in f.scene.edges.iter().enumerate() {
        let be = &base.scene.edges[j];
        let kept = !moved.contains(&be.from) && !moved.contains(&be.to);
        if kept {
            if dbg(e) != dbg(be) {
                fail(format!("{name}: kept edge {j} ({}->{}) changed", be.from, be.to));
            }
            if j < d.relations.len() && bspots[j] != fspots[j] {
                fail(format!("{name}: kept rel {j} cards moved {:?} -> {:?}", bspots[j], fspots[j]));
            }
        }
    }
    let (ox, oy) = f.scene.origin;
    for nd in &f.scene.nodes {
        if nd.x - nd.w / 2.0 < ox - 1e-6
            || nd.y - nd.h / 2.0 < oy - 1e-6
            || nd.x + nd.w / 2.0 > ox + f.scene.width + 1e-6
            || nd.y + nd.h / 2.0 > oy + f.scene.height + 1e-6
        {
            fail(format!("{name}: box {} off canvas", nd.id));
        }
    }
    let _ = n;
    let _ = class::to_svg(&f);
    f
}

fn class_probe(all: &[(String, Document)]) {
    let mut frames = 0;
    for (name, doc) in all {
        let Document::Class(d) = doc else { continue };
        let ok = catch_unwind(AssertUnwindSafe(|| {
        let n = d.classes.len();
        let nn = n + d.notes.len();
        let laid = class::scene(d);
        let at = cl_at(&laid); // classes + notes
        let at_c: Vec<(f64, f64)> = at[..n].to_vec();
        let routed = class::route(d, &at);
        let routed_c = class::route(d, &at_c);
        let bases: Vec<(&str, &class::ClassScene, Vec<(f64, f64)>)> = vec![
            ("laid/all", &laid, at.clone()),
            ("routed/all", &routed, at.clone()),
            ("routed_c/classes", &routed_c, at_c.clone()),
            ("routed_c/all", &routed_c, cl_at(&routed_c)),
        ];
        for (bn, base, bat) in &bases {
            if dbg(&class::route_partial(d, bat, base, bat)) != dbg(*base) {
                fail(format!("{name}/{bn}: no-move != base"));
            }
            for i in pick_nodes(nn) {
                for with_notes in [true, false] {
                    if !with_notes && i >= n {
                        continue;
                    }
                    let mut to: Vec<(f64, f64)> = if with_notes { cl_at(base) } else { cl_at(base)[..n].to_vec() };
                    to[i].0 += 37.0;
                    to[i].1 -= 23.0;
                    let f = class_frame_check(&format!("{name}/{bn}/{i}/notes={with_notes}"), d, base, bat, &to);
                    frames += 1;
                    let mut to2 = to.clone();
                    let k = (i + 1) % to.len();
                    to2[k].1 += 17.0;
                    class_frame_check(&format!("{name}/{bn}/{i}+{k}"), d, &f, &to, &to2);
                    let mut far = to.clone();
                    far[i] = (-420.0, -310.0);
                    let ff = class_frame_check(&format!("{name}/{bn}/{i}/far"), d, base, bat, &far);
                    if ff.scene.origin.0 >= 0.0 || ff.scene.origin.1 >= 0.0 {
                        fail(format!("{name}: far origin {:?}", ff.scene.origin));
                    }
                }
            }
            for k in counts(n, nn) {
                let c: Vec<(f64, f64)> = (0..k).map(|q| at.get(q).copied().unwrap_or((10.0 * q as f64, 5.0))).collect();
                let ok = k == n || k == nn;
                match catch_unwind(AssertUnwindSafe(|| class::try_route_partial(d, &c, base, bat))) {
                    Err(_) => fail(format!("{name}: try panicked at count {k}")),
                    Ok(v) => {
                        if v.is_some() != ok {
                            fail(format!("{name}: try count {k} -> {}", v.is_some()));
                        }
                    }
                }
                let a = catch_unwind(AssertUnwindSafe(|| dbg(&class::route_partial(d, &c, base, bat)))).ok();
                let b = catch_unwind(AssertUnwindSafe(|| dbg(&class::route(d, &c)))).ok();
                if a.is_some() != b.is_some() {
                    fail(format!("{name}: count {k} panic-ness differs partial={} route={}", a.is_some(), b.is_some()));
                }
                if !ok {
                    if let (Some(a), Some(b)) = (&a, &b) {
                        if a != b {
                            fail(format!("{name}: count {k} differs from route"));
                        }
                    }
                }
                if !ok && n > 0 {
                    let mut to = bat.clone();
                    to[0].0 += 5.0;
                    let a = catch_unwind(AssertUnwindSafe(|| dbg(&class::route_partial(d, &to, base, &c)))).ok();
                    if a.as_deref() != Some(&dbg(&class::route(d, &to))) {
                        fail(format!("{name}: base count {k} not route"));
                    }
                }
            }
            for bad in [f64::NAN, f64::INFINITY, -2e6] {
                for i in 0..n.min(2) {
                    let mut c = bat.clone();
                    c[i].1 = bad;
                    match catch_unwind(AssertUnwindSafe(|| class::try_route_partial(d, &c, base, bat))) {
                        Err(_) => fail(format!("{name}: try panics on {bad}")),
                        Ok(Some(_)) => fail(format!("{name}: try accepts {bad}")),
                        Ok(None) => {}
                    }
                    let mut to = bat.clone();
                    to[i].1 += 3.0;
                    match catch_unwind(AssertUnwindSafe(|| class::try_route_partial(d, &to, base, &c))) {
                        Err(_) => fail(format!("{name}: try panics on base {bad}")),
                        Ok(Some(s)) => {
                            if dbg(&s) != dbg(&class::route(d, &to)) {
                                fail(format!("{name}: bad base {bad} not route"));
                            }
                        }
                        Ok(None) => fail(format!("{name}: try refuses good centers")),
                    }
                }
            }
        }
        }));
        if ok.is_err() {
            fail(format!("{name}: PANIC in probe body"));
        }
    }
    println!("class: {frames} frames checked");
    for src in ["classDiagram\n", "classDiagram\n  note \"only a note\"\n"] {
        match parse_document(src) {
            Ok(Document::Class(d)) => {
                let b = class::scene(&d);
                let at = cl_at(&b);
                for c in [vec![], at.clone()] {
                    let r = catch_unwind(AssertUnwindSafe(|| dbg(&class::route_partial(&d, &c, &b, &at))));
                    let q = catch_unwind(AssertUnwindSafe(|| dbg(&class::route(&d, &c))));
                    println!("empty class {:?} len {}: partial ok={} route ok={} try={:?}", src, c.len(), r.is_ok(), q.is_ok(),
                        catch_unwind(AssertUnwindSafe(|| class::try_route_partial(&d, &c, &b, &at).is_some())));
                    if r.is_err() && q.is_ok() {
                        fail("empty class partial panics where route does not".into());
                    }
                }
            }
            _ => println!("{src:?} does not parse"),
        }
    }
    // edits
    let d0 = class_of(CLASS_NOTES);
    let base = class::scene(&d0);
    let at = cl_at(&base);
    let routed = class::route(&d0, &at);
    let n0 = d0.classes.len();
    let edits: Vec<(&str, String)> = vec![
        ("add class", format!("{CLASS_NOTES}  class Extra\n")),
        ("add class+rel", format!("{CLASS_NOTES}  Duck --> Pond\n")),
        ("remove rel", CLASS_NOTES.replace("  Fish ..> Water\n", "")),
        ("add member", CLASS_NOTES.replace("+eat() void", "+eat() void\n    +sleep() void")),
        ("member text", CLASS_NOTES.replace("+String name", "+String nom")),
        ("add note", format!("{CLASS_NOTES}  note for Animal \"base\"\n")),
        ("remove note", CLASS_NOTES.replace("  note for Fish \"swims\"\n", "")),
        ("note text", CLASS_NOTES.replace("can fly", "can fly!")),
        ("note target", CLASS_NOTES.replace("note for Fish", "note for Animal")),
        ("rel kind", CLASS_NOTES.replace("Animal <|-- Fish", "Animal *-- Fish")),
        ("rel dashed", CLASS_NOTES.replace("Fish ..> Water", "Fish --> Water")),
        ("card", CLASS_NOTES.replace("\"*\" Egg", "\"0..1\" Egg")),
        ("label", CLASS_NOTES.replace(": lays", ": drops")),
        ("annotation", format!("{CLASS_NOTES}  <<interface>> Fish\n")),
        ("generic", CLASS_NOTES.replace("class Duck\n", "class Duck~T~\n")),
    ];
    for (en, src) in &edits {
        let Ok(Document::Class(d2)) = parse_document(src) else {
            println!("class edit {en} does not parse");
            continue;
        };
        let names0: Vec<&str> = d0.classes.iter().map(|c| c.name.as_str()).collect();
        let mut c2: Vec<(f64, f64)> = d2
            .classes
            .iter()
            .enumerate()
            .map(|(k, c)| names0.iter().position(|x| *x == c.name).map(|p| at[p]).unwrap_or((900.0 + 60.0 * k as f64, 100.0)))
            .collect();
        // classes-only centres and with notes (notes kept by order where counts allow)
        let mut variants = vec![c2.clone()];
        let mut withn = c2.clone();
        for k in 0..d2.notes.len() {
            withn.push(at.get(n0 + k).copied().unwrap_or((50.0, 800.0 + 40.0 * k as f64)));
        }
        variants.push(withn);
        for v in variants.iter_mut() {
            for (bn, b) in [("laid", &base), ("routed", &routed)] {
                let a = class::route_partial(&d2, v, b, &at);
                if dbg(&a) != dbg(&class::route(&d2, v)) {
                    fail(format!("class edit {en}/{bn}/len{} (no move): not route", v.len()));
                }
                v[0].0 += 13.0;
                let a = class::route_partial(&d2, v, b, &at);
                if dbg(&a) != dbg(&class::route(&d2, v)) {
                    fail(format!("class edit {en}/{bn}/len{} (moved): not route", v.len()));
                }
                v[0].0 -= 13.0;
            }
        }
        c2.clear();
    }
}

// ------------------------------------------------------------- mindmap

fn mm_at(ms: &mindmap::MindScene) -> Vec<(f64, f64)> {
    ms.nodes.iter().map(|n| (n.cx(), n.cy())).collect()
}

fn close(a: &mindmap::MindScene, b: &mindmap::MindScene) -> bool {
    if a.nodes.len() != b.nodes.len() || a.edges.len() != b.edges.len() {
        return false;
    }
    let c = |x: f64, y: f64| (x - y).abs() <= 1e-9 * x.abs().max(1.0);
    a.nodes.iter().zip(&b.nodes).all(|(p, q)| c(p.x, q.x) && c(p.y, q.y) && p.w == q.w && p.h == q.h && p.text == q.text && p.fill == q.fill)
        && a.edges.iter().zip(&b.edges).all(|(p, q)| p.bezier.iter().zip(&q.bezier).all(|(u, v)| c(u.0, v.0) && c(u.1, v.1)) && p.color == q.color && p.width == q.width)
        && c(a.width, b.width)
        && c(a.height, b.height)
}

fn mind_probe(all: &[(String, Document)]) {
    let mut frames = 0;
    for (name, doc) in all {
        let Document::Mindmap(d) = doc else { continue };
        let ok = catch_unwind(AssertUnwindSafe(|| {
        let n = d.nodes.len();
        let laid = mindmap::scene(d);
        let at = mm_at(&laid);
        let routed = mindmap::route(d, &at);
        let ends = mindmap::link_ends(d);
        if ends.len() != laid.edges.len() {
            fail(format!("{name}: link_ends len"));
        }
        for (j, &(p, c)) in ends.iter().enumerate() {
            let e = &laid.edges[j];
            let (a, z) = (&laid.nodes[p], &laid.nodes[c]);
            if (e.bezier[0].0 - a.cx()).abs() > 1e-6 || (e.bezier[3].1 - z.cy()).abs() > 1e-6 {
                fail(format!("{name}: link_ends {j} does not match the drawn connector"));
            }
        }
        for (bn, base) in [("laid", &laid), ("routed", &routed)] {
            if dbg(&mindmap::route_partial(d, &at, base, &at)) != dbg(base) {
                fail(format!("{name}/{bn}: no-move != base"));
            }
            for i in pick_nodes(n) {
                let mut to = at.clone();
                to[i].0 += 37.0;
                to[i].1 -= 23.0;
                let f = mindmap::route_partial(d, &to, base, &at);
                frames += 1;
                let full = mindmap::route(d, &to);
                if !close(&f, &full) {
                    fail(format!("{name}/{bn}/{i}: not route (to rounding)"));
                }
                for (j, &(p, c)) in ends.iter().enumerate() {
                    if p != i && c != i && dbg(&f.edges[j]) != dbg(&base.edges[j]) {
                        fail(format!("{name}/{bn}/{i}: kept connector {j} changed"));
                    }
                }
                for (k, nd) in f.nodes.iter().enumerate() {
                    if k != i && dbg(nd) != dbg(&base.nodes[k]) {
                        fail(format!("{name}/{bn}/{i}: unmoved node {k} changed"));
                    }
                }
                if (f.nodes[i].cx() - to[i].0).abs() > 1e-9 || (f.nodes[i].cy() - to[i].1).abs() > 1e-9 {
                    fail(format!("{name}: moved node not at position"));
                }
                // far
                let mut far = at.clone();
                far[i] = (-420.0, -310.0);
                let ff = mindmap::route_partial(d, &far, base, &at);
                if !close(&ff, &mindmap::route(d, &far)) {
                    fail(format!("{name}/{bn}/{i}: far drag not route"));
                }
                // chain
                let mut to2 = to.clone();
                to2[(i + 1) % n].1 += 9.0;
                let f2 = mindmap::route_partial(d, &to2, &f, &to);
                if !close(&f2, &mindmap::route(d, &to2)) {
                    fail(format!("{name}/{bn}/{i}: chain not route"));
                }
                let _ = mindmap::to_svg(&f);
            }
            for k in counts(n, n) {
                let c: Vec<(f64, f64)> = (0..k).map(|q| at.get(q).copied().unwrap_or((10.0 * q as f64, 5.0))).collect();
                match catch_unwind(AssertUnwindSafe(|| mindmap::try_route_partial(d, &c, base, &at))) {
                    Err(_) => fail(format!("{name}: try panicked count {k}")),
                    Ok(v) => {
                        if v.is_some() != (k == n) {
                            fail(format!("{name}: try count {k} -> {}", v.is_some()));
                        }
                    }
                }
                match catch_unwind(AssertUnwindSafe(|| dbg(&mindmap::route_partial(d, &c, base, &at)))) {
                    Err(_) => fail(format!("{name}: partial panicked count {k}")),
                    Ok(a) => {
                        if k != n && a != dbg(&mindmap::route(d, &c)) {
                            fail(format!("{name}: count {k} not route"));
                        }
                    }
                }
                let mut to = at.clone();
                to[0].0 += 4.0;
                match catch_unwind(AssertUnwindSafe(|| dbg(&mindmap::route_partial(d, &to, base, &c)))) {
                    Err(_) => fail(format!("{name}: partial panicked base count {k}")),
                    Ok(a) => {
                        if k != n && a != dbg(&mindmap::route(d, &to)) {
                            fail(format!("{name}: base count {k} not route"));
                        }
                    }
                }
            }
            for bad in [f64::NAN, f64::INFINITY, -2e6] {
                let mut c = at.clone();
                c[0].0 = bad;
                if !matches!(catch_unwind(AssertUnwindSafe(|| mindmap::try_route_partial(d, &c, base, &at))), Ok(None)) {
                    fail(format!("{name}: try with {bad} not None"));
                }
                let r = catch_unwind(AssertUnwindSafe(|| dbg(&mindmap::route_partial(d, &c, base, &at))));
                let q = catch_unwind(AssertUnwindSafe(|| dbg(&mindmap::route(d, &c))));
                if r.is_ok() != q.is_ok() {
                    fail(format!("{name}: {bad} panic-ness differs"));
                }
                let mut to = at.clone();
                to[0].1 += 2.0;
                match catch_unwind(AssertUnwindSafe(|| mindmap::try_route_partial(d, &to, base, &c))) {
                    Ok(Some(s)) => {
                        if dbg(&s) != dbg(&mindmap::route(d, &to)) {
                            fail(format!("{name}: bad base {bad} not route"));
                        }
                    }
                    _ => fail(format!("{name}: bad base {bad} try failed")),
                }
            }
        }
        }));
        if ok.is_err() {
            fail(format!("{name}: PANIC in probe body"));
        }
    }
    println!("mindmap: {frames} frames checked");
    let empty = Mindmap::default();
    let b = mindmap::route(&empty, &[]);
    for c in [vec![], vec![(1.0, 1.0)]] {
        let r = catch_unwind(AssertUnwindSafe(|| dbg(&mindmap::route_partial(&empty, &c, &b, &[]))));
        let t = catch_unwind(AssertUnwindSafe(|| mindmap::try_route_partial(&empty, &c, &b, &[]).is_some()));
        println!("empty mindmap len {}: partial ok={} (== route {}) try={:?}", c.len(), r.is_ok(), r.as_ref().ok() == Some(&dbg(&mindmap::route(&empty, &c))), t);
    }
    // edits
    let d0 = mind_of(MIND);
    let base = mindmap::scene(&d0);
    let at = mm_at(&base);
    let edits: Vec<(&str, String)> = vec![
        ("add node", format!("{MIND}    E\n")),
        ("add child mid", MIND.replace("      A1\n", "      A1\n      A1b\n")),
        ("remove node", MIND.replace("    D\n", "")),
        ("remove mid", MIND.replace("      A1\n", "")),
        ("reparent", MIND.replace("    D\n", "      D\n")),
        ("text", MIND.replace("B1\n", "B1x\n")),
        ("shape", MIND.replace("A[Square]", "A(Square)")),
        ("icon", MIND.replace("fa fa-book", "fa fa-car")),
        ("class", format!("{MIND}    :::urgent\n")),
        ("swap order", MIND.replace("    D\n", "").replace("    B(Round)", "    D\n    B(Round)")),
    ];
    for (en, src) in &edits {
        let Ok(Document::Mindmap(d2)) = parse_document(src) else {
            println!("mind edit {en} does not parse");
            continue;
        };
        let names0: Vec<&str> = d0.nodes.iter().map(|x| x.text.as_str()).collect();
        let mut c2: Vec<(f64, f64)> = d2
            .nodes
            .iter()
            .enumerate()
            .map(|(k, x)| names0.iter().position(|t| *t == x.text).map(|p| at[p]).unwrap_or((700.0 + 30.0 * k as f64, 50.0)))
            .collect();
        let a = mindmap::route_partial(&d2, &c2, &base, &at);
        if dbg(&a) != dbg(&mindmap::route(&d2, &c2)) {
            fail(format!("mind edit {en} (no move): not route"));
        }
        c2[1].0 += 13.0;
        let a = mindmap::route_partial(&d2, &c2, &base, &at);
        if dbg(&a) != dbg(&mindmap::route(&d2, &c2)) {
            fail(format!("mind edit {en} (moved): not route"));
        }
    }
}

fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    std::panic::set_hook(Box::new(|_| {}));
    let all = docs();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for (_, d) in &all {
        let k = match d {
            Document::Er(_) => "er",
            Document::Class(_) => "class",
            Document::Mindmap(_) => "mindmap",
            _ => "other",
        };
        *counts.entry(k).or_default() += 1;
    }
    println!("docs: {counts:?}");
    if which.is_empty() || which == "er" {
        er_probe(&all);
    }
    if which.is_empty() || which == "class" {
        class_probe(&all);
    }
    if which.is_empty() || which == "mind" {
        mind_probe(&all);
    }
    unsafe {
        println!("TOTAL FAILS: {FAILS}");
    }
}
