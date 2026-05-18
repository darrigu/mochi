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
            "Parser encountered errors for '{}': {:#?}",
            input,
            parser.errors
        );

        let mut compiler = Compiler::new();
        if let Err(e) = compiler.compile_program(&program) {
            panic!("Compiler error for '{}': {}", input, e);
        }

        let mut vm = VM::new(compiler.bytecode());
        if let Err(e) = vm.run() {
            panic!("VM error for '{}': {}", input, e);
        }

        assert_eq!(
            vm.last_popped_stack_elem,
            Some(expected),
            "Script failed: {}",
            input
        );

        assert_eq!(
            vm.sp,
            0,
            "Stack not clean after '{}': sp = {}, stack = {:?}",
            input,
            vm.sp,
            &vm.stack[..vm.sp.min(10)]
        );
    }

    #[test]
    fn test_number_literals() {
        test_script("0", Object::Number(0.0));
        test_script("1", Object::Number(1.0));
        test_script("42", Object::Number(42.0));
        test_script("1000000", Object::Number(1000000.0));
        test_script("3.14", Object::Number(3.14));
        test_script("0.5", Object::Number(0.5));
        test_script("100.0", Object::Number(100.0));
    }

    #[test]
    fn test_basic_arithmetic() {
        test_script("1 + 1", Object::Number(2.0));
        test_script("0 + 0", Object::Number(0.0));
        test_script("10 + 20", Object::Number(30.0));

        test_script("5 - 3", Object::Number(2.0));
        test_script("3 - 5", Object::Number(-2.0));
        test_script("0 - 0", Object::Number(0.0));

        test_script("2 * 3", Object::Number(6.0));
        test_script("0 * 100", Object::Number(0.0));
        test_script("1 * 1", Object::Number(1.0));

        test_script("10 / 2", Object::Number(5.0));
        test_script("9 / 3", Object::Number(3.0));
        test_script("7 / 2", Object::Number(3.5));
    }

    #[test]
    fn test_complex_arithmetic() {
        test_script("5 + 5 + 5 + 5 - 10", Object::Number(10.0));
        test_script("2 * 2 * 2 * 2 * 2", Object::Number(32.0));
        test_script("-50 + 100 + -50", Object::Number(0.0));

        test_script("5*2 + 10", Object::Number(20.0));
        test_script("5 + 2*10", Object::Number(25.0));
        test_script("20 + 2*-10", Object::Number(0.0));
        test_script("50 / 2*2 + 10", Object::Number(60.0));

        test_script("2*(5 + 10)", Object::Number(30.0));
        test_script("(5 + 5)*2", Object::Number(20.0));
        test_script("((2 + 3)*4)", Object::Number(20.0));
        test_script("(10 - 5)*(3 + 2)", Object::Number(25.0));

        test_script("((1 + 2)*(3 + 4))", Object::Number(21.0));
        test_script("(((1 + 2)))", Object::Number(3.0));
    }

    #[test]
    fn test_negation() {
        test_script("-5", Object::Number(-5.0));
        test_script("-10", Object::Number(-10.0));
        test_script("-0", Object::Number(0.0));
        test_script("-(-5)", Object::Number(5.0));
        test_script("-(-(-5))", Object::Number(-5.0));
        test_script("-(5 + 5)", Object::Number(-10.0));
        test_script("-5 + 10", Object::Number(5.0));
    }

    #[test]
    fn test_boolean_literals() {
        test_script("true", Object::Boolean(true));
        test_script("false", Object::Boolean(false));
    }

    #[test]
    fn test_comparisons() {
        test_script("1 == 1", Object::Boolean(true));
        test_script("1 == 2", Object::Boolean(false));
        test_script("0 == 0", Object::Boolean(true));

        test_script("1 != 1", Object::Boolean(false));
        test_script("1 != 2", Object::Boolean(true));
        test_script("0 != 0", Object::Boolean(false));

        test_script("true == true", Object::Boolean(true));
        test_script("false == false", Object::Boolean(true));
        test_script("true == false", Object::Boolean(false));
        test_script("false == true", Object::Boolean(false));

        test_script("true != true", Object::Boolean(false));
        test_script("false != false", Object::Boolean(false));
        test_script("true != false", Object::Boolean(true));
        test_script("false != true", Object::Boolean(true));
    }

    #[test]
    fn test_bang_operator() {
        test_script("!true", Object::Boolean(false));
        test_script("!false", Object::Boolean(true));
        test_script("!5", Object::Boolean(false));
        test_script("!0", Object::Boolean(false));

        test_script("!!true", Object::Boolean(true));
        test_script("!!false", Object::Boolean(false));
        test_script("!!5", Object::Boolean(true));
        test_script("!!0", Object::Boolean(true));
    }

    #[test]
    fn test_global_variables() {
        test_script("let a = 5 a", Object::Number(5.0));
        test_script("let a = 10 a", Object::Number(10.0));
        test_script("let a = true a", Object::Boolean(true));
        test_script("let a = false a", Object::Boolean(false));

        test_script("let a = 5 * 5 a", Object::Number(25.0));
        test_script("let a = 10 + 20 a", Object::Number(30.0));

        test_script("let a = 5 let b = 10 a + b", Object::Number(15.0));
        test_script("let a = 5 let b = a b", Object::Number(5.0));
        test_script("let a = 5 let b = a let c = a + b c", Object::Number(10.0));

        test_script("let a = 5 let a = 10 a", Object::Number(10.0));
    }

    #[test]
    fn test_if_else_expressions() {
        test_script("if true do 10 end", Object::Number(10.0));
        test_script("if false do 10 end", Object::Boolean(false));
        test_script("if 1 do 10 end", Object::Number(10.0));
        test_script("if 0 do 10 end", Object::Number(10.0));

        test_script("if true do 10 else 20 end", Object::Number(10.0));
        test_script("if false do 10 else 20 end", Object::Number(20.0));

        test_script("if 1 == 1 do 10 else 20 end", Object::Number(10.0));
        test_script("if 1 == 2 do 10 else 20 end", Object::Number(20.0));
        test_script("if 1 != 2 do 10 else 20 end", Object::Number(10.0));
        test_script("if 1 != 1 do 10 else 20 end", Object::Number(20.0));

        test_script(
            "if true do if true do 10 else 20 end else 30 end",
            Object::Number(10.0),
        );
        test_script(
            "if true do if false do 10 else 20 end else 30 end",
            Object::Number(20.0),
        );
        test_script(
            "if false do if true do 10 else 20 end else 30 end",
            Object::Number(30.0),
        );

        test_script("if 5 > 3 do 10 else 20 end", Object::Number(10.0));
    }

    #[test]
    fn test_functions_basic() {
        let input = "
            let identity = fn(a) do return a end
            identity(5)
        ";
        test_script(input, Object::Number(5.0));

        let input = "
            let always_five = fn() do return 5 end
            always_five()
        ";
        test_script(input, Object::Number(5.0));

        let input = "
            let add = fn(a, b) do return a + b end
            add(3, 4)
        ";
        test_script(input, Object::Number(7.0));

        let input = "
            let add_three = fn(a, b, c) do return a + b + c end
            add_three(10, 20, 30)
        ";
        test_script(input, Object::Number(60.0));
    }

    #[test]
    fn test_functions_return_values() {
        let input = "
            let double = fn(x) do return x * 2 end
            double(5)
        ";
        test_script(input, Object::Number(10.0));

        let input = "
            let is_positive = fn(x) do return x > 0 end
            is_positive(5)
        ";
        test_script(input, Object::Boolean(true));

        let input = "
            let calc = fn(x) do return x + x * x end
            calc(3)
        ";
        test_script(input, Object::Number(12.0));
    }

    #[test]
    fn test_functions_local_state() {
        let input = "
            let complex_math = fn(x, y) do
                let z = 10
                let multiplier = 2
                return (x + y + z) * multiplier
            end
            complex_math(5, 5)
        ";
        test_script(input, Object::Number(40.0));

        let input = "
            let calculate = fn(a, b) do
                let sum = a + b
                let product = a * b
                return sum + product
            end
            calculate(3, 4)
        ";
        test_script(input, Object::Number(19.0));
    }

    #[test]
    fn test_functions_global_access() {
        let input = "
            let global_val = 100
            let add_to_global = fn(x) do return x + global_val end
            add_to_global(50)
        ";
        test_script(input, Object::Number(150.0));

        let input = "
            let x = 10
            let y = 20
            let add = fn() do return x + y end
            add()
        ";
        test_script(input, Object::Number(30.0));
    }

    #[test]
    fn test_functions_recursion() {
        let input = "
            let factorial = fn(n) do
                if n == 0 do
                    return 1
                else
                    return n * factorial(n - 1)
                end
            end
            factorial(5)
        ";
        test_script(input, Object::Number(120.0));

        let input = "
            let fib = fn(n) do
                if n == 0 do return 0
                else if n == 1 do return 1
                else return fib(n - 1) + fib(n - 2)
                end
                end
            end
            fib(10)
        ";
        test_script(input, Object::Number(55.0));
    }

    #[test]
    fn test_functions_higher_order() {
        let input = "
            let apply_twice = fn(f, x) do return f(f(x)) end
            let double = fn(x) do return x * 2 end
            apply_twice(double, 5)
        ";
        test_script(input, Object::Number(20.0));
    }

    #[test]
    fn test_block_expressions() {
        test_script("do 5 end", Object::Number(5.0));
        test_script("do 1 + 2 end", Object::Number(3.0));

        test_script("do let x = 5 x end", Object::Number(5.0));
        test_script("do let x = 5 let y = 10 x + y end", Object::Number(15.0));

        test_script("do do 5 end end", Object::Number(5.0));
        test_script("do do let x = 5 x end end", Object::Number(5.0));
    }

    #[test]
    fn test_closures() {
        let input = "
            let x = 10
            let get_x = fn() do return x end
            get_x()
        ";
        test_script(input, Object::Number(10.0));
    }

    #[test]
    fn test_complex_programs() {
        let input = "
            let add = fn(a, b) do return a + b end
            let multiply = fn(a, b) do return a * b end
            let result = add(multiply(2, 3), multiply(4, 5))
            result
        ";
        test_script(input, Object::Number(26.0));

        let input = "
            let a = 5
            let b = 10
            let c = fn(x) do return x * 2 end
            let result = if a < b do c(a + b) else c(a - b) end
            result
        ";
        test_script(input, Object::Number(30.0))
    }

    #[test]
    fn test_edge_cases() {
        test_script("42", Object::Number(42.0));

        test_script("((((1 + 2))))", Object::Number(3.0));

        test_script(
            "1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10",
            Object::Number(55.0),
        );
    }

    #[test]
    fn test_mixed_syntax() {
        let input = "
            let add = fn(a, b) a + b
            let result = fn(x) do return add(x, x) end
            result(5)
        ";
        test_script(input, Object::Number(10.0));

        let input = "
            let add = fn(a, b) do return a + b end
            let result = fn(x) add(x, x)
            result(5)
        ";
        test_script(input, Object::Number(10.0));

        let input = "
            let check = fn(x) if x > 0 do 1 else -1 end
            check(5)
        ";
        test_script(input, Object::Number(1.0));

        let input = "
            let check = fn(x) do if x > 0 1 else -1 end
            check(5)
        ";
        test_script(input, Object::Number(1.0));
    }

    #[test]
    #[should_panic(expected = "Undefined variable")]
    fn test_undefined_variable() {
        test_script("x", Object::Number(0.0));
    }

    #[test]
    #[should_panic(expected = "Wrong number of arguments")]
    fn test_wrong_number_of_arguments() {
        let input = "
            let add = fn(a, b) do return a + b end
            add(1, 2, 3)
        ";
        test_script(input, Object::Number(0.0));
    }
}
