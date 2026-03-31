# Phase 3: Parallelism Earns Its Name — Concurrent Runtime

**Principle:** Parallel by Default
**Status:** Planned
**Depends on:** Phase 1 (verification of concurrent correctness)

---

## 1. Overview

The MVP executes FORK branches sequentially. The spec promises "runtime analyzes the dependency graph and automatically schedules independent operations on available compute resources." This phase replaces sequential execution with real concurrency and implements BEST_EFFORT and PARTIAL join strategies.

## 2. Requirements

### 2.1 Concurrent Fork/Join

- **R1.1:** FORK branches execute concurrently using a thread pool (Tokio or Rayon).
- **R1.2:** ALL_COMPLETE strategy: wait for all branches, collect results in declaration order.
- **R1.3:** Branch failures are collected; all branches run to completion before join evaluates.
- **R1.4:** Thread pool size is configurable (default: number of CPU cores).
- **R1.5:** Per-branch timeout support: branch that exceeds timeout is cancelled.

### 2.2 BEST_EFFORT Join Strategy

- **R2.1:** `JOIN strategy: BEST_EFFORT` accepts whatever branches complete within a timeout.
- **R2.2:** Syntax: `JOIN strategy: BEST_EFFORT TIMEOUT 10s`.
- **R2.3:** Result is a `List` of completed branch results (may be partial).
- **R2.4:** Timed-out branches are cancelled; their partial work is discarded.
- **R2.5:** If zero branches complete, result is `FAILURE("TIMEOUT", "no branches completed", details)`.

### 2.3 PARTIAL Join Strategy

- **R3.1:** `JOIN strategy: PARTIAL(min: k)` requires at least k branches to succeed.
- **R3.2:** If fewer than k branches succeed, result is `FAILURE("PARTIAL_FAILED", ...)`.
- **R3.3:** Once k branches complete, remaining branches are allowed to finish (not cancelled) but the join proceeds.
- **R3.4:** Optional timeout: `PARTIAL(min: 2) TIMEOUT 10s`.

### 2.4 Dataflow DAG Parallelism

- **R4.1:** Pipeline stages with no data dependency execute concurrently.
- **R4.2:** Dependency analysis at compile time: type checker annotates which stages are independent.
- **R4.3:** Runtime scheduler reads annotations and schedules independent stages in parallel.

### 2.5 Resource-Aware Scheduling

- **R5.1:** Agent `MAX_CONCURRENCY` limits the number of concurrent operations per agent.
- **R5.2:** Agent `MEMORY_LIMIT` triggers backpressure when approached.
- **R5.3:** `TIMEOUT_DEFAULT` applies as per-operation timeout unless overridden.

## 3. Architecture

### 3.1 Crate Changes

**`al-runtime`:**
- New `Scheduler` module with thread pool management
- `execute_fork` refactored: sequential → concurrent (feature-gated for backward compat)
- New `JoinStrategy` runtime handling for BEST_EFFORT and PARTIAL
- Branch cancellation via tokio::CancellationToken or std::sync channels
- Result collection with timeout

**`al-ast`:**
- `JoinStrategy` enum extended: `AllComplete`, `BestEffort { timeout }`, `Partial { min, timeout }`

**`al-lexer` / `al-parser`:**
- Parse `BEST_EFFORT`, `PARTIAL(min: N)`, `TIMEOUT` in JOIN context

**`al-types`:**
- Type of BEST_EFFORT join result: `List[Result[T]]` (partial results)
- Type of PARTIAL join result: `List[T]` (at least min results)

## 4. Testing

### 4.1 Unit Tests — Runtime (`al-runtime`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_fork_concurrent_execution` | FORK with 3 branches executes concurrently (wall-clock < sum of branch times) |
| T1.2 | `test_fork_all_complete` | ALL_COMPLETE waits for all branches, returns all results |
| T1.3 | `test_fork_all_complete_with_failure` | One branch fails → failure collected, other branches complete |
| T1.4 | `test_fork_best_effort_all_succeed` | BEST_EFFORT with all branches completing → all results returned |
| T1.5 | `test_fork_best_effort_timeout` | BEST_EFFORT with timeout: slow branch cancelled, fast branches returned |
| T1.6 | `test_fork_best_effort_none_complete` | BEST_EFFORT with very short timeout → FAILURE result |
| T1.7 | `test_fork_partial_min_met` | PARTIAL(min: 2) with 3 branches, 2 succeed → success |
| T1.8 | `test_fork_partial_min_not_met` | PARTIAL(min: 3) with 3 branches, 1 fails → FAILURE |
| T1.9 | `test_fork_partial_timeout` | PARTIAL with timeout: min met before timeout → success |
| T1.10 | `test_fork_branch_timeout` | Per-branch timeout: slow branch cancelled, others continue |
| T1.11 | `test_fork_result_order` | Results returned in branch declaration order, not completion order |
| T1.12 | `test_max_concurrency_respected` | Agent with MAX_CONCURRENCY 2: only 2 branches run simultaneously |

### 4.2 Unit Tests — Scheduler (`al-runtime`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_scheduler_thread_pool` | Thread pool spawns correct number of workers |
| T2.2 | `test_scheduler_cancellation` | Cancelled task stops execution promptly |
| T2.3 | `test_scheduler_backpressure` | Memory limit triggers backpressure (pauses new tasks) |

### 4.3 Unit Tests — Parser (`al-parser`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_parse_best_effort` | `JOIN strategy: BEST_EFFORT TIMEOUT 10s` parses correctly |
| T3.2 | `test_parse_partial` | `JOIN strategy: PARTIAL(min: 2)` parses correctly |
| T3.3 | `test_parse_partial_timeout` | `JOIN strategy: PARTIAL(min: 2) TIMEOUT 5s` parses correctly |

### 4.4 Integration Tests

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_concurrent_fork_e2e` | Full program with FORK/JOIN executes concurrently |
| T4.2 | `test_best_effort_e2e` | BEST_EFFORT program returns partial results on timeout |
| T4.3 | `test_partial_e2e` | PARTIAL program succeeds when min met |
| T4.4 | `test_backward_compat_sequential` | ALL_COMPLETE behavior identical to MVP when run sequentially |

### 4.5 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C29 | `conformance_concurrent_fork` | FORK branches execute concurrently |
| C30 | `conformance_best_effort` | BEST_EFFORT returns partial results |
| C31 | `conformance_partial_join` | PARTIAL(min: k) enforces minimum branch completion |
| C32 | `conformance_branch_cancellation` | Timed-out branches are cleanly cancelled |

## 5. Acceptance Criteria

- [ ] FORK branches execute concurrently (measurable by wall-clock time)
- [ ] BEST_EFFORT and PARTIAL join strategies are lexed, parsed, type-checked, and executed
- [ ] Branch cancellation is clean (no resource leaks)
- [ ] MAX_CONCURRENCY is respected per agent
- [ ] All existing tests pass (backward compatibility with sequential fallback)
- [ ] 4 new conformance tests (C29-C32) pass
