# Phase 1: Trust the Proof, Not the Agent — Real Verification

**Principle:** Verification by Construction
**Status:** Planned
**Depends on:** Nothing (foundation for all subsequent phases)

---

## 1. Overview

The MVP stub solver returns `Unknown` for all non-trivial verification conditions, falling back to runtime assertions. This phase replaces the stub with a real SMT solver (Z3) so that `REQUIRE`, `ENSURE`, `INVARIANT`, and `ASSERT` clauses produce actual compile-time proofs.

Without this, the language's core promise — "programs carry their own correctness proofs" — is aspirational, not real.

## 2. Requirements

### 2.1 SMT Solver Backend

- **R1.1:** Integrate Z3 as the default SMT solver backend via the `z3` Rust crate (z3-sys FFI bindings).
- **R1.2:** Define a `Solver` trait in `al-vc` that abstracts over solver backends, preserving the ability to swap solvers (CVC5, custom).
- **R1.3:** The `StubSolver` remains available as a fallback and for testing environments without Z3 installed.
- **R1.4:** Solver selection is configurable: `--solver z3` (default), `--solver stub` (legacy).

### 2.2 VC-to-SMT Translation

- **R2.1:** Translate `REQUIRE` preconditions into SMT assertions. A `REQUIRE x GT 0` on an `Int64` input generates `(assert (> x 0))` as a premise.
- **R2.2:** Translate `ENSURE` postconditions into SMT proof goals. An `ENSURE result GT x` generates `(assert (not (> result x)))` and checks for unsatisfiability.
- **R2.3:** Translate `INVARIANT` clauses into loop induction obligations:
  - Base case: invariant holds at loop entry
  - Inductive step: if invariant holds before iteration, it holds after
- **R2.4:** Translate `ASSERT` expressions into SMT check-sat queries.
- **R2.5:** Support arithmetic constraints: `GT`, `GTE`, `LT`, `LTE`, `EQ`, `NEQ` on `Int64` and `Float64`.
- **R2.6:** Support boolean constraints: `AND`, `OR`, `NOT` combinations.
- **R2.7:** Support range constraints: `Int64 :: range(a, b)` translates to `(and (>= x a) (<= x b))`.

### 2.3 Result Handling

- **R3.1:** `Valid` — the condition is proven. No runtime check needed.
- **R3.2:** `Invalid { counterexample }` — the condition is provably false. Emit a compile-time error with the counterexample.
- **R3.3:** `Unknown { reason }` — the solver cannot decide. Inject a synthetic runtime `ASSERT` (existing MVP behavior preserved).
- **R3.4:** Solver timeout is configurable (default: 5 seconds per VC). Timeout produces `Unknown`.

### 2.4 Per-Iteration Loop Invariant Checking

- **R4.1:** `INVARIANT` inside a `LOOP` generates two VCs:
  - VC-base: invariant holds when loop is entered (given REQUIRE preconditions)
  - VC-step: if invariant holds at start of iteration, it holds at end of iteration
- **R4.2:** Both VCs are submitted to the solver independently.
- **R4.3:** If VC-base is `Invalid`, emit error: "Loop invariant does not hold at entry."
- **R4.4:** If VC-step is `Invalid`, emit error: "Loop invariant not preserved by iteration."

### 2.5 Pipeline Type Propagation Constraints

- **R5.1:** For pipeline chains `A -> B -> C`, the solver verifies that the output type of each stage satisfies the `REQUIRE` of the next stage.
- **R5.2:** Pipeline VC failures identify the specific stage transition that violates the contract.

## 3. Architecture

### 3.1 Crate Changes

**`al-vc` (primary changes):**
- New `Solver` trait: `fn solve(&self, vc: &VerificationCondition) -> VcResult`
- New `Z3Solver` struct implementing `Solver`
- New `SmtTranslator` module: converts `VerificationCondition` → Z3 AST
- `StubSolver` retains existing behavior, now implements `Solver` trait
- New `SolverConfig` with timeout, backend selection

**`al-types` (minor changes):**
- Type checker accepts a `Box<dyn Solver>` instead of hardcoded `StubSolver`
- Pass solver selection from CLI flags through to type checker

**`al-cli` (minor changes):**
- New `--solver` flag: `z3` | `stub` (default: `z3`)
- New `--solver-timeout` flag (default: `5000` ms)

### 3.2 SMT Translation Rules

```
AgentLang Expression     →  SMT-LIB2
─────────────────────────────────────
x GT y                   →  (> x y)
x GTE y                  →  (>= x y)
x LT y                   →  (< x y)
x LTE y                  →  (<= x y)
x EQ y                   →  (= x y)
x NEQ y                  →  (not (= x y))
a AND b                  →  (and a b)
a OR b                   →  (or a b)
NOT a                    →  (not a)
x + y                    →  (+ x y)
x - y                    →  (- x y)
x * y                    →  (* x y)
x / y                    →  (div x y)  [Int64]
x MOD y                  →  (mod x y)
Int64                    →  Int sort
Float64                  →  Real sort
Bool                     →  Bool sort
Str                      →  String sort
range(a, b)              →  (and (>= x a) (<= x b))
```

### 3.3 Solver Trait

```rust
pub trait Solver: Send + Sync {
    fn solve(&self, vc: &VerificationCondition, config: &SolverConfig) -> VcResult;
    fn name(&self) -> &str;
}

pub struct SolverConfig {
    pub timeout_ms: u64,
}
```

## 4. Implementation Plan

1. Add `z3` crate dependency to `al-vc` (feature-gated: `feature = "z3"`)
2. Define `Solver` trait, make `StubSolver` implement it
3. Implement `SmtTranslator`: VC → Z3 context/assertions
4. Implement `Z3Solver` wrapping the translator
5. Update `TypeChecker` to accept `Box<dyn Solver>`
6. Update `al-cli` with `--solver` and `--solver-timeout` flags
7. Implement per-iteration loop invariant VC generation
8. Implement pipeline constraint VC generation
9. Integration tests with real Z3 proofs

## 5. Testing

### 5.1 Unit Tests (`al-vc`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_solver_trait_stub` | `StubSolver` implements `Solver` trait, returns `Unknown` as before |
| T1.2 | `test_z3_simple_valid` | `REQUIRE x GT 0` with input `x: Int64` where `x = 5` → `Valid` |
| T1.3 | `test_z3_simple_invalid` | `REQUIRE x GT 0` is provably violated when no constraint on x → `Invalid` with counterexample |
| T1.4 | `test_z3_ensure_valid` | `REQUIRE x GT 0`, body `STORE result = x + 1`, `ENSURE result GT x` → `Valid` |
| T1.5 | `test_z3_ensure_invalid` | `REQUIRE x GT 0`, body `STORE result = x - 1`, `ENSURE result GT x` → `Invalid` |
| T1.6 | `test_z3_arithmetic_ops` | All arithmetic operators translate correctly (+, -, *, /, MOD) |
| T1.7 | `test_z3_boolean_ops` | AND, OR, NOT combinations translate correctly |
| T1.8 | `test_z3_comparison_ops` | All comparison operators: GT, GTE, LT, LTE, EQ, NEQ |
| T1.9 | `test_z3_range_constraint` | `Int64 :: range(0, 100)` produces valid range assertion |
| T1.10 | `test_z3_timeout` | Complex VC with 1ms timeout → `Unknown { reason: "timeout" }` |
| T1.11 | `test_z3_multiple_requires` | Multiple REQUIRE clauses combine as conjunction |
| T1.12 | `test_z3_nested_expressions` | `REQUIRE (x GT 0) AND (x LT 100)` → valid range proof |

### 5.2 Loop Invariant Tests (`al-vc`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_loop_invariant_base_valid` | Loop with `INVARIANT counter GTE 0` where `STORE counter = 0` → base case `Valid` |
| T2.2 | `test_loop_invariant_base_invalid` | Loop with `INVARIANT counter GT 0` where `STORE counter = 0` → base case `Invalid` |
| T2.3 | `test_loop_invariant_step_valid` | `INVARIANT counter GTE 0` with `counter = counter + 1` → step `Valid` |
| T2.4 | `test_loop_invariant_step_invalid` | `INVARIANT counter GTE 0` with `counter = counter - 2` → step `Invalid` (may go negative) |
| T2.5 | `test_loop_invariant_both_vcs` | Both base and step VCs generated for a single INVARIANT |

### 5.3 Pipeline Constraint Tests (`al-vc`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_pipeline_constraint_valid` | Pipeline `A -> B` where A's output satisfies B's REQUIRE → `Valid` |
| T3.2 | `test_pipeline_constraint_invalid` | Pipeline `A -> B` where A's output type conflicts with B's REQUIRE → `Invalid` with stage identification |
| T3.3 | `test_pipeline_multi_stage` | 3-stage pipeline: constraints checked at each transition |

### 5.4 Integration Tests (`al-cli`)

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_cli_solver_flag_z3` | `al check file.al --solver z3` uses Z3 backend |
| T4.2 | `test_cli_solver_flag_stub` | `al check file.al --solver stub` uses stub backend |
| T4.3 | `test_cli_solver_timeout` | `al check file.al --solver-timeout 100` respects timeout |
| T4.4 | `test_cli_proven_program` | Full program with REQUIRE/ENSURE that Z3 proves → exit 0, "proved" in output |
| T4.5 | `test_cli_disproven_program` | Program with invalid ENSURE → exit 1, counterexample in output |
| T4.6 | `test_cli_json_vc_results` | `--format json` includes VC results with status and counterexamples |

### 5.5 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C21 | `conformance_vc_proven` | A conformance-level test: program with arithmetic REQUIRE/ENSURE is statically proven |
| C22 | `conformance_vc_disproven` | A conformance-level test: invalid postcondition produces compile-time error with counterexample |
| C23 | `conformance_loop_invariant` | Conformance test: loop invariant induction proof succeeds for valid invariant |
| C24 | `conformance_vc_unknown_fallback` | Complex VC that times out → synthetic runtime ASSERT injected (backward compat with MVP) |

### 5.6 Property-Based Tests

| ID | Test | Description |
|----|------|-------------|
| P1.1 | `proptest_smt_translation_roundtrip` | Random arithmetic expressions translate to SMT and back without semantic change |
| P1.2 | `proptest_valid_range_constraints` | Random `range(a, b)` where `a <= b` always produces satisfiable SMT |
| P1.3 | `proptest_stub_z3_agreement` | For trivially valid/invalid VCs, stub and Z3 agree on result |

## 6. Acceptance Criteria

- [ ] `REQUIRE x GT 0` with constant `x = 5` produces `Valid` at compile time (no runtime check)
- [ ] `ENSURE result GT x` with body `result = x - 1` produces `Invalid` at compile time with counterexample
- [ ] Loop `INVARIANT counter GTE 0` with `counter = counter + 1` produces two VCs, both `Valid`
- [ ] All 484 existing tests continue to pass with `--solver stub`
- [ ] At least 3 new conformance tests (C21-C24) pass with `--solver z3`
- [ ] `--solver z3` is the default; `--solver stub` available for environments without Z3
- [ ] Solver timeout produces `Unknown` and injects runtime ASSERT (existing behavior preserved)
