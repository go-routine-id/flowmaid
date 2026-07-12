//! flowmaid — a Mermaid-like diagram engine.
//!
//! Supported diagram types: flowcharts (`flowchart` / `graph`) and
//! Entity-Relationship diagrams (`erDiagram`).
//!
//! Library usage:
//!
//! ```
//! let svg = flowmaid::render_svg("flowchart TD\nA[Start] --> B[Done]").unwrap();
//! assert!(svg.starts_with("<svg"));
//!
//! let er = flowmaid::render_svg("erDiagram\nusers ||--o{ posts : writes").unwrap();
//! assert!(er.contains("users"));
//! ```

pub mod er;
pub mod layout;
pub mod model;
pub mod parser;
pub mod render;
pub mod scene;

pub use model::Document;
pub use parser::ParseError;

/// Shortcut: Mermaid-syntax text -> SVG string. Dispatches on the
/// diagram type header (flowchart/graph or erDiagram).
pub fn render_svg(source: &str) -> Result<String, ParseError> {
    match parser::parse_document(source)? {
        Document::Flowchart(g) => Ok(render::render(&g)),
        Document::Er(d) => Ok(render::render_er(&d)),
    }
}
