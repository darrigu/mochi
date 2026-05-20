mod ast;
mod code;
mod compiler;
mod error_reporter;
mod lexer;
mod object;
mod parser;
mod serializer;
mod tests;
mod type_checker;
mod vm;

use crate::compiler::Bytecode;
use crate::compiler::Compiler;
use crate::lexer::Lexer;
use crate::object::Object;
use crate::parser::Parser;
use std::env;
use std::fs;
use std::process;

fn builtin_print(args: Vec<Object>) -> Object {
    for arg in args.iter() {
        match arg {
            Object::String(s) => print!("{}", s),
            _ => print!("{:?}", arg),
        }
    }
    println!();
    Object::Atom("null".to_string())
}

fn compile_source(source: &str) -> Bytecode {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    if !parser.errors.is_empty() {
        error_reporter::report_errors(source, &parser.errors);
        process::exit(1);
    }

    let mut checker = type_checker::TypeChecker::new();
    let env = std::rc::Rc::new(std::cell::RefCell::new(type_checker::TypeEnv::new()));
    for expr in &program.expressions {
        if let Err(e) = checker.check(expr, &env) {
            error_reporter::report_errors(source, &[e]);
            process::exit(1);
        }
    }

    let mut compiler = Compiler::new();
    if let Err(e) = compiler.compile_program(&program) {
        error_reporter::report_errors(source, &[e]);
        process::exit(1);
    }

    compiler.bytecode()
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Mochi Language CLI");
        println!("Usage:");
        println!("  mochi run <file.moc>    - Parse and run a source file immediately");
        println!("  mochi run <file.anko>   - Execute pre-compiled bytecode");
        println!("  mochi build <file.moc>  - Compile a source file to an .anko binary");
        process::exit(1);
    }

    let command = &args[1];
    let filename = &args[2];

    if command == "build" {
        let source = fs::read_to_string(filename).unwrap_or_else(|_| {
            println!("\x1b[31;1merror\x1b[0m: Could not read file '{}'", filename);
            process::exit(1);
        });

        let bytecode = compile_source(&source);
        let out_filename = filename.replace(".moc", ".anko");

        let bytes = serializer::serialize(&bytecode);
        fs::write(&out_filename, bytes).expect("Failed to write .anko file");

        println!(
            "\x1b[32;1mSuccess\x1b[0m: Compiled {} -> {}",
            filename, out_filename
        );
    } else if command == "run" {
        let bytecode = if filename.ends_with(".anko") {
            let bytes = fs::read(filename).unwrap_or_else(|_| {
                println!(
                    "\x1b[31;1merror\x1b[0m: Could not read binary '{}'",
                    filename
                );
                process::exit(1);
            });
            serializer::deserialize(&bytes).unwrap_or_else(|e| {
                error_reporter::report_system_error("deserializer", &e);
                process::exit(1);
            })
        } else {
            let source = fs::read_to_string(filename).unwrap_or_else(|_| {
                println!("\x1b[31;1merror\x1b[0m: Could not read file '{}'", filename);
                process::exit(1);
            });
            compile_source(&source)
        };

        let mut machine = vm::VM::new(bytecode);
        machine.set_global(0, Object::Native(builtin_print));

        if let Err(e) = machine.run() {
            error_reporter::report_system_error("runtime", &e);
            process::exit(1);
        }
    } else {
        println!("\x1b[31;1merror\x1b[0m: Unknown command '{}'", command);
        process::exit(1);
    }
}
