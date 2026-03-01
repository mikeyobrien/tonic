mod acceptance;
mod c_backend;
mod cache;
mod cli_diag;
mod deps;
mod docs;
mod formatter;
mod guard_builtins;
mod interop;
mod lexer;
mod manifest;
mod mir;
mod native_abi;
mod native_runtime;
mod parser;
mod resolver;
mod resolver_diag;
mod typing;

#[cfg(feature = "llvm-backend")]
mod llvm_backend;

use cli_diag::{CliDiagnostic, EXIT_FAIL, EXIT_OK};
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = run(&args);
    process::exit(result);
}

fn run(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("Usage: tonic <command> [options]");
        return EXIT_FAIL;
    }

    match args[1].as_str() {
        "check" => cmd_check(args),
        "run" => cmd_run(args),
        "fmt" => cmd_fmt(args),
        "docs" => cmd_docs(args),
        "version" => cmd_version(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            EXIT_FAIL
        }
    }
}