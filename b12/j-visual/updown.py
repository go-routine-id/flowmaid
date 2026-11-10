import re,sys
def count(p, axis):
    s=open(p).read()
    s=re.sub(r'<defs>.*?</defs>','',s,flags=re.S)
    up=down=flat=0
    for m in re.finditer(r'<path\b[^>]*\bd="([^"]+)"[^>]*>\s*<title>([^<]*)</title>',s):
        d=m.group(1)
        nums=[float(x) for x in re.findall(r'-?\d+(?:\.\d+)?',d)]
        if len(nums)<4: continue
        x0,y0,x1,y1=nums[0],nums[1],nums[-2],nums[-1]
        a=(y1-y0) if axis=='y' else (x1-x0)
        if a>8: down+=1
        elif a<-8: up+=1
        else: flat+=1
    return down,up,flat
for name,axis in [('infra-overview','y'),('dense-td','y'),('bench-75','x'),('dense-lr','x'),('advanced','y')]:
    for lab,p in [('0.38','metrics/fm038-svg/%s.svg'),('global','d-global/proto-lanes/%s.svg'),('hybrid','d-hybrid/final/%s.svg'),('recursive','d-recursive/metrics/%s.svg')]:
        try:
            print(name,lab,'with-flow/against/level =',count(p%name,axis))
        except Exception as e:
            print(name,lab,'ERR',e)
