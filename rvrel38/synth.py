import sys
M=(1<<64)-1
class L:
    def __init__(s,v): s.v=v
    def next(s,n):
        s.v=(s.v*6364136223846793005+1442695040888963407)&M
        return (s.v>>33)%n
def synth(layers,width):
    out=["flowchart TD\n"]; r=L(42)
    for l in range(layers-1):
        for w in range(width):
            f=l*width+w
            for _ in range(2):
                t=(l+1)*width+r.next(width)
                out.append(f"N{f}[Node {f}] --> N{t}[Node {t}]\n")
    for _ in range(layers):
        a=(layers-1)*width+r.next(width); b=r.next(width)
        out.append(f"N{a} --> N{b}\n")
    return "".join(out)
l,w,p=int(sys.argv[1]),int(sys.argv[2]),sys.argv[3]
open(p,'w').write(synth(l,w))
