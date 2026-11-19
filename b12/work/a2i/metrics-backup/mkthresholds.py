#!/usr/bin/env python3
"""Writes thresholds.json (static rows) and thresholds-routed.json (4.3 h) from
the tables of SPEC.md section 4.3. Every rule carries the checkpoint its row
is first due at (spec 4.1): judge --due A1 leaves the later ones out.
Names are the basenames of the inputs in b12/local/in."""
import json, os
HERE = os.path.dirname(os.path.abspath(__file__))
ANALOG = 'infra-overview'
V2 = 'infra-overview-v2'
PERMS = ['perm-a-edges-reversed', 'perm-b-subgraphs-reversed', 'perm-c-actors-reversed', 'perm-d-edges-rotated', 'perm-e-one-more-inner-edge']
ADIRS = ['infra-bt', 'infra-lr', 'infra-rl']
PUB = 'blocks-overview'
PUBPERMS = ['blocks-overview-edges-reversed', 'blocks-overview-edges-rotated', 'blocks-overview-actors-reversed', 'blocks-overview-subgraphs-reversed']
PUBDIRS = ['blocks-overview-bt', 'blocks-overview-lr', 'blocks-overview-rl']
DENSE = ['bench-75', 'dense-lr', 'dense-td']
# 4.3 (e): the inputs the spec names; B12.3 prints the full rows-equal list
ROWS_EQUAL = ['advanced', 'subgraph', 'fanout-frames', 'near-drag', 'spread-bottom', 'straight-link', 'wide-frames',
              'p03-ext-dir', 'p04-to-cluster', 'p05-siblings', 'p06-block-cycle', 'p07-nested', 'p08-chain3',
              'p10-mixed-rank', 'p11-lr', 'p12-state', 'hub5', 'cycle2', 'state-note',
              'swimlane', 'swim-cycle', 'swimlane-nested']
REF = lambda f=1.0, a=0.0: {'min': [{'ref': 'baseline', 'factor': f, 'add': a}, {'ref': 'start', 'factor': f, 'add': a}]}
MM = lambda f=1.0, a=0.0: {'ref': 'mermaid', 'factor': f, 'add': a}
rules = []
def rule(scope, metric, op, must=None, target=None, due='A2', why=''):
    r = {'scope': scope, 'metric': metric, 'op': op, 'due': due, 'why': why}
    if must is not None: r['must'] = must
    if target is not None: r['target'] = target
    rules.append(r)
def zeros(scope, why, lateral=True):
    for m in ['xframe_runs', 'sib_side_linked']:
        rule(scope, m, '<=', 0, due='A1', why=why)
    for m in ['sib_stagger'] + (['lateral_x'] if lateral else []) + ['lbl_lbl', 'lbl_node', 'frame_overlap', 'frame_foreign_nodes', 'node_hits', 'warn']:
        rule(scope, m, '<=', 0, due='A2', why=why)

# ---- (a) the analog, against mermaid 12.2
zeros(ANALOG, '4.3 (a)')
rule(ANALOG, 'compact_max', '<=', 5.5, MM(), why="4.3 (a): mermaid's + 20 %")
rule(ANALOG, 'compact_mean', '<=', 3.9, MM(), why="4.3 (a): mermaid's + 20 %")
rule(ANALOG, 'span_max', '<=', 0.37, MM(), why="4.3 (a): mermaid's + 20 %")
rule(ANALOG, 'crossings', '<=', 20, 16, due='A1', why="4.3 (a): mermaid's + 4")
rule(ANALOG, 'bends', '<=', 120, why="4.3 (a): mermaid's total + 20 %")
rule(ANALOG, 'bends_max', '<=', 8, why="4.3 (a): 0.38's own worst edge")
rule(ANALOG, 'detour_max', '<=', 1.63, why="4.3 (a): mermaid's + 20 %")
rule(ANALOG, 'lbl_struck', '<=', 0, why='4.3 (a)')
rule(ANALOG, 'lbl_crowd', '<=', 0, why="4.3 (a): mermaid's count of crowded chips")
rule(ANALOG, 'lbl_gap_min', '>=', 12, why='4.3 (a)')
rule(ANALOG, 'lbl_in_end', '<=', 0, why='4.3 (a)')
rule(ANALOG, 'lbl_own_gap_max', '<=', 1, why='4.3 (a)')
rule(ANALOG, 'lbl_off_mean', '<=', 0.20, 0.13, why='4.3 (a)')
rule(ANALOG, 'lbl_off_max', '<=', 0.35, why='4.3 (a)')
rule(ANALOG, 'lbl_endpx_mean', '<=', 386, 340, why="4.3 (a): the smaller of mermaid's and 1.25 x 0.38's")
rule(ANALOG, 'lane_gap_min', '>=', 9.3, 12, why="4.3 (a): 0.38's")
rule(ANALOG, 'straight_max', '<=', 1240, why="4.3 (a): mermaid's")
rule(ANALOG, 'area', '<=', {'min': [MM(), {'ref': 'baseline', 'factor': 1.25}]}, why="4.3 (a): the smaller of mermaid's and 1.25 x 0.38's")
rule(ANALOG, 'edge_len', '<=', MM(), why="4.3 (a): mermaid's")

# ---- (b) v2 and the five permutations
zeros(V2, '4.3 (b) v2')
rule(V2, 'compact_max', '<=', 5.5, why="4.3 (b): the analog's bound")
rule(V2, 'compact_mean', '<=', 3.9, why="4.3 (b): the analog's bound")
rule(V2, 'span_max', '<=', 0.28, why='4.3 (b)')
rule(V2, 'crossings', '<=', 17, 13, due='A1', why="4.3 (b): mermaid's + 4")
rule(V2, 'bends', '<=', 114, why="4.3 (b): mermaid's + 20 %")
rule(V2, 'bends_max', '<=', 8, why="4.3 (b): the analog's bound")
rule(V2, 'detour_max', '<=', 1.76, why="4.3 (b): mermaid's + 20 %")
rule(V2, 'lbl_struck', '<=', 0, why='4.3 (b)')
rule(V2, 'lbl_crowd', '<=', 1, why="4.3 (b): mermaid's count")
rule(V2, 'lbl_in_end', '<=', 0, why='4.3 (b)')
rule(V2, 'lbl_own_gap_max', '<=', 1, why='4.3 (b)')
rule(V2, 'lbl_endpx_mean', '<=', 329, 290, why="4.3 (b): the smaller of mermaid's and 1.25 x 0.38's")
rule(V2, 'lane_gap_min', '>=', 8, why='4.3 (b)')
rule(V2, 'area', '<=', {'ref': 'baseline', 'factor': 1.25}, why="4.3 (b): 1.25 x 0.38's")
rule(V2, 'edge_len', '<=', MM(), why="4.3 (b): mermaid's")
zeros(PERMS, '4.3 (b) permutations')
rule(PERMS, 'compact_max', '<=', 5.5, why='4.3 (b)')
rule(PERMS, 'span_max', '<=', 0.37, why='4.3 (b)')
rule(PERMS, 'lbl_struck', '<=', 0, why='4.3 (b)')
rule(PERMS, 'lbl_crowd', '<=', 0, why='4.3 (b)')
rule(PERMS, 'lbl_in_end', '<=', 0, why='4.3 (b)')
rule(PERMS, 'bends_max', '<=', 8, why='4.3 (b)')
rule(PERMS, 'lane_gap_min', '>=', 8, why='4.3 (b): EDGE_SEP_MIN')
for n, c in zip(PERMS, [20, 23, 20, 20, 22]):
    rule(n, 'crossings', '<=', c, due='A1', why="4.3 (b): the analog's must (a, d), mermaid's + 4 (c, b, e)")
rule(PERMS, 'bands', '==', {'same_as': ANALOG}, due='A1', why='4.3 (b): the same band order as the analog')
rule(PERMS, 'breadth', '==', {'within': 0.10, 'of': ANALOG}, due='A1', why="4.3 (b): canvas breadth within 10 % of the analog's")

# ---- (c) the public fixture and its permutations
PUBALL = [PUB] + PUBPERMS
zeros(PUBALL, '4.3 (c)')
rule(PUBALL, 'compact_max', '<=', 7.97, why="4.3 (c): mermaid's 6.64 + 20 %")
rule(PUBALL, 'compact_mean', '<=', 4.63, why="4.3 (c): mermaid's 3.86 + 20 %")
rule(PUBALL, 'span_max', '<=', 0.22, why="4.3 (c): mermaid's 0.183 + 20 %")
rule(PUBALL, 'crossings', '<=', 7, 5, due='A1', why="4.3 (c): mermaid's 5 + 2")
rule(PUBALL, 'bends', '<=', 110, why="4.3 (c): mermaid's + 20 %")
rule(PUBALL, 'bends_max', '<=', 8, why='4.3 (c)')
rule(PUBALL, 'detour_max', '<=', 1.43, why="4.3 (c): mermaid's + 20 %")
rule(PUBALL, 'lbl_struck', '<=', 0, why='4.3 (c)')
rule(PUBALL, 'lbl_crowd', '<=', 0, why='4.3 (c)')
rule(PUBALL, 'lbl_in_end', '<=', 0, why='4.3 (c)')
rule(PUBALL, 'lbl_own_gap_max', '<=', 1, why='4.3 (c)')
rule(PUBALL, 'lbl_endpx_mean', '<=', 295, why="4.3 (c): 1.1 x 0.38's 268")
rule(PUBALL, 'lane_gap_min', '>=', 8, why='4.3 (c)')
rule(PUBALL, 'area', '<=', MM(), why="4.3 (c): mermaid's")
rule(PUBALL, 'edge_len', '<=', MM(), why="4.3 (c): mermaid's")
rule(PUBPERMS, 'bands', '==', {'same_as': PUB}, due='A1', why='4.3 (c): the same band order across its four permutations')
rule(PUBPERMS, 'breadth', '==', {'within': 0.10, 'of': PUB}, due='A1', why='4.3 (c): canvas breadth within 10 %')

# ---- (d) new measures, static: lateral crossings
rule(ADIRS + PUBDIRS + ['hub5', 'cycle2'], 'lateral_x', '<=', 0, why='4.3 (d), B12.11')
rule(DENSE, 'lateral_x', '<=', MM(), 0, why="4.3 (d): not above mermaid's count (3 / 0 / 1), target 0")

# ---- (e) rows-equal inputs: not worse than the reference (due at C, with the picture triples)
for m in ['crossings', 'xframe_runs', 'sib_side_linked', 'sib_stagger', 'lbl_struck', 'lbl_crowd']:
    rule(ROWS_EQUAL, m, '<=', REF(), due='C', why='4.3 (e): not above the reference')
rule(ROWS_EQUAL, 'compact_max', '<=', {'max': [REF(1.0, 0.05), MM(1.2)]}, due='C', why="4.3 (e): the larger of the reference + 0.05 and 1.2 x mermaid's")
rule(ROWS_EQUAL, 'area', '<=', {'min': [MM(), {'ref': 'baseline', 'factor': 1.25}, {'ref': 'start', 'factor': 1.25}]}, due='C', why="4.3 (e): the smaller of mermaid's and 1.25 x the reference")
rule(ROWS_EQUAL, 'edge_len', '<=', REF(1.1), due='C', why='4.3 (e): 1.1 x the reference')
rule(ROWS_EQUAL, 'lateral_x', '<=', {'min': [{'ref': 'baseline'}, {'ref': 'start'}, MM()]}, due='C', why="4.3 (e): not above the reference and not above mermaid's count")

# ---- (f) no worse on the dense corpus
for m in ['xframe_runs', 'sib_side_linked']:
    rule(DENSE, m, '<=', 0, due='A1', why='4.3 (f)')
for m in ['sib_stagger', 'lbl_lbl', 'lbl_node', 'frame_overlap', 'frame_foreign_nodes', 'node_hits', 'warn']:
    rule(DENSE, m, '<=', 0, why='4.3 (f)')
for m in ['lbl_struck', 'lbl_crowd', 'compact_max']:
    rule(DENSE, m, '<=', REF(), why='4.3 (f): not above the reference')
rule(DENSE, 'crossings', '<=', REF(1.1, 2), MM(), due='A1', why="4.3 (f): the smaller of 1.1 x 0.38's + 2 and 1.1 x the starting release's + 2")
rule(DENSE, 'area', '<=', MM(), why="4.3 (f): mermaid's")
rule(DENSE, 'edge_len', '<=', {'max': [MM(), REF(1.1)]}, why="4.3 (f): the larger of mermaid's and 1.1 x the reference")
# every block-mode input: no run through a foreign frame (the not-block-mode inputs are byte-identical and hold none)
rule('*', 'xframe_runs', '<=', 0, due='A1', why='4.3 (f): no run through a block that holds neither end')
json.dump({'note': 'generated by mkthresholds.py from SPEC.md 4.3; due = the checkpoint a row is first judged at (4.1)', 'rules': rules},
          open(os.path.join(HERE, 'thresholds.json'), 'w'), indent=1)
n_static = len(rules)

# ---- (h) routed at the scene's centres (reference tables: the routed twins)
rules = []
H = [ANALOG, V2] + PERMS + [PUB]
rule(H, 'xframe_runs', '<=', 0, due='R', why='4.3 (h)')
rule(H, 'lateral_x', '<=', 0, due='R', why='4.3 (h)')
rule(H, 'lbl_struck', '<=', 0, due='R', why='4.3 (h)')
rule(H, 'lbl_crowd', '<=', REF(), due='R', why="4.3 (h): not above the reference's routed value")
for n, c in [(ANALOG, 24), (V2, 21), (PUB, 11)] + list(zip(PERMS, [24, 27, 24, 24, 26])):
    rule(n, 'crossings', '<=', {'min': [{'ref': 'baseline'}, {'ref': 'start'}, c]}, due='R', why="4.3 (h): not above the reference's routed count and the static must + 4")
rule(PUBPERMS, 'crossings', '==', {'same_as': PUB}, due='R', why='B12.13: routed crossings of the public fixture equal across its permutations')
rule(ADIRS + PUBDIRS + ['hub5', 'cycle2'] + PUBPERMS, 'lateral_x', '<=', 0, due='R', why='4.3 (d): routed')
rule(DENSE, 'lateral_x', '<=', REF(), due='R', why='4.3 (d): routed, not above the reference')
rule(DENSE, 'xframe_runs', '<=', REF(), due='R', why='B12.13: routed runs through a foreign frame')
json.dump({'note': 'generated by mkthresholds.py from SPEC.md 4.3 (h) and (d); the reference tables are ref-038-routed.tsv and ref-start-routed.tsv', 'rules': rules},
          open(os.path.join(HERE, 'thresholds-routed.json'), 'w'), indent=1)
print(n_static, 'static rules,', len(rules), 'routed rules')
