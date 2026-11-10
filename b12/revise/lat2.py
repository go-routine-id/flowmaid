import sys
sys.path.insert(0,'critic')
import importlib.util,io,contextlib
src=open('critic/lateral.py').read().rsplit('for f in sys.argv[1:]',1)[0]
exec(src)
for a in sys.argv[1:]:
    f,v=a.split(':'); run(f, v=='v')
