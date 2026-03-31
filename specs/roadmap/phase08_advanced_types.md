# Phase 8: Types That Think Harder — Advanced Type System

**Principle:** Semantic Density + Verification by Construction (type-level)
**Status:** Planned
**Depends on:** Phase 1 (SMT solver for dependent type constraints)

---

## 1. Overview

The MVP has monomorphic types and simplified constraints. The spec envisions full parametric polymorphism, dependent types verified by SMT, explicit effect tracking, and schema evolution. This phase makes the type system expressive enough to catch entire categories of errors at compile time.

## 2. Requirements

### 2.1 Parametric Polymorphism

- **R1.1:** Generic operation definitions: `OPERATION Transform[T, U] => INPUT items: List[T] OUTPUT List[U]`.
- **R1.2:** Type parameter constraints: `T: Serializable`, `U: Numeric`.
- **R1.3:** Implicit type parameter resolution at call sites when unambiguous.
- **R1.4:** Explicit type application: `Transform[Int64, Str](items)`.
- **R1.5:** Type parameters in SCHEMA: `SCHEMA Pair[A, B] => { first: A, second: B }`.

### 2.2 Dependent Types (Decidable Fragment)

- **R2.1:** Value-indexed collections: `List[User, length: 10]` — list must have exactly 10 elements.
- **R2.2:** Range-constrained numerics: `Float64 :: range(0.0..1.0)` — value must be in range.
- **R2.3:** Tensor dimensions: `Tensor[Float32, 768]` — fixed-dimension tensor.
- **R2.4:** Constraints verified by SMT solver at compile time.
- **R2.5:** Runtime fallback: if solver returns Unknown, inject runtime assertion.
- **R2.6:** Constraint propagation through pipelines: output constraints derived from input constraints + operation semantics.

### 2.3 Effect System

- **R3.1:** Operations declare effects alongside types: `OPERATION Fetch => EFFECTS [API_CALL, FILE_READ]`.
- **R3.2:** If EFFECTS not declared, inferred from body statements (stdlib calls, DELEGATE, etc.).
- **R3.3:** Effect composition: pipeline effects = union of stage effects.
- **R3.4:** Compile-time check: operation effects must be subset of agent capabilities.
- **R3.5:** Effect polymorphism: generic operations propagate effects from type parameters.

### 2.4 Schema Evolution

- **R4.1:** Width subtyping: SCHEMA with additional fields is a subtype of SCHEMA without them.
- **R4.2:** Depth subtyping: narrowing a field's type is safe (e.g., `Int64` subtype of `Numeric`).
- **R4.3:** Optional fields: `SCHEMA User => { name: Str, nickname: Str? }` — `nickname` may be absent.
- **R4.4:** Schema compatibility check: `UserV2` backward-compatible with `UserV1` → compile-time proof.
- **R4.5:** Migration path: `MIGRATE UserV1 -> UserV2 => { nickname: NONE }` fills defaults.

### 2.5 Type Inference Improvements

- **R5.1:** Bidirectional type inference: expected types propagate inward.
- **R5.2:** MATCH arm type unification: all arms must return compatible types.
- **R5.3:** Pipeline type inference: output type of `A -> B` inferred from A's OUTPUT and B's INPUT/OUTPUT.
- **R5.4:** Closure/lambda type inference (if lambdas are added to language).

## 3. Architecture

### 3.1 Crate Changes

**`al-types`:**
- Generic type unification engine (Algorithm W / Hindley-Milner with extensions)
- Constraint solver integration with SMT for dependent types
- Effect inference and checking pass
- Schema subtyping and compatibility checker
- New passes: effect inference, generic instantiation, dependent constraint checking

**`al-ast`:**
- Type parameter syntax: `[T, U]` after declaration names
- Constraint syntax: `T: Trait`
- Effect declaration: `EFFECTS [...]`
- Optional field marker: `Type?`
- Migration declaration

**`al-lexer` / `al-parser`:**
- Type parameter parsing
- Constraint parsing
- EFFECTS keyword
- Optional type marker `?`
- MIGRATE keyword

**`al-vc`:**
- Dependent type constraints → SMT assertions
- Schema compatibility → SMT proof obligations

## 4. Testing

### 4.1 Unit Tests — Polymorphism (`al-types`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_generic_operation` | `OPERATION Id[T] => INPUT x: T OUTPUT T` type-checks |
| T1.2 | `test_generic_instantiation` | `Id[Int64](42)` instantiates T=Int64 |
| T1.3 | `test_generic_inference` | `Id(42)` infers T=Int64 from argument |
| T1.4 | `test_generic_constraint_met` | `T: Numeric` with `T=Int64` → valid |
| T1.5 | `test_generic_constraint_violated` | `T: Numeric` with `T=Str` → type error |
| T1.6 | `test_generic_schema` | `SCHEMA Pair[A, B]` instantiates correctly |
| T1.7 | `test_generic_nested` | `List[Pair[Int64, Str]]` resolves correctly |
| T1.8 | `test_generic_pipeline` | Generic operation in pipeline infers types from context |

### 4.2 Unit Tests — Dependent Types (`al-types`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_range_constraint_valid` | `Int64 :: range(0, 100)` with REQUIRE x GTE 0 AND x LTE 100 → valid |
| T2.2 | `test_range_constraint_invalid` | `Int64 :: range(0, 100)` with no constraint on input → Unknown/runtime assert |
| T2.3 | `test_list_length_constraint` | `List[T, length: 10]` verified at construction |
| T2.4 | `test_constraint_propagation` | FILTER on `List[T, length: 10]` → output length ≤ 10 |
| T2.5 | `test_tensor_dimension` | `Tensor[Float32, 768]` dimension checked |

### 4.3 Unit Tests — Effect System (`al-types`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_effect_declaration` | `EFFECTS [API_CALL]` parsed and checked |
| T3.2 | `test_effect_inference` | Operation calling HTTP.GET infers API_CALL effect |
| T3.3 | `test_effect_subset` | Agent with [API_CALL, DB_READ] can call operation with EFFECTS [API_CALL] |
| T3.4 | `test_effect_violation` | Agent without DB_WRITE can't call operation with EFFECTS [DB_WRITE] |
| T3.5 | `test_effect_composition` | Pipeline effects = union of stage effects |
| T3.6 | `test_effect_polymorphism` | Generic op effect depends on type parameter's effects |

### 4.4 Unit Tests — Schema Evolution (`al-types`)

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_width_subtyping` | Schema with extra field is subtype |
| T4.2 | `test_depth_subtyping` | Schema with narrower field type is subtype |
| T4.3 | `test_optional_field` | `Str?` field can be absent |
| T4.4 | `test_schema_compat_valid` | UserV2 with added optional field compatible with UserV1 |
| T4.5 | `test_schema_compat_invalid` | Removing required field → incompatible |
| T4.6 | `test_migration_fills_defaults` | MIGRATE fills default values for new fields |

### 4.5 Integration Tests

| ID | Test | Description |
|----|------|-------------|
| T5.1 | `test_generic_e2e` | Program with generic operations executes correctly |
| T5.2 | `test_dependent_e2e` | Range-constrained value verified at compile time |
| T5.3 | `test_effect_e2e` | Effect violation caught at compile time |
| T5.4 | `test_schema_evolution_e2e` | Schema migration executes correctly |

### 4.6 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C49 | `conformance_polymorphism` | Generic operations instantiate and execute correctly |
| C50 | `conformance_dependent_types` | Value constraints verified by SMT solver |
| C51 | `conformance_effect_system` | Effect violations caught at compile time |
| C52 | `conformance_schema_evolution` | Schema compatibility proven at compile time |

## 5. Acceptance Criteria

- [ ] Generic operations with type parameters are supported
- [ ] Type parameter constraints are enforced
- [ ] Dependent type constraints verified by SMT solver
- [ ] Effect system infers and checks operation effects
- [ ] Schema subtyping and compatibility checking works
- [ ] Optional fields are supported in SCHEMA
- [ ] All existing tests pass (monomorphic programs still work)
- [ ] 4 new conformance tests (C49-C52) pass
