import random, sys, os
out = sys.argv[1]; n_files = int(sys.argv[2]); seed0 = int(sys.argv[3])
words = ["api","queue","worker","cache","db","auth","billing","ship","pack","audit","index","search","mail","report","ingest","parse","store","merge","split","notify"]
titles = ["Core","Edge services","Data tier","Back office and finance","Ops","Fulfilment desk","Analytics","Ingest","Platform","Storefront"]
os.makedirs(out, exist_ok=True)
for k in range(n_files):
    r = random.Random(seed0 + k)
    d = r.choice(["TD","TD","BT","LR","RL"])
    nb = r.randint(2, 4)
    lines = [f"flowchart {d}"]
    nodes = []
    blocks = []
    nid = 0
    def node():
        global nid
        nid += 1
        w = r.choice(words)
        if r.random() < 0.25:
            w = w + " " + r.choice(words) + " " + r.choice(words)
        if r.random() < 0.15:
            w = w + "<br/>" + r.choice(words)
        return f"n{nid}", w
    outside = []
    for _ in range(r.randint(1, 3)):
        i, w = node(); outside.append(i); lines.append(f'  {i}["{w}"]')
    for b in range(nb):
        bid = f"B{b}"
        lines.append(f'  subgraph {bid}["{r.choice(titles)}"]')
        members = []
        nest = r.random() < 0.45
        for _ in range(r.randint(1, 4)):
            i, w = node(); members.append(i); lines.append(f'    {i}["{w}"]')
        if nest:
            for s in range(r.randint(1, 2)):
                sid = f"B{b}s{s}"
                lines.append(f'    subgraph {sid}["{r.choice(titles)} {s}"]')
                for _ in range(r.randint(1, 3)):
                    i, w = node(); members.append(i); lines.append(f'      {i}["{w}"]')
                lines.append("    end")
                blocks.append(sid)
        lines.append("  end")
        blocks.append(bid)
        nodes.extend(members)
    for _ in range(r.randint(1, 2)):
        i, w = node(); outside.append(i); lines.append(f'  {i}["{w}"]')
    allnodes = nodes + outside
    ne = r.randint(len(allnodes), int(len(allnodes) * 1.6))
    seen = set()
    for _ in range(ne):
        a = r.choice(allnodes); b = r.choice(allnodes + (blocks if r.random() < 0.15 else []))
        if a == b or (a, b) in seen:
            continue
        seen.add((a, b))
        arrow = r.choice(["-->", "-->", "-->", "-.->", "==>"])
        if r.random() < 0.3:
            lines.append(f"  {a} {arrow}|{r.choice(words)}| {b}")
        else:
            lines.append(f"  {a} {arrow} {b}")
    open(os.path.join(out, f"g{seed0+k:04d}.mmd"), "w").write("\n".join(lines) + "\n")
