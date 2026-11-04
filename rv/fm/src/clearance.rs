//! Shared separation thresholds (px): what the edge stages keep apart
//! and what the [`crate::quality`] yardstick measures, in one place.
//!
//! The router, the port stage and the scene builder steer by these;
//! `quality` re-exports them, so a reroute the router calls clean is
//! clean by the metrics too, and tuning a stage never silently moves the
//! yardstick (or the other way round).

/// A node is "crossed" when an unrelated edge comes closer than this
/// to its outline (px) — covers the stroke half-width plus
/// anti-aliasing, so a line grazing the border counts.
pub const NODE_CLEARANCE: f64 = 2.0;
/// An edge running closer than this to an unrelated node outline (but
/// not crossing it) is "cramped" (px, G4).
pub const CRAMP_NODE_GAP: f64 = 8.0;
/// Two edges running closer than this, nearly parallel, are drawn on
/// top of each other (px, G1/G4).
pub const EDGE_SPACING: f64 = 6.0;
/// |cos| of the angle between two edge directions above which they
/// count as parallel (≈ 20°).
pub const PARALLEL_COS: f64 = 0.94;
/// A pair of edges counts as overlapping once their shared parallel
/// run is longer than this (px) — shorter touches are just crossings
/// drawn at a shallow angle.
pub const MIN_OVERLAP_RUN: f64 = 12.0;
/// Around an endpoint two edges share, this much (px from its box) is
/// excluded from the parallel-run and gap metrics: lines may converge
/// there; port spacing is measured separately.
/// Crossings are counted right up to the shared end.
pub const PORT_ZONE: f64 = 16.0;
/// Backtracking along the flow axis above this (px) is a wiggle (G8).
pub const WIGGLE_TOL: f64 = 3.0;
/// Edge polylines are resampled to at most this step (px) before the
/// point-based tests, so thin nodes can't slip between samples.
pub const SAMPLE_STEP: f64 = 2.0;
/// A frame counts as entered when an edge point lies this far inside
/// its rectangle (px) — the border stroke itself is not "inside".
pub const FRAME_INSET: f64 = 1.0;
/// Two crossing points of the same edge pair closer than this (px)
/// are one crossing (the sampled polylines can hit twice).
pub const CROSS_MERGE: f64 = 4.0;
/// Cluster title chip, as `to_svg` draws it: left inset inside the
/// frame (px).
pub(crate) const CHIP_INSET_X: f64 = 6.0;
/// Cluster title chip, as `to_svg` draws it: top inset inside the
/// frame (px).
pub(crate) const CHIP_INSET_Y: f64 = 5.0;
/// Chip padding added to the title text width (px).
pub(crate) const CHIP_PAD_W: f64 = 8.0;
/// Chip height for a one-line title (px).
pub(crate) const CHIP_H: f64 = 16.0;
/// Comfortable clearance (px, G4): an edge running parallel to another
/// edge, a frame border or an unrelated node's side closer than this
/// reads as cramped even where it does not touch. Twice the spacing at
/// which two strokes read as one ([`EDGE_SPACING`]).
pub const CLEAR_GAP: f64 = 2.0 * EDGE_SPACING;
/// A cramped parallel stretch (beside another edge, a frame border or a
/// node side, within [`CLEAR_GAP`]) counts once it is longer than this
/// (px): two node clearances, so a line merely passing a corner does not.
pub const TIGHT_RUN_MIN: f64 = 3.0 * CRAMP_NODE_GAP;
/// An edge runs along a frame border when its samples stay within this
/// distance (px) of the border line, inside the border's span (G4):
/// closer than an edge keeps from a node it does not touch.
pub const BORDER_HUG_GAP: f64 = CRAMP_NODE_GAP - 1.0;

/// Spacing (px) the edge stages keep between two parallel edges: the
/// comfortable clearance, so neighbouring legs never read as cramped.
pub const EDGE_SEP: f64 = CLEAR_GAP;
/// Spacing (px) kept between an edge and the side of a node it does
/// not touch.
pub const NODE_SEP: f64 = CLEAR_GAP;
/// Spacing (px) kept between an edge and a frame border, inside or
/// out: a little over [`NODE_SEP`], as a border is a long straight line
/// a parallel leg is easily read as part of.
pub const FRAME_SEP: f64 = CLEAR_GAP + 2.0;
/// Spacing (px) kept between an edge and a frame's title chip.
pub const CHIP_SEP: f64 = EDGE_SPACING;
/// The closest (px) the channel nudge squeezes two parallel legs when
/// their channel cannot hold [`EDGE_SEP`]: still clearly two strokes.
pub const EDGE_SEP_MIN: f64 = EDGE_SPACING + 2.0;
