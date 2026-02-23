# Codex Validation of Claude Review + Implementation Plan

## 1) Validation Outcome

Claude's review is directionally strong. The most important findings are valid and are real blockers for a conformant implementation. A smaller set are strategy-level enhancements that are useful but not required for an MVP.

Validation basis:
- `claude_full_review.md`
- `AgentLang_Specification_v1.0.md`
- `formal_semantics.md`
- `stdlib_spec.md`

## 2) Validated Findings by Implementation Impact

### A. Critical Blocks (must resolve before building a serious compiler/runtime)

1. **Spec mismatch: base stdlib promises vs actual stdlib coverage**
- Validated: base spec lists many operations/modules not specified in stdlib contracts (e.g., `db.graph`, `api.grpc`, `ACK/NACK/REPLAY`, `SUMMARIZE`, `SEARCH`, `PRIORITIZE`, `REPLAN`, `IMPROVE`, `LEARN`, etc.).
- Evidence: base spec section 8 tables vs stdlib coverage index (58 ops only).
- Impact: implementation target is ambiguous; impossible to claim v1.0 conformance without either full contracts or an explicit profile.

2. **`FAILURE` shape contradiction (2-arg vs 3-arg)**
- Validated: base type alias uses `FAILURE(ErrorCode, Str)` while stdlib templates use `FAILURE(code, message, details)`.
- Evidence: base spec type alias and stdlib section 1.2/5.2.
- Impact: type checker + pattern matching design cannot stabilize until one canonical result/error type is selected.

3. **Grammar/surface mismatch for core constructs**
- Validated: simplified EBNF omits/under-specifies constructs used heavily in examples (`STORE`, `MUTABLE`, channel `EMIT TO`, `OBSERVE`, `<=>`, `|>` semantics).
- Evidence: base grammar section 15 vs examples and keyword tables.
- Impact: parser and AST are under-defined; incompatible parsers likely.

4. **Formal semantics gaps for runtime-critical keywords**
- Validated: formal semantics includes core calculus but does not provide reduction rules for several listed surface constructs (`EMIT` as channel send, `OBSERVE`, `BROADCAST`, streaming behavior).
- Evidence: `formal_semantics.md` open questions + core rule set.
- Impact: runtime behavior differs by implementation; conformance tests cannot be written reliably.

5. **Typed API holes in stdlib (`HttpResponse[T]` undefined)**
- Validated: used by `core.http.*`, but no schema/type definition appears in any document.
- Impact: HTTP module cannot be implemented with static typing guarantees.

6. **Capability model inconsistency across docs**
- Validated: base spec defines one capability set (e.g., `DB_READ`, `API_CALL`), stdlib requires differently named capabilities (`LLM capability`, `register capability`, `invoke capability`, etc.) without canonical mapping.
- Impact: static capability checks and runtime enforcement cannot be deterministic.

7. **FORK/JOIN failure + typing semantics are incomplete**
- Validated: semantics names join strategies but leaves partial failure/timeouts and merged type behavior under-specified.
- Impact: scheduler/retry/merge behavior cannot be implemented safely for production-like workloads.

8. **Delegation capability boundary unresolved**
- Validated: explicit open question in formal semantics on caller vs callee capabilities (or intersection).
- Impact: security model for multi-agent execution is undefined.

### B. Important but Iterative (can be deferred or scoped for MVP)

1. **Token/cost budget primitive (`BUDGET`)**
- Validated as a gap, but not required to ship a minimal prototype if runtime-level quotas are used first.

2. **Prompt templating system (`PROMPT` typed artifacts)**
- High leverage, but can be delayed if MVP accepts plain `Str` prompts.

3. **Confidence provenance explainability and calibration rigor**
- Real issue (also acknowledged in formal semantics open questions).
- MVP can start with conservative confidence semantics + explicit "experimental" marker.

4. **Multimodal types**
- Useful but deferrable; text-first MVP is acceptable.

5. **Discovery/marketplace protocol at language level**
- Dynamic discovery can wait; explicit `DELEGATE ... TO fixed_agent` is enough for early iterations.

6. **Ergonomic uniformity (invocation style, comments/import ergonomics, naming consistency)**
- Improves generation reliability; not always blocker if a strict MVP grammar/profile is adopted.

### C. Partially Valid / Needs Narrowing

1. **"No import syntax"**
- Partially valid: there is `IMPORT` in FFI examples, but no clear module import mechanism for stdlib namespaces.

2. **"No learning/adaptation primitive"**
- Partially valid: primitives are listed in base module tables, but missing normative stdlib contracts; this is a coverage gap rather than complete conceptual absence.

## 3) Recommended Decision: Ship an Explicit MVP Profile

Do not attempt full v1.0 conformance immediately. Define and publish:

- **AgentLang MVP Profile v0.1** (normative subset)
- **AgentLang Full v1.0 (target)** (after spec closure)

This avoids blocking implementation on unresolved parts while preserving a path to full language conformance.

## 4) Concrete Next Steps / Implementation Plan

## Phase 0 (Week 1): Spec Freeze for MVP

1. Create `MVP_PROFILE.md` that explicitly includes/excludes features.
2. Choose canonical `Result` type and `FAILURE` arity (recommend 3-field: `code,message,details`).
3. Define canonical capability registry + alias map (old names accepted only as deprecated aliases).
4. Add missing foundational types required by included modules (`HttpResponse[T]`, regex/tokenize result types, etc.).
5. Publish grammar delta (`GRAMMAR_MVP.ebnf`) matching the parser target exactly.

Exit criteria:
- No unresolved contradictions for included MVP features.
- Parser grammar and type contracts are unambiguous.

## Phase 1 (Weeks 2-3): Reference Frontend (Parser + Type Checker)

1. Implement lexer/parser for `GRAMMAR_MVP.ebnf`.
2. Build typed AST with source spans and recoverable parse errors.
3. Implement type checker for:
- `Probable[T]` introduction/elimination
- `Result` + exhaustive `MATCH`
- refinement constraints in a restricted SMT-friendly subset
4. Implement capability checking pass against canonical registry.
5. Generate machine-readable diagnostics (for LLM self-correction loops).

Exit criteria:
- Golden tests for parsing and type-checking pass.
- Negative tests prove rejected programs fail with deterministic errors.

## Phase 2 (Weeks 4-6): Runtime Kernel + Minimal Scheduler

1. Implement runtime state model: CAS heap + named references + mutable cells.
2. Implement sequential pipeline execution and deterministic `->` semantics.
3. Implement `STORE`, `MUTABLE`, assignment, `MATCH`, `CHECKPOINT/RESUME` (single-agent first).
4. Implement minimal DAG scheduler with **explicitly limited** FORK/JOIN semantics:
- support `ALL_COMPLETE`
- mark `BEST_EFFORT` as not yet supported (or behind feature flag) until semantics are finalized.
5. Implement runtime capability gate + audit events.

Exit criteria:
- End-to-end execution of canonical examples in MVP profile.
- Checkpoint/restore reproducibility test suite passes.

## Phase 3 (Weeks 7-8): Stdlib MVP Implementation

Implement and test a strict subset first:
- `core.data`: `FILTER`, `MAP`, `REDUCE`, `SORT`, `GROUP`, `TAKE`, `SKIP`
- `core.io`: `READ`, `WRITE`, `FETCH`
- `core.text`: `PARSE`, `FORMAT`, split `REGEX` into typed variants or typed union
- `core.http`: `GET`, `POST` (after `HttpResponse[T]` spec)
- `agent.llm`: `GENERATE`, `CLASSIFY`, `EXTRACT`
- `agent.memory`: `REMEMBER`, `RECALL`, `FORGET`

For deferred operations/modules, mark `NOT_IMPLEMENTED` with compile-time capability/version diagnostics.

Exit criteria:
- Contract tests validate `REQUIRE/ENSURE/FAILURE` behavior for each shipped op.

## Phase 4 (Weeks 9-10): Multi-Agent MVP Hardening

1. Implement `DELEGATE` with explicit capability policy (recommend: callee runs with own capabilities; optional explicit delegated-token mechanism later).
2. Implement typed channel send/receive for one primitive path (`EMIT TO` + `OBSERVE`) or formally remove from MVP profile.
3. Define trust attenuation semantics used at runtime and record provenance evidence shape.
4. Add conformance test harness with:
- parser/type tests
- capability-denial tests
- checkpoint/delegation tests
- probabilistic handling tests

Exit criteria:
- Reproducible demo of multi-agent workflow from spec examples under MVP constraints.

## Phase 5 (Weeks 11-12): Developer Experience + Release

1. CLI (`agentlang check`, `agentlang run`, `agentlang test`).
2. Structured error output + suggested fixes.
3. Versioned feature flags (`--profile mvp-0.1`, experimental flags for deferred semantics).
4. Publish:
- MVP conformance matrix (feature-by-feature)
- migration roadmap to full v1.0

Exit criteria:
- MVP release candidate with documentation and passing CI.

## 5) Priority Action Items (Immediate)

1. **Publish MVP profile and canonical errata** for `FAILURE`, capabilities, grammar, and required missing types.
2. **Choose implementation subset intentionally** (do not imply full v1.0 coverage yet).
3. **Start parser/typechecker against frozen grammar/contracts**, not against narrative examples.
4. **Gate unresolved semantics behind feature flags** (`BEST_EFFORT`, advanced streaming/reactive constructs).
5. **Build conformance tests in parallel with implementation** so spec drift is caught early.

## 6) Risks if Not Addressed

1. Competing implementations will diverge in parser and runtime behavior.
2. Capability/security model may be inconsistent across delegation boundaries.
3. "Conformance" claims will be unverifiable due to undefined or contradictory contracts.
4. LLM-generated code reliability will remain low without stable syntax and diagnostics.

## 7) Suggested Ownership Model

1. **Spec Editor**: resolves contradictions and owns MVP profile.
2. **Compiler Lead**: parser/type system + diagnostics.
3. **Runtime Lead**: scheduler, checkpointing, capabilities, audit.
4. **Stdlib Lead**: operation contracts/tests and capability mapping.
5. **Conformance Lead**: golden tests + compliance matrix.

This structure keeps spec and implementation synchronized while moving quickly toward a working prototype.
