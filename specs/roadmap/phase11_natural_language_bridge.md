# Phase 11: Speak Both Languages — Natural Language Bridge

**Principle:** Agents as Primary Developers + Human Accountability
**Status:** Planned
**Depends on:** Phase 4 (LLM backends), Phase 8 (advanced type system)

---

## 1. Overview

The spec envisions bidirectional translation between natural language and AgentLang. Humans express intent in English; the system generates verified AgentLang programs. Agents explain their programs in human-readable form for audit. This bridges the gap between human intent and agent execution without sacrificing verification guarantees.

## 2. Requirements

### 2.1 Natural Language to AgentLang

- **R1.1:** `al generate "natural language description"` produces AgentLang source.
- **R1.2:** Generated code includes REQUIRE/ENSURE contracts synthesized from intent.
- **R1.3:** Generated code is type-checked and verified before presenting to user.
- **R1.4:** If verification fails, the system iterates (up to N attempts) with refined prompts.
- **R1.5:** Output includes confidence score for the generated program's alignment with intent.
- **R1.6:** User can accept, reject, or provide feedback for refinement.

### 2.2 AgentLang to Natural Language

- **R2.1:** `al explain <file.al>` produces human-readable explanation.
- **R2.2:** Operation summaries: what it does, what it requires, what it guarantees.
- **R2.3:** Pipeline narratives: step-by-step explanation of dataflow.
- **R2.4:** Audit trail explanations: why an agent took a specific action.
- **R2.5:** Verification explanations: what was proven and what wasn't.
- **R2.6:** Multiple detail levels: `--detail brief|standard|verbose`.

### 2.3 Multi-Model Agent Backends

- **R3.1:** Agent declaration can specify backend: `AGENT analyzer => BACKEND llm:claude-sonnet`.
- **R3.2:** Supported backend types: `llm` (language model), `rl` (reinforcement learning), `symbolic` (logic/planning).
- **R3.3:** All backends communicate through AgentLang types — unified type system.
- **R3.4:** Backend-specific configuration in agent properties.
- **R3.5:** Backend trait: `AgentBackend { fn execute(operation, inputs) -> Result<Value> }`.

### 2.4 Intent Specification Language

- **R4.1:** Structured intent format beyond free-form natural language.
- **R4.2:** Intent includes: goal, constraints, expected inputs/outputs, quality criteria.
- **R4.3:** Intent maps to REQUIRE/ENSURE contracts deterministically where possible.
- **R4.4:** Ambiguous intent → system asks clarifying questions.

## 3. Architecture

### 3.1 New Crates

**`al-nlbridge`:**
- NL → AgentLang translation engine
- AgentLang → NL explanation engine
- Intent parser
- Iterative refinement loop

**`al-backends-multi`:**
- Multi-model backend registry
- Backend trait and implementations
- Backend-specific configuration

### 3.2 Crate Changes

**`al-ast`:**
- `AgentProperty::Backend(BackendSpec)` — agent backend specification

**`al-runtime`:**
- Backend dispatch: route operation execution to agent's declared backend
- Backend registration at runtime startup

**`al-cli`:**
- `al generate "..."` subcommand
- `al explain <file>` subcommand
- `--detail brief|standard|verbose` flag for explain
- `--backend-config` flag for multi-model configuration

## 4. Testing

### 4.1 Unit Tests — NL to AgentLang (`al-nlbridge`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_simple_generation` | "add two numbers" → valid OPERATION with INPUT/OUTPUT/BODY |
| T1.2 | `test_generation_with_contracts` | "ensure result is positive" → ENSURE result GT 0 |
| T1.3 | `test_generation_type_checks` | Generated code passes type checker |
| T1.4 | `test_generation_iteration` | Invalid generation → retry with refined prompt |
| T1.5 | `test_generation_confidence` | Generated program has confidence score |
| T1.6 | `test_ambiguous_intent` | Vague intent → clarifying questions returned |

### 4.2 Unit Tests — AgentLang to NL (`al-nlbridge`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_explain_operation` | OPERATION → "This operation takes X and returns Y" |
| T2.2 | `test_explain_pipeline` | PIPELINE → step-by-step narrative |
| T2.3 | `test_explain_contracts` | REQUIRE/ENSURE → "Requires that X, guarantees that Y" |
| T2.4 | `test_explain_brief` | `--detail brief` → one-sentence summary |
| T2.5 | `test_explain_verbose` | `--detail verbose` → full explanation with type details |
| T2.6 | `test_explain_audit_event` | Audit event → "Agent X performed Y because Z" |

### 4.3 Unit Tests — Multi-Model Backends (`al-backends-multi`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_backend_llm` | LLM backend executes through LLM API |
| T3.2 | `test_backend_symbolic` | Symbolic backend executes through logic engine |
| T3.3 | `test_backend_dispatch` | Agent with `BACKEND llm:claude` routes to LLM |
| T3.4 | `test_backend_type_unification` | Different backends return compatible AgentLang types |
| T3.5 | `test_backend_fallback` | No backend specified → default (current runtime) |

### 4.4 Unit Tests — Parser (`al-parser`)

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_parse_backend_spec` | `BACKEND llm:claude-sonnet` in agent declaration |
| T4.2 | `test_parse_backend_config` | Backend with configuration properties |

### 4.5 Integration Tests

| ID | Test | Description |
|----|------|-------------|
| T5.1 | `test_generate_e2e` | NL description → generated program → executes correctly |
| T5.2 | `test_explain_e2e` | Program → explanation → covers all declarations |
| T5.3 | `test_multi_backend_e2e` | Program with agents using different backends |
| T5.4 | `test_roundtrip_e2e` | Generate → explain → regenerate preserves semantics |

### 4.6 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C61 | `conformance_nl_generation` | Generated code is valid AgentLang |
| C62 | `conformance_nl_explanation` | Explanation covers all declarations and contracts |
| C63 | `conformance_multi_backend` | Different backends produce compatible types |
| C64 | `conformance_backend_capability_gating` | Backend operations respect capability system |

## 5. Acceptance Criteria

- [ ] `al generate` produces valid, type-checked AgentLang from natural language
- [ ] Generated code includes synthesized REQUIRE/ENSURE contracts
- [ ] `al explain` produces human-readable explanations at multiple detail levels
- [ ] Multi-model backends dispatch to correct engine based on agent declaration
- [ ] All backends communicate through unified AgentLang type system
- [ ] Iterative refinement improves generation quality
- [ ] All existing tests pass unchanged
- [ ] 4 new conformance tests (C61-C64) pass
