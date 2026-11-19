import re,sys,itertools
def parse(path):
    nodes=[];idx={};parent={};clusters={};cpar={};stack=[];edges=[]
    def node(n):
        if n not in idx:
            idx[n]=len(nodes);nodes.append(n);parent[n]=stack[-1] if stack else None
        return n
    for line in open(path):
        line=line.split('%%')[0].strip()
        if not line or line.startswith('flowchart') or line.startswith('direction'): continue
        m=re.match(r'subgraph\s+(\w+)',line)
        if m:
            c=m.group(1);cpar[c]=stack[-1] if stack else None;clusters[c]=[];stack.append(c);continue
        if line=='end': stack.pop();continue
        # edges
        parts=re.split(r'\s*(?:-->|-\.->|==>|---|-\.-|===|~~~)(?:\|[^|]*\|)?\s*',line)
        if len(parts)>1:
            ids=[re.match(r'(\w+)',p).group(1) for p in parts]
            for a,b in zip(ids,ids[1:]):
                edges.append((a,b))
            for i in ids:
                if i not in idx and i not in clusters: node(i)
        else:
            m=re.match(r'(\w+)',line)
            if m: node(m.group(1))
    return nodes,idx,parent,clusters,cpar,edges
nodes,idx,parent,clusters,cpar,edges=parse(sys.argv[1])
def path(x):
    p=[];c=parent.get(x) if x in parent else cpar.get(x)
    while c: p.append(c); c=cpar[c]
    return p[::-1]
def unit_at(x,depth_path):
    p=path(x) if x in idx else path(x)+[x]
    if p[:len(depth_path)]!=depth_path: return None
    if len(p)==len(depth_path): return x
    return p[len(depth_path)]
def level(dp):
    w={}
    for a,b in edges:
        if a in clusters or b in clusters:
            # sub-edge: treat cluster as unit directly
            pass
        ua=unit_at(a,dp) if a in idx else (a if cpar.get(a)==(dp[-1] if dp else None) else unit_at(a,dp))
        ub=unit_at(b,dp) if b in idx else (b if cpar.get(b)==(dp[-1] if dp else None) else unit_at(b,dp))
        if ua is None or ub is None or ua==ub: continue
        w[(ua,ub)]=w.get((ua,ub),0)+1
    return w
def units(dp):
    us=[]
    for n in nodes:
        u=unit_at(n,dp)
        if u is not None and u not in us: us.append(u)
    return us
for dp in [[]]+[path(c)+[c] for c in clusters]:
    us=units(dp); w=level(dp)
    if len(us)<2: continue
    print('LEVEL',dp, us)
    for (a,b),c in sorted(w.items()): 
        rev=w.get((b,a),0)
        if rev and a<b: print('  2cycle',a,b,c,'vs',rev)
    if len(us)<=8:
        def cost(o):
            pos={u:i for i,u in enumerate(o)}
            return sum(c for (a,b),c in w.items() if pos[a]>pos[b])
        best=min(itertools.permutations(us),key=lambda o:(cost(o),[us.index(u) for u in o]))
        print('  best',cost(best),best)
        # dfs order
        adj={u:[] for u in us}
        for (a,b),c in w.items(): adj[a].append(b)
        # dfs following edge declaration order
        seen=set();order=[];post=[]
        def dfs(u):
            seen.add(u)
            for (a,b) in [(a2,b2) for (a2,b2) in edges]:
                ua=unit_at(a,dp) if a in idx else a; ub=unit_at(b,dp) if b in idx else b
                if ua==u and ub in adj and ub not in seen and ub!=u: dfs(ub)
            post.append(u)
        for u in us:
            if u not in seen: dfs(u)
        dord=post[::-1]
        print('  dfs-topo',cost(dord),dord)
