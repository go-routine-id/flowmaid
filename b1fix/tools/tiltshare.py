import re,sys,glob,os,collections
qdir=sys.argv[1]
det=[l.rstrip('\n').split('\t') for l in open(qdir+'/details.txt') if l.count('\t')>=2]
bows=set(); tilts=collections.defaultdict(list)
for f,m,rest in det:
    e=rest.split(' ')[0]
    if 'idiom bow' in rest: bows.add((f,m,e))
    if 'end_tilt' in rest: tilts[(f,m)].append((e,rest))
for mode in ['static','routed']:
    ends=0; bow_ends=0; bad=0; badbow=0
    for svg in glob.glob(qdir+'/svg/*.%s.svg'%mode):
        f=os.path.basename(svg).split('.')[0]
        s=open(svg).read()
        for m in re.finditer(r'<path d="[^"]*"([^>]*)><title>([^<]*)</title>',s):
            attrs,title=m.groups()
            t=title.split(':')[0].replace(' → ','->').replace(' ↔ ','->').strip()
            n=attrs.count('marker-end')+attrs.count('marker-start')
            if not n: continue
            ends+=n
            if (f,mode,t) in bows: bow_ends+=n
        for e,r in tilts[(f,mode)]:
            if (f,mode,e) in bows: badbow+=1
            else: bad+=1
    nb=ends-bow_ends
    print(mode,'glyph ends',ends,'bow ends',bow_ends,'tilted non-bow',bad,'tilted bow',badbow,'share ok non-bow %.1f%%'%(100*(1-bad/nb)))
