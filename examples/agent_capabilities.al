// AgentLang Example: Agents with Capabilities
//
// Demonstrates: AGENT declaration, CAPABILITIES, DENY, TRUST_LEVEL.
// Expected result: 42

AGENT DataReader =>
  CAPABILITIES [FILE_READ, API_CALL]
  DENY [FILE_WRITE, DB_WRITE]
  TRUST_LEVEL ~0.9

AGENT Processor =>
  CAPABILITIES [DB_READ, DB_WRITE]
  TRUST_LEVEL ~0.85

OPERATION read_data =>
  BODY {
    EMIT 21
  }

OPERATION transform =>
  INPUT x: Int64
  BODY {
    EMIT x * 2
  }

PIPELINE Main => read_data -> transform
