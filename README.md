# AgentLang

A programming language for agent-oriented computing.

AgentLang provides first-class primitives for defining autonomous agents, their capabilities, data schemas, computational operations, and multi-stage pipelines — with built-in verification, fault tolerance, and security enforcement.

**Version:** 0.1.0-rc1 (MVP)
**License:** MIT
**MSRV:** Rust 1.75.0

## Quick Start

### Prerequisites

- Rust toolchain >= 1.75.0 (`rustup install 1.75.0`)

### Build

```bash
cargo build --workspace
```

### Install (pre-built binary)

```bash
curl -fsSL https://raw.githubusercontent.com/mohammedabuhalib/agentlang/main/install.sh | sh
```

Or build from source:

```bash
cargo install --path crates/al-cli
```

### Run an Example

```bash
# Execute a pipeline
al run examples/calculate.al

# Factorial with loops
al run examples/factorial.al

# Pattern matching on Result types
al run examples/match_result.al
```

### CLI Commands

```bash
# Tokenize a source file
al lex <file.al>

# Parse and print AST summary
al parse <file.al>

# Type-check a source file
al check <file.al>

# Full pipeline: lex -> parse -> check -> execute
al run <file.al>
```

### Output Formats

All commands support `--format` for diagnostic output:

```bash
al run program.al --format human   # Human-readable (default)
al run program.al --format json    # JSON object
al run program.al --format jsonl   # JSON Lines
```

### Run Tests

```bash
# All tests
cargo test --workspace

# Conformance suite (C1-C20)
cargo test -p al-conformance --test conformance -- --nocapture

# Stdlib signature lock
cargo test -p al-stdlib-mvp -- --nocapture

# Diagnostic audit
cargo test -p al-diagnostics -- audit --nocapture
```

## Language Overview

AgentLang programs are built from five core declarations:

```agentlang
TYPE UserId = Int64

SCHEMA User => { name: Str, age: Int64 }

AGENT Worker =>
  CAPABILITIES [FILE_READ, API_CALL]
  DENY [FILE_WRITE]
  TRUST_LEVEL ~0.8

OPERATION GetUser =>
  INPUT id: Int64
  OUTPUT User
  REQUIRE id GT 0
  BODY {
    STORE user = fetch(id)
    EMIT user
  }

PIPELINE Process => fetch -> validate |> transform -> store
```

### Key Features (MVP v0.1)

| Feature | Description |
|---------|-------------|
| **Declarations** | `TYPE`, `SCHEMA`, `AGENT`, `OPERATION`, `PIPELINE` |
| **Control flow** | `MATCH`/`WHEN`/`OTHERWISE`, bounded `LOOP`, `HALT` |
| **Error handling** | `SUCCESS(T)` / `FAILURE(code, msg, details)`, `RETRY(n)`, `ESCALATE` |
| **Concurrency** | `FORK` / `JOIN strategy: ALL_COMPLETE` |
| **Verification** | `REQUIRE`, `ENSURE`, `INVARIANT`, `ASSERT` |
| **Capabilities** | 22 canonical capability identifiers, `DENY`, `DELEGATE` |
| **Fault tolerance** | `CHECKPOINT`, `RESUME` |
| **Pipelines** | Arrow `->` and pipe-forward `|>` composition |

### Standard Library (MVP)

| Module | Operations |
|--------|-----------|
| `core.data` | FILTER, MAP, REDUCE, SORT, GROUP, TAKE, SKIP |
| `core.io` | READ, WRITE, FETCH |
| `core.text` | PARSE, FORMAT, REGEX, TOKENIZE |
| `core.http` | GET, POST |
| `agent.llm` | GENERATE, CLASSIFY, EXTRACT |
| `agent.memory` | REMEMBER, RECALL, FORGET |

## Architecture

```
crates/
  al-ast/          Abstract Syntax Tree definitions
  al-lexer/        Tokenization with span tracking
  al-parser/       Recursive-descent parser with error recovery
  al-hir/          High-Intermediate Representation
  al-types/        Type checker (6 passes)
  al-vc/           Verification condition generation
  al-capabilities/ Capability system (22 caps, alias normalization)
  al-runtime/      Interpreter with full execution pipeline
  al-stdlib-mvp/   Standard library (21 operations)
  al-checkpoint/   Checkpoint/resume fault tolerance
  al-diagnostics/  Error/warning rendering (human/JSON/JSONL)
  al-conformance/  Conformance test suite (C1-C20, 45 tests)
  al-cli/          Command-line interface
```

## Conformance

This release passes all 20 conformance requirements (C1-C20) with 45 conformance-level tests. See `CONFORMANCE_MATRIX.md` for the full matrix.

## Documentation

| Document | Location |
|----------|----------|
| Full specification | `AgentLang_Specification_v1.0.md` |
| MVP profile | `specs/MVP_PROFILE.md` |
| Grammar (EBNF) | `specs/GRAMMAR_MVP.ebnf` |
| Formal semantics | `specs/formal_semantics.md` |
| Stdlib spec | `specs/stdlib_spec_mvp.md` |
| Known limitations | `KNOWN_LIMITATIONS.md` |
| Release notes | `RELEASE_NOTES_v0.1.0-rc1.md` |

## CI Pipeline

9 automated gates on every push/PR to `main`:

1. **Format** — `cargo fmt --check`
2. **Clippy** — zero warnings
3. **Build** — full workspace compilation
4. **Test** — 484 tests across 13 crates
5. **Conformance** — C1-C20 (45 tests)
6. **Signature Lock** — stdlib API stability
7. **Audit Schema** — diagnostic integrity
8. **MSRV** — Rust 1.75.0 compatibility
9. **Security Audit** — `cargo-audit` dependency scan
