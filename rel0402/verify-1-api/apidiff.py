import json, sys
def load(p):
    j = json.load(open(p))
    idx = j['index']; paths = j['paths']
    out = {}
    # Public items reachable: walk from root module through public items
    root = j['root']
    def name_of(it): return it.get('name')
    seen=set()
    def sig(it):
        inner = it['inner']
        k = list(inner.keys())[0]
        v = inner[k]
        if k == 'function':
            return k + json.dumps(v.get('sig'), sort_keys=True)
        if k == 'struct':
            return k + json.dumps(v.get('kind'), sort_keys=True)[:0]
        if k in ('enum',):
            return k
        return k
    def walk(mid, prefix):
        if (mid, prefix) in seen: return
        seen.add((mid, prefix))
        it = idx[str(mid)] if str(mid) in idx else None
        if it is None: return
        inner = it['inner']; k = list(inner.keys())[0]
        if k == 'module':
            for c in inner['module']['items']:
                ci = idx.get(str(c))
                if ci is None: continue
                if ci.get('visibility') != 'public' and list(ci['inner'].keys())[0] != 'use': continue
                ck = list(ci['inner'].keys())[0]
                if ck == 'use':
                    if ci.get('visibility') != 'public': continue
                    u = ci['inner']['use']
                    tgt = u.get('id')
                    nm = u['name']
                    if tgt is not None and str(tgt) in idx:
                        walk_item(tgt, prefix + '::' + nm)
                    else:
                        out[prefix + '::' + nm] = 'use-ext ' + u['source']
                    continue
                walk_item(c, prefix + '::' + (ci.get('name') or '?'))
    def walk_item(c, path):
        ci = idx[str(c)]
        ck = list(ci['inner'].keys())[0]
        if ck == 'module':
            out[path] = 'module'
            walk(c, path); return
        if ck == 'function':
            out[path] = 'fn ' + json.dumps(ci['inner']['function']['sig'], sort_keys=True) + ' attrs=' + json.dumps(ci.get('attrs'))
            return
        if ck == 'struct':
            s = ci['inner']['struct']
            out[path] = 'struct ' + json.dumps(s['kind'] if not isinstance(s['kind'], dict) else list(s['kind'].keys()), sort_keys=True) + ' attrs=' + json.dumps(ci.get('attrs'))
            kind = s['kind']
            if isinstance(kind, dict) and 'plain' in kind:
                for f in kind['plain']['fields']:
                    fi = idx[str(f)]
                    if fi.get('visibility') == 'public':
                        out[path + '.' + fi['name']] = 'field ' + json.dumps(fi['inner'], sort_keys=True)
            for imp in s['impls']:
                walk_impl(imp, path)
            return
        if ck == 'enum':
            e = ci['inner']['enum']
            out[path] = 'enum attrs=' + json.dumps(ci.get('attrs'))
            for v in e['variants']:
                vi = idx[str(v)]
                out[path + '::' + vi['name']] = 'variant ' + json.dumps(vi['inner'], sort_keys=True)
            for imp in e['impls']:
                walk_impl(imp, path)
            return
        out[path] = ck + ' ' + json.dumps(ci['inner'], sort_keys=True)[:400]
    def walk_impl(imp, path):
        ii = idx[str(imp)]
        im = ii['inner']['impl']
        tr = im.get('trait')
        if im.get('is_synthetic') or im.get('blanket_impl'): return
        tname = tr['path'] if tr else ''
        if tr:
            out[path + ' impl ' + tname] = 'impl'
            return
        for it in im['items']:
            x = idx[str(it)]
            if x.get('visibility') != 'public': continue
            k = list(x['inner'].keys())[0]
            if k == 'function':
                out[path + '::' + x['name']] = 'method ' + json.dumps(x['inner']['function']['sig'], sort_keys=True)
            else:
                out[path + '::' + x['name']] = k + json.dumps(x['inner'], sort_keys=True)[:300]
    walk(root, 'flowmaid')
    return out
import re
norm=lambda d:{k:re.sub(r"\"id\": \d+|\[\d+\]","ID",v) for k,v in d.items()}
a = norm(load(sys.argv[1])); b = norm(load(sys.argv[2]))
print('base items', len(a), 'head items', len(b))
rem = sorted(set(a) - set(b)); add = sorted(set(b) - set(a))
chg = sorted(k for k in set(a) & set(b) if a[k] != b[k])
print('REMOVED', len(rem)); [print('  -', k) for k in rem]
print('CHANGED', len(chg)); [print('  ~', k, '\n     base:', a[k][:300], '\n     head:', b[k][:300]) for k in chg]
print('ADDED', len(add)); [print('  +', k) for k in add]
