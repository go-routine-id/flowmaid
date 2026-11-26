// Cue points of shot `render` (LOCAL frames, 0 = SHOTS.render.from), as rendered by shots/Render.tsx.
import type { LocalCue } from '../timeline';

export const CUES: LocalCue[] = [
  { at: 2, action: 'code pane draws its 1px frame (SVG pane follows at 6)', sound: 'very soft line swish' },
  { at: 6, action: 'hero.mmd typing starts (one line burst every 4.4 f, 44 f in all)', sound: 'keyboard typing bed, 44 f (1.5 s)' },
  { at: 50, action: 'code confirmed (pane rules turn accent, MERMAID tag filled)', sound: 'short key press' },
  { at: 54, action: 'submit: FLOWMAID tag filled, drip line runs to the root node, camera pull-back starts (72 f)', sound: 'soft click + start of a slow pull-back whoosh' },
  { at: 60, action: 'root node Request pops', sound: 'soft pop (series start)' },
  { at: 72, action: 'Auth? pops', sound: 'soft pop' },
  { at: 80, action: 'Services subgraph frame draws as an outline', sound: 'very soft line swish' },
  { at: 84, action: 'API Gateway pops with its "yes" chip', sound: 'soft pop' },
  { at: 89, action: '401 pops with its "no" chip', sound: 'soft pop, quieter' },
  { at: 96, action: 'Cache pops', sound: 'soft pop' },
  { at: 101, action: 'Database pops', sound: 'soft pop, quieter' },
  { at: 108, action: 'Response pops - tree complete (last arrow lands 5 f later)', sound: 'soft pop, last and lowest' },
  { at: 114, action: 'tree breathes 1.00-1.02-1.00 over 20 f, then static', sound: 'gentle swell, no hit' },
];
