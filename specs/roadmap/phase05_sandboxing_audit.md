# Phase 5: Constrain, Don't Trust — Sandboxing and Audit

**Principle:** Trust, Safety, Accountability
**Status:** Planned
**Depends on:** Phase 1 (verified non-escalation), Phase 4 (real backends to sandbox)

---

## 1. Overview

The MVP has 22 capabilities enforced at runtime and JSONL audit to stdout. This phase adds the enforcement layers the spec envisions: SANDBOX resource isolation, resource budgets, cryptographic audit trails with hash chains, and human escalation workflows. The goal: untrusted agents are mechanically constrained, not politely asked.

## 2. Requirements

### 2.1 SANDBOX Construct

- **R1.1:** `SANDBOX name => { MEMORY_LIMIT, CPU_LIMIT, NETWORK, FILESYSTEM, MAX_DURATION }` is a new declaration.
- **R1.2:** `MEMORY_LIMIT` — hard memory ceiling (e.g., `512MB`). Exceeding triggers termination.
- **R1.3:** `CPU_LIMIT` — core allocation (e.g., `2 cores`). Enforced via thread affinity or cgroup.
- **R1.4:** `NETWORK` — allowlist: `restricted_to: [domain1, domain2]` or `NONE`.
- **R1.5:** `FILESYSTEM` — path-scoped access or `NONE`. Enforced before every file operation.
- **R1.6:** `MAX_DURATION` — hard timeout. Exceeding triggers termination with audit event.
- **R1.7:** On any limit exceeded: `TERMINATE -> LOG -> ALERT` failure chain.
- **R1.8:** Operations run inside a sandbox inherit its constraints. Nested sandboxes take the intersection (stricter) of limits.

### 2.2 Resource Budget Tracking

- **R2.1:** `BUDGET` scope: `BUDGET name => { LLM_TOKENS max: 10000, API_CALLS max: 100, WALL_TIME max: 5m }`.
- **R2.2:** Each operation within budget scope decrements the relevant counter.
- **R2.3:** Budget exhaustion triggers degradation policy (configurable: ABORT, ESCALATE, FALLBACK).
- **R2.4:** Budget usage queryable at runtime: `budget.remaining.llm_tokens`.
- **R2.5:** Budget is per-scope, not global. Nested budgets take the minimum.

### 2.3 Cryptographic Audit Trail

- **R3.1:** Each audit entry includes SHA256 hash of its content.
- **R3.2:** Each entry includes hash of the previous entry, forming a chain.
- **R3.3:** Chain root is a known genesis hash (deterministic).
- **R3.4:** Any modification to an entry breaks the chain (detectable).
- **R3.5:** Audit entries include proof hashes linking to VC results.
- **R3.6:** Pluggable audit sinks: stdout (default), file, HTTP endpoint.
- **R3.7:** Audit sink configured via `--audit-sink` CLI flag.

### 2.4 Human Escalation Workflows

- **R4.1:** `REQUIRE HUMAN_APPROVAL` blocks execution until approval received.
- **R4.2:** Approval request includes: approver role, timeout, evidence payload.
- **R4.3:** Evidence: `{ reason, impact_analysis, rollback_plan }`.
- **R4.4:** `ON_TIMEOUT ABORT` — if approval not received within timeout, abort operation.
- **R4.5:** Approval decisions recorded in audit trail with approver identity.
- **R4.6:** `ESCALATE_HUMAN` capability required to trigger human approval.

### 2.5 Formal Non-Escalation Proof

- **R5.1:** At compile time, verify delegation chains cannot escalate capabilities.
- **R5.2:** At compile time, verify sandbox constraints cannot be bypassed through delegation.
- **R5.3:** SMT solver checks: for all delegation paths, callee caps ⊆ callee declared caps.

## 3. Architecture

### 3.1 Crate Changes

**New crate: `al-sandbox`**
- Sandbox configuration and enforcement
- Resource budget tracking
- Limit checking hooks for runtime

**`al-ast`:**
- New `Declaration::Sandbox` variant
- New `Declaration::Budget` variant
- New `Statement::RequireHumanApproval` variant

**`al-diagnostics`:**
- `AuditEvent` gains `hash` and `previous_hash` fields
- New `AuditChain` struct managing the hash chain
- Pluggable `AuditSink` trait

**`al-runtime`:**
- Sandbox enforcement layer wrapping operation execution
- Budget counter decremented on each operation
- Human approval blocking (with timeout)

**`al-cli`:**
- `--audit-sink stdout|file:path|http:url` flag
- `--sandbox-mode` flag for enabling/disabling enforcement

## 4. Testing

### 4.1 Unit Tests — Sandbox (`al-sandbox`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_sandbox_memory_limit` | Operation exceeding memory limit → terminated |
| T1.2 | `test_sandbox_max_duration` | Operation exceeding duration → terminated |
| T1.3 | `test_sandbox_network_allowlist` | HTTP to allowed domain → success; to blocked domain → denied |
| T1.4 | `test_sandbox_filesystem_none` | FILE_READ inside FILESYSTEM NONE sandbox → denied |
| T1.5 | `test_sandbox_filesystem_scoped` | File outside allowed path → denied; inside → allowed |
| T1.6 | `test_sandbox_nested_intersection` | Inner sandbox cannot exceed outer sandbox limits |
| T1.7 | `test_sandbox_termination_audit` | Terminated sandbox produces audit event |

### 4.2 Unit Tests — Budget (`al-sandbox`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_budget_llm_tokens` | Budget with 100 LLM tokens: 50 used → 50 remaining |
| T2.2 | `test_budget_exhausted` | Budget exhausted → configured degradation policy triggers |
| T2.3 | `test_budget_api_calls` | API call count tracked correctly |
| T2.4 | `test_budget_wall_time` | Wall time budget → ABORT after timeout |
| T2.5 | `test_budget_query_remaining` | `budget.remaining.llm_tokens` returns correct value |
| T2.6 | `test_budget_nested_minimum` | Inner budget ≤ outer budget |

### 4.3 Unit Tests — Audit Chain (`al-diagnostics`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_audit_chain_hash` | Each entry's hash is SHA256 of its content |
| T3.2 | `test_audit_chain_linked` | Each entry's `previous_hash` matches prior entry's hash |
| T3.3 | `test_audit_chain_genesis` | First entry has deterministic genesis hash |
| T3.4 | `test_audit_chain_tamper_detection` | Modifying an entry breaks chain validation |
| T3.5 | `test_audit_chain_proof_hash` | VC proof hash embedded in audit entry |
| T3.6 | `test_audit_sink_file` | File audit sink writes JSONL to disk |
| T3.7 | `test_audit_sink_stdout` | Stdout sink outputs to stderr (default behavior) |

### 4.4 Unit Tests — Human Escalation (`al-runtime`)

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_human_approval_blocks` | REQUIRE HUMAN_APPROVAL pauses execution |
| T4.2 | `test_human_approval_timeout_abort` | Timeout without approval → ABORT |
| T4.3 | `test_human_approval_evidence` | Evidence payload (reason, impact, rollback) included in request |
| T4.4 | `test_human_approval_audit` | Approval decision logged in audit trail |
| T4.5 | `test_human_approval_requires_capability` | ESCALATE_HUMAN capability required |

### 4.5 Unit Tests — Non-Escalation (`al-vc`)

| ID | Test | Description |
|----|------|-------------|
| T5.1 | `test_delegation_no_escalation` | DELEGATE to agent with fewer caps → proven safe |
| T5.2 | `test_delegation_escalation_detected` | Delegation chain attempting cap escalation → compile error |
| T5.3 | `test_sandbox_bypass_detected` | Delegation from sandbox to unsandboxed → compile error |

### 4.6 Integration Tests

| ID | Test | Description |
|----|------|-------------|
| T6.1 | `test_sandbox_e2e` | Agent in SANDBOX with FILESYSTEM NONE cannot read files |
| T6.2 | `test_budget_e2e` | Agent exhausts budget → graceful degradation |
| T6.3 | `test_audit_chain_e2e` | Full program produces valid hash chain in JSONL output |

### 4.7 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C37 | `conformance_sandbox_enforcement` | SANDBOX limits enforced at runtime |
| C38 | `conformance_audit_chain_integrity` | Audit hash chain is valid and tamper-detectable |
| C39 | `conformance_budget_tracking` | Resource budget correctly tracks usage |
| C40 | `conformance_human_escalation` | REQUIRE HUMAN_APPROVAL blocks and times out correctly |

## 5. Acceptance Criteria

- [ ] SANDBOX construct is lexed, parsed, type-checked, and enforced at runtime
- [ ] Resource budgets track LLM tokens, API calls, and wall time
- [ ] Audit trail is a cryptographic hash chain (SHA256)
- [ ] Tampered audit entry detectable by chain validation
- [ ] Human escalation blocks execution with timeout and evidence
- [ ] Delegation cannot escalate capabilities (proven by SMT solver)
- [ ] All existing tests pass unchanged
- [ ] 4 new conformance tests (C37-C40) pass
