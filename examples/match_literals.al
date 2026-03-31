// AgentLang Example: Pattern Matching on Literal Values
//
// Demonstrates: MATCH with literal patterns and OTHERWISE fallback.
// Expected result: 200

OPERATION classify =>
  INPUT code: Int64
  BODY {
    MATCH code => {
      WHEN 1 -> { EMIT 100 }
      WHEN 2 -> { EMIT 200 }
      WHEN 3 -> { EMIT 300 }
      OTHERWISE -> { EMIT 0 }
    }
  }

OPERATION produce =>
  BODY {
    EMIT 2
  }

PIPELINE Main => produce -> classify
