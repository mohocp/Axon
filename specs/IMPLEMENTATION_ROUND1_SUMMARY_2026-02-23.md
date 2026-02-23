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

---

## Round 2 — Implementation Summary

**Date:** 2026-02-24
**Focus:** P1/P2 issues from Codex review

### Changes Implemented

#### 1. Parser Error Recovery & Synchronization (P1 #1)
- Added `is_statement_keyword()` and `is_declaration_keyword()` helpers
- Added `recover_to_statement()` method for intra-block error recovery (synchronizes on statement keywords, `;`, `}`, or declaration keywords)
- Improved `parse_block()` to use structured statement-level recovery instead of ad-hoc token skipping
- Added `parse_recovering()` public API that returns `(Program, Vec<Diagnostic>)` — always returns partial results with diagnostics
- **6 new parser tests** covering recovery and new match body support

#### 2. Match Arm Body Statement Support (P1 #2, P2 #4)
- Rewrote `parse_match_body()` to recognize statement keywords (EMIT, ESCALATE, RETRY, HALT, CHECKPOINT, ASSERT, STORE, MATCH, LOOP, DELEGATE) directly after `->` without requiring block braces
- Statement keywords are parsed as a single statement and wrapped in a synthetic block, preserving AST compatibility
- **Tests:** `parse_match_body_emit_without_block`, `parse_match_body_escalate_without_block`, `parse_match_body_retry_checkpoint_assert`

#### 3. Type Reference Resolution (P1 #3)
- Added `BUILTIN_TYPES` constant (22 built-in type names: Int64, Float64, Str, Bool, List, Map, Set, Result, Option, Duration, Size, Confidence, Hash, Record, Any, Unit, Void, Int, Float, String, Bytes)
- Added Pass 4: `resolve_type_references()` — walks all type expressions in TYPE, SCHEMA, and OPERATION declarations
- `check_type_expr()` recursively verifies Named, Union, Constrained, and Record types
- Respects generic type parameters in scope (e.g., `TYPE Wrapper[T] = List[T]` — `T` is not flagged)
- Schema names are valid type references (e.g., `OUTPUT User` where `User` is a schema)
- Emits `UNKNOWN_IDENTIFIER` errors for undefined type references
- **5 new type checker tests**

#### 4. REQUIRE Clause Validation (P2 #5)
- Added Pass 5: `check_require_clauses()` — walks REQUIRE expressions on operations
- `check_require_expr()` recursively checks that top-level identifiers reference operation inputs
- Function call names are exempted (may be stdlib functions)
- Member access bases are checked (e.g., `data.fields` — `data` must be an input)
- **2 new type checker tests**

#### 5. Pipeline/Fork Reference Resolution (P2 #6, #7)
- Added Pass 6: `resolve_pipeline_fork_references()` — checks pipeline stages and fork branches
- Pipeline stages that are bare identifiers are checked against the operation table
- Fork branch pipeline chains are similarly checked
- Emits `UNRESOLVED_REFERENCE` **warnings** (not errors) since stages may reference stdlib functions
- **2 new type checker tests**

#### 6. Diagnostics
- Added `WarningCode::UnresolvedReference` to al-diagnostics for pipeline/fork warnings
- All new diagnostics use deterministic error codes aligned with conformance docs
- Error codes: `UNKNOWN_IDENTIFIER` for type/require errors, `UNRESOLVED_REFERENCE` (warning) for pipeline/fork

#### 7. New Conformance Fixtures (C11-C14)
| Fixture | Description | Tests |
|---------|-------------|-------|
| C11 | Match arm body: statement keywords after `->` | 1 |
| C12 | Undefined type reference detection | 2 (undefined + builtin) |
| C13 | Parser error recovery | 1 |
| C14 | REQUIRE clause validation | 2 (valid + unknown) |

### Build & Test Status

```
cargo check:  OK  (3 pre-existing warnings: unused imports in al-diagnostics, al-capabilities, al-checkpoint)
cargo test:   265 passed, 0 failed  (+16 from Round 1's 249)

Test breakdown by crate:
  al-lexer         95
  al-capabilities  33  (+ 3 doc-tests)
  al-runtime       31
  al-conformance   27  (was 21; +6 new integration tests)
  al-parser        26  (was 20; +6 new unit tests)
  al-diagnostics   16
  al-types         16  (was 7; +9 new unit tests)
  al-hir            5
  al-stdlib-mvp     5
  al-checkpoint     4
  al-vc             4
  al-ast            0  (types only)
  al-cli            0  (binary)
```

### What Remains (Round 3+)

1. **Full type inference:** Infer expression types, propagate through assignments and function calls
2. **Pipeline type propagation:** Check that operation output types chain correctly in pipelines
3. **HIR enrichment:** Populate `ty` and `required_caps` in HirMeta during type checking
4. **VC generation:** Wire `al-vc` for ASSERT/REQUIRE verification conditions
5. **Runtime interpreter:** Execute AST/HIR programs end-to-end
6. **Excluded feature rejection:** SPAWN, CHANNEL, non-MVP constructs
7. **CLI improvements:** REPL, file watching, diagnostic formatting with source snippets
8. **Stdlib integration:** Wire al-stdlib-mvp into runtime
9. **CI/CD:** GitHub Actions for cargo check/test/clippy
