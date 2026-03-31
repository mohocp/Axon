// AgentLang Example: Fork/Join Parallel Execution
//
// Demonstrates: FORK with multiple branches, JOIN ALL_COMPLETE.
// Expected result: list of branch results

OPERATION branch_a =>
  BODY {
    EMIT 10
  }

OPERATION branch_b =>
  BODY {
    EMIT 20
  }

OPERATION branch_c =>
  BODY {
    EMIT 30
  }

OPERATION orchestrate =>
  BODY {
    STORE results = FORK { a: branch_a, b: branch_b, c: branch_c } -> JOIN strategy: ALL_COMPLETE
    EMIT results
  }

PIPELINE Main => orchestrate
