//! Core data model: graph, nodes, edges.

use std::collections::HashMap;

/// Flow direction of the diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Top-down. Alias: TB.
    TD,
    /// Left to right.
    LR,
    /// Right to left.
    RL,
    /// Bottom to top.
    BT,
}

impl Default for Direction {
    fn default() -> Self {
        Direction::TD
    }
}

/// Node shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `A[text]` — rectangle.
    Rect,
    /// `A(text)` — rounded corners.
    Rounded,
    /// `A([text])` — stadium / pill.
    Stadium,
    /// `A{text}` — diamond (decision).
    Diamond,
    /// `A((text))` — circle.
    Circle,
}

/// Edge line style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// `-->` regular arrow.
    Arrow,
    /// `---` plain line, no arrowhead.
    Open,
    /// `-.->` dotted line.
    Dotted,
    /// `==>` thick line.
    Thick,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: Shape,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
    pub kind: EdgeKind,
}

/// Parsed graph, ready for layout.
#[derive(Debug, Default)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    index: HashMap<String, usize>,
}

impl Graph {
    /// Look up a node by id; create it if missing.
    /// The latest label/shape overrides earlier ones (Mermaid behaviour).
    pub fn ensure_node(&mut self, id: &str, label: Option<String>, shape: Option<Shape>) -> usize {
        if let Some(&i) = self.index.get(id) {
            if let Some(l) = label {
                self.nodes[i].label = l;
            }
            if let Some(s) = shape {
                self.nodes[i].shape = s;
            }
            i
        } else {
            let i = self.nodes.len();
            self.nodes.push(Node {
                id: id.to_string(),
                label: label.unwrap_or_else(|| id.to_string()),
                shape: shape.unwrap_or(Shape::Rect),
            });
            self.index.insert(id.to_string(), i);
            i
        }
    }

    pub fn add_edge(&mut self, from: usize, to: usize, label: Option<String>, kind: EdgeKind) {
        self.edges.push(Edge {
            from,
            to,
            label,
            kind,
        });
    }
}
