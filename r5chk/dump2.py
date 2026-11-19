import re,sys
# parse now_dump: class edge i a->b bez [...] wps n ; then "   wps [...]"
lines=open(sys.argv[1]).read().splitlines()
for i,l in enumerate(lines):
    if l.startswith('class edge') or l.startswith('edge'):
        print(l[:120])
