// AgentLang Example: Multi-Stage Pipeline
//
// Demonstrates: 5-stage pipeline with data threading.
// 1 -> double(1)=2 -> add_ten(2)=12 -> square(12)=144 -> subtract(144)=139
// Expected result: 139

OPERATION start =>
  BODY {
    EMIT 1
  }

OPERATION double =>
  INPUT x: Int64
  BODY {
    EMIT x * 2
  }

OPERATION add_ten =>
  INPUT x: Int64
  BODY {
    EMIT x + 10
  }

OPERATION square =>
  INPUT x: Int64
  BODY {
    EMIT x * x
  }

OPERATION subtract_five =>
  INPUT x: Int64
  BODY {
    EMIT x - 5
  }

PIPELINE Main => start -> double -> add_ten -> square -> subtract_five
