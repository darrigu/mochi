use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use indexmap::IndexMap;

use crate::code::Opcode;
use crate::compiler::Bytecode;
use crate::object::Object;
use crate::stdlib;

const STACK_SIZE: usize = 2048;
const GLOBALS_SIZE: usize = 65536;

pub struct Frame {
    pub instructions: Vec<u8>,
    pub free: Vec<Object>,
    pub ip: usize,
    pub bp: usize,
}

pub struct VM {
    frames: Vec<Frame>,
    pub stack: Vec<Object>,
    pub sp: usize,
    globals: Vec<Object>,
    pub constants: Vec<Object>,
    pub last_popped_stack_elem: Option<Object>,
    pub loaded_modules: HashMap<String, Object>,
    pub import_handler: Option<fn(&str) -> Result<Object, String>>,
    pub current_dir: PathBuf,
}

#[inline]
fn read_u16(frame: &mut Frame) -> usize {
    let val = ((frame.instructions[frame.ip] as usize) << 8)
        | (frame.instructions[frame.ip + 1] as usize);
    frame.ip += 2;
    val
}

#[inline]
fn read_u8(frame: &mut Frame) -> usize {
    let val = frame.instructions[frame.ip] as usize;
    frame.ip += 1;
    val
}

#[inline]
fn to_key_string(obj: &Object) -> String {
    match obj {
        Object::String(s) | Object::Atom(s) => s.clone(),
        other => format!("{:?}", other),
    }
}

impl VM {
    pub fn new(bytecode: Bytecode) -> Self {
        let main_frame = Frame {
            instructions: bytecode.instructions,
            free: vec![],
            ip: 0,
            bp: 0,
        };

        Self {
            frames: vec![main_frame],
            stack: vec![Object::Number(0.0); STACK_SIZE],
            sp: 0,
            globals: vec![Object::Number(0.0); GLOBALS_SIZE],
            constants: bytecode.constants,
            last_popped_stack_elem: None,
            loaded_modules: HashMap::new(),
            import_handler: None,
            current_dir: PathBuf::from("."),
        }
    }

    pub fn set_global(&mut self, index: usize, obj: Object) {
        if index < GLOBALS_SIZE {
            self.globals[index] = obj;
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut frame = self.frames.pop().unwrap();

        while frame.ip < frame.instructions.len() {
            let op = Opcode::from(read_u8(&mut frame) as u8);

            match op {
                Opcode::OpConstant => {
                    let const_index = read_u16(&mut frame);
                    self.push(self.constants[const_index].clone())?;
                }

                Opcode::OpSetGlobal => {
                    let global_index = read_u16(&mut frame);
                    self.globals[global_index] = self.peek(0);
                }
                Opcode::OpGetGlobal => {
                    let global_index = read_u16(&mut frame);
                    self.push(self.globals[global_index].clone())?;
                }

                Opcode::OpSetLocal => {
                    let local_idx = read_u8(&mut frame);
                    self.stack[frame.bp + local_idx] = self.peek(0);
                }
                Opcode::OpGetLocal => {
                    let local_idx = read_u8(&mut frame);
                    self.push(self.stack[frame.bp + local_idx].clone())?;
                }

                Opcode::OpClosure => {
                    let const_idx = read_u16(&mut frame);
                    let num_free = read_u8(&mut frame);

                    let mut free = Vec::with_capacity(num_free);
                    for _ in 0..num_free {
                        free.push(self.pop());
                    }
                    free.reverse();

                    let func = self.constants[const_idx].clone();
                    self.push(Object::Closure {
                        func: Box::new(func),
                        free,
                    })?;
                }
                Opcode::OpGetFree => {
                    let free_idx = read_u8(&mut frame);
                    self.push(frame.free[free_idx].clone())?;
                }
                Opcode::OpSetFree => {
                    let free_idx = read_u8(&mut frame);
                    frame.free[free_idx] = self.peek(0);
                }

                Opcode::OpJump => {
                    frame.ip = read_u16(&mut frame);
                }
                Opcode::OpJumpNotTruthy => {
                    let pos = read_u16(&mut frame);
                    let condition = self.pop();
                    if !self.is_truthy(&condition) {
                        frame.ip = pos;
                    }
                }

                Opcode::OpCall => {
                    let num_args = read_u8(&mut frame);
                    let func_obj = self.peek(num_args);

                    match func_obj {
                        Object::Closure { func, free } => {
                            if let Object::CompiledFunction {
                                instructions,
                                num_locals,
                                num_parameters,
                            } = *func
                            {
                                if num_parameters != num_args {
                                    return Err("Wrong number of arguments".into());
                                }

                                self.frames.push(frame);
                                let bp = self.sp - num_args;
                                self.sp = bp + num_locals;

                                frame = Frame {
                                    instructions,
                                    free,
                                    ip: 0,
                                    bp,
                                };
                            }
                        }
                        Object::Native(func) => {
                            let args = self.stack[self.sp - num_args..self.sp].to_vec();
                            self.sp -= num_args + 1;
                            let result = func(args);
                            self.push(result)?;
                        }
                        _ => return Err("Calling non-function".into()),
                    }
                }
                Opcode::OpReturnValue => {
                    let return_value = self.pop();
                    self.sp = frame.bp - 1;
                    self.push(return_value)?;
                    frame = self.frames.pop().unwrap();
                }

                Opcode::OpArray => {
                    let num_elements = read_u16(&mut frame);

                    let start = self.sp - num_elements;
                    let array = self.stack[start..self.sp].to_vec();
                    self.sp = start;

                    self.push(Object::Array(Rc::new(RefCell::new(array))))?;
                }
                Opcode::OpHash => {
                    let num_pairs = read_u16(&mut frame);
                    let mut hash = IndexMap::new();

                    let mut temp = Vec::with_capacity(num_pairs * 2);
                    for _ in 0..(num_pairs * 2) {
                        temp.push(self.pop());
                    }
                    temp.reverse();

                    for i in (0..temp.len()).step_by(2) {
                        hash.insert(to_key_string(&temp[i]), temp[i + 1].clone());
                    }

                    self.push(Object::Hash(Rc::new(RefCell::new(hash))))?;
                }
                Opcode::OpArrayLen => {
                    let arr_obj = self.pop();
                    match arr_obj {
                        Object::Array(arr) => {
                            let len = arr.borrow().len();
                            self.push(Object::Number(len as f64))?;
                        }
                        _ => return Err("Expected array for length check".into()),
                    }
                }
                Opcode::OpHashKeys => {
                    let hash_obj = self.pop();
                    match hash_obj {
                        Object::Hash(hash) => {
                            let keys: Vec<Object> = hash
                                .borrow()
                                .keys()
                                .map(|k| Object::String(k.clone()))
                                .collect();
                            self.push(Object::Array(Rc::new(RefCell::new(keys))))?;
                        }
                        _ => return Err("Expected hash for keys extraction".into()),
                    }
                }
                Opcode::OpIndex => {
                    let index = self.pop();
                    let left = self.pop();

                    match left {
                        Object::Hash(hash) => {
                            let val = hash
                                .borrow()
                                .get(&to_key_string(&index))
                                .cloned()
                                .unwrap_or(Object::Atom("null".to_string()));
                            self.push(val)?;
                        }
                        Object::Array(arr) => {
                            if let Object::Number(idx) = index {
                                let i = idx as usize;
                                let val = if idx >= 0.0 && i < arr.borrow().len() {
                                    arr.borrow()[i].clone()
                                } else {
                                    Object::Atom("null".to_string())
                                };
                                self.push(val)?;
                            } else {
                                return Err("Array index must be a number".into());
                            }
                        }
                        Object::Tuple(elements) => {
                            if let Object::Number(idx) = index {
                                let i = idx as usize;
                                let val = if idx >= 0.0 && i < elements.len() {
                                    elements[i].clone()
                                } else {
                                    Object::Atom("null".to_string())
                                };
                                self.push(val)?;
                            } else {
                                return Err("Tuple index must be a number".into());
                            }
                        }
                        _ => return Err("Index operator not supported on this type".into()),
                    }
                }
                Opcode::OpSetIndex => {
                    let value = self.pop();
                    let index = self.pop();
                    let left = self.pop();

                    match left {
                        Object::Hash(hash) => {
                            hash.borrow_mut()
                                .insert(to_key_string(&index), value.clone());
                            self.push(value)?;
                        }
                        Object::Array(arr) => {
                            if let Object::Number(idx) = index {
                                if idx < 0.0 || idx.fract() != 0.0 {
                                    return Err("Array index must be a positive integer".into());
                                }
                                let i = idx as usize;
                                let mut array = arr.borrow_mut();

                                if i < array.len() {
                                    array[i] = value.clone();
                                } else if i == array.len() {
                                    array.push(value.clone());
                                } else {
                                    return Err("Array index out of bounds".into());
                                }
                                self.push(value)?;
                            } else {
                                return Err("Array index must be a number".into());
                            }
                        }
                        Object::Tuple(_) => return Err("Tuples are immutable".into()),
                        _ => return Err("Property assignment not supported on this type".into()),
                    }
                }

                Opcode::OpGetMethod => {
                    let const_idx = read_u16(&mut frame);
                    let method_name = self.constants[const_idx].clone();

                    let obj = self.pop();
                    let method_key = to_key_string(&method_name);

                    let func = match &obj {
                        Object::Hash(hash) => hash
                            .borrow()
                            .get(&method_key)
                            .cloned()
                            .unwrap_or(Object::Atom("null".to_string())),
                        Object::String(_) => stdlib::get_string_method(&method_key),
                        Object::Array(_) => stdlib::get_array_method(&method_key),
                        Object::Number(_) => stdlib::get_number_method(&method_key),
                        _ => {
                            return Err(format!(
                                "Method calls not supported on this type: {:?}",
                                obj
                            ))
                        }
                    };

                    if matches!(func, Object::Atom(ref s) if s == "null") {
                        return Err(format!("Method '{}' not found on object", method_key));
                    }

                    self.push(func)?;
                    self.push(obj)?;
                }

                Opcode::OpTuple => {
                    let num_elements = read_u16(&mut frame);

                    let start = self.sp - num_elements;
                    let tuple_elements = self.stack[start..self.sp].to_vec();
                    self.sp = start;

                    self.push(Object::Tuple(tuple_elements))?;
                }
                Opcode::OpTupleLen => {
                    let val = self.pop();
                    match val {
                        Object::Tuple(elements) => {
                            self.push(Object::Number(elements.len() as f64))?;
                        }
                        _ => {
                            self.push(Object::Number(-1.0))?;
                        }
                    }
                }

                Opcode::OpImport => {
                    let path_obj = self.pop();
                    if let Object::String(path) = path_obj {
                        let raw_path = self.current_dir.join(&path);

                        let resolved_path_str = std::fs::canonicalize(&raw_path)
                            .unwrap_or(raw_path)
                            .to_string_lossy()
                            .into_owned();

                        if let Some(module) = self.loaded_modules.get(&resolved_path_str) {
                            self.push(module.clone())?;
                        } else if let Some(handler) = self.import_handler {
                            let module_obj = handler(&resolved_path_str)?;
                            self.loaded_modules
                                .insert(resolved_path_str, module_obj.clone());
                            self.push(module_obj)?;
                        } else {
                            return Err("Imports are not supported in this environment".into());
                        }
                    } else {
                        return Err("Import path must be a string".into());
                    }
                }

                Opcode::OpAdd | Opcode::OpSub | Opcode::OpMul | Opcode::OpDiv => {
                    self.execute_binary_operation(op)?
                }
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

        self.last_popped_stack_elem = if self.sp > 0 { Some(self.pop()) } else { None };
        Ok(())
    }

    #[inline]
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

        if let (Object::String(l), Object::String(r)) = (&left, &right) {
            if op == Opcode::OpAdd {
                return self.push(Object::String(format!("{}{}", l, r)));
            }
        }

        Err("Unsupported types for binary operation".into())
    }

    #[inline]
    fn execute_comparison(&mut self, op: Opcode) -> Result<(), String> {
        let right = self.pop();
        let left = self.pop();

        let mut result = false;

        if let (Object::Number(l), Object::Number(r)) = (&left, &right) {
            result = match op {
                Opcode::OpEqual => l == r,
                Opcode::OpNotEqual => l != r,
                Opcode::OpGreater => l > r,
                Opcode::OpLess => l < r,
                _ => unreachable!(),
            };
        } else if let (Object::Atom(l), Object::Atom(r)) = (&left, &right) {
            result = match op {
                Opcode::OpEqual => l == r,
                Opcode::OpNotEqual => l != r,
                _ => return Err("Unsupported atom operation".into()),
            };
        } else if let (Object::String(l), Object::String(r)) = (&left, &right) {
            result = match op {
                Opcode::OpEqual => l == r,
                Opcode::OpNotEqual => l != r,
                _ => return Err("Unsupported string operation".into()),
            };
        } else if op == Opcode::OpNotEqual {
            result = true;
        }

        let obj = if result {
            Object::Atom("true".to_string())
        } else {
            Object::Atom("false".to_string())
        };
        self.push(obj)
    }

    #[inline]
    fn execute_minus_operator(&mut self) -> Result<(), String> {
        if let Object::Number(val) = self.pop() {
            self.push(Object::Number(-val))
        } else {
            Err("Negation err".into())
        }
    }

    #[inline]
    fn execute_bang_operator(&mut self) -> Result<(), String> {
        let obj = self.pop();
        let is_truthy = self.is_truthy(&obj);
        let obj = if is_truthy {
            Object::Atom("false".to_string())
        } else {
            Object::Atom("true".to_string())
        };
        self.push(obj)
    }

    #[inline]
    fn is_truthy(&self, obj: &Object) -> bool {
        match obj {
            Object::Atom(s) => s.as_str() != "false" && s.as_str() != "null",
            _ => true,
        }
    }

    #[inline]
    fn push(&mut self, obj: Object) -> Result<(), String> {
        if self.sp >= STACK_SIZE {
            return Err("Stack overflow".into());
        }
        self.stack[self.sp] = obj;
        self.sp += 1;
        Ok(())
    }

    #[inline]
    fn pop(&mut self) -> Object {
        self.sp -= 1;
        std::mem::take(&mut self.stack[self.sp])
    }

    #[inline]
    fn peek(&self, offset: usize) -> Object {
        self.stack[self.sp - 1 - offset].clone()
    }
}
