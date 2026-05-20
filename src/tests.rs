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

        let mut checker = crate::type_checker::TypeChecker::new();
        let env = std::rc::Rc::new(std::cell::RefCell::new(crate::type_checker::TypeEnv::new()));
        for expr in &program.expressions {
            if let Err(e) = checker.check(expr, &env) {
                panic!("Typechecker error for '{}': {}", input, e.message);
            }
        }

        let mut compiler = Compiler::new();
        if let Err(e) = compiler.compile_program(&program) {
            panic!("Compiler error for '{}': {}", input, e.message);
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

    fn test_type_error(input: &str, expected_substring: &str) {
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program();

        assert!(
            parser.errors.is_empty(),
            "Parser encountered errors for '{}': {:#?}",
            input,
            parser.errors
        );

        let mut checker = crate::type_checker::TypeChecker::new();
        let env = std::rc::Rc::new(std::cell::RefCell::new(crate::type_checker::TypeEnv::new()));
        let mut actual_error = None;

        for expr in &program.expressions {
            if let Err(e) = checker.check(expr, &env) {
                actual_error = Some(e.message);
                break;
            }
        }

        match actual_error {
            Some(err) => {
                assert!(
                    err.contains(expected_substring),
                    "Expected static type error containing '{}', but got '{}' in program:\n{}",
                    expected_substring,
                    err,
                    input
                );
            }
            None => {
                panic!(
                    "Expected program to fail typecheck with error containing '{}', but it compiled successfully:\n{}",
                    expected_substring, input
                );
            }
        }
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
    fn test_comparisons() {
        test_script("1 == 1", Object::Atom("true".to_string()));
        test_script("1 == 2", Object::Atom("false".to_string()));
        test_script("0 == 0", Object::Atom("true".to_string()));

        test_script("1 != 1", Object::Atom("false".to_string()));
        test_script("1 != 2", Object::Atom("true".to_string()));
        test_script("0 != 0", Object::Atom("false".to_string()));

        test_script(":true == :true", Object::Atom("true".to_string()));
        test_script(":false == :false", Object::Atom("true".to_string()));
        test_script(":true == :false", Object::Atom("false".to_string()));
        test_script(":false == :true", Object::Atom("false".to_string()));

        test_script(":true != :true", Object::Atom("false".to_string()));
        test_script(":false != :false", Object::Atom("false".to_string()));
        test_script(":true != :false", Object::Atom("true".to_string()));
        test_script(":false != :true", Object::Atom("true".to_string()));
    }

    #[test]
    fn test_bang_operator() {
        test_script("!:true", Object::Atom("false".to_string()));
        test_script("!:false", Object::Atom("true".to_string()));
        test_script("!5", Object::Atom("false".to_string()));
        test_script("!0", Object::Atom("false".to_string()));

        test_script("!!:true", Object::Atom("true".to_string()));
        test_script("!!:false", Object::Atom("false".to_string()));
        test_script("!!5", Object::Atom("true".to_string()));
        test_script("!!0", Object::Atom("true".to_string()));
    }

    #[test]
    fn test_global_variables() {
        test_script("let a = 5 a", Object::Number(5.0));
        test_script("let a = 10 a", Object::Number(10.0));
        test_script("let a = :true a", Object::Atom("true".to_string()));
        test_script("let a = :false a", Object::Atom("false".to_string()));

        test_script("let a = 5 * 5 a", Object::Number(25.0));
        test_script("let a = 10 + 20 a", Object::Number(30.0));

        test_script("let a = 5 let b = 10 a + b", Object::Number(15.0));
        test_script("let a = 5 let b = a b", Object::Number(5.0));
        test_script("let a = 5 let b = a let c = a + b c", Object::Number(10.0));

        test_script("let a = 5 let a = 10 a", Object::Number(10.0));
    }

    #[test]
    fn test_variable_assignment() {
        test_script("let a = 5 a = 10 a", Object::Number(10.0));

        test_script(
            "let a = 5 let b = 10 a = b = 20 a + b",
            Object::Number(40.0),
        );

        let input = "
            fn mutate() do
                let a = 10
                a = 20
                return a
            end
            mutate()
        ";
        test_script(input, Object::Number(20.0));
    }

    #[test]
    fn test_constants() {
        test_script("const a = 5 a", Object::Number(5.0));
        test_script("const a = 10 let b = a * 2 b", Object::Number(20.0));
    }

    #[test]
    #[should_panic(expected = "Cannot reassign constant")]
    fn test_reassign_const_fails() {
        test_script("const a = 10 a = 20", Object::Number(0.0));
    }

    #[test]
    #[should_panic(expected = "Cannot reassign constant")]
    fn test_reassign_named_function_fails() {
        test_script(
            "fn math_magic() do return 5 end math_magic = 10",
            Object::Number(0.0),
        );
    }

    #[test]
    fn test_if_else_expressions() {
        test_script("if :true do 10 end", Object::Number(10.0));
        test_script("if :false do 10 end", Object::Atom("null".to_string()));
        test_script("if 1 do 10 end", Object::Number(10.0));
        test_script("if 0 do 10 end", Object::Number(10.0));

        test_script("if :true do 10 else 20 end", Object::Number(10.0));
        test_script("if :false do 10 else 20 end", Object::Number(20.0));

        test_script("if 1 == 1 do 10 else 20 end", Object::Number(10.0));
        test_script("if 1 == 2 do 10 else 20 end", Object::Number(20.0));
        test_script("if 1 != 2 do 10 else 20 end", Object::Number(10.0));
        test_script("if 1 != 1 do 10 else 20 end", Object::Number(20.0));

        test_script(
            "if :true do if :true do 10 else 20 end else 30 end",
            Object::Number(10.0),
        );
        test_script(
            "if :true do if :false do 10 else 20 end else 30 end",
            Object::Number(20.0),
        );
        test_script(
            "if :false do if :true do 10 else 20 end else 30 end",
            Object::Number(30.0),
        );

        test_script("if 5 > 3 do 10 else 20 end", Object::Number(10.0));
    }

    #[test]
    fn test_functions_basic() {
        test_script(
            "fn identity(a) do return a end identity(5)",
            Object::Number(5.0),
        );
        test_script(
            "fn always_five() do return 5 end always_five()",
            Object::Number(5.0),
        );
        test_script(
            "fn add(a, b) do return a + b end add(3, 4)",
            Object::Number(7.0),
        );
        test_script(
            "fn add_three(a, b, c) do return a + b + c end add_three(10, 20, 30)",
            Object::Number(60.0),
        );
    }

    #[test]
    fn test_functions_return_values() {
        test_script(
            "fn double(x) do return x * 2 end double(5)",
            Object::Number(10.0),
        );
        test_script(
            "fn is_positive(x) do return x > 0 end is_positive(5)",
            Object::Atom("true".to_string()),
        );
        test_script(
            "fn calc(x) do return x + x * x end calc(3)",
            Object::Number(12.0),
        );
    }

    #[test]
    fn test_functions_local_state() {
        let input = "
            fn complex_math(x, y) do
                let z = 10
                let multiplier = 2
                return (x + y + z) * multiplier
            end
            complex_math(5, 5)
        ";
        test_script(input, Object::Number(40.0));

        let input = "
            fn calculate(a, b) do
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
            fn add_to_global(x) do return x + global_val end
            add_to_global(50)
        ";
        test_script(input, Object::Number(150.0));

        let input = "
            let x = 10
            let y = 20
            fn add() do return x + y end
            add()
        ";
        test_script(input, Object::Number(30.0));
    }

    #[test]
    fn test_functions_recursion() {
        let input = "
            fn factorial(n) do
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
            fn fib(n) do
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
            fn make_adder(x) do
                return fn(y) do return x + y end
            end
            let add5 = make_adder(5)
            add5(10)
        ";
        test_script(input, Object::Number(15.0));

        let input = "
            fn apply_twice(f, x) do return f(f(x)) end
            fn double(x) do return x * 2 end
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
            fn get_x() do return x end
            get_x()
        ";
        test_script(input, Object::Number(10.0));
    }

    #[test]
    fn test_complex_programs() {
        let input = "
            fn add(a, b) do return a + b end
            fn multiply(a, b) do return a * b end
            let result = add(multiply(2, 3), multiply(4, 5))
            result
        ";
        test_script(input, Object::Number(26.0));

        let input = "
            fn compose(f, g) do
                return fn(x) do return f(g(x)) end
            end
            fn add1(x) do return x + 1 end
            fn mul2(x) do return x * 2 end
            let add1_then_mul2 = compose(mul2, add1)
            add1_then_mul2(5)
        ";
        test_script(input, Object::Number(12.0));

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
    fn test_everything_is_an_expression() {
        test_script("let x = 100", Object::Number(100.0));
        test_script("let x = let y = 100", Object::Number(100.0));

        test_script("let a = 5 + (let b = 10) a + b", Object::Number(25.0));

        test_script("let x = if :true do 10 else 20 end x", Object::Number(10.0));
        test_script(
            "let x = if :false do 10 else 20 end x",
            Object::Number(20.0),
        );

        test_script(
            "let x = if :true do let y = 5 y else 0 end x",
            Object::Number(5.0),
        );
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
            fn add(a, b) a + b
            fn result(x) do return add(x, x) end
            result(5)
        ";
        test_script(input, Object::Number(10.0));

        let input = "
            fn add(a, b) do return a + b end
            fn result(x) add(x, x)
            result(5)
        ";
        test_script(input, Object::Number(10.0));

        let input = "
            fn check(x) if x > 0 do 1 else -1 end
            check(5)
        ";
        test_script(input, Object::Number(1.0));

        let input = "
            fn check(x) do if x > 0 1 else -1 end
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
    #[should_panic(expected = "Function arity mismatch: expected 2 arguments, got 3")]
    fn test_wrong_number_of_arguments() {
        let input = "
            fn add(a, b) do return a + b end
            add(1, 2, 3)
        ";
        test_script(input, Object::Number(0.0));
    }

    #[test]
    fn test_comments() {
        let input = "
            let x = 10 -- This is a comment
            -- This entire line is ignored
            let y = 5 -- Another comment
            x + y -- Result should be 15
        ";
        test_script(input, Object::Number(15.0));
    }

    #[test]
    fn test_arrays() {
        test_script("let list = [10, 20, 30] list[1]", Object::Number(20.0));
        test_script(
            "let list = [10 + 10, 20 * 2] list[0] + list[1]",
            Object::Number(60.0),
        );

        test_script(
            "let list = [1, 2] list[5]",
            Object::Atom("null".to_string()),
        );

        let input = "
            let data = [{ value: 10 }, [1, 2, 99]]
            data[0].value + data[1][2]
        ";
        test_script(input, Object::Number(109.0));

        let input2 = "
            let original = [1, 2, 3]
            let reference = original
            
            reference[0] = 100
            reference[3] = 400
            
            original[0] + original[3]
        ";
        test_script(input2, Object::Number(500.0));
    }

    #[test]
    fn test_objects_and_mutability() {
        test_script(
            "let obj = { x: 10, y: 20 } obj.x + obj.y",
            Object::Number(30.0),
        );

        test_script(
            "let obj = { name: \"Mochi\" } obj[\"name\"]",
            Object::String("Mochi".to_string()),
        );

        test_script(
            "let obj = { a: 1 } obj.a = 100 obj.a",
            Object::Number(100.0),
        );

        test_script("let obj = {} obj.missing", Object::Atom("null".to_string()));

        let input = "
            let a = { score: 10 }
            let b = a
            b.score = 999
            a.score
        ";
        test_script(input, Object::Number(999.0));
    }

    #[test]
    fn test_atoms() {
        test_script(":ok", Object::Atom("ok".to_string()));
        test_script(":error == :error", Object::Atom("true".to_string()));
        test_script(":ok == :error", Object::Atom("false".to_string()));
        test_script(":ok != :error", Object::Atom("true".to_string()));

        let input = "
            let status = :success
            if status == :success do
                100
            else
                0
            end
        ";
        test_script(input, Object::Number(100.0));
    }

    #[test]
    fn test_method_calls() {
        let input = "
            const user = {
                points: 0,
                bump: fn(self) self.points = self.points + 1
            }
            user:bump()
            user:bump()
            user.points
        ";
        test_script(input, Object::Number(2.0));

        let input2 = "
            const bank = {
                balance: 100,
                deposit: fn(self, amount) self.balance = self.balance + amount
            }
            bank:deposit(50)
            bank.balance
        ";
        test_script(input2, Object::Number(150.0));
    }

    #[test]
    fn test_static_type_checking() {
        test_type_error(
            ":ok == \"ok\"",
            "Type mismatch: cannot unify 'Atom' with 'String'",
        );
        test_type_error(
            "1 != :false",
            "Type mismatch: cannot unify 'Number' with 'Atom'",
        );

        test_type_error(
            "1 + \"hello\"",
            "Type mismatch: cannot unify 'Number' with 'String'",
        );
        test_type_error(
            "\"hello\" - 5",
            "Type mismatch: cannot unify 'String' with 'Number'",
        );

        test_type_error(
            "const limit = 100 \n limit = 200",
            "Cannot reassign constant 'limit'",
        );

        test_type_error(
            "let status = :ok \n status = 50",
            "Type mismatch: cannot unify 'Atom' with 'Number'",
        );

        test_type_error(
            "let value = 42 \n value(10)",
            "Type mismatch: cannot unify 'Number'",
        );

        test_type_error(
            "let primitive = 3.14 \n primitive:bump()",
            "receiver is of type 'Number', which is not an object",
        );

        test_type_error(
            "let primitive = :atom \n primitive[0]",
            "Index operator not supported on type 'Atom'",
        );

        test_script("let x: Number = 5 x", Object::Number(5.0));
        test_script(
            "const arr: [Number] = [1, 2, 3] arr[1]",
            Object::Number(2.0),
        );
        test_script(
            "fn add(a: Number, b: Number): Number do return a + b end add(10, 20)",
            Object::Number(30.0),
        );
        test_script(
            "const user: { name: String, age: Number } = { name: \"Hugo\", age: 26 } user.age",
            Object::Number(26.0),
        );

        test_type_error(
            "let x: String = 42",
            "Type mismatch: cannot unify 'Number' with 'String'",
        );
        test_type_error(
            "const arr: [Number] = [1, \"mismatch\", 3]",
            "Type mismatch: cannot unify 'String' with 'Number'",
        );
        test_type_error(
            "fn get_name(name: String): String do return 42 end",
            "Type mismatch: cannot unify 'Number' with 'String'",
        );
        test_type_error(
            "const user: { name: String, age: Number } = { name: \"Hugo\", age: :unknown }",
            "Type mismatch: cannot unify 'Atom' with 'Number'",
        );

        test_type_error(
            "const user: { name: String, age: Number } = { name: \"Hugo\", number: 3 }",
            "Record type mismatch",
        );
        test_type_error(
            "const user: { name: String, age: Number } = { name: \"Hugo\" }",
            "Record type mismatch: missing required fields",
        );

        test_script(
            "const add: fn(x: Number, y: Number): Number = fn(x, y) x + y \n add(3, 4)",
            Object::Number(7.0),
        );

        test_type_error(
            "const add: fn(x: Number, y: Number): Number = fn(x, y) \"mismatch\"",
            "Type mismatch: cannot unify 'String' with 'Number'",
        );
    }

    #[test]
    fn test_while_loops() {
        let input = "
            let i = 0
            let sum = 0
            while i < 10 do
                sum = sum + i
                i = i + 1
            end
            sum
        ";
        test_script(input, Object::Number(45.0));
    }

    #[test]
    fn test_for_array_loops() {
        let input = "
            let list = [10, 20, 30]
            let sum = 0
            for x in list do
                sum = sum + x
            end
            sum
        ";
        test_script(input, Object::Number(60.0));
    }

    #[test]
    fn test_for_hash_loops() {
        let input = "
            let obj = { a: 10, b: 20 }
            let sum = 0
            for k, v in obj do
                sum = sum + v
            end
            sum
        ";
        test_script(input, Object::Number(30.0));
    }

    #[test]
    fn test_loop_type_safety() {
        test_script(
            "const list: [Number] = [1, 2, 3] let sum: Number = 0 for x in list do sum = sum + x end sum",
            Object::Number(6.0),
        );

        test_type_error(
            "const list: [String] = [\"a\"] let sum: Number = 0 for x in list do sum = sum + x end",
            "cannot unify 'Number' with 'String'",
        );
    }
}
