# Release Notes — AgentLang v0.1.0-rc3

**Date:** 2026-02-25
**Profile:** MVP v0.1
**Tag:** `v0.1.0-rc3`

---

## Overview

Hotfix release addressing CLI flag parsing and machine-readable output correctness. All changes are backwards-compatible with rc1.

## Changes

### Bug Fixes

- **`--help` / `-h` flag**: Now correctly shows usage and exits 0 instead of being misparsed as a file path.
- **`--version` / `-V` flag**: Now correctly prints `al <version>` and exits 0 instead of being misparsed as a file path.
- **JSON/JSONL output on success paths**: `--format json` and `--format jsonl` now emit true machine-parseable JSON for all commands (`lex`, `parse`, `check`, `run`), not just for error diagnostics.
- **JSON/JSONL output on runtime errors**: `al run --format json` now emits structured JSON on stderr for runtime failures (e.g., ESCALATE), not human text.

### Output Schema (JSON mode)

Success responses follow this schema:

```json
{
  "status": "ok",
  "command": "<lex|parse|check|run>",
  ...command-specific fields...
}
```

Error responses (runtime failures):

```json
{
  "status": "error",
  "command": "run",
  "phase": "exec",
  "message": "<error description>"
}
```

Compile-time diagnostics retain the existing `Diagnostic` schema with `code`, `severity`, `span`, `message`, `profile`, and `notes` fields.

### New Tests (12 tests added)

- `smoke_help_flag_long` — `--help` exits 0 with usage
- `smoke_help_flag_short` — `-h` exits 0 with usage
- `smoke_help_with_command_still_shows_help` — `--help` takes priority over commands
- `smoke_version_flag_long` — `--version` exits 0 with version string
- `smoke_version_flag_short` — `-V` exits 0 with version string
- `smoke_check_json_success_is_valid_json` — `check --format json` on success
- `smoke_check_jsonl_success_is_valid_jsonl` — `check --format jsonl` on success
- `smoke_check_json_error_is_valid_json` — `check --format json` on error
- `smoke_check_jsonl_error_is_valid_jsonl` — `check --format jsonl` on error
- `smoke_run_json_success_is_valid_json` — `run --format json` on success
- `smoke_run_jsonl_success_is_valid_jsonl` — `run --format jsonl` on success
- `smoke_run_json_error_is_valid_json` — `run --format json` on runtime error
- `smoke_run_jsonl_error_is_valid_jsonl` — `run --format jsonl` on runtime error
- `smoke_run_json_result_contains_value` — `run --format json` result value correctness

### Other

- Added `serde_json` dependency to `al-cli` crate for structured output serialization.
- Usage help text now documents `--help`, `--version`, and `--format` options.

## Test Summary

| Category | Count |
|----------|-------|
| Total workspace tests | 498+ |
| New rc3 tests | 14 |
| CLI smoke tests | 42 |
| CLI integration tests | 15 |
| Conformance tests (C1-C20) | 45 |

## Upgrade from rc1

Drop-in replacement. No breaking changes. Machine-parseable output is new behavior for success paths — human format remains the default.
