# ⚖️ Court-Martial Renegotiation Workflow

## Trigger Condition
$$\text{Failures}(M) \ge 3 \implies \text{CircuitBreakerTripped}$$

When a module experiences 3 consecutive Quality Gate failures, automatic retries freeze immediately to prevent LLM Thrashing.

## Court-Martial Protocol
1. **Utu (Prosecutor):** Formulates formal indictment citing violated quality gate invariants and test failure logs.
2. **Enki (Defendant):** Submits defense arguing whether failure is due to implementation limits or contract ambiguity.
3. **Bel (Presiding Judge):** Issues binding Single-Turn verdict:
   - Option A: Amend contract specification (`RULE_IN_FAVOR_OF_ENKI_AMEND_CONTRACT`).
   - Option B: Revoke worktree, reset branch, and reassign task (`UPHOLD_UTU_PROSECUTION_REVOKE_WORKTREE_AND_REASSIGN`).
