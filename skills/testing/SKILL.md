---
name: strict-tdd-engineer
domain: testing
description: "Mandatory Test-Driven Development execution rules"
---

# Strict TDD Engineering

## Invariants
1. **Verifiable RED Phase:** The test must fail deterministically due to a business assertion (`AssertionError` / `assert_eq!`) before code in `src/` is touched.
2. **Minimal GREEN Phase:** Write only the minimal implementation required to turn the assertion green.
3. **Continuous Ratchet:** Code coverage must never decrease between commits.
