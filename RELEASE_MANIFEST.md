# Release Manifest — AgentLang v0.1.0-rc1

**Date:** 2026-02-24
**Tag:** `v0.1.0-rc1`
**Profile:** MVP v0.1

---

## Release Artifacts

### Documentation

| File | Description | SHA-256 |
|------|-------------|---------|
| `README.md` | Quick-start, language overview, architecture | `af291da82c67aa6e...3247e10c` |
| `KNOWN_LIMITATIONS.md` | 13 documented limitations | `a748f97994c2f2ed...9237453a` |
| `RELEASE_NOTES_v0.1.0-rc1.md` | Full release notes | `5096dd5bd6e140c2...2bfd265b` |
| `CONFORMANCE_MATRIX.md` | Conformance matrix (human-readable) | `4ac8e171906ef2db...8a07734` |
| `conformance_matrix.json` | Conformance matrix (machine-readable) | `9407dc0d3675da6d...ce06c90` |
| `RC_CHECKLIST.md` | RC validation checklist | `4c3d7d866f35f2fd...bfa6bf43` |
| `RELEASE_MANIFEST.md` | This file | (self) |

### Build Artifacts

| File | Description | SHA-256 |
|------|-------------|---------|
| `target/release/al-cli` | Release binary (macOS, local build) | `ce4f015e15f67cc3...7959ab0` |

### Configuration

| File | Description | SHA-256 |
|------|-------------|---------|
| `Cargo.toml` | Workspace root (version 0.1.0) | `a0629b520782d1e8...a99c2a7` |
| `Cargo.lock` | Locked dependencies | `664091b2f1bf6584...2538c6ae` |
| `.github/workflows/ci.yml` | CI pipeline (9 gates) | `fbb740c5a6fd847c...0682401` |

### Source Crates (13)

| Crate | Path | Description |
|-------|------|-------------|
| `al-ast` | `crates/al-ast/` | Abstract Syntax Tree |
| `al-lexer` | `crates/al-lexer/` | Lexer/tokenizer |
| `al-parser` | `crates/al-parser/` | Parser |
| `al-hir` | `crates/al-hir/` | High-Intermediate Representation |
| `al-types` | `crates/al-types/` | Type checker |
| `al-vc` | `crates/al-vc/` | Verification conditions |
| `al-capabilities` | `crates/al-capabilities/` | Capability system |
| `al-runtime` | `crates/al-runtime/` | Interpreter |
| `al-stdlib-mvp` | `crates/al-stdlib-mvp/` | Standard library |
| `al-checkpoint` | `crates/al-checkpoint/` | Checkpoint/resume |
| `al-diagnostics` | `crates/al-diagnostics/` | Diagnostics rendering |
| `al-conformance` | `crates/al-conformance/` | Conformance test suite |
| `al-cli` | `crates/al-cli/` | CLI binary |

### Example Programs

| File | Description | Expected Output |
|------|-------------|----------------|
| `examples/calculate.al` | Pipeline with arithmetic | Result: 94 |
| `examples/factorial.al` | Bounded loop factorial | Result: 720 |
| `examples/match_result.al` | Pattern matching + agents | Result: 84 |

### Specifications

| File | Description |
|------|-------------|
| `AgentLang_Specification_v1.0.md` | Full v1.0 specification |
| `specs/MVP_PROFILE.md` | MVP v0.1 normative profile |
| `specs/GRAMMAR_MVP.ebnf` | MVP parser grammar |
| `specs/formal_semantics.md` | Formal calculus and proofs |
| `specs/stdlib_spec_mvp.md` | MVP stdlib specification |
| `specs/CONFORMANCE_ACCEPTANCE_CRITERIA.md` | Conformance acceptance criteria |

---

## Verification

To verify the release locally:

```bash
# Build
cargo build --workspace

# Run all tests
cargo test --workspace

# Run conformance suite
cargo test -p al-conformance --test conformance -- --nocapture

# Smoke test CLI
cargo run -p al-cli -- run examples/calculate.al
cargo run -p al-cli -- run examples/factorial.al
cargo run -p al-cli -- run examples/match_result.al
```

Expected: 484 tests pass, 45 conformance tests pass, all examples produce correct results.
