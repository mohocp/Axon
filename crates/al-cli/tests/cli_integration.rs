//! CLI integration tests for `al-cli run`.
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

    let output = Command::new(env!("CARGO_BIN_EXE_al-cli"))
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
    assert!(stdout.contains("Result: 84"), "expected 84 in output: {}", stdout);
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
    assert!(stdout.contains("Result: 120"), "expected 120 in output: {}", stdout);
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
    assert_eq!(code, 0, "HALT inside operation produces FAILURE value, not crash");
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
