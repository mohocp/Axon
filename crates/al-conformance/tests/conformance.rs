//! AgentLang Conformance Tests - MVP v0.1
//!
//! These tests validate the implementation against the specification
//! requirements identified as C1-C10.

use al_conformance::*;
use std::collections::BTreeMap;

// ===========================================================================
// C1: Lex/parse round-trip
// ===========================================================================

#[test]
fn c1_lex_parse_roundtrip() {
    let fixture = &all_fixtures()[0];
    assert_eq!(fixture.id, "C1");

    // Lex
    let token_count = lex_source(fixture.source).expect("C1: lexing should succeed");
    assert!(token_count > 0, "C1: should produce tokens");

    // Parse
    let program = parse_source(fixture.source).expect("C1: parsing should succeed");
    assert_eq!(
        program.declarations.len(),
        fixture.expected_declarations.unwrap(),
        "C1: declaration count mismatch"
    );

    // Verify declaration types
    assert!(matches!(
        program.declarations[0].node,
        al_ast::Declaration::TypeDecl { .. }
    ));
    assert!(matches!(
        program.declarations[1].node,
        al_ast::Declaration::SchemaDecl { .. }
    ));
    assert!(matches!(
        program.declarations[2].node,
        al_ast::Declaration::OperationDecl { .. }
    ));
    assert!(matches!(
        program.declarations[3].node,
        al_ast::Declaration::PipelineDecl { .. }
    ));
}

#[test]
fn c1_empty_source_parses() {
    let program = parse_source("").expect("empty source should parse");
    assert_eq!(program.declarations.len(), 0);
}

#[test]
fn c1_tokens_have_spans() {
    let tokens = al_lexer::tokenize("TYPE UserId = Int64").unwrap();
    // TYPE keyword at line 1, col 1
    assert_eq!(tokens[0].span.line, 1);
    assert_eq!(tokens[0].span.column, 1);
}

// ===========================================================================
// C2: Failure arity (3-field FAILURE pattern)
// ===========================================================================

#[test]
fn c2_failure_arity_3_fields() {
    let fixture = &all_fixtures()[1];
    assert_eq!(fixture.id, "C2");

    let program = parse_source(fixture.source).expect("C2: parsing should succeed");

    // Type check
    let checker = check_source(fixture.source).expect("C2: type check should succeed");
    assert!(!checker.has_errors(), "C2: should have no type errors");

    // Verify the MATCH statement contains a FAILURE pattern
    if let al_ast::Declaration::OperationDecl { body, .. } = &program.declarations[0].node {
        if let al_ast::Statement::Match { arms, .. } = &body.node.stmts[0].node {
            // Second arm should be a FAILURE pattern
            let failure_arm = &arms[1];
            assert!(
                matches!(failure_arm.node.pattern.node, al_ast::Pattern::Failure { .. }),
                "C2: second match arm should be a FAILURE pattern"
            );
        } else {
            panic!("C2: expected MATCH statement");
        }
    }
}

// ===========================================================================
// C3: Capability deny
// ===========================================================================

#[test]
fn c3_capability_deny() {
    let fixture = &all_fixtures()[2];
    assert_eq!(fixture.id, "C3");

    let program = parse_source(fixture.source).expect("C3: parsing should succeed");
    assert_eq!(program.declarations.len(), 1);

    if let al_ast::Declaration::AgentDecl { name, properties } = &program.declarations[0].node {
        assert_eq!(name.node, "SecureWorker");

        // Should have CAPABILITIES, DENY, TRUST_LEVEL
        let has_caps = properties.iter().any(|p| {
            matches!(p.node, al_ast::AgentProperty::Capabilities(_))
        });
        let has_deny = properties.iter().any(|p| {
            matches!(p.node, al_ast::AgentProperty::Deny(_))
        });
        let has_trust = properties.iter().any(|p| {
            matches!(p.node, al_ast::AgentProperty::TrustLevel(_))
        });

        assert!(has_caps, "C3: should have CAPABILITIES");
        assert!(has_deny, "C3: should have DENY");
        assert!(has_trust, "C3: should have TRUST_LEVEL");
    } else {
        panic!("C3: expected AgentDecl");
    }
}

#[test]
fn c3_capability_runtime_check() {
    use al_capabilities::{Capability, CapabilitySet};
    use al_runtime::Runtime;

    let mut rt = Runtime::new();
    let mut caps = CapabilitySet::empty();
    caps.insert(Capability::FileRead);
    caps.insert(Capability::ApiCall);
    rt.register_agent("secure-worker", caps);

    // Granted capabilities succeed
    assert!(rt.check_capability("secure-worker", Capability::FileRead).is_ok());
    assert!(rt.check_capability("secure-worker", Capability::ApiCall).is_ok());

    // Denied capabilities fail
    assert!(rt.check_capability("secure-worker", Capability::FileWrite).is_err());
    assert!(rt.check_capability("secure-worker", Capability::DbWrite).is_err());
}

// ===========================================================================
// C4: Fork/join ALL_COMPLETE
// ===========================================================================

#[test]
fn c4_fork_join_parse() {
    let fixture = &all_fixtures()[3];
    assert_eq!(fixture.id, "C4");

    let program = parse_source(fixture.source).expect("C4: parsing should succeed");
    assert_eq!(program.declarations.len(), 1);
}

#[test]
fn c4_fork_join_runtime() {
    use al_diagnostics::{ErrorCode, RuntimeFailure};
    use al_runtime::{Runtime, Value};

    let mut rt = Runtime::new();

    // ALL_COMPLETE: all branches succeed
    let branches: Vec<Box<dyn FnOnce(&mut Runtime) -> Result<Value, RuntimeFailure>>> = vec![
        Box::new(|_| Ok(Value::Int(1))),
        Box::new(|_| Ok(Value::Int(2))),
    ];
    let results = rt.execute_fork_join(branches).unwrap();
    assert_eq!(results, vec![Value::Int(1), Value::Int(2)]);

    // ALL_COMPLETE: one branch fails -> entire fork fails
    let branches: Vec<Box<dyn FnOnce(&mut Runtime) -> Result<Value, RuntimeFailure>>> = vec![
        Box::new(|_| Ok(Value::Int(1))),
        Box::new(|_| Err(RuntimeFailure::new(ErrorCode::NotImplemented, "fail"))),
    ];
    assert!(rt.execute_fork_join(branches).is_err());
}

// ===========================================================================
// C5: Checkpoint/resume
// ===========================================================================

#[test]
fn c5_checkpoint_parse() {
    let fixture = &all_fixtures()[4];
    assert_eq!(fixture.id, "C5");

    let program = parse_source(fixture.source).expect("C5: parsing should succeed");
    assert_eq!(program.declarations.len(), 1);

    if let al_ast::Declaration::OperationDecl { body, .. } = &program.declarations[0].node {
        // Should have a CHECKPOINT statement
        let has_checkpoint = body.node.stmts.iter().any(|s| {
            matches!(s.node, al_ast::Statement::Checkpoint { .. })
        });
        assert!(has_checkpoint, "C5: should have CHECKPOINT statement");
    }
}

#[test]
fn c5_checkpoint_runtime() {
    use al_capabilities::CapabilitySet;
    use al_runtime::{Runtime, Value};

    let mut rt = Runtime::new();
    rt.register_agent("agent-1", CapabilitySet::empty());

    let state = Value::Map({
        let mut m = BTreeMap::new();
        m.insert("counter".into(), Value::Int(42));
        m
    });

    let cp_id = rt.create_checkpoint("agent-1", state.clone());
    let restored = rt.restore_checkpoint(&cp_id).unwrap();
    assert_eq!(restored, state);
}

// ===========================================================================
// C6: Pipeline composition
// ===========================================================================

#[test]
fn c6_pipeline_parse() {
    let fixture = &all_fixtures()[5];
    assert_eq!(fixture.id, "C6");

    let program = parse_source(fixture.source).expect("C6: parsing should succeed");
    assert_eq!(program.declarations.len(), 2);

    // First pipeline: 4 stages
    if let al_ast::Declaration::PipelineDecl { name, chain } = &program.declarations[0].node {
        assert_eq!(name.node, "DataFlow");
        assert_eq!(chain.node.stages.len(), 4);
    }

    // Second pipeline: 2 stages
    if let al_ast::Declaration::PipelineDecl { name, chain } = &program.declarations[1].node {
        assert_eq!(name.node, "Simple");
        assert_eq!(chain.node.stages.len(), 2);
    }
}

// ===========================================================================
// C7: Audit trail
// ===========================================================================

#[test]
fn c7_audit_assert_parse() {
    let fixture = &all_fixtures()[6];
    assert_eq!(fixture.id, "C7");

    let program = parse_source(fixture.source).expect("C7: parsing should succeed");
    assert_eq!(program.declarations.len(), 1);

    if let al_ast::Declaration::OperationDecl { requires, body, .. } = &program.declarations[0].node {
        assert!(!requires.is_empty(), "C7: should have REQUIRE clauses");
        let has_assert = body.node.stmts.iter().any(|s| {
            matches!(s.node, al_ast::Statement::Assert { .. })
        });
        assert!(has_assert, "C7: should have ASSERT statement");
    }
}

#[test]
fn c7_audit_runtime() {
    use al_diagnostics::AuditEventType;
    use al_runtime::Runtime;

    let mut rt = Runtime::new();

    // Failed assertion emits audit event
    let _ = rt.execute_assert(false, "vc-c7", "x > 0");
    assert_eq!(rt.audit_log.len(), 1);
    assert_eq!(rt.audit_log[0].event_type, AuditEventType::AssertFailed);
    assert_eq!(rt.audit_log[0].details["vc_id"], "vc-c7");

    // Insert runtime assert emits audit
    rt.insert_runtime_assert("vc-c7-insert", "invariant check");
    assert_eq!(rt.audit_log.len(), 2);
    assert_eq!(rt.audit_log[1].event_type, AuditEventType::AssertInserted);
}

// ===========================================================================
// C8: Excluded features
// ===========================================================================

#[test]
fn c8_valid_mvp_subset() {
    let fixture = &all_fixtures()[7];
    assert_eq!(fixture.id, "C8");

    let program = parse_source(fixture.source).expect("C8: parsing should succeed");
    assert_eq!(program.declarations.len(), 2);

    let checker = check_source(fixture.source).expect("C8: type check should succeed");
    assert!(!checker.has_errors());
}

#[test]
fn c8_profile_is_mvp() {
    assert_eq!(al_conformance::PROFILE, "mvp-0.1");
    assert_eq!(al_diagnostics::MVP_PROFILE, "mvp-0.1");
}

// ===========================================================================
// C9: Type checking (duplicate detection)
// ===========================================================================

#[test]
fn c9_duplicate_type_detected() {
    let fixture = &all_fixtures()[8];
    assert_eq!(fixture.id, "C9");

    let program = parse_source(fixture.source).expect("C9: parsing should succeed");
    assert_eq!(program.declarations.len(), 2);

    let checker = check_source(fixture.source).expect("C9: check_source should not error");
    assert!(
        checker.has_errors(),
        "C9: duplicate type should produce error"
    );
}

#[test]
fn c9_duplicate_operation_detected() {
    let source = r#"OPERATION Foo => BODY { EMIT 1 }
OPERATION Foo => BODY { EMIT 2 }"#;
    let checker = check_source(source).expect("should parse");
    assert!(checker.has_errors());
}

// ===========================================================================
// C10: Retry/escalation
// ===========================================================================

#[test]
fn c10_retry_escalate_parse() {
    let fixture = &all_fixtures()[9];
    assert_eq!(fixture.id, "C10");

    let program = parse_source(fixture.source).expect("C10: parsing should succeed");
    assert_eq!(program.declarations.len(), 1);

    if let al_ast::Declaration::OperationDecl { body, .. } = &program.declarations[0].node {
        assert!(matches!(body.node.stmts[0].node, al_ast::Statement::Retry { .. }));
        assert!(matches!(body.node.stmts[1].node, al_ast::Statement::Escalate { .. }));
    }
}

#[test]
fn c10_retry_runtime() {
    use al_diagnostics::{ErrorCode, RuntimeFailure};
    use al_runtime::{Runtime, Value};

    let mut rt = Runtime::new();
    let mut attempt = 0u64;

    // Retry succeeds on 3rd attempt
    let result = rt.execute_retry(2, |_| {
        attempt += 1;
        if attempt < 3 {
            Err(RuntimeFailure::new(ErrorCode::NotImplemented, "fail"))
        } else {
            Ok(Value::Str("success".into()))
        }
    });
    assert!(result.is_ok());
    assert_eq!(attempt, 3);
}

#[test]
fn c10_escalation_runtime() {
    use al_capabilities::CapabilitySet;
    use al_diagnostics::{AuditEventType, ErrorCode};
    use al_runtime::Runtime;

    let mut rt = Runtime::new();
    rt.register_agent("agent-1", CapabilitySet::empty());

    let failure = rt.execute_escalate(Some("critical error".into()), "agent-1");
    assert_eq!(failure.code, ErrorCode::Escalated);
    assert_eq!(rt.audit_log.len(), 1);
    assert_eq!(rt.audit_log[0].event_type, AuditEventType::Escalated);
}

// ===========================================================================
// C11: Match arm body — statement keywords after ->
// ===========================================================================

#[test]
fn c11_match_body_statement_keywords() {
    let fixture = all_fixtures().into_iter().find(|f| f.id == "C11").unwrap();

    let program = parse_source(fixture.source).expect("C11: parsing should succeed");
    assert_eq!(program.declarations.len(), 1);

    if let al_ast::Declaration::OperationDecl { body, .. } = &program.declarations[0].node {
        if let al_ast::Statement::Match { arms, otherwise, .. } = &body.node.stmts[0].node {
            assert_eq!(arms.len(), 2, "C11: should have 2 WHEN arms");
            // First arm: EMIT val (wrapped in synthetic block)
            assert!(
                matches!(arms[0].node.body.node, al_ast::MatchBody::Block(_)),
                "C11: EMIT arm should be wrapped in block"
            );
            // Second arm: ESCALATE(msg)
            assert!(
                matches!(arms[1].node.body.node, al_ast::MatchBody::Block(_)),
                "C11: ESCALATE arm should be wrapped in block"
            );
            // OTHERWISE: HALT
            assert!(otherwise.is_some(), "C11: should have OTHERWISE");
        } else {
            panic!("C11: expected MATCH statement");
        }
    } else {
        panic!("C11: expected OperationDecl");
    }
}

// ===========================================================================
// C12: Undefined type reference detection
// ===========================================================================

#[test]
fn c12_undefined_type_detected() {
    let fixture = all_fixtures().into_iter().find(|f| f.id == "C12").unwrap();

    let _program = parse_source(fixture.source).expect("C12: parsing should succeed");
    let checker = check_source(fixture.source).expect("C12: check_source should not error");
    assert!(
        checker.has_errors(),
        "C12: undefined type reference should produce error"
    );
}

#[test]
fn c12_builtin_types_resolve() {
    let source = r#"
TYPE UserId = Int64
SCHEMA User => { name: Str, id: Int64 }
OPERATION GetUser =>
  INPUT id: Int64
  OUTPUT User
  BODY { EMIT id }
"#;
    let checker = check_source(source).expect("should parse");
    assert!(
        !checker.has_errors(),
        "C12: built-in and schema types should resolve"
    );
}

// ===========================================================================
// C13: Parser error recovery
// ===========================================================================

#[test]
fn c13_parser_recovery() {
    // Use parse_recovering to get partial results after errors.
    // `OPERATION => BODY { }` is missing the name — a parse error.
    let source = r#"
TYPE Valid1 = Int64
OPERATION => BODY { }
TYPE Valid2 = Str
"#;
    let (program, diagnostics) = al_parser::parse_recovering(source);
    assert_eq!(
        program.declarations.len(),
        2,
        "C13: should recover 2 valid declarations"
    );
    assert!(
        !diagnostics.is_empty(),
        "C13: should have reported parse errors"
    );
}

// ===========================================================================
// C14: REQUIRE clause validation
// ===========================================================================

#[test]
fn c14_require_valid_input_reference() {
    let fixture = all_fixtures().into_iter().find(|f| f.id == "C14").unwrap();

    let checker = check_source(fixture.source).expect("C14: should parse");
    assert!(
        !checker.has_errors(),
        "C14: REQUIRE referencing input should not error"
    );
}

#[test]
fn c14_require_unknown_identifier() {
    let source = r#"
OPERATION Guarded =>
  INPUT x: Int64
  REQUIRE unknown_var GT 0
  BODY { EMIT x }
"#;
    let checker = check_source(source).expect("should parse");
    assert!(
        checker.has_errors(),
        "C14: REQUIRE with unknown identifier should error"
    );
}

// ===========================================================================
// Cross-cutting: all fixtures parse
// ===========================================================================

#[test]
fn all_fixtures_conform() {
    for fixture in all_fixtures() {
        let parse_result = parse_source(fixture.source);

        if fixture.should_parse {
            let program = parse_result.unwrap_or_else(|e| {
                panic!("Fixture {} should parse but got: {}", fixture.id, e)
            });

            if let Some(expected) = fixture.expected_declarations {
                assert_eq!(
                    program.declarations.len(),
                    expected,
                    "Fixture {} declaration count",
                    fixture.id
                );
            }

            if fixture.should_typecheck {
                let checker = check_source(fixture.source).expect("should parse for typecheck");
                assert!(
                    !checker.has_errors(),
                    "Fixture {} should typecheck but has errors",
                    fixture.id
                );
            }
        } else {
            assert!(
                parse_result.is_err(),
                "Fixture {} should fail to parse but succeeded",
                fixture.id
            );
        }
    }
}
