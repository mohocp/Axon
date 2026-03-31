# Phase 7: Agents Improve Themselves — Self-Modification

**Principle:** Verification by Construction + Evolutionary Development
**Status:** Planned
**Depends on:** Phase 1 (real SMT solver), Phase 5 (cryptographic audit trails)

---

## 1. Overview

The spec's most ambitious feature: agents that modify their own operations with formal proof obligations. An agent proposes a change, proves it preserves contracts, and the system either accepts automatically or escalates for review. Without Phase 1's real solver, this would be theater. Without Phase 5's audit trails, it would be unaccountable.

## 2. Requirements

### 2.1 MUTATE OPERATION

- **R1.1:** `MUTATE OPERATION name => { PROPOSED_CHANGE { ... }, PROOF { ... }, APPROVAL ... }` is a new statement.
- **R1.2:** Requires `SELF_MODIFY` capability.
- **R1.3:** PROPOSED_CHANGE contains the new operation body.
- **R1.4:** The original operation's signature (INPUT/OUTPUT) cannot change (mutation is body-only in v1).

### 2.2 Proof Obligations

- **R2.1:** `preconditions_preserved` — all original REQUIRE clauses still hold after mutation.
- **R2.2:** `postconditions_preserved` — all original ENSURE clauses still hold after mutation.
- **R2.3:** `no_new_capabilities_required` — mutation doesn't introduce new capability requirements.
- **R2.4:** `invariants_preserved` — all INVARIANT clauses still hold.
- **R2.5:** Each obligation submitted to SMT solver. All must be `Valid` for auto-approval.

### 2.3 Approval Workflows

- **R3.1:** `APPROVAL AUTO WHEN proof.all_pass` — automatic acceptance if all proofs discharge.
- **R3.2:** `APPROVAL ESCALATE WHEN proof.any_fail` — escalate to higher-authority agent or human.
- **R3.3:** Escalation includes: proposed change diff, proof results, operation context.
- **R3.4:** Approval/rejection recorded in audit trail with full evidence.

### 2.4 Mutation Execution

- **R4.1:** On approval, the operation body is replaced in the runtime's declaration table.
- **R4.2:** The mutation is versioned: original body preserved, new body activated.
- **R4.3:** Rollback: `ROLLBACK OPERATION name` restores previous version.
- **R4.4:** Version history queryable: `VERSIONS(operation_name)` returns list.

### 2.5 Evolutionary Op-Code Extension

- **R5.1:** Agents can propose new operations from compositions: `REGISTER OPERATION name => { ... }`.
- **R5.2:** New operations undergo the same proof obligations as mutations.
- **R5.3:** Registered operations are available to all agents in scope.
- **R5.4:** Registration recorded in audit trail.

## 3. Architecture

### 3.1 Crate Changes

**`al-ast`:**
- New `Statement::MutateOperation { name, proposed_change, proof, approval }` variant
- New `Statement::RollbackOperation { name }` variant
- New `Statement::RegisterOperation { decl }` variant
- New `ProofBlock` struct: list of proof obligations
- New `ApprovalPolicy` enum: Auto, Escalate

**`al-vc`:**
- New `generate_mutation_vcs(original, proposed)` — generates VCs comparing old and new bodies
- Precondition preservation: REQUIRE clauses of original hold under new body
- Postcondition preservation: ENSURE clauses of original hold under new body
- Capability check: no new capabilities required by new body

**`al-types`:**
- Type check proposed change against original signature
- Verify no new capability requirements
- Verify INVARIANT preservation

**`al-runtime`:**
- Declaration table mutation with version tracking
- Rollback mechanism
- Auto-approval logic
- Escalation trigger (integrates with Phase 5 human escalation)

**`al-diagnostics`:**
- Mutation audit events: proposed change, proof results, approval decision
- Version history audit events

## 4. Testing

### 4.1 Unit Tests — Mutation VCs (`al-vc`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_mutation_precondition_preserved` | New body satisfies original REQUIRE → Valid |
| T1.2 | `test_mutation_precondition_violated` | New body violates original REQUIRE → Invalid |
| T1.3 | `test_mutation_postcondition_preserved` | New body satisfies original ENSURE → Valid |
| T1.4 | `test_mutation_postcondition_violated` | New body violates original ENSURE → Invalid |
| T1.5 | `test_mutation_no_new_caps` | New body uses same capabilities → Valid |
| T1.6 | `test_mutation_new_cap_detected` | New body requires additional capability → Invalid |
| T1.7 | `test_mutation_invariant_preserved` | INVARIANT clauses hold for new body |

### 4.2 Unit Tests — Approval (`al-runtime`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_auto_approval_all_pass` | All proofs valid → mutation applied automatically |
| T2.2 | `test_auto_approval_some_fail` | One proof invalid → mutation rejected |
| T2.3 | `test_escalate_on_failure` | Proof failure + ESCALATE policy → escalation triggered |
| T2.4 | `test_requires_self_modify` | MUTATE without SELF_MODIFY → CapabilityDenied |

### 4.3 Unit Tests — Version Management (`al-runtime`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_mutation_version_created` | After mutation, version count increments |
| T3.2 | `test_rollback_restores_previous` | ROLLBACK returns to previous body |
| T3.3 | `test_rollback_at_v1_fails` | ROLLBACK on original version → error |
| T3.4 | `test_version_history` | VERSIONS returns ordered list of versions |

### 4.4 Unit Tests — Registration (`al-runtime`)

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_register_new_operation` | REGISTER creates callable operation |
| T4.2 | `test_register_requires_proof` | Registration without proof obligations → error |
| T4.3 | `test_register_duplicate_name` | Registering existing name → error |
| T4.4 | `test_registered_op_callable` | Registered operation callable by other operations |

### 4.5 Unit Tests — Parser (`al-parser`)

| ID | Test | Description |
|----|------|-------------|
| T5.1 | `test_parse_mutate_operation` | MUTATE OPERATION parses correctly |
| T5.2 | `test_parse_proof_block` | PROOF block with obligations parses correctly |
| T5.3 | `test_parse_approval_auto` | APPROVAL AUTO WHEN ... parses correctly |
| T5.4 | `test_parse_approval_escalate` | APPROVAL ESCALATE WHEN ... parses correctly |
| T5.5 | `test_parse_rollback` | ROLLBACK OPERATION parses correctly |
| T5.6 | `test_parse_register` | REGISTER OPERATION parses correctly |

### 4.6 Integration Tests

| ID | Test | Description |
|----|------|-------------|
| T6.1 | `test_mutate_e2e` | Full mutation workflow: propose → prove → approve → execute |
| T6.2 | `test_mutate_reject_e2e` | Invalid mutation: propose → prove fails → reject |
| T6.3 | `test_rollback_e2e` | Mutate then rollback → original behavior restored |
| T6.4 | `test_mutation_audit_trail` | Mutation produces complete audit trail entries |

### 4.7 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C45 | `conformance_mutate_operation` | MUTATE OPERATION applies change when proofs pass |
| C46 | `conformance_mutation_proof_required` | Mutation without valid proofs is rejected |
| C47 | `conformance_rollback` | ROLLBACK restores previous operation version |
| C48 | `conformance_mutation_audit` | Every mutation is fully audited |

## 5. Acceptance Criteria

- [ ] MUTATE OPERATION is lexed, parsed, type-checked, and executable
- [ ] All 5 proof obligations (preconditions, postconditions, caps, invariants, tests) are checked
- [ ] Auto-approval works when all proofs pass
- [ ] Escalation triggers when any proof fails
- [ ] Version history is maintained with rollback capability
- [ ] REGISTER OPERATION creates new callable operations with proof obligations
- [ ] Every mutation is fully audited (proposed change, proof results, decision)
- [ ] SELF_MODIFY capability is required
- [ ] All existing tests pass unchanged
- [ ] 4 new conformance tests (C45-C48) pass
