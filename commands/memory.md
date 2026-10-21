# `/memory` — Hybrid Vector Memory Query & Distillation

## Description
Interacts with Alexandria's 64-dim quantized `int8` + BM25 Reciprocal Rank Fusion episodic memory.

## Usage
```text
/memory query <search_phrase> [budget]
/memory add <note> [module]
```

## CLI Equivalent
```bash
# Query memory with 150-token limit:
alexandria memory query "prisma transaction billing closure" --budget 150

# Add user-supplied memory:
alexandria memory add "Strict lexical closure required in billing" --module billing
```
