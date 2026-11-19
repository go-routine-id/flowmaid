- **`PartialEq` on the graph model** (`model::Graph`, `Node`, `Edge`,
  `Subgraph`, `SubEdge`), so a host can tell whether an edit changed the
  graph.
- **Benchmark not re-measured:** the README table stands as measured on
  0.39.0 (about 113x faster than mermaid 12.2.0, geometric mean over 25
  diagrams). 0.40.0 draws all 25 samples byte for byte the same, and
  three alternated rounds of both releases show no difference in time
  beyond the noise of the shared box. Two runs of the whole table on
  0.40.0, on a busier box where both sides ran slower, gave 127x and
  134x.

### Added
