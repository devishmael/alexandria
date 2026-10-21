---
name: sast-and-secrets-guard
domain: security
description: "Static analysis, secret leak prevention, and safe execution rules"
---

# SAST & Secret Leak Interception

## Invariants
1. **Zero Hardcoded Secrets:** Immediate rejection of strings matching token formats (`sk-`, `ghp_`, `AKIA`, PEM private keys).
2. **Safe Code Execution:** Rejection of dynamic evaluation primitives (`eval`, `new Function`, `dangerouslySetInnerHTML`).
3. **Sandbox Confinement:** Isolated test execution under OS permission restrictions (`node --permission`).
