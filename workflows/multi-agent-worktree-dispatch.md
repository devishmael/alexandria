# 🌿 Multi-Agent Worktree Dispatch Workflow

## Purpose
Enables concurrent subagents to work on different parts of the codebase simultaneously without git merge collisions or architectural interference.

## Topological Disjunction Check
Before allocating a git worktree (`alexandria-worktrees/task-<module>`):
1. The kernel queries `petgraph::StableDiGraph` in RAM.
2. Evaluates intersection of import closures: $\text{Closure}(M_A) \cap \text{Closure}(M_B) = \emptyset$.
3. If disjoint, spawns isolated worktree branch: `alexandria/task/<module_name>`.
4. If overlapping, serializes task execution to prevent branch conflict.
