//! Smoke tests for `al` CLI command UX.
//!
//! These tests validate the command surface, help output, output formats,
//! exit codes, and overall user experience of the `al` binary.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);
fn unique_id() -> u64 {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn al_bin() -> &'static str {
    env!("CARGO_BIN_EXE_al")
}

/// Helper: run `al` with given args.
fn run_al(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(al_bin())
        .args(args)
        .output()
        .expect("failed to execute al");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    (stdout, stderr, code)
}

/// Helper: run `al <cmd>` on an inline source string via a temp file.
fn run_al_cmd(cmd: &str, source: &str) -> (String, String, i32) {
    let dir = std::env::temp_dir().join("agentlang_smoke");
    std::fs::create_dir_all(&dir).unwrap();
    let unique = format!(
        "smoke_{}.al",
        std::process::id() as u64 * 1000 + unique_id()
    );
    let file = dir.join(unique);
    std::fs::write(&file, source).unwrap();

    let output = Command::new(al_bin())
        .args([cmd, file.to_str().unwrap()])
        .output()
        .expect("failed to execute al");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    std::fs::remove_file(&file).ok();

    (stdout, stderr, code)
}

/// Helper: run `al <cmd> --format <fmt>` on inline source.
fn run_al_cmd_format(cmd: &str, source: &str, format: &str) -> (String, String, i32) {
    let dir = std::env::temp_dir().join("agentlang_smoke");
    std::fs::create_dir_all(&dir).unwrap();
    let unique = format!(
        "smoke_{}.al",
        std::process::id() as u64 * 1000 + unique_id()
    );
    let file = dir.join(unique);
    std::fs::write(&file, source).unwrap();

    let output = Command::new(al_bin())
        .args([cmd, file.to_str().unwrap(), "--format", format])
        .output()
        .expect("failed to execute al");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    std::fs::remove_file(&file).ok();

    (stdout, stderr, code)
}

// =========================================================================
// Help / usage output
// =========================================================================

#[test]
fn smoke_no_args_shows_usage() {
    let (_stdout, stderr, code) = run_al(&[]);
    assert_ne!(code, 0, "no args should exit non-zero");
    assert!(
        stderr.contains("Usage: al"),
        "should show usage: {}",
        stderr
    );
    assert!(
        stderr.contains("Commands:"),
        "should list commands: {}",
        stderr
    );
}

#[test]
fn smoke_usage_lists_all_commands() {
    let (_stdout, stderr, _code) = run_al(&[]);
    assert!(stderr.contains("lex"), "usage should list lex: {}", stderr);
    assert!(
        stderr.contains("parse"),
        "usage should list parse: {}",
        stderr
    );
    assert!(
        stderr.contains("check"),
        "usage should list check: {}",
        stderr
    );
    assert!(stderr.contains("run"), "usage should list run: {}", stderr);
}

#[test]
fn smoke_usage_mentions_format_flag() {
    let (_stdout, stderr, _code) = run_al(&[]);
    assert!(
        stderr.contains("--format"),
        "usage should mention --format: {}",
        stderr
    );
}

// =========================================================================
// al lex — command UX
// =========================================================================

#[test]
fn smoke_lex_valid_source() {
    let source = "OPERATION test => BODY { EMIT 42 }";
    let (stdout, _stderr, code) = run_al_cmd("lex", source);
    assert_eq!(code, 0, "lex should succeed for valid source");
    assert!(
        stdout.contains("OK:"),
        "lex output should contain OK: {}",
        stdout
    );
    assert!(
        stdout.contains("tokens"),
        "lex output should mention token count: {}",
        stdout
    );
}

#[test]
fn smoke_lex_shows_token_positions() {
    let source = "TYPE Foo = Int64";
    let (stdout, _stderr, code) = run_al_cmd("lex", source);
    assert_eq!(code, 0);
    // Tokens should have line:column format
    assert!(
        stdout.contains("1:"),
        "tokens should show line numbers: {}",
        stdout
    );
}

#[test]
fn smoke_lex_missing_file_exits_nonzero() {
    let (_, stderr, code) = run_al(&["lex", "/nonexistent/file.al"]);
    assert_ne!(code, 0, "missing file should exit non-zero");
    assert!(stderr.contains("error"), "should report error: {}", stderr);
}

// =========================================================================
// al parse — command UX
// =========================================================================

#[test]
fn smoke_parse_valid_source() {
    let source = r#"
TYPE UserId = Int64
SCHEMA User => { name: Str }
OPERATION fetch => BODY { EMIT 42 }
PIPELINE Main => fetch
"#;
    let (stdout, _stderr, code) = run_al_cmd("parse", source);
    assert_eq!(code, 0, "parse should succeed");
    assert!(
        stdout.contains("OK:"),
        "parse output should contain OK: {}",
        stdout
    );
    assert!(stdout.contains("TYPE"), "should list TYPE: {}", stdout);
    assert!(stdout.contains("SCHEMA"), "should list SCHEMA: {}", stdout);
    assert!(
        stdout.contains("OPERATION"),
        "should list OPERATION: {}",
        stdout
    );
    assert!(
        stdout.contains("PIPELINE"),
        "should list PIPELINE: {}",
        stdout
    );
}

#[test]
fn smoke_parse_reports_declaration_count() {
    let source = r#"
OPERATION a => BODY { EMIT 1 }
OPERATION b => BODY { EMIT 2 }
PIPELINE Main => a -> b
"#;
    let (stdout, _stderr, code) = run_al_cmd("parse", source);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("3 declarations"),
        "should report 3 declarations: {}",
        stdout
    );
}

#[test]
fn smoke_parse_error_shows_diagnostic() {
    let source = "OPERATION ??? => BODY { }";
    let (_stdout, stderr, code) = run_al_cmd("parse", source);
    assert_ne!(code, 0, "parse error should exit non-zero");
    assert!(
        stderr.contains("error"),
        "should show error diagnostic: {}",
        stderr
    );
}

// =========================================================================
// al check — command UX
// =========================================================================

#[test]
fn smoke_check_valid_source() {
    let source = r#"
OPERATION produce => BODY { EMIT 42 }
PIPELINE Main => produce
"#;
    let (stdout, _stderr, code) = run_al_cmd("check", source);
    assert_eq!(code, 0, "check should pass");
    assert!(
        stdout.contains("OK: type check passed"),
        "should report type check passed: {}",
        stdout
    );
}

#[test]
fn smoke_check_reports_env_summary() {
    let source = r#"
TYPE Rank = Int64
SCHEMA User => { name: Str }
AGENT Worker => CAPABILITIES [FILE_READ]
OPERATION fetch => BODY { EMIT 42 }
PIPELINE Main => fetch
"#;
    let (stdout, _stderr, code) = run_al_cmd("check", source);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("types") && stdout.contains("schemas") && stdout.contains("agents"),
        "should report environment summary: {}",
        stdout
    );
}

#[test]
fn smoke_check_type_error_exits_nonzero() {
    let source = r#"
TYPE Dup = Int64
TYPE Dup = Str
"#;
    let (_stdout, stderr, code) = run_al_cmd("check", source);
    assert_ne!(code, 0, "type error should exit non-zero");
    assert!(
        stderr.contains("error"),
        "should show type error: {}",
        stderr
    );
}

// =========================================================================
// al run — command UX
// =========================================================================

#[test]
fn smoke_run_shows_all_phases() {
    let source = r#"
OPERATION produce => BODY { EMIT 42 }
PIPELINE Main => produce
"#;
    let (stdout, _stderr, code) = run_al_cmd("run", source);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("Phase 1 (lex)"),
        "should show Phase 1: {}",
        stdout
    );
    assert!(
        stdout.contains("Phase 2 (parse)"),
        "should show Phase 2: {}",
        stdout
    );
    assert!(
        stdout.contains("Phase 3 (check)"),
        "should show Phase 3: {}",
        stdout
    );
    assert!(
        stdout.contains("Phase 4 (caps)"),
        "should show Phase 4: {}",
        stdout
    );
    assert!(
        stdout.contains("Phase 5 (exec)"),
        "should show Phase 5: {}",
        stdout
    );
}

#[test]
fn smoke_run_shows_result() {
    let source = r#"
OPERATION produce => BODY { EMIT 42 }
PIPELINE Main => produce
"#;
    let (stdout, _stderr, code) = run_al_cmd("run", source);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("Result: 42"),
        "should show result: {}",
        stdout
    );
}

#[test]
fn smoke_run_missing_file_exits_nonzero() {
    let (_, stderr, code) = run_al(&["run", "/nonexistent/file.al"]);
    assert_ne!(code, 0, "missing file should exit non-zero");
    assert!(stderr.contains("error"), "should report error: {}", stderr);
}

// =========================================================================
// --format flag
// =========================================================================

#[test]
fn smoke_format_json_on_error() {
    let source = r#"
TYPE Dup = Int64
TYPE Dup = Str
"#;
    let (_stdout, stderr, code) = run_al_cmd_format("check", source, "json");
    assert_ne!(code, 0);
    // JSON format should produce structured output
    assert!(
        stderr.contains('{') && stderr.contains('}'),
        "json format should produce JSON: {}",
        stderr
    );
}

#[test]
fn smoke_format_jsonl_on_error() {
    let source = r#"
TYPE Dup = Int64
TYPE Dup = Str
"#;
    let (_stdout, stderr, code) = run_al_cmd_format("check", source, "jsonl");
    assert_ne!(code, 0);
    // JSONL format should produce structured output
    assert!(
        stderr.contains('{'),
        "jsonl format should produce JSON lines: {}",
        stderr
    );
}

#[test]
fn smoke_format_human_is_default() {
    let source = r#"
TYPE Dup = Int64
TYPE Dup = Str
"#;
    let (_stdout1, stderr1, _) = run_al_cmd_format("check", source, "human");
    let (_stdout2, stderr2, _) = run_al_cmd("check", source);
    // Human format should be the same whether specified or default
    assert_eq!(stderr1, stderr2, "human format should be the default");
}

// =========================================================================
// Exit code semantics
// =========================================================================

#[test]
fn smoke_exit_0_on_success() {
    let source = "OPERATION a => BODY { EMIT 1 }\nPIPELINE Main => a";
    let (_, _, code) = run_al_cmd("run", source);
    assert_eq!(code, 0, "successful run should exit 0");
}

#[test]
fn smoke_exit_nonzero_on_runtime_error() {
    let source = r#"
OPERATION test => BODY { ESCALATE("fail") }
PIPELINE Main => test
"#;
    let (_, _, code) = run_al_cmd("run", source);
    assert_ne!(code, 0, "runtime error should exit non-zero");
}

#[test]
fn smoke_exit_nonzero_unknown_command_runs_as_file() {
    // Unknown command is treated as a file path — should fail because file doesn't exist
    let (_, stderr, code) = run_al(&["nonexistent_command"]);
    assert_ne!(
        code, 0,
        "unknown command (treated as file) should exit non-zero"
    );
    assert!(stderr.contains("error"), "should show error: {}", stderr);
}

// =========================================================================
// Example file smoke tests
// =========================================================================

#[test]
fn smoke_example_calculate() {
    let output = Command::new(al_bin())
        .args(["run", "examples/calculate.al"])
        .current_dir(env!("CARGO_MANIFEST_DIR").replace("/crates/al-cli", ""))
        .output()
        .expect("failed to execute al");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout.contains("Result: 94"), "expected 94: {}", stdout);
}

#[test]
fn smoke_example_factorial() {
    let output = Command::new(al_bin())
        .args(["run", "examples/factorial.al"])
        .current_dir(env!("CARGO_MANIFEST_DIR").replace("/crates/al-cli", ""))
        .output()
        .expect("failed to execute al");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout.contains("Result: 720"), "expected 720: {}", stdout);
}

#[test]
fn smoke_example_match_result() {
    let output = Command::new(al_bin())
        .args(["run", "examples/match_result.al"])
        .current_dir(env!("CARGO_MANIFEST_DIR").replace("/crates/al-cli", ""))
        .output()
        .expect("failed to execute al");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout.contains("Result: 84"), "expected 84: {}", stdout);
}

// =========================================================================
// Subcommand-specific missing file behavior
// =========================================================================

#[test]
fn smoke_lex_no_file_shows_usage() {
    let (_stdout, stderr, code) = run_al(&["lex"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("Usage: al lex"),
        "should show lex usage: {}",
        stderr
    );
}

#[test]
fn smoke_parse_no_file_shows_usage() {
    let (_stdout, stderr, code) = run_al(&["parse"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("Usage: al parse"),
        "should show parse usage: {}",
        stderr
    );
}

#[test]
fn smoke_check_no_file_shows_usage() {
    let (_stdout, stderr, code) = run_al(&["check"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("Usage: al check"),
        "should show check usage: {}",
        stderr
    );
}

#[test]
fn smoke_run_no_file_shows_usage() {
    let (_stdout, stderr, code) = run_al(&["run"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("Usage: al run"),
        "should show run usage: {}",
        stderr
    );
}
