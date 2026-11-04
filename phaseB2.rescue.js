const results = [{ id: 'b0-metrics', status: 'kept (previous run, merged to dev via PR #84)' }]
phase('Packages')

// ---- B1 rescue: the package passed everything but one blocking item after 3 verifications ----
{
  const B1 = PKGS.find(p => p.id === 'b1-arrows')
  const B1PRE = '519b38c240308e0f72433e95d121ca9729210092'
  const scope = `

SCOPE DECIDED BY THE LEAD after three verifications (the package is otherwise accepted):
- Legitimate deferrals (list them in items_skipped / do not count them as failures): elbows at the head base of strongly turning single S-cubics (belongs to B10 all-orthogonal edges / edge-shape work, report §2.1); routed ports-stage end angles (B7 ports); the router's longer end stub (it moved dense routes; dropped); to_svg slowdown on large synthetic graphs within the 2x static budget.
- Must hold: no painted ink outside the SVG canvas anywhere (static scene(), route()/route_sized() fed static centres, drag / route_partial scenes, examples/*.mmd, state); a curve shorter than the trim never draws its head past the curve end into the target node; very wide linkStyle strokes end cleanly at the head; everything the earlier rounds fixed stays fixed (thick head == normal head, tips on borders, square heads, picking and to_svg speed, crossing gaps clear of heads, crowded heads apart, advance heads).`
  const B1R = { ...B1, spec: B1.spec + scope }
  const rescueExtra = `RESCUE ROUND for B1. The package (commits fb3af47..05d93b3 on top of ${B1PRE}) passed everything in its third independent verification except one blocking item. Fix with new commits (no amend/rebase):
A. BLOCKING — edges clipped off the canvas (verifier item 1 below). Make the canvas cover every drawn edge: waypoint splines and routes, the painter's squared ends and arrowheads plus half the stroke width, edge labels, frames — then pad SVG_MARGIN (16). Do it consistently for scene() and route()/route_sized()/route_partial() (grow_scene and/or to_svg's viewBox); Scene width/height and node/edge coordinates must stay consistent with the SVG viewBox that to_svg emits, so hosts drawing the scene see the same canvas. Add a regression test that, for every fixture (corpus, examples/*.mmd, state corpus, the verifier's x3_long-style long detour, a few random graphs) in static AND routed mode, every coordinate of every painted path and every marker tip lies inside the viewBox; and add an ink-outside-canvas count to the quality metrics with a gate of 0. Simplest robust option wins; keeping the painted padding as wide as the router's gutter is acceptable only if that test proves nothing clips (and then say so in CHANGELOG instead of 'MARGIN 16').
B. Verifier item 5: when a curve is shorter than the trim (+1 px), never draw the head past the curve's end into the target node (shrink/omit the head or shorten consistently); keep pick == render.
C. Verifier item 4: a linkStyle stroke wider than the head must not end in a butt wider than / notched beside the head (scale the head with the stroke above the head width, or cut deeper and square the end), if local.
Leave items 2, 3 and 6 as deferred (see scope). Run guard and perf as usual; update CHANGELOG [Unreleased] (Q10 wording must match what was really built).

The last verifier's full findings, for context:
${args.b1problems}

Unfixed items it listed:
${args.b1unfixed}`
  let impl = await agent(implPrompt(B1R, rescueExtra), { label: 'fix:b1-arrows#3', phase: 'Packages', schema: IMPL })
  let ver = null
  for (let round = 0; round < 2; round++) {
    ver = await agent(verifyPrompt(B1R, B1PRE, impl), { label: `verify:b1-arrows#${round + 3}`, phase: 'Packages', schema: VERDICT })
    if (!ver || ver.verdict === 'pass' || round === 1) break
    log(`b1-arrows rescue: verifier found ${ver.problems.length} problem(s), one more fix round`)
    impl = await agent(implPrompt(B1R, ver.problems.map((p, i) => `${i + 1}. ${p}`).join('\n') + (ver.unfixed_items.length ? `\nUnfixed spec items: ${ver.unfixed_items.join('; ')}` : '')), { label: 'fix:b1-arrows#4', phase: 'Packages', schema: IMPL })
  }
  if (ver && ver.verdict === 'pass') {
    await agent(`Run in ${REPO}: \`git push -u origin ${BR}\`. If it fails with a network error, retry up to 4 times waiting 2s, 4s, 8s, 16s between tries. Report the final output. Do nothing else.`, { label: 'push:b1-arrows', phase: 'Packages', effort: 'low' })
    results.push({ id: 'b1-arrows', status: 'kept (after rescue)', done: impl && impl.items_done, skipped: impl && impl.items_skipped, notes: ver.notes })
  } else {
    await agent(`The flowmaid package "b1-arrows" failed its rescue verification. Preserve and revert it: in ${REPO} run \`git branch -f failed/b1-arrows-r4 HEAD && git reset --hard ${B1PRE} && git log --oneline -1\` (only if HEAD differs from ${B1PRE}; never touch commits at or before ${B1PRE}). Report the output. Do nothing else.`, { label: 'revert:b1-arrows', phase: 'Packages', effort: 'low' })
    results.push({ id: 'b1-arrows', status: 'reverted (saved on failed/b1-arrows-r4)', problems: ver && ver.problems, unfixed: ver && ver.unfixed_items })
  }
  log(`b1-arrows: ${results[results.length - 1].status}`)
}
