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

/// Parsed CLI arguments.
struct CliArgs {
    command: String,
    file: Option<String>,
    format: OutputFormat,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = env::args().collect();
    let mut format = OutputFormat::Human;
    let mut positional = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
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
    }
}

fn main() {
    let cli = parse_args();

    if cli.command.is_empty() {
        eprintln!("AgentLang CLI v0.1 - MVP");
        eprintln!("Usage: al <command> [file.al] [--format human|json|jsonl]");
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  lex    <file>  Tokenize and print tokens");
        eprintln!("  parse  <file>  Parse and print AST summary");
        eprintln!("  check  <file>  Type-check a source file");
        eprintln!("  run    <file>  Parse, check, and execute");
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
        Ok(tokens) => {
            for tok in &tokens {
                println!("  {}:{} {}", tok.span.line, tok.span.column, tok.token);
            }
            println!("OK: {} tokens", tokens.len());
        }
        Err(diags) => {
            emit_diagnostics(&diags, &source, format);
            process::exit(1);
        }
    }
}

fn cmd_parse(path: &str, format: OutputFormat) {
    let source = read_source(path);
    match al_parser::parse(&source) {
        Ok(program) => {
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
    println!("Phase 1 (lex):   {} tokens", tokens.len());

    // Phase 2: Parse
    let program = match al_parser::parse(&source) {
        Ok(p) => p,
        Err(diags) => {
            emit_diagnostics(&diags, &source, format);
            process::exit(1);
        }
    };
    println!(
        "Phase 2 (parse): {} declarations",
        program.declarations.len()
    );

    // Phase 3: Type check
    let mut checker = al_types::TypeChecker::new();
    checker.check(&program);

    if checker.has_errors() {
        let diags = checker.take_diagnostics();
        let errors: Vec<_> = diags.errors().into_iter().cloned().collect();
        emit_diagnostics(&errors, &source, format);
        process::exit(1);
    }
    println!("Phase 3 (check): passed");

    // Phase 4: Capability check (static)
    println!(
        "Phase 4 (caps):  {} agents registered",
        checker.env.agents.len()
    );

    // Phase 5: Execute
    let mut interp = al_runtime::interpreter::Interpreter::new();
    interp.load_program(&program);

    match interp.run() {
        Ok(result) => {
            println!("Phase 5 (exec):  OK");
            println!();
            println!("Result: {}", result);
        }
        Err(e) => {
            eprintln!("Phase 5 (exec):  FAILED");
            eprintln!("  {}", e);
            process::exit(1);
        }
    }
}
