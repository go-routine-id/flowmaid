#!/bin/bash
# Render frames + SFX and mux the final MP4 (BT.709 tagged, ~-19 LUFS, true peak <= -1.5 dBTP).
set -euo pipefail
cd "$(dirname "$0")"
V=..; rm -rf $V/frames
node render.js $V/frames
DUR=$(node -e "console.log(require('fs').readFileSync('compose.html','utf8').match(/const DUR = ([0-9.]+)/)[1])")
python3 sfx.py $DUR
I=$(ffmpeg -hide_banner -nostats -i sfx.wav -af ebur128 -f null - 2>&1 | awk '/Summary/{s=1} s&&/I:/{print $2; exit}')
G=$(python3 -c "print(round(-19.0-($I),2))")
echo "sfx integrated $I LUFS -> gain $G dB"
ffmpeg -hide_banner -loglevel error -y -framerate 30 -i $V/frames/%05d.png -i sfx.wav \
  -filter:v "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p" \
  -filter:a "volume=${G}dB,alimiter=limit=0.84:attack=2:release=60:level=false" \
  -c:v libx264 -preset slow -crf 18 -profile:v high -level 4.1 \
  -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv \
  -movflags +faststart -c:a aac -b:a 192k -ar 48000 -shortest $V/flowmaid-linkedin.mp4
ffmpeg -hide_banner -nostats -i $V/flowmaid-linkedin.mp4 -af ebur128=peak=true -f null - 2>&1 | sed -n '/Summary/,$p' | grep -E "I:|Peak:"
ffprobe -v error -show_entries format=duration,size:stream=codec_name,width,height,color_space,color_primaries,color_transfer -of compact $V/flowmaid-linkedin.mp4
