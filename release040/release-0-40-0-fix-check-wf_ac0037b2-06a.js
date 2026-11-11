export const meta = {
  name: 'release-0-40-0-fix-check',
  description: 'Check that the fix commit on release/0.40.0 settles every confirmed finding of the claim review and adds nothing false',
  phases: [
    { title: 'Check', detail: 'two independent checkers: CHANGELOG, and README plus the rest' },
  ],
}

const S = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
const T = S + '/wt/rel040'

const COMMON = `
You are checking a fix commit on the release branch of the Rust crate flowmaid, version 0.40.0, before it is merged to main. READ-ONLY: change no file in the release tree, make no commit, push nothing.

- Release tree: ${T} (branch release/0.40.0). Head 8f42975 is the fix commit; 5207de9 is the release commit it fixes. "git show 8f42975" is the whole fix (CHANGELOG.md, README.md, one comment in src/class.rs). origin/main is the previous release, 0.39.0.
- The claim review the fix answers: ${S}/release040/review/claim-review-results.json. Keys "review:changelog", "review:readme", "review:packaging" hold the findings (lists, numbered from 1 in order); keys "verify:<part>#<n>" hold a skeptic's verdict for each: real (true/false), reason, and fix (the wording the skeptic judged correct, with the numbers they re-measured). verify:readme#3 is real=false: that finding was refuted and must NOT have been applied.
- Do NOT run cargo in the release tree (a test gate is running there). Prebuilt binaries: ${S}/release040/bin/flowmaid040 and flowmaid039 ("flowmaid file.mmd" prints the SVG; parse errors go to stderr), bench040 and bench039 (only ever as "bench files --runs N a.mmd ..."). mermaid 12.2.0: node ${S}/tools/mmrender.js <outDir> <file.mmd>...
- Records: ${S}/history/c-leftovers-run-results.json (the package's verification; "verify:c-leftovers#2"), ${S}/compare/bench039/compare.md, bench040/compare.md, bench040b/compare.md, ${S}/release040/ab/ (a1..a3.tsv = 0.39.0, b1..b3.tsv = 0.40.0, three alternated rounds).
- Scratch: your own directory under ${S}/release040/review/.

For each confirmed finding in your part decide: SETTLED (the new text is true and says what the verdict asked, in its own words or the verdict's), NOT SETTLED (still false, misleading or missing), or WRONG FIX (the new text introduces something false, or a number that does not match the verdict or the records). The fix did not always copy the verdict's wording; judge the text that is there. Recheck every number the fix commit adds against the verdicts and records (you need not re-measure what a verdict already re-measured, but the number must be the verdict's). Then read the changed passages once more in full, in the tree, as a user would, and report anything in them that is false, self-contradictory, garbled by the edit (a broken sentence, a list that no longer renders as Markdown, a line the edit orphaned), or that leaves a template token or stray marker behind.

House rule for this project's README: short and plain; it must not restate mermaid's basic syntax.

Return one entry per finding you judged, and a list of new problems (empty if none). Do not invent problems; style preferences are not problems.`

const SCHEMA = {
  type: 'object',
  properties: {
    judged: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          finding: { type: 'string', description: 'e.g. changelog#2' },
          status: { type: 'string', enum: ['settled', 'not_settled', 'wrong_fix', 'correctly_not_applied'] },
          note: { type: 'string' },
        },
        required: ['finding', 'status', 'note'],
      },
    },
    new_problems: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          where: { type: 'string' },
          problem: { type: 'string' },
          evidence: { type: 'string' },
          fix: { type: 'string', description: 'exact replacement text' },
        },
        required: ['where', 'problem', 'evidence', 'fix'],
      },
    },
    verdict: { type: 'string', enum: ['complete_and_correct', 'needs_changes'] },
  },
  required: ['judged', 'new_problems', 'verdict'],
}

const PARTS = [
  {
    key: 'changelog',
    prompt: `${COMMON}

YOUR PART: CHANGELOG.md. Findings to judge: changelog#1 (same as readme#1 and packaging#1: the placeholder and the benchmark bullet that replaces it), changelog#2 with packaging#2 (scope of the costs; all four face-change rates), changelog#3 (dotted names: where a dot at an end is an error), changelog#4 (timing cost), changelog#5 (the 3.5 px figure, also the test comment in src/class.rs), and the CHANGELOG halves of readme#4 and readme#5 (the [0.40.0] Documentation entry on state composites). Read the whole [0.40.0] section afterwards: the costs are now a nested bullet list inside the entry "Class diagrams: self-relations keep off a neighbour's"; render lines 100-160 with pandoc (-f gfm -t html) if it is installed, or reason from the Markdown indentation, and confirm the nested list and the entry that follows it ("State diagrams: a drag inside a composite ...") still render as intended. Confirm "git diff origin/main HEAD -- CHANGELOG.md" still leaves the [0.39.0] section and everything below it untouched and [Unreleased] empty.`,
  },
  {
    key: 'readme',
    prompt: `${COMMON}

YOUR PART: README.md and everything else. Findings to judge: readme#2 (self-relation labels), readme#3 (refuted: the Fans bullet must be unchanged from 5207de9), readme#4 and readme#5 (the two state rows of the "Differences from mermaid" table: test the new wording against mermaid 12.2.0 and flowmaid040 with inputs of your own, including an inbound-only transition, an outbound one, a transition to the composite itself, a note on the composite and a note inside it), readme#6 and readme#7 (the benchmark Method note: the aggregation is named, "in the 0.39.0 run", and the note was not shortened in a way that drops evidence). Check the table still renders (same number of cells in every row). Then: "git diff 5207de9 8f42975 --stat" must list only CHANGELOG.md, README.md and src/class.rs, and the src change must be a comment only (no code token changed); search the tree outside target/ for any leftover template token (pattern @@[A-Z_0-9]+@@) and for the strings TODO, FIXME, XXX in the lines the two release commits added ("git diff a49ac2c HEAD"); and confirm the commit message of 8f42975 carries the Co-Authored-By and Claude-Session trailer lines and names no AI model outside the Co-Authored-By line.`,
  },
]

const out = await parallel(PARTS.map(p => () =>
  agent(p.prompt, { label: `fixcheck:${p.key}`, phase: 'Check', schema: SCHEMA }).then(r => ({ part: p.key, ...(r || { failed: true }) }))
))
return out.filter(Boolean)