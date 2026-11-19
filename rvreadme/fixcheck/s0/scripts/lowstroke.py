import re,sys,subprocess,glob,os
B=sys.argv[1]; E=sys.argv[2]
def lum(h):
    h=h.lstrip('#')
    if len(h)==3: h=''.join(c*2 for c in h)
    if len(h)!=6: return None
    def lin(c):
        c=c/255
        return c/12.92 if c<=0.04045 else ((c+0.055)/1.055)**2.4
    r,g,b=[int(h[i:i+2],16) for i in (0,2,4)]
    return 0.2126*lin(r)+0.7152*lin(g)+0.0722*lin(b)
def cr(a,b):
    la,lb=lum(a),lum(b)
    if la is None or lb is None: return None
    hi,lo=max(la,lb),min(la,lb)
    return (hi+0.05)/(lo+0.05)
for theme in sys.argv[3:]:
    for f in sorted(glob.glob(E+'/*.mmd')):
        out=subprocess.run([B,f,'--theme',theme],capture_output=True,text=True).stdout
        bgm=re.search(r"<svg[^>]*>\s*<title>[^<]*</title>\s*<rect width=\"[^\"]*\" height=\"[^\"]*\" fill=\"(#[0-9a-fA-F]+)\"",out)
        bg=bgm.group(1) if bgm else '#ffffff'
        low={}
        for m in re.finditer(r'<(path|line|polyline)\b([^>]*)>',out):
            a=m.group(2)
            fm=re.search(r'\bfill="([^"]*)"',a)
            if fm and fm.group(1)!='none': continue
            sm=re.search(r'\bstroke="(#[0-9a-fA-F]+)"',a)
            if not sm: continue
            s=sm.group(1).lower()
            c=cr(s,bg)
            if c is not None and c<3.0 and s!=bg.lower():
                low[s]=low.get(s,0)+1
        if low:
            print(theme,os.path.basename(f),'bg',bg,{k:(v,round(cr(k,bg),2)) for k,v in low.items()})
