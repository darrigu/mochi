use crate::ast::{Expression, Program};
use crate::code::{Opcode, make};
use crate::object::Object;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolScope {
    Global,
    Local,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub scope: SymbolScope,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub store: HashMap<String, Symbol>,
    pub num_definitions: usize,
    pub outer: Option<Box<SymbolTable>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            num_definitions: 0,
            outer: None,
        }
    }

    pub fn new_enclosed(outer: SymbolTable) -> Self {
        Self {
            store: HashMap::new(),
            num_definitions: 0,
            outer: Some(Box::new(outer)),
        }
    }

    pub fn define(&mut self, name: String) -> Symbol {
        let symbol = Symbol {
            scope: if self.outer.is_none() {
                SymbolScope::Global
            } else {
                SymbolScope::Local
            },
            index: self.num_definitions,
        };
        self.store.insert(name, symbol.clone());
        self.num_definitions += 1;
        symbol
    }

    pub fn resolve(&self, name: &str) -> Option<Symbol> {
        if let Some(symbol) = self.store.get(name) {
            Some(symbol.clone())
        } else if let Some(outer) = &self.outer {
            outer.resolve(name)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bytecode {
    pub instructions: Vec<u8>,
    pub constants: Vec<Object>,
}

pub struct Compiler {
    pub instructions: Vec<u8>,
    pub constants: Vec<Object>,
    pub symbol_table: SymbolTable,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            instructions: vec![],
            constants: vec![],
            symbol_table: SymbolTable::new(),
        }
    }

    pub fn new_with_state(outer: SymbolTable) -> Self {
        Self {
            instructions: vec![],
            constants: vec![],
            symbol_table: SymbolTable::new_enclosed(outer),
        }
    }

    pub fn compile_program(&mut self, program: &Program) -> Result<(), String> {
        for expr in program.expressions.iter() {
            self.compile_expression(expr)?;
        }
        Ok(())
    }

    fn replace_instruction(&mut self, pos: usize, new_instruction: Vec<u8>) {
        for (i, byte) in new_instruction.iter().enumerate() {
            self.instructions[pos + i] = *byte;
        }
    }

    fn change_operand(&mut self, op_pos: usize, operand: usize) {
        let op = Opcode::from(self.instructions[op_pos]);
        let new_instruction = make(op, &[operand]);
        self.replace_instruction(op_pos, new_instruction);
    }

    fn compile_expression(&mut self, expr: &Expression) -> Result<(), String> {
        match expr {
            Expression::Identifier(name) => {
                if let Some(symbol) = self.symbol_table.resolve(name) {
                    if symbol.scope == SymbolScope::Local {
                        self.emit(Opcode::OpGetLocal, &[symbol.index]);
                    } else {
                        self.emit(Opcode::OpGetGlobal, &[symbol.index]);
                    }
                } else {
                    return Err(format!("Undefined variable: {}", name));
                }
            }
            Expression::Function { parameters, body } => {
                let mut fn_compiler = Compiler::new_with_state(self.symbol_table.clone());

                for param in parameters {
                    fn_compiler.symbol_table.define(param.clone());
                }

                fn_compiler.compile_block(body)?;
                fn_compiler.emit(Opcode::OpReturnValue, &[]);

                let num_locals = fn_compiler.symbol_table.num_definitions;

                let bytecode = fn_compiler.bytecode();
                let fn_obj = Object::CompiledFunction {
                    instructions: bytecode.instructions,
                    constants: bytecode.constants,
                    num_locals,
                    num_parameters: parameters.len(),
                };

                let pos = self.add_constant(fn_obj);
                self.emit(Opcode::OpConstant, &[pos]);
            }
            Expression::Call {
                function,
                arguments,
            } => {
                self.compile_expression(function)?;
                for arg in arguments {
                    self.compile_expression(arg)?;
                }
                self.emit(Opcode::OpCall, &[arguments.len()]);
            }
            Expression::Infix {
                left,
                operator,
                right,
            } => {
                self.compile_expression(left)?;
                self.compile_expression(right)?;

                match operator.as_str() {
                    "+" => self.emit(Opcode::OpAdd, &[]),
                    "-" => self.emit(Opcode::OpSub, &[]),
                    "*" => self.emit(Opcode::OpMul, &[]),
                    "/" => self.emit(Opcode::OpDiv, &[]),
                    "==" => self.emit(Opcode::OpEqual, &[]),
                    "!=" => self.emit(Opcode::OpNotEqual, &[]),
                    ">" => self.emit(Opcode::OpGreater, &[]),
                    "<" => self.emit(Opcode::OpLess, &[]),
                    _ => return Err(format!("Unknown operator: {}", operator)),
                };
            }
            Expression::Prefix { operator, right } => {
                self.compile_expression(right)?;
                match operator.as_str() {
                    "-" => self.emit(Opcode::OpMinus, &[]),
                    "!" => self.emit(Opcode::OpBang, &[]),
                    _ => return Err(format!("Unknown prefix operator: {}", operator)),
                };
            }
            Expression::Number(val) => {
                let obj = Object::Number(*val);
                let pos = self.add_constant(obj);
                self.emit(Opcode::OpConstant, &[pos]);
            }
            Expression::Boolean(val) => {
                if *val {
                    self.emit(Opcode::OpTrue, &[]);
                } else {
                    self.emit(Opcode::OpFalse, &[]);
                }
            }
            Expression::Block(exprs) => {
                self.compile_block(exprs)?;
            }
            Expression::If {
                condition,
                consequence,
                alternative,
            } => {
                self.compile_expression(condition)?;
                let jump_not_truthy_pos = self.emit(Opcode::OpJumpNotTruthy, &[9999]);

                self.compile_expression(consequence)?;

                if let Some(alt) = alternative {
                    let jump_pos = self.emit(Opcode::OpJump, &[9999]);

                    let alternative_pos = self.instructions.len();
                    self.change_operand(jump_not_truthy_pos, alternative_pos);

                    self.compile_expression(alt)?;

                    let end_pos = self.instructions.len();
                    self.change_operand(jump_pos, end_pos);
                } else {
                    let jump_pos = self.emit(Opcode::OpJump, &[9999]);

                    let alternative_pos = self.instructions.len();
                    self.change_operand(jump_not_truthy_pos, alternative_pos);

                    self.emit(Opcode::OpFalse, &[]);

                    let end_pos = self.instructions.len();
                    self.change_operand(jump_pos, end_pos);
                }
            }
            Expression::Let { name, value } => {
                let symbol = self.symbol_table.define(name.clone());

                if let Expression::Function { parameters, body } = value.as_ref() {
                    let mut fn_compiler = Compiler::new_with_state(self.symbol_table.clone());

                    for param in parameters {
                        fn_compiler.symbol_table.define(param.clone());
                    }

                    fn_compiler.compile_block(body)?;
                    fn_compiler.emit(Opcode::OpReturnValue, &[]);

                    let num_locals = fn_compiler.symbol_table.num_definitions;
                    let bytecode = fn_compiler.bytecode();
                    let fn_obj = Object::CompiledFunction {
                        instructions: bytecode.instructions,
                        constants: bytecode.constants,
                        num_locals,
                        num_parameters: parameters.len(),
                    };

                    let pos = self.add_constant(fn_obj);
                    self.emit(Opcode::OpConstant, &[pos]);
                } else {
                    self.compile_expression(value)?;
                }

                if symbol.scope == SymbolScope::Local {
                    self.emit(Opcode::OpSetLocal, &[symbol.index]);
                } else {
                    self.emit(Opcode::OpSetGlobal, &[symbol.index]);
                }
            }
            Expression::Return(expr) => {
                self.compile_expression(expr)?;
                self.emit(Opcode::OpReturnValue, &[]);
            }
            _ => return Err(format!("Unimplemented expression: {:?}", expr)),
        }
        Ok(())
    }

    fn compile_block(&mut self, expressions: &[Expression]) -> Result<(), String> {
        if expressions.is_empty() {
            self.emit(Opcode::OpFalse, &[]);
            return Ok(());
        }

        for expr in expressions.iter() {
            self.compile_expression(expr)?;
        }
        Ok(())
    }

    fn add_constant(&mut self, obj: Object) -> usize {
        self.constants.push(obj);
        self.constants.len() - 1
    }

    fn emit(&mut self, op: Opcode, operands: &[usize]) -> usize {
        let instr = make(op, operands);
        let pos = self.instructions.len();
        self.instructions.extend(instr);
        pos
    }

    pub fn bytecode(self) -> Bytecode {
        Bytecode {
            instructions: self.instructions,
            constants: self.constants,
        }
    }
}
