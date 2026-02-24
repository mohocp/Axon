# AgentLang MVP — Implementation Round 3 Summary

**Date:** 2026-02-24  
**Profile:** mvp-0.1  
**Focus:** Build artifact hygiene, warning cleanup, and MVP pipeline type propagation.

## What Was Implemented

1. **Build artifact hygiene**
- Added root `.gitignore` with `/target/`.
- Verified no tracked build outputs under `target/` (`git ls-files | rg '^target/'` returns empty).

2. **Compile warning cleanup**
- Removed unused `DateTime` import in `al-diagnostics`.
- Made `Severity` import test-only in `al-capabilities`.
- Removed unused `Span` import in `al-checkpoint`.
- Result: `cargo check` runs clean with zero warnings.

3. **Pipeline type propagation (highest-priority unfinished MVP scope)**
- Extended `al-types` operation metadata to retain structured input/output `TypeExpr` values.
- Added **Pass 7** in type checking:
  - checks adjacent resolved operation stages in `PIPELINE` chains,
  - checks resolved operation stages in `FORK` branch chains,
  - emits deterministic `TYPE_MISMATCH` on incompatible output -> first-input chains,
  - preserves existing unresolved-stage behavior as warning-level (`UNRESOLVED_REFERENCE`).

## Tests Added

In `crates/al-types/src/lib.rs`:
- `pipeline_stage_type_mismatch_detected`
- `pipeline_stage_type_match_accepted`

## Validation

- `cargo check` ✅
- `cargo test` ✅

Current test total after this round: **267 passed, 0 failed**.

## Remaining MVP Work (next priority)

1. HIR enrichment (`ty` and `required_caps` population)
2. VC generation wiring (`ASSERT`/`REQUIRE` to `al-vc`)
3. End-to-end AST/HIR runtime interpretation path
4. Parse-time exclusion diagnostics for non-MVP constructs beyond current coverage
