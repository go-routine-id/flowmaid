<!-- Work package A5 (gitGraph). Merge these bullets into `## [Unreleased]`
     under the matching Keep-a-Changelog headings. -->

### Added

- **More of mermaid's gitGraph syntax parses:** the documented header forms
  `gitGraph TB:` / `gitGraph LR:` / `gitGraph BT:` and a bare `gitGraph:`
  (previously "unknown gitGraph orientation: 'TB:'"); `branch name order: n`;
  `merge name type: NORMAL|REVERSE|HIGHLIGHT`; quoted branch references
  (`branch "my topic"`, `checkout "my topic"`, `merge "my topic" id: "m"`);
  and commit messages (`commit msg: "…"` or `commit "…"`), read and not drawn,
  as in mermaid.
- **`gitgraph::REVERSE_MARK`**: the label a REVERSE commit's scene node
  carries, so a host that draws node labels shows the commit crossed out.

### Changed

- **gitGraph looks like a git graph, not a draft.** Every line is drawn in
  the colour of the lane it runs along, 5px wide with round caps (it was a
  2px navy thread for every edge). Forks and merges are rounded elbows, as in
  mermaid: a fork drops straight from its parent and runs along the new
  branch, and a merge runs along its branch and climbs straight into the merge
  commit. The old curves hugged the source lane and arrived on a 38-72° slant.
  When a commit sits on the stretch the elbow would run along (main merged
  into a branch that has committed since), the elbow takes the other lane
  instead of running through that commit. Two legs that leave one commit on
  the same side run side by side instead of on top of each other, so a merge
  in and a fork out of the same commit stay distinct.
- **Commit glyphs:** a merge commit is a ring around a light core, REVERSE a
  filled disc crossed out with an X, HIGHLIGHT a square outlined in ink. As in
  mermaid, the commit type decides the glyph before merge-ness does. For
  generic `Scene` painters the glyph is the node shape: `DoubleCircle` for a
  merge, a square `Rect` for HIGHLIGHT, and a `Circle` labelled
  `REVERSE_MARK` for REVERSE.
- **Labels never sit on a line.** A commit id or tag that its own commit's leg
  would cross steps aside of it, and ids are drawn over a white halo. In LR,
  ids are measured: each column sits just far enough right that no id, tag or
  leg reaches into the one before it (`COMMIT_DX` is now the 50px minimum
  instead of a fixed 90px). Short ids pack tighter than before, and long ids
  no longer run together. In TB/BT, commit ids sit left of their commit and
  tags right of it (it used to be the other way round), and each lane sits far
  enough over to fit the ids beside it. The canvas is fitted around everything
  drawn (`PAD` 24px), so a long last id or a TB tag is never clipped.
- **Tags are luggage labels** (12px text in the lane's ink on a white pill
  with a hole). They used to be bold 13px text, styled exactly like the branch
  names. **Branch names sit on pills in the lane's colour**, with white or dark
  text, whichever reads on the pill. A dashed guide runs along every lane.
- **`GitBranch::order` is the lane rank** after `order:` options are applied
  (it was always the creation index). The renderer ranks branches by it, ties
  by index, so a hand-built graph may use any increasing numbers. Colours
  follow the lane.
- Scene edges carry their colour and width in `meta.style`, and an elbow
  carries its corner as both `bezier` control points plus a dense rounded
  `waypoints` polyline that a host's spline reproduces. Merge edges are now
  `EdgeKind::Open`: the scene said `Arrow` but the SVG never drew an
  arrowhead, and mermaid draws none. Commit dots are r=11 (was 12).

### Fixed

- **A REVERSE commit no longer reads as a merge.** It was drawn as a white
  hollow ring, which is mermaid's merge marker (report P0 #15).
- **Merge curves no longer strike commit ids.** "merge-hotfix" was crossed out
  mid-word on the comparison fixture.
- **`branch dev order: 2`** created a branch literally named "dev order: 2".
- **`merge x type: HIGHLIGHT`** parsed, but the type was dropped.
- **TB/BT tags on the first lane** hung off the left edge of the canvas, and
  LR tags sat about 2px from its top.
- **A branch name longer than about 9 characters** ran into the first commit
  column. The name column now widens to fit the name.
