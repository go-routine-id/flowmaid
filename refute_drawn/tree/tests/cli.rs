//! End-to-end checks for the `flowmaid` binary's command-line flags.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run the CLI with `args`, feeding `stdin`, and return its stdout.
fn run(args: &[&str], stdin: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_flowmaid"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn flowmaid");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "flowmaid {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// The full-canvas white rect `svg_open` writes right after `<title>`.
fn has_canvas_rect(svg: &str) -> bool {
    svg.split("</title>\n").nth(1).is_some_and(|rest| {
        rest.starts_with("<rect width=") && {
            let first = rest.lines().next().unwrap();
            !first.contains(" x=") && first.ends_with("fill=\"#ffffff\"/>")
        }
    })
}

#[test]
fn transparent_flag_drops_the_background_for_every_input_mode() {
    let flow = "flowchart TD\nA --> B --> C --> D";
    let advance = "lane a \"A\" {\n  x[Start]\n  y[End]\n  x --> y\n}";
    let advance_json = r#"{
        "lanes": [{ "id": "dev", "title": "Development" }],
        "nodes": [{ "id": "a", "label": "Design", "lane": "dev" },
                  { "id": "b", "label": "Build", "lane": "dev" }],
        "edges": [{ "from": "a", "to": "b" }]
    }"#;
    let cases: [(&[&str], &str); 6] = [
        (&[], flow),
        (&["--compact", "120"], flow),
        (&[], "erDiagram\nA ||--o{ B : has"),
        (&[], "pie\n\"a\" : 1\n\"b\" : 2"),
        (&["--advance-text"], advance),
        (&["--advance"], advance_json),
    ];
    for (args, src) in cases {
        let base = run(args, src);
        assert!(has_canvas_rect(&base), "{args:?} default: {base}");
        assert_eq!(
            base,
            flowmaid_lib_default(args, src),
            "{args:?}: CLI default must match the library default"
        );
        let with: Vec<&str> = args.iter().copied().chain(["--transparent"]).collect();
        let t = run(&with, src);
        assert!(!has_canvas_rect(&t), "{args:?} transparent: {t}");
        assert!(t.starts_with("<svg"), "{args:?}: still an SVG");
    }
}

/// What the library renders for the same input with default options, so
/// the CLI default is pinned to it (no accidental output drift).
fn flowmaid_lib_default(args: &[&str], src: &str) -> String {
    if args.contains(&"--advance-text") {
        return flowmaid::render_advance_text_svg(src).unwrap();
    }
    if args.contains(&"--advance") {
        return flowmaid::render_advance_svg(src).unwrap();
    }
    if let Some(i) = args.iter().position(|a| *a == "--compact") {
        let px: f64 = args[i + 1].parse().unwrap();
        let g = flowmaid::parser::parse(src).unwrap();
        return flowmaid::fold::render_compact_titled(
            &g,
            &flowmaid::fold::CompactOptions::for_extent(px),
            "Flowchart diagram",
        );
    }
    flowmaid::render_svg(src).unwrap()
}

/// A class diagram of notes alone renders its notes (mermaid does);
/// the CLI used to stop with "empty diagram: no classes defined".
#[test]
fn class_diagram_of_notes_alone_renders() {
    let svg = run(&[], "classDiagram\nnote \"just a note\"");
    assert!(svg.contains(">just a note</text>"), "{svg}");
}

/// `--theme` (report M10) paints every input mode in the theme, beats a
/// theme the source names, and reads the source's own otherwise; an
/// unknown name warns and paints the default theme.
#[test]
fn theme_flag_and_source_themes() {
    use flowmaid::style::{Palette, Theme};
    let dark_canvas = format!("fill=\"{}\"/>", Palette::new(Theme::Dark).background);
    let flow = "flowchart TD\nA --> B --> C";
    let advance = "lane a \"A\" {\n  x[Start]\n  y[End]\n  x --> y\n}";
    for (args, src) in [
        (&[][..], flow),
        (&["--compact", "120"][..], flow),
        (&[][..], "sequenceDiagram\nA->>B: hi"),
        (&[][..], "gitGraph\ncommit\nbranch f\ncommit"),
        (&["--advance-text"][..], advance),
    ] {
        let with: Vec<&str> = args.iter().copied().chain(["--theme", "dark"]).collect();
        let svg = run(&with, src);
        assert!(svg.contains(&dark_canvas), "{args:?} {src}: {svg}");
        // The library's explicit option paints the same.
        if args.is_empty() {
            let mut opts = flowmaid::SvgOptions::default();
            opts.theme = Some(Theme::Dark);
            assert_eq!(svg, flowmaid::render_svg_advanced(src, &opts).unwrap());
        }
    }
    // The source's theme without the flag; the flag beats it.
    let themed = format!("%%{{init: {{\"theme\": \"dark\"}}}}%%\n{flow}");
    assert!(run(&[], &themed).contains(&dark_canvas));
    assert_eq!(run(&["--theme", "flowmaid"], &themed), run(&[], flow));
    assert_eq!(run(&[], &themed), flowmaid::render_svg(&themed).unwrap());

    let out = Command::new(env!("CARGO_BIN_EXE_flowmaid"))
        .args(["--theme", "sparkly"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            c.stdin.take().unwrap().write_all(themed.as_bytes())?;
            c.wait_with_output()
        })
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("unknown theme 'sparkly'") && err.contains("neutral"),
        "{err}"
    );
    assert_eq!(String::from_utf8(out.stdout).unwrap(), run(&[], flow));
}
