//! AgentLang CLI - MVP v0.1
//!
//! Provides a command-line interface for lexing, parsing, and type-checking
//! AgentLang source files.

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("AgentLang CLI v0.1 - MVP");
        eprintln!("Usage: al-cli <command> [file.al]");
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  lex    <file>  Tokenize and print tokens");
        eprintln!("  parse  <file>  Parse and print AST summary");
        eprintln!("  check  <file>  Type-check a source file");
        eprintln!("  run    <file>  Parse, check, and summarize");
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "lex" => {
            if args.len() < 3 {
                eprintln!("Usage: al-cli lex <file.al>");
                process::exit(1);
            }
            cmd_lex(&args[2]);
        }
        "parse" => {
            if args.len() < 3 {
                eprintln!("Usage: al-cli parse <file.al>");
                process::exit(1);
            }
            cmd_parse(&args[2]);
        }
        "check" => {
            if args.len() < 3 {
                eprintln!("Usage: al-cli check <file.al>");
                process::exit(1);
            }
            cmd_check(&args[2]);
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("Usage: al-cli run <file.al>");
                process::exit(1);
            }
            cmd_run(&args[2]);
        }
        _ => {
            // If no command given, treat first arg as a file to run
            cmd_run(command);
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

fn cmd_lex(path: &str) {
    let source = read_source(path);
    match al_lexer::tokenize(&source) {
        Ok(tokens) => {
            for tok in &tokens {
                println!("  {}:{} {}", tok.span.line, tok.span.column, tok.token);
            }
            println!("OK: {} tokens", tokens.len());
        }
        Err(diags) => {
            for d in &diags {
                eprintln!(
                    "error[{}]: {} ({}:{})",
                    d.code, d.message, d.span.line, d.span.column
                );
            }
            process::exit(1);
        }
    }
}

fn cmd_parse(path: &str) {
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
            for d in &diags {
                eprintln!(
                    "error[{}]: {} ({}:{})",
                    d.code, d.message, d.span.line, d.span.column
                );
            }
            process::exit(1);
        }
    }
}

fn cmd_check(path: &str) {
    let source = read_source(path);
    let program = match al_parser::parse(&source) {
        Ok(p) => p,
        Err(diags) => {
            for d in &diags {
                eprintln!(
                    "error[{}]: {} ({}:{})",
                    d.code, d.message, d.span.line, d.span.column
                );
            }
            process::exit(1);
        }
    };

    let mut checker = al_types::TypeChecker::new();
    checker.check(&program);

    if checker.has_errors() {
        let diags = checker.take_diagnostics();
        for d in diags.errors() {
            eprintln!(
                "error[{}]: {} ({}:{})",
                d.code, d.message, d.span.line, d.span.column
            );
        }
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

fn cmd_run(path: &str) {
    let source = read_source(path);

    // Phase 1: Lex
    let tokens = match al_lexer::tokenize(&source) {
        Ok(t) => t,
        Err(diags) => {
            for d in &diags {
                eprintln!(
                    "lex error[{}]: {} ({}:{})",
                    d.code, d.message, d.span.line, d.span.column
                );
            }
            process::exit(1);
        }
    };
    println!("Phase 1 (lex):   {} tokens", tokens.len());

    // Phase 2: Parse
    let program = match al_parser::parse(&source) {
        Ok(p) => p,
        Err(diags) => {
            for d in &diags {
                eprintln!(
                    "parse error[{}]: {} ({}:{})",
                    d.code, d.message, d.span.line, d.span.column
                );
            }
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
        for d in diags.errors() {
            eprintln!(
                "type error[{}]: {} ({}:{})",
                d.code, d.message, d.span.line, d.span.column
            );
        }
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
