// Cue sheet aggregator: maps every shot's LOCAL cues (shots/<Name>.cues.ts) through SHOTS
// and adds the generated transition cues. Exported as out/cues.csv + out/cues.md by
// scripts/export-cues.mjs (`node scripts/export-cues.mjs`). Data only — no React imports.
import { CAPTIONS, SHOTS, TRANSITION_CUES, cue, fmtTime, type Cue, type LocalCue, type ShotId } from './timeline';
import { CUES as open } from './shots/Open.cues';
import { CUES as render } from './shots/Render.cues';
import { CUES as speed } from './shots/Speed.cues';
import { CUES as types } from './shots/Types.cues';
import { CUES as browser } from './shots/Browser.cues';
import { CUES as flowcli } from './shots/Flowcli.cues';
import { CUES as dbx } from './shots/Dbx.cues';
import { CUES as outro } from './shots/Outro.cues';

const LOCAL: Record<ShotId, LocalCue[]> = { open, render, speed, types, browser, flowcli, dbx, outro };

export const CUES: Cue[] = [
  ...(Object.keys(LOCAL) as ShotId[]).flatMap((id) => LOCAL[id].map((c) => cue(id, c.at, c.action, c.sound))),
  ...TRANSITION_CUES,
].sort((a, b) => a.frame - b.frame);

export const VO_WINDOWS = CAPTIONS.map((c) => ({ ...c, fromTime: fmtTime(c.from), toTime: fmtTime(c.to) }));
export { SHOTS };
export { FPS } from "./timeline";
