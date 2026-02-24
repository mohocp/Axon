# Conformance Matrix — AgentLang v0.1.0-rc1

**Profile:** mvp-0.1
**Date:** 2026-02-24
**Test command:** `cargo test -p al-conformance --test conformance -- --nocapture`

---

## Summary

| Status | Count |
|--------|-------|
| PASS | 20/20 |
| FAIL | 0 |
| SKIP | 0 |

**Result: CONFORMANT**

---

## Conformance Requirements

### Core Requirements (C1-C10)

| ID | Requirement | Parse | Type Check | Runtime | Tests | Status |
|----|------------|-------|------------|---------|-------|--------|
| C1 | Lex/parse round-trip: all MVP grammar constructs | PASS | PASS | — | `c1_lex_parse_roundtrip`, `c1_all_declaration_types_parse`, `c1_tokens_have_spans`, `c1_empty_source_parses`, `c1_negative_invalid_keyword_as_identifier` | PASS |
| C2 | FAILURE arity: 3-field canonical form enforced | PASS | PASS | — | `c2_failure_arity_3_fields`, `c2_failure_2_fields_rejected` | PASS |
| C3 | Capability deny: agent CAPABILITIES/DENY/TRUST_LEVEL | PASS | PASS | PASS | `c3_capability_deny`, `c3_capability_runtime_check`, `c3_multiple_agents_with_capabilities` | PASS |
| C4 | Fork/join: ALL_COMPLETE only; BEST_EFFORT/PARTIAL rejected | PASS | PASS | PASS | `c4_fork_join_parse`, `c4_fork_join_runtime`, `c4_non_mvp_join_strategy_rejected` | PASS |
| C5 | Checkpoint/resume: CHECKPOINT and RESUME statements | PASS | PASS | PASS | `c5_checkpoint_parse`, `c5_checkpoint_runtime`, `c5_checkpoint_resume_full` | PASS |
| C6 | Pipeline composition: arrow and pipe-forward operators | PASS | PASS | — | `c6_pipeline_parse` | PASS |
| C7 | Audit trail: ASSERT/REQUIRE/ENSURE with VC generation | PASS | PASS | PASS | `c7_audit_assert_parse`, `c7_audit_runtime`, `c7_vc_generation_from_require_ensure_assert`, `c7_vc_invalid_emits_compile_error`, `c7_vc_unknown_rewrite_is_synthetic_assert` | PASS |
| C8 | Excluded features: non-MVP constructs rejected | PASS | PASS | — | `c8_valid_mvp_subset`, `c8_profile_is_mvp`, `c8_malformed_failure_arity_rejected`, `c8_partial_join_rejected` | PASS |
| C9 | Type checking: duplicate definitions detected | PASS | FAIL (expected) | — | `c9_duplicate_type_detected`, `c9_duplicate_schema_detected`, `c9_duplicate_operation_detected`, `c9_duplicate_pipeline_detected`, `c9_duplicate_agent_detected` | PASS |
| C10 | Retry/escalation: RETRY(n) and ESCALATE semantics | PASS | PASS | PASS | `c10_retry_escalate_parse`, `c10_retry_runtime`, `c10_retry_exhausted_returns_failure`, `c10_escalation_runtime`, `c10_escalation_with_audit_details` | PASS |

### Extended Positive Requirements (C11-C14)

| ID | Requirement | Parse | Type Check | Runtime | Tests | Status |
|----|------------|-------|------------|---------|-------|--------|
| C11 | Match arm body: statement keywords after `->` | PASS | PASS | — | `c11_match_body_statement_keywords` | PASS |
| C12 | Undefined type reference: type checker rejects unknown types | PASS | FAIL (expected) | — | `c12_undefined_type_detected`, `c12_builtin_types_resolve` | PASS |
| C13 | Parser error recovery: continues after syntax errors | PASS | PASS | — | `c13_parser_recovery` | PASS |
| C14 | REQUIRE clause: validates references to operation inputs | PASS | PASS | — | `c14_require_valid_input_reference`, `c14_require_unknown_identifier` | PASS |

### Negative Requirements (C15-C20)

| ID | Requirement | Expected | Actual | Tests | Status |
|----|------------|----------|--------|-------|--------|
| C15 | Malformed FAILURE (2 fields) | PARSE FAIL | PARSE FAIL | `c2_failure_2_fields_rejected` (shared) | PASS |
| C16 | PARTIAL join strategy rejected | PARSE FAIL | PARSE FAIL | `c8_partial_join_rejected` (shared) | PASS |
| C17 | BEST_EFFORT join strategy rejected | PARSE FAIL | PARSE FAIL | `c4_non_mvp_join_strategy_rejected` (shared) | PASS |
| C18 | Duplicate schema definition rejected | TYPE FAIL | TYPE FAIL | `c9_duplicate_schema_detected` (shared) | PASS |
| C19 | ENSURE postcondition accepted | PARSE OK | PARSE OK | `c19_ensure_clause_accepted` | PASS |
| C20 | INVARIANT in OPERATION accepted | PARSE OK | PARSE OK | `c20_operation_invariant_accepted` | PASS |

### Meta Test

| Test | Description | Status |
|------|-------------|--------|
| `all_fixtures_conform` | Validates all 20 fixtures against their expected parse/typecheck/declaration-count outcomes | PASS |

---

## Test Inventory

**Total conformance tests:** 45
**All passing:** Yes

### Tests by Conformance Requirement

| Requirement | Test Count |
|-------------|-----------|
| C1 | 5 |
| C2 | 2 |
| C3 | 3 |
| C4 | 3 |
| C5 | 3 |
| C6 | 1 |
| C7 | 5 |
| C8 | 4 |
| C9 | 5 |
| C10 | 5 |
| C11 | 1 |
| C12 | 2 |
| C13 | 1 |
| C14 | 2 |
| C15-C18 | (covered by C2/C4/C8/C9 negative tests) |
| C19 | 1 |
| C20 | 1 |
| Meta | 1 |

---

## Governing Specifications

| Document | Path |
|----------|------|
| MVP Profile | `specs/MVP_PROFILE.md` |
| Grammar | `specs/GRAMMAR_MVP.ebnf` |
| Formal Semantics | `specs/formal_semantics.md` |
| Stdlib Spec | `specs/stdlib_spec_mvp.md` |
| Acceptance Criteria | `specs/CONFORMANCE_ACCEPTANCE_CRITERIA.md` |
