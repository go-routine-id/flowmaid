import random, os, sys
out = sys.argv[1]; N = int(sys.argv[2]); seed0 = int(sys.argv[3])
titles = ["Order processing", "Core", "Payments", "Data ingestion workers", "Edge", "Billing and invoices", "Ops"]
for i in range(N):
    r = random.Random(seed0 + i)
    d = r.choice(["TD", "BT", "LR", "RL", "TD", "BT"])
    ng = r.randint(2, 3)
    lines = [f"flowchart {d}"]
    members = []
    for g in range(ng):
        k = r.randint(2, 4)
        ms = [f"g{g}m{j}" for j in range(k)]
        members.append(ms)
        lines.append(f"    subgraph G{g}[{r.choice(titles)} {g}]")
        if r.random() < 0.25:
            lines.append(f"        subgraph H{g}[inner {g}]")
            lines += [f"            {x}[{x}]" for x in ms[:2]]
            lines.append("        end")
            lines += [f"        {x}[{x}]" for x in ms[2:]]
        else:
            lines += [f"        {x}[{x}]" for x in ms]
        lines.append("    end")
    outs = [f"o{j}" for j in range(r.randint(1, 4))]
    allm = [x for ms in members for x in ms]
    tg = [f"G{g}" for g in range(ng)]
    for _ in range(r.randint(5, 14)):
        a = r.choice(allm + outs + tg[:1])
        b = r.choice(allm + outs + tg)
        if a == b: continue
        arrow = r.choice(["-->", "-->", "-.->", "==>", "-->"])
        if r.random() < 0.12:
            lines.append(f"    {a} -- l{r.randint(1,9)} --> {b}")
        else:
            lines.append(f"    {a} {arrow} {b}")
    open(os.path.join(out, f"q{seed0 + i}-{d}.mmd"), "w").write("\n".join(lines) + "\n")
