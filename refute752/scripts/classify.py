import sys, re, subprocess, os
D = sys.argv[1]
files = sys.argv[3:]
mode = sys.argv[2]
pt = re.compile(r"\(([-\d.]+),([-\d.]+)\)")
def corners(w):
    if len(w) < 3: return []
    out = []
    for k in range(1, len(w)-1):
        a, c, b = w[k-1], w[k], w[k+1]
        d1 = (c[0]-a[0], c[1]-a[1]); d2 = (b[0]-c[0], b[1]-c[1])
        h1 = abs(d1[1]) < 0.02; h2 = abs(d2[1]) < 0.02
        v1 = abs(d1[0]) < 0.02; v2 = abs(d2[0]) < 0.02
        if (h1 and v2) or (v1 and h2):
            out.append(c)
    return out
def eq(p, q): return abs(p[0]-q[0]) < 0.02 and abs(p[1]-q[1]) < 0.02
tot = {}
for f in files:
    args = [D, f] + (["route"] if mode == "route" else [])
    env = dict(os.environ, FMTRACE="1")
    r = subprocess.run(args, capture_output=True, text=True, env=env, timeout=120)
    tr = {}
    for line in r.stderr.splitlines():
        m = re.match(r"(REPAIR|NUDGE|SPREAD) (\d+) ", line)
        if m: tr.setdefault(int(m.group(2)), set()).add(m.group(1))
    for line in r.stdout.splitlines():
        m = re.match(r"#(\d+) (\S+) bez=\[(.*?)\](.*?) wps\((\d+)\)=(.*)", line)
        if not m: continue
        i = int(m.group(1)); n = int(m.group(5))
        if n == 0: continue
        bez = [(float(a), float(b)) for a, b in pt.findall(m.group(3))]
        w = [(float(a), float(b)) for a, b in pt.findall(m.group(6))]
        cs = corners(w)
        if not cs: kind = "nocorner"
        elif eq(bez[1], cs[0]) and eq(bez[2], cs[-1]): kind = "firstlast"
        elif eq(bez[1], bez[0]) and eq(bez[2], bez[3]): kind = "straight"
        else: kind = "other"
        t = ",".join(sorted(tr.get(i, [])))
        key = (kind, t)
        tot[key] = tot.get(key, 0) + 1
        if kind in ("other",) or (kind == "straight" and t in ("", "SPREAD")):
            print(f"{os.path.basename(f)} #{i} {m.group(2)} {kind} trace={t} bez={bez} corners={cs[:2]}..{cs[-2:]}")
for k, v in sorted(tot.items()): print(k, v)
