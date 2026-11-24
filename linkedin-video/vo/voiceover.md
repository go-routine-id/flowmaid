# flowmaid LinkedIn video: voice-over script

English narration. 8 lines, 71 words. At a calm pace of about 2.6 words per second, it fits the current 34.5 s video.
Voice: plain and calm, engineer to engineer. No hype and no exclamation marks, as in the Arian Saputra brand voice.

## Script and timing

| # | Scene | Start | Slot | Estimated | On screen | Line (caption text) |
|---|---|---|---|---|---|---|
| 1 | 1 Hook | 0.2 s | 3.1 s | 2.7 s | headline, code typing | flowmaid renders Mermaid diagrams in pure Rust. |
| 2 | 1 Code → SVG | 3.5 s | 2.2 s | 2.2 s | the diagram draws itself | Text in, clean SVG out. |
| 3 | 1 Speed | 5.9 s | 2.9 s | 2.9 s | ~113× faster than mermaid.js | Over a hundred times faster than mermaid.js. |
| 4 | 2 Types | 8.8 s | 2.8 s | 2.9 s | grid of 12 diagram types | Twelve diagram types, from flowcharts to sankey. |
| 5 | 3 Browser | 11.9 s | 5.1 s | 4.3 s | flowmaid-web, a node is dragged | In the browser, drag a node, and its edges follow. |
| 6 | 4 flowcli | 17.5 s | 4.9 s | 4.5 s | flowcli in the terminal | flowcli draws them in your terminal, and redraws when you save. |
| 7 | 5 dbx | 22.9 s | 4.9 s | 4.5 s | dbx ERD | In dbx, flowmaid lays out your database schema as an ERD. |
| 8 | 6 CTA | 28.3 s | 6.0 s | 5.5 s | end card with links | flowmaid is open source. Try it in your browser, or cargo add flowmaid. |

Line 3 runs about 0.8 s past the cut to the 12 types. That is fine: when the video is fitted to your voice, the cuts move with it.

## Text for ElevenLabs

Generate **one clip per line**. They are easy to place at the start times above and easy to redo one at a time. In this text, abbreviations are spelled out so the voice reads them correctly:

1. flowmaid renders Mermaid diagrams in pure Rust.
2. Text in, clean S V G out.
3. Over a hundred times faster than mermaid J S.
4. Twelve diagram types, from flowcharts to sankey.
5. In the browser, drag a node, and its edges follow.
6. flow C L I draws them in your terminal, and redraws when you save.
7. In D B X, flowmaid lays out your database schema as an E R D.
8. flowmaid is open source. Try it in your browser, or cargo add flowmaid.

If you prefer one take, paste the text below. `<break>` tags work in the v2 / v2.5 models (Multilingual, Turbo, Flash). If your model ignores them, generate one clip per line.

```
flowmaid renders Mermaid diagrams in pure Rust. <break time="0.6s" /> Text in, clean S V G out. <break time="0.3s" /> Over a hundred times faster than mermaid J S. <break time="0.3s" /> Twelve diagram types, from flowcharts to sankey. <break time="0.3s" /> In the browser, drag a node, and its edges follow. <break time="1.2s" /> flow C L I draws them in your terminal, and redraws when you save. <break time="0.9s" /> In D B X, flowmaid lays out your database schema as an E R D. <break time="0.9s" /> flowmaid is open source. Try it in your browser, or cargo add flowmaid.
```

## Pronunciation

- **flowmaid**: "flow-maid". If it comes out wrong, write it as `flow maid`.
- **mermaid.js**: write `mermaid J S`.
- **SVG**, **ERD**, **dbx**: spelled out as `S V G`, `E R D`, `D B X`.
- **flowcli**: `flow C L I`.
- **cargo add flowmaid**: as written.

## Why each line is safe to say

- "Over a hundred times faster than mermaid.js": the README benchmark measures about 113× (geometric mean over 25 diagrams), and the exact number is on screen.
- "Twelve diagram types": the README list.
- "In the browser, drag a node, and its edges follow": flowmaid-web, as filmed.
- "flowcli … redraws when you save": `flowcli --watch`, as filmed.
- "flowmaid lays out your database schema as an ERD": dbx lays out its ERD with `flowmaid::er`.
- "Open source": GPL-3.0-or-later. No URL is read aloud; the links are on screen and in the post.

## Captions

`captions.srt` holds the same lines at the same times. Upload it to LinkedIn as the video's captions, since most people watch on mute.

## After you record

Send me the clips, or just how long each one runs. I will fit each scene to its line so the cuts land on your voice, and render the silent video to match.
