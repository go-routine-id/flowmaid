#!/usr/bin/env bash
# clirun.sh <cli> <list> <out>: default and --compact 600 SVG hashes, exit codes
cli=$1; list=$2; out=$3; : > "$out"; tmp=$(mktemp -d)
while IFS= read -r f; do
  r1=$(timeout 120 "$cli" "$f" -o "$tmp/a.svg" > /dev/null 2> "$tmp/e1"; echo $?); h1=$( [ -f "$tmp/a.svg" ] && md5sum < "$tmp/a.svg" | cut -c1-16 ); rm -f "$tmp/a.svg"
  r2=$(timeout 120 "$cli" "$f" --compact 600 -o "$tmp/b.svg" > /dev/null 2> "$tmp/e2"; echo $?); h2=$( [ -f "$tmp/b.svg" ] && md5sum < "$tmp/b.svg" | cut -c1-16 ); rm -f "$tmp/b.svg"
  e=$(sed "s#$tmp##g" "$tmp/e1" "$tmp/e2" | md5sum | cut -c1-8)
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$f" "$r1" "$h1" "$r2" "$h2" "$e" >> "$out"
done < "$list"
rm -rf "$tmp"
