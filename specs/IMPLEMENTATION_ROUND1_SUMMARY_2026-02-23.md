# AgentLang MVP — Implementation Round 1 Summary

**Date:** 2026-02-23
**Profile:** mvp-0.1
**Total crate source lines:** ~10,650

---

## What Was Implemented

### 1. al-parser (NEW — 2,358 lines)
Full recursive-descent parser consuming the token stream produced by `al-lexer`.

- **Declarations:** TYPE (with generic params), SCHEMA (record fields), AGENT (CAPABILITIES, DENY, TRUST_LEVEL, TIMEOUT, MEMORY), OPERATION (INPUT, OUTPUT, REQUIRE, BODY, FAILURE_POLICY), PIPELINE (arrow `->` and pipe-forward `|>` chains)
- **Statements:** STORE, MUTABLE (with `@reason`), ASSIGN, MATCH/WHEN/OTHERWISE, LOOP (with `max:`), EMIT, HALT, ASSERT, RETRY, ESCALATE, CHECKPOINT, DELEGATE
- **Expressions:** Precedence-climbing (logical-or → logical-and → equality → comparison → additive → multiplicative → unary → postfix → primary); function calls with named arguments; member access (`.`); list/map literals; FORK/JOIN; parenthesised groups
- **Type expressions:** Named types with generic params, record types, union types (`|`), constrained types (`WHERE`)
- **Pattern matching:** SUCCESS, FAILURE (3-field canonical form), generic constructor patterns, literal/identifier/wildcard patterns
- **Tests:** 20 unit tests covering all declaration types, statements, and expressions

### 2. al-hir (EXPANDED — 384 lines, was 74)
High-level Intermediate Representation with full AST-to-HIR lowering pass.

- **HirDeclaration:** Type, Schema (field_count), Agent (capabilities), Operation (body statements), Pipeline (stage_count)
- **HirStatement:** Assert, Retry, Escalate, Checkpoint, Resume, Fork, Delegate, Store, Mutable, Assign, Match, Loop, Emit, Halt, Expr
- **HirExpr / HirExprKind:** Literal, Identifier, Call, BinaryOp, UnaryOp, Member, Pipeline, List, Map, Other
- **Lowering:** `lower_program`, `lower_declaration`, `lower_statement`, `lower_expr`
- **Metadata:** HirMeta (span, ty, required_caps, profile, synthetic)
- **Tests:** 5 unit tests including round-trip through parser

### 3. al-types (FIXED & EXPANDED — 420 lines, was 359)
Type checker with declaration table construction and semantic checks.

- Fixed test compilation bugs (TypeExpr::Named field types, iterator collect)
- Added parser-integration tests (`type_check_parsed_program`, `duplicate_operation_detected`)
- Passes: build declarations → check failure arity → check excluded features
- Utility: `check_retry_count`, `reject_non_mvp_join`

### 4. al-conformance (NEW — 211 lib + 430 tests = 641 lines)
Conformance test harness with all C1-C10 fixtures and 21 test functions.

| Fixture | Description | Tests |
|---------|-------------|-------|
| C1 | Lex/parse round-trip | 3 (roundtrip, empty, spans) |
| C2 | Failure arity 3-field pattern | 1 |
| C3 | Capability deny | 2 (parse + runtime) |
| C4 | Fork/join ALL_COMPLETE | 2 (parse + runtime) |
| C5 | Checkpoint/resume | 2 (parse + runtime) |
| C6 | Pipeline composition | 1 |
| C7 | Audit trail / ASSERT | 2 (parse + runtime) |
| C8 | Excluded features / MVP profile | 2 (subset + profile) |
| C9 | Type checking / duplicate detection | 2 |
| C10 | Retry/escalation | 3 (parse + retry runtime + escalation runtime) |
| Meta | all_fixtures_conform | 1 |

### 5. al-cli (NEW — 243 lines)
CLI binary with 4 commands: `lex`, `parse`, `check`, `run`.

- `lex`: Tokenizes input, prints token list with spans
- `parse`: Parses and prints AST summary (declaration types/counts)
- `check`: Parse + type-check, reports errors or env stats
- `run`: Full 4-phase pipeline (lex → parse → type check → capability check)

### 6. Pre-existing crates (NOT modified, already complete)
- **al-lexer:** 2,481 lines, 95 tests — full tokenizer with NEWLINE suppression
- **al-ast:** 608 lines — complete AST types
- **al-diagnostics:** 868 lines, 16 tests — Span, ErrorCode, Diagnostic, AuditEvent
- **al-capabilities:** 1,123 lines, 33 tests — 22 capabilities, grant/deny/delegation
- **al-runtime:** 1,596 lines, 31 tests — Value, Runtime, H/R/M/K/Q/L, fork-join, retry, checkpoint
- **al-stdlib-mvp:** ~200 lines, 5 tests — module/op registry
- **al-checkpoint:** ~150 lines, 4 tests — checkpoint store
- **al-vc:** ~120 lines, 4 tests — verification condition stubs

---

## Build & Test Status

```
cargo check:  OK  (3 minor warnings in pre-existing crates: unused imports)
cargo test:   249 passed, 0 failed

Test breakdown by crate:
  al-lexer         95
  al-capabilities  33
  al-runtime       31
  al-conformance   21 (integration tests)
  al-parser        20
  al-diagnostics   16
  al-types          7
  al-hir            5
  al-stdlib-mvp     5
  al-checkpoint     4
  al-vc             4
  al-capabilities   3 (doc-tests)
  al-ast            0 (types only)
  al-cli            0 (binary)
```

---

## What Remains (Round 2+)

1. **Parser completions:** Resume expression, error recovery / synchronisation, richer error messages with suggestions
2. **Type checker expansion:** Full type inference, schema field type resolution, operation input/output type matching, pipeline stage type propagation, capability requirement inference from operation bodies
3. **HIR enrichment:** Populate `ty` and `required_caps` metadata fields during type checking pass
4. **VC generation:** Wire `al-vc` to produce verification conditions from ASSERT/REQUIRE; add Z3/SMT solver backend or strengthen the stub solver
5. **Runtime interpreter:** Execute HIR/AST programs end-to-end (currently only individual semantic operations are tested)
6. **CLI improvements:** REPL mode, file watching, diagnostic formatting with source snippets, JSON/JSONL output
7. **Excluded feature detection:** Proper rejection of non-MVP constructs at parse time (SPAWN, CHANNEL, etc.)
8. **Error recovery:** Parser should attempt to continue after errors to report multiple issues
9. **Stdlib integration:** Wire al-stdlib-mvp into the runtime for built-in operation dispatch
10. **CI/CD:** GitHub Actions workflow for cargo check + cargo test + cargo clippy
