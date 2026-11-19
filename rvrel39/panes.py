import glob
from PIL import Image
for f in sorted(glob.glob('/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/wt/rel39/docs/img/compare/*.png')):
    im=Image.open(f).convert('RGB'); w,h=im.size; px=im.load()
    bg=(255,255,255)
    def nonbg(p): return sum(abs(a-b) for a,b in zip(p,bg))>40
    cols=[sum(nonbg(px[x,y]) for y in range(h)) for x in range(w)]
    div=max(range(w),key=lambda x:cols[x])
    # divider colour may be light: detect by column uniformity
    uni=[x for x in range(w) if len({px[x,y] for y in range(0,h,3)})==1 and px[x,0]!=bg]
    d=uni[0] if uni else div
    L=[x for x in range(0,d) if cols[x]>0]; R=[x for x in range(d+1,w) if cols[x]>0]
    rows_L=[y for y in range(h) if any(nonbg(px[x,y]) for x in range(0,d,1))]
    rows_R=[y for y in range(h) if any(nonbg(px[x,y]) for x in range(d+1,w,1))]
    print(f.split('/')[-1],'divider',d,px[d,5],'left pane x',min(L),max(L),'gap to divider',d-max(L),'| right pane x',min(R),max(R),'gap',min(R)-d,'right margin',w-1-max(R),'| rows L',min(rows_L),max(rows_L),'R',min(rows_R),max(rows_R),'h',h)
