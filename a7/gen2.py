# Realistic mindmaps: root with B branches, each with 2-8 topics, each with 0-5 leaves, some with a 4th level.
import random,sys
words="plan design research budget launch review hiring feedback testing docs legal sales marketing support metrics ops".split()
def lab(r): return " ".join(r.choice(words) for _ in range(r.randint(1,3)))
for seed in range(int(sys.argv[1]), int(sys.argv[2])):
    r=random.Random(seed)
    B=r.randint(4,10)
    out=["mindmap","  root((Central idea))"]
    for b in range(B):
        out.append("    "+lab(r).title()+f" {b}")
        for t in range(r.randint(1,7)):
            out.append("      "+lab(r))
            for l in range(r.choice([0,0,1,2,3,4,5])):
                out.append("        "+lab(r))
                if r.random()<0.15:
                    for q in range(r.randint(1,3)):
                        out.append("          "+lab(r))
    open(f"real/m{seed}.mmd","w").write("\n".join(out)+"\n")
