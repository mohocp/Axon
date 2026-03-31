// AgentLang Example: Collatz Conjecture Step Counter
//
// Demonstrates: complex loop logic, conditional mutation, modulo for parity.
// Counts steps for n=27 to reach 1 (answer: 111 steps).
// Expected result: 111

OPERATION collatz_steps =>
  BODY {
    MUTABLE n @reason("current value") = 27
    MUTABLE steps @reason("step counter") = 0
    LOOP max: 200 => {
      MATCH n EQ 1 => {
        WHEN TRUE -> { EMIT steps }
        OTHERWISE -> {
          MUTABLE rem @reason("parity check") = n - (n / 2) * 2
          MATCH rem EQ 0 => {
            WHEN TRUE -> { n = n / 2 }
            OTHERWISE -> { n = n * 3 + 1 }
          }
          steps = steps + 1
        }
      }
    }
    EMIT steps
  }

PIPELINE Main => collatz_steps
