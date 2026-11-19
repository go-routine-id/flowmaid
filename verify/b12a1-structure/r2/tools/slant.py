#!/usr/bin/env python3
# Long slanted straight strokes in a flowmaid SVG: path segments 'L' (or M..L) longer than 60px whose dx and dy both exceed 6px.
import re,sys
for f in sys.argv[1:]:
    s=open(f).read(); n=0; out=[]
    for m in re.finditer(r'<path[^>]* d="([^"]+)"',s):
        d=m.group(1)
        toks=re.findall(r'([MLCQZ])\s*([^MLCQZ]*)',d)
        cur=None
        for c,args in toks:
            nums=[float(x) for x in re.findall(r'-?\d+\.?\d*',args)]
            if c=='M' and len(nums)>=2: cur=(nums[0],nums[1])
            elif c=='L' and len(nums)>=2 and cur:
                p=(nums[0],nums[1]); dx=abs(p[0]-cur[0]); dy=abs(p[1]-cur[1])
                if dx>6 and dy>6 and (dx*dx+dy*dy)**.5>60: n+=1; out.append('L %.0fx%.0f at %.0f,%.0f'%(dx,dy,cur[0],cur[1]))
                cur=p
            elif c in 'CQ' and len(nums)>=2:
                if c=='C' and len(nums)>=6 and cur:
                    # a straight cubic: control points on the chord
                    p=(nums[4],nums[5]); dx=abs(p[0]-cur[0]); dy=abs(p[1]-cur[1])
                    c1=(nums[0],nums[1]); 
                    cross=abs((p[0]-cur[0])*(c1[1]-cur[1])-(p[1]-cur[1])*(c1[0]-cur[0]))
                    L=(dx*dx+dy*dy)**.5
                    if L>60 and dx>6 and dy>6 and cross/L<0.5: n+=1; out.append('C(straight) %.0fx%.0f at %.0f,%.0f'%(dx,dy,cur[0],cur[1]))
                cur=(nums[-2],nums[-1])
    print(f.split('/')[-1],n,'; '.join(out[:4]))
