import re,sys
src=open(sys.argv[1]).read()
nodes=[];edges=[]
def add(n):
    if n not in nodes: nodes.append(n)
for line in src.splitlines():
    l=line.strip()
    m=re.match(r'^(\w+)[\[\(]',l)
    if m and '-->' not in l: add(m.group(1))
    m=re.match(r'^(\w+)\s*-->\s*(\w+)$',l)
    if m:
        add(m.group(1));add(m.group(2));edges.append((m.group(1),m.group(2)))
idx={n:i for i,n in enumerate(nodes)}
adj=[[] for _ in nodes]
for ei,(a,b) in enumerate(edges): adj[idx[a]].append((idx[b],ei))
state=[0]*len(nodes);back=[False]*len(edges)
for s in range(len(nodes)):
    if state[s]: continue
    state[s]=1;st=[[s,0]]
    while st:
        u,ci=st[-1]
        if ci<len(adj[u]):
            st[-1][1]+=1;v,ei=adj[u][ci]
            if state[v]==0: state[v]=1;st.append([v,0])
            elif state[v]==1: back[ei]=True
        else:
            state[u]=2;st.pop()
print('DFS back edges:',[edges[i] for i in range(len(edges)) if back[i]])
# longest path layering
from collections import deque
indeg=[0]*len(nodes)
for u in range(len(nodes)):
    for v,ei in adj[u]:
        if not back[ei]: indeg[v]+=1
layer=[0]*len(nodes)
q=deque([v for v in range(len(nodes)) if indeg[v]==0])
while q:
    u=q.popleft()
    for v,ei in adj[u]:
        if back[ei]: continue
        layer[v]=max(layer[v],layer[u]+1); indeg[v]-=1
        if indeg[v]==0:q.append(v)
for L in range(max(layer)+1):
    print(L,[nodes[i] for i in range(len(nodes)) if layer[i]==L])
# greedy Eades FAS
import itertools
def eades(nodes_i, E):
    rem=set(nodes_i); s1=[]; s2=[]
    E=list(E)
    while rem:
        changed=True
        while changed:
            changed=False
            for v in list(rem):
                outs=[e for e in E if e[0]==v and e[1] in rem and e[1]!=v]
                if not outs: s2.insert(0,v); rem.remove(v); changed=True
            for v in list(rem):
                ins=[e for e in E if e[1]==v and e[0] in rem and e[0]!=v]
                if not ins: s1.append(v); rem.remove(v); changed=True
        if rem:
            best=max(rem,key=lambda v: sum(1 for e in E if e[0]==v and e[1] in rem)-sum(1 for e in E if e[1]==v and e[0] in rem))
            s1.append(best); rem.remove(best)
    order=s1+s2; pos={v:i for i,v in enumerate(order)}
    return [e for e in E if pos[e[0]]>pos[e[1]]]
E=[(idx[a],idx[b]) for a,b in edges]
fas=eades(range(len(nodes)),E)
print('Eades FAS:',[(nodes[a],nodes[b]) for a,b in fas])
