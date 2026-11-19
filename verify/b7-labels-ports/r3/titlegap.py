import re,sys,glob,os
for d in sys.argv[1:]:
  out=[]
  for f in sorted(glob.glob(d+'/*.labels')):
    nodes=[];frames=[]
    for l in open(f):
      m=re.match(r'FRAME (\S+) .* at ([-\d.]+),([-\d.]+) ([\d.]+)x([\d.]+)',l)
      if m: frames.append((m.group(1),float(m.group(2)),float(m.group(3)),float(m.group(4)),float(m.group(5)))); continue
      m=re.match(r'(\S+) at ([-\d.]+),([-\d.]+) ([\d.]+)x([\d.]+)',l)
      if m:
        x,y,w,h=map(float,m.groups()[1:]); nodes.append((m.group(1),x-w/2,y-h/2,x+w/2,y+h/2))
    for (fid,fx,fy,fw,fh) in frames:
      inside=[n for n in nodes if n[1]>=fx and n[3]<=fx+fw and n[2]>=fy and n[4]<=fy+fh]
      sub=[g for g in frames if g[0]!=fid and g[1]>=fx and g[1]+g[3]<=fx+fw and g[2]>=fy and g[2]+g[4]<=fy+fh]
      tops=[n[2]-fy for n in inside]+[g[2]-fy for g in sub]
      bots=[fy+fh-n[4] for n in inside]+[fy+fh-(g[2]+g[4]) for g in sub]
      if tops: out.append(f"{os.path.basename(f)[:-7]}:{fid} top={min(tops):.1f} bottom={min(bots):.1f}")
  print(d); print('\n'.join(out))
