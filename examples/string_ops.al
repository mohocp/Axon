// AgentLang Example: String Operations
//
// Demonstrates: string literals, string concatenation, string comparisons.
// Expected result: "hello world"

OPERATION greet =>
  BODY {
    STORE greeting = "hello" + " " + "world"
    EMIT greeting
  }

PIPELINE Main => greet
