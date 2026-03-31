// AgentLang Example: Error Handling with Match
//
// Demonstrates: HALT to produce failure, MATCH to catch and handle it.
// Expected result: "recovered"

OPERATION might_fail =>
  INPUT x: Int64
  BODY {
    MATCH x LT 0 => {
      WHEN TRUE -> { HALT(invalid_input) }
      OTHERWISE -> { EMIT x * 2 }
    }
  }

OPERATION safe_process =>
  BODY {
    STORE result = might_fail(-1)
    MATCH result => {
      WHEN FAILURE(code, msg, details) -> { EMIT "recovered" }
      OTHERWISE -> { EMIT result }
    }
  }

PIPELINE Main => safe_process
