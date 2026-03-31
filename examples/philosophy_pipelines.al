// ============================================================
// AgentLang Philosophy: Semantic Density + Parallel by Default
// ============================================================
//
// Every token carries maximum meaning. A PIPELINE declaration
// expresses an entire dataflow in one line. No boilerplate,
// no ceremony — just the semantic structure of computation.
//
// The runtime analyzes the dependency graph and could schedule
// independent operations in parallel. The programmer never
// writes async/await or manages threads. Parallelism is implicit
// in the dataflow structure.
//
// Pipeline stages are composable: each operation has a clear
// contract (INPUT -> OUTPUT) and pipelines chain them.
// FAILURE at any stage short-circuits the rest — automatic
// error propagation without try/catch.
//
// Expected result: 25

OPERATION ingest =>
  BODY {
    EMIT 100
  }

OPERATION validate =>
  INPUT raw: Int64
  REQUIRE raw GT 0
  BODY {
    MATCH raw GT 0 => {
      WHEN TRUE -> { EMIT raw }
      OTHERWISE -> { HALT(invalid_data) }
    }
  }

OPERATION normalize =>
  INPUT value: Int64
  BODY {
    EMIT value / 4
  }

OPERATION enrich =>
  INPUT value: Int64
  BODY {
    STORE metadata = { "original": value, "processed": TRUE }
    EMIT value
  }

OPERATION score =>
  INPUT value: Int64
  BODY {
    EMIT value
  }

// One line. Five stages. Full dataflow. Short-circuits on failure.
PIPELINE Main => ingest -> validate -> normalize -> enrich -> score
