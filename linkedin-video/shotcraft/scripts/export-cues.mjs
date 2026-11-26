// Export the cue sheet, the caption file and the VO caption windows from the SAME timeline the
// film renders from (src/flowmaid/timeline.ts -> SHOTS / CAPTIONS / CUES).
//   node scripts/export-cues.mjs   -> out/cues.csv, out/cues.md, out/captions.srt
// Bundles src/flowmaid/cues.ts (data only) with the local esbuild, imports it, writes files.
import { build } from 'esbuild';
import { mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const tmp = join(root, 'out', '.cues-bundle.mjs');
mkdirSync(join(root, 'out'), { recursive: true });
await build({ entryPoints: [join(root, 'src/flowmaid/cues.ts')], bundle: true, platform: 'node', format: 'esm', outfile: tmp, logLevel: 'warning' });
const m = await import(pathToFileURL(tmp).href + `?t=${process.hrtime.bigint()}`);
rmSync(tmp, { force: true });

const { CUES, VO_WINDOWS, SHOTS, FPS } = m;
const fps = FPS;
const secs = (frame) => (frame / fps).toFixed(3);

// ── out/cues.csv: frame,time_s,shot,event,suggested_sound ──
const csvEsc = (s) => (/[",\n]/.test(String(s)) ? `"${String(s).replace(/"/g, '""')}"` : String(s));
const csv =
  ['frame,time_s,shot,event,suggested_sound', ...CUES.map((c) => [c.frame, secs(c.frame), c.shot, c.action, c.sound].map(csvEsc).join(','))].join('\n') +
  '\n';
writeFileSync(join(root, 'out', 'cues.csv'), csv);

// ── out/captions.srt: exactly the burned-in caption windows (CAPTIONS) ──
const srtTime = (frame) => {
  const ms = Math.round((frame / fps) * 1000);
  const h = Math.floor(ms / 3600000);
  const mi = Math.floor((ms % 3600000) / 60000);
  const s = Math.floor((ms % 60000) / 1000);
  const r = ms % 1000;
  const p = (n, w = 2) => String(n).padStart(w, '0');
  return `${p(h)}:${p(mi)}:${p(s)},${p(r, 3)}`;
};
const srt = VO_WINDOWS.map((c, i) => `${i + 1}\n${srtTime(c.from)} --> ${srtTime(c.to)}\n${c.text}\n`).join('\n');
writeFileSync(join(root, 'out', 'captions.srt'), srt);

// ── out/cues.md ──
const total = Object.values(SHOTS).reduce((n, s) => Math.max(n, s.end), 0);
const md = [
  '# flowmaid promo: cue sheet',
  '',
  `Silent master \`out/flowmaid-promo.mp4\`, 1080x1350 @ ${fps} fps, ${total} frames (${(total / fps).toFixed(2)} s), no audio track.`,
  'Generated from `src/flowmaid/timeline.ts` (SHOTS) and the per-shot `shots/<Name>.cues.ts` by `node scripts/export-cues.mjs`.',
  'Frames are absolute (frame 0 = first frame); `time_s` = frame / 30; `mm:ss.ss` is the same time for editors.',
  'Burned-in captions are also written as `out/captions.srt` (same windows as the table below).',
  'Durations inside event / sound text are in film frames (30 f = 1 s); "clip f" / "src" numbers are frames of the',
  'source clip, not of the film. In shot 1 (open, from frame 0) frame ranges in the text are absolute.',
  '',
  '**Thumbnail:** frame 0 is the empty dotted canvas the opening draws on, so do not let LinkedIn pick the first',
  `frame as the poster: upload \`out/cover.png\` (frame ${total - 1}, the final lockup with the VO1 line) as the custom thumbnail.`,
  '',
  '## Shots',
  '',
  '| # | shot | from | to | duration | mode |',
  '|---|------|------|----|----------|------|',
  ...Object.values(SHOTS).map(
    (s) => `| ${s.index + 1} | ${s.id} | ${s.from} (${secs(s.from)} s) | ${s.end} (${secs(s.end)} s) | ${s.duration} f (${(s.duration / fps).toFixed(2)} s) | ${s.mode} |`,
  ),
  '',
  '## Voice-over windows (burned-in captions)',
  '',
  'Fit each recorded VO line inside its window (or move the window: `lead` / `tail` in `VO_LINES`, timeline.ts).',
  '',
  '| VO | caption | from | to | on screen |',
  '|----|---------|------|----|-----------|',
  ...VO_WINDOWS.map((c) => `| ${c.vo} | ${c.text} | ${c.from} (${secs(c.from)} s) | ${c.to} (${secs(c.to)} s) | ${((c.to - c.from) / fps).toFixed(2)} s |`),
  '',
  '## Visual hits',
  '',
  '| frame | time_s | mm:ss.ss | shot | event | suggested sound |',
  '|-------|--------|----------|------|-------|-----------------|',
  ...CUES.map((c) => `| ${c.frame} | ${secs(c.frame)} | ${c.time} | ${c.shot} | ${c.action.replace(/\|/g, '\\|')} | ${c.sound.replace(/\|/g, '\\|')} |`),
  '',
].join('\n');
writeFileSync(join(root, 'out', 'cues.md'), md);
console.log(`wrote out/cues.csv (${CUES.length} cues), out/captions.srt (${VO_WINDOWS.length} captions) and out/cues.md`);
