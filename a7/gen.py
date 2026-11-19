import sys
M=(1<<64)-1
def gen(seed):
    x=(seed*0x9E3779B97F4A7C15 & M) | 1
    st=[x]
    def nxt(m):
        x=st[0]
        x^=(x<<13)&M; x^=x>>7; x^=(x<<17)&M
        st[0]=x
        return x%m
    shapes=["","(","[","((","{{","))",")"]; closes=["",")","]","))","}}","((","("]
    src="mindmap\n  Root topic\n"; depth=1
    for i in range(5+nxt(30)):
        depth=max(1,min(4,depth+nxt(3)-1))
        s=nxt(7)
        w=["node","a longer label","x","multi<br/>line"][nxt(4)]
        src+="  "*(depth+1)+shapes[s]+w+" "+str(i)+closes[s]+"\n"
    return src
print(gen(int(sys.argv[1])),end="")
