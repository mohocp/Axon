// ============================================================
// AgentLang Philosophy: Fault Tolerance by Design
// ============================================================
//
// Agents fail. Networks drop. LLMs hallucinate. AgentLang
// builds fault tolerance into the language with:
//
// CHECKPOINT — save state at critical points for recovery
// MATCH on FAILURE — catch and handle errors explicitly
// HALT — explicit, auditable termination
//
// Failure is not exceptional. It is a first-class value
// (FAILURE(code, message, details)) that flows through
// pipelines, triggers pattern matching, and produces
// audit trail entries.
//
// Every failure is typed, structured, and traceable.
//
// Expected result: "safe default"

OPERATION unreliable_source =>
  INPUT mode: Int64
  BODY {
    MATCH mode EQ 0 => {
      WHEN TRUE -> { HALT(service_unavailable) }
      OTHERWISE -> { EMIT mode * 10 }
    }
  }

OPERATION resilient_processor =>
  BODY {
    // Save state before risky operation
    CHECKPOINT "before_call"

    // Call might fail — and that's OK
    STORE result = unreliable_source(0)

    // Handle failure explicitly — no exceptions, no surprises
    MATCH result => {
      WHEN FAILURE(code, msg, details) -> {
        EMIT "safe default"
      }
      OTHERWISE -> {
        EMIT result
      }
    }
  }

PIPELINE Main => resilient_processor
