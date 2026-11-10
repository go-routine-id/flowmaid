#### subgraph.dagre.svg  (canvas 480 x 737, flow axis y)

Blocks (cluster rectangles):

| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |
|---|---|---|---|---|---|---|---|---|---|
| platform | 0 | - | 8 | 204 | 464 | 430 | 58% | 2 (5 incl. nested) | 1 / 1 / 1 / 2 |
| events | 1 | platform | 28 | 419 | 424 | 190 | 26% | 3 (3 incl. nested) | 1 / 2 |

Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):

| band | from | to | size | items in cross-axis order | gap to next band |
|---|---|---|---|---|---|
| 0 | 8 | 53 | 45 | Client | 50 |
| 1 | 103 | 154 | 51 | GW | 50 |
| 2 | 204 | 634 | 430 | [platform] | 50 |
| 3 | 684 | 729 | 45 | Store | - |

Top-level blocks: 1; bands: 4; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): none.

Sides of block frames crossed by inter-block edges: enters via top: 1, leaves via bottom: 2.

Edges: 8 total, 4 with both ends in the same top-level block (intra) and 4 between different top-level items (inter); 0 have a block (not a node) as an end.

Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:

| place | edges with a run here | total run length | distinct lanes (flow coordinates) |
|---|---|---|---|

| inter-block edge | from (block) | to (block) | bands | cross-flow runs |
|---|---|---|---|---|
| L_Client_GW_0 | Client (Client) | GW (GW) | 0 -> 1 | straight |
| L_GW_Auth_0 | GW (GW) | Auth (platform) | 1 -> 2 | straight |
| L_C1_Store_0 | C1 (platform) | Store (Store) | 2 -> 3 | straight |
| L_C2_Store_0 | C2 (platform) | Store (Store) | 2 -> 3 | straight |

Edges drawn through a block they do not belong to: none.

Edges drawn over a node that is not their end: none.

Edges drawn over a block title: L_GW_Auth_0 over title of [platform]; L_Users_Bus_0 over title of [events].

Totals: canvas area 0.35 Mpx2, edge length 634 px, bends 14, edge crossings (drawn polylines) 0.

