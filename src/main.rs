mod ast;
mod code;
mod compiler;
mod error_reporter;
mod lexer;
mod object;
mod parser;
mod tests;
mod vm;

use crate::object::Object;
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

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Mochi Language Compiler");
        println!("Usage: mochi <filename.moc>");
        process::exit(1);
    }

    let source = fs::read_to_string(&args[1]).expect("Could not read file");

    let lexer = lexer::Lexer::new(&source);
    let mut parser = parser::Parser::new(lexer);
    let program = parser.parse_program();

    if !parser.errors.is_empty() {
        error_reporter::report_errors(&source, &parser.errors);
        process::exit(1);
    }

    let mut compiler = compiler::Compiler::new();
    if let Err(e) = compiler.compile_program(&program) {
        error_reporter::report_system_error("compiler", &e);
        process::exit(1);
    }

    let mut machine = vm::VM::new(compiler.bytecode());
    machine.set_global(0, Object::Native(builtin_print));

    if let Err(e) = machine.run() {
        error_reporter::report_system_error("runtime", &e);
        process::exit(1);
    }
}
