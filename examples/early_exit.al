// AgentLang Example: Early Exit from Loop
//
// Demonstrates: using MUTABLE flag + MATCH to find a value and exit.
// Finds the first number > 5 in a counting sequence.
// Expected result: 6

OPERATION find_first_gt_five =>
  BODY {
    MUTABLE i @reason("counter") = 0
    MUTABLE found @reason("result holder") = -1
    LOOP max: 100 => {
      i = i + 1
      MATCH i GT 5 => {
        WHEN TRUE -> {
          MATCH found EQ -1 => {
            WHEN TRUE -> { found = i }
            OTHERWISE -> { }
          }
        }
        OTHERWISE -> { }
      }
    }
    EMIT found
  }

PIPELINE Main => find_first_gt_five
