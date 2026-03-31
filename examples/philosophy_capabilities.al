// ============================================================
// AgentLang Philosophy: Constrain, Don't Trust
// ============================================================
//
// Every agent declares EXACTLY what it can do.
// CAPABILITIES lists permissions. DENY lists prohibitions.
// TRUST_LEVEL quantifies how much its output should be trusted.
//
// This is the principle of least privilege at the language level.
// An agent cannot acquire capabilities it wasn't born with.
// An agent's trust level attenuates the confidence of its output.
//
// No ambient authority. No privilege escalation. No exceptions.
//
// Expected result: 42

AGENT DataReader =>
  CAPABILITIES [DB_READ, API_CALL]
  DENY [DB_WRITE, FILE_WRITE, SELF_MODIFY]
  TRUST_LEVEL ~0.9

AGENT Analyst =>
  CAPABILITIES [LLM_INFER, MEMORY_READ]
  DENY [DB_WRITE, NETWORK_RAW, AGENT_SPAWN]
  TRUST_LEVEL ~0.85

AGENT Admin =>
  CAPABILITIES [DB_READ, DB_WRITE, FILE_READ, FILE_WRITE]
  DENY [SELF_MODIFY, AGENT_SPAWN]
  TRUST_LEVEL ~0.95

OPERATION read_data =>
  BODY {
    EMIT 21
  }

OPERATION analyze =>
  INPUT data: Int64
  BODY {
    EMIT data * 2
  }

PIPELINE Main => read_data -> analyze
