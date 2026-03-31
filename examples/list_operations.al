// AgentLang Example: List Operations
//
// Demonstrates: list literals, TAKE, SKIP, SORT stdlib operations.
// Expected result: [1, 2, 3]

OPERATION build_list =>
  BODY {
    STORE data = [5, 3, 1, 4, 2]
    STORE sorted = SORT(data)
    STORE first_three = TAKE(sorted, 3)
    EMIT first_three
  }

PIPELINE Main => build_list
