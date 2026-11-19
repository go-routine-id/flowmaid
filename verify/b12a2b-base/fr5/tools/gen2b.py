import random, os, sys
out = sys.argv[1]; N = int(sys.argv[2])
titles = ["Shipping and returns desk", "Édition générale des données", "WWW Mammoth Wombat Overview",
          "Order processing", "Core", "Data ingestion workers", "MMMM WWWW QQQQ", "Payments"]
mems = ["queue", "charge", "x wide member node", "y", "ledger", "w", "auth service", "db"]
for i in range(N):
    r = random.Random(7000 + i)
    d = r.choice(["TD", "TD", "BT", "LR", "RL"])
    nested = r.random() < 0.35
    t1, t2 = r.sample(titles, 2)
    k = r.randint(2, 3)
    m = [f"m{j}[{r.choice(mems)}]" for j in range(k)]
    lines = [f"flowchart {d}"]
    srcs = [f"s{j}[src{j}]" for j in range(r.randint(2, 5))]
    body = []
    if nested:
        body += [f"    subgraph O[{t1}]", f"        subgraph G[{t2}]"] + [f"            {x}" for x in m] + ["        end", "        k[k]", "    end"]
        for j in range(k):
            if r.random() < 0.6: body.append(f"    m{j} --> k")
        tgts = [f"m{j}" for j in range(k)] + ["G", "O", "G"]
    else:
        body += [f"    subgraph G[{t1}]"] + [f"        {x}" for x in m]
        if k >= 2 and r.random() < 0.7: body.append("        m0 --> m1")
        body.append("    end")
        tgts = [f"m{j}" for j in range(k)] + ["G", "G"]
    lines += body
    first = True
    for s in srcs:
        for _ in range(r.randint(1, 2)):
            t = r.choice(tgts)
            lab = f' -- "lbl{r.randint(1,9)}" ' if r.random() < 0.15 else " "
            sd = s if first else s.split("[")[0]
            lines.append(f"    {sd}{('-->' if lab==' ' else '').join([''])}" if False else f"    {sd}{lab}--> {t}" if lab == " " else f"    {sd}{lab}--> {t}")
            first = False
        first = True
    if r.random() < 0.5: lines.append(f"    m{k-1} --> z[end]")
    if r.random() < 0.3: lines.append("    G --> z2[after]")
    open(os.path.join(out, f"r{i:03d}-{d}.mmd"), "w").write("\n".join(lines) + "\n")
