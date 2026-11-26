// Cue points of shot `browser` (LOCAL frames, 0 = SHOTS.browser.from), derived from the camera
// beats in shots/c/browserCam.ts (web.mp4 shown at src = local - 6, capped at src 134).
import type { LocalCue } from '../timeline';
import { ANT, CORNERS_AT, HIT, PUSH, RELEASE, localOfSrc } from './c/browserCam';

export const CUES: LocalCue[] = [
  { at: CORNERS_AT, action: 'accent lock corners contract onto the drag region (16 f)', sound: 'soft focus click' },
  { at: ANT, action: `anticipation pull-back (${PUSH - ANT} f)`, sound: 'short inhale / reverse swell' },
  { at: PUSH, action: `crash zoom 1x -> 2.12x (${HIT - PUSH} f, speed peak on the landing frame)`, sound: 'fast whoosh into a soft stop' },
  { at: HIT, action: 'crash lands; cursor presses the Auth? node on the same frame (src 52); 3% bounce settles over 12 f', sound: 'impact thump + mouse click' },
  { at: localOfSrc(60), action: 'drag starts: Auth? travels left, its 3 edges re-route every frame (clip src 60-112, 52 f)', sound: 'subtle continuous drag texture, 52 f' },
  { at: RELEASE, action: 'drag released (src 117), node toolbar pops back; still from here to the cut', sound: 'mouse release click' },
];
