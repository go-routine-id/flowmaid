//! Shared color theme, exposed so hosts (GUI painters) can match
//! the SVG writers exactly.
//!
//! Flowchart nodes are colored by shape semantics — the shape
//! already encodes meaning in Mermaid (stadium = start/end,
//! diamond = decision, ...), so color reinforces it for free.
//! ER entities cycle through an accent palette by index, which is
//! stable across renders because entity order is parse order.
//!
//! Text drawn on an accent fill picks its colour with [`text_on`]
//! (WCAG relative luminance), so header bands, wedges and branches stay
//! readable (≥ 4.5:1) whatever accent they land on.

use crate::model::Shape;

/// Fill + stroke pair, as `#rrggbb` hex strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeStyle {
    pub fill: &'static str,
    pub stroke: &'static str,
}

/// Semantic color per flowchart shape: pastel fill, saturated
/// stroke, all tuned to keep `#232840` text readable.
pub fn shape_style(shape: Shape) -> ShapeStyle {
    match shape {
        // Process step — the neutral workhorse, keeps the old indigo.
        Shape::Rect => ShapeStyle {
            fill: "#eef1fb",
            stroke: "#5b6dc0",
        },
        // Rounded step — teal, a softer sibling of Rect.
        Shape::Rounded => ShapeStyle {
            fill: "#e4f5f4",
            stroke: "#199a8e",
        },
        // Stadium (start / end) — green, "go".
        Shape::Stadium => ShapeStyle {
            fill: "#e5f5ea",
            stroke: "#33a35c",
        },
        // Diamond (decision) — amber, "attention".
        Shape::Diamond => ShapeStyle {
            fill: "#fcf2da",
            stroke: "#d99114",
        },
        // Circle (terminal / connector) — violet.
        Shape::Circle => ShapeStyle {
            fill: "#f2ecfa",
            stroke: "#8a5cd6",
        },
        // Double circle (terminal) — green like start/stop.
        Shape::DoubleCircle => ShapeStyle {
            fill: "#e5f5ea",
            stroke: "#33a35c",
        },
        // Cylinder (database / storage) — steel blue.
        Shape::Cylinder => ShapeStyle {
            fill: "#e7eef7",
            stroke: "#4d8fbf",
        },
        // Subroutine — indigo sibling of Rect.
        Shape::Subroutine => ShapeStyle {
            fill: "#eef1fb",
            stroke: "#5b6dc0",
        },
        // Hexagon (preparation) — violet.
        Shape::Hexagon => ShapeStyle {
            fill: "#f2ecfa",
            stroke: "#8a5cd6",
        },
        // Parallelograms (I/O) — amber.
        Shape::Parallelogram | Shape::ParallelogramAlt => ShapeStyle {
            fill: "#fcf2da",
            stroke: "#d99114",
        },
        // stateDiagram pseudostates (start/end dots, fork bar) —
        // solid dark, mermaid-style, not part of the shape theme.
        Shape::StateStart | Shape::StateEnd | Shape::ForkBar => ShapeStyle {
            fill: "#44507a",
            stroke: "#44507a",
        },
    }
}

/// Accent palette for ER entity headers (and anything else that
/// wants a stable per-item color). Mid-luminance by design: text on
/// these fills takes its colour from [`text_on`], never a fixed white.
pub const ACCENTS: &[&str] = &[
    "#5b6dc0", // indigo
    "#33a35c", // green
    "#d99114", // amber
    "#c2588c", // rose
    "#199a8e", // teal
    "#8a5cd6", // violet
    "#d96459", // coral
    "#4d8fbf", // steel blue
];

/// Stable accent for item `index` (wraps around the palette).
pub fn accent(index: usize) -> &'static str {
    ACCENTS[index % ACCENTS.len()]
}

/// [`ACCENTS`] darkened just enough (every sRGB channel scaled by the
/// same factor, so the hue holds) to reach [`MIN_TEXT_CONTRAST`]
/// against white — the colour for text drawn IN an item's accent on
/// the white canvas (gitgraph lane names and tags, sequence actor
/// labels). Entries that already pass (indigo, violet) are the accent
/// itself, so those keep their exact look. Index-parallel with
/// [`ACCENTS`].
pub const ACCENT_INKS: &[&str] = &[
    "#5b6dc0", // indigo (4.8:1 as is)
    "#2a864c", // green
    "#a16b0f", // amber
    "#b85485", // rose
    "#15847a", // teal
    "#8a5cd6", // violet (4.6:1 as is)
    "#be574e", // coral
    "#437ca5", // steel blue
];

/// [`ACCENT_INKS`] entry for item `index` — `accent(index)` made
/// readable as text on white (wraps like [`accent`]).
pub fn accent_ink(index: usize) -> &'static str {
    ACCENT_INKS[index % ACCENT_INKS.len()]
}

/// Second cycle of the accent palette: every [`ACCENTS`] entry with
/// its sRGB channels scaled by 0.65 (35% toward black). Deep enough to
/// stay distinct from the first cycle and from each other (CIE76
/// ΔE ≥ 20 across all sixteen), and white text clears 5.6:1 on each.
/// Used through [`accent_wide`]; index-parallel with [`ACCENTS`].
pub const ACCENTS_DEEP: &[&str] = &[
    "#3b477d", // deep indigo
    "#216a3c", // deep green
    "#8d5e0d", // deep amber
    "#7e395b", // deep rose
    "#10645c", // deep teal
    "#5a3c8b", // deep violet
    "#8d413a", // deep coral
    "#325d7c", // deep steel blue
];

/// Accent for item `index` from a sixteen-colour cycle — [`ACCENTS`],
/// then [`ACCENTS_DEEP`] — for renderers that can show more than
/// `ACCENTS.len()` items side by side (pie slices, sankey nodes,
/// journey actors), where [`accent`]'s wrap would give item 8 the
/// colour of item 0. Identical to [`accent`] for the first eight, so
/// switching a renderer over changes nothing for small diagrams. Pair
/// it with [`text_on`] for any text drawn on the fill.
pub fn accent_wide(index: usize) -> &'static str {
    let k = index % (ACCENTS.len() + ACCENTS_DEEP.len());
    if k < ACCENTS.len() {
        ACCENTS[k]
    } else {
        ACCENTS_DEEP[k - ACCENTS.len()]
    }
}

/// Note card colours (sequence notes and the like): a pale yellow card
/// with an olive-gold border, mermaid's note idiom. Deliberately
/// outside [`ACCENTS`] / [`ACCENTS_DEEP`] (border ΔE ≥ 20 from every
/// one of them) and away from the amber decision fill, so a note never
/// reads as a participant, a branch or a decision; the border keeps
/// 3:1 against the card and `#232840` text 13:1.
pub const NOTE_STYLE: ShapeStyle = ShapeStyle {
    fill: "#fff8c5",
    stroke: "#9c8a2c",
};

/// WCAG 2.x AA minimum contrast ratio for normal-size text.
pub const MIN_TEXT_CONTRAST: f64 = 4.5;

/// The dark choice of [`text_on`]: a navy near-black from the diagram
/// ink's family, deep enough that every [`ACCENTS`] entry white fails
/// on still reaches [`MIN_TEXT_CONTRAST`] — rose, the closest call,
/// gets 4.6:1 (a `#111827`-style ink only reaches 4.3:1 there).
pub const DARK_INK: &str = "#0a0e1a";

/// WCAG 2.x relative luminance (0 = black, 1 = white) of a `#rrggbb`
/// or `#rgb` colour; `None` for any other syntax (named colours,
/// `rgb()`, ...).
pub fn relative_luminance(color: &str) -> Option<f64> {
    let hex = color.strip_prefix('#')?;
    let d: Vec<u32> = hex.chars().map(|c| c.to_digit(16)).collect::<Option<_>>()?;
    let (r, g, b) = match d.len() {
        3 => (d[0] * 17, d[1] * 17, d[2] * 17),
        6 => (d[0] * 16 + d[1], d[2] * 16 + d[3], d[4] * 16 + d[5]),
        _ => return None,
    };
    // sRGB transfer curve → linear light.
    let lin = |c: u32| {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    Some(0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b))
}

/// WCAG 2.x contrast ratio (1 to 21) between two colours; `None` when
/// either is not `#rrggbb` / `#rgb`.
pub fn contrast_ratio(a: &str, b: &str) -> Option<f64> {
    let (la, lb) = (relative_luminance(a)?, relative_luminance(b)?);
    Some((la.max(lb) + 0.05) / (la.min(lb) + 0.05))
}

/// The text colour to draw ON a solid `fill` (accent header bands,
/// pie wedges, mindmap branches, ...): white wherever it reaches
/// [`MIN_TEXT_CONTRAST`], so fills that already read well keep their
/// look; otherwise [`DARK_INK`]; otherwise pure black, which always
/// clears 4.5:1 where white does not. A fill this cannot parse keeps
/// white, the historical default.
pub fn text_on(fill: &str) -> &'static str {
    const WHITE: &str = "#ffffff";
    let reads = |ink| contrast_ratio(fill, ink).map_or(true, |c| c >= MIN_TEXT_CONTRAST);
    if reads(WHITE) {
        WHITE
    } else if reads(DARK_INK) {
        DARK_INK
    } else {
        "#000000"
    }
}

/// Test helper for the renderers' contrast regressions: asserts the
/// first `<text>` whose content (or a `<tspan>` line of it) is exactly
/// `label` is painted in an ink reaching [`MIN_TEXT_CONTRAST`] on
/// `bg`, and returns that ink.
#[cfg(test)]
pub(crate) fn assert_text_reads<'a>(svg: &'a str, label: &str, bg: &str) -> &'a str {
    let end = [format!(">{label}</text>"), format!(">{label}</tspan>")]
        .iter()
        .filter_map(|pat| svg.find(pat.as_str()))
        .min()
        .unwrap_or_else(|| panic!("no <text> reading {label:?} in\n{svg}"));
    let open = svg[..end].rfind("<text").expect("enclosing <text>");
    let tag = &svg[open..end];
    let f = tag.find(" fill=\"").expect("<text> has a fill") + 7;
    let ink = &tag[f..f + tag[f..].find('"').unwrap()];
    let c = contrast_ratio(ink, bg).expect("hex colours");
    assert!(
        c >= MIN_TEXT_CONTRAST,
        "{label:?}: {ink} on {bg} is only {c:.2}:1"
    );
    ink
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_has_a_distinct_stroke() {
        let shapes = [
            Shape::Rect,
            Shape::Rounded,
            Shape::Stadium,
            Shape::Diamond,
            Shape::Circle,
        ];
        for (i, a) in shapes.iter().enumerate() {
            for b in &shapes[i + 1..] {
                assert_ne!(shape_style(*a).stroke, shape_style(*b).stroke);
            }
        }
    }

    #[test]
    fn accent_wraps_stably() {
        assert_eq!(accent(0), accent(ACCENTS.len()));
        assert_ne!(accent(0), accent(1));
    }

    fn rgb(c: &str) -> [f64; 3] {
        let h = c.strip_prefix('#').unwrap();
        let ch = |i: usize| f64::from(u8::from_str_radix(&h[i..i + 2], 16).unwrap());
        [ch(0), ch(2), ch(4)]
    }

    /// CIE76 ΔE between two `#rrggbb` colours (D65), for the
    /// distinctness checks below.
    fn delta_e(a: &str, b: &str) -> f64 {
        let lab = |c: &str| {
            let [r, g, b] = rgb(c).map(|v| {
                let v = v / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            });
            let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
            let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
            let f = |t: f64| {
                if t > 216.0 / 24389.0 {
                    t.cbrt()
                } else {
                    (24389.0 / 27.0 * t + 16.0) / 116.0
                }
            };
            let (fx, fy, fz) = (f(x), f(y), f(z));
            [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
        };
        let (p, q) = (lab(a), lab(b));
        ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
    }

    /// `c` is `base` with every channel scaled by one factor `k <= 1`
    /// (within the ±1 of hex rounding) — same hue, only darker.
    fn is_shade_of(c: &str, base: &str) -> bool {
        let (c, b) = (rgb(c), rgb(base));
        let k = c[1] / b[1];
        k <= 1.0 + 1e-9 && (0..3).all(|i| (c[i] - b[i] * k).abs() <= 1.5)
    }

    #[test]
    fn luminance_matches_wcag_reference_values() {
        let near = |a: Option<f64>, b: f64| (a.unwrap() - b).abs() < 1e-3;
        assert!(near(relative_luminance("#ffffff"), 1.0));
        assert!(near(relative_luminance("#000000"), 0.0));
        assert!(near(relative_luminance("#fff"), 1.0), "#rgb shorthand");
        assert!(
            near(relative_luminance("#FF0000"), 0.2126),
            "case-insensitive"
        );
        assert!(near(contrast_ratio("#000", "#fff"), 21.0));
        assert!(near(contrast_ratio("#777777", "#ffffff"), 4.478));
        // Symmetric.
        assert_eq!(
            contrast_ratio("#5b6dc0", "#ffffff"),
            contrast_ratio("#ffffff", "#5b6dc0")
        );
        for bad in ["red", "#12345", "#gggggg", "5b6dc0", "", "#ééé"] {
            assert_eq!(relative_luminance(bad), None, "{bad}");
        }
    }

    /// Report Q5 / §2.11: white text hard-coded on every accent failed
    /// WCAG AA on six of the eight (amber 2.6:1, green 3.2:1, ...).
    #[test]
    fn every_accent_gets_text_reaching_aa() {
        for &a in ACCENTS.iter().chain(ACCENTS_DEEP) {
            let ink = text_on(a);
            let c = contrast_ratio(a, ink).unwrap();
            assert!(c >= MIN_TEXT_CONTRAST, "{ink} on {a}: {c:.2}:1");
            // Always one of the two designed inks, never the black
            // last resort, so the palette reads as one system.
            assert!(ink == "#ffffff" || ink == DARK_INK, "{a} → {ink}");
        }
    }

    #[test]
    fn text_on_keeps_white_where_it_already_passes() {
        // Indigo (4.8:1) and violet (4.6:1) keep their white headers.
        assert_eq!(text_on("#5b6dc0"), "#ffffff");
        assert_eq!(text_on("#8a5cd6"), "#ffffff");
        // Amber (2.6:1 with white) flips to the dark ink.
        assert_eq!(text_on("#d99114"), DARK_INK);
        assert_eq!(text_on("#ffffff"), DARK_INK);
        // Unparseable fills keep the historical white.
        assert_eq!(text_on("tomato"), "#ffffff");
    }

    #[test]
    fn text_on_reaches_aa_on_any_fill() {
        // Sweep the whole grey ramp plus saturated primaries: whatever
        // the fill, the chosen ink clears 4.5:1 — including the band
        // (luminance ~0.18-0.22) where neither white nor DARK_INK does
        // and pure black takes over.
        let mut fills: Vec<String> = (0..=255)
            .map(|v| format!("#{v:02x}{v:02x}{v:02x}"))
            .collect();
        for c in [
            "#ff0000", "#00ff00", "#0000ff", "#ffff00", "#00ffff", "#ff00ff",
        ] {
            fills.push(c.to_string());
        }
        for f in &fills {
            let c = contrast_ratio(f, text_on(f)).unwrap();
            assert!(c >= MIN_TEXT_CONTRAST, "{f}: {c:.2}:1");
        }
    }

    #[test]
    fn accent_inks_read_on_white_and_keep_their_hue() {
        assert_eq!(ACCENT_INKS.len(), ACCENTS.len());
        for (i, (&ink, &a)) in ACCENT_INKS.iter().zip(ACCENTS).enumerate() {
            let c = contrast_ratio(ink, "#ffffff").unwrap();
            assert!(c >= MIN_TEXT_CONTRAST, "ink {i} {ink}: {c:.2}:1");
            assert!(is_shade_of(ink, a), "ink {ink} is not a shade of {a}");
            // Accents that already pass are not touched at all.
            if contrast_ratio(a, "#ffffff").unwrap() >= MIN_TEXT_CONTRAST {
                assert_eq!(ink, a);
            }
            assert_eq!(accent_ink(i), ink);
            assert_eq!(accent_ink(i + ACCENTS.len()), ink, "wraps like accent()");
        }
    }

    #[test]
    fn wide_accent_cycle_is_distinct_and_extends_accent() {
        assert_eq!(ACCENTS_DEEP.len(), ACCENTS.len());
        let n = ACCENTS.len();
        // The first cycle is exactly `accent`, so adopting it changes
        // nothing for diagrams with eight items or fewer.
        for i in 0..n {
            assert_eq!(accent_wide(i), accent(i));
            assert!(is_shade_of(accent_wide(i + n), accent(i)));
        }
        assert_eq!(accent_wide(0), accent_wide(2 * n), "wraps after 16");
        // Sixteen genuinely different colours: item 8 no longer
        // repeats item 0 (pie slice 9, journey actor 9, sankey).
        let all: Vec<&str> = (0..2 * n).map(accent_wide).collect();
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                let d = delta_e(a, b);
                assert!(d >= 20.0, "{a} vs {b}: ΔE {d:.1}");
            }
        }
    }

    #[test]
    fn note_style_stands_apart_from_the_accents() {
        let ShapeStyle { fill, stroke } = NOTE_STYLE;
        for &a in ACCENTS.iter().chain(ACCENTS_DEEP) {
            assert!(delta_e(stroke, a) >= 20.0, "note border vs {a}");
        }
        // Not the amber decision / attention fill either.
        let diamond = shape_style(Shape::Diamond);
        assert!(delta_e(fill, diamond.fill) >= 10.0);
        assert!(delta_e(stroke, diamond.stroke) >= 20.0);
        assert!(
            contrast_ratio(stroke, fill).unwrap() >= 3.0,
            "border visible"
        );
        let text = contrast_ratio(crate::scene::TEXT_COLOR, fill).unwrap();
        assert!(text >= MIN_TEXT_CONTRAST);
    }
}
