I've been building flowmaid: Mermaid diagrams, rendered in pure Rust.

Mermaid syntax in, clean SVG out. No browser, no JavaScript runtime, zero dependencies.

What's in the video:
→ 12 diagram types: flowchart, sequence, class, state, ER, pie, user journey, git graph, mindmap, timeline, sankey and architecture
→ About 113× faster than mermaid.js (geometric mean over 25 sample diagrams)
→ It runs in the browser as WebAssembly: drag a node on the canvas and its edges re-route live
→ flowmaid already powers two of my terminal tools:
   • flowcli draws diagrams right in the terminal and redraws when the file changes
   • dbx, a terminal database client, draws your schema as an ERD

Try it in your browser: https://go-routine-id.github.io/flowmaid-web/
Code: https://github.com/go-routine-id/flowmaid
Rust: cargo add flowmaid

Next up: subgraphs laid out as tidy blocks (opt-in in 0.41.0).

Feedback, issues and stars are very welcome 🙏

#rust #opensource #webassembly #mermaid #diagrams #devtools
