// Cue points of shot `outro` (LOCAL frames, 0 = SHOTS.outro.from). Final values as rendered by
// shots/Outro.tsx (package D). Card sound phrase: riser into the assembly -> impact on the
// wordmark stamp (film peak) -> light accent on the rule.
import type { LocalCue } from '../timeline';

export const CUES: LocalCue[] = [
  { at: 14, action: 'first tile flies in (6 tiles, one every 3 f)', sound: 'riser into the assembly, 28 f' },
  { at: 41, action: 'last tile lands (6 landings, one every 3 f, ending here)', sound: 'optional soft thuds on landings, quieter each' },
  { at: 42, action: 'wordmark letterpress starts (tiles step back over 8 f)', sound: 'impact / deep whoosh on the stamp (film peak)' },
  { at: 44, action: 'eyebrow OPEN SOURCE · GPL-3.0-OR-LATER types (18 f)', sound: 'none (under the impact tail)' },
  { at: 58, action: 'accent rule grows under the wordmark (gutter lines shoot 6 f later)', sound: 'soft shimmer or none' },
  { at: 70, action: '`$ cargo add flowmaid` types (36 f, 2 f per char)', sound: 'keyboard ticks, 36 f' },
  { at: 100, action: 'links rise in (two lines, 6 f apart)', sound: 'none' },
  { at: 120, action: 'final lockup complete (caret off) - hold to the end, fully static for the last 1 s', sound: 'silence / tail' },
];
