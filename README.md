# flowmaid mermaid-parity workspace backup

Scratch workspace of the mermaid-parity effort (session_01JGDFJkgB3ttBJtcTtc6aL5), saved so work can resume in a fresh container.
Not part of the crate. Restore into the session scratchpad: `cp -r . $SCRATCHPAD/` (tools expect
`S=/tmp/claude-0/-home-user-flowmaid/<session-id>/scratchpad`; adjust the `S=` line in tools/*.sh|*.js if the path differs).

- PLAN.md — invariants every package must keep; report.md / style.md / perdiagram.json — verified comparison vs mermaid 12.2.0
- phaseB.draft.js — the phase B workflow script (12 flowchart packages B0..B11)
- tools/ — guard.sh, render.sh (+render3.js), perf.sh (+perfcmp.py), fmrender.sh, mmrender.js, mmbench.js
- cmp/corpus, flowcorpus — inputs; mermaid-ref/ — mermaid 12.2.0 SVGs; baseline/ — flowmaid 0.32 (5f318a0) SVGs + perf.tsv
- prototypes/*.src.patch — prototype fixes as patches against 5f318a0 (rv_be = back-edge lanes, rv_adv_skeptic = nested subgraph, rev_ll = label gap/bias)
- ixcheck/ — interactive-API drag/pick smoke test crate (path dep `FM` → repo)
- mermaid itself: `npm i mermaid@12.2.0` into mm/ (mmrender.js expects mm/node_modules/mermaid/dist/mermaid.min.js)
