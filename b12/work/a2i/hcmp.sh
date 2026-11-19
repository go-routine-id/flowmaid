#!/usr/bin/env bash
# hcmp.sh <old-quality> <new-quality> <label>: harness columns per fixture dir, diff old vs new
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2i
COLS="defects x_drawn edge_frame_pairs overlap_pairs lateral_x fan_x edge_chip frame_side shared_ports arrow_on_start wiggles node_frame frame_overlap edge_node_x own_frame_x min_edge_gap pin_corner side_ends up_lanes title_cut compact_max sib_stagger lbl_crowd chip_crowd bends_max"
for d in blocks quality notes; do
  $W/qcol.sh $1 /home/user/flowmaid/tests/fixtures/$d $COLS > $W/h-old-$3-$d.tsv
  $W/qcol.sh $2 /home/user/flowmaid/tests/fixtures/$d $COLS > $W/h-new-$3-$d.tsv
  diff $W/h-old-$3-$d.tsv $W/h-new-$3-$d.tsv | grep '^[<>]' | sort -k2,3 -s | head -80
done
