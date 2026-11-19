import json
stmts = r'''class A.
class .A
class A..B
class A...B
class A....B
class A..
class ..A
class A.B.
class .A.B
class A..B.C
class .
class A .
class  .A
class A . . B
class A-.
class A.~T~
class A.B.~T~
class A..B~T~
class A.:::foo
class A.B.:::foo
class A..B:::foo
class A.["lbl"]
class A., B
class A, B.
class A, .B
class A. {||}
class .A {||}
class A...B {||}
<<interface>> A.
<<interface>> .A
class A||class B.
class A.||A --> B
class A.B
class a.b.C
class A.-B
class A-.B
class A._B
class A.1
class 1.A
class A.-
class A.B-
class A.B~T~
class Map~a.b~
class A.B {||+x int||}
class A.B.C {||}
class `A.`
class `.A`
class `A..B`
class A.B {||}||<<interface>> A.B
A.B --> C
A..B
class A. B
class A .B
class A . B
class A.B["lbl"]
class A.B["x.y"]
class A["x."]
class A.B:::foo
class A.B.C
class a.b.c.D
class A_.
class A._
class _.A
class A.-.
class A-.-B
class A.{||}
class A.B.{||}
class A.B{||}
namespace a.b {||class C||}
namespace a. {||class C||}
namespace .a {||class C||}
namespace a..b {||class C||}
namespace N {||class A.||}
namespace N {||class A.B||}
namespace N {||class .A||}
A. --> B
.A --> B
A.B. --> C
A --> B.
A ..> B.
A --> .B
A...B
A.. B
A. : +x
A.B : +x
.A : +x
class A.B||A.B : +int x
note for A. "x"
note for A.B "x"
class A||cssClass "A." foo
class A||style A. fill:#f00
class A||click A. href "x"
class A.B||click A.B href "x"
class A.B~T.~
class A:::foo.bar
class A.  
class Ä.Ö
class A.Ö
class 1.2
class A.1.
class A.B ["lbl"]
class A. .B
class A.B, C.D
class A.B,C.
class A~T~.
class A~T~.B
class A["lbl"].
class A.B["lbl"].
class A..B["lbl"]
<<interface>> A..B
<<interface>> A.B.
class A.B||<<interface>> A.B.
class A {||<<interface>>||}||class B.
class A {||+x.y int||}
class A {||+method(a.b) void||}
class A.B||class A.B
A <|-- B.
A.B <|-- C.D
A.B.C <|-- D
class A$.
class A.$
class A-.B.
class A.B-.
class A.-B.'''.split('\n')
cases = ['classDiagram\n' + s.replace('||', '\n') for s in stmts]
json.dump(cases, open('cases.json', 'w'))
open('cases.txt', 'w').write('\n%%%%\n'.join(cases))
print(len(cases))
