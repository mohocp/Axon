# Phase 2: Uncertainty is Not Optional — Probabilistic Types

**Principle:** Probabilistic Awareness
**Status:** Planned
**Depends on:** Phase 1 (Real Verification — for verified confidence thresholds)

---

## 1. Overview

Every LLM operation produces uncertain output. The MVP ignores this — operations return bare values with no confidence metadata. This phase implements `Probable[T]`, a first-class type that wraps any value with a confidence score, provenance chain, and degradation policy. The type checker enforces that `Probable[T]` cannot be used where `T` is expected without explicit threshold handling.

## 2. Requirements

### 2.1 Probable[T] Type

- **R1.1:** `Probable[T]` is a generic type wrapping value of type `T` with confidence `c ∈ [0.0, 1.0]`.
- **R1.2:** Construction: `SUCCESS(value) WITH CONFIDENCE ~0.85` or returned from LLM/probabilistic operations.
- **R1.3:** `Probable[T]` is distinct from `T` in the type system. Assignment of `Probable[T]` to a `T` binding is a type error.
- **R1.4:** Confidence query operator `?`: `result?` returns the confidence score as `Float64`.
- **R1.5:** Threshold extraction: `WHEN result? >= 0.9 -> USE result` extracts `T` from `Probable[T]`.

### 2.2 Confidence Composition

- **R2.1:** Pipeline composition: `c_pipeline = c_1 * c_2 * ... * c_n` (multiplication — conservative).
- **R2.2:** Agent trust attenuation: `c_effective = c_value * trust_level_agent`.
- **R2.3:** Fork/join aggregation: `c_merged = min(c_branch_1, c_branch_2, ...)` for ALL_COMPLETE.
- **R2.4:** Confidence floor: any value with `c < 0.0` is clamped to `0.0`; above `1.0` clamped to `1.0`.

### 2.3 Provenance Chains

- **R3.1:** Every `Probable[T]` carries a provenance record: `{ source_operation, source_agent, timestamp, input_hashes, model_id (optional) }`.
- **R3.2:** Provenance is append-only: each pipeline stage appends its provenance entry.
- **R3.3:** Provenance is serializable to JSON for audit trail integration.
- **R3.4:** Provenance is queryable at runtime: `result.provenance` returns the chain.

### 2.4 Degradation Policies

- **R4.1:** `Probable[T]` values can declare degradation: `ON_LOW_CONFIDENCE RETRY(3) -> ESCALATE`.
- **R4.2:** Degradation triggers when confidence drops below a declared threshold.
- **R4.3:** The type checker verifies that all degradation paths are handled (no unhandled low-confidence values).

### 2.5 Type Checker Enforcement

- **R5.1:** Using `Probable[T]` where `T` is expected without threshold check → compile error.
- **R5.2:** Pattern match on `Probable[T]` requires handling both high-confidence and low-confidence cases.
- **R5.3:** Confidence threshold in `WHEN result? >= threshold` is verified by the SMT solver when threshold is a constant.

## 3. Architecture

### 3.1 Crate Changes

**`al-ast`:**
- New `TypeExpr::Probable(Box<TypeExpr>)` variant
- New `Expression::ConfidenceQuery` for `?` operator (already exists as stub)
- New `Expression::WithConfidence { value, confidence }` for construction
- New `Expression::Provenance` for `.provenance` access

**`al-diagnostics`:**
- New `ErrorCode::ProbableWithoutThreshold` — using Probable[T] as T
- New `ErrorCode::UnhandledLowConfidence` — degradation path not handled

**`al-types`:**
- Probable[T] type registration and inference rules
- Confidence query typing: `Probable[T]?` → `Float64`
- Threshold extraction typing: `WHEN p? >= threshold -> USE p` yields `T`
- Pipeline confidence propagation: multiply confidences across stages

**`al-runtime`:**
- New `Value::Probable { value: Box<Value>, confidence: f64, provenance: Vec<ProvenanceEntry> }`
- Confidence query evaluation
- Threshold extraction evaluation
- Pipeline confidence multiplication
- Agent trust attenuation on DELEGATE results

**`al-lexer`:**
- `WITH` keyword (if not already present)
- `CONFIDENCE` keyword

### 3.2 Value Representation

```rust
pub struct ProvenanceEntry {
    pub operation: String,
    pub agent_id: Option<String>,
    pub timestamp: String,
    pub input_hashes: Vec<String>,
    pub model_id: Option<String>,
}

// In Value enum:
Probable {
    value: Box<Value>,
    confidence: f64,
    provenance: Vec<ProvenanceEntry>,
}
```

## 4. Testing

### 4.1 Unit Tests — Type Checker (`al-types`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_probable_type_distinct` | `Probable[Int64]` is not assignable to `Int64` |
| T1.2 | `test_probable_threshold_extract` | `WHEN p? >= 0.9 -> USE p` yields type `Int64` from `Probable[Int64]` |
| T1.3 | `test_probable_no_threshold_error` | Using `Probable[Str]` as `Str` argument → `ProbableWithoutThreshold` error |
| T1.4 | `test_probable_confidence_query_type` | `result?` on `Probable[T]` has type `Float64` |
| T1.5 | `test_probable_nested` | `Probable[Probable[T]]` is a valid type (double uncertainty) |
| T1.6 | `test_probable_in_list` | `List[Probable[Int64]]` is valid |
| T1.7 | `test_probable_pipeline_propagation` | Pipeline of two Probable-returning ops → confidence multiplied |
| T1.8 | `test_probable_pattern_match_required` | MATCH on Probable[T] must handle low-confidence case |

### 4.2 Unit Tests — Runtime (`al-runtime`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_probable_value_construction` | `SUCCESS(42) WITH CONFIDENCE ~0.85` creates Probable with confidence 0.85 |
| T2.2 | `test_probable_confidence_query` | `result?` returns 0.85 for above value |
| T2.3 | `test_probable_threshold_pass` | `WHEN result? >= 0.8 -> USE result` extracts 42 |
| T2.4 | `test_probable_threshold_fail` | `WHEN result? >= 0.9 -> USE result` falls to OTHERWISE |
| T2.5 | `test_probable_pipeline_confidence` | Two operations with c=0.9 each → pipeline result c=0.81 |
| T2.6 | `test_probable_trust_attenuation` | Agent trust 0.8 * value confidence 0.9 → effective 0.72 |
| T2.7 | `test_probable_provenance_chain` | Two pipeline stages → provenance has two entries |
| T2.8 | `test_probable_provenance_serialization` | Provenance serializes to JSON correctly |
| T2.9 | `test_probable_fork_join_confidence` | Fork with 3 branches → merged confidence = min(branches) |
| T2.10 | `test_probable_confidence_clamp` | Confidence > 1.0 clamped to 1.0; < 0.0 clamped to 0.0 |

### 4.3 Unit Tests — Lexer/Parser (`al-lexer`, `al-parser`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_lex_confidence_literal` | `~0.85` lexes as Confidence token |
| T3.2 | `test_parse_with_confidence` | `SUCCESS(x) WITH CONFIDENCE ~0.9` parses correctly |
| T3.3 | `test_parse_confidence_query` | `result?` parses as ConfidenceQuery expression |
| T3.4 | `test_parse_probable_type` | `Probable[Int64]` parses as TypeExpr |

### 4.4 Integration Tests (`al-cli`)

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_probable_full_pipeline` | Program constructs Probable, checks threshold, extracts value → correct output |
| T4.2 | `test_probable_type_error_output` | Program using Probable[T] as T → error message mentions confidence threshold |
| T4.3 | `test_probable_audit_provenance` | `--format jsonl` includes provenance in audit events |

### 4.5 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C25 | `conformance_probable_type_safety` | Probable[T] cannot be used as T without threshold |
| C26 | `conformance_confidence_propagation` | Pipeline confidence multiplication is correct |
| C27 | `conformance_provenance_chain` | Provenance chain grows through pipeline stages |
| C28 | `conformance_trust_attenuation` | Agent trust level attenuates delegation result confidence |

## 5. Acceptance Criteria

- [ ] `Probable[T]` is a first-class type recognized by lexer, parser, type checker, and runtime
- [ ] Using `Probable[T]` where `T` is expected without threshold check is a compile-time error
- [ ] `result?` returns confidence score as `Float64`
- [ ] Pipeline confidence multiplies across stages
- [ ] Agent trust attenuates confidence on delegation results
- [ ] Provenance chain is append-only and serializable to JSON
- [ ] All existing 484+ tests pass unchanged
- [ ] 4 new conformance tests (C25-C28) pass
