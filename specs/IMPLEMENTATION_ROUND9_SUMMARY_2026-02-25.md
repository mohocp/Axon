# Implementation Round 9 Summary — CLI Distribution

**Date:** 2026-02-25
**Scope:** CLI distribution, binary naming, release automation

---

## Milestones

### M1. CLI Binary Rename (al-cli → al)
- Added `[[bin]]` section with `name = "al"` to `crates/al-cli/Cargo.toml`
- Updated all usage strings and help text to `al` command
- Updated integration tests to use `CARGO_BIN_EXE_al`
- Updated README with `al` as canonical command
- Updated KNOWN_LIMITATIONS L8 (resolved)

### M2. Release Automation
- Created `.github/workflows/release.yml`
- Builds on tag push (`v*`) for 4 targets:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu` (via cross)
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`
- SHA-256 checksums generated per artifact
- GitHub Release created with attached binaries

### M3. Install Script
- Created `install.sh` (POSIX sh compatible)
- Auto-detects platform (Linux/macOS) and architecture (x86_64/aarch64)
- Downloads from GitHub Releases, verifies SHA-256 checksum
- Configurable: `AL_VERSION`, `AL_INSTALL` env vars

### M4. Packaging Documentation
- Created `specs/CLI_DISTRIBUTION.md`:
  - Quick-start install (no Cargo required)
  - Migration from `cargo run` to `al`
  - Supported platforms matrix
  - Release automation docs
  - CLI command reference

### M5. Smoke Tests
- 28 new smoke tests in `crates/al-cli/tests/smoke_tests.rs`
- Covers: help output, all 4 subcommands, --format flag, exit codes, example files, missing-file handling

---

## Metrics

| Metric | Before | After |
|--------|--------|-------|
| Total tests | 484 | 512 |
| CI workflows | 1 (ci.yml) | 2 (ci.yml + release.yml) |
| Supported platforms | 0 (source-only) | 4 (Linux/macOS × x86_64/aarch64) |
| Binary name | al-cli | al |
| Install methods | cargo build | install.sh, GitHub Releases, cargo install |
| Known limitations resolved | 0 | 1 (L8) |

---

## Files Changed

| File | Change |
|------|--------|
| `crates/al-cli/Cargo.toml` | Added `[[bin]]` section |
| `crates/al-cli/src/main.rs` | Updated usage strings |
| `crates/al-cli/tests/cli_integration.rs` | Updated binary reference |
| `crates/al-cli/tests/smoke_tests.rs` | New: 28 smoke tests |
| `.github/workflows/release.yml` | New: release automation |
| `install.sh` | New: platform installer |
| `specs/CLI_DISTRIBUTION.md` | New: packaging documentation |
| `README.md` | Updated CLI commands + install section |
| `KNOWN_LIMITATIONS.md` | L8 marked resolved |
