# AgentLang Implementation Round 6 Summary — 2026-02-24

## Slice 1: stdlib + checkpoint/audit track (MVP-first scope)

### Completed Items

#### 1. Core stdlib data operations: FILTER, MAP, REDUCE
- Implemented as built-in operations recognized by the interpreter before
  falling through to user-defined operations.
- **FILTER(list, predicate_op_name)**: Calls a named predicate operation for
  each element; keeps those returning `Bool(true)`.
- **MAP(list, transform_op_name)**: Calls a named transform operation for each
  element; collects results.
- **REDUCE(list, initial, reducer_op_name)**: Folds a list by calling
  `reducer(accumulator, element)` for each element.
- All three ops are pure (non-fallible) `core.data` module operations.
- Type errors for non-list first arguments propagate as `InterpreterError::TypeError`.

#### 2. STDLIB_MVP_SIGNATURES.json and consistency checks
- Created `crates/al-stdlib-mvp/STDLIB_MVP_SIGNATURES.json` with canonical
  type signatures for FILTER, MAP, REDUCE.
- Added `SignaturesFile`, `OpSignature`, `SigParam` serde types to `al-stdlib-mvp`.
- Added `load_signatures()` using `include_str!` for compile-time embedding.
- **8 signature-lock tests** verify:
  - File parses correctly.
  - All implemented ops present in signatures.
  - All ops belong to `core.data` module.
  - All core.data ops marked non-fallible.
  - FILTER / MAP / REDUCE input/output shapes match specification.
  - Cross-consistency between signatures file and module registry.

#### 3. Audit JSONL emission for runtime events
- Added 3 new `AuditEventType` variants to `al-diagnostics`:
  - `OperationCalled` — emitted when a user-defined operation is invoked.
  - `PipelineStarted` — emitted when a pipeline chain begins execution.
  - `StdlibCall` — emitted when a built-in stdlib operation is invoked.
- Added `Runtime::emit_audit_event()` (public) and `Runtime::audit_to_jsonl()`
  methods to `al-runtime`.
- Wired audit emission into the interpreter:
  - `exec_pipeline_chain` emits `PIPELINE_STARTED` with stage count.
  - `call_operation` emits `OPERATION_CALLED` with operation name.
  - `call_stdlib_builtin` emits `STDLIB_CALL` with operation name.

#### 4. Test coverage
- **26 new tests** (387 total, up from 361), all passing:
  - 13 stdlib behavior tests (FILTER, MAP, REDUCE: normal, empty, no-match,
    type-error, composition).
  - 8 signature-lock tests in `al-stdlib-mvp`.
  - 5 audit schema tests (PIPELINE_STARTED, OPERATION_CALLED, STDLIB_CALL,
    JSONL format validation, schema field completeness).
- Updated existing audit event serialization test for new variants.

#### 5. Existing suite remains green
- `cargo test -q`: 387/387 passing, 0 failures.
- `cargo check`: clean, no warnings.

### Deferred Items (for later slices)
- **SORT, GROUP, TAKE, SKIP** stdlib ops (registered in module but not yet
  implemented at runtime).
- **core.io, core.text, core.http, agent.llm, agent.memory** module
  implementations (fallible ops requiring external capabilities).
- **Audit JSONL file output**: Currently in-memory `Vec<AuditEvent>`; writing
  to disk JSONL file deferred.
- **Checkpoint resume expression evaluation** (AST only in Round 5).
- **Constructor pattern matching** in MATCH arms.

### Files Changed

| File | Change |
|------|--------|
| `crates/al-diagnostics/src/lib.rs` | +3 AuditEventType variants, updated Display + test |
| `crates/al-runtime/src/interpreter.rs` | +FILTER/MAP/REDUCE builtins, +audit emission, +26 tests |
| `crates/al-runtime/src/lib.rs` | +emit_audit_event(), +audit_to_jsonl() |
| `crates/al-stdlib-mvp/src/lib.rs` | +signature types, +load_signatures(), +8 tests |
| `crates/al-stdlib-mvp/Cargo.toml` | +serde, serde_json deps |
| `crates/al-stdlib-mvp/STDLIB_MVP_SIGNATURES.json` | New: canonical signatures |
| `specs/IMPLEMENTATION_ROUND6_SUMMARY_2026-02-24.md` | New: this file |

### Metrics
- **Tests**: 387 (was 361, +26)
- **New code**: ~350 lines (implementation + tests)
- **Build**: clean (`cargo check` + `cargo test -q`)
