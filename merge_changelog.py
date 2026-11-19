import re
CL='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/changelog/'
pkgs=['a2-er','a4-seq','a5-git','a6-pie-journey-timeline','a7-sankey-mindmap','a8-architecture']
sections={'Added':[],'Changed':[],'Fixed':[]}
for p in pkgs:
    s=open(CL+p+'.md').read()
    s=re.sub(r'<!--.*?-->','',s,flags=re.S)
    cur=None
    bullets=[]
    for line in s.split('\n'):
        m=re.match(r'### (\w+)',line)
        if m:
            cur=m.group(1); continue
        if cur is None: continue
        if line.startswith('- '):
            sections[cur].append([line])
        elif line.strip()=='' :
            if sections[cur] and sections[cur][-1][-1]!='': sections[cur][-1].append('')
        else:
            sections[cur][-1].append(line)
def text(b):
    while b and b[-1]=='': b=b[:-1]
    return '\n'.join(b)
for k in sections:
    sections[k]=[text(b) for b in sections[k]]
# drop per-package breaking bullets; consolidated below
sections['Changed']=[b for b in sections['Changed'] if not b.startswith('- **Breaking**')]
new=[]
for b in sections['Added']:
    b=b.replace(""" **Breaking (minor bump):** `Architecture`, `ArchService`
  and `ArchEdge` are now `#[non_exhaustive]`; build an `Architecture` with
  `Architecture::default()` and set its fields, services and edges come from
  the parser.""","")
    b=b.replace(""" **Breaking (same minor bump):** `ArchScene` is now
  `#[non_exhaustive]` (it is only built by `architecture::scene`).""","")
    assert 'Breaking' not in b, b
    new.append(b)
sections['Added']=new
breaking='''- **Breaking** (needs a **minor** bump): these public structs gain fields
  and become `#[non_exhaustive]` — ER `model::Entity`, `er::ErRow`,
  `er::ErTable`; sequence `seq::SeqScene`; `PieChart`, `Journey`,
  `Timeline`, `JourneyTask`, `pie::PieScene`, `journey::TaskGlyph`,
  `timeline::PeriodGlyph`; `SankeyConfig`, `SankeyNodeGlyph`,
  `SankeyScene`; `MindNode`, `mindmap::MindNodeBox`; `model::Architecture`,
  `ArchService`, `ArchEdge`, `architecture::ArchScene`. `MindShape` gains a
  `Default` variant (now its `#[default]`) and becomes `#[non_exhaustive]`.
  Build the model structs with `Default` (or the parser) and set fields
  (`SeqScene::default()` likewise); the scene structs come from each
  module's `scene()`. Only struct literals and exhaustive patterns outside
  the crate are affected.'''
sections['Changed'].append(breaking)
p='/home/user/flowmaid/CHANGELOG.md'
s=open(p).read()
un=s.index('## [Unreleased]'); nxt=s.index('## [0.32.0]')
body=s[un:nxt]
intro_old='''zero. Needs a **minor** bump: `AdvanceScene` gains a public field.'''
intro_new='''zero. ER, sequence, gitGraph, pie, journey, timeline, sankey, mindmap and
architecture diagrams were reworked to read at least as well as mermaid's,
keeping flowmaid's interactive scene API, portable SVG and compact size.
Flowchart output is byte-identical. Needs a **minor** bump: `AdvanceScene`
and several diagram structs gain public fields (see **Breaking** under
Changed).'''
assert intro_old in body
body=body.replace(intro_old,intro_new)
def insert(body,head,nexthead,items):
    i=body.index('### '+head)
    j=body.index(nexthead,i) if nexthead else len(body)
    seg=body[i:j].rstrip('\n')
    seg=seg+'\n'+''.join('\n'+b+'\n' for b in items)+'\n'
    if nexthead: seg+='\n' if False else ''
    return body[:i]+seg+body[j:]
body=insert(body,'Added','### Changed',sections['Added'])
body=insert(body,'Changed','### Fixed',sections['Changed'])
body=insert(body,'Fixed',None,sections['Fixed'])
s=s[:un]+body+s[nxt:]
open(p,'w').write(s)
print({k:len(v) for k,v in sections.items()})
