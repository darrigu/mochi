use crate::ast::{Expression, Program, Statement};
use crate::code::{Opcode, make};
use crate::object::Object;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SymbolTable {
    store: HashMap<String, usize>,
    num_definitions: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            num_definitions: 0,
        }
    }

    pub fn define(&mut self, name: String) -> usize {
        let id = self.num_definitions;
        self.store.insert(name, id);
        self.num_definitions += 1;
        id
    }

    pub fn resolve(&self, name: &str) -> Option<usize> {
        self.store.get(name).copied()
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

    pub fn compile_program(&mut self, program: &Program) -> Result<(), String> {
        for stmt in &program.statements {
            self.compile_statement(stmt)?;
        }
        Ok(())
    }

    fn compile_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                self.emit(Opcode::OpPop, &[]);
            }
            Statement::Let { name, value } => {
                self.compile_expression(value)?;
                let index = self.symbol_table.define(name.clone());
                self.emit(Opcode::OpSetGlobal, &[index]);
            }
            _ => return Err(format!("Unimplemented statement: {:?}", stmt)),
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
                if let Some(index) = self.symbol_table.resolve(name) {
                    self.emit(Opcode::OpGetGlobal, &[index]);
                } else {
                    return Err(format!("Undefined variable: {}", name));
                }
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
            Expression::If {
                condition,
                consequence,
                alternative,
            } => {
                self.compile_expression(condition)?;
                let jump_not_truthy_pos = self.emit(Opcode::OpJumpNotTruthy, &[9999]);

                self.compile_block(consequence)?;

                if let Some(alt) = alternative {
                    let jump_pos = self.emit(Opcode::OpJump, &[9999]);

                    let alternative_pos = self.instructions.len();
                    self.change_operand(jump_not_truthy_pos, alternative_pos);

                    self.compile_block(alt)?;

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
            _ => return Err(format!("Unimplemented expression: {:?}", expr)),
        }
        Ok(())
    }

    fn compile_block(&mut self, statements: &[Statement]) -> Result<(), String> {
        if statements.is_empty() {
            self.emit(Opcode::OpFalse, &[]);
            return Ok(());
        }

        for (i, stmt) in statements.iter().enumerate() {
            let is_last = i == statements.len() - 1;

            match stmt {
                Statement::Expression(expr) => {
                    self.compile_expression(expr)?;
                    if !is_last {
                        self.emit(Opcode::OpPop, &[]);
                    }
                }
                Statement::Let { name, value } => {
                    self.compile_expression(value)?;
                    let index = self.symbol_table.define(name.clone());
                    self.emit(Opcode::OpSetGlobal, &[index]);

                    if is_last {
                        self.emit(Opcode::OpFalse, &[]);
                    }
                }
                Statement::Return(_) => {
                    return Err("Return not implemented in compiler yet!".to_string());
                }
            }
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
