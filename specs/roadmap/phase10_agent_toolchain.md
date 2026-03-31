# Phase 10: The Agent's Own Toolchain

**Principle:** Agents as Primary Developers
**Status:** Planned
**Depends on:** Phase 1 (verification for tooling), independently parallelizable with Phases 2-9

---

## 1. Overview

The spec states: "debugging tools are designed for programmatic consumption." This phase builds the development tools agents need: REPL for interactive exploration, LSP for IDE integration, structured debugging for programmatic analysis, a package manager for operation distribution, and incremental compilation for fast iteration.

## 2. Requirements

### 2.1 Interactive REPL

- **R1.1:** `al repl` starts an interactive session.
- **R1.2:** Evaluate expressions, statements, and declarations incrementally.
- **R1.3:** State persists across evaluations (variables, operations, agents available).
- **R1.4:** Structured output: `--format json` for agent consumption.
- **R1.5:** Capability scoping: REPL session has configurable capabilities.
- **R1.6:** History and recall of previous evaluations.
- **R1.7:** Load files into session: `:load file.al`.

### 2.2 Language Server Protocol (LSP)

- **R2.1:** Implement LSP for IDE integration (VS Code, JetBrains, etc.).
- **R2.2:** Code completion: suggest operations, types, capabilities, keywords.
- **R2.3:** Hover information: type, capability requirements, verification status.
- **R2.4:** Go-to-definition: navigate to operation, schema, agent declarations.
- **R2.5:** Real-time diagnostics: errors and warnings as-you-type.
- **R2.6:** VC status: inline indicators showing which conditions are proven.
- **R2.7:** Capability lens: show which capabilities an operation requires.

### 2.3 Structured Debugging

- **R3.1:** `TRACE PIPELINE` construct: trace execution with structured JSON output.
- **R3.2:** Capture options: inputs, outputs, intermediate values, timing, confidence changes.
- **R3.3:** Breakpoint support: `BREAKPOINT WHEN condition` in source.
- **R3.4:** Step-through: step by statement, by operation, by pipeline stage.
- **R3.5:** Value inspection: examine any value at any point in execution.
- **R3.6:** Time-travel: navigate forward and backward through checkpoint history.
- **R3.7:** All debug output in structured JSON for agent consumption.

### 2.4 Package Manager

- **R4.1:** `al pkg init` initializes a package.
- **R4.2:** `al pkg publish` publishes to registry.
- **R4.3:** `al pkg install name` installs a dependency.
- **R4.4:** Content-addressed packages (CAS-based: package identified by content hash).
- **R4.5:** Verified interfaces: packages carry proof certificates.
- **R4.6:** Dependency resolution with capability compatibility checking.
- **R4.7:** Package manifest: `PACKAGE { name, version, exports, requires, capabilities_needed }`.
- **R4.8:** Registry API for package discovery and search.

### 2.5 Incremental Compilation

- **R5.1:** File-level change detection (content hash comparison).
- **R5.2:** Cache intermediate representations: tokens, AST, HIR, type info.
- **R5.3:** Re-check only affected declarations when a file changes.
- **R5.4:** Watch mode: `al watch <dir>` continuously recompiles on file changes.
- **R5.5:** Cache invalidation: dependency graph determines what to re-check.

## 3. Architecture

### 3.1 New Crates

**`al-repl`:**
- Interactive session management
- Incremental state accumulation
- Expression evaluation loop

**`al-lsp`:**
- LSP protocol implementation
- Document synchronization
- Completion, hover, diagnostics providers
- VC status integration

**`al-debug`:**
- TRACE instrumentation
- Breakpoint management
- Step-through controller
- Time-travel via checkpoint history

**`al-pkg`:**
- Package manifest parsing
- Registry client
- Content-addressed storage
- Dependency resolver

### 3.2 Crate Changes

**`al-cli`:**
- `al repl` subcommand
- `al lsp` subcommand (starts LSP server)
- `al pkg init|publish|install` subcommands
- `al watch` subcommand

**`al-types`:**
- Incremental type checking API
- Cache layer for type environments

**`al-parser`:**
- Incremental parsing support
- Parse single declaration (not full program)

## 4. Testing

### 4.1 Unit Tests — REPL (`al-repl`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_repl_evaluate_expression` | `42 + 1` returns `43` |
| T1.2 | `test_repl_store_persist` | `STORE x = 42` then `x` returns `42` |
| T1.3 | `test_repl_operation_define` | Define then call operation across evaluations |
| T1.4 | `test_repl_json_output` | `--format json` returns structured output |
| T1.5 | `test_repl_load_file` | `:load` imports declarations from file |
| T1.6 | `test_repl_capability_scoping` | REPL session respects capability limits |
| T1.7 | `test_repl_error_recovery` | Error in one evaluation doesn't break session |

### 4.2 Unit Tests — LSP (`al-lsp`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_lsp_completion_keywords` | Typing "OP" suggests "OPERATION" |
| T2.2 | `test_lsp_completion_types` | After ":" suggests available types |
| T2.3 | `test_lsp_hover_type` | Hover on variable shows its type |
| T2.4 | `test_lsp_hover_capabilities` | Hover on operation shows required capabilities |
| T2.5 | `test_lsp_goto_definition` | Go-to-definition navigates to declaration |
| T2.6 | `test_lsp_diagnostics` | Real-time error shown on type mismatch |
| T2.7 | `test_lsp_vc_status` | Proven REQUIRE shows green indicator |

### 4.3 Unit Tests — Debug (`al-debug`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_trace_pipeline` | TRACE captures all pipeline stage inputs/outputs |
| T3.2 | `test_trace_json_output` | Trace output is valid structured JSON |
| T3.3 | `test_breakpoint_condition` | BREAKPOINT WHEN x GT 10 pauses at correct point |
| T3.4 | `test_step_by_statement` | Step advances one statement at a time |
| T3.5 | `test_value_inspection` | Inspect value at breakpoint |
| T3.6 | `test_time_travel_backward` | Navigate to previous checkpoint state |

### 4.4 Unit Tests — Package Manager (`al-pkg`)

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_pkg_init` | Creates package manifest with defaults |
| T4.2 | `test_pkg_content_hash` | Package identified by content hash |
| T4.3 | `test_pkg_dependency_resolve` | Dependencies resolved correctly |
| T4.4 | `test_pkg_capability_compat` | Package requiring DB_READ rejected if not available |
| T4.5 | `test_pkg_publish_roundtrip` | Publish then install retrieves same package |

### 4.5 Unit Tests — Incremental Compilation (`al-types`)

| ID | Test | Description |
|----|------|-------------|
| T5.1 | `test_incremental_unchanged` | Unchanged file → cached result used |
| T5.2 | `test_incremental_changed` | Changed declaration → re-checked |
| T5.3 | `test_incremental_dependency` | Changed dependency → dependents re-checked |
| T5.4 | `test_watch_mode` | File change triggers recompilation |

### 4.6 Integration Tests

| ID | Test | Description |
|----|------|-------------|
| T6.1 | `test_repl_e2e` | Full REPL session: define, compute, inspect |
| T6.2 | `test_lsp_e2e` | LSP server handles initialize, completion, shutdown |
| T6.3 | `test_debug_e2e` | Trace pipeline produces correct structured output |
| T6.4 | `test_pkg_e2e` | Init, publish, install workflow completes |

### 4.7 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C57 | `conformance_repl_session` | REPL session evaluates expressions correctly |
| C58 | `conformance_incremental` | Incremental compilation produces same results as full compilation |
| C59 | `conformance_debug_trace` | TRACE output matches actual execution |
| C60 | `conformance_package_verified` | Installed package carries valid proof certificates |

## 5. Acceptance Criteria

- [ ] `al repl` provides interactive AgentLang evaluation
- [ ] LSP server provides completion, hover, go-to-definition, diagnostics
- [ ] TRACE PIPELINE produces structured JSON debug output
- [ ] Package manager handles init, publish, install with content addressing
- [ ] Incremental compilation caches and re-uses intermediate representations
- [ ] Watch mode recompiles on file changes
- [ ] All output formats are agent-consumable (structured JSON)
- [ ] All existing tests pass unchanged
- [ ] 4 new conformance tests (C57-C60) pass
