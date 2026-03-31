// AgentLang Example: Map Literals and Member Access
//
// Demonstrates: map construction, dot-notation field access, nested data.
// Expected result: 42

OPERATION build_record =>
  BODY {
    STORE record = { "name": "AgentLang", "version": 1, "value": 42 }
    EMIT record
  }

OPERATION extract_value =>
  INPUT record: Map
  BODY {
    STORE v = record.value
    EMIT v
  }

PIPELINE Main => build_record -> extract_value
