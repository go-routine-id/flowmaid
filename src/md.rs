//! Markdown companion (behind the optional `markdown` cargo
//! feature): find the ```mermaid blocks of a Markdown document,
//! write edited diagram sources back into their fences, and render
//! whole documents to HTML with the diagrams inlined as SVG.
//!
//! The engine core stays zero-dependency — this module (and its one
//! small pure-Rust dependency, the `markdown` crate) only exists
//! when the feature is enabled:
//!
//! ```toml
//! flowmaid = { version = "0.12", features = ["markdown"] }
//! ```
//!
//! Blocks are located through the Markdown AST, not regexes, so
//! tilde fences, info strings, and fences inside blockquotes are
//! all detected correctly.

use crate::ParseError;
use std::ops::Range;

/// One ```mermaid (or ```mmd) fence found in a Markdown document.
#[derive(Debug, Clone)]
pub struct MermaidBlock {
    /// Diagram source between the fences (dedented by the parser).
    pub source: String,
    /// Byte range of the WHOLE fence (opening through closing line)
    /// in the original document.
    pub span: Range<usize>,
    /// 0-based position among the document's mermaid blocks.
    pub index: usize,
}

/// All mermaid blocks of a document, in source order.
pub fn mermaid_blocks(md: &str) -> Vec<MermaidBlock> {
    fn walk(node: &markdown::mdast::Node, out: &mut Vec<MermaidBlock>) {
        if let markdown::mdast::Node::Code(c) = node {
            let lang = c.lang.as_deref().unwrap_or("");
            if lang.eq_ignore_ascii_case("mermaid") || lang.eq_ignore_ascii_case("mmd") {
                if let Some(p) = &c.position {
                    out.push(MermaidBlock {
                        source: c.value.clone(),
                        span: p.start.offset..p.end.offset,
                        index: out.len(),
                    });
                }
            }
        }
        if let Some(children) = node.children() {
            for ch in children {
                walk(ch, out);
            }
        }
    }
    let mut out = Vec::new();
    if let Ok(ast) = markdown::to_mdast(md, &markdown::ParseOptions::default()) {
        walk(&ast, &mut out);
    }
    out
}

/// Replace the CONTENT of mermaid block `index` with `source`,
/// keeping the opening/closing fence lines verbatim (info string
/// included). Returns `None` when the block no longer exists or its
/// fences are indented (inside a list — the content would need
/// re-indentation, which is not supported).
pub fn splice_block(md: &str, index: usize, source: &str) -> Option<String> {
    let block_info = mermaid_blocks(md).into_iter().nth(index)?;
    let range = block_info.span;
    let block = &md[range.clone()];
    let open_len = block.find('\n')?;
    let close_start = block.rfind('\n')?;
    let (open, close) = (&block[..open_len], &block[close_start + 1..]);
    // Both lines must be genuine, unindented fences: an indented
    // fence needs content re-indentation, and an unclosed fence at
    // EOF makes the last line content rather than a fence.
    let fence = |s: &str| s.starts_with("```") || s.starts_with("~~~");
    if !fence(open) || !fence(close) {
        return None;
    }
    let mut out = String::with_capacity(md.len() + source.len());
    out.push_str(&md[..range.start]);
    out.push_str(open);
    out.push('\n');
    out.push_str(source.trim_end_matches('\n'));
    out.push('\n');
    out.push_str(close);
    out.push_str(&md[range.end..]);
    Some(out)
}

/// Render every mermaid block of a document to SVG, in order.
pub fn render_blocks(md: &str) -> Vec<Result<String, ParseError>> {
    mermaid_blocks(md)
        .iter()
        .map(|b| crate::render_svg(&b.source))
        .collect()
}

/// Render a whole Markdown document to an HTML fragment with every
/// mermaid block replaced by its rendered SVG (wrapped in
/// `<figure class="flowmaid-diagram">`). A block that fails to parse
/// becomes `<pre class="flowmaid-error">` carrying the line-numbered
/// message instead of breaking the document.
pub fn render_html(md: &str) -> String {
    let blocks = mermaid_blocks(md);
    // Swap each fence for a unique raw-HTML marker, convert the
    // document, then swap the markers for the rendered SVGs. Markers
    // survive `to_html` because raw HTML is allowed here — the only
    // raw HTML in the intermediate source is ours.
    let mut source = String::with_capacity(md.len());
    let mut last = 0;
    for b in &blocks {
        source.push_str(&md[last..b.span.start]);
        source.push_str(&marker(b.index));
        last = b.span.end;
    }
    source.push_str(&md[last..]);

    let options = markdown::Options {
        compile: markdown::CompileOptions {
            allow_dangerous_html: true,
            ..markdown::CompileOptions::default()
        },
        ..markdown::Options::default()
    };
    let mut html = markdown::to_html_with_options(&source, &options)
        .unwrap_or_else(|_| markdown::to_html(&source));

    for b in &blocks {
        let replacement = match crate::render_svg(&b.source) {
            Ok(svg) => format!("<figure class=\"flowmaid-diagram\">{svg}</figure>"),
            Err(e) => format!(
                "<pre class=\"flowmaid-error\">mermaid block #{}: {}</pre>",
                b.index + 1,
                html_escape(&e.to_string())
            ),
        };
        html = html.replace(&marker(b.index), &replacement);
    }
    html
}

fn marker(index: usize) -> String {
    format!("<!--flowmaid:block:{index}-->")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "# Title\n\nintro text\n\n```mermaid\nflowchart TD\nA-->B\n```\n\n\
                       between\n\n```js\nconsole.log(1)\n```\n\n\
                       ~~~mermaid\npie\n\"x\" : 1\n~~~\n\ntail\n";

    #[test]
    fn blocks_are_found_in_order_with_spans() {
        let blocks = mermaid_blocks(DOC);
        assert_eq!(blocks.len(), 2, "the js fence is not a mermaid block");
        assert!(blocks[0].source.starts_with("flowchart TD"));
        assert!(blocks[1].source.starts_with("pie"));
        assert!(DOC[blocks[0].span.clone()].starts_with("```mermaid"));
        assert!(DOC[blocks[1].span.clone()].starts_with("~~~mermaid"));
        assert_eq!((blocks[0].index, blocks[1].index), (0, 1));
    }

    #[test]
    fn splice_replaces_only_the_target_block() {
        let out = splice_block(DOC, 1, "pie\n\"y\" : 9").unwrap();
        assert!(out.contains("~~~mermaid\npie\n\"y\" : 9\n~~~"));
        assert!(out.contains("A-->B"), "block #1 untouched");
        assert!(out.contains("console.log(1)") && out.contains("tail"));
        // Out-of-range and indented fences refuse instead of guessing.
        assert!(splice_block(DOC, 5, "x").is_none());
        assert!(splice_block("- item\n\n    ```mermaid\n    A-->B\n    ```\n", 0, "x").is_none());
    }

    #[test]
    fn render_html_inlines_svg_and_reports_bad_blocks() {
        let html = render_html(DOC);
        assert!(html.contains("<h1>Title</h1>"));
        assert_eq!(html.matches("<figure class=\"flowmaid-diagram\">").count(), 2);
        assert!(html.contains("<svg"), "diagrams inlined as SVG");
        assert!(html.contains("console.log"), "js fence stays a code block");
        assert!(!html.contains("flowmaid:block:"), "no markers leak");

        let bad = "text\n\n```mermaid\ngantt\nnope\n```\n";
        let html = render_html(bad);
        assert!(html.contains("flowmaid-error"));
        assert!(html.contains("not supported yet"));
    }

    #[test]
    fn render_blocks_matches_block_order() {
        let out = render_blocks(DOC);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|r| r.is_ok()));
    }
}
