# LinkedIn video (English, 34.5 s, 1080x1350)

- `flowmaid-linkedin.mp4`: the final video (H.264 high, BT.709, AAC, about -19 LUFS).
- `flowmaid-linkedin-cover.png`: cover image to upload as the LinkedIn thumbnail.
- `post.md`: the English caption.

## How it is built

- `build/compose.html` draws the whole video as a page.
  - `window.render(t)` draws frame t.
  - `window.events()` returns every sound cue on the same timeline, so sound and picture cannot drift apart.
- `build/render.js` screenshots every frame with Playwright at 30 fps. It also writes `events.json`.
  - It reads `build/clips.json`, which holds the clip folders, the frame segments shown, the crops, the sound cues and the playground URL.
- `build/sfx.py` makes all the sound effects from `events.json`. It uses numpy only, with no samples.
- `build/make.sh` runs everything and encodes the MP4.

## Clips

The three app clips are real recordings: `capture/web`, `capture/flowcli` and `capture/dbx` (take2 is the clip that is used).

- **web**: the flowmaid-web playground, served locally from its repo (commit 849b8fa). Driven by Playwright. The mouse pointer is an overlay.
- **flowcli**: built from its repo and run in tmux. The screen is captured with `capture-pane -e`, converted from ANSI to HTML and then to PNG.
- **dbx**: dbx 0.5.10 on a demo SQLite DB made by `mkdb.py`. Same capture pipeline as flowcli.
  - dbx's ERD label painter overprints labels on short edges.
  - take2 frames 19-26 still show this and are skipped (see the segments in `clips.json`).
  - `take2/check.py` verifies that every shown frame is clean.

The clip frame PNGs are not stored here, because they are large. Re-record them with the capture scripts, then run `build/make.sh`. The scripts hard-code the scratchpad path, so edit `S` if it changes.
