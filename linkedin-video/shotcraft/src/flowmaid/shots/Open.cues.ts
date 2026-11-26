// Cue points of shot `open` (LOCAL frames, 0 = SHOTS.open.from), as rendered by shots/Open.tsx.
import type { LocalCue } from '../timeline';

export const CUES: LocalCue[] = [
  { at: 0, action: 'accent crosshair starts drawing (vertical 0-9, horizontal 8-18)', sound: 'soft pen tick / quiet' },
  { at: 10, action: 'first wordmark letter presses in (letterpress run 10-43, one glyph every 3 f)', sound: 'soft letterpress thuds, light, accelerating' },
  { at: 28, action: 'kicker "OPEN-SOURCE RUST CRATE" types (28-43)', sound: 'light keyboard ticks 28-43' },
  { at: 43, action: 'last glyph lands - lockup complete', sound: 'soft transition / tone bed, no hit' },
  { at: 46, action: '1 s brand hold begins (static to 74)', sound: 'tone bed continues, no hit' },
  { at: 76, action: 'wordmark visibly lifts and docks into the header (dock 74-87, header rules draw 74-88)', sound: 'short whoosh up' },
  { at: 87, action: 'wordmark lands in the header slot', sound: 'tiny click/tick' },
];
