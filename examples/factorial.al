// AgentLang Example: Factorial with LOOP and MUTABLE
//
// Demonstrates: MUTABLE, LOOP, MATCH, ASSIGN, comparison ops.
// Computes 6! = 720

OPERATION produce =>
  BODY {
    EMIT 6
  }

OPERATION factorial =>
  INPUT n: Int64
  BODY {
    MUTABLE result @reason("accumulator") = 1
    MUTABLE i @reason("counter") = 1
    LOOP max: 20 => {
      result = result * i
      i = i + 1
      MATCH i GT n => {
        WHEN TRUE -> { EMIT result }
        OTHERWISE -> { }
      }
    }
  }

PIPELINE Factorial => produce -> factorial
