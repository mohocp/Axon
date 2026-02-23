# AgentLang MVP v0.1 Rust Implementation Plan (Execution-Ready, v2)

Date: 2026-02-23  
Scope: `MVP_PROFILE.md`, `GRAMMAR_MVP.ebnf`, `formal_semantics.md`, `stdlib_spec_mvp.md`, `README.md`, `CONFORMANCE_ACCEPTANCE_CRITERIA.md`

## 1) Delivery Target

Build a Rust reference implementation that passes C1-C10 with deterministic diagnostics and CI-gated conformance.

- Profile: `mvp-0.1`
- Non-MVP: compile-time `NOT_IMPLEMENTED` + profile tag
- Canonical failures: `FAILURE(ErrorCode, message: Str, details: FailureDetails)`

## 2) Architecture

Pipeline:
1. Lexer (strict NEWLINE/terminator behavior)
2. Parser (MVP grammar only)
3. Profile gate (reject excluded syntax/modules)
4. Name resolution + capability alias normalization
5. AST -> HIR lowering
6. Type checker + VC generation
7. VC solve (`Valid | Invalid | Unknown`)
8. Unknown rewrite: inject runtime `ASSERT` with `vc_id`, `solver_reason`
9. Runtime interpreter (scheduler, capability checks, checkpoint, audit)

Runtime state follows formal semantics model (`H,R,M,K,Q,L`).

## 3) Workspace / Crates

```text
agentlang/
  crates/
    al-cli/
    al-lexer/
    al-parser/
    al-ast/
    al-hir/
    al-types/
    al-vc/
    al-capabilities/
    al-runtime/
    al-stdlib-mvp/
    al-checkpoint/
    al-diagnostics/
    al-conformance/
  specs/
    STDLIB_MVP_SIGNATURES.json
    CONFORMANCE_ACCEPTANCE_CRITERIA.md
```

## 4) Parser Strategy (C1)

- Use `lalrpop` + handwritten lexer.
- Source of truth: `GRAMMAR_MVP.ebnf`.
- Canonical OUTPUT form: `OUTPUT type_expr`.
- `JOIN` accepts only `ALL_COMPLETE`.
- Deterministic parser diagnostics for excluded syntax.

### C1 exactness proof method
- **Positive corpus:** one fixture per production/variant in grammar.
- **Negative corpus:** excluded operators/modules/join variants/non-MVP constructs.
- Coverage gate: every grammar rule referenced by at least one positive test.
- **Parser invariant gate:** zero unresolved parser conflicts (shift/reduce or reduce/reduce) in generated parser artifacts.
- **Lexer newline matrix gate:** explicit edge-case matrix covering all suppression/collapse rules from `GRAMMAR_MVP.ebnf` (after/before token classes and nested delimiters).
- CI gate: 100% pass on corpora + parser invariant + newline matrix.

## 5) AST/HIR

HIR must explicitly model: `ASSERT`, `RETRY`, `ESCALATE`, `CHECKPOINT`, `RESUME`, `FORK/JOIN`, `DELEGATE`.

Each HIR node carries:
- `span`
- `type`
- `required_caps`
- `profile`
- `synthetic` (for inserted ASSERTs)

## 6) Type Checker + VC (C2/C3/C6/C10)

Checker passes:
1. Declaration/type table
2. Expression typing
3. Pattern arity + exhaustiveness
4. Capability requirements
5. VC generation

Mandatory rules:
- C2: only 3-field `FAILURE` accepted.
- C3/C7: fallible stdlib ops return `Result[T]` (from signature manifest).
- C5: delegation boundary enforced statically where possible.
- C10: `RETRY`/`ESCALATE` accepted in parse/type phases with deterministic MVP runtime contract and diagnostics.

### Unknown policy
- `Invalid`: compile error (`VC_INVALID`).
- `Unknown(reason)`: inject runtime `ASSERT` with `{vc_id, solver_reason}`.
- Runtime assert false: fail-closed with `ASSERTION_FAILED`.

## 7) Capability Model (C5/C9)

- Canonical enum from `MVP_PROFILE.md`.
- Alias normalization warnings (`CAP_ALIAS_DEPRECATED`).
- Runtime guard emits `CAPABILITY_DENIED`.
- Delegation semantics: caller must have `DELEGATE`; effective caps = callee caps only.

## 8) Runtime Scheduler + Join (C4)

- DAG execution for pipelines/forks.
- `ALL_COMPLETE` only.
- Any non-MVP join variant rejected during compile.
- Deterministic failure propagation for fork branches.

### C10 Runtime Contract: `RETRY` / `ESCALATE` (normative for MVP implementation)

`RETRY` policy:
- `RETRY(n)` performs up to `n` additional attempts for the current operation stage.
- `n` must be `>= 0`; invalid values are compile-time `TYPE_MISMATCH`.
- Retry scheduling is deterministic in test mode (fixed policy seed and ordering).
- Backoff policy for MVP: `none` by default; unsupported backoff options emit deterministic `NOT_IMPLEMENTED`.
- If all retries fail, emit terminal `FAILURE` from the final attempt.

`ESCALATE` policy:
- `ESCALATE(msg?)` emits an escalation event and terminates current operation stage with `FAILURE(ESCALATED, ...)`.
- Escalation target resolution in MVP is policy-local only (no dynamic discovery).
- Missing/invalid escalation policy yields deterministic `CAPABILITY_DENIED` (authorization/policy denial) or `NOT_IMPLEMENTED` (non-MVP branch), depending on failure mode.

Deterministic diagnostics:
- Unsupported policy branches for `RETRY`/`ESCALATE` are compile-time when statically known, otherwise runtime with stable error code mapping in conformance fixtures.

## 9) Checkpoint/Resume Contract

Snapshot scope: task-local runtime state only.

Persisted fields:
- execution env/registers
- task DAG pending state
- references required for resume
- capability context required by resumed task
- metadata: `{checkpoint_id, created_at, profile, schema_version, hash}`

Deterministic boundary and idempotency:
- Checkpoint captures state **between** effectful stage commits (quiescent boundary).
- External side effects must be recorded in an effect journal with idempotency keys.
- On resume, runtime replays only non-committed effects; committed effects are not re-issued.
- Missing idempotency key for effectful stage is compile-time error when provable, else runtime `CHECKPOINT_INVALID`.

Restore rules:
- verify schema version + hash
- verify effect-journal consistency and idempotency keys
- reject incompatible snapshots with `CHECKPOINT_INVALID`
- resume under same profile/capability context

## 10) Error + Diagnostic Contract

Compile diagnostics shape:
```json
{ "code": "...", "message": "...", "span": {...}, "profile": "mvp-0.1", "notes": [] }
```

Runtime failure shape:
```json
{ "kind":"FAILURE", "code":"...", "message":"...", "details": {...} }
```

Required code mapping by checklist item is maintained in `al-conformance` fixtures and `CONFORMANCE_ACCEPTANCE_CRITERIA.md`.

Minimum mandatory deterministic error/warning codes:
- `NOT_IMPLEMENTED`
- `TYPE_MISMATCH`
- `FAILURE_ARITY_MISMATCH`
- `CAPABILITY_DENIED`
- `VC_INVALID`
- `ASSERTION_FAILED`
- `CHECKPOINT_INVALID`
- warning: `CAP_ALIAS_DEPRECATED`

Taxonomy normalization (MVP):
- canonical capability authorization failure code: `CAPABILITY_DENIED`
- `UNAUTHORIZED` is deprecated/non-canonical and must be normalized or rejected deterministically

## 11) Audit Event Schema (C6 hard requirement)

JSONL base schema:
```json
{
  "event_id": "uuid",
  "timestamp": "iso8601",
  "agent_id": "string",
  "task_id": "string",
  "event_type": "ASSERT_INSERTED|ASSERT_FAILED|CAPABILITY_DENIED|CHECKPOINT_CREATED|CHECKPOINT_RESTORED|ESCALATED",
  "profile": "mvp-0.1",
  "details": {}
}
```

Event-type required fields:
- `ASSERT_INSERTED` => `details.vc_id` (required), `details.solver_reason` (required)
- `ASSERT_FAILED` => `details.vc_id` (required), `details.solver_reason` (required)
- `CAPABILITY_DENIED` => `details.capability` (required)
- `CHECKPOINT_CREATED` => `details.checkpoint_id` (required)
- `CHECKPOINT_RESTORED` => `details.checkpoint_id` (required)
- `ESCALATED` => `details.escalation_reason` (required), `details.policy` (required), `details.target` (required)

CI asserts schema validity **and** required-field presence by event type.

## 12) Conformance Suite (C1-C10)

Source of truth:
- `CONFORMANCE_ACCEPTANCE_CRITERIA.md`
- `STDLIB_MVP_SIGNATURES.json`

Test generation:
- generate signature-lock tests from manifest
- generate rejection tests for excluded features
- maintain explicit mapping: C-item -> test ids -> expected diagnostic code

## 13) CI Strategy

Required jobs:
- fmt / clippy / build
- unit + integration
- conformance C1-C10
- signature-lock generation + check
- audit schema validation tests
- MSRV build
- cargo audit

Artifacts:
- conformance report (md+json)
- diagnostics snapshot diff
- audit events sample validation report

## 14) Milestones with Entry/Exit Gates

### W1 Foundation
- Entry: workspace clean
- Exit: crate skeleton + diagnostics infra + conformance harness skeleton merged

### W2 Parser (C1/C8)
- Exit gate: positive/negative parser corpora green; grammar-rule coverage report generated

### W3 Types (C2/C3/C7)
- Exit gate: failure arity enforcement + signature-lock tests green

### W4 VC/Capabilities (C5/C6/C9)
- Exit gate: Unknown->ASSERT tested; alias warnings + delegation boundary tests green

### W5 Runtime (C4/C10)
- Exit gate: ALL_COMPLETE scheduler + deterministic retry/escalate policy diagnostics green

### W6 Checkpoint/Audit
- Exit gate: checkpoint restore integrity tests + audit schema validation green

### W7 Stdlib Completion
- Exit gate: all included MVP stdlib ops implemented and contract-tested

### W8 Hardening/RC
- Exit gate: full C1-C10 pass in CI + release candidate docs/artifacts produced

## 15) Risk Hotspots / Mitigations

- Lexer newline drift -> parser exactness corpus + rule coverage in CI
- Signature drift -> manifest-generated tests (`STDLIB_MVP_SIGNATURES.json`)
- Unknown path regressions -> mandatory audit-schema checks in CI
- Delegation leakage -> property tests over nested delegation
- Checkpoint corruption -> version/hash validation + strict reject on mismatch

## 16) Definition of Done

Done when:
1. C1-C10 pass in CI with deterministic diagnostics.
2. Signature-lock tests auto-generated from manifest and passing.
3. Audit schema validated with required fields (`vc_id`, `solver_reason` where applicable).
4. Runtime honors canonical failure shape and delegation boundary.
5. RC shipped with conformance matrix and known limitations.
