import random, os, sys
out = sys.argv[1]; N = int(sys.argv[2])
titles = ["Shipping and returns desk", "Order processing", "Identity provider", "Monitoring stack",
          "Inventory management", "WWW Mammoth Wombat", "Customer data platform", "Billing", "Édition générale"]
for i in range(N):
    r = random.Random(9100 + i)
    d = r.choice(["TD", "TD", "BT"])
    t = r.choice(titles)
    na, nb = r.randint(2, 3), r.randint(2, 3)
    L = [f"flowchart {d}", f"    subgraph A[{t}]"]
    for j in range(na):
        L.append(f"        a{j}[a{j}{' wide node' if r.random()<0.4 else ''}]")
    L += ["    end", f"    subgraph B[{t} {r.choice(['two','backend','east','x'])}]"]
    for j in range(nb):
        L.append(f"        b{j}[b{j}{' wide node' if r.random()<0.4 else ''}]")
    L.append("    end")
    es = set()
    for _ in range(r.randint(2, 5)):
        es.add((r.randrange(na), r.randrange(nb)))
    for a, b in sorted(es):
        lab = ' -- "x" ' if r.random() < 0.1 else ' '
        L.append(f"    a{a}{lab}--> b{b}")
    for k, nm in enumerate(["p", "q", "r"][: r.randint(1, 3)]):
        tgt = r.choice(["A", "B", "B"])
        L.append(f"    {nm}[{nm}] --> {tgt}")
        if r.random() < 0.7:
            L.append(f"    {nm} --> {('a' if tgt=='A' else 'b')}{r.randrange(na if tgt=='A' else nb)}")
    open(os.path.join(out, f"n{i:03d}-{d}.mmd"), "w").write("\n".join(L) + "\n")
