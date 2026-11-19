import re,sys
f=sys.argv[1]; want=sys.argv[2]; thr=int(sys.argv[3])
cur=None; ln=None
out=[]
for l in open(f):
    m=re.match(r'-- Auto-annotated source: (.*)',l)
    if m: cur=m.group(1).strip().split('/')[-1]; continue
    m=re.match(r'-- line (\d+) -',l)
    if m: ln=int(m.group(1))-1; continue
    if cur!=want or ln is None: continue
    m=re.match(r'\s*([\d,]+|\.)\s(.*)$',l)
    if not m: continue
    body=m.group(2)
    if body.strip().startswith('=>'):
        mm=re.match(r'\s*=> (.*?) \((\d[\d,]*)x\)',body)
        if int(m.group(1).replace(',',''))>thr: out.append((ln,int(m.group(1).replace(',','')),'   CALL '+body.strip()[:120]))
        continue
    ln+=1
    if m.group(1)!='.' and int(m.group(1).replace(',',''))>thr: out.append((ln,int(m.group(1).replace(',','')),body[:110]))
for o in out: print(o[0],o[1],o[2])
