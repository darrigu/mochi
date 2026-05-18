use crate::code::Opcode;
use crate::compiler::Bytecode;
use crate::object::Object;

const STACK_SIZE: usize = 2048;
const GLOBALS_SIZE: usize = 65536;

pub struct Frame {
    pub instructions: Vec<u8>,
    pub constants: Vec<Object>,
    pub free: Vec<Object>,
    pub ip: usize,
    pub bp: usize,
}

pub struct VM {
    frames: Vec<Frame>,
    pub stack: Vec<Object>,
    pub sp: usize,
    globals: Vec<Object>,
    pub last_popped_stack_elem: Option<Object>,
}

impl VM {
    pub fn new(bytecode: Bytecode) -> Self {
        let main_frame = Frame {
            instructions: bytecode.instructions,
            constants: bytecode.constants,
            free: vec![],
            ip: 0,
            bp: 0,
        };

        Self {
            frames: vec![main_frame],
            stack: vec![Object::Number(0.0); STACK_SIZE],
            sp: 0,
            globals: vec![Object::Number(0.0); GLOBALS_SIZE],
            last_popped_stack_elem: None,
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut frame = self.frames.pop().unwrap();

        while frame.ip < frame.instructions.len() {
            let op = Opcode::from(frame.instructions[frame.ip]);
            frame.ip += 1;

            match op {
                Opcode::OpConstant => {
                    let const_index = ((frame.instructions[frame.ip] as usize) << 8)
                        | (frame.instructions[frame.ip + 1] as usize);
                    frame.ip += 2;
                    self.push(frame.constants[const_index].clone())?;
                }
                Opcode::OpSetGlobal => {
                    let global_index = ((frame.instructions[frame.ip] as usize) << 8)
                        | (frame.instructions[frame.ip + 1] as usize);
                    frame.ip += 2;
                    let val = self.stack[self.sp - 1].clone();
                    self.globals[global_index] = val;
                }
                Opcode::OpGetGlobal => {
                    let global_index = ((frame.instructions[frame.ip] as usize) << 8)
                        | (frame.instructions[frame.ip + 1] as usize);
                    frame.ip += 2;
                    self.push(self.globals[global_index].clone())?;
                }
                Opcode::OpSetLocal => {
                    let local_idx = frame.instructions[frame.ip] as usize;
                    frame.ip += 1;
                    let val = self.stack[self.sp - 1].clone();
                    self.stack[frame.bp + local_idx] = val;
                }
                Opcode::OpGetLocal => {
                    let local_idx = frame.instructions[frame.ip] as usize;
                    frame.ip += 1;
                    self.push(self.stack[frame.bp + local_idx].clone())?;
                }
                Opcode::OpClosure => {
                    let const_idx = ((frame.instructions[frame.ip] as usize) << 8)
                        | (frame.instructions[frame.ip + 1] as usize);
                    let num_free = frame.instructions[frame.ip + 2] as usize;
                    frame.ip += 3;

                    let mut free = Vec::with_capacity(num_free);
                    for _ in 0..num_free {
                        free.push(self.pop());
                    }
                    free.reverse();

                    let func = frame.constants[const_idx].clone();
                    self.push(Object::Closure {
                        func: Box::new(func),
                        free,
                    })?;
                }
                Opcode::OpGetFree => {
                    let free_idx = frame.instructions[frame.ip] as usize;
                    frame.ip += 1;
                    self.push(frame.free[free_idx].clone())?;
                }
                Opcode::OpSetFree => {
                    let free_idx = frame.instructions[frame.ip] as usize;
                    frame.ip += 1;
                    let val = self.stack[self.sp - 1].clone();
                    frame.free[free_idx] = val;
                }
                Opcode::OpJump => {
                    let pos = ((frame.instructions[frame.ip] as usize) << 8)
                        | (frame.instructions[frame.ip + 1] as usize);
                    frame.ip = pos;
                }
                Opcode::OpJumpNotTruthy => {
                    let pos = ((frame.instructions[frame.ip] as usize) << 8)
                        | (frame.instructions[frame.ip + 1] as usize);
                    frame.ip += 2;
                    let condition = self.pop();
                    if !self.is_truthy(condition) {
                        frame.ip = pos;
                    }
                }
                Opcode::OpCall => {
                    let num_args = frame.instructions[frame.ip] as usize;
                    frame.ip += 1;

                    let func_obj = self.stack[self.sp - 1 - num_args].clone();

                    if let Object::Closure { func, free } = func_obj {
                        if let Object::CompiledFunction {
                            instructions,
                            constants,
                            num_locals,
                            num_parameters,
                        } = *func
                        {
                            if num_parameters != num_args {
                                return Err(format!("Wrong number of arguments"));
                            }

                            self.frames.push(frame);
                            let bp = self.sp - num_args;
                            self.sp = bp + num_locals;

                            frame = Frame {
                                instructions,
                                constants,
                                free,
                                ip: 0,
                                bp,
                            };
                        }
                    } else {
                        return Err("Calling non-function".to_string());
                    }
                }
                Opcode::OpReturnValue => {
                    let return_value = self.pop();
                    self.sp = frame.bp - 1;
                    self.push(return_value)?;
                    frame = self.frames.pop().unwrap();
                }
                Opcode::OpAdd | Opcode::OpSub | Opcode::OpMul | Opcode::OpDiv => {
                    self.execute_binary_operation(op)?
                }
                Opcode::OpTrue => self.push(Object::Boolean(true))?,
                Opcode::OpFalse => self.push(Object::Boolean(false))?,
                Opcode::OpEqual | Opcode::OpNotEqual | Opcode::OpGreater | Opcode::OpLess => {
                    self.execute_comparison(op)?
                }
                Opcode::OpMinus => self.execute_minus_operator()?,
                Opcode::OpBang => self.execute_bang_operator()?,
                Opcode::OpPop => {
                    self.pop();
                }
            }
        }

        if self.sp > 0 {
            self.last_popped_stack_elem = Some(self.pop());
        } else {
            self.last_popped_stack_elem = None;
        }

        Ok(())
    }

    fn execute_binary_operation(&mut self, op: Opcode) -> Result<(), String> {
        let right = self.pop();
        let left = self.pop();
        if let (Object::Number(l), Object::Number(r)) = (&left, &right) {
            let result = match op {
                Opcode::OpAdd => l + r,
                Opcode::OpSub => l - r,
                Opcode::OpMul => l * r,
                Opcode::OpDiv => l / r,
                _ => unreachable!(),
            };
            return self.push(Object::Number(result));
        }
        Err("Unsupported types for binary".to_string())
    }

    fn execute_comparison(&mut self, op: Opcode) -> Result<(), String> {
        let right = self.pop();
        let left = self.pop();
        if let (Object::Number(l), Object::Number(r)) = (&left, &right) {
            let result = match op {
                Opcode::OpEqual => l == r,
                Opcode::OpNotEqual => l != r,
                Opcode::OpGreater => l > r,
                Opcode::OpLess => l < r,
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
        if let Object::Number(val) = self.pop() {
            self.push(Object::Number(-val))
        } else {
            Err("Negation err".to_string())
        }
    }

    fn execute_bang_operator(&mut self) -> Result<(), String> {
        let result = match self.pop() {
            Object::Boolean(val) => !val,
            _ => false,
        };
        self.push(Object::Boolean(result))
    }

    fn is_truthy(&self, obj: Object) -> bool {
        match obj {
            Object::Boolean(val) => val,
            _ => true,
        }
    }

    fn push(&mut self, obj: Object) -> Result<(), String> {
        if self.sp >= STACK_SIZE {
            return Err("Stack overflow".to_string());
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
