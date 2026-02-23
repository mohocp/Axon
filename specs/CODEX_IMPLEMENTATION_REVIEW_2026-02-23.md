# AgentLang MVP Round 2 — Code Review

**Date:** 2026-02-24 (updated from 2026-02-23 Round 1 review)
**Reviewer:** Claude (independent review pass)
**Scope:** All code produced through Round 2 implementation

---

## Status of Round 1 Issues

### P1 — Critical (Round 1) → Status

1. **~~No parser error recovery.~~** ✅ FIXED. Added `recover_to_statement()` and `recover_to_declaration()` methods. `parse_block()` now uses structured statement-level recovery. New `parse_recovering()` API returns partial program with diagnostics.

2. **~~Match arm bodies require block syntax for statements.~~** ✅ FIXED. `parse_match_body()` now detects statement keywords (EMIT, ESCALATE, RETRY, HALT, CHECKPOINT, ASSERT, etc.) after `->` and wraps them in a synthetic block. `WHEN SUCCESS(val) -> EMIT val` now parses correctly.

3. **~~Type checker does not verify type references.~~** ✅ FIXED. Added Pass 4 (`resolve_type_references`) with 22 built-in types. Walks TYPE, SCHEMA, OPERATION declarations. Generic type parameters are tracked in scope. Emits `UNKNOWN_IDENTIFIER` for undefined references.

### P2 — Important (Round 1) → Status

4. **~~ESCALATE in MATCH body requires block wrapper.~~** ✅ FIXED. Same fix as #2 — all statement keywords are recognized in match bodies.

5. **~~REQUIRE clauses not type-checked.~~** ✅ FIXED. Added Pass 5 (`check_require_clauses`) that validates identifiers in REQUIRE expressions against operation inputs.

6. **~~Pipeline type propagation absent.~~** ⚠️ PARTIAL. Added Pass 6 with warning-level reference resolution. Pipeline stages that are bare identifiers are checked against the operation table. Full output→input type chaining is deferred to Round 3.

7. **~~Fork branch names unresolved.~~** ⚠️ PARTIAL. Fork branch pipeline chains are now checked for operation references (warning level). Semantic validation of fork branch types is deferred.

8. **HIR lowering discards expression details.** ❌ NOT ADDRESSED. Still only captures variable names/counts. Deferred to Round 3 HIR enrichment.

9. **`parse_fork_expr` span arithmetic unvalidated.** ❌ NOT ADDRESSED. Low risk — spans are computed from actual token positions.

### P3 — Minor (Round 1) → Status

10. **Unused `al-capabilities` dependency in `al-types`.** ❌ NOT ADDRESSED. Cosmetic.
11. **CLI capability check hardcoded.** ❌ NOT ADDRESSED.
12. **No `Display` impl for AST nodes.** ❌ NOT ADDRESSED.
13. **`check_source` swallows type errors.** By design — documented.
14. **Three unused-import warnings.** ❌ NOT ADDRESSED. Pre-existing.
15. **Binding power starting at 0.** ❌ NOT ADDRESSED. Not a bug.
16. **`NONE` keyword handling.** ✅ Already handled — `Token::None` has a case in `parse_primary`.

---

## New Issues Found in Round 2

### P1 — Critical

*(None — no new P1 issues introduced.)*

### P2 — Important

17. **REQUIRE identifier check is strict but context-limited.** The REQUIRE validator flags any top-level identifier that isn't an operation input, but doesn't know about `STORE` bindings in scope or schema field references. `REQUIRE compute(data) GT 0` works (function calls are exempted), but `REQUIRE threshold GT 0` where `threshold` is a STORE binding from a prior operation would incorrectly flag. This is acceptable for MVP since REQUIRE runs before BODY, but the limitation should be documented.

18. **Pipeline/fork reference warnings use `WarningCode::UnresolvedReference`.** This is a new warning code but currently doesn't have a serde roundtrip test. The existing `all_error_codes_roundtrip` test doesn't cover warning codes beyond `CapAliasDeprecated`.

19. **`parse_recovering` does not recover from lex errors.** If `al_lexer::tokenize` returns `Err`, `parse_recovering` returns an empty program. True recovery would require a fault-tolerant lexer or per-line re-lexing. This is a known limitation.

20. **Type reference check does not track forward declarations.** `TYPE Foo = Bar` where `Bar` is defined later in the same file will fail because `build_declarations` runs sequentially and `resolve_type_references` checks against the complete table. Wait — actually this works because `build_declarations` builds the full table first, then `resolve_type_references` runs as a separate pass. ✅ No issue.

### P3 — Minor

21. **`BUILTIN_TYPES` list may need expansion.** The current list (22 entries) covers common types but may miss domain-specific types as the language evolves. The list should be extracted into a shared constant or configuration.

22. **Warning for unresolved pipeline stages could be noisy.** Every fixture with a pipeline (C1, C6) generates warnings for stage names like `fetch`, `validate`, `transform`. These are expected in test contexts but may confuse real users. Consider a `--strict` flag or separate lint pass.

23. **Synthetic block spans reuse the statement span.** When a match body statement is wrapped in a synthetic block, the block's span is identical to the statement's span. This is correct for error reporting but could confuse span-based tooling that expects blocks to have `{`/`}` positions.

---

## Metrics

| Metric | Round 1 | Round 2 | Delta |
|--------|---------|---------|-------|
| Total tests | 249 | 265 | +16 |
| al-parser tests | 20 | 26 | +6 |
| al-types tests | 7 | 16 | +9 |
| al-conformance tests | 21 | 27 | +6 |
| Conformance fixtures | C1–C10 | C1–C14 | +4 |
| Type checker passes | 3 | 6 | +3 |
| Compilation warnings | 3 | 3 | 0 |

---

## Enhancements (carried from Round 1 + new)

### Architecture
- Add a `Session`/`CompilationUnit` struct threading source, AST, HIR, type env, diagnostics
- Introduce a `Visitor` trait for AST/HIR traversal
- Extract `BUILTIN_TYPES` into a shared constant or config

### Testing
- Add `proptest` property-based tests for lexer/parser
- Add negative conformance fixtures (C15+): SPAWN, CHANNEL, malformed FAILURE arity
- Add CLI integration tests (`assert_cmd`)
- Add serde roundtrip test for `WarningCode::UnresolvedReference`

### Performance
- String interning for identifiers
- Arena allocation for AST/HIR nodes

### Developer Experience
- `cargo clippy` in CI
- `#[must_use]` on `parse()`, `check()`, etc.
- Document EBNF-to-parser mapping in comments

---

## Summary

Round 2 addressed all 3 P1 critical issues and 3 of 6 P2 important issues from the Round 1 review. The parser now has multi-level error recovery (declaration and statement boundaries), match arm bodies accept statement keywords directly, and the type checker resolves type references, validates REQUIRE clauses, and warns on unresolved pipeline/fork references. 265 tests pass with 0 failures. 4 new conformance fixtures (C11–C14) validate the new functionality.

**Remaining high-priority items:** Full type inference, pipeline type propagation, HIR enrichment, and VC generation. These are natural targets for Round 3.

**Verdict:** Solid progress. All P1 issues resolved. Foundation ready for type inference and runtime interpreter work.
