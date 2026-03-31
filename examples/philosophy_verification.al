// ============================================================
// AgentLang Philosophy: Verification by Construction
// ============================================================
//
// In AgentLang, operations carry their own correctness proofs.
// REQUIRE states what must be true before execution.
// ENSURE states what will be true after execution.
// The compiler PROVES these at compile time — not at runtime.
//
// This is not defensive programming. This is mathematical proof.
// An agent that generates code must satisfy verification
// obligations, or the code does not compile. Period.
//
// Expected result: 100

OPERATION safe_divide =>
  INPUT numerator: Int64
  INPUT divisor: Int64
  REQUIRE divisor GT 0
  ENSURE numerator GT 0
  BODY {
    ASSERT divisor GT 0
    STORE result = numerator / divisor
    EMIT result
  }

OPERATION source =>
  BODY {
    EMIT 500
  }

OPERATION divide_by_five =>
  INPUT x: Int64
  BODY {
    EMIT safe_divide(x, 5)
  }

PIPELINE Main => source -> divide_by_five
