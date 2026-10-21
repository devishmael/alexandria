# 🔨 Strict TDD Cycle Workflow (Red-Green-Refactor)

## Inviolable Preconditions
1. Target file must have been inspected/read in the active session (`E_FACT_FORCING`).
2. An architectural contract must exist in `wiki/modules/<module>.md`.

## Execution Steps

### 1. Phase RED (Assertion Failure)
- Author the unit test in `tests/`.
- Run the test suite:
  ```bash
  cargo test --test <test_name>
  ```
- **Verification Rule:** The test **MUST FAIL** deterministically due to a business assertion (`AssertionError` / `assert_eq!`).
- Rejections: Syntax errors, missing imports, or unhandled panics invalidate the RED phase.

### 2. Phase GREEN (Minimal Implementation)
- Write only the minimal necessary code in `src/` to satisfy the failing assertion.
- Re-run the test suite until it passes green.

### 3. Phase REFACTOR (Code Hygiene)
- Eliminate duplication, enforce nominal branded types, and verify zero `any`.
- Verify that Gate 0 and Gate 2 pass with zero diagnostics.
