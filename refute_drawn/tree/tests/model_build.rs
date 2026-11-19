//! Downstream construction of the diagram models (and reading of the
//! scenes) from outside the crate.
//!
//! Several model structs became `#[non_exhaustive]` when they gained
//! fields, which rules out struct literals in other crates. Each one
//! must still be buildable from here — through `Default` plus field
//! assignment, or a documented constructor — and a hand-built model must
//! render exactly like the parsed source it mirrors, so the constructors
//! apply the parser's own rules. The scene structs are built by each
//! module's `scene()`; hosts read (and may mutate) their fields.

use flowmaid::model::{
    ArchEdge, ArchGroup, ArchService, ArchSide, Architecture, Attr, Card, ClassDiagram, ClassNote,
    ClassRel, Direction, Document, ErDiagram, Journey, JourneySection, JourneyTask, Key, MindShape,
    Mindmap, NodeStyle, PieChart, PieSlice, RelKind, Relation, SankeyConfig, SankeyDiagram,
    SankeyLabelStyle, SankeyLink, SeqHead, SeqItem, SequenceDiagram, Timeline, TimelinePeriod,
    TimelineSection,
};
use flowmaid::parser::parse_document;
use flowmaid::render;

fn parse(src: &str) -> Document {
    parse_document(src).unwrap_or_else(|e| panic!("{src}\n{e:?}"))
}

#[test]
fn er_entity_is_built_with_ensure_entity() {
    let src = "erDiagram\n\
               direction LR\n\
               p[Person] {\n\
               string name PK \"full name\"\n\
               }\n\
               p ||--o{ CAR : drives\n";
    let Document::Er(parsed) = parse(src) else {
        panic!("not ER")
    };

    let mut d = ErDiagram::default();
    d.direction = Direction::LR;
    let p = d.ensure_entity("p");
    d.entities[p].alias = Some("Person".to_string());
    d.entities[p].attrs.push(Attr {
        ty: "string".to_string(),
        name: "name".to_string(),
        keys: vec![Key::Pk],
        comment: Some("full name".to_string()),
    });
    let car = d.ensure_entity("CAR");
    d.relations.push(Relation {
        from: p,
        to: car,
        card_from: Card::One,
        card_to: Card::ZeroMany,
        identifying: true,
        label: Some("drives".to_string()),
    });
    assert_eq!(d.entities[p].title(), "Person");
    assert_eq!(render::render_er(&d), render::render_er(&parsed));

    // Scene structs: read the fields added for GUI painters.
    let es = flowmaid::er::scene(&d);
    let t = &es.tables[p];
    assert_eq!((t.name.as_str(), t.title.as_str()), ("p", "Person"));
    assert_eq!(t.rows[0].comment, "full name");
    assert!(t.rows[0].comment_full.is_none());
    assert!(t.comment_col_w > 0.0 && t.comment_x() > t.name_x());
}

#[test]
fn pie_chart_is_built_from_default() {
    let src = "pie showData\n\
               title Pets\n\
               accTitle: Pets owned\n\
               \"Dogs\" : 386\n\
               \"Cats\" : 85\n\
               \"Rats\" : 6\n";
    let Document::Pie(parsed) = parse(src) else {
        panic!("not pie")
    };

    let mut d = PieChart::default();
    d.title = Some("Pets".to_string());
    d.show_data = true;
    d.acc_title = Some("Pets owned".to_string());
    for (label, value) in [("Dogs", 386.0), ("Cats", 85.0), ("Rats", 6.0)] {
        d.slices.push(PieSlice {
            label: label.to_string(),
            value,
        });
    }
    let svg = render::render_pie(&d);
    assert_eq!(svg, render::render_pie(&parsed));
    assert!(svg.contains("Pets owned"), "accTitle names the SVG");

    let ps = flowmaid::pie::scene(&d);
    assert_eq!(ps.labels.len(), 3, "every slice of at least 1% is labelled");
    let l = &ps.labels[2];
    assert_eq!((l.slice, l.text.as_str()), (2, "1%"));
    assert!(l.inside || !l.leader.is_empty());
}

#[test]
fn journey_is_built_from_default_and_task_new() {
    let src = "journey\n\
               title My day\n\
               accTitle: Day\n\
               section Morning\n\
               Make tea: 3.5: Me, Cat\n\
               Go upstairs: -1: Me\n\
               section Work\n\
               Code: 9: Me\n";
    let Document::Journey(parsed) = parse(src) else {
        panic!("not journey")
    };

    let mut d = Journey::default();
    d.title = Some("My day".to_string());
    d.acc_title = Some("Day".to_string());
    d.actors = vec!["Me".to_string(), "Cat".to_string()];
    d.sections.push(JourneySection {
        name: "Morning".to_string(),
        tasks: vec![
            JourneyTask::new("Make tea", 3.5, vec![0, 1]),
            JourneyTask::new("Go upstairs", -1.0, vec![0]),
        ],
    });
    d.sections.push(JourneySection {
        name: "Work".to_string(),
        tasks: vec![JourneyTask::new("Code", 9.0, vec![0])],
    });
    // `new` reads a score exactly as the parser does.
    let t = &d.sections[0].tasks[0];
    assert_eq!((t.score, t.exact_score), (4, 3.5));
    let t = &d.sections[0].tasks[1];
    assert_eq!((t.score, t.exact_score), (1, 1.0), "-1 clamps to 1");
    let t = &d.sections[1].tasks[0];
    assert_eq!((t.score, t.exact_score), (5, 5.0), "9 clamps to 5");
    let nan = JourneyTask::new("?", f64::NAN, Vec::new());
    assert_eq!((nan.score, nan.exact_score), (3, 3.0), "NaN is neutral");
    assert_eq!(render::render_journey(&d), render::render_journey(&parsed));

    let js = flowmaid::journey::scene(&d);
    let g = &js.tasks[0];
    assert_eq!((g.exact_score, g.actor_ids.clone()), (3.5, vec![0, 1]));
    assert_eq!(g.label, "Make tea");
}

#[test]
fn timeline_is_built_from_default() {
    let src = "timeline\n\
               title History\n\
               accDescr: Social media\n\
               2002 : LinkedIn\n\
               section Later\n\
               2004 : Facebook : Google\n";
    let Document::Timeline(parsed) = parse(src) else {
        panic!("not timeline")
    };

    let mut d = Timeline::default();
    d.title = Some("History".to_string());
    d.acc_descr = Some("Social media".to_string());
    d.sections.push(TimelineSection {
        name: String::new(),
        periods: vec![TimelinePeriod {
            period: "2002".to_string(),
            events: vec!["LinkedIn".to_string()],
        }],
    });
    d.sections.push(TimelineSection {
        name: "Later".to_string(),
        periods: vec![TimelinePeriod {
            period: "2004".to_string(),
            events: vec!["Facebook".to_string(), "Google".to_string()],
        }],
    });
    let svg = render::render_timeline(&d);
    assert_eq!(svg, render::render_timeline(&parsed));
    assert!(svg.contains(">Social media</desc>"));

    let ts = flowmaid::timeline::scene(&d);
    let p = &ts.sections[1].periods[0];
    assert_eq!(p.label, "2004");
    assert!(!p.line_segments.is_empty());
}

/// Needs its `_` arm: `MindShape` is `#[non_exhaustive]`.
fn shape_name(s: MindShape) -> &'static str {
    match s {
        MindShape::Default => "default",
        MindShape::Rounded => "pill",
        MindShape::Circle => "circle",
        _ => "other",
    }
}

#[test]
fn mindmap_nodes_are_built_with_add_node() {
    let src = "mindmap\n\
               \x20 root((Root))\n\
               \x20   A[Square]\n\
               \x20     a1\n\
               \x20     :::hot\n\
               \x20   B(Pill)\n\
               \x20   ::icon(fa fa-book)\n\
               \x20     b1))Bang((\n";
    let Document::Mindmap(parsed) = parse(src) else {
        panic!("not mindmap")
    };

    let mut d = Mindmap::default();
    let root = d.add_node(None, "Root", MindShape::Circle);
    let a = d.add_node(Some(root), "Square", MindShape::Square);
    let a1 = d.add_node(Some(a), "a1", MindShape::default());
    d.nodes[a1].classes.push("hot".to_string());
    let b = d.add_node(Some(root), "Pill", MindShape::Rounded);
    d.nodes[b].icon = Some("fa fa-book".to_string());
    let b1 = d.add_node(Some(b), "Bang", MindShape::Bang);
    assert_eq!(format!("{:?}", d.nodes), format!("{:?}", parsed.nodes));
    assert_eq!((d.nodes[b1].depth, d.nodes[b1].branch), (2, Some(b)));
    assert_eq!(d.nodes[root].children, vec![a, b]);
    assert_eq!(render::render_mindmap(&d), render::render_mindmap(&parsed));

    assert_eq!(shape_name(MindShape::default()), "default");
    assert_eq!(shape_name(d.nodes[b1].shape), "other");

    // `None` once a root exists hangs the node off the root, as the
    // parser does with a stray shallow line.
    let stray = d.add_node(None, "stray", MindShape::Default);
    assert_eq!(d.nodes[stray].parent, Some(root));
    assert_eq!(d.nodes[stray].branch, Some(stray));
    d.nodes[stray].style = NodeStyle {
        fill: Some("#204060".to_string()),
        ..NodeStyle::default()
    };
    let ms = flowmaid::mindmap::scene(&d);
    let n = &ms.nodes[b];
    assert_eq!(n.icon.as_deref(), Some("fa fa-book"));
    assert!(n.font_size > 0.0 && n.font_weight >= 400);
    assert_eq!(ms.nodes[stray].paint(), "#204060");
}

#[test]
fn architecture_is_built_from_default_and_constructors() {
    let src = "architecture-beta\n\
               accTitle: Shop\n\
               group api(cloud)[API]\n\
               service db(database)[Database] in api\n\
               service web(internet)[Web]\n\
               junction j\n\
               web:R -[calls]-> L:db\n\
               db{group}:B -- T:j\n";
    let Document::Architecture(parsed) = parse(src) else {
        panic!("not architecture")
    };

    let mut d = Architecture::default();
    d.acc_title = Some("Shop".to_string());
    d.groups.push(ArchGroup {
        id: "api".to_string(),
        title: "API".to_string(),
        icon: Some("cloud".to_string()),
        parent: None,
    });
    let mut db = ArchService::new("db", "Database");
    db.icon = Some("database".to_string());
    db.group = Some(0);
    let mut web = ArchService::new("web", "Web");
    web.icon = Some("internet".to_string());
    let mut j = ArchService::new("j", "");
    j.junction = true;
    d.services = vec![db, web, j];
    let mut calls = ArchEdge::new(1, ArchSide::R, 0, ArchSide::L);
    calls.arrow_end = true;
    calls.title = Some("calls".to_string());
    let mut down = ArchEdge::new(0, ArchSide::B, 2, ArchSide::T);
    down.from_group = true;
    d.edges = vec![calls, down];
    assert_eq!(
        format!("{:?}", d.services),
        format!("{:?}", parsed.services)
    );
    assert_eq!(format!("{:?}", d.edges), format!("{:?}", parsed.edges));
    assert_eq!(
        render::render_architecture(&d),
        render::render_architecture(&parsed)
    );

    let a = flowmaid::architecture::scene(&d);
    assert_eq!(a.title_at.len(), a.scene.clusters.len());
}

#[test]
fn sankey_config_is_built_from_default() {
    let src = "---\n\
               config:\n\
               \x20 sankey:\n\
               \x20   labelStyle: outlined\n\
               \x20   nodeColors:\n\
               \x20     A: \"#33a35c\"\n\
               ---\n\
               sankey-beta\n\
               A,B,10\n\
               B,C,5\n";
    let Document::Sankey(parsed) = parse(src) else {
        panic!("not sankey")
    };

    let mut config = SankeyConfig::default();
    config.label_style = SankeyLabelStyle::Outlined;
    config
        .node_colors
        .push(("A".to_string(), "#33a35c".to_string()));
    let mut d = SankeyDiagram {
        config,
        ..SankeyDiagram::default()
    };
    for (s, t, value) in [("A", "B", 10.0), ("B", "C", 5.0)] {
        let source = d.ensure_node(s);
        let target = d.ensure_node(t);
        d.links.push(SankeyLink {
            source,
            target,
            value,
        });
    }
    assert_eq!(d.config, parsed.config);
    assert_eq!(render::render_sankey(&d), render::render_sankey(&parsed));

    let ss = flowmaid::sankey::scene(&d);
    assert_eq!(ss.label_style, SankeyLabelStyle::Outlined);
    assert_eq!(ss.nodes[0].custom_color.as_deref(), Some("#33a35c"));
    assert_eq!(ss.nodes[0].fill(), "#33a35c");
}

#[test]
fn sequence_default_mirrors_actors_like_the_parser() {
    let Document::Sequence(parsed) = parse("sequenceDiagram\nactor A\nA->>B: hi\n") else {
        panic!("not sequence")
    };

    let mut d = SequenceDiagram::default();
    assert!(d.mirror_actors && !d.scoped_frames, "the parser's defaults");
    let a = d.ensure_participant("A");
    d.participants[a].actor = true;
    let b = d.ensure_participant("B");
    d.items.push(SeqItem::Message {
        from: a,
        to: b,
        text: "hi".to_string(),
        dashed: false,
        head: SeqHead::Filled,
        activate: false,
        deactivate: false,
    });
    assert_eq!(render::render_seq(&d), render::render_seq(&parsed));

    // Hosts read the new scene fields; struct update syntax is out for
    // `#[non_exhaustive]` scenes, so a modified copy is cloned and set.
    let sc = flowmaid::seq::scene(&d);
    assert_eq!(sc.footer.len(), sc.boxes.len());
    let fig = flowmaid::seq::actor_figure(&sc.boxes[a]);
    assert!(fig.head_r > 0.0 && !fig.segments.is_empty());
    let mut no_footer = sc.clone();
    no_footer.footer.clear();
    assert!(flowmaid::seq::to_svg(&no_footer).starts_with("<svg"));

    d.mirror_actors = false;
    assert!(flowmaid::seq::scene(&d).footer.is_empty());
}

#[test]
fn class_diagram_is_built_with_ensure_class_and_constructors() {
    let src = "classDiagram\n\
               direction LR\n\
               Dog --|> Animal : is\n\
               note for Dog \"barks<br/>loud\"\n\
               note \"free note\"\n";
    let Document::Class(parsed) = parse(src) else {
        panic!("not a class diagram")
    };

    let mut d = ClassDiagram::default();
    d.direction = Direction::LR;
    let dog = d.ensure_class("Dog");
    let animal = d.ensure_class("Animal");
    let mut rel = ClassRel::new(dog, animal, RelKind::Inheritance);
    rel.label = Some("is".to_string());
    d.relations.push(rel);
    // `ClassNote` is `#[non_exhaustive]`: built through its constructor,
    // its text already folded (`<br/>` is a `\n`).
    d.notes.push(ClassNote::new("barks\nloud", Some(dog)));
    d.notes.push(ClassNote::new("free note", None));
    assert_eq!(render::render_class(&d), render::render_class(&parsed));
}

#[test]
fn a_hand_built_mindmap_of_any_depth_renders() {
    // The fan layout recursed a few frames per level: a chain some
    // thousands deep overflowed the stack. It now leaves maps deeper
    // than it takes on to the ring layout. On a spawned thread with the
    // default stack (as a test thread has, and less than a main thread).
    let svg = std::thread::spawn(|| {
        let mut d = Mindmap::default();
        let mut at = d.add_node(None, "root", MindShape::Default);
        for k in 0..20_000 {
            at = d.add_node(Some(at), format!("n{k}"), MindShape::Default);
        }
        assert_eq!(d.nodes[at].depth, 20_000);
        render::render_mindmap(&d)
    })
    .join()
    .expect("renders without overflowing the stack");
    assert!(svg.starts_with("<svg") && svg.contains(">n19999<"));
}
