import subprocess,sys,os
src=open(sys.argv[1]).read().split('\n')
fm=sys.argv[2]; out=sys.argv[3]
def differs(lines):
    t='\n'.join(lines)
    open('/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/regress/min/_t.mmd','w').write(t)
    p='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/regress/min/_t.mmd'
    a=subprocess.run([fm,p],capture_output=True,env=dict(os.environ)).stdout
    b=subprocess.run([fm,p],capture_output=True,env=dict(os.environ,FM_RFIX='1')).stdout
    return a!=b and len(a)>100
assert differs(src)
changed=True
while changed:
    changed=False
    i=1
    while i<len(src):
        t=src[:i]+src[i+1:]
        if differs(t):
            src=t; changed=True
        else:
            i+=1
open(out,'w').write('\n'.join(src))
print('\n'.join(src))
