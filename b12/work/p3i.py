p='src/layout.rs'; s=open(p).read()
def rep(a,b):
    global s
    assert s.count(a)==1, (s.count(a), a[:70])
    s=s.replace(a,b)
rep('''/// Block mode: wrap the long rows of members without inner links.
///
/// A member of a block with no link to anything else inside the block
/// lies on the block's first row, so a block of such members is one
/// row however many they are. Where that row is more than
/// [`RIBBON_ENTRY`] times as broad as the block is deep, and the block
/// has room below it in its band, the row is wrapped: member `i` ranks
/// above member `i + cols` (rank-only pairs, as the grid of loose
/// members is packed), in as many rows as bring the block under that
/// ratio — or as fit: a wrap that would add a layer to the drawing is
/// not taken, and one row fewer is tried. So no block row is longer
/// than its band needs, and a block alone in its band keeps its row.
///
/// `groups` gains the pairs of the wraps taken and `rk` is the layering
/// with them.''','''/// Block mode: wrap the long rows of unlinked members.
///
/// A member of a block with no link at all lies on the block's first
/// row, and the grid of loose members packs such members only from four
/// on (the scene's `grid_pairs`). Where a row of three or more of them
/// is more than [`RIBBON_ENTRY`] times as broad as the block is deep,
/// and the block has room below it in its band, the row is wrapped:
/// member `i` ranks above member `i + cols` (rank-only pairs, as the
/// grid is packed), in as many rows as bring the block under that ratio
/// — or as fit: a wrap that would add a layer to the drawing is not
/// taken, and one row fewer is tried. So no row of unlinked members is
/// longer than its band needs, and a block alone in its band keeps its
/// row.
///
/// Members with a link stay where the layering put them: a member an
/// edge enters from outside the block is entered through the block's
/// flow-facing side, and a row above it would stand in that edge's way
/// (wrapped by the same test, the public fixture's shared services — a
/// row of four, each with links out of the block — became two rows).
///
/// `groups` gains the pairs of the wraps taken and `rk` is the layering
/// with them.''')
rep('''        // Members with a link to something else inside the block.
        let mut linked = vec![false; n];
        let mut join = |a: usize, b: usize| {
            if a != b && inside(a) && inside(b) {
                linked[a] = true;
                linked[b] = true;
            }
        };''','''        // Nodes with a link (a self-loop is none).
        let mut linked = vec![false; n];
        let mut join = |a: usize, b: usize| {
            if a != b {
                linked[a] = true;
                linked[b] = true;
            }
        };''')
open(p,'w').write(s)
