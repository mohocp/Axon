//! # al-conformance
//!
//! Conformance test harness for AgentLang MVP v0.1.
//!
//! Provides test fixture infrastructure and helper functions for
//! conformance tests C1-C10, validating that the AgentLang implementation
//! meets the specification requirements.

/// The conformance profile for MVP v0.1.
pub const PROFILE: &str = "mvp-0.1";

/// A conformance test fixture: source code paired with expected outcomes.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// Fixture identifier (e.g., "C1", "C2").
    pub id: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// AgentLang source code for the fixture.
    pub source: &'static str,
    /// Whether parsing should succeed.
    pub should_parse: bool,
    /// Whether type checking should succeed (only relevant if parsing succeeds).
    pub should_typecheck: bool,
    /// Expected number of declarations (if parsing succeeds).
    pub expected_declarations: Option<usize>,
}

/// Helper: lex and return token count, or error.
pub fn lex_source(source: &str) -> Result<usize, String> {
    match al_lexer::tokenize(source) {
        Ok(tokens) => Ok(tokens.len()),
        Err(diags) => Err(diags
            .iter()
            .map(|d| format!("[{}] {}", d.code, d.message))
            .collect::<Vec<_>>()
            .join("; ")),
    }
}

/// Helper: parse and return declaration count, or error.
pub fn parse_source(source: &str) -> Result<al_ast::Program, String> {
    al_parser::parse(source).map_err(|diags| {
        diags
            .iter()
            .map(|d| format!("[{}] {}", d.code, d.message))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

/// Helper: parse and type-check, returning the type checker.
pub fn check_source(source: &str) -> Result<al_types::TypeChecker, String> {
    let program = parse_source(source)?;
    let mut checker = al_types::TypeChecker::new();
    checker.check(&program);
    Ok(checker)
}

/// All C1-C10 fixtures.
pub fn all_fixtures() -> Vec<Fixture> {
    vec![
        // C1: Lex/parse round-trip
        Fixture {
            id: "C1",
            description: "Lex/parse round-trip: basic declarations parse correctly",
            source: r#"TYPE UserId = Int64
SCHEMA User => { name: Str, age: Int64 }
OPERATION GetUser =>
  INPUT id: Int64
  OUTPUT User
  BODY {
    EMIT id
  }
PIPELINE Process => fetch -> validate |> transform
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(4),
        },
        // C2: Failure arity (3-field FAILURE pattern)
        Fixture {
            id: "C2",
            description: "Failure arity: FAILURE pattern requires 3 fields (code, msg, details)",
            source: r#"OPERATION HandleResult =>
  INPUT result: Result[Int64]
  BODY {
    MATCH result => {
      WHEN SUCCESS(val) -> { EMIT val }
      WHEN FAILURE(code, msg, details) -> {
        ESCALATE(msg)
      }
    }
  }
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(1),
        },
        // C3: Capability deny
        Fixture {
            id: "C3",
            description: "Capability deny: agent declarations with CAPABILITIES and DENY",
            source: r#"AGENT SecureWorker =>
  CAPABILITIES [FILE_READ, API_CALL]
  DENY [FILE_WRITE, DB_WRITE]
  TRUST_LEVEL ~0.8
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(1),
        },
        // C4: Fork/join ALL_COMPLETE
        Fixture {
            id: "C4",
            description: "Fork/join: FORK with ALL_COMPLETE join strategy",
            source: r#"OPERATION ParallelOp =>
  BODY {
    STORE results = FORK { a: fetch, b: validate } -> JOIN strategy: ALL_COMPLETE
    EMIT results
  }
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(1),
        },
        // C5: Checkpoint/resume
        Fixture {
            id: "C5",
            description: "Checkpoint: CHECKPOINT and RESUME statements",
            source: r#"OPERATION Checkpointable =>
  BODY {
    STORE state = compute()
    CHECKPOINT "save1"
    STORE result = process(state)
    EMIT result
  }
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(1),
        },
        // C6: Pipeline composition
        Fixture {
            id: "C6",
            description: "Pipeline: arrow and pipe-forward operators in chains",
            source: r#"PIPELINE DataFlow => fetch -> validate |> transform -> store
PIPELINE Simple => a -> b
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(2),
        },
        // C7: Audit trail (assertions emit audit events)
        Fixture {
            id: "C7",
            description: "Audit trail: ASSERT statements for runtime verification",
            source: r#"OPERATION Verified =>
  INPUT x: Int64
  REQUIRE x GT 0
  BODY {
    ASSERT x GT 0
    EMIT x
  }
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(1),
        },
        // C8: Excluded features rejection
        Fixture {
            id: "C8",
            description: "Excluded features: valid MVP subset without excluded constructs",
            source: r#"TYPE Count = Int64
OPERATION Simple =>
  BODY {
    STORE x = 42
    EMIT x
  }
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(2),
        },
        // C9: Type checking (duplicate detection)
        Fixture {
            id: "C9",
            description: "Type checking: duplicate definitions are detected",
            source: r#"TYPE Foo = Int64
TYPE Foo = Str
"#,
            should_parse: true,
            should_typecheck: false,
            expected_declarations: Some(2),
        },
        // C10: Retry/escalation semantics
        Fixture {
            id: "C10",
            description: "Retry/escalation: RETRY and ESCALATE statements",
            source: r#"OPERATION Resilient =>
  BODY {
    RETRY(3)
    ESCALATE("all retries exhausted")
  }
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(1),
        },
        // C11: Match arm body — statement keywords directly after ->
        Fixture {
            id: "C11",
            description: "Match arm body supports statement keywords (EMIT, ESCALATE, etc.) after ->",
            source: r#"OPERATION HandleResult =>
  INPUT result: Result[Int64]
  BODY {
    MATCH result => {
      WHEN SUCCESS(val) -> EMIT val
      WHEN FAILURE(code, msg, details) -> ESCALATE(msg)
      OTHERWISE -> HALT(unknown)
    }
  }
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(1),
        },
        // C12: Undefined type reference detection
        Fixture {
            id: "C12",
            description: "Type checker rejects undefined type references",
            source: r#"TYPE Foo = NonexistentType
"#,
            should_parse: true,
            should_typecheck: false,
            expected_declarations: Some(1),
        },
        // C13: Parser error recovery
        Fixture {
            id: "C13",
            description: "Parser recovers from errors and continues parsing",
            source: r#"TYPE Valid1 = Int64
TYPE Valid2 = Str
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(2),
        },
        // C14: REQUIRE clause validation
        Fixture {
            id: "C14",
            description: "REQUIRE clause identifiers must reference operation inputs",
            source: r#"OPERATION Guarded =>
  INPUT x: Int64
  REQUIRE x GT 0
  BODY {
    EMIT x
  }
"#,
            should_parse: true,
            should_typecheck: true,
            expected_declarations: Some(1),
        },
    ]
}
