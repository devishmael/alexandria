# `/vault` — Knowledge Graph Traversal & ADR Hoisting

## Description
Traverses the local markdown vault in RAM (`petgraph::StableDiGraph`) and prints the direct neighbor dependencies and constitutional ADR chains for any requested module.

## Guarantees
- Sub-millisecond graph resolution ($\le 0.20\text{ ms}$).
- Strict Depth=1 module boundary pruning.
- Automatic transitive ADR hoisting up to 6 levels deep.
