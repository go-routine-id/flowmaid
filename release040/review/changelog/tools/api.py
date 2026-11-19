import re,sys,subprocess
repo=sys.argv[1]
def files(rev):
    out=subprocess.run(['git','-C',repo,'ls-tree','-r','--name-only',rev,'src','examples'],capture_output=True,text=True).stdout.split()
    return [f for f in out if f.endswith('.rs')]
def show(rev,f):
    return subprocess.run(['git','-C',repo,'show',f'{rev}:{f}'],capture_output=True,text=True).stdout
def items(src):
    res=set()
    # strip test modules roughly: cut at "#[cfg(test)]\nmod tests"
    m=re.search(r'#\[cfg\(test\)\]\s*mod \w+',src)
    if m: src=src[:m.start()]
    # pub items: signature up to { or ;
    for m in re.finditer(r'(?m)^[ \t]*pub (?!\(crate\)|\(super\)|\(in )(?:(?:const |async |unsafe |extern "C" )*fn|struct|enum|trait|type|const|static|mod|use)\b[^{;]*',src):
        res.add(re.sub(r'\s+',' ',m.group(0)).strip())
    # pub fields
    for m in re.finditer(r'(?m)^[ \t]+pub (?!\(crate\)|\(super\)|\(in |fn |struct |enum |trait |type |const |static |mod |use )(\w+)\s*:\s*([^,\n]+)',src):
        res.add('FIELD '+re.sub(r'\s+',' ',m.group(0)).strip())
    # enum bodies of pub enums
    for m in re.finditer(r'(?ms)^pub enum (\w+)[^{]*\{(.*?)^\}',src):
        body=re.sub(r'//[^\n]*','',m.group(2))
        for v in re.finditer(r'(?m)^\s{4}(\w+)',body):
            res.add(f'VARIANT {m.group(1)}::{v.group(1)}')
    # impl blocks (trait impls)
    for m in re.finditer(r'(?m)^impl(?:<[^>]*>)?\s+[^{]*\{',src):
        res.add('IMPL '+re.sub(r'\s+',' ',m.group(0)).strip())
    for m in re.finditer(r'(?ms)#\[derive\(([^)]*)\)\]\s*(?:#\[[^\]]*\]\s*)*pub (?:struct|enum) (\w+)',src):
        for d in m.group(1).split(','):
            res.add(f'DERIVE {m.group(2)}: {d.strip()}')
    return res
a,b=sys.argv[2],sys.argv[3]
fa,fb=set(files(a)),set(files(b))
print('files removed:',sorted(fa-fb),'added:',sorted(fb-fa))
for f in sorted(fa|fb):
    ia=items(show(a,f)) if f in fa else set()
    ib=items(show(b,f)) if f in fb else set()
    for x in sorted(ia-ib): print(f'- {f}: {x}')
    for x in sorted(ib-ia): print(f'+ {f}: {x}')
