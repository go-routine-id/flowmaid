import json, sys
# Gaps along the flow between the top-level blocks (and free nodes) at the experiment's positions.
name = sys.argv[1]; B = sys.argv[2]
st = json.load(open(f"{B}/local/struct/{name}.struct.json"))
pos = {}
for l in open(f"{B}/spike/pos/{name}.pos"):
    p = l.rstrip("\n").split("\t")
    pos[p[0]] = tuple(map(float, p[1:5]))
subs = st.get("subgraphs") or st.get("subs") or []
parent = [s.get("parent") for s in subs]
def top(s):
    while parent[s] is not None: s = parent[s]
    return s
horiz = st["direction"] in ("LR", "RL")
units = {}
for n in st["nodes"]:
    if n["id"] not in pos: continue
    x, y, w, h = pos[n["id"]]
    c, half = (x, w / 2) if horiz else (y, h / 2)
    key = ("n", n["id"]) if n["sub"] is None else ("b", top(n["sub"]))
    lo, hi = units.get(key, (1e18, -1e18))
    units[key] = (min(lo, c - half), max(hi, c + half))
iv = sorted(units.values())
bands = []
for lo, hi in iv:
    if bands and lo < bands[-1][1]: bands[-1][1] = max(bands[-1][1], hi)
    else: bands.append([lo, hi])
gaps = [round(bands[i + 1][0] - bands[i][1], 1) for i in range(len(bands) - 1)]
print(name, st["direction"], "bands", len(bands), "gaps between member boxes of consecutive bands:", gaps)
