# Conformance Acceptance Criteria (MVP v0.1)

## C1 Parser Exactness
- Positive corpus: every declaration/statement/expression form in `GRAMMAR_MVP.ebnf`.
- Negative corpus: excluded operators/features/modules.
- Parser invariant: zero unresolved parser conflicts in generated parser artifacts.
- Lexer invariant: newline suppression/collapse edge-case matrix fully green.
- Pass: 100% positive parse + 100% negative deterministic rejection with `NOT_IMPLEMENTED` when profile-excluded.

## C2 FAILURE Shape
- Reject 2-field `FAILURE` in parse/type/pattern positions.
- Pass only 3-field canonical shape.

## C3 Result Uniformity
- Enforce fallible ops return `Result[T]`, pure `core.data` ops may return bare T.
- Source of truth: `STDLIB_MVP_SIGNATURES.json`.

## C4 Join Restriction
- Accept only `JOIN strategy: ALL_COMPLETE`.
- Reject `BEST_EFFORT`/`PARTIAL` with `NOT_IMPLEMENTED` + profile tag.

## C5 Delegation Boundary
- Caller must hold `DELEGATE`.
- Execution caps == callee caps only.
- No implicit inheritance/intersection.

## C6 SMT Unknown Policy
- On solver `Unknown(reason)`: compile succeeds with injected runtime `ASSERT`.
- Runtime false assert => `FAILURE(ASSERTION_FAILED, ..., details{vc_id,solver_reason})`.
- Audit events for assertion flow must include `vc_id` and `solver_reason` as required fields.

## C7 Stdlib Signature Lock
- Auto-generated tests validate ops against `STDLIB_MVP_SIGNATURES.json` + `stdlib_spec_mvp.md`.

## C8 Excluded Feature Rejection
- All excluded syntax/modules rejected with profile-tagged diagnostic.

## C9 Capability Alias Normalization
- Deprecated aliases accepted with warning + canonical normalization.

## C10 ASSERT/RETRY/ESCALATE
- `ASSERT`: full runtime semantics.
- `RETRY`: deterministic attempt budget semantics (`RETRY(n)`), stable handling for unsupported policy branches.
- `ESCALATE`: deterministic escalation event + terminal failure behavior with stable diagnostics.
- `RETRY`/`ESCALATE`: parser+typechecker acceptance with profile-conformant runtime handling.

### C10 deterministic branch-to-code matrix
- `RETRY(n)` where `n` is non-integer/invalid type -> `TYPE_MISMATCH` (compile-time)
- `RETRY` with unsupported policy option -> `NOT_IMPLEMENTED` (compile-time when static; runtime otherwise)
- `ESCALATE` with missing required local escalation policy -> `CAPABILITY_DENIED`
- `ESCALATE` using explicitly non-MVP policy branch -> `NOT_IMPLEMENTED`
- terminal escalation outcome -> `FAILURE(ESCALATED, message, details)` + required `ESCALATED` audit event

## Diagnostic Code Determinism Matrix (C1-C10)
- C1/C8 excluded syntax or modules: `NOT_IMPLEMENTED` + `profile: mvp-0.1`
- C2 failure arity mismatch in any position (parse/type/pattern): `FAILURE_ARITY_MISMATCH`
- C3/C7 fallible return mismatch: `TYPE_MISMATCH`
- C4 non-MVP join strategy: `NOT_IMPLEMENTED`
- C5 capability violation: `CAPABILITY_DENIED`
- C6 invalid VC: `VC_INVALID`; runtime failed inserted assert: `ASSERTION_FAILED`
- C9 alias normalization: warning `CAP_ALIAS_DEPRECATED` (non-fatal)
- C10 as above branch matrix

## Error Taxonomy Normalization Rule
- Canonical capability authorization failure code for MVP is **`CAPABILITY_DENIED`**.
- `UNAUTHORIZED` is treated as deprecated/non-canonical; normalize or reject with deterministic diagnostic policy in conformance fixtures.
