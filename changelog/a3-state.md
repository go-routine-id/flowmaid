<!-- Work package A3 (stateDiagram-v2). Merge these bullets into `## [Unreleased]`
     under the matching Keep-a-Changelog headings. -->

### Added

- **State diagram notes are drawn.** `note right of X : text`, `note left of X : text`
  and the `note … end note` block form used to be parsed and thrown away, so the
  output was identical with or without them. Each note is now a yellow card
  (`style::NOTE_STYLE`) joined to its state by a dotted line, placed the way
  mermaid places it: `right of` one layer after the state along the flow, `left of`
  one layer before it. A note sits in its state's composite, and a note on a
  composite links to the composite's frame. Cards are ordinary nodes, so `scene`,
  `route`, `route_partial`, `hit_test` and dragging work on them. Edge tooltips call
  them "note". A card that lands wholly to one side of its state across the flow
  (in `LR`, a `note right of` card below the next state) is linked from the
  state's side facing it, so the dotted line stays out of the channel the
  state's own transitions and their labels use; when that elbow would cross
  one of the state's transitions (one running to a state beyond the card),
  the card keeps the ordinary link instead.
- **`layout::CHOICE_SIZE`** (28px), the side of an unlabelled diamond, public like
  `BASE_H` / `MIN_W` so embedders can reproduce node sizes.

### Changed

- **`<<choice>>` is a small 28×28 diamond**, mermaid's size, instead of an empty
  80×64.6 one. The rule applies to any diamond with no text (a flowchart `A{ }`
  too): the larger minimum only exists to fit a label. Diamonds with labels are
  unchanged.
- **Fork/join bars span their branches, and the branches drop straight.** A bar
  was a fixed 60×8 rect, so its branches to states about 120px apart were
  S-curves, and a fork's single incoming edge landed 18px off centre, in line
  with one branch. A bar is now stretched to reach every state on its busier
  side. Each branch meets it directly above or below its state, and a bar's
  single incoming or outgoing edge meets it at the middle. In `LR`/`RL` the bar
  stands upright, as in mermaid. The static scene and `route` / `route_partial`
  fit bars the same way, so dragging never resizes a bar. Fitting is
  idempotent: `fold::scene_compact` and a host handing `route_sized` the sizes
  it drew get the same bars, also when a bar is a branch of another bar
  (`f --> j`, nested forks, which now drop straight too). Applies to flowchart
  `@{ shape: fork }` nodes too. A composite state on a bar counts as a branch
  (sized from its members), and a note beside a bar does not: the bar centres
  over its branches and the card sits beside them, joined to the bar's end.
- **A `<<choice>>`'s branch labels sit by their targets, in rows.** The small
  diamond's branches leave it a few px apart, so a label kept at the middle of
  its line sat on a sibling's line and read as the wrong transition. A choice's
  labels (any unlabelled diamond's, a flowchart `A{ }` too) now stack in rows
  across the far part of their lines, where the branches have spread apart:
  each on its own line, clear of the other lines, and at least 20px from a
  sibling beside it. The gap after a choice grows just enough for the rows.
  Two transitions to one state, whose lines run side by side, put their labels
  just beside their lines, one each side, also among more branches. In
  `TB`/`BT` such a state is spaced for a label either side of the pair, and
  labels too wide for any row spread their states apart so they sit side by
  side, as in mermaid. A choice dragged so close to its targets that the rows
  run out steps its labels beside their lines. Where even that finds no clean
  spot, a label can still touch a sibling's line: in `LR`/`RL`, a label longer
  than the stretch where the branches have spread apart (its states are not
  spread for it), or after a drag toward the targets; and with three
  transitions to one state, whose lines run too close for a label between
  them, the middle label lies over a neighbour's line. The static scene,
  `route` and `route_partial` agree. Flowcharts without such a diamond are
  unchanged.
- **Tooltips for choices and bars name the state.** A `<<choice>>` now has a
  `<title>` (its id, where it had none), and fork/join bars are titled by id
  (`forked`, `joined`) instead of "fork/join".

### Fixed

- **Parallel edges no longer cross where their ends converge.** Two edges
  between one pair whose ports bunch up at one end (the lean pins both to a
  box's corner, or they leave a small diamond at almost one point) head the
  other way round seen from there, so spacing that end's ports in heading order
  swapped them: the lines crossed and the router then rerouted one round the
  far node's side (in `LR`, over its top). Port spacing now keeps parallel
  edges in the order they are drawn in; only the final reorder, which uncrosses
  legs, still places them by heading.
- **`note "text" as N` no longer swallows the rest of a state diagram.** It used
  to start a block note that ran to the end of the file, so every later line
  vanished. It is now a free-standing note card named `N`. mermaid drops it
  entirely. A block note with no `end note` shows the lines it swallowed, so the
  mistake can be seen.
