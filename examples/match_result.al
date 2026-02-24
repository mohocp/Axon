// AgentLang Example: Match on SUCCESS/FAILURE patterns
//
// Demonstrates: MATCH, SUCCESS/FAILURE patterns, STORE, AGENT, PIPELINE.
// Expected result: "Data processed: 42"

SCHEMA DataResult => {
  value: Int64
}

AGENT Processor =>
  CAPABILITIES [FILE_READ]
  TRUST_LEVEL ~0.95

OPERATION fetch_data =>
  BODY {
    EMIT 42
  }

OPERATION wrap_success =>
  INPUT data: Int64
  BODY {
    STORE result = { "value": data, "status": "ok" }
    EMIT result
  }

OPERATION process_result =>
  INPUT result: Map
  BODY {
    STORE val = result.value
    MATCH val GT 0 => {
      WHEN TRUE -> {
        EMIT val * 2
      }
      OTHERWISE -> {
        HALT(invalid_data)
      }
    }
  }

PIPELINE DataFlow => fetch_data -> wrap_success -> process_result
