# Known Limitations — AgentLang v0.1.0-rc1

**Date:** 2026-02-24
**Profile:** MVP v0.1

This document lists known limitations and deferred features in the v0.1.0-rc1 release candidate. Items are categorized by severity and area.

---

## Language Limitations

### L1. No Real SMT Solver Backend
- **Area:** Verification conditions
- **Impact:** Medium
- **Description:** The VC pipeline generates verification conditions from `REQUIRE`, `ENSURE`, `INVARIANT`, and `ASSERT` clauses, but uses a stub solver that returns `Unknown` for non-trivial conditions. Per spec, `Unknown` triggers fail-open compile (synthetic runtime ASSERT injection) and fail-closed runtime behavior.
- **Workaround:** Runtime ASSERTs catch violations at execution time. No false negatives — only false positives (unnecessary runtime checks).
- **Deferred to:** Post-MVP (Z3/CVC5 integration).

### L2. Monomorphic Type Instantiation Only
- **Area:** Type system
- **Impact:** Low
- **Description:** Generic types (e.g., `Result[T]`) are instantiated monomorphically. Full polymorphic type inference (higher-kinded types, variance) is deferred.
- **Workaround:** Explicit type annotations at call sites.
- **Deferred to:** Post-MVP.

### L3. Sequential Fork-Join Execution
- **Area:** Runtime
- **Impact:** Low
- **Description:** `FORK` branches execute sequentially (not concurrently). The `ALL_COMPLETE` join strategy collects all results, but there is no parallelism.
- **Workaround:** Correct semantics preserved; only performance is affected.
- **Deferred to:** Post-MVP (async runtime / thread pool).

### L4. Excluded Language Features
- **Area:** Language surface
- **Impact:** By design
- **Description:** The following are explicitly excluded from MVP v0.1 and rejected at compile time with `NOT_IMPLEMENTED`:
  - Reactive/channel runtime (`OBSERVE`, `BROADCAST`, `EMIT TO channel`)
  - Sync operator (`<=>`)
  - Join variants (`BEST_EFFORT`, `PARTIAL(min=k)`)
  - Self-modification flow (`MUTATE OPERATION`)
  - Streaming semantics (`STREAM`)
  - Dynamic agent discovery (`DISCOVER`)
  - Force/override operator (`!`)
- **Reference:** `specs/MVP_PROFILE.md` sections 2-3.

### L5. No Recursion Support
- **Area:** Runtime
- **Impact:** Low
- **Description:** Recursive operation calls are not supported. No cycle detection at type-check time; runtime may stack overflow on recursive programs.
- **Workaround:** Use bounded `LOOP` for iterative computation.
- **Deferred to:** Post-MVP (depth-limited recursion with trampoline).

---

## Tooling Limitations

### L6. No REPL or Language Server
- **Area:** Developer experience
- **Impact:** Medium
- **Description:** No interactive REPL or LSP server is provided. Development requires the batch CLI (`al-cli`).
- **Deferred to:** Post-MVP.

### L7. No Incremental Compilation
- **Area:** Performance
- **Impact:** Low
- **Description:** Every invocation re-lexes, re-parses, and re-checks the entire source file. No caching of intermediate results.
- **Deferred to:** Post-MVP (file-watching / session caching).

### L8. CLI Binary Not Cross-Compiled
- **Area:** Distribution
- **Impact:** Low
- **Description:** This RC ships as source only. Pre-built binaries for Linux/macOS/Windows are not provided.
- **Workaround:** Build from source with `cargo build --release -p al-cli`.
- **Deferred to:** GA release (CI cross-compilation matrix).

---

## Standard Library Limitations

### L9. Stub I/O and HTTP Backends
- **Area:** Stdlib
- **Impact:** Medium
- **Description:** `core.io` (READ, WRITE, FETCH) and `core.http` (GET, POST) operations use simplified in-process implementations. No real filesystem I/O or HTTP networking in MVP.
- **Workaround:** Programs execute with mock/in-memory backends suitable for validation and testing.
- **Deferred to:** Post-MVP (trait-based backend injection).

### L10. LLM Operations Are Stubs
- **Area:** Stdlib
- **Impact:** Medium
- **Description:** `agent.llm` operations (GENERATE, CLASSIFY, EXTRACT) return placeholder results. No real LLM provider integration.
- **Workaround:** Backend-agnostic trait interface is defined; real providers can be wired post-MVP.
- **Deferred to:** Post-MVP.

### L11. Excluded Stdlib Modules
- **Area:** Stdlib
- **Impact:** By design
- **Description:** The following modules are excluded from MVP v0.1:
  - `db.sql`, `db.vector`, `db.graph`
  - `api.rest`, `api.grpc`
  - `queue.pubsub`
  - `core.http` PUT/DELETE
  - `core.io` STREAM
  - `core.math`, `core.time`, `core.crypto`, `core.json`
  - `agent.tools`, `agent.planning`, `agent.reflection`
- **Reference:** `specs/MVP_PROFILE.md` section 6.

---

## Specification Gaps

### L12. Checkpoint Schema Forward Compatibility
- **Area:** Fault tolerance
- **Impact:** Low
- **Description:** Checkpoint serialization uses JSON. Schema versioning is implemented, but no formal migration path exists for schema evolution across versions.
- **Deferred to:** Post-MVP.

### L13. Audit Trail Storage
- **Area:** Diagnostics
- **Impact:** Low
- **Description:** Audit events are emitted as JSONL to stdout/stderr. No persistent audit store or log rotation is provided.
- **Deferred to:** Post-MVP (pluggable audit sinks).

---

## Summary

| ID | Area | Severity | Status |
|----|------|----------|--------|
| L1 | Verification | Medium | Deferred (stub solver, fail-safe) |
| L2 | Type system | Low | Deferred |
| L3 | Runtime | Low | Deferred (sequential fork) |
| L4 | Language | By design | Excluded features rejected |
| L5 | Runtime | Low | Deferred |
| L6 | Tooling | Medium | Deferred |
| L7 | Performance | Low | Deferred |
| L8 | Distribution | Low | Source-only for RC |
| L9 | Stdlib | Medium | Stub backends |
| L10 | Stdlib | Medium | Stub LLM |
| L11 | Stdlib | By design | Excluded modules |
| L12 | Checkpoint | Low | Deferred |
| L13 | Audit | Low | Deferred |

Total: 13 known limitations. None are blockers for RC validation. All have documented workarounds or are excluded by design.
