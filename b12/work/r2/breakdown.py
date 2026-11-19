import json, sys
def load(path, rec=0):
    return json.loads(open(path).read().split("\n====\n")[rec])
for path in sys.argv[1:]:
    d = load(path); N = d["nodes"]; nl = d["nlayers"]
    pos = [n["pos"] for n in N]
    tot = wall = 0
    pergap = []
    for l in range(nl-1):
        es = [(u, v) for u, n in enumerate(N) if n["l"] == l for v in n["succs"] if N[v]["l"] == l+1]
        g = gw = 0
        for i in range(len(es)):
            for j in range(i+1, len(es)):
                a, b = es[i], es[j]
                if (pos[a[0]] - pos[b[0]]) * (pos[a[1]] - pos[b[1]]) < 0:
                    g += 1
                    if N[a[0]]["border"] or N[b[0]]["border"]: gw += 1
        tot += g; wall += gw; pergap.append((g, gw))
    print(path, "layered", tot, "of which wall", wall, "edge-edge", tot-wall, "per gap", pergap)
