#[cfg(test)]
mod tests {
    use crate::compiler::Compiler;
    use crate::lexer::Lexer;
    use crate::object::Object;
    use crate::parser::Parser;
    use crate::vm::VM;

    fn test_script(input: &str, expected: Object) {
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program();

        assert!(
            parser.errors.is_empty(),
            "Parser encountered errors: {:#?}",
            parser.errors
        );

        let mut compiler = Compiler::new();
        if let Err(e) = compiler.compile_program(&program) {
            panic!("Compiler error: {}", e);
        }

        let mut vm = VM::new(compiler.bytecode());
        if let Err(e) = vm.run() {
            panic!("VM error: {}", e);
        }

        assert_eq!(
            vm.last_popped_stack_elem,
            Some(expected),
            "Script failed: {}",
            input
        );
    }

    #[test]
    fn test_math_operations() {
        test_script("5", Object::Number(5.0));
        test_script("10", Object::Number(10.0));
        test_script("-5", Object::Number(-5.0));
        test_script("-10", Object::Number(-10.0));
        test_script("5 + 5 + 5 + 5 - 10", Object::Number(10.0));
        test_script("2*2*2*2*2", Object::Number(32.0));
        test_script("-50 + 100 + -50", Object::Number(0.0));
        test_script("5*2 + 10", Object::Number(20.0));
        test_script("5 + 2*10", Object::Number(25.0));
        test_script("20 + 2*-10", Object::Number(0.0));
        test_script("50 / 2*2 + 10", Object::Number(60.0));
        test_script("2*(5 + 10)", Object::Number(30.0));
        test_script("3*3*3 + 10", Object::Number(37.0));
        test_script("3*(3*3) + 10", Object::Number(37.0));
    }

    #[test]
    fn test_boolean_logic() {
        test_script("true", Object::Boolean(true));
        test_script("false", Object::Boolean(false));
        test_script("1 == 1", Object::Boolean(true));
        test_script("1 != 1", Object::Boolean(false));
        test_script("1 == 2", Object::Boolean(false));
        test_script("1 != 2", Object::Boolean(true));
        test_script("true == true", Object::Boolean(true));
        test_script("false == false", Object::Boolean(true));
        test_script("true == false", Object::Boolean(false));
        test_script("true != false", Object::Boolean(true));
    }

    #[test]
    fn test_bang_prefix() {
        test_script("!true", Object::Boolean(false));
        test_script("!false", Object::Boolean(true));
        test_script("!5", Object::Boolean(false));
        test_script("!!true", Object::Boolean(true));
        test_script("!!false", Object::Boolean(false));
        test_script("!!5", Object::Boolean(true));
    }

    #[test]
    fn test_global_variables() {
        test_script("let a = 5 a", Object::Number(5.0));
        test_script("let a = 5*5 a", Object::Number(25.0));
        test_script("let a = 5 let b = a b", Object::Number(5.0));
        test_script(
            "let a = 5 let b = a let c = a + b + 5 c",
            Object::Number(15.0),
        );
    }

    #[test]
    fn test_if_else_expressions() {
        test_script("if true do 10 end", Object::Number(10.0));
        test_script("if false do 10 end", Object::Boolean(false));
        test_script("if 1 do 10 end", Object::Number(10.0));
        test_script("if 1 != 2 do 10 else 20 end", Object::Number(10.0));
        test_script("if 1 == 2 do 10 else 20 end", Object::Number(20.0));
    }

    #[test]
    fn test_functions() {
        let input = "
            let identity = fn(a) do return a end
            identity(5)
        ";
        test_script(input, Object::Number(5.0));

        let input2 = "
            let multiply = fn(x, y) do return x*y end
            multiply(5, 5)
        ";
        test_script(input2, Object::Number(25.0));

        let input3 = "
            let add_three = fn(x, y, z) do return x + y + z end
            add_three(10, 20, 30)
        ";
        test_script(input3, Object::Number(60.0));
    }

    #[test]
    fn test_functions_with_local_state() {
        let input = "
            let complex_math = fn(x, y) do
                let z = 10
                let multiplier = 2
                return (x + y + z)*multiplier
            end
            complex_math(5, 5)
        ";
        test_script(input, Object::Number(40.0));
    }

    #[test]
    fn test_functions_reading_globals() {
        let input = "
            let global_val = 100
            let add_to_global = fn(x) do return x + global_val end
            add_to_global(50)
        ";
        test_script(input, Object::Number(150.0));
    }
}
