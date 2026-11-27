# ElevenLabs kit (Remotion version, 36 s, 1080x1350)

`flowmaid-linkedin-video-final.zip` is the final state: the voice-over (ElevenLabs voice "Eric") and sound
effects are in the project, and `deliverables/final/` holds the finished film, captions and cover.
It is the full Remotion project of the flowmaid-web-design video, packaged for a local
Claude Code to add an ElevenLabs voice-over. It supersedes `../shotcraft/` as the source of truth.

Inside the zip:
- `CLAUDE.md` (English, for the AI) and `MULAI.md` (Indonesian, for the owner).
- `.claude/skills/flowmaid-voiceover`, `.claude/skills/flowmaid-sfx` and `.mcp.json` (hosted ElevenLabs MCP, OAuth, no key).
- `scripts/` with the Node pipeline: voices, tts (with-timestamps), import from the web app, fit (captions follow the speech),
  sfx plan/gen, loudnorm, and a local mock of the API (`npm run mock:on` / `mock:off`).
- `deliverables/silent/`: the approved silent master, cover, cue sheet and captions.

Start: unzip, `npm ci`, copy `.env.example` to `.env` and fill in the key, run `claude`, then say
"Baca CLAUDE.md lalu buatkan voice-over ElevenLabs untuk video ini."
