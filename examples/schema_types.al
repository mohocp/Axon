// AgentLang Example: Schema and Type Definitions
//
// Demonstrates: TYPE alias, SCHEMA with fields, using schemas in operations.
// Expected result: 30

TYPE Age = Int64

SCHEMA Person => {
  name: Str,
  age: Int64
}

SCHEMA Team => {
  members: Int64,
  budget: Int64
}

OPERATION create_person =>
  BODY {
    STORE person = { "name": "Alice", "age": 30 }
    EMIT person
  }

OPERATION get_age =>
  INPUT person: Map
  BODY {
    EMIT person.age
  }

PIPELINE Main => create_person -> get_age
