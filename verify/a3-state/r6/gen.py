import random, sys
out = sys.argv[1]; n = int(sys.argv[2])
for k in range(n):
    r = random.Random(k)
    d = r.choice(["", "direction LR", "direction RL", "direction BT", ""])
    L = ["stateDiagram-v2"]
    if d: L.append(d)
    states = []
    L.append("[*] --> S0"); states.append("S0")
    ncomp = r.randint(1, 2)
    comps = []
    for c in range(ncomp):
        cn = f"C{c}"; comps.append(cn)
        L.append(f"state {cn} {{")
        inner = [f"{cn}i{j}" for j in range(r.randint(2, 4))]
        L.append(f"  [*] --> {inner[0]}")
        for j in range(1, len(inner)):
            L.append(f"  {inner[r.randint(0, j-1)]} --> {inner[j]}")
        for _ in range(r.randint(1, 2)):
            L.append(f"  note {r.choice(['right of','left of'])} {r.choice(inner)} : n{r.randint(0,99)}")
        L.append("}")
    outs = [f"T{j}" for j in range(r.randint(1, 3))]
    L.append(f"S0 --> {comps[0]}")
    for j, c in enumerate(comps[1:]):
        L.append(f"{comps[j]} --> {c}")
    for t in outs:
        L.append(f"{r.choice(comps + ['S0'])} --> {t}")
    for _ in range(r.randint(1, 3)):
        L.append(f"note {r.choice(['right of','left of'])} {r.choice(outs + ['S0'])} : m{r.randint(0,99)}")
    if r.random() < 0.5:
        L.append(f"{outs[-1]} --> S0 : back")
    open(f"{out}/r{k:03d}.mmd", "w").write("\n".join(L) + "\n")
