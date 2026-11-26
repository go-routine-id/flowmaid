// Cue points of shot `speed` (LOCAL frames, 0 = SHOTS.speed.from). Values come from the same
// beat constants the shot renders from (shots/b/beats.ts), so they cannot drift.
import type { LocalCue } from '../timeline';
import { SPEED_BEATS as B } from './b/beats';

export const CUES: LocalCue[] = [
  { at: B.footAt, action: 'footnote fades in with the readout and stays (geometric mean over 25 sample diagrams)', sound: 'none' },
  { at: B.breakAt, action: 'mermaid.js 1x limit line snaps (halves slide apart)', sound: 'small dry snap' },
  { at: B.antAt, action: 'anticipation pull-back of the tape', sound: 'soft tension / inhale' },
  { at: B.sprintAt, action: 'tape sprints up the log scale (blurred), readout rolls 1 -> 122', sound: 'fast mechanical roll / riser, 22 f' },
  { at: B.brakeAt, action: 'tape brakes and swings back once (122 -> 112 -> 113)', sound: 'dull mechanical clunk' },
  { at: B.lockAt, action: 'readout locks at ~113x on the FLOWMAID marker', sound: 'firm impact, the speed hit' },
  { at: B.claimIn, action: `'FASTER THAN MERMAID.JS' rises under the readout (locked composition complete at the still point ${B.stillAt - B.claimIn} f later; fully still to the cut)`, sound: 'none (under the lock hit)' },
];
