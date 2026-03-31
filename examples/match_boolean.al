// AgentLang Example: Match on Boolean Condition
//
// Demonstrates: MATCH on comparison result with TRUE/OTHERWISE arms.
// Expected result: "positive"

OPERATION sign =>
  INPUT x: Int64
  BODY {
    MATCH x GT 0 => {
      WHEN TRUE -> { EMIT "positive" }
      OTHERWISE -> { EMIT "non-positive" }
    }
  }

OPERATION source =>
  BODY {
    EMIT 42
  }

PIPELINE Main => source -> sign
