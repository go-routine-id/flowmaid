// Cue points of shot `types` (LOCAL frames, 0 = SHOTS.types.from). Derived from the beat
// constants the shot renders from (shots/b/beats.ts): the blueprint bed starts on the first
// clearly visible stroke (BLUEPRINT_VISIBLE), row completions are the frames where the eased scan
// edge passes each row's bottom (ROW_DONE ≈ 58, 61, 64, 72), and the counter lands on 12/12 and
// turns accent on one frame (COUNTER_DONE), 5-6 f after row 4.
import type { LocalCue } from '../timeline';
import { BLUEPRINT_VISIBLE, COUNTER_DONE, ROW_DONE, TYPES_BEATS as B } from './b/beats';

const row = (r: number) => Math.round(ROW_DONE[r]);
const blueprintEnd = B.breathAt;

export const CUES: LocalCue[] = [
  {
    at: BLUEPRINT_VISIBLE,
    action: `blueprint: grid rules, then 12 wire diagrams draw on (${blueprintEnd - BLUEPRINT_VISIBLE} f)`,
    sound: `pencil / fine line scratch bed, ${blueprintEnd - BLUEPRINT_VISIBLE} f`,
  },
  { at: B.scanAt, action: 'accent scan line starts top -> bottom (diagrams materialise)', sound: `scanner sweep, ${B.scanDur} f` },
  { at: row(0), action: 'row 1 materialised (flowchart, sequence, class) - counter rolls to 03/12', sound: 'light tick' },
  { at: row(1), action: 'row 2 materialised (state, ER, pie) - counter rolls to 06/12', sound: 'light tick' },
  { at: row(2), action: 'row 3 materialised (journey, git graph, mindmap) - counter rolls to 09/12', sound: 'light tick' },
  { at: row(3), action: 'row 4 materialised (timeline, sankey, architecture)', sound: 'light tick' },
  { at: COUNTER_DONE, action: 'counter lands on 12/12 and turns accent - full wall holds to the cut', sound: 'settle (soft, final)' },
];
