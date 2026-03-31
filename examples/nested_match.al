// AgentLang Example: Nested Match Expressions
//
// Demonstrates: nested MATCH for multi-level decision making.
// Expected result: "big even"

OPERATION classify =>
  INPUT n: Int64
  BODY {
    STORE is_big = n GT 10
    STORE remainder = n - (n / 2) * 2
    STORE is_even = remainder EQ 0
    MATCH is_big => {
      WHEN TRUE -> {
        MATCH is_even => {
          WHEN TRUE -> { EMIT "big even" }
          OTHERWISE -> { EMIT "big odd" }
        }
      }
      OTHERWISE -> {
        MATCH is_even => {
          WHEN TRUE -> { EMIT "small even" }
          OTHERWISE -> { EMIT "small odd" }
        }
      }
    }
  }

OPERATION source =>
  BODY {
    EMIT 42
  }

PIPELINE Main => source -> classify
