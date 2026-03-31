// AgentLang Example: Filter, Map, Reduce Pipeline
//
// Demonstrates: FILTER, MAP, REDUCE stdlib operations with named ops.
// Filters positive numbers, doubles them, then sums.
// [3, -1, 4, -2, 5] -> filter positive -> [3, 4, 5] -> double -> [6, 8, 10] -> sum -> 24
// Expected result: 24

OPERATION is_positive =>
  INPUT x: Int64
  BODY {
    EMIT x GT 0
  }

OPERATION double =>
  INPUT x: Int64
  BODY {
    EMIT x * 2
  }

OPERATION add =>
  INPUT a: Int64
  INPUT b: Int64
  BODY {
    EMIT a + b
  }

OPERATION process =>
  BODY {
    STORE data = [3, -1, 4, -2, 5]
    STORE positives = FILTER(data, "is_positive")
    STORE doubled = MAP(positives, "double")
    STORE total = REDUCE(doubled, 0, "add")
    EMIT total
  }

PIPELINE Main => process
