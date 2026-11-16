# Commit messages to reword when the history is tidied (decision D5)

Workspace only. Verify round 3, finding #2: two messages of this package carry numbers of the local acceptance
family, which D5 keeps out of the repository (commit messages included; a message may cite only the public
fixture and the ratio / hub / cycle fixtures, and "local acceptance inputs: all musts met"). The fix rounds may not
amend or rebase, so the lead rewords them when the package's history is tidied, before the merge. Proposed text
below: replace the quoted paragraph, keep the rest of each message as it is.

## f1879b1 (fix(blocks): the structural defects of verify round 2 ...)

Replace the paragraph that starts "Suite-on, static: local acceptance crossings unchanged" with:

```
Suite-on, static: no crossing count changes on the public fixture and
its permutations (6 each), bench-75 247, dense-lr 17, dense-td 68;
local acceptance inputs: crossing musts held, unchanged. Must-fails
5 -> 3 (dense-td xframe_runs gone; bench-75 2 and notes/fork-twins 1
left). Repository fixtures: hard zeros 23 -> 10 against the start
(blocks/nested-chain 0). Suite-on failing tests: the same ten library
tests and six quality gates as before the round, none new. No snapshot
or gate changes.
```

## 32a3ebf (feat(blocks): a free pin stands clear of what else crosses its frame side ...)

In the first paragraph, replace "the one fan crossing left on the local acceptance family, and a shared port on
notes/spread-face." with "a fan crossing, and a shared port on notes/spread-face.".

Replace the paragraph that starts "Measured with the switch on (static):" with:

```
Measured with the switch on (static): notes/spread-face shared_ports
1 -> 0, x_drawn 3 -> 1; the public fixture and its permutations
unchanged (6); blocks-overview-lr/-rl 5 -> 6 and dense-lr 16 -> 17
(inside their ceilings). Local acceptance inputs: fan_x improved; one
fan crossing left, an entry under a title chip (B12.11).
```

## Checked and clean

Every other message from `c024ac0` to the last commit of fix round 4 was searched for the family's file names,
ids, labels and for "analog", "perm", "v2", "local acceptance": `1fa410e` says "local acceptance inputs: layers and
upstream edges as specified" (no number, allowed); none other names the family. Fix round 4's own messages cite
the public fixture, the dense fixtures and the notes fixtures only.
