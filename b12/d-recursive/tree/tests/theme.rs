//! Colour themes (report M10): every theme renders every example,
//! deterministically and readably; the default theme is the historical
//! output byte for byte; a source's `%%{init}%%` / frontmatter theme
//! applies unless an explicit option beats it.

use flowmaid::style::{contrast_ratio, Palette, Theme, MIN_TEXT_CONTRAST};
use flowmaid::{render_svg, render_svg_advanced, SvgOptions};

/// Every mermaid example in `examples/` (the advance DSL files are not
/// mermaid), as (name, source).
fn examples() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| {
            let path = e.unwrap().path();
            let name = path.file_name()?.to_str()?.to_string();
            (name.ends_with(".mmd") && !name.starts_with("advance"))
                .then(|| (name, std::fs::read_to_string(&path).unwrap()))
        })
        .collect();
    out.sort();
    assert!(out.len() >= 15, "examples missing: {out:?}");
    out
}

fn themed(theme: Theme) -> SvgOptions {
    let mut opts = SvgOptions::default();
    opts.theme = Some(theme);
    opts
}

fn render(src: &str, theme: Theme) -> String {
    render_svg_advanced(src, &themed(theme)).unwrap()
}

#[test]
fn every_theme_renders_every_example_deterministically() {
    for (name, src) in examples() {
        let default = render_svg(&src).unwrap();
        for theme in Theme::ALL {
            let a = render(&src, theme);
            assert_eq!(
                a,
                render(&src, theme),
                "{name} {theme:?}: not deterministic"
            );
            assert!(a.starts_with("<svg") && a.ends_with("</svg>\n"), "{name}");
            if theme == Theme::Flowmaid {
                // The explicit default is the default, byte for byte.
                assert_eq!(a, default, "{name}: explicit Flowmaid differs");
            } else {
                assert_ne!(a, default, "{name} {theme:?}: theme had no effect");
                let canvas = &Palette::new(theme).background;
                assert!(
                    a.contains(&format!("fill=\"{canvas}\"/>")),
                    "{name} {theme:?}"
                );
            }
        }
    }
}

/// One element of an SVG, as far as the readability checks need it.
struct Tag<'a> {
    name: &'a str,
    text: &'a str,
    /// A `<text>`'s glyphs (its tspans' run together), else empty.
    content: String,
}

impl<'a> Tag<'a> {
    /// An attribute's value, in either quote style.
    fn attr(&self, key: &str) -> Option<&'a str> {
        for q in ['"', '\''] {
            let pat = format!(" {key}={q}");
            if let Some(i) = self.text.find(&pat) {
                let rest = &self.text[i + pat.len()..];
                return rest.find(q).map(|j| &rest[..j]);
            }
        }
        None
    }
    fn num(&self, key: &str) -> f64 {
        self.attr(key).and_then(|v| v.parse().ok()).unwrap_or(0.0)
    }
    /// A solid hex fill, if the element has one.
    fn fill(&self) -> Option<&'a str> {
        let f = self.attr("fill")?;
        let opaque = self
            .attr("fill-opacity")
            .and_then(|o| o.parse::<f64>().ok())
            .map_or(true, |o| o >= 0.5);
        (f.starts_with('#') && opaque).then_some(f)
    }
}

/// The opening tags of an SVG in paint order, `<defs>` (markers, masks)
/// left out.
fn tags(svg: &str) -> Vec<Tag<'_>> {
    let mut out = Vec::new();
    let mut in_defs = false;
    let mut rest = svg;
    while let Some(i) = rest.find('<') {
        let after = &rest[i + 1..];
        let end = after.find('>').unwrap_or(after.len());
        let text = &after[..end];
        rest = &after[end..];
        let closing = text.starts_with('/');
        let name = text
            .trim_start_matches('/')
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or("");
        match (closing, name) {
            (false, "defs") => in_defs = !text.ends_with('/'),
            (true, "defs") => in_defs = false,
            (false, _) if !in_defs => {
                let content = if name == "text" {
                    let body = &rest[1..rest.find("</text>").unwrap_or(rest.len())];
                    let mut plain = String::new();
                    let mut in_tag = false;
                    for c in body.chars() {
                        match c {
                            '<' => in_tag = true,
                            '>' => in_tag = false,
                            _ if !in_tag => plain.push(c),
                            _ => {}
                        }
                    }
                    plain.trim().to_string()
                } else {
                    String::new()
                };
                out.push(Tag {
                    name,
                    text,
                    content,
                })
            }
            _ => {}
        }
    }
    out
}

/// Numbers of a `points` / path `d` attribute, paired up.
fn pairs(v: &str) -> Vec<(f64, f64)> {
    let nums: Vec<f64> = v
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter_map(|t| t.parse().ok())
        .collect();
    nums.chunks_exact(2).map(|c| (c[0], c[1])).collect()
}

/// What is painted under the point `(x, y)` just before tag `k`: the
/// fill of the topmost rect or circle containing it, else `canvas`;
/// `None` when the topmost filled shape there is a path or polygon,
/// whose outline this does not trace (a diamond, a header band, a pie
/// wedge).
fn backdrop<'a>(all: &[Tag<'a>], k: usize, (x, y): (f64, f64), canvas: &'a str) -> Option<&'a str> {
    for s in all[..k].iter().rev() {
        let Some(fill) = s.fill() else { continue };
        let (inside, exact) = match s.name {
            "rect" => {
                let (rx, ry) = (s.num("x"), s.num("y"));
                let (w, h) = (s.num("width"), s.num("height"));
                (x >= rx && x <= rx + w && y >= ry && y <= ry + h, true)
            }
            "circle" => {
                let (cx, cy, r) = (s.num("cx"), s.num("cy"), s.num("r"));
                ((x - cx).hypot(y - cy) <= r, true)
            }
            "path" | "polygon" => {
                let pts = pairs(s.attr("d").or(s.attr("points")).unwrap_or(""));
                let inside = !pts.is_empty() && {
                    let (x0, x1) = pts
                        .iter()
                        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
                    let (y0, y1) = pts
                        .iter()
                        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
                    x >= x0 && x <= x1 && y >= y0 && y <= y1
                };
                (inside, false)
            }
            _ => (false, false),
        };
        if inside {
            return exact.then_some(fill);
        }
    }
    Some(canvas)
}

/// Report M10: in every theme each text reads (4.5:1) on what it is
/// painted over ([`backdrop`]). Text over a path or polygon is left to
/// the writers' own contrast tests, as is text in a colour the author
/// wrote in the source.
#[test]
fn themed_text_reads_on_what_it_is_painted_over() {
    for (name, src) in examples() {
        let lower = src.to_ascii_lowercase();
        for theme in Theme::ALL.into_iter().filter(|&t| t != Theme::Flowmaid) {
            let svg = render(&src, theme);
            let canvas = Palette::new(theme).background;
            let all = tags(&svg);
            let mut checked = 0;
            for (k, t) in all.iter().enumerate() {
                if t.name != "text" || t.attr("aria-hidden").is_some() || t.content.is_empty() {
                    continue;
                }
                let Some(ink) = t.fill() else { continue };
                if lower.contains(&ink.to_ascii_lowercase()) {
                    continue; // the author's own colour
                }
                let at = (t.num("x"), t.num("y"));
                let Some(bg) = backdrop(&all, k, at, &canvas) else {
                    continue;
                };
                if let Some(c) = contrast_ratio(ink, bg) {
                    assert!(
                        c >= MIN_TEXT_CONTRAST,
                        "{name} {theme:?}: text {ink} on {bg} is {c:.2}:1 in\n<{}>{}",
                        t.text,
                        t.content
                    );
                    checked += 1;
                }
            }
            assert!(checked > 0, "{name} {theme:?}: no text checked");
        }
    }
}

/// Report M10: every line a theme draws — edges, guides, relationship
/// lines, lanes, axes — keeps 3:1 on what it starts over (the canvas,
/// mostly); strokes in the canvas colour (a pie's cuts, a halo) and the
/// author's own colours aside.
#[test]
fn themed_lines_keep_three_to_one_on_what_they_cross() {
    for (name, src) in examples() {
        let lower = src.to_ascii_lowercase();
        for theme in [Theme::Mermaid, Theme::Neutral, Theme::Dark, Theme::Forest] {
            let svg = render(&src, theme);
            let pal = Palette::new(theme);
            let all = tags(&svg);
            for (k, t) in all.iter().enumerate() {
                if !matches!(t.name, "path" | "line" | "polyline") {
                    continue;
                }
                if t.attr("fill").is_some_and(|f| f != "none") {
                    continue; // a shape, not a line
                }
                let Some(stroke) = t.attr("stroke").filter(|s| s.starts_with('#')) else {
                    continue;
                };
                // The canvas colour cuts (pie seams, halos); the label
                // chip's hairline token also rules ER rows, faint by
                // design as in the default theme.
                if stroke == pal.background
                    || stroke == pal.label_border
                    || lower.contains(&stroke.to_ascii_lowercase())
                {
                    continue;
                }
                let start = match t.name {
                    "line" => (t.num("x1"), t.num("y1")),
                    _ => pairs(t.attr("d").or(t.attr("points")).unwrap_or(""))
                        .first()
                        .copied()
                        .unwrap_or_default(),
                };
                let Some(bg) = backdrop(&all, k, start, &pal.background) else {
                    continue;
                };
                let c = contrast_ratio(stroke, bg).unwrap();
                assert!(
                    c >= 3.0 || (theme != Theme::Dark && is_flowmaid_accent(stroke)),
                    "{name} {theme:?}: line {stroke} on {bg} is {c:.2}:1"
                );
            }
        }
    }
}

/// The categorical accents keep their colours in every theme; on a white
/// canvas a couple sit under 3:1 as they always have (the default theme's
/// palette, unchanged), so only the dark canvas, picked for them, is held
/// to 3:1 for them.
fn is_flowmaid_accent(c: &str) -> bool {
    flowmaid::style::ACCENTS.contains(&c) || flowmaid::style::ACCENTS_DEEP.contains(&c)
}

#[test]
fn a_source_theme_applies_unless_an_option_beats_it() {
    let body = "flowchart TD\nA[Start] --> B{Ok?}\n";
    let dark_src = format!("%%{{init: {{\"theme\": \"dark\"}}}}%%\n{body}");
    let dark = render_svg(&dark_src).unwrap();
    assert!(dark.contains("fill=\"#1e1f22\"/>"), "{dark}");
    assert_eq!(dark, render(body, Theme::Dark), "directive = option");
    // An explicit option wins, the default theme included.
    assert_eq!(
        render(&dark_src, Theme::Neutral),
        render(body, Theme::Neutral)
    );
    assert_eq!(
        render(&dark_src, Theme::Flowmaid),
        render_svg(body).unwrap()
    );
    // Frontmatter, and an unknown name (the default theme, no error).
    let fm = format!("---\nconfig:\n  theme: forest\n---\n{body}");
    assert_eq!(render_svg(&fm).unwrap(), render(body, Theme::Forest));
    let unknown = format!("%%{{init: {{'theme': 'sparkly'}}}}%%\n{body}");
    assert_eq!(render_svg(&unknown).unwrap(), render_svg(body).unwrap());
    // themeVariables recolour the shared roles; an option drops them.
    let vars = format!(
        "%%{{init: {{'theme': 'base', 'themeVariables': {{'primaryColor': '#BB2528', \
         'primaryTextColor': '#fff', 'lineColor': '#F8B229'}}}}}}%%\n{body}"
    );
    let svg = render_svg(&vars).unwrap();
    assert!(svg.contains("fill=\"#bb2528\" stroke=\"#9370db\""), "{svg}");
    assert!(svg.contains("stroke=\"#f8b229\""), "{svg}");
    assert!(svg.contains("fill=\"#ffffff\">Start</text>"), "{svg}");
    assert_eq!(render(&vars, Theme::Mermaid), render(body, Theme::Mermaid));
    // `with_source_theme` is what the CLI and hosts call.
    let opts = SvgOptions::default().with_source_theme(&dark_src);
    assert_eq!(opts.palette(), Palette::new(Theme::Dark));
    assert_eq!(
        themed(Theme::Neutral)
            .with_source_theme(&dark_src)
            .palette(),
        Palette::new(Theme::Neutral)
    );
}

#[test]
fn every_diagram_type_honours_the_source_theme() {
    for (name, src) in examples() {
        // At the end: a frontmatter block must stay the first thing.
        let directive = format!("{src}\n%%{{init: {{\"theme\": \"dark\"}}}}%%\n");
        assert_eq!(
            render_svg(&directive).unwrap(),
            render(&src, Theme::Dark),
            "{name}: directive and option differ"
        );
    }
}

#[test]
fn style_and_link_style_colours_beat_the_theme() {
    let src = "flowchart LR\nA --> B\nstyle A fill:#fde68a,stroke:#b45309,color:#111111\n\
               linkStyle 0 stroke:#16a34a,color:#2563eb\nA -- yes --> C\n";
    for theme in Theme::ALL {
        let svg = render(src, theme);
        assert!(
            svg.contains("fill=\"#fde68a\" stroke=\"#b45309\""),
            "{theme:?}"
        );
        assert!(svg.contains("fill=\"#111111\">A</text>"), "{theme:?}");
        assert!(svg.contains("stroke=\"#16a34a\""), "{theme:?}");
    }
    // A fill without a `color` keeps its text readable under a theme
    // whose ink is light.
    let svg = render("flowchart LR\nA --> B\nstyle A fill:#fef3c7\n", Theme::Dark);
    let i = svg.find(">A</text>").unwrap();
    let tag = &svg[svg[..i].rfind("<text").unwrap()..i];
    assert!(
        tag.contains(&format!("fill=\"{}\"", flowmaid::style::DARK_INK)),
        "{tag}"
    );
}

/// Marker ids are page-global: two themes of one diagram inlined on a
/// page must not share a head of the wrong colour.
#[test]
fn themed_marker_ids_never_collide_with_the_default_ones() {
    let src = "flowchart LR\nA --> B\nC <--> D\nE --x F\n";
    let ids = |svg: &str| -> Vec<String> {
        svg.match_indices("<marker id=\"")
            .map(|(i, m)| {
                let rest = &svg[i + m.len()..];
                rest[..rest.find('"').unwrap()].to_string()
            })
            .collect()
    };
    let default = ids(&render_svg(src).unwrap());
    assert!(default.contains(&"arrow".to_string()));
    for theme in [Theme::Mermaid, Theme::Neutral, Theme::Dark, Theme::Forest] {
        let svg = render(src, theme);
        let themed = ids(&svg);
        assert_eq!(themed.len(), default.len());
        // Neutral and Mermaid share an ink, and so their heads.
        assert!(
            themed.iter().all(|id| !default.contains(id)),
            "{theme:?}: {themed:?}"
        );
        for id in &themed {
            assert!(
                svg.contains(&format!("url(#{id})")),
                "{theme:?}: {id} unused"
            );
        }
    }
    let arch = "architecture-beta\nservice a(server)[A]\nservice b(server)[B]\na:R --> L:b\n";
    let svg = render(arch, Theme::Dark);
    assert!(!svg.contains("id='flowmaid-arch-arrow-end'"), "{svg}");
}

#[test]
fn advance_diagrams_take_an_explicit_theme() {
    let text = "lane a \"A\" {\n  x[Start]\n  y[End]\n  x --> y\n}";
    let base = flowmaid::render_advance_text_svg_with(text, &SvgOptions::default()).unwrap();
    let dark = flowmaid::render_advance_text_svg_with(text, &themed(Theme::Dark)).unwrap();
    let pal = Palette::new(Theme::Dark);
    let (lane_fill, lane_stroke) = pal.cluster_style(0);
    assert!(
        dark.contains(&format!("fill=\"{lane_fill}\" stroke=\"{lane_stroke}\"")),
        "{dark}"
    );
    assert!(dark.contains(&format!("fill=\"{}\"/>", pal.background)));
    assert_eq!(
        flowmaid::render_advance_text_svg_with(text, &themed(Theme::Flowmaid)).unwrap(),
        base
    );
    // A colour the diagram sets itself is kept.
    let json = r##"{"style": {"lane_fill": "#112233"},
        "lanes": [{"id": "dev", "title": "Dev"}],
        "nodes": [{"id": "a", "label": "A", "lane": "dev"}, {"id": "b", "label": "B", "lane": "dev"}],
        "edges": [{"from": "a", "to": "b"}]}"##;
    let svg = flowmaid::render_advance_svg_with(json, &themed(Theme::Dark)).unwrap();
    assert!(svg.contains("fill=\"#112233\""), "{svg}");
    assert!(svg.contains(&format!("stroke=\"{}\"", pal.line)), "{svg}");
}

/// Charts keep their categorical colours in every theme while their
/// text, lines and canvas follow it; class, ER and sequence items take
/// the theme's one node paint, as mermaid paints them.
#[test]
fn categories_stay_and_items_take_the_node_paint() {
    use flowmaid::style::{accent, accent_ink};
    let dark = Palette::new(Theme::Dark);
    // Pie: the first slice keeps its accent; the seams are the canvas.
    let pie = render("pie\n\"a\" : 3\n\"b\" : 2\n\"c\" : 1", Theme::Dark);
    assert!(pie.contains(&format!("fill=\"{}\"", accent(0))), "{pie}");
    assert!(
        pie.contains(&format!("stroke=\"{}\" stroke-width", dark.background)),
        "{pie}"
    );
    // Git graph: lanes keep their accent; a tag's lane ink is lifted to
    // read on the dark canvas.
    let git = render(
        "gitGraph\ncommit tag: \"v1\"\nbranch f\ncommit",
        Theme::Dark,
    );
    assert!(git.contains(&format!("stroke=\"{}\"", accent(0))), "{git}");
    let ink = dark.accent_ink(0);
    assert_ne!(
        ink,
        accent_ink(0),
        "white's ink would vanish on the dark canvas"
    );
    assert!(git.contains(&format!("fill=\"{ink}\">v1</text>")), "{git}");
    // Mindmap: branches keep their accents; the root takes the theme's
    // strong node border, its label by contrast.
    let mm = render("mindmap\n  root((Root))\n    A\n    B", Theme::Dark);
    let border = dark.node_style(flowmaid::model::Shape::Rect).1;
    assert!(mm.contains(&format!("fill=\"{border}\"")), "{mm}");
    assert!(!mm.contains(flowmaid::mindmap::ROOT_FILL), "{mm}");
    // Timeline: the unnamed section's ticks follow the theme's axis.
    let tl = render("timeline\n  2020 : a\n  2021 : b", Theme::Neutral);
    assert!(!tl.contains(flowmaid::timeline::AXIS_COLOR), "{tl}");
    // Class / ER / sequence: one paint for every item.
    let m = Palette::new(Theme::Mermaid);
    let (fill, stroke) = m.node_style(flowmaid::model::Shape::Rect);
    for src in [
        "classDiagram\nA <|-- B\nB <|-- C",
        "erDiagram\nA ||--o{ B : has\nB ||--o{ C : has",
        "sequenceDiagram\nA->>B: hi\nB->>C: ho",
    ] {
        let svg = render(src, Theme::Mermaid);
        for i in 0..3 {
            assert!(
                !svg.contains(&format!("\"{}\"", accent(i))),
                "{src}: accent {i} left"
            );
        }
        assert!(svg.contains(&format!("fill=\"{fill}\"")), "{src}");
        assert!(svg.contains(&format!("stroke=\"{stroke}\"")), "{src}");
    }
}
