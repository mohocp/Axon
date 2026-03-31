// AgentLang Example: Arithmetic Operations
//
// Demonstrates: all arithmetic operators (+, -, *, /), chained computation.
// Expected result: 42

OPERATION compute =>
  BODY {
    STORE a = 10 * 5
    STORE b = a - 8
    STORE c = b + 0
    STORE d = c / 1
    EMIT d
  }

PIPELINE Main => compute
