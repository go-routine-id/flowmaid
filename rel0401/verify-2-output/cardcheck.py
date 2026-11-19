import subprocess, sys, re, random
V = "/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0401/verify-2-output"
rows = [l.rstrip("\n").split("\t") for f in sys.argv[1:] for l in open(f)]
rows = [r for r in rows if r[2] == "MISMATCH"]
random.seed(1); sample = random.sample(rows, min(60, len(rows)))
ok = bad = 0
for fid, mode, _, path in sample:
    args = {"d1.r": ["mid", "-60", "-40"], "d4.r": ["left", "+-150", "+0"], "d9.r": ["0", "all", "-40", "-25"], "d6.r": ["mid", "+25", "+15"], "r0": ["0", "all", "0", "0"]}[mode]
    out = subprocess.run([V + "/cards-head", path] + args + ["/dev/null"], capture_output=True, text=True).stdout
    spots = [tuple(map(float, m)) for m in re.findall(r"spot rel \d+ end \d+: \(([-\d.]+), ([-\d.]+)\)", out)]
    vb = None
    svg_cards = [tuple(map(float, m)) for m in re.findall(r'svg: <text x="([-\d.]+)" y="([-\d.]+)" dy="0.33em" text-anchor="middle" fill="[^"]*" font-size="11"', out)]
    miss = [c for c in svg_cards if not any(abs(c[0]-s[0]) <= 0.06 and abs(c[1]-s[1]) <= 0.06 for s in spots)]
    if miss: bad += 1; print("not at a spot:", path.split("/")[-1], mode, miss[:3])
    else: ok += 1
print("cards at card_spots:", ok, "files; not:", bad)
