# AgentLang MVP v0.1 — Implementation Roadmap (Round 3+)

**Date:** 2026-02-24
**Baseline:** Round 2 complete (265 tests, 13 crates, ~11.2K LOC)
**Target:** C1–C10 full conformance, end-to-end execution, RC candidate

---

## Current State Assessment

### What's Done (Rounds 1–2)

| Area | Status | Coverage |
|------|--------|----------|
| Lexer (al-lexer) | **Complete** | 95 tests, full NEWLINE suppression |
| Parser (al-parser) | **Complete** | 26 tests, error recovery, all MVP grammar |
| AST (al-ast) | **Complete** | All declaration/statement/expression types |
| HIR (al-hir) | **Structural only** | Lowering works; `ty`/`required_caps` unpopulated |
| Type checker (al-types) | **6 passes, partial** | Declaration table, failure arity, excluded features, type refs, REQUIRE, pipeline/fork refs (warnings only) |
| Capabilities (al-capabilities) | **Complete** | 22 caps, alias normalization, grant/deny/delegation |
| Runtime (al-runtime) | **Stubs** | Value types, state model (H/R/M/K/Q/L), fork-join/retry stubs |
| Stdlib (al-stdlib-mvp) | **Registry only** | 21 ops registered, 0 implemented |
| VC (al-vc) | **Stubs** | Type definitions only, no generation or solving |
| Checkpoint (al-checkpoint) | **Stubs** | Store/restore signatures, no implementation |
| Diagnostics (al-diagnostics) | **Complete** | All error/warning codes, audit event schema |
| Conformance (al-conformance) | **C1–C14 fixtures** | 27 integration tests |
| CLI (al-cli) | **Basic** | lex/parse/check/run commands |

### Open Issues from Round 2 Review

| ID | Priority | Description | Target |
|----|----------|-------------|--------|
| #6 | P2 | Pipeline type propagation (warning-only) | Round 3 |
| #7 | P2 | Fork branch type validation deferred | Round 3 |
| #8 | P2 | HIR lowering discards expression details | Round 3 |
| #17 | P2 | REQUIRE doesn't know STORE bindings | Round 4 |
| #18 | P2 | UnresolvedReference warning lacks serde roundtrip test | Round 3 |
| #19 | P2 | parse_recovering doesn't recover from lex errors | Round 4 |
| #21 | P3 | BUILTIN_TYPES needs extraction to shared constant | Round 3 |
| #22 | P3 | Unresolved pipeline warnings are noisy | Round 4 |
| #23 | P3 | Synthetic block spans reuse statement span | Backlog |

### W-Milestone Mapping

The original plan defined W1–W8. Current completion:

| Milestone | Status | Notes |
|-----------|--------|-------|
| W1 Foundation | **Done** | Workspace, diagnostics, conformance harness |
| W2 Parser (C1/C8) | **Done** | Grammar exactness, excluded feature rejection |
| W3 Types (C2/C3/C7) | **Partial** | Failure arity done; type inference, signature-lock missing |
| W4 VC/Caps (C5/C6/C9) | **Partial** | Capabilities done; VC generation/solving missing |
| W5 Runtime (C4/C10) | **Partial** | Stubs exist; no end-to-end execution |
| W6 Checkpoint/Audit | **Stubs** | Types defined; no implementation |
| W7 Stdlib | **Stubs** | Registry exists; 0/21 ops implemented |
| W8 Hardening/RC | **Not started** | — |

---

## Roadmap: Rounds 3–7

### Guiding Principles

1. **MVP-first.** Ship the smallest thing that passes C1–C10. Defer optimization and ergonomics.
2. **Vertical slices.** Each round should produce new observable behavior, not just internal plumbing.
3. **Test-before-ship.** No round is complete without conformance tests covering its claims.
4. **Incremental risk.** Tackle the hardest unknowns (type inference, VC pipeline) before the mechanical work (stdlib, CI).

---

### Round 3 — Type Inference & HIR Enrichment

**Goal:** Complete the static analysis pipeline so the type checker can validate real programs end-to-end.

**Completes:** W3 (Types), addresses P2 #6/#7/#8/#18/#21.

#### Deliverables

| # | Deliverable | Acceptance Criteria |
|---|-------------|---------------------|
| 3.1 | **Expression type inference** | `check_expr()` returns a resolved type for all expression kinds (literals, identifiers, binary ops, calls, member access, list/map constructors). Tests cover each kind. |
| 3.2 | **STORE/MUTABLE type tracking** | Type environment tracks bindings introduced by STORE and MUTABLE. Subsequent references resolve to the bound type. |
| 3.3 | **Operation signature typing** | INPUT/OUTPUT types fully resolved. Call expressions to user operations are type-checked against declared signatures. |
| 3.4 | **Pipeline type propagation** | Output type of stage N is checked against input type of stage N+1. Type mismatch emits `TYPE_MISMATCH`. Upgrade from warning to error. |
| 3.5 | **Fork branch type validation** | Each fork branch's result type is inferred. All branches must unify for the JOIN result. |
| 3.6 | **HIR `ty` population** | After type checking, every `HirMeta.ty` is populated with the inferred type. Round-trip test: parse → lower → type-check → verify all HIR nodes have types. |
| 3.7 | **BUILTIN_TYPES extraction** | Move to a shared `al-types::builtins` module importable by other crates. |
| 3.8 | **Serde roundtrip for WarningCode** | Add test covering `UnresolvedReference` serialization. |

#### Exit Gate

- All existing 265 tests still pass (no regressions).
- New tests: ≥20 type inference tests, ≥5 pipeline propagation tests, ≥3 HIR enrichment tests.
- C3 (Result uniformity) and C7 (stdlib signature lock) fully enforced at type level.

#### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Generic type unification complexity | Medium | MVP restricts to monomorphic instantiation; defer full polymorphic inference. |
| Pipeline stages referencing stdlib ops without signatures | Medium | Use `stdlib_spec_mvp.md` signatures as ground truth; generate `STDLIB_MVP_SIGNATURES.json` manifest. |
| Breaking existing conformance tests | Low | Run full suite after each sub-deliverable. |

---

### Round 4 — Verification Conditions & Capability Enforcement

**Goal:** Wire the VC pipeline (generate → solve → Unknown rewrite) and enforce delegation boundary statically.

**Completes:** W4 (VC/Capabilities).

#### Deliverables

| # | Deliverable | Acceptance Criteria |
|---|-------------|---------------------|
| 4.1 | **VC generation from REQUIRE/ENSURE** | `al-vc` produces VCs from REQUIRE (preconditions) and ENSURE (postconditions) clauses. Each VC has a unique `vc_id`. |
| 4.2 | **VC generation from ASSERT** | Explicit ASSERT statements produce VCs. User-written ASSERTs are distinct from compiler-injected ones (`synthetic` flag). |
| 4.3 | **VC generation from INVARIANT** | LOOP invariants produce VCs checked at loop entry and each iteration boundary. |
| 4.4 | **Stub solver with configurable results** | Solver returns `Valid`/`Invalid`/`Unknown` based on configuration (test mode: programmable; prod mode: always `Unknown` until real solver integrated). |
| 4.5 | **Unknown → ASSERT injection** | When solver returns `Unknown(reason)`, compiler injects a synthetic runtime ASSERT with `{vc_id, solver_reason}` in HIR. HIR node has `synthetic: true`. |
| 4.6 | **Invalid → compile error** | When solver returns `Invalid`, emit `VC_INVALID` diagnostic with counterexample info. |
| 4.7 | **Delegation static check** | Type checker verifies caller holds `DELEGATE` capability at DELEGATE sites. Emits `CAPABILITY_DENIED` on violation. |
| 4.8 | **HIR `required_caps` population** | After VC/capability pass, every HIR node carries its inferred capability requirements. |
| 4.9 | **REQUIRE scope expansion** | REQUIRE expressions can reference STORE bindings from enclosing scope (#17). |

#### Exit Gate

- C5 (delegation boundary) fully enforced.
- C6 (SMT Unknown policy) tested end-to-end: Unknown → injected ASSERT → runtime failure path.
- C9 (alias normalization) still passing.
- New tests: ≥15 VC tests, ≥5 delegation tests, ≥3 Unknown-rewrite tests.

#### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| VC generation for complex expressions | Medium | Restrict MVP VCs to simple boolean/comparison expressions; flag complex ones as `Unknown`. |
| Solver integration (Z3/CVC5) | High | Defer real solver to post-MVP. Use configurable stub that defaults to `Unknown` for anything non-trivial. |
| REQUIRE scope expansion complexity | Low | Only add STORE bindings visible at the operation level; don't recurse into nested scopes. |

---

### Round 5 — Runtime Interpreter

**Goal:** Execute AgentLang programs end-to-end. This is the "it actually runs" round.

**Completes:** W5 (Runtime).

#### Deliverables

| # | Deliverable | Acceptance Criteria |
|---|-------------|---------------------|
| 5.1 | **Statement interpreter** | Evaluate STORE, MUTABLE, ASSIGN, MATCH, LOOP, EMIT, HALT in sequence. State updates reflected in H/R/M. |
| 5.2 | **Expression evaluator** | Evaluate all expression kinds to `Value`. Binary ops, unary ops, member access, list/map constructors, function calls (dispatch to operation bodies or stdlib). |
| 5.3 | **Pattern matching runtime** | MATCH/WHEN/OTHERWISE evaluates patterns against values. SUCCESS/FAILURE destructuring works. Wildcard `_` and literal patterns work. |
| 5.4 | **Pipeline execution** | Execute pipeline stages sequentially, threading output → input. Result short-circuits on FAILURE. |
| 5.5 | **Fork/Join (ALL_COMPLETE)** | Execute fork branches (sequentially in MVP; concurrent optional). Collect all results. Any FAILURE → whole fork fails. |
| 5.6 | **RETRY runtime** | `RETRY(n)` re-executes the failing stage up to `n` times. Terminal failure returns last attempt's FAILURE value. |
| 5.7 | **ESCALATE runtime** | `ESCALATE(msg)` emits ESCALATED audit event and returns `FAILURE(ESCALATED, msg, details)`. |
| 5.8 | **ASSERT runtime** | Evaluate ASSERT condition. True → continue. False → `FAILURE(ASSERTION_FAILED, ..., {vc_id, solver_reason})`. Emit audit event. |
| 5.9 | **Capability runtime checks** | Before executing capability-gated operations, check agent's capability set. Denied → `CAPABILITY_DENIED` + audit event. |
| 5.10 | **DELEGATE runtime** | Execute delegated operation under callee agent's capabilities. Caller's caps are not inherited. |
| 5.11 | **CLI `run` end-to-end** | `al-cli run program.al` executes a complete program and prints the final result or failure. |

#### Exit Gate

- C4 (Join restriction) fully enforced at runtime.
- C10 (ASSERT/RETRY/ESCALATE) deterministic branch-to-code matrix fully tested.
- At least 3 non-trivial programs execute end-to-end through `al-cli run`.
- New tests: ≥30 runtime execution tests, ≥5 end-to-end CLI tests.

#### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Infinite loops in bounded LOOP | Low | Enforce `max:` bound at runtime; panic/error if exceeded. |
| Recursive operation calls | Medium | MVP: disallow recursion (detect at type-check time or runtime depth limit). |
| Value representation gaps | Medium | Expand `Value` enum as needed; ensure all AST literal types have Value counterparts. |

---

### Round 6 — Stdlib, Checkpoint & Audit

**Goal:** Implement the 21 stdlib operations, checkpoint/resume persistence, and audit trail JSONL emission.

**Completes:** W6 (Checkpoint/Audit), W7 (Stdlib).

#### Deliverables

| # | Deliverable | Acceptance Criteria |
|---|-------------|---------------------|
| 6.1 | **core.data operations (7)** | FILTER, MAP, REDUCE, SORT, GROUP, TAKE, SKIP implemented. Pure functions returning bare `T`. Tests for each with list/map inputs. |
| 6.2 | **core.io operations (3)** | READ, WRITE, FETCH implemented. Return `Result[T]`. File I/O for READ/WRITE; HTTP stub for FETCH. |
| 6.3 | **core.text operations (4)** | PARSE, FORMAT, REGEX, TOKENIZE implemented. Return `Result[...]` with correct types (RegexResult, TokenizeResult). |
| 6.4 | **core.http operations (2)** | GET, POST implemented. Return `Result[HttpResponse[T]]`. Use real or mock HTTP backend (configurable). |
| 6.5 | **agent.llm operations (3)** | GENERATE, CLASSIFY, EXTRACT implemented. Return `Result[Probable[T]]`. Backend-agnostic (trait-based). |
| 6.6 | **agent.memory operations (3)** | REMEMBER, RECALL, FORGET implemented. Return `Result[T]`. In-memory store for MVP. |
| 6.7 | **STDLIB_MVP_SIGNATURES.json** | Auto-generated manifest of all 21 operations with input/output types. Signature-lock tests validate against it. |
| 6.8 | **Checkpoint serialization** | Checkpoint captures task-local state (H/R/M/K context) to a persistent store. Versioned with schema hash. |
| 6.9 | **Resume restoration** | Resume restores state from checkpoint. Validates schema version and hash. Rejects incompatible with `CHECKPOINT_INVALID`. |
| 6.10 | **Effect journal** | External side effects recorded with idempotency keys. Resume replays only non-committed effects. |
| 6.11 | **Audit trail JSONL** | All audit events written as JSONL. Schema validated: required fields per event type (vc_id, solver_reason, capability, checkpoint_id, escalation_reason, policy, target). |

#### Exit Gate

- C7 (stdlib signature lock) fully green with manifest-generated tests.
- Checkpoint round-trip test: checkpoint → corrupt → resume → `CHECKPOINT_INVALID`.
- Audit schema validation test: every event type checked for required fields.
- New tests: ≥40 stdlib tests (≥2 per operation), ≥8 checkpoint tests, ≥6 audit tests.

#### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| LLM/HTTP backend coupling | Medium | Use trait objects for backends; mock in tests, real in prod. |
| Checkpoint serialization format | Low | Use `serde_json` for MVP; document schema for forward compatibility. |
| Stdlib type signature drift | Low | Signature-lock tests auto-generated from manifest; CI blocks on mismatch. |

---

### Round 7 — Hardening, CI & Release Candidate

**Goal:** Full C1–C10 conformance in CI, polish, and produce a release candidate.

**Completes:** W8 (Hardening/RC).

#### Deliverables

| # | Deliverable | Acceptance Criteria |
|---|-------------|---------------------|
| 7.1 | **CI pipeline** | GitHub Actions: `fmt` + `clippy` + `build` + `test` + conformance + signature-lock + audit-schema + MSRV + `cargo audit`. |
| 7.2 | **Full C1–C10 conformance** | Every conformance criterion has ≥2 positive and ≥1 negative test. All green. Conformance report (md + json) generated. |
| 7.3 | **Diagnostics snapshot tests** | Diagnostic output for all error/warning codes snapshot-tested. Regressions caught automatically. |
| 7.4 | **CLI diagnostic formatting** | Source snippets with caret underlines in error output. JSON/JSONL output mode for tooling. |
| 7.5 | **Property-based tests** | `proptest` for lexer (arbitrary token sequences), parser (fuzz with valid/invalid inputs), type checker (random type expressions). |
| 7.6 | **Negative conformance fixtures** | C15+: SPAWN rejection, CHANNEL rejection, malformed FAILURE arity, non-MVP join variants, excluded stdlib modules. |
| 7.7 | **Session/CompilationUnit struct** | Single struct threading source → tokens → AST → HIR → type env → diagnostics → audit trail. Replaces ad-hoc passing. |
| 7.8 | **Documentation** | README with quick-start, language overview, and CLI usage. `--help` text for all commands. |
| 7.9 | **Release artifacts** | Tagged release with binary, conformance report, known limitations. |

#### Exit Gate

- C1–C10 all PASS in CI with zero flaky tests.
- `cargo clippy` clean (zero warnings).
- MSRV build succeeds.
- Conformance matrix published.

#### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Flaky concurrent tests | Low | MVP fork-join runs sequentially; no real concurrency in test mode. |
| MSRV compatibility | Low | Pin MSRV early; test in CI from Round 3. |
| Scope creep | Medium | Strict: only C1–C10 conformance matters. Defer everything else. |

---

## Dependency Graph

```
Round 3 (Types/HIR)
  │
  ├──→ Round 4 (VC/Capabilities)
  │       │
  │       └──→ Round 5 (Runtime)
  │               │
  │               ├──→ Round 6 (Stdlib/Checkpoint/Audit)
  │               │       │
  │               │       └──→ Round 7 (Hardening/RC)
  │               │
  │               └──→ Round 7 (Hardening/RC)
  │
  └──→ Round 5 (Runtime)  [partial — execution doesn't need VCs]
```

**Critical path:** Round 3 → Round 5 → Round 6 → Round 7

**Parallelizable:** Round 4 (VC) can proceed in parallel with early Round 5 work (statement interpreter, expression evaluator) since runtime execution doesn't strictly require VC solving.

---

## Effort Estimates (Relative)

| Round | Relative Size | Key Complexity |
|-------|---------------|----------------|
| Round 3 | Medium | Type inference is the hardest design problem |
| Round 4 | Small–Medium | Mostly plumbing; real solver deferred |
| Round 5 | Large | Most new code; broadest surface area |
| Round 6 | Medium | Mechanical but high volume (21 ops) |
| Round 7 | Small–Medium | Polish and CI; no new semantics |

---

## Post-MVP Backlog (Out of Scope)

These items are explicitly deferred beyond the MVP RC:

- Real SMT solver backend (Z3/CVC5 integration)
- Concurrent fork-join execution (thread pool / async)
- REPL / language server (LSP)
- BEST_EFFORT / PARTIAL join strategies
- SPAWN, CHANNEL, OBSERVE, reactive semantics
- Performance: string interning, arena allocation
- Full polymorphic type inference
- Self-modification flow
- Dynamic agent discovery
- File-watching / incremental compilation
- Advanced backoff policies for RETRY
- Fault-tolerant lexer (lex error recovery)

---

## Success Criteria for MVP v0.1 RC

The implementation is done when:

1. **C1–C10 pass in CI** with deterministic diagnostics and zero flaky tests.
2. **Signature-lock tests** are auto-generated from `STDLIB_MVP_SIGNATURES.json` and passing.
3. **Audit schema** is validated with required fields per event type.
4. **Runtime** honors canonical FAILURE shape and delegation boundary.
5. **21 stdlib operations** are implemented and contract-tested.
6. **Checkpoint/resume** round-trips correctly with version/hash validation.
7. **At least 5 non-trivial programs** execute end-to-end through `al-cli run`.
8. **Release candidate** shipped with conformance matrix and known-limitations document.
