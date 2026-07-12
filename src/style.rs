//! Shared color theme, exposed so hosts (GUI painters) can match
//! the SVG writers exactly.
//!
//! Flowchart nodes are colored by shape semantics — the shape
//! already encodes meaning in Mermaid (stadium = start/end,
//! diamond = decision, ...), so color reinforces it for free.
//! ER entities cycle through an accent palette by index, which is
//! stable across renders because entity order is parse order.

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
    }
}

/// Accent palette for ER entity headers (and anything else that
/// wants a stable per-item color).
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
}
