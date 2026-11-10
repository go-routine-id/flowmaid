export const meta = {
  name: 'b12-design',
  description: 'Design package B12 "subgraphs as blocks": understand flowmaid and mermaid, three independent designs with prototypes, judge panel, spec, adversarial critique',
  phases: [
    { title: 'Understand', detail: 'flowmaid today, how mermaid places subgraphs, block metrics + baseline' },
    { title: 'Design', detail: 'three independent designs, each with a prototype render of the analog' },
    { title: 'Judge', detail: 'visual, engineering and invariants lenses' },
    { title: 'Spec', detail: 'synthesize SPEC.md, critique it, revise' },
  ],
}

const S = args.scratch
const TREE = `${S}/b12/tree`

const CONTEXT = `CONTEXT
flowmaid is a pure-Rust (std only, zero dependencies, MSRV 1.75, wasm) mermaid-like diagram engine. Read ${S}/PLAN.md first: its invariants are non-negotiable (the interactive scene / route / route_partial / hit-test API and its drag speed, fold/compact, to_mermaid, the advance module, determinism, portable SVG, zero deps).
Source tree for this design work, READ-ONLY: ${TREE} (a git worktree at dev = release 0.38.0, commit 0366084). Do NOT touch /home/user/flowmaid: another package (B11: crossing hops + comb bundling for fans) is being implemented there right now. Do not commit or push anything anywhere, and do not edit files under ${TREE}.
Scratch space: ${S}/b12/<your label>/ (create it). For builds, copy the tree (cp -r ${TREE} ${S}/b12/<your label>/tree; there is no target/ in it) and use CARGO_TARGET_DIR=${S}/b12/target-<your label>; build release only, only what you need, and DELETE that target dir when you are done. Disk is limited: run df first and stop building if under 5 GB is free. The box is shared with a perf-sensitive job, so avoid long CPU-heavy loops and never run two cargo builds at once yourself.
Tools: a prebuilt 0.38.0 CLI ${S}/b12/fm038 (\`${S}/b12/fm038 in.mmd -o out.svg\`); SVG to PNG \`cd ${S}/b12 && node shot.js <in.svg> <out.png>\` (then Read the PNG to look at it); mermaid 12.2 renders \`node ${S}/tools/mmrender.js <outdir> <files.mmd...>\` (writes <name>.mermaid.svg and .png with mermaid's default config; copy and adapt the script if you need another config). Corpus inputs: ${TREE}/examples/*.mmd, ${TREE}/tests/fixtures/quality/*.mmd, ${S}/cmp/corpus/*.mmd (mermaid reference PNGs in ${S}/cmp/out/<name>.mermaid.png).

THE PROBLEM (user feedback on 0.38.0, 2026-11-10)
The user rendered a private architecture diagram in the flowmaid playground and in mermaid 12.2 and said mermaid is still far tidier. It is a \`flowchart TD\` with 3 top-level actor nodes, 5 subgraphs (code hosting: 2 nodes; external services: 5; three servers: 3, 5 and 4 nodes) and 27 edges, most of them labelled, many between subgraphs, three of them from a node to a whole subgraph.
- Screenshots: ${S}/b12/private/user-shot-flowmaid038.png and ${S}/b12/private/user-shot-mermaid122.png. They are PRIVATE (real hostnames and IPs): look at them, but never copy them, or any name or IP you read in them, into any file outside ${S}/b12/private/.
- A sanitized analog with the same structure, which reproduces the problem: ${S}/b12/in/infra-overview.mmd (its 0.38.0 render: ${S}/b12/infra-overview.fm038.png). Use the analog for everything you write down.
What differs between the two renders:
- mermaid lays every subgraph out as ONE COMPACT BLOCK and stacks the blocks along the flow: actors, then external services (one row), then the three servers one under another; edges between blocks run in the gaps between them.
- flowmaid ranks every node on its own: the "External services" frame spans the whole drawing height (the DNS node at the top, the chat bot at the bottom), the three server frames sit side by side, long horizontal runs cross frames they do not belong to, and about eight parallel lines share one corridor with their labels far from both ends.
- flowmaid already lays a subgraph out on its own (src/compound.rs, package B9), but only one that has its own \`direction\` and no edge across its border.
USER DECISIONS: a new package B12 "subgraphs as blocks". It is the DEFAULT for every flowchart and state diagram with subgraphs (not only \`flowchart-elk\`). Diagrams without subgraphs must not change. It runs after B11 and a class/state leftovers package. Quality bar, in the user's words: "tidak main-main dan premium" (rare edge cases still matter).`

const DOC = {
  type: 'object',
  properties: {
    file: { type: 'string', description: 'absolute path of the document you wrote' },
    summary: { type: 'string', description: 'the findings a designer must know, in under 400 words' },
    key_points: { type: 'array', items: { type: 'string' } },
    other_files: { type: 'array', items: { type: 'string' } },
  },
  required: ['file', 'summary', 'key_points'],
}
const DESIGN = {
  type: 'object',
  properties: {
    file: { type: 'string' },
    summary: { type: 'string', description: 'the algorithm in under 400 words' },
    prototype_images: { type: 'array', items: { type: 'string' }, description: 'PNG paths of prototype renders (the analog first)' },
    prototype_kind: { type: 'string', description: 'what the prototype really is: full code hack, placement-only, hand-computed sketch, none' },
    metrics: { type: 'string', description: 'numbers from the block-metrics tool for the prototype vs 0.38.0 and mermaid, or why none' },
    code_touch: { type: 'array', items: { type: 'string' }, description: 'functions/files to change, with file:line' },
    risks: { type: 'array', items: { type: 'string' } },
  },
  required: ['file', 'summary', 'prototype_images', 'prototype_kind', 'metrics', 'code_touch', 'risks'],
}
const SCORE = {
  type: 'object',
  properties: {
    scores: { type: 'array', items: { type: 'object', properties: {
      design: { type: 'string' }, visual: { type: 'number' }, corpus: { type: 'number' }, invariants: { type: 'number' }, risk: { type: 'number' }, perf: { type: 'number' }, total: { type: 'number' }, why: { type: 'string' } },
      required: ['design', 'visual', 'corpus', 'invariants', 'risk', 'perf', 'total', 'why'] } },
    winner: { type: 'string' },
    graft: { type: 'array', items: { type: 'string' }, description: 'ideas from the other designs worth grafting onto the winner' },
    blocking_flaws: { type: 'array', items: { type: 'string' }, description: 'flaws in the winner that the spec must resolve, with evidence' },
  },
  required: ['scores', 'winner', 'graft', 'blocking_flaws'],
}
const SPEC = {
  type: 'object',
  properties: {
    spec_file: { type: 'string' },
    pkg_spec: { type: 'string', description: 'a short package spec (under 250 words) for the workflow PKGS list; it must tell the implementer to read the spec file first' },
    changes: { type: 'array', items: { type: 'string' } },
    open_questions: { type: 'array', items: { type: 'string' }, description: 'decisions only the user can take, if any' },
  },
  required: ['spec_file', 'pkg_spec', 'changes', 'open_questions'],
}
const CRIT = {
  type: 'object',
  properties: {
    findings: { type: 'array', items: { type: 'object', properties: {
      where: { type: 'string' }, problem: { type: 'string' }, evidence: { type: 'string' }, severity: { type: 'string', enum: ['blocking', 'non-blocking'] }, fix: { type: 'string' } },
      required: ['where', 'problem', 'evidence', 'severity', 'fix'] } },
    verdict: { type: 'string' },
  },
  required: ['findings', 'verdict'],
}

phase('Understand')
const UNDERSTAND = [
  { key: 'flowmaid', prompt: `${CONTEXT}

YOUR TASK (label: u-flowmaid): map how flowmaid lays out and draws a flowchart WITH subgraphs today, and what any new block layout must respect. Write ${S}/b12/understand-flowmaid.md with file:line references into ${TREE}/src:
1. The pipeline for a clustered flowchart, stage by stage: parser (subgraphs, sub_edges), node_cluster_paths, sub_edge_proxies and stand-in edges, layout (layout_clustered_ranked / layout_core / place: ranking, cluster handling in ordering and in coordinate assignment, header slots, gap needs), cluster frames (cluster_raw_boxes, frame growth), the edge stages (edgepass::finish: ports, ortho, router, lanes, labels), SVG. Say exactly which stage decides (a) a node's rank, (b) whether a subgraph's members stay contiguous, (c) where sibling subgraphs sit relative to each other.
2. Explain, from the code, why the analog looks as it does: why "External services" spans the full height, why the three servers end up side by side, where the long horizontal runs come from. Dump ranks/orders if you need to (in a tree copy).
3. src/compound.rs (B9): split(), blocks, proxies, join, refit, and the route / route_partial paths; its preconditions; precisely what stops it from handling a subgraph with edges across its border.
4. The interactive contract: what scene(), route(), route_sized(), route_partial(), hit_test and fold/compact guarantee for clustered diagrams (frames derived from member positions while dragging? scene() equal to route() at the scene's own positions? index-parallel SceneEdge?), which tests pin that, and the perf budgets (PLAN.md invariant 1).
5. What pins today's layouts: snapshot tests, quality gates and ratchets (tests/fixtures/quality, GAP_FLOORS and the like), README images; which corpus/example/fixture inputs have subgraphs or state composites, and which of those have edges across a subgraph border (those are the ones B12 changes).
6. A list of hard constraints and soft preferences for a block layout design, each tied to code or a test.`, label: 'u-flowmaid' },
  { key: 'mermaid', prompt: `${CONTEXT}

YOUR TASK (label: u-mermaid): establish exactly how mermaid 12.2 places subgraphs, as the reference. Write ${S}/b12/understand-mermaid.md:
1. Which layout produced the user's mermaid screenshot: dagre (mermaid's default) or ELK? Render the analog with mermaid's default config and with the ELK layout (try front-matter \`config: layout: elk\`, \`flowchart-elk\`, and whatever the bundled build in ${S}/mm/node_modules/mermaid needs; elkjs is in ${S}/mm/node_modules) and compare the block arrangement and edge style with the screenshot. Keep the renders in ${S}/b12/u-mermaid/.
2. How that layout handles subgraphs, from mermaid's source in ${S}/mm/node_modules (dist chunks) and from dagre/ELK behaviour: for dagre, mermaid's cluster handling (compound graph, nesting graph and border nodes, the recursive render of a cluster with no external edges, how an edge to a cluster is anchored, how \`direction\` inside a subgraph is treated); for ELK, the layered algorithm's hierarchy handling and the options mermaid's adapter sets. State the rules a re-implementation needs: how blocks are ordered along the flow, when two blocks sit side by side, how block-to-block and node-to-block edges are routed, where labels go.
3. Measure the mermaid SVGs with a small script: each cluster's rectangle and each node's box for the analog and for five corpus inputs with subgraphs (examples/advanced.mmd, examples/subgraph.mmd, a fanout-frames fixture, a state diagram with a composite, one more): block order, gaps between blocks, which edges run through which gaps. Give the tables.
4. What mermaid does badly on these inputs (labels far from their edges or on top of each other, edges through blocks, wasted space, unstable order) that flowmaid should NOT copy, with image evidence.`, label: 'u-mermaid' },
  { key: 'metrics', prompt: `${CONTEXT}

YOUR TASK (label: u-metrics): build the measuring tool the package will be judged with, and the baseline. Deliver:
1. ${S}/b12/metrics/blockmetrics (a node or python3 script, no external packages) that reads an SVG rendered by flowmaid (find how its SVG marks clusters, nodes, edges and labels: read ${TREE}/src/scene.rs to_svg and render a sample; if the SVG lacks what you need, also provide a tiny Rust example file that dumps a scene as JSON through the public API, kept in ${S}/b12/metrics/, to be compiled against a tree copy) and prints one TSV row per diagram with at least: canvas width, height, area; number of subgraph frames; per-frame compactness (frame area / sum of member box areas; report mean and max); count of sibling-frame pairs whose projections on the flow axis overlap (side by side) vs stacked; number of edge runs crossing a frame that holds neither end, and their total length inside such frames; edge crossings; total edge length; longest straight run; for labelled edges the distance from the label to the edge's midpoint as a fraction of the edge's length (mean and max) and labels overlapping another label or a foreign node.
2. The same metrics for mermaid's SVG of the same inputs where feasible (mermaid marks clusters, nodes and edge paths too), so the two engines can be compared on one table.
3. Run it on the analog and on every flowchart/state input with subgraphs or composites in ${TREE}/examples, ${TREE}/tests/fixtures/quality and ${S}/cmp/corpus: write ${S}/b12/metrics/baseline-038.tsv and ${S}/b12/metrics/mermaid.tsv, plus ${S}/b12/metrics/README.md (usage, column meanings, how each number was validated on a hand-checked case).
4. In your summary: the analog's numbers for flowmaid 0.38.0 vs mermaid, which corpus inputs have edges across a subgraph border, and proposed acceptance thresholds for B12 on the analog (what "as tidy as mermaid" means in these numbers).`, label: 'u-metrics' },
]
const und = (await parallel(UNDERSTAND.map(u => () => agent(u.prompt, { label: u.label, phase: 'Understand', schema: DOC }).then(r => r && ({ key: u.key, ...r }))))).filter(Boolean)
const undText = und.map(u => `- ${u.key}: ${u.file}\n  ${u.summary}`).join('\n')
log(`understand: ${und.length}/3 documents`)

phase('Design')
const READ_FIRST = `Read these first (written for you by three analysts):\n${undText}\nThe block-metrics tool is in ${S}/b12/metrics/ (README.md there).`
const ANGLES = [
  { key: 'recursive', title: 'recursive blocks (a generalised compound.rs)', angle: `Every subgraph is laid out on its own, bottom-up, and in its parent it is ONE node the size of its frame (as src/compound.rs does today for the narrow B9 case). The new part is edges across a border: decide how they are anchored and routed (ports on the block border chosen before or after the inner layout, or a route between placed blocks by the orthogonal router; node-to-block edges; edges between nested levels), how block-level cycles are broken, and how the inner layout learns which side its outward edges leave from.` },
  { key: 'global', title: 'one global layering with block constraints', angle: `Keep ONE Sugiyama pass over all nodes, but make ranking and ordering block-aware: build the block graph (top-level subgraphs and loose nodes as vertices), break its cycles, give each block a contiguous rank interval in block-topological order so that blocks stack along the flow (independent blocks may share an interval side by side), rank members inside their block's interval, keep every block contiguous in each layer's order; recursive for nested subgraphs. Decide how long edges between blocks get their channels and how frames stay compact.` },
  { key: 'hybrid', title: 'smallest change that gets there', angle: `Find the SMALLEST change to today's clustered pipeline that gives the mermaid arrangement on the analog and does not hurt the corpus: for example rank compaction per subgraph (pull a block's members onto adjacent ranks; a member with only outward edges takes the block's first rank), plus block-order constraints between sibling subgraphs, plus a measurable rule for when a subgraph is better left as today. Be honest about where this falls short of true block layout and what a follow-up would need.` },
]
const designs = (await parallel(ANGLES.map(a => () => agent(`${CONTEXT}

YOUR TASK (label: d-${a.key}): design package B12 from ONE angle, independently of the other designers: ${a.title}.
${a.angle}
${READ_FIRST}
Write ${S}/b12/design-${a.key}.md:
1. The algorithm, precisely: inputs, steps, tie-breaks (deterministic), and the rules for each case: nested subgraphs; an edge from a node to a whole subgraph, subgraph to subgraph, subgraph to its own member; a cycle between blocks; a node linked to many blocks (a hub such as the DNS node in the analog); an empty subgraph; a subgraph with its own \`direction\` (B9's blocks must keep working); LR/RL/BT diagrams; state diagrams (composites, notes, fork/join bars); invisible links; very large drawings (the router's 400-edge cap, the layout search budget).
2. The code plan against ${TREE}/src with file:line: which functions change, what is new, what compound.rs becomes; how scene(), route(), route_sized(), route_partial(), hit-testing and fold/compact stay consistent (route() takes the host's positions, so say what of the block logic runs there and what does not); the expected cost in ms on bench-75, dense-td and the analog.
3. A PROTOTYPE, as real as you can make it within about an hour of work: best is a hack in a tree copy that renders the analog (and examples/advanced.mmd, examples/subgraph.mmd and a fanout-frames fixture) so the result can be SEEN; second best is placement only (blocks and nodes in the new positions with today's routing); least is a computed sketch drawn as SVG rectangles. Save PNGs under ${S}/b12/d-${a.key}/, look at them yourself next to the mermaid screenshot, and say frankly what still looks worse than mermaid and why. Run the block-metrics tool on the prototype output and report the numbers against 0.38.0 and mermaid.
4. What this design does to diagrams WITHOUT cross-border edges and to diagrams without subgraphs (must be byte-identical), which pinned tests/gates it will change, and its risks.`, { label: `d-${a.key}`, phase: 'Design', schema: DESIGN }).then(r => r && ({ key: a.key, title: a.title, ...r }))))).filter(Boolean)
log(`designs: ${designs.map(d => `${d.key} (${d.prototype_kind})`).join(', ')}`)
if (!designs.length) return { error: 'no design returned', und }
const desText = designs.map(d => `- ${d.key} (${d.title}): ${d.file}\n  prototype: ${d.prototype_kind}; images: ${d.prototype_images.join(', ') || 'none'}\n  metrics: ${d.metrics}\n  ${d.summary}`).join('\n')

phase('Judge')
const LENSES = [
  { key: 'visual', lens: `You judge with your EYES. Read every prototype image, the 0.38.0 render of the analog, and the user's two screenshots, side by side. For each design: is the analog as tidy as mermaid's (blocks compact, stacked along the flow, edges in the gaps, labels near their edges, no run through a foreign frame)? What is still worse, what is better than mermaid? Then look at the other prototype renders (advanced, subgraph, fanout-frames) for anything that got uglier than 0.38.0.` },
  { key: 'engineering', lens: `You judge FEASIBILITY against the code. For each design verify its code plan in ${TREE}/src: do the named functions do what the design assumes, what did it overlook (sub-edge stand-ins, cluster header slots, the layout search and narrow(), ports/ortho/router stages, compound.rs, fold.rs, state diagrams riding the flowchart pipeline), how big and risky is the change, can it be delivered and verified as one package or does it need two steps?` },
  { key: 'invariants', lens: `You judge the INVARIANTS. For each design: does scene() stay consistent with route() / route_sized() / route_partial() and hit-testing (what happens on the first drag frame: do frames or edges jump?), does it keep the drag and static-scene budgets (PLAN.md invariant 1), determinism, diagrams without subgraphs byte-identical, class/ER untouched, to_mermaid round trip, fold/compact; which pinned tests and quality gates would it break, and is each such change justified?` },
]
const verdicts = (await parallel(LENSES.map(l => () => agent(`${CONTEXT}

YOUR TASK (label: j-${l.key}): you are one of three judges of three independent designs for package B12. ${l.lens}
Background documents:\n${undText}
The designs:\n${desText}
Score each design from 0 to 10 on: visual (the analog against mermaid), corpus (effect on other diagrams), invariants (drag, perf, determinism, contracts), risk (10 = small and safe), perf; total = your weighted judgement out of 10. Name the winner, the ideas from the others worth grafting onto it, and every flaw in the winner that the final spec must resolve, with evidence (file:line, image, numbers). Be skeptical: a design that only looks good in its own prose scores low; verify claims.`, { label: `j-${l.key}`, phase: 'Judge', schema: SCORE }).then(r => r && ({ lens: l.key, ...r }))))).filter(Boolean)
const tally = {}
for (const v of verdicts) for (const s of v.scores) { const k = s.design; tally[k] = (tally[k] || 0) + s.total }
log(`judges: ${Object.entries(tally).map(([k, t]) => `${k} ${t.toFixed(1)}`).join(', ')}; winners named: ${verdicts.map(v => v.winner).join(', ')}`)

phase('Spec')
const judgeText = JSON.stringify(verdicts)
const spec = await agent(`${CONTEXT}

YOUR TASK (label: spec): write the specification of package B12 from three designs and three judges' verdicts. Background:\n${undText}
The designs:\n${desText}
The judges' verdicts (JSON): ${judgeText}
Decide the design to build (normally the judges' winner; explain if you depart from it), graft the best ideas of the others, and resolve every blocking flaw the judges named; where the evidence is not enough to decide, run a small experiment yourself. Then write ${S}/b12/SPEC.md, for an implementer who has not seen any of this:
1. Goal and the user's words; the before/after on the analog (image paths), what must not change.
2. The algorithm, step by step, with the rule for every case the designers listed (nested, node-to-subgraph, subgraph-to-subgraph, own member, block cycles, hubs, empty subgraph, \`direction\` blocks from B9, LR/RL/BT, state composites, notes and bars, invisible links, large drawings), tie-breaks, and determinism.
3. The code plan with file:line against ${TREE}/src (note that the implementer will work on top of package B11, which adds crossing hops in the SVG painter and comb bundling for fans, so name functions, not just lines), and how scene(), route(), route_sized(), route_partial(), hit-testing and fold/compact stay consistent.
4. Numbered work items (B12.1, B12.2, ...) each with its acceptance check; measurable acceptance on the analog with the block-metrics tool (${S}/b12/metrics/) against mermaid's numbers, "no worse than 0.38.0" rules for the corpus, the tests to add, the pinned tests/gates/README images expected to change and why, the perf budgets, docs to update (README Subgraphs section, CHANGELOG).
5. What is out of scope, and whether the work should be split into two packages (say where the cut is).
Also return pkg_spec: the short package spec for the workflow's package list, telling the implementer to read ${S}/b12/SPEC.md first and summarising the items and success criteria.`, { label: 'spec', phase: 'Spec', schema: SPEC })
if (!spec) return { error: 'spec agent returned nothing', und, designs, verdicts }

const crit = await agent(`${CONTEXT}

YOUR TASK (label: critic): adversarially review the specification ${spec.spec_file} of package B12 before it is handed to an implementer. Background:\n${undText}
The designs it was built from:\n${desText}
Hunt for: cases the algorithm does not define or defines wrongly (try concrete small inputs with ${S}/b12/fm038 and mermaid to see what "right" looks like: a hub linked to five blocks, two blocks in a cycle, a subgraph inside a subgraph with edges across both borders, an edge from a subgraph to its own member, an empty subgraph, an LR diagram, a state diagram with composites and a note, a \`direction\` block from B9 inside a block); steps that contradict the code in ${TREE}/src or a PLAN.md invariant; acceptance checks that cannot be measured or that the prototype numbers show to be unreachable; regressions it would cause on the corpus that the spec does not own up to; a drag (route / route_partial) story with a hole in it; work too large for one package without a stated cut. Report only concrete, evidenced problems, each with the exact change to the spec.`, { label: 'critic', phase: 'Spec', schema: CRIT })

let final = spec
if (crit && crit.findings.length) {
  log(`critic: ${crit.findings.length} finding(s), ${crit.findings.filter(f => f.severity === 'blocking').length} blocking`)
  const rev = await agent(`${CONTEXT}

YOUR TASK (label: revise): revise the specification ${spec.spec_file} of package B12. A critic found these problems (JSON): ${JSON.stringify(crit.findings)}
Background:\n${undText}
For each finding: check it, and if it holds, change the spec (edit the file in place) so the problem is resolved; if it does not hold, say why in a short "Reviewed and kept" appendix at the end of the spec. Keep the spec coherent as one document (no patch notes in the body). Return the updated pkg_spec too (the earlier one was: ${JSON.stringify(spec.pkg_spec)}).`, { label: 'revise', phase: 'Spec', schema: SPEC })
  if (rev) final = rev
}
return { spec_file: final.spec_file, pkg_spec: final.pkg_spec, open_questions: final.open_questions, tally, winners: verdicts.map(v => ({ lens: v.lens, winner: v.winner, blocking_flaws: v.blocking_flaws })), designs: designs.map(d => ({ key: d.key, file: d.file, prototype_kind: d.prototype_kind, images: d.prototype_images, metrics: d.metrics })), understand: und.map(u => ({ key: u.key, file: u.file })), critic: crit ? { verdict: crit.verdict, findings: crit.findings.length } : null }