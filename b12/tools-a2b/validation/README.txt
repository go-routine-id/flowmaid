Validation of the tools on known cases (2026-11-17):
  cells-squeeze-S-vs-base.txt   check 1 on the skeptic's squeeze prototype renders (design-a2b/critic/svg/S),
                                 re-judged with the live judge: 12 NEW, 22 WORSE inside failing (89 -> 70 must-fail)
  panics-squeeze-S-vs-base.txt  check 2 on its suite-on log (design-a2b/critic/t/S.txt, lib + quality in one file):
                                 2 newly failing, 11 worse message lines
  qcmp-synthetic-vs-base.txt    check 3 on base-7122a53's dirs.tsv with three injected changes (a missing copy,
                                 routed fan_x +1 on blocks-overview-TD, static min_edge_gap 40 -> 12.7 on ratio2x4-TD)
                                 and one informational one (static lat_wiggles +3 on bench-75-TD): WORSE (3)
  panics-ton-split-vs-base.txt  work/a2i/ton-split.txt (7122a53 suite-on, cargo test from /home/user/flowmaid) against
                                 the baseline's gates: no message differs
