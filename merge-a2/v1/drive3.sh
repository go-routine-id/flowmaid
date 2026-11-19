#!/usr/bin/env bash
# drive.sh <probe> <list> <out>: one probe run per file, 120 s each; failures recorded as rows
probe=$1; list=$2; out=$3; : > "$out"
while IFS= read -r f; do
  line=$(timeout 1800 "$probe" "$f" 2>/dev/null); rc=$?
  if [ $rc -eq 0 ]; then printf '%s\n' "$line" >> "$out"; else printf '%s\tEXIT=%s\n' "$f" "$rc" >> "$out"; fi
done < "$list"
