# Rust Readiness Audit: AgentLang MVP v0.1

Date: 2026-02-22
Scope audited:
- `AgentLang_Specification_v1.0.md`
- `formal_semantics.md`
- `stdlib_spec.md`
- `MVP_PROFILE.md`
- `GRAMMAR_MVP.ebnf`

## Executive Verdict

**Status: NOT READY for Phase 1 implementation yet.**

The current artifacts are close, but there are **normative contradictions and underspecified boundaries** that will force arbitrary implementation decisions in Rust. These are fixable with a final patch pass.

## 1. Parser Readiness (`GRAMMAR_MVP.ebnf`)

### Verdict

- **Can be implemented in Rust**, but **not yet as a clean deterministic MVP grammar spec**.
- As written, it is **not LL(1)** and is only **conditionally LALR(1)** (with conflict resolution and lexer conventions that are not specified).
- It is acceptable for PEG (`pest`) with ordered choice, but the language contract still needs tightening for consistent compiler behavior.

### Blocking issues

1. **`ASSERT` is in MVP feature freeze but absent from grammar statements.**
   - Included in MVP: `MVP_PROFILE.md:23`
   - Missing in grammar statement set: `GRAMMAR_MVP.ebnf:53-62`

2. **Operation output syntax mismatch with normative examples.**
   - Grammar expects `OUTPUT type_expr`: `GRAMMAR_MVP.ebnf:38`
   - Spec uses named output (`OUTPUT discount: Float64 ...`): `AgentLang_Specification_v1.0.md:325`

3. **Known ambiguous decision points for LALR/LL frontends are undocumented.**
   - `assign_stmt` vs `expr_stmt` (`identifier`-prefix): `GRAMMAR_MVP.ebnf:67-68`
   - `identifier` pattern vs `constructor_pattern`: `GRAMMAR_MVP.ebnf:92,96`
   - Requires explicit parser strategy (lookahead/precedence) in spec.

4. **Terminator model (`; | NEWLINE`) is underspecified for lexer mode transitions.**
   - `GRAMMAR_MVP.ebnf:188`
   - No normative newline-emission rules in audited set.

5. **Surface grammar in v1.0 spec is not aligned with MVP grammar and contains unresolved nonterminals.**
   - `source_clause`, `operation_ref`, `join_strategy`, `retry_opts`, etc. appear but are undefined: `AgentLang_Specification_v1.0.md:1094-1135`
   - This causes dual, conflicting parser targets.

### Required parser patch

- Make `GRAMMAR_MVP.ebnf` the single normative parser source for MVP.
- Add `assert_stmt`:
  - `assert_stmt = "ASSERT" expression terminator ;`
  - Include in `statement` alternatives.
- Normalize output clause grammar to match canonical form (pick one):
  - `OUTPUT type_expr` **or** `OUTPUT identifier ":" type_expr`; then update all docs/examples.
- Add normative parser notes:
  - `assign_stmt` wins over `expr_stmt` when lookahead sees `=` after identifier.
  - `constructor_pattern` selected when identifier is followed by `(`, else identifier pattern.
- Define newline lexing rules for `NEWLINE` token emission/suppression.
- Mark `AgentLang_Specification_v1.0.md` EBNF section as non-normative or replace it with a pointer to `GRAMMAR_MVP.ebnf`.

## 2. Type System + Refinements/Dependent Types in Rust

### Verdict

- **Modeling is feasible in Rust**: AST/HIR types + refinement obligations + solver-backed VC checks are implementable.
- **Major blocker:** SMT boundary behavior is not fully normative.

### What is implementable now

- Refinement encoding via obligations (`Ω`) and VC generation: `formal_semantics.md:373-397`
- `Probable[T]` no-implicit-erasure discipline: `formal_semantics.md:122-128,346`
- Decidable fragment constraints are clear enough as a starting set: `formal_semantics.md:375-380`

### Blocking issues

1. **`unknown` solver outcome policy is undefined operationally.**
   - `formal_semantics.md:391,400,596`
   - Current text says “require ASSERT/ASSUME or runtime proof level” but gives no deterministic compile/runtime rule.

2. **`Result`/`FAILURE` arity is inconsistent across artifacts.**
   - MVP canonical 3-arity: `MVP_PROFILE.md:52-64`
   - v1.0 spec still shows 2-arity type: `AgentLang_Specification_v1.0.md:278`
   - formal values/step text use 2-arity forms (`failure(ε,m)`, `failure(code,msg)`): `formal_semantics.md:51,185,428`

3. **Capability namespace and aliases are inconsistent with grammar/token model.**
   - Canonical IDs in MVP: `MVP_PROFILE.md:68-92`
   - Deprecated aliases include multi-word forms (`"read capability"` etc.): `MVP_PROFILE.md:95-114`
   - Grammar capability atoms are identifiers only: `GRAMMAR_MVP.ebnf:148`

### Required type-system patch

Define a normative compiler-solver interface section (in `formal_semantics.md` or `MVP_PROFILE.md`) with:
- Solver API result enum: `Valid | Invalid(counterexample) | Unknown(reason)`.
- Deterministic policy for `Unknown` by context:
  - In `REQUIRE/ENSURE/INVARIANT`: compile error unless explicitly wrapped by `ASSERT(...)` or declared `PROVE_RUNTIME`.
  - In type annotation refinements: either reject or auto-insert runtime check with required audit record format.
- Fixed SMT theory subset for MVP (LIA/LRA + finite enums + acyclic field equalities only).
- Timeout budget + cache key rules for incremental solving.

Also:
- Standardize `Result[T] = SUCCESS(T) | FAILURE(ErrorCode, Str, FailureDetails)` across **all** docs.
- Define capability alias normalization at semantic layer (not lexer), and restrict accepted alias forms to parser-representable syntax.

## 3. Dataflow Execution Readiness (Rust async/thread-pool mapping)

### Verdict

- DAG execution can map to Rust (`tokio` task graph or work-stealing thread pool), but **runtime semantics are incomplete for deterministic MVP behavior**.

### Defined well enough

- DAG readiness predicate and scheduler permission: `formal_semantics.md:469-480`
- `ALL_COMPLETE` join exists in grammar: `GRAMMAR_MVP.ebnf:131`
- MVP excludes other join modes: `MVP_PROFILE.md:21,31`

### Blocking issues

1. **Semantics document still specifies non-MVP join modes.**
   - `BEST_EFFORT`, `PARTIAL(min=k)`: `formal_semantics.md:493-495`
   - Contradicts MVP exclusion: `MVP_PROFILE.md:31`

2. **Checkpoint consistency model is unspecified.**
   - Open ambiguity: `formal_semantics.md:602`
   - Needed to implement `CHECKPOINT/RESUME` safely under concurrent tasks.

3. **Lock ordering source is unspecified.**
   - A global order is assumed: `formal_semantics.md:501-505`
   - But source/declaration of that order is unresolved: `formal_semantics.md:599`

4. **Delegation capability execution boundary needs one canonical rule across docs.**
   - MVP states callee capabilities: `MVP_PROFILE.md:188-191`
   - Formal semantics flags ambiguity in source set: `formal_semantics.md:598`

### Required dataflow/runtime patch

- In MVP profile, explicitly restrict operational semantics to `JOIN strategy: ALL_COMPLETE` and remove/mark other strategies non-MVP in `formal_semantics.md`.
- Add checkpoint consistency contract:
  - Snapshot level (`TASK`-scope atomic snapshot minimum),
  - Visibility guarantees across `LOCAL/TASK/AGENT/SHARED/GLOBAL`,
  - Resume replay/idempotency rules.
- Define canonical lock order source (compile-time declaration table or deterministic runtime canonicalization function).
- Add explicit delegation execution rule to formal semantics matching MVP (`callee caps only`, caller must hold `DELEGATE`).

## 4. Missing Types/Constructs in MVP Freeze

### Blocking inconsistencies

1. **MVP includes `ASSERT`, grammar does not.**
   - `MVP_PROFILE.md:23` vs `GRAMMAR_MVP.ebnf:53-62`

2. **MVP excludes many stdlib modules/ops, but `stdlib_spec.md` remains fully normative for them.**
   - MVP exclusions: `MVP_PROFILE.md:129-147`
   - Still specified as normative contracts: e.g., `stdlib_spec.md:532-777`

3. **MVP requires `REGEX`/`TOKENIZE` concrete return types; stdlib still uses `Any`/`List[Any]`.**
   - Requirement: `MVP_PROFILE.md:183-184`
   - Current signatures: `stdlib_spec.md:255-267`

4. **Main spec examples use excluded or ungrammatical constructs in MVP context without profile gating.**
   - `BEST_EFFORT/PARTIAL`: `AgentLang_Specification_v1.0.md:759-762`
   - Channel/reactive syntax (`EMIT TO`, `OBSERVE`): `AgentLang_Specification_v1.0.md:523,530`

### Required MVP freeze patch

- Split `stdlib_spec.md` into:
  - `stdlib_spec_mvp.md` (normative for MVP only),
  - full spec file marked non-MVP/forward-looking.
- Update signatures to MVP-required concrete types:
  - `REGEX -> RegexResult`
  - `TOKENIZE -> TokenizeResult`
- Add a top-level normative rule in `AgentLang_Specification_v1.0.md` and `stdlib_spec.md`:
  - “For MVP compiler/runtime conformance, `MVP_PROFILE.md` overrides all broader v1.0 examples/features.”
- Mark non-MVP syntax examples with explicit profile label (e.g., `vNext`, `non-MVP`).

## Rust Implementation Impact (if patched)

After the above patches, Rust implementation can proceed cleanly with:
- `lalrpop` (or PEG `pest`) parser from a single normative grammar,
- HIR + constraint engine + SMT boundary (Z3 via `z3` crate or SMT-LIB subprocess),
- DAG runtime on `tokio` or a fixed thread pool with deterministic join/checkpoint policies,
- Capability and failure model encoded as typed enums (`CapabilityId`, `FailureCode`, `FailureDetails`).

## Final Go/No-Go

**No-Go** until the blockers above are patched.

Minimum gate to start Phase 1 coding:
1. Unify grammar and MVP feature set (`ASSERT`, output clause, parser conflict rules, newline rules).
2. Unify `FAILURE`/`Result` shape across all artifacts.
3. Freeze SMT `Unknown` handling and runtime-check fallback semantics.
4. Remove or profile-gate non-MVP stdlib/runtime semantics from normative MVP path.

