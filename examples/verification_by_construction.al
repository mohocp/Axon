// AgentLang Philosophy: Verification by Construction
//
// Programs carry their own correctness proofs. Every operation declares
// what it REQUIRES from callers and what it ENSURES to consumers.
// The solver proves these at compile time — not at review time.
//
// This operation guarantees: if you give me a positive number,
// I will give you back a number strictly greater than what you gave me.
//
// Expected result: 51

OPERATION safe_increment =>
  INPUT x: Int64
  REQUIRE x GT 0
  ENSURE x GT 0
  BODY {
    STORE result = x + 1
    ASSERT result GT x
    EMIT result
  }

OPERATION source =>
  BODY {
    EMIT 50
  }

PIPELINE Main => source -> safe_increment
