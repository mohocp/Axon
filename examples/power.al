// AgentLang Example: Exponentiation via Loop
//
// Demonstrates: computing 2^10 = 1024 with bounded LOOP.
// Expected result: 1024

OPERATION power =>
  BODY {
    STORE base = 2
    STORE exp = 10
    MUTABLE result @reason("accumulator") = 1
    MUTABLE i @reason("counter") = 0
    LOOP max: 10 => {
      result = result * base
      i = i + 1
    }
    EMIT result
  }

PIPELINE Main => power
