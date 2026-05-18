use crate::code::Opcode;
use crate::compiler::Bytecode;
use crate::object::Object;

const STACK_SIZE: usize = 2048;
const GLOBALS_SIZE: usize = 65536;

pub struct VM {
    constants: Vec<Object>,
    instructions: Vec<u8>,

    stack: Vec<Object>,
    sp: usize,

    globals: Vec<Object>,

    pub last_popped_stack_elem: Option<Object>,
}

impl VM {
    pub fn new(bytecode: Bytecode) -> Self {
        Self {
            constants: bytecode.constants,
            instructions: bytecode.instructions,
            stack: vec![Object::Number(0.0); STACK_SIZE],
            sp: 0,
            globals: vec![Object::Number(0.0); GLOBALS_SIZE],
            last_popped_stack_elem: None,
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut ip = 0;

        while ip < self.instructions.len() {
            let op = Opcode::from(self.instructions[ip]);
            ip += 1;

            match op {
                Opcode::OpSetGlobal => {
                    let global_index = ((self.instructions[ip] as usize) << 8)
                        | (self.instructions[ip + 1] as usize);
                    ip += 2;

                    let val = self.pop();
                    self.globals[global_index] = val;
                }
                Opcode::OpGetGlobal => {
                    let global_index = ((self.instructions[ip] as usize) << 8)
                        | (self.instructions[ip + 1] as usize);
                    ip += 2;

                    let val = self.globals[global_index].clone();
                    self.push(val)?;
                }
                Opcode::OpConstant => {
                    let const_index = ((self.instructions[ip] as usize) << 8)
                        | (self.instructions[ip + 1] as usize);
                    ip += 2;

                    let constant = self.constants[const_index].clone();
                    self.push(constant)?;
                }
                Opcode::OpAdd | Opcode::OpSub | Opcode::OpMul | Opcode::OpDiv => {
                    self.execute_binary_operation(op)?;
                }
                Opcode::OpPop => {
                    let popped = self.pop();
                    self.last_popped_stack_elem = Some(popped);
                }
                Opcode::OpTrue => self.push(Object::Boolean(true))?,
                Opcode::OpFalse => self.push(Object::Boolean(false))?,
                Opcode::OpEqual | Opcode::OpNotEqual => self.execute_comparison(op)?,
                Opcode::OpMinus => self.execute_minus_operator()?,
                Opcode::OpBang => self.execute_bang_operator()?,
            }
        }

        Ok(())
    }

    fn execute_comparison(&mut self, op: Opcode) -> Result<(), String> {
        let right = self.pop();
        let left = self.pop();

        if let (Object::Number(l), Object::Number(r)) = (&left, &right) {
            let result = match op {
                Opcode::OpEqual => l == r,
                Opcode::OpNotEqual => l != r,
                _ => unreachable!(),
            };
            return self.push(Object::Boolean(result));
        }

        if let (Object::Boolean(l), Object::Boolean(r)) = (&left, &right) {
            let result = match op {
                Opcode::OpEqual => l == r,
                Opcode::OpNotEqual => l != r,
                _ => unreachable!(),
            };
            return self.push(Object::Boolean(result));
        }

        Err("Unsupported types for comparison".to_string())
    }

    fn execute_minus_operator(&mut self) -> Result<(), String> {
        let operand = self.pop();
        if let Object::Number(val) = operand {
            self.push(Object::Number(-val))
        } else {
            Err(format!("Unsupported type for negation: {:?}", operand))
        }
    }

    fn execute_bang_operator(&mut self) -> Result<(), String> {
        let operand = self.pop();
        let result = match operand {
            Object::Boolean(val) => !val,
            _ => false,
        };
        self.push(Object::Boolean(result))
    }

    fn execute_binary_operation(&mut self, op: Opcode) -> Result<(), String> {
        let right = self.pop();
        let left = self.pop();

        match (left, right) {
            (Object::Number(l), Object::Number(r)) => {
                let result = match op {
                    Opcode::OpAdd => l + r,
                    Opcode::OpSub => l - r,
                    Opcode::OpMul => l * r,
                    Opcode::OpDiv => l / r,
                    _ => unreachable!(),
                };
                self.push(Object::Number(result))?;
            }
            _ => return Err("Unsupported types for binary operation".to_string()),
        }

        Ok(())
    }

    fn push(&mut self, obj: Object) -> Result<(), String> {
        if self.sp >= STACK_SIZE {
            return Err("Stack overflow!".to_string());
        }
        self.stack[self.sp] = obj;
        self.sp += 1;
        Ok(())
    }

    fn pop(&mut self) -> Object {
        self.sp -= 1;
        self.stack[self.sp].clone()
    }
}
