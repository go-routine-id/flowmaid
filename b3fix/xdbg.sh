#!/usr/bin/env bash
# xdbg.sh on|off : toggle the per-crossing detail lines in /home/user/flowmaid/src/quality.rs
cd /home/user/flowmaid
python3 - "$1" <<'PY'
import sys
p='src/quality.rs'; s=open(p).read()
blk='''            for &p in &hits {
                m.details.push(format!(
                    "{}->{} XDBG {}->{} at ({:.0},{:.0})",
                    a.e.from, a.e.to, b.e.from, b.e.to, p.0, p.1
                ));
            }
'''
anchor='''            m.x_drawn += hits.len();
'''
if sys.argv[1]=='on':
    if blk not in s: s=s.replace(anchor,anchor+blk,1)
else:
    s=s.replace(blk,'')
open(p,'w').write(s)
PY
grep -c XDBG src/quality.rs
