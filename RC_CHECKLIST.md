# RC Checklist — AgentLang v0.1.0-rc1

**Date:** 2026-02-24
**Profile:** MVP v0.1

---

## Pre-Release Validation

### Build & Test

- [x] `cargo build --workspace` — compiles without errors
- [x] `cargo test --workspace` — 484 tests, 0 failures
- [x] `cargo test -p al-conformance --test conformance` — 45 conformance tests, 0 failures
- [x] `cargo test -p al-stdlib-mvp` — 19 signature-lock tests pass
- [x] `cargo test -p al-diagnostics -- audit` — 3 audit schema tests pass
- [x] `cargo build -p al-cli --release` — release binary builds

### CLI Smoke Tests

- [x] `al-cli lex examples/calculate.al` — 55 tokens, OK
- [x] `al-cli parse examples/factorial.al` — 3 declarations, OK
- [x] `al-cli check examples/match_result.al` — type check passed
- [x] `al-cli run examples/calculate.al --format human` — Result: 94
- [x] `al-cli run examples/factorial.al --format json` — Result: 720
- [x] `al-cli run examples/match_result.al --format jsonl` — Result: 84

### Conformance

- [x] C1-C10 core requirements: ALL PASS
- [x] C11-C14 extended positive: ALL PASS
- [x] C15-C20 negative conformance: ALL PASS
- [x] `all_fixtures_conform` meta-test: PASS
- [x] Conformance matrix generated (markdown + JSON)

### Documentation

- [x] `README.md` — quick-start, language overview, architecture
- [x] `KNOWN_LIMITATIONS.md` — 13 documented limitations
- [x] `RELEASE_NOTES_v0.1.0-rc1.md` — full release notes
- [x] `CONFORMANCE_MATRIX.md` — conformance matrix with test mappings
- [x] `conformance_matrix.json` — machine-readable conformance data
- [x] `RC_CHECKLIST.md` — this checklist
- [x] `RELEASE_MANIFEST.md` — artifact inventory with checksums

### CI/CD

- [x] `.github/workflows/ci.yml` — 9 gates, conformance label updated to C1-C20
- [x] All CI gates documented in README

### Release Metadata

- [x] `Cargo.toml` workspace version: `0.1.0`
- [x] License: MIT
- [x] MSRV: 1.75.0
- [x] Git tag: `v0.1.0-rc1` (annotated)

---

## Deferred Backlog (Not in RC Scope)

| Item | Priority | Notes |
|------|----------|-------|
| Real SMT solver (Z3/CVC5) | P2 | Stub solver is fail-safe |
| Concurrent fork-join | P3 | Sequential is correct |
| REPL / LSP | P2 | CLI-only for RC |
| Cross-compiled binaries | P2 | Source-only for RC |
| BEST_EFFORT / PARTIAL join | P3 | Excluded by design |
| Reactive semantics | P3 | Excluded by design |
| Full polymorphic inference | P3 | Monomorphic sufficient for MVP |
| Performance optimization | P3 | Not a blocker |

---

## Sign-Off

| Gate | Status | Verified |
|------|--------|----------|
| All tests pass | PASS | 2026-02-24 |
| Conformance C1-C20 | PASS | 2026-02-24 |
| CLI smoke tests | PASS | 2026-02-24 |
| Docs complete | PASS | 2026-02-24 |
| Known limitations documented | PASS | 2026-02-24 |
| No open blockers | PASS | 2026-02-24 |

**RC Status: READY FOR TAG**
