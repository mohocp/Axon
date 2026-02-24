# AgentLang MVP — Round 4 Execution Advisory

**Date:** 2026-02-24
**Author:** Claude (execution review)
**Scope:** Risk controls and sequencing for Round 4 (VC pipeline & capability enforcement)
**Baseline:** Round 3 complete — 267 tests, 13 crates, 7 type-checker passes, zero warnings

---

## 1. Round 4 Objective

Wire the verification-condition pipeline (generate -> solve -> Unknown rewrite) and
enforce the delegation boundary statically. Targets spec criteria **C5** and **C6**.

---

## 2. Codebase Entry-Point Assessment

| Component | State | Round 4 Relevance |
|-----------|-------|--------------------|
| `al-vc` | Stub: `VerificationCondition`, `MvpSolver` (always Unknown), helpers | Must grow VC generation + configurable solver |
| `al-capabilities` | **Complete**: 22 caps, `check_delegation()`, diagnostics | Ready — type checker must call it |
| `al-types` | 7 passes; no expr inference, no ENSURE/INVARIANT handling | Must add delegation pass + VC-aware pass |
| `al-hir` | Structural lowering only; `ty`/`required_caps` always empty | Must carry contract clauses + accept injected assertions |
| `al-diagnostics` | `VcInvalid`, `CapabilityDenied`, `AssertionFailed` codes exist | Ready |
| `al-runtime` | `execute_assert`, `insert_runtime_assert` implemented | Ready for downstream Round 5 consumption |
| `al-conformance` | 27 integration tests; C5/C6 partially covered | Must add dedicated delegation + VC end-to-end fixtures |

---

## 3. Structural Pre-Requisites (Do First)

These are not deliverables themselves but **gates** that block multiple deliverables.

### 3.1 HIR Must Carry Contract Clauses

**Problem:** `HirDeclaration::Operation` has no `requires`, `ensures`, or `invariants`
fields. The parser stores them in the AST, but `lower_declaration` discards them.
Every deliverable in 4.1-4.3 is blocked.

**Fix:** Add `requires: Vec<HirExpr>`, `ensures: Vec<HirExpr>`, `invariants: Vec<HirExpr>`
to `HirDeclaration::Operation`. Update `lower_declaration` to lower them. Existing
tests remain green because these fields are additive.

**Risk:** Low. Purely additive struct change.

### 3.2 HIR Must Support Synthetic ASSERT Injection

**Problem:** After lowering, the HIR body is immutable. Deliverable 4.5 requires
inserting new `HirStatement::Assert` nodes with `meta.synthetic = true`.

**Fix:** Add a post-lowering HIR mutation function (e.g., `inject_synthetic_assert`)
that appends/prepends synthetic asserts to an operation's body. The
`HirMeta::synthetic()` constructor already exists and is tested.

**Risk:** Low. `HirStatement::Assert` variant already exists.

### 3.3 `al-vc` Must Actually Use Its `al-hir` Dependency

**Problem:** `al-vc/Cargo.toml` declares `al-hir` as a dependency, but the code
never imports from it. VC generation needs to read HIR contract clauses.

**Fix:** Wire the import. No Cargo.toml change needed.

**Risk:** None.

---

## 4. Recommended Execution Sequence

### Phase A — Structural Foundation (pre-requisites 3.1-3.3)

| Step | Deliverable | Depends On | Est. Risk |
|------|-------------|------------|-----------|
| A1 | Extend `HirDeclaration::Operation` with contract fields | — | Low |
| A2 | Update `lower_declaration` to lower REQUIRE/ENSURE/INVARIANT | A1 | Low |
| A3 | Add `inject_synthetic_assert` function to `al-hir` | — | Low |

**Gate check:** `cargo test` — 267+ tests pass, zero regressions.

### Phase B — VC Generation (deliverables 4.1-4.3)

| Step | Deliverable | Depends On | Est. Risk |
|------|-------------|------------|-----------|
| B1 | 4.1: `generate_vcs_from_require_ensure()` in `al-vc` | A2 | Medium |
| B2 | 4.2: `generate_vcs_from_assert()` in `al-vc` | A2 | Low |
| B3 | 4.3: `generate_vcs_from_invariant()` in `al-vc` | A2 | Medium |

**Design note:** MVP VCs should be limited to simple boolean/comparison expressions.
Complex expressions (nested calls, member chains) should immediately produce
`Unknown` rather than attempting deep analysis. This bounds complexity.

**Gate check:** >= 8 new unit tests in `al-vc`. Each VC has a unique `vc_id`.

### Phase C — Solver + Rewrite (deliverables 4.4-4.6)

| Step | Deliverable | Depends On | Est. Risk |
|------|-------------|------------|-----------|
| C1 | 4.4: Configurable `MvpSolver` (test mode: programmable results) | B1 | Low |
| C2 | 4.5: Unknown -> synthetic ASSERT injection into HIR | C1, A3 | Medium |
| C3 | 4.6: Invalid -> `VC_INVALID` compile error emission | C1 | Low |

**Risk control:** The solver must default to `Unknown` in production mode. Only test
configurations may return `Valid` or `Invalid`. This prevents false confidence.

**Gate check:** >= 3 Unknown-rewrite tests. Round-trip: REQUIRE clause -> VC ->
Unknown -> synthetic `HirStatement::Assert` with `meta.synthetic = true`.

### Phase D — Delegation Enforcement (deliverables 4.7-4.8)

| Step | Deliverable | Depends On | Est. Risk |
|------|-------------|------------|-----------|
| D1 | 4.7: Static delegation check (new type-checker pass) | — | Low |
| D2 | 4.8: HIR `required_caps` population | D1 | Low |

**Note:** Phase D is independent of Phases B-C and can be developed in parallel.
`al-capabilities::check_delegation()` is already complete and tested. The work is
purely wiring: iterate `HirStatement::Delegate` nodes, cross-reference the enclosing
agent's `capabilities` set, emit `CAPABILITY_DENIED` on violation.

**Gate check:** >= 5 delegation tests. Positive (caller has DELEGATE) and negative
(caller lacks DELEGATE -> error).

### Phase E — REQUIRE Scope Expansion + Integration (deliverable 4.9)

| Step | Deliverable | Depends On | Est. Risk |
|------|-------------|------------|-----------|
| E1 | 4.9: Expand Pass 5 to include STORE bindings in REQUIRE scope | — | Low |
| E2 | Conformance fixture updates for C5 and C6 | C2, D1 | Low |
| E3 | Exit gate validation | E2 | — |

---

## 5. Risk Register

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|-----------|--------|------------|
| R1 | VC generation for complex expressions exceeds MVP scope | Medium | High | Restrict to simple boolean/comparison. Flag everything else as `Unknown`. Define a whitelist of supported VC expression forms. |
| R2 | HIR contract fields break downstream crate compilation | Low | Medium | Fields are additive (`Vec<HirExpr>`, default empty). Run `cargo check --all` after A1. |
| R3 | Configurable solver leaks test-mode results into production | Low | High | Use a builder pattern or feature flag. Default constructor always returns `Unknown`. Test mode requires explicit opt-in via `MvpSolver::with_results(map)`. |
| R4 | Synthetic ASSERT injection changes HIR node count, breaking span-dependent logic | Low | Medium | Synthetic nodes carry `HirMeta::synthetic()` span (reusing the REQUIRE/ENSURE span). No downstream code currently relies on HIR node count. |
| R5 | INVARIANT VC requires loop-iteration semantics not yet modeled | Medium | Medium | MVP: generate a single VC per INVARIANT, checked once at operation level (not per-iteration). Document the simplification. Defer loop-boundary checking to post-MVP. |
| R6 | Conformance fixture numbering misaligns with spec criteria | Low | Low | Current fixture indices (C1-C14 in code) don't map 1:1 to spec C1-C10. When adding C5/C6 fixtures, use explicit names (e.g., `c5_delegation_boundary_static`) not just indices. |
| R7 | Round 3 left HIR `ty` unpopulated — Round 4 VC generation may need type info | Medium | Medium | VC generation should operate on untyped HIR for now. Type-aware VCs are post-MVP. If type info is needed for a specific VC, emit `Unknown` instead. |

---

## 6. Exit Gate Checklist

Per the roadmap, Round 4 is complete when:

- [ ] **C5 fully enforced:** Static check that DELEGATE callers hold `Capability::Delegate`. `CAPABILITY_DENIED` emitted on violation.
- [ ] **C6 end-to-end:** REQUIRE/ENSURE/ASSERT -> VC -> solve -> Unknown -> synthetic ASSERT injected into HIR. Runtime path: false assert -> `FAILURE(ASSERTION_FAILED, ...)` with `vc_id` + `solver_reason`.
- [ ] **C9 still passing:** Capability alias normalization regression-free.
- [ ] **New tests:** >= 15 VC tests, >= 5 delegation tests, >= 3 Unknown-rewrite tests.
- [ ] **All 267+ existing tests pass** (zero regressions).
- [ ] `cargo check --all` zero warnings.

---

## 7. What Round 4 Should NOT Do

To prevent scope creep:

- **No expression type inference.** That is Round 3 unfinished scope (3.1-3.6). Round 4 VC generation works on untyped expressions.
- **No real SMT solver.** The stub solver is the product. Z3/CVC5 is post-MVP.
- **No runtime execution changes.** `execute_assert` and `insert_runtime_assert` are already implemented. Round 4 is compiler-side only.
- **No stdlib implementation.** That is Round 6.
- **No HIR `ty` population.** That is a Round 3 carry-over, not Round 4 scope.

---

## 8. Parallelization Opportunity

```
Phase A (foundation)  ────>  Phase B (VC gen)  ────>  Phase C (solver/rewrite)
                                                              |
Phase D (delegation)  ──────────────────────────────────────> |
                                                              v
Phase E (scope expansion + integration)                 Exit Gate
```

Phases B-C and D are independent. A developer (or agent) working on delegation
enforcement need not wait for VC generation to complete, and vice versa. This
is the primary parallelization seam.

---

## 9. Key File Paths

| File | Role in Round 4 |
|------|-----------------|
| `crates/al-hir/src/lib.rs` | Add contract fields to Operation, synthetic injection fn |
| `crates/al-vc/src/lib.rs` | VC generation functions, configurable solver |
| `crates/al-types/src/lib.rs` | New delegation check pass (Pass 8), REQUIRE scope expansion |
| `crates/al-capabilities/src/lib.rs` | Already complete; called by new Pass 8 |
| `crates/al-conformance/src/lib.rs` | New C5/C6 fixtures |
| `crates/al-conformance/tests/conformance.rs` | New integration tests |
| `crates/al-diagnostics/src/lib.rs` | Already has all needed error/warning codes |

---

*End of advisory.*
