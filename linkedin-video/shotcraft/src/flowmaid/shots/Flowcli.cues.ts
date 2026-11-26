// Cue points of shot `flowcli` (LOCAL frames, 0 = SHOTS.flowcli.from), as rendered by
// shots/Flowcli.tsx with the remap in shots/c/flowcliMap.ts (mp4 frame = local - 20 until the freeze).
import type { LocalCue } from '../timeline';

export const CUES: LocalCue[] = [
  { at: 30, action: "shell types the echo line (clip f10-38, 28 f)", sound: 'keyboard typing, 28 f' },
  { at: 60, action: 'Enter pressed (clip f40)', sound: 'key press' },
  { at: 66, action: 'flowcli --watch redraws, 8 -> 9 nodes (clip f46)', sound: 'soft redraw blip / paper flick' },
  { at: 67, action: 'FREEZE: pane flash (3 f) + corner brackets, LIVE -> PAUSED', sound: 'camera shutter / pause click' },
  { at: 76, action: 'marker circle draws round "Email buyer" (10 f)', sound: 'marker scribble 10 f' },
  { at: 86, action: 'arrow shaft (6 f), then head (3 f)', sound: 'short marker stroke' },
  { at: 90, action: "annotation 'redrawn on save' + '8 -> 9 nodes' rise", sound: 'none or soft pen' },
  { at: 102, action: 'hand underline (8 f)', sound: 'short marker stroke' },
  { at: 110, action: 'ink fades, "+1" comment chip pins to the node', sound: 'small pin tick' },
  { at: 118, action: 'unfreeze: status back to LIVE, clip ramps up', sound: 'short rewind-forward whoosh' },
  { at: 124, action: 'click picks Backorder (clip f88; the rest of the diagram dims)', sound: 'mouse click' },
  { at: 128, action: 'drag: Backorder moves up-right, edges re-route (clip f94-121, 15 f, braking)', sound: 'subtle drag texture, 15 f' },
  { at: 143, action: 'drag lands (clip f121), hold to the end', sound: 'mouse release click' },
];
