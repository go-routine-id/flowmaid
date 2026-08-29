// The layout code walks several parallel arrays (positions, sizes,
// layers) by index on purpose — iterator zips would obscure the
// math, not clarify it.
#![allow(clippy::needless_range_loop)]

//! flowmaid — a Mermaid-like diagram engine.
//!
//! Supported diagram types: flowcharts (`flowchart` / `graph`),
//! Entity-Relationship diagrams (`erDiagram`), UML class diagrams
//! (`classDiagram`), sequence diagrams (`sequenceDiagram`), pie
//! charts (`pie`), state diagrams (`stateDiagram-v2`), mindmaps
//! (`mindmap`), user-journey diagrams (`journey`), git graphs
//! (`gitGraph`), and architecture diagrams (`architecture-beta`).
//!
//! There is also an engine-native swimlane renderer driven by JSON in
//! the [`advance`] module (`render_advance_svg`, `layout_advance`, and
//! drag-friendly routing helpers) — it has no mermaid header.
//!
//! Library usage:
//!
//! ```
//! let flow = flowmaid::render_svg("flowchart TD\nA[Start] --> B[Done]").unwrap();
//! assert!(flow.starts_with("<svg"));
//!
//! let er = flowmaid::render_svg("erDiagram\nusers ||--o{ posts : writes").unwrap();
//! assert!(er.contains("users"));
//!
//! let uml = flowmaid::render_svg("classDiagram\nAnimal <|-- Dog").unwrap();
//! assert!(uml.contains("Animal"));
//!
//! let git = flowmaid::render_svg("gitGraph\ncommit\nbranch feat\ncommit").unwrap();
//! assert!(git.contains("main"));
//! ```

pub mod advance;
pub mod architecture;
pub mod class;
pub mod emit;
pub mod er;
pub mod fold;
pub mod gitgraph;
pub(crate) mod json;
pub mod journey;
pub mod layout;
pub mod mindmap;
pub mod model;
pub mod parser;
pub mod pie;
pub mod render;
pub mod scene;
pub mod seq;
pub mod style;

pub use emit::to_mermaid;
pub use model::Document;
pub use parser::ParseError;
pub use scene::SvgOptions;

pub use advance::{
    edge_kind_name, layout_advance, render_advance_routed, render_advance_routed_with_lanes,
    render_advance_svg, scene_to_json, shape_name, to_json, AdvanceConfig, AdvanceDiagram,
    AdvanceDirection, AdvanceEdge, AdvanceError, AdvanceLane, AdvanceNode, AdvanceOrder,
    AdvanceScene, AdvanceSceneEdge, AdvanceSceneLane, AdvanceSceneNode, AdvanceStyle,
};

/// Shortcut: Mermaid-syntax text -> SVG string. Dispatches on the
/// diagram type header (flowchart/graph, erDiagram, classDiagram,
/// sequenceDiagram, pie, stateDiagram-v2, mindmap, journey, gitGraph,
/// architecture-beta).
pub fn render_svg(source: &str) -> Result<String, ParseError> {
    render_svg_advanced(source, &SvgOptions::default())
}

/// [`render_svg`] with explicit viewport options — opt in to a responsive
/// root `<svg>` (`width="100%"`) and/or a custom `preserveAspectRatio`.
/// Dispatches on the same diagram-type headers as [`render_svg`].
pub fn render_svg_advanced(source: &str, opts: &SvgOptions) -> Result<String, ParseError> {
    match parser::parse_document(source)? {
        // State diagrams live on the same Graph as flowcharts.
        Document::Flowchart(g) => Ok(render::render_with(&g, opts)),
        Document::State(g) => Ok(render::render_titled_with(&g, "State diagram", opts)),
        Document::Er(d) => Ok(render::render_er_with(&d, opts)),
        Document::Class(d) => Ok(render::render_class_with(&d, opts)),
        Document::Sequence(d) => Ok(render::render_seq_with(&d, opts)),
        Document::Pie(d) => Ok(render::render_pie_with(&d, opts)),
        Document::Mindmap(d) => Ok(render::render_mindmap_with(&d, opts)),
        Document::Journey(d) => Ok(render::render_journey_with(&d, opts)),
        Document::GitGraph(d) => Ok(render::render_gitgraph_with(&d, opts)),
        Document::Architecture(d) => Ok(render::render_architecture_with(&d, opts)),
    }
}
