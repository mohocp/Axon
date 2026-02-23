# AgentLang MVP Round 1 — Code Review

**Date:** 2026-02-23
**Reviewer:** Claude (independent review pass)
**Scope:** All code produced in Round 1 implementation

---

## Issues

### P1 — Critical / Must Fix

1. **No parser error recovery.** The parser aborts on the first error (`self.error()` returns `Err(())` and propagates up). Any single syntax error prevents parsing the rest of the file. Real-world usage requires synchronisation (skip to next declaration/statement boundary).

2. **Match arm bodies require block syntax for statements.** `WHEN SUCCESS(val) -> EMIT val` doesn't parse because `EMIT` is a keyword, not an expression. The workaround (`-> { EMIT val }`) deviates from what the spec likely intends. The parser's `parse_match_body` should handle statement-or-expression after `->`.

3. **Type checker does not verify type references.** `TYPE Foo = Bar` compiles even if `Bar` is never defined. The declaration table is built but never consulted for reference resolution. Undefined type references should emit `UNDEFINED_REFERENCE` errors.

### P2 — Important

4. **`ESCALATE(msg)` inside MATCH body requires block wrapper.** Same root cause as #2 — ESCALATE is a keyword token. The parser should recognise statement keywords (EMIT, ESCALATE, RETRY, HALT, CHECKPOINT, ASSERT) as valid match body starts, not just expressions and blocks.

5. **`REQUIRE` clauses not type-checked.** The parser correctly parses REQUIRE conditions on operations, but the type checker ignores them entirely. The condition expressions should be validated.

6. **Pipeline type propagation absent.** `PIPELINE DataFlow => fetch -> validate |> transform -> store` is parsed correctly, but there is no check that `fetch`, `validate`, `transform`, `store` refer to defined operations, or that their input/output types chain correctly.

7. **Fork branch names unresolved.** `FORK { a: fetch, b: validate }` parses the named branches, but there's no check that `fetch` and `validate` are defined operations.

8. **HIR lowering discards expression details.** `lower_statement` for `Statement::Store { .. }` only captures the variable name, losing the initialiser expression entirely. Similarly, `Match` only captures arm count, losing pattern/body information. This makes HIR unsuitable for a future interpreter or code generator without re-consulting the AST.

9. **`parse_fork_expr` uses span arithmetic but doesn't validate.** The fork expression builds spans from `start` captured before parsing. If the parser backtracks or errors during fork body parsing, the span could be incorrect.

### P3 — Minor / Enhancement

10. **Unused `al-capabilities` dependency in `al-types`.** `Cargo.toml` lists `al-capabilities` as a dependency, but the type checker never uses it. This adds unnecessary compilation.

11. **CLI `cmd_run` capability check is hardcoded.** It creates a `CapabilitySet::all()` for every agent, which means capability checking always succeeds. Real checking needs to use the agent's declared capabilities.

12. **No `Display` impl for AST nodes.** Pretty-printing relies on `{:?}` (Debug), which produces Rust-internal formatting. A proper `Display` implementation would improve CLI output and diagnostics.

13. **`check_source` in al-conformance swallows type errors.** It returns `Ok(checker)` even when `checker.has_errors()` is true. This is by design (callers inspect `has_errors()`), but the API is surprising — callers might expect `Err` on type errors.

14. **Three unused-import warnings in pre-existing crates.** `DateTime` in al-diagnostics, `Severity` in al-capabilities, `Span` in al-checkpoint. These are cosmetic but should be cleaned.

15. **`parse_expression_bp` minimum binding power starts at 0.** The precedence climbing implementation uses `min_bp: u8` starting at 0, which works but means all operators must have bp >= 1. This isn't documented and could confuse future contributors.

16. **`NONE` keyword parsed as identifier.** In expression context, `NONE` is lexed as a keyword token but the parser's `parse_primary` doesn't handle it, falling through to an error or treating it as an identifier depending on context.

---

## Enhancements

### Architecture

- **Add a `Session` or `CompilationUnit` struct** that threads source text, AST, HIR, type env, and diagnostics through the pipeline. Currently the CLI manually passes data between stages.

- **Introduce a `Visitor` trait for AST/HIR traversal** instead of manual recursive `match` in every pass. This would simplify type checking, capability inference, and future lint passes.

- **Consider making Diagnostic implement `std::error::Error`** so it integrates with Rust's `?` operator and `anyhow`/`thiserror` patterns.

### Testing

- **Add property-based tests** (e.g. via `proptest`) for the lexer and parser: generate random valid AgentLang programs and verify round-trip (source → tokens → AST → pretty-print → tokens → AST).

- **Add negative conformance fixtures** (C11+): programs that should be rejected (e.g., SPAWN, CHANNEL, non-MVP join strategies, malformed FAILURE patterns with wrong arity).

- **Add integration tests for CLI binary** using `assert_cmd` or similar, testing actual `al-cli lex/parse/check/run` invocations on fixture files.

### Performance

- **Intern strings** in the AST (use a string interner like `lasso` or `string-interner`). Currently every identifier/type name is a separate `String` allocation. For large programs this adds up.

- **Consider arena allocation** for AST/HIR nodes (e.g. `bumpalo`) to reduce allocator pressure during parsing.

### Developer Experience

- **Add `cargo clippy` to CI** — there are likely additional warnings beyond the unused imports.

- **Add `#[must_use]` annotations** on functions like `parse()`, `check()`, `lex_source()` that return `Result`.

- **Document the EBNF-to-parser mapping** — each parser function should reference the grammar rule it implements (e.g., `// Grammar: statement ::= STORE ...`).

---

## Summary

Round 1 delivers a working end-to-end pipeline: lex → parse → type-check (basic) → HIR lowering. The parser handles the full MVP grammar and all 10 conformance fixtures pass. The most critical gap is parser error recovery (#1) and the match body statement/expression ambiguity (#2/#4). The type checker is shallow — it catches duplicates but doesn't resolve references or check type compatibility. These are natural targets for Round 2.

**Verdict:** Solid foundation. 249 tests passing. Ready to build on.
