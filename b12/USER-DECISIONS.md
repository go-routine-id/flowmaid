# B12: the user's answers to the spec's open questions (2026-11-10 ~18:55 UTC)

Asked two questions (D5, D3); the user answered both with the bar itself:
> "pokoknya mirip atau lebih bagus dan rapi dari mermaid js 12" — it must look like, or better and tidier than, mermaid 12.
> (second question) "seperti jawabanku sebelumnya" — the same answer.

What follows from it (the lead's reading; apply to SPEC.md):
- **Acceptance bar = mermaid 12.2 itself**, not "no worse than 0.38.0": on the analog and on infra-overview-v2 every
  block-shape number must be at least as good as mermaid's within a stated small tolerance, crossings must come within a
  small stated margin of mermaid's (16), no label struck, no crowded labels beyond mermaid's count, edges enter through
  flow-facing sides as mermaid's do. Where flowmaid can beat mermaid (area, edge length, label distance) keep that.
  A gate that only matches 0.38.0 (e.g. "crossings <= 24") is not enough for the analog; state the tighter gate and the
  stop rule if it cannot be met.
- **D3 (upstream edges between blocks): channels** (leave the top of a node, run straight up between blocks, as mermaid
  draws them) — the tidier option on the evidence (20 crossings against 34).
- **D5 (the analog in the public repo): not granted explicitly.** Keep `b12/in/infra-overview.mmd` and `-v2` as LOCAL
  acceptance inputs only (workspace/backup). For the repository's test fixture and README image write a synthetic
  diagram with the same kind of difficulty but a different structure (different block count, sizes and links).
- **D2 (split): yes** — B12a (structure and route(), behind a switch, nothing visible, merged to dev, no release) then
  B12b (edges, labels, default on) → release 0.41.0. The user wants a release at every phase; B12a has nothing to show.
- **D1:** static limit for files under 1 ms may be max(2 x baseline, baseline + 0.3 ms) (the lead's own budget, sub-ms).
- **D4:** the lane rule's principle is accepted; the constant comes from the sweep; such diagrams must not get worse.
- **D6:** on a stop rule the implementer stops and reports; gates are not re-baselined to pass.
