// AgentLang Example: Fibonacci Sequence
//
// Demonstrates: MUTABLE, LOOP, multi-variable state tracking.
// Computes the 10th Fibonacci number.
// Expected result: 55

OPERATION fib =>
  BODY {
    MUTABLE a @reason("previous") = 0
    MUTABLE b @reason("current") = 1
    MUTABLE i @reason("counter") = 0
    MUTABLE temp @reason("swap") = 0
    LOOP max: 9 => {
      temp = a + b
      a = b
      b = temp
      i = i + 1
    }
    EMIT b
  }

PIPELINE Main => fib
