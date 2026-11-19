import glob,sys
try:
    from PIL import Image
except Exception as e:
    print('no PIL',e); sys.exit()
for f in sorted(glob.glob('/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/wt/rel39/docs/img/compare/*.png')):
    im=Image.open(f).convert('RGB'); w,h=im.size; px=im.load()
    bg=px[2,h-3]
    def nonbg(p): return sum(abs(a-b) for a,b in zip(p,bg))>40
    # find the divider column: a column where most pixels equal a light grey
    # content bbox
    xs=[x for x in range(w) for y in range(0,h,2) if nonbg(px[x,y])]
    ys=[y for y in range(h) for x in range(0,w,2) if nonbg(px[x,y])]
    # per-edge counts
    top=sum(nonbg(px[x,0]) for x in range(w)); bot=sum(nonbg(px[x,h-1]) for x in range(w))
    left=sum(nonbg(px[0,y]) for y in range(h)); right=sum(nonbg(px[w-1,y]) for y in range(h))
    # columns that are non-bg for nearly the full height = divider
    div=[x for x in range(w) if sum(nonbg(px[x,y]) or px[x,y]!=bg for y in range(0,h,4))>0.9*(h/4)]
    print(f.split('/')[-1],(w,h),'bg',bg,'bbox x',min(xs),max(xs),'y',min(ys),max(ys),'edge nonbg t/b/l/r',top,bot,left,right,'divider cols',div[:4])
