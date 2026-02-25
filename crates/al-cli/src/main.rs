//! AgentLang CLI - MVP v0.1
//!
//! Provides a command-line interface for lexing, parsing, type-checking,
//! and executing AgentLang source files.
//!
//! Supports `--format human|json|jsonl` for diagnostic output.

use al_diagnostics::{render_diagnostic, OutputFormat};
use std::env;
use std::fs;
use std::process;

/// Version string for the CLI.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Parsed CLI arguments.
struct CliArgs {
    command: String,
    file: Option<String>,
    format: OutputFormat,
    help: bool,
    version: bool,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = env::args().collect();
    let mut format = OutputFormat::Human;
    let mut positional = Vec::new();
    let mut help = false;
    let mut version = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                help = true;
            }
            "--version" | "-V" => {
                version = true;
            }
            "--format" => {
                i += 1;
                if i < args.len() {
                    format = match args[i].as_str() {
                        "json" => OutputFormat::Json,
                        "jsonl" => OutputFormat::Jsonl,
                        _ => OutputFormat::Human,
                    };
                }
            }
            _ => positional.push(args[i].clone()),
        }
        i += 1;
    }

    CliArgs {
        command: positional.first().cloned().unwrap_or_default(),
        file: positional.get(1).cloned(),
        format,
        help,
        version,
    }
}

fn print_usage() {
    eprintln!("AgentLang CLI v{}", VERSION);
    eprintln!("Usage: al <command> [file.al] [--format human|json|jsonl]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  lex    <file>  Tokenize and print tokens");
    eprintln!("  parse  <file>  Parse and print AST summary");
    eprintln!("  check  <file>  Type-check a source file");
    eprintln!("  run    <file>  Parse, check, and execute");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --format <fmt>  Output format: human, json, jsonl (default: human)");
    eprintln!("  --help, -h      Show this help message");
    eprintln!("  --version, -V   Show version information");
}

fn print_version() {
    println!("al {}", VERSION);
}

fn main() {
    let cli = parse_args();

    if cli.version {
        print_version();
        process::exit(0);
    }

    if cli.help || cli.command.is_empty() {
        print_usage();
        if cli.help {
            process::exit(0);
        }
        process::exit(1);
    }

    match cli.command.as_str() {
        "lex" => {
            let path = cli.file.as_deref().unwrap_or_else(|| {
                eprintln!("Usage: al lex <file.al>");
                process::exit(1);
            });
            cmd_lex(path, cli.format);
        }
        "parse" => {
            let path = cli.file.as_deref().unwrap_or_else(|| {
                eprintln!("Usage: al parse <file.al>");
                process::exit(1);
            });
            cmd_parse(path, cli.format);
        }
        "check" => {
            let path = cli.file.as_deref().unwrap_or_else(|| {
                eprintln!("Usage: al check <file.al>");
                process::exit(1);
            });
            cmd_check(path, cli.format);
        }
        "run" => {
            let path = cli.file.as_deref().unwrap_or_else(|| {
                eprintln!("Usage: al run <file.al>");
                process::exit(1);
            });
            cmd_run(path, cli.format);
        }
        other => {
            // If no command given, treat first arg as a file to run
            cmd_run(other, cli.format);
        }
    }
}

fn read_source(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("error: could not read '{}': {}", path, e);
            process::exit(1);
        }
    }
}

fn emit_diagnostics(diags: &[al_diagnostics::Diagnostic], source: &str, format: OutputFormat) {
    for d in diags {
        eprintln!("{}", render_diagnostic(d, source, format));
    }
}

fn cmd_lex(path: &str, format: OutputFormat) {
    let source = read_source(path);
    match al_lexer::tokenize(&source) {
        Ok(tokens) => match format {
            OutputFormat::Json => {
                let items: Vec<serde_json::Value> = tokens
                    .iter()
                    .map(|tok| {
                        serde_json::json!({
                            "line": tok.span.line,
                            "column": tok.span.column,
                            "token": tok.token.to_string()
                        })
                    })
                    .collect();
                let result = serde_json::json!({
                    "status": "ok",
                    "command": "lex",
                    "count": tokens.len(),
                    "tokens": items
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            OutputFormat::Jsonl => {
                for tok in &tokens {
                    let item = serde_json::json!({
                        "line": tok.span.line,
                        "column": tok.span.column,
                        "token": tok.token.to_string()
                    });
                    println!("{}", serde_json::to_string(&item).unwrap());
                }
            }
            OutputFormat::Human => {
                for tok in &tokens {
                    println!("  {}:{} {}", tok.span.line, tok.span.column, tok.token);
                }
                println!("OK: {} tokens", tokens.len());
            }
        },
        Err(diags) => {
            emit_diagnostics(&diags, &source, format);
            process::exit(1);
        }
    }
}

fn cmd_parse(path: &str, format: OutputFormat) {
    let source = read_source(path);
    match al_parser::parse(&source) {
        Ok(program) => match format {
            OutputFormat::Json => {
                let decls: Vec<serde_json::Value> = program
                    .declarations
                    .iter()
                    .map(|decl| match &decl.node {
                        al_ast::Declaration::TypeDecl { name, .. } => {
                            serde_json::json!({"kind": "TYPE", "name": name.node})
                        }
                        al_ast::Declaration::SchemaDecl { name, fields } => {
                            serde_json::json!({"kind": "SCHEMA", "name": name.node, "fields": fields.len()})
                        }
                        al_ast::Declaration::AgentDecl { name, properties } => {
                            serde_json::json!({"kind": "AGENT", "name": name.node, "properties": properties.len()})
                        }
                        al_ast::Declaration::OperationDecl { name, body, .. } => {
                            serde_json::json!({"kind": "OPERATION", "name": name.node, "statements": body.node.stmts.len()})
                        }
                        al_ast::Declaration::PipelineDecl { name, chain } => {
                            serde_json::json!({"kind": "PIPELINE", "name": name.node, "stages": chain.node.stages.len()})
                        }
                    })
                    .collect();
                let result = serde_json::json!({
                    "status": "ok",
                    "command": "parse",
                    "count": program.declarations.len(),
                    "declarations": decls
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            OutputFormat::Jsonl => {
                for decl in &program.declarations {
                    let item = match &decl.node {
                        al_ast::Declaration::TypeDecl { name, .. } => {
                            serde_json::json!({"kind": "TYPE", "name": name.node})
                        }
                        al_ast::Declaration::SchemaDecl { name, fields } => {
                            serde_json::json!({"kind": "SCHEMA", "name": name.node, "fields": fields.len()})
                        }
                        al_ast::Declaration::AgentDecl { name, properties } => {
                            serde_json::json!({"kind": "AGENT", "name": name.node, "properties": properties.len()})
                        }
                        al_ast::Declaration::OperationDecl { name, body, .. } => {
                            serde_json::json!({"kind": "OPERATION", "name": name.node, "statements": body.node.stmts.len()})
                        }
                        al_ast::Declaration::PipelineDecl { name, chain } => {
                            serde_json::json!({"kind": "PIPELINE", "name": name.node, "stages": chain.node.stages.len()})
                        }
                    };
                    println!("{}", serde_json::to_string(&item).unwrap());
                }
            }
            OutputFormat::Human => {
                println!("OK: {} declarations", program.declarations.len());
                for decl in &program.declarations {
                    match &decl.node {
                        al_ast::Declaration::TypeDecl { name, .. } => {
                            println!("  TYPE {}", name.node);
                        }
                        al_ast::Declaration::SchemaDecl { name, fields } => {
                            println!("  SCHEMA {} ({} fields)", name.node, fields.len());
                        }
                        al_ast::Declaration::AgentDecl { name, properties } => {
                            println!("  AGENT {} ({} properties)", name.node, properties.len());
                        }
                        al_ast::Declaration::OperationDecl { name, body, .. } => {
                            println!(
                                "  OPERATION {} ({} statements)",
                                name.node,
                                body.node.stmts.len()
                            );
                        }
                        al_ast::Declaration::PipelineDecl { name, chain } => {
                            println!(
                                "  PIPELINE {} ({} stages)",
                                name.node,
                                chain.node.stages.len()
                            );
                        }
                    }
                }
            }
        },
        Err(diags) => {
            emit_diagnostics(&diags, &source, format);
            process::exit(1);
        }
    }
}

fn cmd_check(path: &str, format: OutputFormat) {
    let source = read_source(path);
    let program = match al_parser::parse(&source) {
        Ok(p) => p,
        Err(diags) => {
            emit_diagnostics(&diags, &source, format);
            process::exit(1);
        }
    };

    let mut checker = al_types::TypeChecker::new();
    checker.check(&program);

    if checker.has_errors() {
        let diags = checker.take_diagnostics();
        let errors: Vec<_> = diags.errors().into_iter().cloned().collect();
        emit_diagnostics(&errors, &source, format);
        process::exit(1);
    }

    match format {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "ok",
                "command": "check",
                "summary": {
                    "types": checker.env.types.len(),
                    "schemas": checker.env.schemas.len(),
                    "agents": checker.env.agents.len(),
                    "operations": checker.env.operations.len(),
                    "pipelines": checker.env.pipelines.len()
                }
            });
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        OutputFormat::Jsonl => {
            let result = serde_json::json!({
                "status": "ok",
                "command": "check",
                "summary": {
                    "types": checker.env.types.len(),
                    "schemas": checker.env.schemas.len(),
                    "agents": checker.env.agents.len(),
                    "operations": checker.env.operations.len(),
                    "pipelines": checker.env.pipelines.len()
                }
            });
            println!("{}", serde_json::to_string(&result).unwrap());
        }
        OutputFormat::Human => {
            println!("OK: type check passed");
            println!(
                "  {} types, {} schemas, {} agents, {} operations, {} pipelines",
                checker.env.types.len(),
                checker.env.schemas.len(),
                checker.env.agents.len(),
                checker.env.operations.len(),
                checker.env.pipelines.len(),
            );
        }
    }
}

fn cmd_run(path: &str, format: OutputFormat) {
    let source = read_source(path);

    // Phase 1: Lex
    let tokens = match al_lexer::tokenize(&source) {
        Ok(t) => t,
        Err(diags) => {
            emit_diagnostics(&diags, &source, format);
            process::exit(1);
        }
    };
    let token_count = tokens.len();

    // Phase 2: Parse
    let program = match al_parser::parse(&source) {
        Ok(p) => p,
        Err(diags) => {
            emit_diagnostics(&diags, &source, format);
            process::exit(1);
        }
    };
    let decl_count = program.declarations.len();

    // Phase 3: Type check
    let mut checker = al_types::TypeChecker::new();
    checker.check(&program);

    if checker.has_errors() {
        let diags = checker.take_diagnostics();
        let errors: Vec<_> = diags.errors().into_iter().cloned().collect();
        emit_diagnostics(&errors, &source, format);
        process::exit(1);
    }
    let agent_count = checker.env.agents.len();

    // Phase 5: Execute
    let mut interp = al_runtime::interpreter::Interpreter::new();
    interp.load_program(&program);

    match interp.run() {
        Ok(result) => match format {
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "status": "ok",
                    "command": "run",
                    "phases": {
                        "lex": token_count,
                        "parse": decl_count,
                        "check": "passed",
                        "caps": agent_count
                    },
                    "result": result.to_string()
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            OutputFormat::Jsonl => {
                let output = serde_json::json!({
                    "status": "ok",
                    "command": "run",
                    "phases": {
                        "lex": token_count,
                        "parse": decl_count,
                        "check": "passed",
                        "caps": agent_count
                    },
                    "result": result.to_string()
                });
                println!("{}", serde_json::to_string(&output).unwrap());
            }
            OutputFormat::Human => {
                println!("Phase 1 (lex):   {} tokens", token_count);
                println!("Phase 2 (parse): {} declarations", decl_count);
                println!("Phase 3 (check): passed");
                println!("Phase 4 (caps):  {} agents registered", agent_count);
                println!("Phase 5 (exec):  OK");
                println!();
                println!("Result: {}", result);
            }
        },
        Err(e) => {
            match format {
                OutputFormat::Json => {
                    let output = serde_json::json!({
                        "status": "error",
                        "command": "run",
                        "phase": "exec",
                        "message": e.to_string()
                    });
                    eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                OutputFormat::Jsonl => {
                    let output = serde_json::json!({
                        "status": "error",
                        "command": "run",
                        "phase": "exec",
                        "message": e.to_string()
                    });
                    eprintln!("{}", serde_json::to_string(&output).unwrap());
                }
                OutputFormat::Human => {
                    eprintln!("Phase 5 (exec):  FAILED");
                    eprintln!("  {}", e);
                }
            }
            process::exit(1);
        }
    }
}
