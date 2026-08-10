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

pub mod architecture;
pub mod class;
pub mod emit;
pub mod er;
pub mod fold;
pub mod gitgraph;
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

/// Shortcut: Mermaid-syntax text -> SVG string. Dispatches on the
/// diagram type header (flowchart/graph, erDiagram, classDiagram,
/// sequenceDiagram, pie, stateDiagram-v2, mindmap, journey, gitGraph,
/// architecture-beta).
pub fn render_svg(source: &str) -> Result<String, ParseError> {
    match parser::parse_document(source)? {
        // State diagrams live on the same Graph as flowcharts.
        Document::Flowchart(g) => Ok(render::render(&g)),
        Document::State(g) => Ok(render::render_titled(&g, "State diagram")),
        Document::Er(d) => Ok(render::render_er(&d)),
        Document::Class(d) => Ok(render::render_class(&d)),
        Document::Sequence(d) => Ok(render::render_seq(&d)),
        Document::Pie(d) => Ok(render::render_pie(&d)),
        Document::Mindmap(d) => Ok(render::render_mindmap(&d)),
        Document::Journey(d) => Ok(render::render_journey(&d)),
        Document::GitGraph(d) => Ok(render::render_gitgraph(&d)),
        Document::Architecture(d) => Ok(render::render_architecture(&d)),
    }
}
