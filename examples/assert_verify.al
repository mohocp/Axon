// AgentLang Example: Runtime Assertions
//
// Demonstrates: ASSERT with passing conditions, REQUIRE on operations.
// Expected result: 100

OPERATION validated_double =>
  INPUT x: Int64
  REQUIRE x GT 0
  BODY {
    ASSERT x GT 0
    STORE result = x * 2
    ASSERT result GT x
    EMIT result
  }

OPERATION source =>
  BODY {
    EMIT 50
  }

PIPELINE Main => source -> validated_double
