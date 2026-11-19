import re,sys,math
for f in sys.argv[1:]:
    s=open(f).read()
    ls=[]
    tr=[]
    for m in re.finditer(r'<path d="(M[^"]*)"[^>]*class="[^"]*edge-depth-(\d+)', s):
        nums=[float(x) for x in re.findall(r'-?\d+\.?\d*', m.group(1))]
        x0,y0,x1,y1=nums[0],nums[1],nums[-2],nums[-1]
        L=math.hypot(x1-x0,y1-y0)
        (tr if m.group(2)=='1' else ls).append(L)
    a=ls+tr
    if not a: print(f, 'none'); continue
    print(f"{f.split('/')[-1]}: trunks {min(tr):.0f}-{max(tr):.0f} all {min(a):.0f}-{max(a):.0f} ratio {max(a)/max(min(a),1):.2f}")
