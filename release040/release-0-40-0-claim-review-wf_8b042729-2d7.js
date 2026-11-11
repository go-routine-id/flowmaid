export const meta = {
  name: 'release-0-40-0-claim-review',
  description: 'Review every claim the 0.40.0 release makes (CHANGELOG, README, packaging) against code, records and mermaid; verify each finding adversarially',
  phases: [
    { title: 'Review', detail: 'three read-only reviewers: changelog claims, README, packaging and API docs' },
    { title: 'Verify', detail: 'one skeptic per finding tries to refute it' },
  ],
}

const S = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
const T = S + '/wt/rel040'

const COMMON = `
You are reviewing the release commit of the Rust crate flowmaid, version 0.40.0, before it goes to the main branch. READ-ONLY: change no file in the release tree, make no commit, push nothing.

Where things are:
- Release tree (git worktree, branch release/0.40.0, head 5207de9): ${T}
  - 5207de9 is the release commit (version, CHANGELOG [0.40.0] Highlights, README notes, comparison images).
  - a49ac2c merges the previous release (0.39.0, origin/main = d218a1a) into the package branch.
  - The package ("C-leftovers", class and state diagram follow-ups) is the commits 3abbfeb..9d3e229.
  - "git diff origin/main HEAD" in that tree is everything 0.40.0 adds to 0.39.0.
- Prebuilt binaries (use these; do NOT run cargo in the release tree, a test gate is running there and holds the build lock):
  - ${S}/release040/bin/flowmaid040 and flowmaid039: the CLI of 0.40.0 and 0.39.0 ("flowmaid file.mmd" prints the SVG, a parse error goes to stderr with a line number).
  - ${S}/release040/bin/bench040 and bench039: "bench files --runs N a.mmd b.mmd" prints a TSV of median render times. Never run bench without the "files" argument (it starts a synthetic run that takes many minutes).
- mermaid 12.2.0 in headless Chromium: node ${S}/tools/mmrender.js <outDir> <file.mmd>...  writes <name>.mermaid.svg and .png (a mermaid parse error shows in the output). Use your own scratch directory under ${S}/release040/review/.
- Records of the package's three independent verification rounds: ${S}/history/c-leftovers-run-results.json (keys such as "verify:c-leftovers#2" hold the verdict with verified_fixes, problems, unfixed_items, notes).
- Benchmark records of this release: ${S}/compare/bench039/compare.md (the table in the README, measured on 0.39.0), ${S}/compare/bench040/compare.md and ${S}/compare/bench040b/compare.md (two runs on 0.40.0), ${S}/release040/ab/ (a1..a3.tsv = 0.39.0, b1..b3.tsv = 0.40.0, three alternated rounds on the 25 sample diagrams in ${S}/cmp/corpus; svg39/ and svg40/ hold the SVGs both releases draw for the 25 samples and the 21 files in examples/).

What counts as a finding: a statement in the release's text that is false, overstated, unsupported by the code or by a record, stale (describes behaviour the release changed), or contradicted elsewhere in the same release; a number that does not match its source; something a user would be misled by. Also a real defect in the release itself that you can demonstrate (wrong version somewhere, broken link or anchor, a file that should not ship). Style preferences and things already stated honestly are not findings. Check claims yourself; do not assume. Small generated test inputs are fine; keep runs short (the machine has 4 CPUs and is shared).

House rule for the README (from the project owner): it must be short and plain and must not restate mermaid's basic syntax; it lists the supported subset, what is not supported yet, what flowmaid adds and where it differs from mermaid.

Return every finding with: where (file and line or section), the claim as written (quote it), what is wrong, your evidence (command and output, or file and line), severity ("blocking" = false or misleading to a user, "minor" = imprecise or stale wording), and the smallest fix in words. If you find nothing, return an empty list; do not invent findings.`

const FINDINGS = {
  type: 'object',
  properties: {
    checked: { type: 'array', items: { type: 'string' }, description: 'claims you checked and found correct, one line each with how you checked' },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          where: { type: 'string' },
          claim: { type: 'string' },
          problem: { type: 'string' },
          evidence: { type: 'string' },
          severity: { type: 'string', enum: ['blocking', 'minor'] },
          fix: { type: 'string' },
        },
        required: ['where', 'claim', 'problem', 'evidence', 'severity', 'fix'],
      },
    },
  },
  required: ['checked', 'findings'],
}

const VERDICT = {
  type: 'object',
  properties: {
    real: { type: 'boolean', description: 'true only if you reproduced the problem yourself' },
    reason: { type: 'string' },
    fix: { type: 'string', description: 'the smallest correct fix, exact replacement text where it is a wording change' },
  },
  required: ['real', 'reason', 'fix'],
}

const DIMENSIONS = [
  {
    key: 'changelog',
    prompt: `${COMMON}

YOUR PART: the CHANGELOG. Read ${T}/CHANGELOG.md from the top through the end of the [0.40.0] section (its Highlights, Added, Fixed, Documentation).
1. Every sentence of the Highlights must be supported by an entry below it, by the code, or by the verification record. Check the numbers in the Highlights and in the paragraph that begins "What it costs" (end of the entry "Class diagrams: self-relations keep off a neighbour's") against "verify:c-leftovers#2" in the record, figure by figure.
2. Check behaviour claims with the binaries: dotted class names in every place the text names (declaration, class block, inline member, relation, note for, backtick literal, "A.B..>C", the error cases "A.", ".A", "A...B", the "o" cases "pkg.o --> B", "o.A --> B", "x.o--o.y"); the plain fan of six in the four directions (count crossings in the SVG paths or compare with flowmaid039); "Flowchart drawings are unchanged, and so are the other diagram types" (compare svg39/ and svg40/, and try a few inputs of your own of several diagram types with both binaries).
3. "No public API is removed or changed": check "git diff origin/main HEAD -- src" for removed or changed pub items (pub fn, pub struct fields, pub enum variants, trait impls removed). PartialEq being added is in the text.
4. The section [0.39.0] must be exactly as on origin/main, [Unreleased] must be empty and above [0.40.0], and no entry of this release may sit in an older section.`,
  },
  {
    key: 'readme',
    prompt: `${COMMON}

YOUR PART: the README and the docs page. Read "git diff origin/main HEAD -- README.md docs/docs/index.html" in ${T}, then the README sections it touches in full (Differences from mermaid, State diagrams, Class diagrams, Performance with its Method notes, the install snippet, the comparison intro).
1. The three new rows of the "Differences from mermaid" table state what mermaid 12.2.0 does and what flowmaid does. Verify each side with real inputs: mermaid through mmrender.js, flowmaid through flowmaid040. Rows: a state composite with no direction of its own (in a direction LR diagram; also the stated exceptions: a state of it linking out, a note attached to it or inside it); a state composite with a direction and a note inside; a class name part that is just "o" ("o.A --> B", "x.o--o.y").
2. Class section: "dotted (com.example.Order)" names are listed as supported and are no longer under "Not yet"; the Self-relations bullet (shared among faces; a loop under a box counts first; more than eight loops keeps to two faces). Check against the code in src/class.rs (search for the loop face sharing, e.g. loop_faces, LOOP_CLUTTER) and with inputs.
3. State section: the Direction bullet and its link to the differences table (the anchor must exist).
4. Performance: the Method note says 0.40.0 draws all 25 samples byte for byte the same as 0.39.0 and in the same time (0.99x in the geometric mean over three alternated rounds), and that two runs of the whole table on 0.40.0 gave 127x and 134x. Recompute each of these from the records. Check that the table in the README is still the one in bench039/compare.md and that "113x" appears consistently.
5. Stale text: search the whole README and docs/docs/index.html for statements this release makes untrue (dotted names being errors, self-relations behaviour, state composite direction "as in mermaid" without the exception, version numbers 0.39 where 0.40 is meant; a 0.39.0 that names when something was measured is correct and stays).
6. The house rule: flag any place where the new text restates mermaid's basic syntax or is longer than it needs to be, with a shorter wording.`,
  },
  {
    key: 'packaging',
    prompt: `${COMMON}

YOUR PART: packaging, versions, doc comments and hygiene.
1. Version 0.40.0 in Cargo.toml and Cargo.lock, nowhere a leftover that should have moved (search the tree outside target/ for "0.39" and judge each hit: a past release's section or a "measured on 0.39.0" note is correct).
2. The crate contents: in a COPY of the tree (cp -r without target/, into ${S}/release040/review/pkgcopy; cargo there is allowed with CARGO_BUILD_JOBS=1, and only "cargo package --list --allow-dirty", no build) check that docs/img is excluded and nothing unexpected ships (scratch files, logs, private notes).
3. Public doc comments the package changed: read "git diff origin/main HEAD -- src" for changed /// comments on pub items (route_partial, the model types that gained PartialEq, class and state scene builders) and check each against the code beside it.
4. Hygiene of everything the release adds (git diff origin/main HEAD, and the commit messages of origin/main..HEAD): no debug output or leftover dbg!/eprintln!/println! in library code, no TODO/FIXME/XXX added, no new #[ignore], no test weakened or deleted without a replacement (list removed assertions and judge them), no host names, IP addresses or personal data, and no AI model name anywhere in the diff or in a commit message other than in the "Co-Authored-By:" trailer line. Each commit of origin/main..HEAD that is not a merge of an older commit must carry the Co-Authored-By and Claude-Session trailer lines.
5. The comparison images docs/img/compare/*.png: the README says they show flowmaid 0.40.0. Look at two of them (Read the PNG) and confirm the caption says 0.40.0 and the picture is whole.
6. docs/docs/index.html install snippet says 0.40.`,
  },
]

const results = await pipeline(
  DIMENSIONS,
  d => agent(d.prompt, { label: `review:${d.key}`, phase: 'Review', schema: FINDINGS }),
  (review, d) => {
    if (!review) { log(`review:${d.key} returned nothing`); return { key: d.key, checked: [], findings: [], failed: true } }
    log(`review:${d.key}: ${review.findings.length} finding(s), ${review.checked.length} claim(s) checked`)
    return parallel(review.findings.map((f, i) => () =>
      agent(`${COMMON}

YOUR PART: another reviewer reported the finding below. Try to REFUTE it: reproduce it yourself from the release tree, the binaries, the records or mermaid. It is real only if you reproduce the problem; if the release's text is in fact correct, or the finding is a matter of taste, say real=false and why. If it is real, give the smallest correct fix; for a wording fix give the exact replacement text, in the same plain style as the text around it.

FINDING (${d.key} #${i + 1}, ${f.severity})
where: ${f.where}
claim as written: ${f.claim}
problem: ${f.problem}
evidence given: ${f.evidence}
fix proposed: ${f.fix}`, { label: `verify:${d.key}#${i + 1}`, phase: 'Verify', schema: VERDICT })
        .then(v => ({ ...f, n: i + 1, verdict: v }))
    )).then(fs => ({ key: d.key, checked: review.checked, findings: fs.filter(Boolean) }))
  }
)

const out = results.filter(Boolean)
const confirmed = out.flatMap(r => r.findings.filter(f => f.verdict && f.verdict.real).map(f => ({ part: r.key, ...f })))
const refuted = out.flatMap(r => r.findings.filter(f => f.verdict && !f.verdict.real).map(f => ({ part: r.key, where: f.where, claim: f.claim, why_not: f.verdict.reason })))
const unverified = out.flatMap(r => r.findings.filter(f => !f.verdict).map(f => ({ part: r.key, ...f })))
return {
  confirmed, refuted, unverified,
  failed_parts: out.filter(r => r.failed).map(r => r.key),
  checked: out.map(r => ({ part: r.key, n: r.checked.length, items: r.checked })),
}