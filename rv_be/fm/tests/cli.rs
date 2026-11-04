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
