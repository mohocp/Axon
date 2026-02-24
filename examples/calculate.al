// AgentLang Example: Simple Calculation Pipeline
//
// Demonstrates: OPERATION, PIPELINE, STORE, EMIT, arithmetic expressions.
// Expected result: 94

OPERATION produce =>
  BODY {
    EMIT 42
  }

OPERATION double =>
  INPUT x: Int64
  BODY {
    EMIT x + x
  }

OPERATION add_ten =>
  INPUT x: Int64
  BODY {
    STORE result = x + 10
    EMIT result
  }

PIPELINE Calculate => produce -> double -> add_ten
