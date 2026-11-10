import re,sys
s=open(sys.argv[1]).read()
s2=re.sub(r'<style>.*?</style>','<style/>',s,flags=re.S)
s2=re.sub(r'<marker.*?</marker>','',s2,flags=re.S)
out=re.sub(r'>\s*<','>\n<',s2)
lines=out.split('\n')
depth=0
pat=sys.argv[2] if len(sys.argv)>2 else None
n=0
for l in lines:
    if l.startswith('</'): depth-=1
    if (pat is None or re.search(pat,l)):
        print('  '*depth + l[:int(sys.argv[3]) if len(sys.argv)>3 else 240]); n+=1
    if not l.startswith('</') and not l.endswith('/>') and not re.search(r'</[a-zA-Z]+>$',l): depth+=1
