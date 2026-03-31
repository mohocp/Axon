// AgentLang Example: Loop Summation
//
// Demonstrates: LOOP with MUTABLE counters, bounded iteration, early exit.
// Computes 1 + 2 + 3 + ... + 10 = 55
// Expected result: 55

OPERATION sum_to_ten =>
  BODY {
    MUTABLE sum @reason("running total") = 0
    MUTABLE i @reason("counter") = 1
    LOOP max: 10 => {
      sum = sum + i
      i = i + 1
    }
    EMIT sum
  }

PIPELINE Main => sum_to_ten
