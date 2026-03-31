// AgentLang Example: Delegation Between Agents
//
// Demonstrates: DELEGATE operation TO agent, capability-scoped execution.
// Expected result: 15

AGENT Orchestrator =>
  CAPABILITIES [delegate]

AGENT Worker =>
  CAPABILITIES [FILE_READ]

OPERATION sub_task =>
  INPUT x: Int64
  BODY {
    EMIT x + 10
  }

OPERATION orchestrate =>
  BODY {
    DELEGATE sub_task TO Worker => {
      INPUT 5
    }
    EMIT sub_task_result
  }

PIPELINE Main => orchestrate
