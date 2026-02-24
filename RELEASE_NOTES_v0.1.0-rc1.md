# Release Notes — AgentLang v0.1.0-rc1

**Date:** 2026-02-24
**Profile:** MVP v0.1
**Tag:** `v0.1.0-rc1`

---

## Overview

This is the first release candidate of AgentLang, an agent-oriented programming language. It implements the full MVP v0.1 profile as defined in `specs/MVP_PROFILE.md`, covering the complete pipeline from source code to execution.

## What's Included

### Language Implementation (13 crates)

- **Lexer** (`al-lexer`): Full tokenization with span tracking, NEWLINE suppression, property-based tests
- **Parser** (`al-parser`): Recursive-descent parser with error recovery, all MVP grammar constructs, proptest coverage
- **AST** (`al-ast`): Complete declaration/statement/expression node types
- **HIR** (`al-hir`): High-Intermediate Representation with type and capability metadata
- **Type Checker** (`al-types`): 6-pass type checking — declaration table, failure arity, excluded features, type references, REQUIRE validation, pipeline/fork refs. Property-based tests.
- **Verification Conditions** (`al-vc`): VC generation from REQUIRE/ENSURE/INVARIANT/ASSERT. Stub solver with Unknown-to-ASSERT rewriting.
- **Capabilities** (`al-capabilities`): 22 canonical capability identifiers, alias normalization, grant/deny/delegation enforcement
- **Runtime** (`al-runtime`): Full interpreter — statement evaluation, expression evaluation, pattern matching, pipeline execution, fork/join, retry, escalate, assert, capability checks, delegation
- **Standard Library** (`al-stdlib-mvp`): 21 operations across 6 modules with signature-lock CI gate
- **Checkpoint** (`al-checkpoint`): Checkpoint/resume with versioned schema, hash validation, effect journal
- **Diagnostics** (`al-diagnostics`): Structured error/warning/audit rendering in human, JSON, and JSONL formats
- **Conformance** (`al-conformance`): 20 conformance requirements (C1-C20) with 45 test functions
- **CLI** (`al-cli`): `lex`, `parse`, `check`, `run` commands with `--format` output control

### Test Coverage

| Category | Count |
|----------|-------|
| Total tests | 484 |
| Conformance tests (C1-C20) | 45 |
| Property-based test crates | 3 (lexer, parser, types) |
| CLI integration tests | 3 (end-to-end) |
| Stdlib signature-lock tests | 19 |
| Diagnostic audit tests | 3 |

### CI Pipeline (9 gates)

1. Format (`cargo fmt --check`)
2. Clippy (zero warnings, `-D warnings`)
3. Build (full workspace)
4. Test (484 tests)
5. Conformance (C1-C20)
6. Signature Lock (stdlib API stability)
7. Audit Schema (diagnostic integrity)
8. MSRV (Rust 1.75.0)
9. Security Audit (`cargo-audit`)

### Example Programs

- `examples/calculate.al` — Pipeline with arithmetic operations (result: 94)
- `examples/factorial.al` — Bounded loops computing factorial (result: 720)
- `examples/match_result.al` — Pattern matching on Result types with agent capabilities (result: 84)

## Implementation Rounds

| Round | Focus | Key Deliverables |
|-------|-------|------------------|
| 1 | Foundation | Workspace, lexer, parser, AST, diagnostics |
| 2 | Static analysis | Type checker (6 passes), capabilities, HIR |
| 3 | Type inference | Expression typing, pipeline propagation, HIR enrichment |
| 4 | Verification | VC generation, solver stub, Unknown rewriting, delegation checks |
| 5 | Runtime | Full interpreter, pipeline execution, fork/join, retry/escalate |
| 6 | Stdlib & checkpoint | 21 stdlib ops, checkpoint/resume, audit trail JSONL |
| 7 | Hardening | CI pipeline, C1-C20 conformance, snapshots, proptests |
| 8 | RC packaging | README, release notes, conformance matrix, known limitations |

## Conformance Status

All 20 conformance requirements pass:

- **C1-C10**: Core MVP requirements (lex/parse, failure arity, capabilities, fork/join, checkpoint, pipelines, audit, excluded features, type checking, retry/escalate)
- **C11-C14**: Extended positive tests (match arms, undefined types, parser recovery, REQUIRE validation)
- **C15-C20**: Negative conformance (malformed FAILURE, excluded join strategies, duplicate definitions, ENSURE/INVARIANT)

See `CONFORMANCE_MATRIX.md` for the full matrix with test mappings.

## Known Limitations

13 documented limitations, none blocking for RC. See `KNOWN_LIMITATIONS.md` for details. Key items:
- Stub SMT solver (fail-safe Unknown handling)
- Sequential fork-join (correct but not parallel)
- Stub I/O, HTTP, and LLM backends
- No REPL or LSP
- Source-only distribution

## Deferred Backlog (Post-MVP)

- Real SMT solver backend (Z3/CVC5)
- Concurrent fork-join execution
- REPL / language server (LSP)
- BEST_EFFORT / PARTIAL join strategies
- Reactive semantics (OBSERVE, BROADCAST, channels)
- Full polymorphic type inference
- Cross-compiled binary distribution
- Performance optimizations (string interning, arena allocation)

## Metrics

| Metric | Value |
|--------|-------|
| Crates | 13 |
| Rust LOC | ~19,000 |
| Tests | 484 |
| Conformance fixtures | 20 (C1-C20) |
| CI gates | 9 |
| Example programs | 3 |
| Stdlib operations | 21 |
| Capability identifiers | 22 |
| MSRV | 1.75.0 |

## Upgrade Path

This is the initial release candidate. No prior version exists to upgrade from. The GA release (v0.1.0) will follow after RC validation feedback is incorporated.
