//! Model data inti: graph, node, edge.

use std::collections::HashMap;

/// Arah aliran diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Atas ke bawah (top-down). Alias: TB.
    TD,
    /// Kiri ke kanan.
    LR,
    /// Kanan ke kiri.
    RL,
    /// Bawah ke atas.
    BT,
}

impl Default for Direction {
    fn default() -> Self {
        Direction::TD
    }
}

/// Bentuk node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `A[teks]` — persegi panjang.
    Rect,
    /// `A(teks)` — sudut membulat.
    Rounded,
    /// `A([teks])` — stadium / pil.
    Stadium,
    /// `A{teks}` — belah ketupat (keputusan).
    Diamond,
    /// `A((teks))` — lingkaran.
    Circle,
}

/// Jenis garis penghubung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// `-->` panah biasa.
    Arrow,
    /// `---` garis tanpa panah.
    Open,
    /// `-.->` garis putus-putus.
    Dotted,
    /// `==>` garis tebal.
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

/// Graph hasil parsing, siap di-layout.
#[derive(Debug, Default)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    index: HashMap<String, usize>,
}

impl Graph {
    /// Ambil node berdasarkan id; buat baru jika belum ada.
    /// Label/bentuk terbaru menimpa yang lama (perilaku ala Mermaid).
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
