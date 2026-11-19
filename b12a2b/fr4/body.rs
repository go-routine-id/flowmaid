        let mut base_acc: Option<Score> = None;
        let can_trade = base0.node == 0 && (base0.frame > 0 || base0.chip > 0);
        // The original's border ride, once a candidate needs it.
        let base_border = std::cell::OnceCell::new();
        // The narrow channel counts in the static scene only: a drag's
        // reroute is searched in a small window and budget, and the
        // curve it would keep often crosses the frame (G13 over G4).
        let squeeze = |pts: &[Pt]| Squeeze {
            narrow: if mode == Mode::Static {
                narrow_px(&w, info, pts)
            } else {
                0.0
            },
            border: border_px(&w.frames, pts),
            base_border: *base_border.get_or_init(|| border_px(&w.frames, &polys[i])),
        };
        let mut best: Option<(Score, Vec<Pt>, Vec<Pt>)> = None;
        // A clean, monotone route beats an original that hits a node or
        // frame whatever the other edges say (the score is ordered), so
        // neither is scored against them.
        let mut decided = false;
        // The builder's ports are free (every other side costs
        // [`SIDE_SWITCH_COST`]), and the path never steps back along the
        // flow (G8) unless the edge already does.
        // Inside a window round the ends, any side, clear of every band
        // and foreign frame.
        let first = Try {
            lat: 0,
            alt: true,
            soft: false,
            soft_walls: false,
            reach: ROUTE_WINDOW,
        };
        // Its smoothed curve still touches something: a wider clearance.
        let wider = Try { lat: 1, ..first };
        // In a block layout's static scene ([`Ctx::blocks`]), after the
        // priced search below found only a way through a foreign frame:
        // still clear of every band and foreign frame, over the whole
        // drawing. A run through a block that holds neither end is what
        // the block layout exists to avoid (spec 4.3 f), and the way
        // round a block lies past the block's far side, beyond the
        // window round the ends (bench-75's `gw -> logs`, spilled to the
        // bottom sides of both ends: the window found no way into `logs`
        // from below but through `metrics`, and the priced search then
        // ran it straight through `app` and `data`).
        let round = Try {
            reach: work.walls_reach,
            ..first
        };
        let round_on = cx.blocks && mode == Mode::Static;
        // No such path: over a wider window (the whole drawing when
        // static), bands and frames priced.
        let walls = Try {
            soft_walls: true,
            reach: work.walls_reach,
            ..first
        };
        // Last, for an edge that wiggles already: steps back priced too.
        let back = Try {
            soft: true,
            ..walls
        };
        let mut n_found = 0;
        let mut built: Built = Vec::new();
        // (Each try with whether it is `round`.)
        let mut tries = vec![(first, false), (wider, false), (walls, false)];
        if round_on {
            tries.push((round, true));
        }
        tries.push((back, false));
        for (t, is_round) in tries {
            // (The whole-drawing strict try only where the priced search
            // found a way through a foreign frame that would replace the
            // original, one through a node: an original that only touches
            // a title chip keeps its course, as before, rather than take
            // a long way round across a dozen lines.)
            if is_round
                && !best.as_ref().is_some_and(|b| {
                    b.0.frame > 0
                        && (b.0.node, b.0.frame, b.0.chip) < (base0.node, base0.frame, base0.chip)
                })
            {
                continue;
            }
            // (A frame the path still enters is no matter of clearance: in
            // `route()` a wider lattice is not tried for it.)
            if t.lat == 1
                && !best.as_ref().is_some_and(|b| {
                    b.0.node + b.0.chip > 0
                        || b.0.near > 0
                        || mode == Mode::Static
                        || b.0.frame > 0 && b.0.frame < base0.frame
                })
            {
                continue; // nothing to widen, or nothing it touches
            }
            // A clear path exists (in `route()`, any path: a drag cannot
            // afford the wide window for a run beside another edge).
            let enough = |b: &(Score, Vec<Pt>, Vec<Pt>)| {
                b.0.hard() == 0 && (b.0.overlap == 0 || mode == Mode::Routed)
            };
            if t.soft_walls
                && !t.soft
                && (n_found > 0 && mode == Mode::Routed || best.as_ref().is_some_and(enough))
            {
                continue;
            }
            // In `route()` an edge that only enters a frame keeps it for
            // the channel detour below: when no strict path got by, the
            // priced one crosses a frame too, at the cost of a wide
            // window a drag cannot spare (G13).
            if t.soft_walls && mode == Mode::Routed && base0.node + base0.chip == 0 {
                continue;
            }
            if t.soft && base0.wiggle == 0 {
                continue; // a backtrack would add a wiggle (G8)
            }
            if work.left == 0 {
                break; // the detour below still runs
            }
            let e = &sc.edges[i];
            let mut env = Env {
                w: &w,
                taken: &taken,
                tracks: mode == Mode::Static,
                acc: &mut accepted,
                built: &mut built,
                work: &mut work,
                sc: &mut scratch,
                blocks: cx.blocks,
            };
            let Some(corners) = route_edge(&mut env, info, i, e, t) else {
                continue;
            };
            n_found += 1;
            let pts = drawn_with(e, &smooth(&corners, CORNER_R));
            // Charged by what the scoring looked at, even when that
            // overdraws: the search is paid for, so the candidate is
            // still scored, and the next check stops the repair. One
            // candidate's scoring is the most a repair goes over its
            // budget.
            let s = w.score(info, i, &pts, Some(&mut accepted));
            let _ = work.take(w.spent());
            let clean = base0.hard() > 0 && s.hard() == 0 && s.wiggle == 0;
            // Clean of the scene and on top of no accepted edge (G1): taken
            // at once; otherwise it still beats the original below, but a
            // later try may find one that is.
            let lone = s.overlap == 0;
            // A candidate the original keeps its curve over is passed
            // by: a later try may find a clear channel.
            let traded = clean
                && lone
                && can_trade
                && trades(
                    base_acc.get_or_insert_with(|| {
                        w.against(info, i, &simplify(&polys[i]), &mut accepted, base0, true)
                    }),
                    &s,
                    squeeze(&pts),
                );
            if traded {
                continue;
            }
            if clean && lone {
                best = Some((s, corners, pts));
                decided = true;
                break;
            }
            if best.as_ref().map_or(true, |b| s < b.0) {
                best = Some((s, corners, pts));
            }
            // A soft edge in the static scene (cramped, or riding a
            // border, nothing broken) searches on while what it found
            // does not beat its original: the wider lattice may
            // (bench-75's `svc4 -> logs` rode the app frame's border for
            // 99 px because the first candidate crossed one more edge at
            // its arrowhead, and the search stopped there).
            let beats = class == 0
                || mode == Mode::Routed
                || &s
                    < base_acc.get_or_insert_with(|| {
                        w.against(info, i, &simplify(&polys[i]), &mut accepted, base0, true)
                    });
            if s.hard() == 0 && s.wiggle == 0 && lone && beats {
                break;
            }
        }
        // Still through a node or frame: the channel detour, when it
        // hits fewer of them (G6) and adds no wiggle.
        let fixed = |s: &Score| (s.node, s.frame, s.chip) < (base0.node, base0.frame, base0.chip);
        if !decided && base0.hard() > 0 && !best.as_ref().is_some_and(|b| fixed(&b.0)) {
            let mut mine = detour_work.min(DETOUR_EDGE_WORK);
            let found = detour(&w, info, i, &sc.edges[i], &mut accepted, &mut mine, false);
            detour_work -= detour_work.min(DETOUR_EDGE_WORK) - mine;
            if let Some((s, corners, pts)) = found {
                let trade = can_trade
                    && trades(
                        base_acc.get_or_insert_with(|| {
                            w.against(info, i, &simplify(&polys[i]), &mut accepted, base0, true)
                        }),
                        &s,
                        squeeze(&pts),
                    );
                // On top of another edge for over MIN_OVERLAP_RUN (G1):
                // not the clear channel G6 asks for. An original that
                // only crosses frames keeps its curve; one through a node
                // or a title chip still takes it (hiding a node or a
                // frame's name is worse). See [`frames_hard`].
                let stacked =
                    s.overlap > 0 && base0.node + base0.chip == 0 && !frames_hard(&s, &base0);
                if fixed(&s) && s.wiggle <= base0.wiggle && !trade && !stacked {
                    best = Some((s, corners, pts));
                    decided = true;
                }
            }
        }
        if !decided && best.is_none() {
            continue; // nothing to compare: the edge keeps its geometry
        }
        let base = if decided {
            base0
        } else {
            base_acc.unwrap_or_else(|| {
                w.against(info, i, &simplify(&polys[i]), &mut accepted, base0, true)
            })
        };
        let _ = work.take(w.spent());
        let better = |s: &Score, pts: &[Pt]| {
            if decided {
                return true;
            }
            // Never add a wiggle (G8), nor squeeze along a border where
            // the original only crossed a frame's empty area.
            s.wiggle <= base.wiggle
                && !trades(&base, s, squeeze(pts))
                // Never trade a frame crossing for a run on top of
                // another edge (G1 over G6, see the detour above), but
                // in `route()` (see [`frames_hard`]).
                && !(s.overlap > base.overlap
                    && base.node + base.chip == 0
                    && !frames_hard(s, &base))
                // A static curve that hides no node or chip is kept over a
                // reroute that rides along a node it cramps, or along a
                // side while crossing another edge at its arrowhead (G4,
                // G11; a shape worse than the curve), unless the reroute
                // also takes it off a long run on top of another edge
                // (G1).
                && !(mode == Mode::Static
                    && base.node + base.chip == 0
                    && base.overlap <= s.overlap + FRAME_OVERLAP_TRADE
                    && (s.near > base.near || s.head > base.head)
                    && s.hug as f64 > base.hug as f64 + HUG_TRADE)
                && if class == 0 {
                    s < &base
                } else {
                    s.hard() == 0 && s.overlap == 0 && s < &base
                }
        };
