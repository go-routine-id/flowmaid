import sys, glob, os, re
sys.path.insert(0, os.path.dirname(sys.argv[0]))
from collections import Counter
import importlib.util
spec = importlib.util.spec_from_file_location("cmp", os.path.join(os.path.dirname(sys.argv[0]), "cmp.py"))
cmp = importlib.util.module_from_spec(spec); spec.loader.exec_module(cmp)
root = sys.argv[1]
kinds = Counter(); files = Counter(); other = []
for v in sorted(glob.glob(f"{root}/v.*.tsv")):
    n = v.split(".")[-2]
    for line in open(v):
        fid, mode, verdict, path = line.rstrip("\n").split("\t")
        if verdict == "TRANSLATION": continue
        files[verdict] += 1
        if verdict != "MISMATCH":
            other.append((verdict, path, mode)); continue
        b = open(f"{root}/b.{n}/{fid}.{mode}.svg").read(); h = open(f"{root}/h.{n}/{fid}.{mode}.svg").read()
        r = cmp.compare(b, h)
        for e in r["bad"]:
            k = e[2] if e[0] == "num" else e[0] + ":" + e[2][:80]
            kinds[k] += 1
print(files); [print(k, c) for k, c in kinds.most_common()]
print("other verdicts:", other[:20])
