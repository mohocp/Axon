# AgentLang

A programming language for agent-oriented computing.

AgentLang provides first-class primitives for defining autonomous agents, their capabilities, data schemas, computational operations, and multi-stage pipelines — with built-in verification, fault tolerance, and security enforcement.

**Version:** 0.1.0-rc1 (MVP)
**License:** MIT
**MSRV:** Rust 1.75.0

## Why AgentLang?

### The Core Insight

Every mainstream programming language is designed for human cognition — readable syntax, manual control flow, implicit trust. AgentLang starts from a different premise: **the primary programmers of the future are AI agents, not humans.** Agents think in token sequences, operate probabilistically, and can hallucinate. They need a language built for how they actually work.

This isn't an incremental improvement on existing languages. It's a greenfield design from first principles.

### Design Principles

**Semantic Density** — Every token carries maximum meaning, zero decoration. Agents pay a computational cost per token during inference. AgentLang replaces ceremony with single high-meaning keywords (`OPERATION`, `AGENT`, `SCHEMA`). Human readability is a beneficial side effect, not a design goal.

**Verification by Construction** — Programs carry their own correctness proofs. Every operation includes preconditions (`REQUIRE`), postconditions (`ENSURE`), and invariants (`INVARIANT`) that are mechanically verified before execution. This is non-optional. Agents hallucinate, generate incorrect logic, and produce unexpected side effects — you can't rely on code review to catch errors in agent-generated code. The language itself enforces correctness.

**Parallel by Default** — Sequential execution is the exception. The runtime analyzes the dataflow dependency graph and automatically parallelizes independent operations. No `async`, no `await`, no thread management. Agent workloads are inherently parallel — forcing sequential expression creates artificial bottlenecks.

**Probabilistic Awareness** — The type system natively represents uncertainty. `Probable[T]` carries a confidence score `c ∈ [0,1]`, provenance chain, and degradation policy. Every LLM output is inherently uncertain — rather than agents pretending otherwise, the type system forces explicit handling. You cannot use a `Probable[T]` where a `T` is expected without crossing a confidence threshold.

**Agent-Native Coordination** — Delegation, capabilities, trust levels, and conflict resolution are language primitives, not library abstractions. Multi-agent coordination in frameworks like LangChain is scattered across user code without language-level guarantees. AgentLang makes agents first-class entities with declared identity, typed delegation, and capability enforcement.

### Trust, Safety, and Accountability

AgentLang encodes a core conviction: **autonomous agents must be constrained, not trusted.**

- **Capability-based security** — Every agent declares exactly what it can do (`CAPABILITIES [DB_READ, API_CALL]`) and what it's forbidden from (`DENY [SELF_MODIFY, NETWORK_RAW]`). 22 canonical capabilities enforce the principle of least privilege at the language level.

- **Delegation isolation** — When agent A delegates to agent B, B runs under B's own capabilities, never A's. This prevents capability smuggling — a high-privilege agent can't trick a low-privilege agent into acting on its behalf with elevated permissions.

- **Trust levels attenuate confidence** — An agent with `TRUST_LEVEL ~0.60` has its output confidence scaled down automatically. High-trust results flow freely; low-trust results require verification by a trusted agent before use.

- **Immutable by default** — `STORE` bindings are immutable. `MUTABLE` requires an explicit reason annotation (`@reason("tracking running total")`). This forces deliberate justification for any mutable state, preventing accidental side effects.

- **Mandatory audit trail** — Every state change, capability usage, and delegation is logged in an append-only audit trail. This isn't optional logging — it's baked into the runtime.

- **Human escalation as a primitive** — `ESCALATE_HUMAN` is a capability. Critical operations can require human approval with timeout and abort semantics. The language acknowledges that some decisions should not be fully automated.

### Verification Philosophy

AgentLang takes a pragmatic stance on formal verification:

- **Static verification** where the SMT solver can prove it — compile-time guarantee for all executions
- **Runtime assertions** where the solver returns Unknown — fail-open at compile time, fail-closed at runtime
- **Never silent failure** — if verification can't prove correctness, it inserts a runtime check that halts with a detailed audit event if violated

This "fail-open compile / fail-closed execute" strategy balances pragmatism with safety.

### Why Not Existing Languages?

| Problem | Existing Languages | AgentLang |
|---|---|---|
| Agent output is uncertain | No native uncertainty types | `Probable[T]` with confidence scores |
| Agents hallucinate | Trust human review | Mandatory REQUIRE/ENSURE/INVARIANT |
| Multi-agent coordination | Libraries (LangChain, CrewAI) | Language primitives with type safety |
| Permission control | OS-level or framework-level | 22 fine-grained capabilities per agent |
| Fault tolerance | Manual serialization | Built-in CHECKPOINT/RESUME with effect journal |
| Token efficiency | Syntax optimized for humans | Semantic density optimized for agents |
| Parallelism | Manual async/threads | Automatic dataflow parallelism |

### The Vision

AgentLang envisions a future where agents write verified programs from natural language intent, where the language evolves organically as agents compose operations into higher-level patterns, where heterogeneous AI models collaborate through a unified type system, and where distributed execution is transparent. It's not trying to be a better Python. It's the native language of autonomous computation.

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
