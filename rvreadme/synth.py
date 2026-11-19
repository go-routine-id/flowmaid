import sys
M=(1<<64)-1
class L:
    def __init__(s,x): s.x=x
    def next(s,n):
        s.x=(s.x*6364136223846793005+1442695040888963407)&M
        return (s.x>>33)%n
def synth(layers,width):
    s="flowchart TD\n"; r=L(42)
    for l in range(layers-1):
        for w in range(width):
            f=l*width+w
            for _ in range(2):
                t=(l+1)*width+r.next(width)
                s+=f"N{f}[Node {f}] --> N{t}[Node {t}]\n"
    for _ in range(layers):
        a=(layers-1)*width+r.next(width); b=r.next(width)
        s+=f"N{a} --> N{b}\n"
    return s
l,w=map(int,sys.argv[1:3]); sys.stdout.write(synth(l,w))
