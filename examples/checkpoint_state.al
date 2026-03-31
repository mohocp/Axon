// AgentLang Example: Checkpoint for Fault Tolerance
//
// Demonstrates: CHECKPOINT to save runtime state, state preservation.
// Expected result: 42

OPERATION compute_with_checkpoint =>
  BODY {
    STORE phase1 = 20
    CHECKPOINT "after_phase1"
    STORE phase2 = phase1 + 22
    EMIT phase2
  }

PIPELINE Main => compute_with_checkpoint
