// ------------------------------------------------------------------
// Themes
// ------------------------------------------------------------------

/// A colour theme for the SVG writers (report M10).
///
/// [`Theme::Flowmaid`] is the default and paints exactly what flowmaid
/// always has: nodes coloured by shape semantics, class / ER /
/// sequence items in a stable accent each. The others are options for
/// a mermaid-like look, every node alike: pick one with
/// [`crate::SvgOptions::theme`], `flowmaid --theme <name>`, or from the
/// diagram source (`%%{init: {"theme": "dark"}}%%`, or `theme:` under
/// a YAML frontmatter's `config:`).
///
/// Charts keep their categorical colours in every theme (pie slices,
/// git branches, journey actors, timeline sections, sankey nodes,
/// mindmap branches), as mermaid's do; their text, lines and canvas
/// follow the theme. `style`, `classDef` and `linkStyle` colours always
/// win over the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Theme {
    /// flowmaid's own semantic palette (the default).
    #[default]
    Flowmaid,
    /// mermaid's `default` theme: lavender `#ececff` nodes with a
    /// `#9370db` border, `#333333` lines and text, pale yellow frames.
    Mermaid,
    /// White nodes and one dark ink for borders, lines and text; grey
    /// frames (mermaid's `neutral`, with the white nodes of its newer
    /// look).
    Neutral,
    /// Light text and lines on a `#333333` canvas, near-black nodes
    /// (mermaid's `dark`); every text keeps 4.5:1 and every line 3:1.
    Dark,
    /// Green nodes with a deep green border, black lines (mermaid's
    /// `forest`).
    Forest,
}

impl Theme {
    /// Every theme, the default first.
    pub const ALL: [Theme; 5] = [
        Theme::Flowmaid,
        Theme::Mermaid,
        Theme::Neutral,
        Theme::Dark,
        Theme::Forest,
    ];

    /// The theme a name stands for, as mermaid spells them (`default`,
    /// `neutral`, `dark`, `forest`; any case, surrounding space
    /// ignored), plus `flowmaid` and `mermaid` (the same as `default`).
    /// mermaid's `base` (a theme meant to be recoloured through
    /// `themeVariables`) reads as `default`, its newer `neo` look as
    /// `neutral` and `neo-dark` as `dark`. `None` for any other name —
    /// callers fall back to the default, as mermaid does.
    pub fn from_name(name: &str) -> Option<Theme> {
        match name.trim().to_ascii_lowercase().as_str() {
            "flowmaid" => Some(Theme::Flowmaid),
            "default" | "mermaid" | "base" => Some(Theme::Mermaid),
            "neutral" | "neo" => Some(Theme::Neutral),
            "dark" | "neo-dark" => Some(Theme::Dark),
            "forest" => Some(Theme::Forest),
            _ => None,
        }
    }

    /// The theme's name: what [`Theme::from_name`] reads back to it.
    pub fn name(self) -> &'static str {
        match self {
            Theme::Flowmaid => "flowmaid",
            Theme::Mermaid => "default",
            Theme::Neutral => "neutral",
            Theme::Dark => "dark",
            Theme::Forest => "forest",
        }
    }

    /// The theme's colours ([`Palette::new`]).
    pub fn palette(self) -> Palette {
        Palette::new(self)
    }

    fn table(self) -> &'static ThemeTable {
        match self {
            Theme::Flowmaid => &FLOWMAID,
            Theme::Mermaid => &MERMAID,
            Theme::Neutral => &NEUTRAL,
            Theme::Dark => &DARK,
            Theme::Forest => &FOREST,
        }
    }
}

/// One theme's colours, as written in the source (see [`Palette`] for
/// what each role paints).
struct ThemeTable {
    background: &'static str,
    text: &'static str,
    muted_text: &'static str,
    line: &'static str,
    guide: &'static str,
    surface: &'static str,
    label_fill: &'static str,
    label_border: &'static str,
    /// Every node's fill and border; `None`: by shape ([`shape_style`]).
    node: Option<ShapeStyle>,
    node_text: &'static str,
    clusters: [ShapeStyle; 4],
    cluster_title: &'static str,
    note: ShapeStyle,
    note_text: &'static str,
    pseudo: &'static str,
}

/// [`Theme::Flowmaid`]: the colours every writer used before themes
/// existed, so the default output is unchanged byte for byte.
const FLOWMAID: ThemeTable = ThemeTable {
    background: "#ffffff",
    text: "#232840",
    muted_text: "#6a7086",
    line: "#44507a",
    guide: "#aeb6d8",
    surface: "#ffffff",
    label_fill: "#ffffff",
    label_border: "#d5d9ec",
    node: None,
    node_text: "#232840",
    clusters: CLUSTER_STYLES,
    cluster_title: CLUSTER_TITLE,
    note: NOTE_STYLE,
    note_text: "#232840",
    pseudo: "#44507a",
};

/// [`Theme::Mermaid`]: mermaid's `default` colours. Its frame and note
/// borders (`#aaaa33`, 2.4:1) are darkened, hue kept, to the 3:1 a
/// border needs on what it is drawn on, and the frames step a shade
/// deeper per nesting level as flowmaid's own do.
const MERMAID: ThemeTable = ThemeTable {
    background: "#ffffff",
    text: "#333333",
    muted_text: "#666666",
    line: "#333333",
    guide: "#888888",
    surface: "#ffffff",
    label_fill: "#e8e8e8",
    label_border: "#e8e8e8",
    node: Some(ShapeStyle {
        fill: "#ececff",
        stroke: "#9370db",
    }),
    node_text: "#333333",
    clusters: [
        ShapeStyle {
            fill: "#ffffde",
            stroke: "#97972d",
        },
        ShapeStyle {
            fill: "#f9f9cb",
            stroke: "#92922c",
        },
        ShapeStyle {
            fill: "#f3f3b8",
            stroke: "#8d8d2a",
        },
        ShapeStyle {
            fill: "#ededa5",
            stroke: "#8a8a29",
        },
    ],
    cluster_title: "#333333",
    note: ShapeStyle {
        fill: "#fff5ad",
        stroke: "#90902b",
    },
    note_text: "#333333",
    pseudo: "#333333",
};

/// [`Theme::Neutral`]: white nodes, one `#333333` ink.
const NEUTRAL: ThemeTable = ThemeTable {
    background: "#ffffff",
    text: "#333333",
    muted_text: "#666666",
    line: "#333333",
    guide: "#888888",
    surface: "#ffffff",
    label_fill: "#ffffff",
    label_border: "#cccccc",
    node: Some(ShapeStyle {
        fill: "#ffffff",
        stroke: "#333333",
    }),
    node_text: "#333333",
    clusters: [
        ShapeStyle {
            fill: "#f7f7f7",
            stroke: "#8a8a8a",
        },
        ShapeStyle {
            fill: "#eeeeee",
            stroke: "#898989",
        },
        ShapeStyle {
            fill: "#e5e5e5",
            stroke: "#838383",
        },
        ShapeStyle {
            fill: "#dcdcdc",
            stroke: "#7c7c7c",
        },
    ],
    cluster_title: "#333333",
    note: ShapeStyle {
        fill: "#f2f2f2",
        stroke: "#666666",
    },
    note_text: "#333333",
    pseudo: "#333333",
};

/// [`Theme::Dark`]: mermaid's `dark` canvas and nodes. Where mermaid
/// falls short on a dark page (its frame border is a 25% white, its
/// edge-label chip `#585858` under `#cccccc` text 4.4:1) the colours
/// are lifted to 3:1 for borders and lines and 4.5:1 for text.
const DARK: ThemeTable = ThemeTable {
    background: "#333333",
    text: "#e6e6e6",
    muted_text: "#b8b8b8",
    line: "#d3d3d3",
    guide: "#8f8f8f",
    surface: "#1f2020",
    label_fill: "#1f2020",
    label_border: "#6b6b6b",
    node: Some(ShapeStyle {
        fill: "#1f2020",
        stroke: "#cccccc",
    }),
    node_text: "#e6e6e6",
    clusters: [
        ShapeStyle {
            fill: "#3d3f3f",
            stroke: "#8a8a8a",
        },
        ShapeStyle {
            fill: "#474949",
            stroke: "#959595",
        },
        ShapeStyle {
            fill: "#515353",
            stroke: "#a3a3a3",
        },
        ShapeStyle {
            fill: "#5b5d5d",
            stroke: "#afafaf",
        },
    ],
    cluster_title: "#f0f0f0",
    note: ShapeStyle {
        fill: "#4d4a2e",
        stroke: "#a69641",
    },
    note_text: "#f2f2f2",
    pseudo: "#f4f4f4",
};

/// [`Theme::Forest`]: mermaid's `forest` greens; frame and note borders
/// darkened to 3:1 as in [`Theme::Mermaid`].
const FOREST: ThemeTable = ThemeTable {
    background: "#ffffff",
    text: "#000000",
    muted_text: "#4d4d4d",
    line: "#000000",
    guide: "#6f8f62",
    surface: "#ffffff",
    label_fill: "#e8e8e8",
    label_border: "#e8e8e8",
    node: Some(ShapeStyle {
        fill: "#cde498",
        stroke: "#13540c",
    }),
    node_text: "#000000",
    clusters: [
        ShapeStyle {
            fill: "#cdffb2",
            stroke: "#639942",
        },
        ShapeStyle {
            fill: "#bdf3a0",
            stroke: "#5c8f3d",
        },
        ShapeStyle {
            fill: "#aee78f",
            stroke: "#568539",
        },
        ShapeStyle {
            fill: "#9fdb7e",
            stroke: "#507c35",
        },
    ],
    cluster_title: "#000000",
    note: ShapeStyle {
        fill: "#fff5ad",
        stroke: "#639942",
    },
    note_text: "#000000",
    pseudo: "#000000",
};

/// mermaid `themeVariables` a diagram's source may set, as sRGB
/// channels: Copy, so [`crate::SvgOptions`] can carry them. Only the
/// handful that recolour the roles every writer shares are read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ThemeVars {
    pub(crate) background: Option<[u8; 3]>,
    pub(crate) primary: Option<[u8; 3]>,
    pub(crate) primary_border: Option<[u8; 3]>,
    pub(crate) primary_text: Option<[u8; 3]>,
    pub(crate) line: Option<[u8; 3]>,
    pub(crate) text: Option<[u8; 3]>,
}

impl ThemeVars {
    /// Take one `themeVariables` entry: `background`, `primaryColor`,
    /// `primaryBorderColor`, `primaryTextColor`, `lineColor` or
    /// `textColor`, any colour [`css_rgb`] reads. Other keys, and
    /// values that are no colour, are ignored (theme config is
    /// decoration, never a reason to fail).
    pub(crate) fn set(&mut self, key: &str, value: &str) {
        let slot = match key {
            "background" => &mut self.background,
            "primaryColor" => &mut self.primary,
            "primaryBorderColor" => &mut self.primary_border,
            "primaryTextColor" => &mut self.primary_text,
            "lineColor" => &mut self.line,
            "textColor" => &mut self.text,
            _ => return,
        };
        if let Some(rgb) = css_rgb(value) {
            *slot = Some(rgb);
        }
    }
}

fn hex([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Every colour a theme paints with, resolved: the shared roles the
/// SVG writers draw from, so a host painting a [`crate::scene::Scene`]
/// can match the SVG exactly (`opts.palette()` for the options it
/// renders with, see [`crate::SvgOptions::palette`]). All colours are
/// `#rrggbb`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Palette {
    /// The theme the colours come from.
    pub theme: Theme,
    /// The canvas (the background `<rect>`).
    pub background: String,
    /// Text on the canvas and on light boxes: edge labels, member rows,
    /// messages, captions.
    pub text: String,
    /// Secondary text: ER attribute types.
    pub muted_text: String,
    /// Edges, relationship lines, messages and their end glyphs.
    pub line: String,
    /// Faint structural lines: lifelines, frames, axes, rails.
    pub guide: String,
    /// The fill of plain boxes on the canvas: class and ER bodies,
    /// activation bars, architecture services, hollow end glyphs.
    pub surface: String,
    /// The chip behind an edge or relationship label.
    pub label_fill: String,
    /// The chip's border.
    pub label_border: String,
    /// Text inside a node (and a class / ER / sequence header when the
    /// theme paints items alike, see [`Palette::node_style`]).
    pub node_text: String,
    /// A subgraph / composite title.
    pub cluster_title: String,
    /// A note card's fill.
    pub note_fill: String,
    /// A note card's border.
    pub note_stroke: String,
    /// A note card's text.
    pub note_text: String,
    /// State start / end dots and fork bars.
    pub pseudo: String,
    node_fill: Option<String>,
    node_stroke: Option<String>,
    clusters: Vec<(String, String)>,
}

impl Palette {
    /// `theme`'s colours.
    pub fn new(theme: Theme) -> Palette {
        Palette::with_vars(theme, &ThemeVars::default())
    }

    /// `theme`'s colours with the source's `themeVariables` laid over
    /// them. A `primaryColor` without a `primaryTextColor` keeps the
    /// theme's node text where it still reads (4.5:1), else takes
    /// [`text_on`] the new fill.
    pub(crate) fn with_vars(theme: Theme, vars: &ThemeVars) -> Palette {
        let t = theme.table();
        let pick = |v: Option<[u8; 3]>, base: &str| v.map_or_else(|| base.to_string(), hex);
        let node_fill = vars.primary.map(hex).or(t.node.map(|n| n.fill.to_string()));
        let node_stroke = vars
            .primary_border
            .map(hex)
            .or(t.node.map(|n| n.stroke.to_string()));
        let node_text = match (vars.primary_text, vars.primary) {
            (Some(c), _) => hex(c),
            (None, Some(fill)) => {
                let fill = hex(fill);
                let reads =
                    contrast_ratio(t.node_text, &fill).is_some_and(|c| c >= MIN_TEXT_CONTRAST);
                if reads {
                    t.node_text.to_string()
                } else {
                    text_on(&fill).to_string()
                }
            }
            (None, None) => t.node_text.to_string(),
        };
        Palette {
            theme,
            background: pick(vars.background, t.background),
            text: pick(vars.text, t.text),
            muted_text: t.muted_text.to_string(),
            line: pick(vars.line, t.line),
            guide: t.guide.to_string(),
            surface: t.surface.to_string(),
            label_fill: t.label_fill.to_string(),
            label_border: t.label_border.to_string(),
            node_text,
            cluster_title: t.cluster_title.to_string(),
            note_fill: t.note.fill.to_string(),
            note_stroke: t.note.stroke.to_string(),
            note_text: t.note_text.to_string(),
            pseudo: t.pseudo.to_string(),
            node_fill,
            node_stroke,
            clusters: t
                .clusters
                .iter()
                .map(|c| (c.fill.to_string(), c.stroke.to_string()))
                .collect(),
        }
    }

    /// Fill and border of an unstyled node of `shape`: the theme's one
    /// node paint, or, in [`Theme::Flowmaid`], [`shape_style`]'s colour
    /// for the shape. State start / end dots and fork bars are
    /// [`Palette::pseudo`] in every theme.
    pub fn node_style(&self, shape: Shape) -> (&str, &str) {
        if matches!(shape, Shape::StateStart | Shape::StateEnd | Shape::ForkBar) {
            return (&self.pseudo, &self.pseudo);
        }
        let by_shape = shape_style(shape);
        (
            self.node_fill.as_deref().unwrap_or(by_shape.fill),
            self.node_stroke.as_deref().unwrap_or(by_shape.stroke),
        )
    }

    /// Fill and border of a subgraph frame `depth` levels in (0 =
    /// outermost), a step deeper per level like [`cluster_style`].
    pub fn cluster_style(&self, depth: usize) -> (&str, &str) {
        let (f, s) = &self.clusters[depth.min(self.clusters.len() - 1)];
        (f, s)
    }

    /// How item `index` of a class / ER / sequence diagram is painted:
    /// its own [`accent`] (text on it by [`text_on`], on the canvas in
    /// [`accent_ink`]) while nodes are coloured by shape, else the
    /// theme's node paint for every item, as mermaid paints them.
    pub(crate) fn item(&self, index: usize) -> ItemPaint<'_> {
        match &self.node_fill {
            None => {
                let a = accent(index);
                ItemPaint {
                    fill: a,
                    stroke: a,
                    ink: text_on(a),
                    canvas_ink: accent_ink(index),
                }
            }
            Some(fill) => ItemPaint {
                fill,
                stroke: self.node_stroke.as_deref().unwrap_or(&self.line),
                ink: &self.node_text,
                canvas_ink: &self.text,
            },
        }
    }

    /// [`accent_ink`] for item `index` made readable on this canvas:
    /// the accent darkened for white as it is, or, where that falls
    /// under 4.5:1 (a dark canvas), the accent lightened until it reads
    /// ([`readable_on`]).
    pub fn accent_ink(&self, index: usize) -> String {
        let ink = accent_ink(index);
        if contrast_ratio(ink, &self.background).is_some_and(|c| c >= MIN_TEXT_CONTRAST) {
            ink.to_string()
        } else {
            readable_on(accent(index), &self.background)
        }
    }

    /// `flowmaid` — one writer's own tuned constant for a role (the
    /// sequence lifeline grey, the timeline axis, ...) — in
    /// [`Theme::Flowmaid`], so the default output keeps every one of
    /// them exactly; `token`, the theme's colour for the role, in any
    /// other theme.
    pub(crate) fn tuned<'a>(&'a self, flowmaid: &'a str, token: &'a str) -> &'a str {
        if self.theme == Theme::Flowmaid {
            flowmaid
        } else {
            token
        }
    }
}

/// [`Palette::item`]: one class box's, ER entity's or sequence
/// participant's colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ItemPaint<'a> {
    /// The header band / box fill.
    pub(crate) fill: &'a str,
    /// Its border (and dividers).
    pub(crate) stroke: &'a str,
    /// Text on `fill`.
    pub(crate) ink: &'a str,
    /// Text in the item's colour straight on the canvas (an actor's
    /// name under its stick figure).
    pub(crate) canvas_ink: &'a str,
}

/// `color` as `#rrggbb`, moved toward white (on a dark `bg`) or black
/// (on a light one) just far enough to reach [`MIN_TEXT_CONTRAST`]
/// against `bg`; unchanged where it already does. Colours
/// [`css_rgb`] cannot read come back as they are.
pub fn readable_on(color: &str, bg: &str) -> String {
    let (Some(rgb), Some(bg_hex)) = (css_rgb(color), css_hex(bg)) else {
        return color.to_string();
    };
    let reads = |c: &str| contrast_ratio(c, &bg_hex).is_some_and(|k| k >= MIN_TEXT_CONTRAST);
    let start = hex(rgb);
    if reads(&start) {
        return start;
    }
    let to_white = relative_luminance(&bg_hex).is_some_and(|l| l < 0.18);
    // 64 even steps: enough to land within a few units of the least
    // move that reads, and the far end (white / black) always does on
    // a background past the 0.18 split.
    for k in 1..=64 {
        let t = f64::from(k) / 64.0;
        let mixed = rgb.map(|c| {
            let c = f64::from(c);
            let target = if to_white { 255.0 } else { 0.0 };
            (c + (target - c) * t).round() as u8
        });
        let h = hex(mixed);
        if reads(&h) {
            return h;
        }
    }
    text_on(&bg_hex).to_string()
}

