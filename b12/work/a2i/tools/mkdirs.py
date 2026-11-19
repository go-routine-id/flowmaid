import re,sys,os,glob
out=sys.argv[1]; os.makedirs(out,exist_ok=True)
n=0
for d in ['blocks','quality','notes']:
    for f in sorted(glob.glob(f'/home/user/flowmaid/tests/fixtures/{d}/*.mmd')):
        s=open(f).read()
        base=os.path.basename(f)[:-4]
        m=re.search(r'^(\s*)(flowchart|graph)\s+(TD|TB|BT|LR|RL)\b',s,re.M)
        st=re.search(r'^\s*stateDiagram(-v2)?\s*$',s,re.M)
        for D in ['TD','BT','LR','RL']:
            if m:
                t=s[:m.start(3)]+D+s[m.end(3):]
            elif st:
                # top-level direction line right after header, else insert
                lines=s.split('\n'); i=[k for k,l in enumerate(lines) if re.match(r'\s*stateDiagram',l)][0]
                if i+1<len(lines) and re.match(r'\s*direction\s+(TB|TD|BT|LR|RL)\s*$',lines[i+1]):
                    lines[i+1]='    direction '+('TB' if D=='TD' else D)
                else:
                    lines.insert(i+1,'    direction '+('TB' if D=='TD' else D))
                t='\n'.join(lines)
            else:
                continue
            open(f'{out}/{d}.{base}-{D}.mmd','w').write(t); n+=1
print(n)
