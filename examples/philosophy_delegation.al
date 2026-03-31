// ============================================================
// AgentLang Philosophy: Agent-Native Coordination
// ============================================================
//
// Delegation is a language primitive, not a library call.
// When Agent A delegates to Agent B, B executes with B's own
// capabilities — never A's. This prevents capability smuggling.
//
// A high-privilege agent cannot trick a low-privilege agent
// into performing forbidden operations. The delegation boundary
// is a security boundary enforced by the language.
//
// Trust levels quantify confidence: a low-trust agent's output
// is automatically treated with more skepticism.
//
// Expected result: 25

AGENT Orchestrator =>
  CAPABILITIES [delegate, API_CALL]
  TRUST_LEVEL ~0.95

AGENT ComputeWorker =>
  CAPABILITIES [DB_READ]
  TRUST_LEVEL ~0.8

OPERATION compute_square =>
  INPUT x: Int64
  BODY {
    EMIT x * x
  }

OPERATION coordinate =>
  BODY {
    // Orchestrator delegates to ComputeWorker
    // ComputeWorker runs with its own caps [DB_READ], not Orchestrator's
    DELEGATE compute_square TO ComputeWorker => {
      INPUT 5
    }
    EMIT compute_square_result
  }

PIPELINE Main => coordinate
