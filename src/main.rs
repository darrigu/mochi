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
use crate::vm::VM;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
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

fn compile_source(source: &str, file_path: &str) -> Result<Bytecode, ()> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    if !parser.errors.is_empty() {
        println!("\x1b[31;1mSyntax errors in {}\x1b[0m:", file_path);
        error_reporter::report_errors(source, &parser.errors);
        return Err(());
    }

    let mut checker = type_checker::TypeChecker::new();
    let env = checker.new_env();
    for expr in &program.expressions {
        if let Err(e) = checker.check(expr, env) {
            println!("\x1b[31;1mType error in {}\x1b[0m:", file_path);
            error_reporter::report_errors(source, &[e]);
            return Err(());
        }
    }

    let mut compiler = Compiler::new();
    if let Err(e) = compiler.compile_program(&program) {
        println!("\x1b[31;1mCompilation error in {}\x1b[0m:", file_path);
        error_reporter::report_errors(source, &[e]);
        return Err(());
    }

    Ok(compiler.bytecode())
}

fn resolve_import(path: &str) -> Result<Object, String> {
    let source = fs::read_to_string(path)
        .map_err(|_| format!("Could not read imported module '{}'", path))?;

    let bytecode =
        compile_source(&source, path).map_err(|_| format!("Failed to compile '{}'", path))?;

    let mut machine = VM::new(bytecode);

    let path_obj = Path::new(path);
    let parent = path_obj.parent().unwrap_or(Path::new(""));
    machine.current_dir = if parent.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        parent.to_path_buf()
    };

    machine.import_handler = Some(resolve_import);
    machine.set_global(0, Object::Native(builtin_print));

    machine.run()?;

    Ok(machine
        .last_popped_stack_elem
        .unwrap_or_else(|| Object::Atom("null".to_string())))
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
    let file_path = &args[2];

    if command == "build" {
        let source = fs::read_to_string(file_path).unwrap_or_else(|_| {
            println!(
                "\x1b[31;1merror\x1b[0m: Could not read file '{}'",
                file_path
            );
            process::exit(1);
        });

        let bytecode = compile_source(&source, file_path).unwrap_or_else(|_| process::exit(1));
        let out_file_path = file_path.replace(".moc", ".anko");

        let bytes = serializer::serialize(&bytecode);
        fs::write(&out_file_path, bytes).expect("Failed to write .anko file");

        println!(
            "\x1b[32;1mSuccess\x1b[0m: Compiled {} -> {}",
            file_path, out_file_path
        );
    } else if command == "run" {
        let bytecode = if file_path.ends_with(".anko") {
            let bytes = fs::read(file_path).unwrap_or_else(|_| {
                println!(
                    "\x1b[31;1merror\x1b[0m: Could not read binary '{}'",
                    file_path
                );
                process::exit(1);
            });
            serializer::deserialize(&bytes).unwrap_or_else(|e| {
                error_reporter::report_system_error("deserializer", &e);
                process::exit(1);
            })
        } else {
            let source = fs::read_to_string(file_path).unwrap_or_else(|_| {
                println!(
                    "\x1b[31;1merror\x1b[0m: Could not read file '{}'",
                    file_path
                );
                process::exit(1);
            });
            compile_source(&source, file_path).unwrap_or_else(|_| process::exit(1))
        };

        let mut machine = VM::new(bytecode);

        let path_obj = Path::new(file_path);
        let parent = path_obj.parent().unwrap_or(Path::new(""));
        machine.current_dir = if parent.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            parent.to_path_buf()
        };

        machine.import_handler = Some(resolve_import);
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
