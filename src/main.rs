mod ast;
mod lexer;
mod parser;

use lexer::Lexer;
use parser::Parser;

fn main() {
    let input = "
        let math_magic = fn(x, y) do
            if x == 0 do
                return y
            else
                return x + y * 2
            end
        end
        
        let result = math_magic(10, 5 + 5)
    ";

    println!("Parsing Mochi script:\n{}", input);

    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    if !parser.errors.is_empty() {
        println!("Parser encountered {} error(s):", parser.errors.len());
        for err in parser.errors {
            println!("- {}", err);
        }
        return;
    }

    println!("AST generated successfully:");
    println!("{:#?}", program);
}
