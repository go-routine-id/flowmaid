//! flowmaid — pustaka mesin diagram flowchart ala Mermaid.
//!
//! Pemakaian sebagai library:
//!
//! ```
//! let svg = flowmaid::render_svg("flowchart TD\nA[Mulai] --> B[Selesai]").unwrap();
//! assert!(svg.starts_with("<svg"));
//! ```

pub mod layout;
pub mod model;
pub mod parser;
pub mod render;
pub mod scene;

pub use parser::ParseError;

/// Jalur pintas: teks bersintaks Mermaid -> string SVG.
pub fn render_svg(source: &str) -> Result<String, ParseError> {
    let g = parser::parse(source)?;
    Ok(render::render(&g))
}
