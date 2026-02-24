//! CLI integration tests for `al run`.
//!
//! These tests exercise the full lex → parse → check → execute pipeline
//! through the CLI binary, verifying stdout output and exit codes.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);
fn rand_u64() -> u64 {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Helper: run `al-cli run` on an inline source string via a temp file.
fn run_cli_source(source: &str) -> (String, String, i32) {
    let dir = std::env::temp_dir().join("agentlang_cli_test");
    std::fs::create_dir_all(&dir).unwrap();
    let unique = format!("test_{}.al", std::process::id() as u64 * 1000 + rand_u64());
    let file = dir.join(unique);
    std::fs::write(&file, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_al"))
        .args(["run", file.to_str().unwrap()])
        .output()
        .expect("failed to execute al-cli");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    std::fs::remove_file(&file).ok();

    (stdout, stderr, code)
}

// =========================================================================
// End-to-end CLI tests
// =========================================================================

#[test]
fn cli_run_calculate_pipeline() {
    let source = r#"
OPERATION produce => BODY { EMIT 42 }
OPERATION double =>
  INPUT x: Int64
  BODY { EMIT x + x }
PIPELINE Main => produce -> double
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(code, 0, "exit code should be 0");
    assert!(
        stdout.contains("Result: 84"),
        "expected 84 in output: {}",
        stdout
    );
}

#[test]
fn cli_run_factorial() {
    let source = r#"
OPERATION produce => BODY { EMIT 5 }
OPERATION factorial =>
  INPUT n: Int64
  BODY {
    MUTABLE result @reason("acc") = 1
    MUTABLE i @reason("ctr") = 1
    LOOP max: 20 => {
      result = result * i
      i = i + 1
      MATCH i GT n => {
        WHEN TRUE -> { EMIT result }
        OTHERWISE -> { }
      }
    }
  }
PIPELINE Main => produce -> factorial
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("Result: 120"),
        "expected 120 in output: {}",
        stdout
    );
}

#[test]
fn cli_run_match_success_failure() {
    let source = r#"
OPERATION make_success => BODY { EMIT 42 }
OPERATION classify =>
  INPUT x: Int64
  BODY {
    MATCH x GT 10 => {
      WHEN TRUE -> { EMIT "big" }
      OTHERWISE -> { EMIT "small" }
    }
  }
PIPELINE Main => make_success -> classify
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(code, 0);
    assert!(
        stdout.contains(r#"Result: "big""#),
        "expected \"big\" in output: {}",
        stdout
    );
}

#[test]
fn cli_run_pipeline_short_circuit() {
    let source = r#"
OPERATION fail_op => BODY { HALT(test_error) }
OPERATION unreachable =>
  INPUT x: Int64
  BODY { EMIT 999 }
PIPELINE Main => fail_op -> unreachable
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(
        code, 0,
        "HALT inside operation produces FAILURE value, not crash"
    );
    assert!(
        stdout.contains("FAILURE"),
        "expected FAILURE in output: {}",
        stdout
    );
}

#[test]
fn cli_run_parse_error_exits_nonzero() {
    let source = "OPERATION ??? => BODY { }";
    let (_stdout, stderr, code) = run_cli_source(source);
    assert_ne!(code, 0, "parse error should exit non-zero");
    assert!(
        stderr.contains("error"),
        "stderr should contain error: {}",
        stderr
    );
}

#[test]
fn cli_run_type_error_exits_nonzero() {
    // Duplicate type definition should fail type check.
    let source = r#"
TYPE Foo = Int64
TYPE Foo = Str
"#;
    let (_stdout, stderr, code) = run_cli_source(source);
    assert_ne!(code, 0, "type error should exit non-zero");
    assert!(
        stderr.contains("error"),
        "stderr should contain error: {}",
        stderr
    );
}

#[test]
fn cli_run_agent_with_caps() {
    let source = r#"
AGENT Worker =>
  CAPABILITIES [FILE_READ, API_CALL]
  TRUST_LEVEL ~0.9

OPERATION produce => BODY { EMIT 42 }
PIPELINE Main => produce
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("1 agents registered"),
        "should register agent: {}",
        stdout
    );
    assert!(stdout.contains("Result: 42"));
}

#[test]
fn cli_run_map_and_member_access() {
    let source = r#"
OPERATION test => BODY {
  STORE m = { "x": 10, "y": 20 }
  EMIT m.x + m.y
}
PIPELINE Main => test
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(code, 0);
    assert!(stdout.contains("Result: 30"), "expected 30: {}", stdout);
}

// =========================================================================
// Round 5 Slice 2 — CLI integration tests
// =========================================================================

#[test]
fn cli_run_fork_join_all_succeed() {
    let source = r#"
OPERATION a => BODY { EMIT 10 }
OPERATION b => BODY { EMIT 20 }
OPERATION test =>
  INPUT x: Int64
  BODY {
    STORE r = FORK { x: a, y: b } -> JOIN strategy: ALL_COMPLETE
    EMIT r
  }
PIPELINE Main => test
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("Result: [10, 20]"),
        "expected [10, 20] in output: {}",
        stdout
    );
}

#[test]
fn cli_run_fork_join_failure_collected() {
    let source = r#"
OPERATION ok_branch => BODY { EMIT 42 }
OPERATION bad_branch => BODY { HALT(branch_err) }
OPERATION test =>
  INPUT x: Int64
  BODY {
    STORE r = FORK { ok: ok_branch, bad: bad_branch } -> JOIN strategy: ALL_COMPLETE
    EMIT r
  }
PIPELINE Main => test
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(
        code, 0,
        "fork/join failure produces FAILURE value, not crash"
    );
    assert!(
        stdout.contains("FORK_JOIN_FAILED"),
        "expected FORK_JOIN_FAILED in output: {}",
        stdout
    );
}

#[test]
fn cli_run_retry_exhausted() {
    let source = r#"
OPERATION always_retry => BODY {
  RETRY(2)
}
PIPELINE Main => always_retry
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(code, 0, "RETRY_EXHAUSTED produces FAILURE value");
    assert!(
        stdout.contains("RETRY_EXHAUSTED"),
        "expected RETRY_EXHAUSTED in output: {}",
        stdout
    );
}

#[test]
fn cli_run_escalate() {
    let source = r#"
OPERATION test => BODY {
  ESCALATE("critical")
}
PIPELINE Main => test
"#;
    let (_stdout, _stderr, code) = run_cli_source(source);
    // ESCALATE produces a runtime failure which causes non-zero exit.
    assert_ne!(code, 0, "ESCALATE should fail execution");
}

#[test]
fn cli_run_assert_pass() {
    let source = r#"
OPERATION test => BODY {
  ASSERT 5 GT 3
  EMIT 42
}
PIPELINE Main => test
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(code, 0);
    assert!(stdout.contains("Result: 42"), "expected 42: {}", stdout);
}

#[test]
fn cli_run_assert_fail() {
    let source = r#"
OPERATION test => BODY {
  ASSERT 1 GT 2
  EMIT 42
}
PIPELINE Main => test
"#;
    let (_stdout, _stderr, code) = run_cli_source(source);
    assert_ne!(code, 0, "ASSERT failure should exit non-zero");
}

#[test]
fn cli_run_delegate_basic() {
    let source = r#"
AGENT Orchestrator =>
  CAPABILITIES [delegate]
AGENT Worker =>
  CAPABILITIES [FILE_READ]
OPERATION sub_task =>
  INPUT x: Int64
  BODY { EMIT x + 10 }
OPERATION main_op => BODY {
  DELEGATE sub_task TO Worker => {
    INPUT 5
  }
  EMIT sub_task_result
}
PIPELINE Main => main_op
"#;
    let (stdout, _stderr, code) = run_cli_source(source);
    assert_eq!(code, 0);
    assert!(stdout.contains("Result: 15"), "expected 15: {}", stdout);
}
