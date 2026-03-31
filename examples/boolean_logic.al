// AgentLang Example: Boolean Logic
//
// Demonstrates: AND, OR, NOT, comparison operators, nested conditions.
// Expected result: TRUE

OPERATION check =>
  BODY {
    STORE a = 10 GT 5
    STORE b = 3 LTE 3
    STORE c = a AND b
    STORE d = NOT FALSE
    STORE result = c AND d
    EMIT result
  }

PIPELINE Main => check
