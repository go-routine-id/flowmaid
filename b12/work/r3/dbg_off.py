R='/home/user/flowmaid/'
for p in ['src/layout.rs','src/edgepass.rs','src/quality.rs','src/ortho.rs','src/router.rs','src/ports.rs','src/scene.rs']:
    s=open(R+p).read()
    out=[l for l in s.split('\n') if 'XXDBG' not in l]
    if len(out)!=len(s.split('\n')):
        open(R+p,'w').write('\n'.join(out)); print('stripped',p)
