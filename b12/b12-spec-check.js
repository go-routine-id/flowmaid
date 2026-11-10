export const meta = {
  name: 'b12-spec-check',
  description: 'Second pass over the revised B12 spec: check each critic fix landed, cold-read it as the implementer, audit the acceptance numbers, verify findings, revise',
  phases: [
    { title: 'Check', detail: 'fix check (two halves), cold implementer read, acceptance audit' },
    { title: 'Verify', detail: 'one skeptic per checker' },
    { title: 'Revise', detail: 'apply confirmed findings; package specs for B12a and B12b' },
  ],
}
const S = args.scratch
const B = `${S}/b12`
const CONTEXT = `CONTEXT
flowmaid is a pure-Rust (std only, zero dependencies) mermaid-like diagram engine; ${S}/PLAN.md holds its non-negotiable invariants. Package B12 "subgraphs as blocks" is specified in ${B}/SPEC.md (about 115 KB): in every flowchart or state diagram where a link crosses a subgraph border, each subgraph becomes one compact block, linked blocks stack along the flow, and edges between blocks run in the gaps. The user's bar: "pokoknya mirip atau lebih bagus dan rapi dari mermaid js 12" (as tidy as mermaid 12.2 or tidier); their decisions are in ${B}/USER-DECISIONS.md. The spec was written from three prototyped designs (${B}/design-*.md, ${B}/d-*/), judged, then revised against a critic's 22 findings (the critic's work is in ${B}/critic/, the revision's evidence in ${B}/revise/, the pre-revision text in ${B}/revise/SPEC.before-revise.md). Background analyses: ${B}/understand-flowmaid.md, ${B}/understand-mermaid.md, the block-metrics tool ${B}/metrics/ (README.md there).
Source tree, READ-ONLY: ${B}/tree (git worktree at dev = release 0.38.0). Package B11 (crossing hops in the SVG painter, comb bundling for fans: new src/hops.rs and src/comb.rs) is being finished on another checkout; B12 will be implemented on top of it. You can read B11's first commit without touching that checkout: \`git -C ${B}/tree show 52e4268 --stat\` and \`git -C ${B}/tree show 52e4268:src/comb.rs\`.
Rules: do NOT touch /home/user/flowmaid; do not commit or push anything; do not edit ${B}/tree or ${B}/SPEC.md (only the last stage edits the spec). Scratch space: ${B}/check-<your label>/. Prebuilt binaries you can run instead of building: ${B}/fm038 (0.38.0 CLI), and the prototypes' binaries named in the design documents and in the spec's appendix A. Avoid cargo builds; if one is essential, copy the tree, use CARGO_TARGET_DIR inside your scratch dir, build release only, delete it afterwards (disk is limited, and the box is shared with a perf-sensitive job). SVG to PNG: \`cd ${B} && node shot.js in.svg out.png\`. Mermaid renders: \`node ${S}/tools/mmrender.js <outdir> <files.mmd...>\`.
PRIVACY: ${B}/private/ holds the user's real screenshots (real hostnames and IPs). Never copy them or any name or IP from them anywhere. The sanitized analog (${B}/in/infra-overview.mmd and the -v2 variant) is a LOCAL acceptance input and must never be committed to the repository; the public fixture is the synthetic ${B}/revise/in/blocks-overview.mmd.`
const FIND = {
  type: 'object',
  properties: {
    findings: { type: 'array', items: { type: 'object', properties: {
      id: { type: 'string', description: 'short id, e.g. A3' },
      where: { type: 'string', description: 'spec section and line' },
      problem: { type: 'string' },
      evidence: { type: 'string' },
      severity: { type: 'string', enum: ['blocking', 'non-blocking'] },
      fix: { type: 'string', description: 'the exact change to the spec' } },
      required: ['id', 'where', 'problem', 'evidence', 'severity', 'fix'] } },
    checked: { type: 'string', description: 'what you checked and found sound' },
  },
  required: ['findings', 'checked'],
}
const VERD = {
  type: 'object',
  properties: {
    verdicts: { type: 'array', items: { type: 'object', properties: {
      id: { type: 'string' }, refuted: { type: 'boolean' }, reason: { type: 'string' }, fix_ok: { type: 'boolean' }, better_fix: { type: 'string' } },
      required: ['id', 'refuted', 'reason', 'fix_ok'] } },
  },
  required: ['verdicts'],
}
const OUT = {
  type: 'object',
  properties: {
    spec_file: { type: 'string' },
    pkg_spec_a: { type: 'string', description: 'package spec for B12a (under 220 words): read the spec first; items, checkpoints, the switch, success criteria, no release' },
    pkg_spec_b: { type: 'string', description: 'package spec for B12b (under 220 words): read the spec first; items, checkpoint, default on, success criteria' },
    changes: { type: 'array', items: { type: 'string' } },
    kept: { type: 'array', items: { type: 'string' }, description: 'findings not applied, with the reason' },
    open_questions: { type: 'array', items: { type: 'string' } },
  },
  required: ['spec_file', 'pkg_spec_a', 'pkg_spec_b', 'changes', 'kept', 'open_questions'],
}
const CHECKS = [
  { key: 'fix-a', prompt: `YOUR TASK (label: fix-a): the critic's findings 1 to 8 were blocking (read them in the critic's material under ${B}/critic/ and in appendix C and section 6 of the spec; the pre-revision text is ${B}/revise/SPEC.before-revise.md). For each of the eight: is the problem really resolved in the BODY of the revised spec, is the new rule consistent with every other section that touches it (grep the spec for every mention of the old rule: the rank-comparison trigger, F2 as a frame rule, ALIGN_SNAP, the TIE weight, lanes::plan for corridors, the clamp of note cards), and does the new rule itself hold against the code in ${B}/tree/src and against a concrete small input you try? Pay special attention to the new structural trigger of 2.2 (block mode whenever a link joins a block to anything outside it): what exactly counts (a sub-edge to the block itself, an invisible link, a note link, a link between a block and its own nested block, a B9 \`direction\` block), which corpus inputs flip, and whether "flat diagrams and diagrams with only isolated subgraphs stay byte-identical" can be guaranteed by a test.` },
  { key: 'fix-b', prompt: `YOUR TASK (label: fix-b): the critic's findings 9 to 22 were non-blocking (read them under ${B}/critic/ and in appendix C of the spec; the pre-revision text is ${B}/revise/SPEC.before-revise.md). For each: is it resolved in the body, consistently with the other sections, and does the rule hold against the code in ${B}/tree/src and against a concrete small input you try (an adjacent-layer upstream edge with a block end, an LR diagram whose block title is longer than the block, a subgraph linked to its own nested subgraph, a level of 13 or more units, a swimlane-style diagram where a third of the links run against the flow, labels at the hub end of long corridor edges)? Also check the route() story end to end (2.9 and 3.2): for a drag of one node inside a block and of one node outside any block, say which frames and edges change in route() and in route_partial(), and whether the spec's rules give the same frames in both.` },
  { key: 'cold-read', prompt: `YOUR TASK (label: cold-read): you are the senior Rust engineer who will implement B12a next, then B12b, with nothing but ${B}/SPEC.md, PLAN.md and the code. Read the spec from top to bottom as your only brief and report everything that would make you stall or build the wrong thing: a step whose input is not defined when it runs; two rules that give different answers for the same case; a named function, constant, file, test or fixture that does not exist and is not marked as new (check each against ${B}/tree and against B11's commit 52e4268, which adds src/hops.rs and src/comb.rs and changes ports.rs, scene.rs, casing.rs: does the spec's code plan still fit on top of it, and does it say how block mode meets the comb bundling and the hub port band?); a work item with no acceptance check or a check that cannot fail; a checkpoint whose stop rule is unclear; what exactly "behind an internal switch" means for B12a (how tests, the quality harness and the verifier turn it on, what must be byte-identical with it off) and what flips in B12b; whether each of the two packages can be verified on its own by an independent verifier; an item order that forces rework. Also say whether the two packages are each of a size one implement-verify-fix cycle can carry (B10, of similar ambition, took about 15 hours and three fix rounds), and where you would cut differently.` },
  { key: 'acceptance', prompt: `YOUR TASK (label: acceptance): audit the acceptance of the spec (section 4.3 and the gates in 4.2, 4.5, 4.6) against the user's bar and against what can be measured. (1) Re-run the block-metrics tool on the mermaid SVGs and the 0.38.0 SVGs of the analog, the -v2 variant and the public fixture (${B}/revise/in/blocks-overview.mmd): are the numbers quoted in the spec right? (2) For every metric the spec gates on (compactness, span, side-by-side and interleaved pairs, runs through a foreign frame, lateral frame crossings, crossings, struck and crowded labels, label distance to the nearer end, pin_corner, title_cut, spine_off, area, edge length): does the tool compute it today, for flowmaid and for mermaid SVGs, in TD and in LR; if not, does a work item create it before the gate is used? (3) Is each gate at least as strict as "as tidy as mermaid 12.2 or tidier" within the spec's stated tolerances, and are the tolerances themselves defensible (compare the analog's mermaid picture ${B}/metrics/mermaid-svg/infra-overview.mermaid.png with the best experiment picture ${B}/spec/evidence/infra-overview-v2.lvl-chains-7starts.png: what would a person still call less tidy than mermaid that no gate catches)? (4) Do the gates for the corpus ("not worse than 0.38") name the files and numbers, and are the perf budgets stated with baselines that exist in ${S}/runs/baseline/perf.tsv? (5) Does every test the spec asks for run on repository fixtures only, with the analog family used only by the local verifier, and does the spec say how the local acceptance is reported?` },
]
phase('Check')
const results = await pipeline(
  CHECKS,
  c => agent(`${CONTEXT}\n\n${c.prompt}\nReport only concrete, evidenced problems, each with the exact change to the spec; say what you checked and found sound.`, { label: c.key, phase: 'Check', schema: FIND }),
  (r, c) => {
    const fs = (r && r.findings) || []
    if (!fs.length) return { key: c.key, checked: (r && r.checked) || '', items: [] }
    return agent(`${CONTEXT}\n\nYOUR TASK (label: verify-${c.key}): a checker (${c.key}) reported these problems in ${B}/SPEC.md (JSON): ${JSON.stringify(fs)}\nFor EACH finding try hard to REFUTE it: re-read the spec sections it names (the problem may be answered elsewhere in the spec), re-check the code, re-run what was run. Return one verdict per finding id: refuted or not, why, whether the proposed fix is right, and a better fix if not.`, { label: `verify-${c.key}`, phase: 'Verify', schema: VERD })
      .then(v => ({ key: c.key, checked: r.checked, items: fs.map(f => ({ ...f, verdict: ((v && v.verdicts) || []).find(x => x.id === f.id) || null })) }))
  }
)
const all = results.filter(Boolean)
const confirmed = all.flatMap(r => r.items).filter(f => !f.verdict || !f.verdict.refuted)
const refuted = all.flatMap(r => r.items).filter(f => f.verdict && f.verdict.refuted)
log(`second pass: ${confirmed.length} confirmed (${confirmed.filter(f => f.severity === 'blocking').length} blocking), ${refuted.length} refuted`)
phase('Revise')
const rev = await agent(`${CONTEXT}

YOUR TASK (label: revise-2): finalise ${B}/SPEC.md. A second review pass confirmed these problems (JSON, each with its verifier's verdict and, where given, a better fix): ${JSON.stringify(confirmed)}
First copy the current spec to ${B}/revise/SPEC.before-pass2.md. Then, for each finding: check it once more and change the spec in place so the problem is resolved (prefer the verifier's better fix where there is one); keep the spec one coherent document (no patch notes in the body); record what you did not apply, and why, in the "Reviewed and kept" appendix. Make sure the two packages are cleanly separable: B12a (structure and route(), behind the internal switch, nothing visible by default, no release) and B12b (edges, labels, default on, release 0.41.0), each with its items, checkpoints, stop rules and its own acceptance that an independent verifier can run. Return pkg_spec_a and pkg_spec_b: the short package specs for the workflow's package list. Each must tell the implementer to read ${B}/SPEC.md first, name the items and checkpoints of that package, the success criteria and stop rules, that the analog family (${B}/in/, ${B}/u-mermaid/in/infra-overview-v2.mmd) is a local acceptance input that is never committed, and for B12a that the package ends with the switch off by default and byte-identical default output.`, { label: 'revise-2', phase: 'Revise', schema: OUT })
return { final: rev, confirmed: confirmed.map(f => ({ id: f.id, severity: f.severity, where: f.where, problem: f.problem.slice(0, 300) })), refuted: refuted.map(f => ({ id: f.id, where: f.where, reason: f.verdict.reason.slice(0, 300) })), checked: all.map(r => ({ key: r.key, checked: r.checked })) }