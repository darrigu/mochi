mod ast;
mod code;
mod compiler;
mod lexer;
mod object;
mod parser;
mod vm;

mod tests;

use compiler::Compiler;
use lexer::Lexer;
use parser::Parser;
use vm::VM;

fn main() {
    // FIXME
    let input = "
        let math_magic = fn(x, y) do
           let test = fn(z) do
              return x + y + z
           end
           return test(20)
        end
        
        let result = math_magic(10, 5)
        result
    ";

    println!("Compiling Mochi script:\n{}\n", input);

    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    if !parser.errors.is_empty() {
        println!("Parser errors: {:#?}", parser.errors);
        return;
    }

    let mut compiler = Compiler::new();
    if let Err(e) = compiler.compile_program(&program) {
        println!("Compiler error: {}", e);
        return;
    }

    let bytecode = compiler.bytecode();
    let mut machine = VM::new(bytecode);

    if let Err(e) = machine.run() {
        println!("VM error: {}", e);
        return;
    }

    println!("SUCCESS!");
    if let Some(result) = machine.last_popped_stack_elem {
        println!("Final Output: {:?}", result);
    }
}
