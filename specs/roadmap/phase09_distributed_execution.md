# Phase 9: Beyond One Machine — Distributed Execution

**Principle:** Parallel by Default (distributed extension)
**Status:** Planned
**Depends on:** Phase 3 (concurrent runtime), Phase 5 (audit trails for distributed accountability)

---

## 1. Overview

The dataflow DAG model was designed from the start to support distribution. This phase extends the single-machine runtime to transparent multi-node execution — pipeline stages and fork branches automatically distribute across machines based on dependency analysis, data locality, and resource availability.

## 2. Requirements

### 2.1 Distributed Dataflow Scheduling

- **R1.1:** Runtime analyzes pipeline DAG for stages that can execute on different nodes.
- **R1.2:** Independent stages (no data dependency) scheduled on separate nodes when available.
- **R1.3:** Data locality optimization: schedule computation close to data.
- **R1.4:** Network cost modeling: distribution only when benefit > communication overhead.
- **R1.5:** Transparent to program source: same AgentLang code runs local or distributed.

### 2.2 Node Management

- **R2.1:** Node discovery: `al-cluster` configuration listing available nodes.
- **R2.2:** Node health monitoring: heartbeat-based failure detection.
- **R2.3:** Node capabilities: each node declares available capabilities and resources.
- **R2.4:** Node selection: scheduler matches operation requirements to node capabilities.

### 2.3 Cross-Machine Checkpoint/Resume

- **R3.1:** Distributed snapshot using Chandy-Lamport algorithm (or equivalent).
- **R3.2:** In-flight messages captured as part of snapshot.
- **R3.3:** Checkpoint stored to shared persistent storage (configurable: filesystem, S3, etc.).
- **R3.4:** Resume from checkpoint on any node with sufficient capabilities.
- **R3.5:** Consistency guarantee: snapshot represents a consistent global state.

### 2.4 Agent Placement

- **R4.1:** Agents placed on nodes matching their capability requirements.
- **R4.2:** DELEGATE across node boundaries is transparent.
- **R4.3:** Agent migration: agent can be moved between nodes (via checkpoint/resume).
- **R4.4:** Failure recovery: agent on failed node restarted on healthy node from checkpoint.

### 2.5 Distributed Capability Enforcement

- **R5.1:** Capability checks enforced at network boundaries (not just local runtime).
- **R5.2:** Delegation messages carry capability attestation.
- **R5.3:** Audit trail synchronized across nodes (merge by timestamp, resolve conflicts).
- **R5.4:** No node can grant capabilities it doesn't possess.

### 2.6 Communication Protocol

- **R6.1:** Binary protocol for inter-node value serialization (CAS hash-based).
- **R6.2:** Values transferred by hash when remote node already has the content (deduplication).
- **R6.3:** Encryption in transit (TLS for all inter-node communication).
- **R6.4:** Authentication: nodes authenticate via shared secret or certificate.

## 3. Architecture

### 3.1 New Crates

**`al-cluster`:**
- Node discovery and configuration
- Health monitoring (heartbeat)
- Node capability registry

**`al-distributed`:**
- Distributed scheduler (extends local scheduler)
- DAG partitioning algorithm
- Cross-node communication protocol
- Distributed checkpoint coordinator

### 3.2 Crate Changes

**`al-runtime`:**
- Scheduler gains distributed mode (delegates to `al-distributed`)
- Value serialization for network transfer
- Remote operation invocation

**`al-checkpoint`:**
- Distributed snapshot protocol
- Shared storage backends

**`al-diagnostics`:**
- Distributed audit trail merge
- Node ID in audit events

**`al-cli`:**
- `--cluster config.toml` flag for distributed mode
- `al cluster status` subcommand
- `al cluster nodes` subcommand

## 4. Testing

### 4.1 Unit Tests — Scheduler (`al-distributed`)

| ID | Test | Description |
|----|------|-------------|
| T1.1 | `test_dag_partition_independent` | Two independent stages → partitioned to separate nodes |
| T1.2 | `test_dag_partition_dependent` | Dependent stages → same node or ordered across nodes |
| T1.3 | `test_data_locality` | Stage scheduled on node holding input data |
| T1.4 | `test_network_cost_model` | High network cost → prefer local execution |
| T1.5 | `test_node_capability_match` | Operation requiring DB_READ → node with DB_READ |
| T1.6 | `test_fallback_local` | No distributed nodes → local execution (backward compat) |

### 4.2 Unit Tests — Node Management (`al-cluster`)

| ID | Test | Description |
|----|------|-------------|
| T2.1 | `test_node_discovery` | Cluster config lists nodes correctly |
| T2.2 | `test_node_health_check` | Healthy node responds to heartbeat |
| T2.3 | `test_node_failure_detection` | Missing heartbeat → node marked unhealthy |
| T2.4 | `test_node_capability_registry` | Node capabilities queryable |

### 4.3 Unit Tests — Distributed Checkpoint (`al-checkpoint`)

| ID | Test | Description |
|----|------|-------------|
| T3.1 | `test_distributed_snapshot` | Multi-node state captured consistently |
| T3.2 | `test_inflight_message_capture` | Messages in transit included in snapshot |
| T3.3 | `test_resume_on_different_node` | Checkpoint restored on different node |
| T3.4 | `test_shared_storage_roundtrip` | Checkpoint saved/loaded from shared storage |

### 4.4 Unit Tests — Communication (`al-distributed`)

| ID | Test | Description |
|----|------|-------------|
| T4.1 | `test_value_serialization` | Value serializes/deserializes across nodes |
| T4.2 | `test_hash_deduplication` | Known value transferred by hash only (no body) |
| T4.3 | `test_capability_attestation` | Delegation message carries capability proof |
| T4.4 | `test_tls_required` | Non-TLS connection rejected |

### 4.5 Unit Tests — Agent Placement (`al-distributed`)

| ID | Test | Description |
|----|------|-------------|
| T5.1 | `test_agent_placement` | Agent placed on node matching capabilities |
| T5.2 | `test_delegate_cross_node` | DELEGATE to agent on different node → transparent |
| T5.3 | `test_agent_migration` | Agent migrated via checkpoint to new node |
| T5.4 | `test_failure_recovery` | Agent on failed node restarted on healthy node |

### 4.6 Integration Tests

| ID | Test | Description |
|----|------|-------------|
| T6.1 | `test_distributed_pipeline_e2e` | Pipeline stages execute on multiple nodes |
| T6.2 | `test_distributed_fork_e2e` | FORK branches execute on separate nodes |
| T6.3 | `test_node_failure_recovery_e2e` | Node failure → automatic recovery from checkpoint |
| T6.4 | `test_distributed_audit_merge` | Audit trails from multiple nodes merge correctly |

### 4.7 Conformance Tests

| ID | Test | Description |
|----|------|-------------|
| C53 | `conformance_distributed_execution` | Same program produces same results locally and distributed |
| C54 | `conformance_distributed_checkpoint` | Distributed checkpoint/resume is consistent |
| C55 | `conformance_distributed_capabilities` | Capabilities enforced across node boundaries |
| C56 | `conformance_distributed_audit` | Audit trail is complete across nodes |

## 5. Acceptance Criteria

- [ ] Pipeline stages distribute across nodes transparently
- [ ] Data locality optimization reduces network transfer
- [ ] Distributed checkpoint produces consistent global snapshot
- [ ] Agent placement respects capability requirements
- [ ] Capability enforcement works across node boundaries
- [ ] Node failure triggers automatic recovery from checkpoint
- [ ] Same program produces identical results locally and distributed
- [ ] All existing tests pass in single-node mode
- [ ] 4 new conformance tests (C53-C56) pass
