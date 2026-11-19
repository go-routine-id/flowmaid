import sys
for p in sys.argv[1:]:
    s=open(p).read()
    key='if std::env::var("FMDBG").is_ok() {'
    while key in s:
        i=s.index(key)
        # start of line
        ls=s.rfind('\n',0,i)+1
        j=i+len(key); depth=1
        while depth:
            c=s[j]
            if c=='{': depth+=1
            elif c=='}': depth-=1
            j+=1
        # consume to end of line
        le=s.index('\n',j)+1
        s=s[:ls]+s[le:]
    open(p,'w').write(s)
