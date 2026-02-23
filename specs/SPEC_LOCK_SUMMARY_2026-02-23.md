# AgentLang MVP Spec Lock Summary — 2026-02-23

## Files Changed (alignment pass)

- `specs/MVP_PROFILE.md`
- `specs/GRAMMAR_MVP.ebnf`
- `specs/formal_semantics.md`
- `specs/stdlib_spec_mvp.md`
- `specs/AgentLang_Specification_v1.0.md`
- `specs/README.md`

## Contradictions Fixed

1. **Canonical result/failure shape unified**
   - Standardized to: `Result[T] = SUCCESS(T) | FAILURE(ErrorCode, message: Str, details: FailureDetails)`
   - Removed/avoided 2-field FAILURE in MVP normative path.

2. **`Any` removed from MVP stdlib surface**
   - Replaced with concrete types (`RegexResult`, `TokenizeResult`, etc.) in `stdlib_spec_mvp.md`.

3. **MVP concurrency restriction made explicit**
   - `JOIN strategy: ALL_COMPLETE` only.
   - `BEST_EFFORT` / `PARTIAL(min=k)` compile-time rejected with `NOT_IMPLEMENTED` + `mvp-0.1` profile tag.

4. **Delegation capability boundary aligned**
   - Caller must hold `DELEGATE`.
   - Callee executes with callee capabilities only.
   - No implicit capability inheritance/intersection.

5. **SMT `Unknown` policy made deterministic**
   - Compile-time fail-open via inserted runtime `ASSERT`.
   - Runtime fail-closed with auditable failure (`vc_id`, `solver_reason`).

6. **Output clause alignment in MVP grammar path**
   - MVP grammar canonicalized to `OUTPUT type_expr`.
   - MVP-oriented examples and grammar fragments aligned to this form.

7. **Conformance checklist hardened**
   - README checklist updated to reflect what is normatively executable vs parser/policy-level in MVP.

## Remaining Risks (non-blocking for MVP reference implementation)

- Full v1 spec still contains non-MVP examples/features; these are profile-gated but should be clearly labeled during publication.
- `RETRY`/`ESCALATE` are intentionally not fully operationalized in MVP formal semantics; runtime behavior is governed by policy profile and implementation contract.

## Readiness Score

**89 / 100**

### Justification

- Core normative contradictions that block Rust implementation are resolved.
- MVP parser/type/runtime boundaries are now explicit.
- Conformance checks are testable from spec text.
- Remaining items are mostly publication clarity and vNext semantics, not MVP blockers.

## Verdict

**MVP v0.1 spec package is ready for Rust reference implementation (Phase 1).**