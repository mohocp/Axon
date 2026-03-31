// AgentLang Example: Greatest Common Divisor (Euclidean Algorithm)
//
// Demonstrates: LOOP with convergence, MUTABLE state, division for modulo.
// Computes GCD(48, 18) = 6
// Expected result: 6

OPERATION gcd =>
  BODY {
    MUTABLE a @reason("first operand") = 48
    MUTABLE b @reason("second operand") = 18
    MUTABLE temp @reason("swap") = 0
    LOOP max: 50 => {
      MATCH b EQ 0 => {
        WHEN TRUE -> { EMIT a }
        OTHERWISE -> {
          temp = a - (a / b) * b
          a = b
          b = temp
        }
      }
    }
    EMIT a
  }

PIPELINE Main => gcd
