import json, sys
name, dbg = sys.argv[1], sys.argv[2]
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
st = json.load(open(S+'/b12/local/struct/'+name+'.struct.json')); ids=[n["id"] for n in st["nodes"]]
recs=[json.loads(r) for r in open(dbg).read().split("\n====\n") if r.strip()]
d=recs[int(sys.argv[3]) if len(sys.argv)>3 else 0]; N=d["nodes"]
def nm(v):
    n=N[v]
    if n["border"]: return ("[" if n["border"]==1 else "]")+"g%d"%n["path"][-1]
    if n["run"]>=0: return "(%s>%s)"%(ids[n["canon"][0]], ids[n["canon"][2]])
    return ids[v] if v < len(ids) else "?%d"%v
for l in range(d["nlayers"]):
    row=sorted((n["pos"],v) for v,n in enumerate(N) if n["l"]==l)
    print(l, " ".join(nm(v) for _,v in row))
