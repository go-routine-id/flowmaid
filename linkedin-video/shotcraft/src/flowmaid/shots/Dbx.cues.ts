// Cue points of shot `dbx` (LOCAL frames, 0 = SHOTS.dbx.from). Final values as rendered by
// shots/Dbx.tsx (package D). The three flashes are the card's beats (sound-strong dependency):
// pin a shutter on each; the focus locks are the "pre-beat" (corners turn accent 3 f before).
import type { LocalCue } from '../timeline';

export const CUES: LocalCue[] = [
  { at: 27, action: 'focus corners lock on the whole window (accent)', sound: 'tiny mechanical focus tick (optional)' },
  { at: 30, action: 'flash 1 - hard cut to photo B: 4-table ERD (clip f24 frozen)', sound: 'camera shutter 1' },
  { at: 49, action: 'focus corners lock on the ERD panel', sound: 'tiny mechanical focus tick (optional)' },
  { at: 52, action: 'flash 2 - hard cut to photo C: zoomed ERD (clip f62 frozen)', sound: 'camera shutter 2' },
  { at: 67, action: 'focus corners lock on the orders table', sound: 'tiny mechanical focus tick (optional)' },
  { at: 70, action: 'flash 3 - hard cut to photo D: orders close-up (clip f110 frozen)', sound: 'camera shutter 3 (strongest)' },
  { at: 84, action: "HUD 'LAYOUT  flowmaid er::scene' rises (its rules started drawing 4 f earlier)", sound: 'soft tick' },
];
