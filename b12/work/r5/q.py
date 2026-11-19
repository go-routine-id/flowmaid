#!/usr/bin/env python3
"""q.py <quality-binary> <dir-or-files...> [--mode static|routed|both] [--all]: compact harness table."""
import subprocess, sys, csv, io, os, tempfile, shutil
args = sys.argv[1:]
mode = 'static'; show_all = False
if '--mode' in args:
    i = args.index('--mode'); mode = args[i+1]; del args[i:i+2]
if '--all' in args:
    args.remove('--all'); show_all = True
q = args[0]; ins = args[1:]
d = None
if len(ins) == 1 and os.path.isdir(ins[0]):
    d = ins[0]
else:
    d = tempfile.mkdtemp(prefix='q-in-')
    for f in ins: shutil.copy(f, d)
out = subprocess.run([q, d], capture_output=True, text=True)
rows = list(csv.DictReader(io.StringIO(out.stdout), delimiter='\t'))
HZ = ['edge_node_x','shared_ports','arrow_on_start','wiggles','frame_side','node_overlap','node_frame','frame_overlap','own_frame_x']
cols = HZ + ['edge_frame_pairs','overlap_pairs','x_drawn','fan_x','defects','scene_ms']
print('%-34s %-6s ' % ('file','mode') + ' '.join(c[:9].rjust(9) for c in cols))
tot = {}
for r in rows:
    if r['file'] == 'TOTAL': continue
    if mode != 'both' and r['mode'] != mode: continue
    hz = sum(float(r[c]) for c in HZ)
    for c in cols:
        try: tot[(r['mode'],c)] = tot.get((r['mode'],c),0)+float(r[c])
        except: pass
    if show_all or hz > 0 or float(r['edge_frame_pairs']) > 0 or float(r['fan_x']) > 0:
        print('%-34s %-6s ' % (r['file'][:34], r['mode']) + ' '.join(r[c][:9].rjust(9) for c in cols))
for m in ('static','routed'):
    if mode in (m,'both'):
        print('%-34s %-6s ' % ('TOTAL', m) + ' '.join(('%g' % tot.get((m,c),0))[:9].rjust(9) for c in cols))
if out.stderr.strip(): print('STDERR:', out.stderr[:2000])
