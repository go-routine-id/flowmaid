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

## After the second review pass (2026-11-10 ~22:00 UTC), the lead's decisions (also at the top of SPEC.md section 0)
- **D7 confirmed (option b):** covered by the user's own choice "Default untuk semua" (2026-11-10 ~04:45 UTC: every
  diagram with subgraphs takes the block layout) and by the bar.
- `lateral_x` on bench-75 / dense-lr / dense-td: must = not above mermaid's count (3 / 0 / 1), target 0.
- Four workflow packages: b12a1-structure (B12.0-B12.6), b12a2-geometry (B12.7-B12.12), b12a3-route (B12.13),
  b12b-default (B12.14-B12.16). The first three: switch off, no release. 0.41.0 after the fourth.

## Order and release number (user decision, 2026-11-11 ~00:15 UTC)
- The user asked why B12 came after c-leftovers and chose **"Tukar: B12 dulu"**: B12 runs right after B11.
  Order: B11 → release 0.39.0 → B12 (four packages) → release **0.40.0** → c-leftovers → release 0.41.0.
  Every earlier mention of "0.41.0 after B12b" in this file is superseded by this entry.

## Side by side (user, 2026-11-11 ~00:50 UTC: "harusnya bisa paralel kan?")
- B12 (main repo) and the class/state leftovers package (worktree, branch `ccr-ccc6d89a-5cxd7e-par`) run at the same
  time, both starting on B11's head. B12 still starts the moment B11 passes. Release numbers follow the order in which
  the packages pass (B12's release is 0.40.0 or 0.41.0). The leftovers skip the two block-placement items.

## README (user, 2026-11-11 ~03:20 UTC)
- "readme sekarang terlalu banyak bertele-tele padahal penjelasan basic yang sudah ada di mermaid malah di jelasin lagi,
  harusnya tidak perlu": the README must not restate standard mermaid syntax (link to mermaid's docs instead) and must
  be short and plain. Applies to every later README edit too (b12b-default's README text: say what blocks change for
  the user in a few lines, no syntax lessons, no internal narrative).
