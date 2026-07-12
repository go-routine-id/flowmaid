//! flowmaid — a Mermaid-like flowchart diagram engine.
//!
//! Library usage:
//!
//! ```
//! let svg = flowmaid::render_svg("flowchart TD\nA[Start] --> B[Done]").unwrap();
//! assert!(svg.starts_with("<svg"));
//! ```

pub mod layout;
pub mod model;
pub mod parser;
pub mod render;
pub mod scene;

pub use parser::ParseError;

/// Shortcut: Mermaid-syntax text -> SVG string.
pub fn render_svg(source: &str) -> Result<String, ParseError> {
    let g = parser::parse(source)?;
    Ok(render::render(&g))
}
