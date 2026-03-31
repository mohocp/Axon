// ============================================================
// AgentLang Philosophy: Accountability Through Audit
// ============================================================
//
// Every state change, capability usage, and inter-agent
// communication is logged. The audit trail is not optional
// infrastructure — it is a language guarantee.
//
// CHECKPOINT creates auditable state snapshots.
// ASSERT creates auditable verification points.
// DELEGATE creates auditable capability transitions.
// FORK/JOIN creates auditable parallel execution records.
//
// The question is never "should we log this?" — every
// significant action is logged. Period.
//
// Expected result: [10, 20]

AGENT Auditor =>
  CAPABILITIES [DB_READ]
  TRUST_LEVEL ~0.99

OPERATION phase_one =>
  BODY {
    CHECKPOINT "phase_one_start"
    STORE data = 10
    ASSERT data GT 0
    EMIT data
  }

OPERATION phase_two =>
  BODY {
    CHECKPOINT "phase_two_start"
    STORE data = 20
    ASSERT data GT 0
    EMIT data
  }

OPERATION audited_workflow =>
  BODY {
    CHECKPOINT "workflow_start"
    STORE results = FORK { a: phase_one, b: phase_two } -> JOIN strategy: ALL_COMPLETE
    CHECKPOINT "workflow_complete"
    EMIT results
  }

PIPELINE Main => audited_workflow
