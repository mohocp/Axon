# Phase 12: Prove It Deeper — Formal Methods Frontier

**Principle:** Verification by Construction (formal methods frontier)
**Status:** Planned
**Depends on:** Phase 1 (real SMT solver), Phase 8 (advanced types)

---

## 1. Overview

Beyond SMT solving, this phase integrates theorem provers for the strongest possible guarantees. Critical agent operations get machine-checked proofs in Lean or Coq. The compiler itself is proven correct. Agents can synthesize proof tactics for their own operations. This is the ultimate expression of "verification by construction."

## 2. Requirements

### 2.1 Theorem Prover Integration

- **R1.1:** `PROVE_DEEP` verification level: submit proof obligation to Lean 4.
- **R1.2:** AgentLang operations exportable to Lean representation.
- **R1.3:** Lean proof certificates importable back into AgentLang verification system.
- **R1.4:** Proof certificate stored in audit trail with hash.
- **R1.5:** TLA+ integration for distributed protocol verification (Phase 9 protocols).

### 2.2 Certified Compilation

- **R2.1:** Operational semantics formalized in Lean/Coq.
- **R2.2:** Preservation theorem: compiler transformations preserve program semantics.
- **R2.3:** Progress theorem: well-typed programs do not get stuck.
- **R2.4:** Proof mechanically checked — not hand-waved.
- **R2.5:** Certified optimization passes: each optimization proven sound in Lean.

### 2.3 Automated Tactic Synthesis

- **R3.1:** Library of proof tactics for common verification patterns.
- **R3.2:** Tactic: arithmetic range proofs, capability non-escalation, schema compatibility.
- **R3.3:** Automated proof search: given a proof obligation, search tactic library for applicable tactics.
- **R3.4:** Agents can invoke tactic synthesis: `PROVE { strategy: AUTO }`.
- **R3.5:** Custom tactics: agents can register new tactics from successful proofs.

### 2.4 Proof-Carrying Code

- **R4.1:** Compiled packages carry proof certificates.
- **R4.2:** Any node can verify proof certificate independently.
- **R4.3:** Proof certificates are content-addressed (CAS hash).
- **R4.4:** Package installation verifies proof certificates before loading.

### 2.5 Interactive Proof Mode

- **R5.1:** `al prove <file.al> --interactive` starts interactive proof session.
- **R5.2:** User (or agent) guides proof by selecting tactics.
- **R5.3:** Proof state visible at each step.
- **R5.4:** Proof saved and replayable.

## 3. Architecture

### 3.1 New Crates

**`al-lean`:**
- AgentLang → Lean 4 translation
- Lean proof certificate parsing
- Lean process management (invoke lean4 binary)

**`al-tla`:**
- AgentLang distributed protocols → TLA+ specifications
- TLC model checker integration

**`al-tactics`:**
- Tactic library
- Proof search engine
- Tactic registration

### 3.2 Crate Changes

**`al-vc`:**
- New `ProveDeep` verification level
- Lean solver backend (alongside Z3)
- Proof certificate storage

**`al-diagnostics`:**
- Proof certificate in audit events
- Lean proof status in diagnostics

**`al-cli`:**
- `al prove <file> [--interactive]` subcommand
- `--prover lean|z3|auto` flag

**`al-pkg`:**
- Proof certificate embedding in packages
- Certificate verification on install

## 4. Testing

### 4.1 Unit Tests — Lean Integration (`al-lean`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_lean_translation_simple` | Arithmetic operation → valid Lean 4 definition |
| T1.2 | `test_lean_translation_contracts` | REQUIRE/ENSURE → Lean propositions |
| T1.3 | `test_lean_proof_valid` | Simple theorem proved in Lean → certificate returned |
| T1.4 | `test_lean_proof_invalid` | False theorem → Lean proof fails, error returned |
| T1.5 | `test_lean_certificate_roundtrip` | Certificate serialized and verified |
| T1.6 | `test_lean_timeout` | Complex proof with short timeout → Unknown |

### 4.2 Unit Tests — TLA+ Integration (`al-tla`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_tla_protocol_translation` | Distributed protocol → TLA+ spec |
| T2.2 | `test_tla_model_check_valid` | Correct protocol → model check passes |
| T2.3 | `test_tla_model_check_invalid` | Buggy protocol → counterexample found |

### 4.3 Unit Tests — Tactics (`al-tactics`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_tactic_arithmetic_range` | Range proof tactic succeeds for `x ∈ [0, 100]` |
| T3.2 | `test_tactic_capability_nonescalation` | Delegation non-escalation tactic |
| T3.3 | `test_tactic_search` | Auto-search finds applicable tactic |
| T3.4 | `test_tactic_registration` | Custom tactic registered and reusable |
| T3.5 | `test_no_tactic_found` | No applicable tactic → falls back to SMT |

### 4.4 Unit Tests — Proof-Carrying Code (`al-pkg`)

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_package_proof_embed` | Proof certificate embedded in package |
| T4.2 | `test_package_proof_verify` | Install verifies certificate |
| T4.3 | `test_package_proof_invalid` | Tampered certificate → install rejected |

### 4.5 Unit Tests — Interactive Proof (`al-lean`)

| ID | Test | Description |
|----|------|-------------|
| T5.1 | `test_interactive_proof_start` | Proof session starts with goal state |
| T5.2 | `test_interactive_tactic_apply` | Applying tactic advances proof state |
| T5.3 | `test_interactive_proof_complete` | All goals discharged → proof complete |
| T5.4 | `test_interactive_proof_save` | Completed proof saved and replayable |

### 4.6 Integration Tests

| ID | Test | Description |
|----|------|-------------|
| T6.1 | `test_lean_prove_e2e` | Full workflow: operation → Lean → proof → certificate → audit |
| T6.2 | `test_tactic_auto_e2e` | AUTO strategy finds proof without user guidance |
| T6.3 | `test_proof_carrying_install_e2e` | Publish with proof → install with verification |

### 4.7 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C65 | `conformance_lean_proof` | Lean proof certificate verifiable independently |
| C66 | `conformance_tactic_library` | Standard tactics prove common patterns |
| C67 | `conformance_proof_carrying_code` | Installed package proof certificates are valid |
| C68 | `conformance_preservation` | Compiler preserves semantics (tested, not fully mechanized in conformance) |

## 5. Acceptance Criteria

- [ ] AgentLang operations translatable to Lean 4 definitions
- [ ] Lean proofs produce verifiable certificates
- [ ] TLA+ integration verifies distributed protocols
- [ ] Tactic library covers arithmetic, capability, and schema patterns
- [ ] Automated proof search finds applicable tactics
- [ ] Proof-carrying packages verify on install
- [ ] Interactive proof mode allows guided proof construction
- [ ] All existing tests pass unchanged
- [ ] 4 new conformance tests (C65-C68) pass
