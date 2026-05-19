use crate::ast::{Expression, Program};
use crate::code::{Opcode, make};
use crate::object::Object;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolScope {
    Global,
    Local,
    Free,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub scope: SymbolScope,
    pub index: usize,
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub store: HashMap<String, Symbol>,
    pub num_definitions: usize,
    pub outer: Option<Box<SymbolTable>>,
    pub free_symbols: Vec<Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self {
            store: HashMap::new(),
            num_definitions: 0,
            outer: None,
            free_symbols: vec![],
        };
        table.define("print".to_string(), true);
        table
    }

    pub fn new_enclosed(outer: SymbolTable) -> Self {
        Self {
            store: HashMap::new(),
            num_definitions: 0,
            outer: Some(Box::new(outer)),
            free_symbols: vec![],
        }
    }

    pub fn define(&mut self, name: String, is_const: bool) -> Symbol {
        let symbol = Symbol {
            scope: if self.outer.is_none() {
                SymbolScope::Global
            } else {
                SymbolScope::Local
            },
            index: self.num_definitions,
            is_const,
        };
        self.store.insert(name, symbol.clone());
        self.num_definitions += 1;
        symbol
    }

    pub fn define_free(&mut self, original: &Symbol) -> Symbol {
        for (i, sym) in self.free_symbols.iter().enumerate() {
            if sym.index == original.index && sym.scope == original.scope {
                return Symbol {
                    scope: SymbolScope::Free,
                    index: i,
                    is_const: original.is_const,
                };
            }
        }
        let idx = self.free_symbols.len();
        self.free_symbols.push(original.clone());
        Symbol {
            scope: SymbolScope::Free,
            index: idx,
            is_const: original.is_const,
        }
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
        if program.expressions.is_empty() {
            return Ok(());
        }
        for (i, expr) in program.expressions.iter().enumerate() {
            self.compile_expression(expr)?;
            if i != program.expressions.len() - 1 {
                self.emit(Opcode::OpPop, &[]);
            }
        }
        Ok(())
    }

    fn compile_expression(&mut self, expr: &Expression) -> Result<(), String> {
        match expr {
            Expression::Identifier(name) => self.compile_identifier(name),
            Expression::StringLiteral(val) => {
                let pos = self.add_constant(Object::String(val.clone()));
                self.emit(Opcode::OpConstant, &[pos]);
                Ok(())
            }
            Expression::Assign { name, value } => self.compile_assign(name, value),
            Expression::Function { parameters, body } => self.compile_function(parameters, body),
            Expression::Call {
                function,
                arguments,
            } => self.compile_call(function, arguments),
            Expression::Infix {
                left,
                operator,
                right,
            } => self.compile_infix(left, operator, right),
            Expression::Prefix { operator, right } => self.compile_prefix(operator, right),
            Expression::Number(val) => {
                let pos = self.add_constant(Object::Number(*val));
                self.emit(Opcode::OpConstant, &[pos]);
                Ok(())
            }
            Expression::Boolean(val) => {
                self.emit(
                    if *val {
                        Opcode::OpTrue
                    } else {
                        Opcode::OpFalse
                    },
                    &[],
                );
                Ok(())
            }
            Expression::Block(exprs) => self.compile_block(exprs),
            Expression::If {
                condition,
                consequence,
                alternative,
            } => self.compile_if(condition, consequence, alternative),
            Expression::Let { name, value } => self.compile_binding(name, value, false),
            Expression::Const { name, value } => self.compile_binding(name, value, true),
            Expression::Return(expr) => {
                self.compile_expression(expr)?;
                self.emit(Opcode::OpReturnValue, &[]);
                Ok(())
            }
            Expression::Hash(pairs) => {
                for (key, val) in pairs {
                    match key {
                        Expression::Identifier(name) => {
                            let pos = self.add_constant(Object::String(name.clone()));
                            self.emit(Opcode::OpConstant, &[pos]);
                        }
                        _ => self.compile_expression(key)?,
                    }
                    self.compile_expression(val)?;
                }
                self.emit(Opcode::OpHash, &[pairs.len()]);
                Ok(())
            }
            Expression::Index { left, index } => {
                self.compile_expression(left)?;
                self.compile_expression(index)?;
                self.emit(Opcode::OpIndex, &[]);
                Ok(())
            }
            Expression::IndexAssign { left, index, value } => {
                self.compile_expression(left)?;
                self.compile_expression(index)?;
                self.compile_expression(value)?;
                self.emit(Opcode::OpSetIndex, &[]);
                Ok(())
            }
        }
    }

    fn compile_identifier(&mut self, name: &str) -> Result<(), String> {
        let symbol = self.resolve_name(name)?;
        match symbol.scope {
            SymbolScope::Local => self.emit(Opcode::OpGetLocal, &[symbol.index]),
            SymbolScope::Global => self.emit(Opcode::OpGetGlobal, &[symbol.index]),
            SymbolScope::Free => self.emit(Opcode::OpGetFree, &[symbol.index]),
        };
        Ok(())
    }

    fn compile_assign(&mut self, name: &str, value: &Expression) -> Result<(), String> {
        let symbol = self.resolve_name(name)?;
        if symbol.is_const {
            return Err(format!("Cannot reassign constant '{}'", name));
        }

        self.compile_expression(value)?;
        match symbol.scope {
            SymbolScope::Local => self.emit(Opcode::OpSetLocal, &[symbol.index]),
            SymbolScope::Global => self.emit(Opcode::OpSetGlobal, &[symbol.index]),
            SymbolScope::Free => self.emit(Opcode::OpSetFree, &[symbol.index]),
        };
        Ok(())
    }

    fn compile_function(
        &mut self,
        parameters: &[String],
        body: &[Expression],
    ) -> Result<(), String> {
        let mut fn_compiler = Compiler::new_with_state(self.symbol_table.clone());

        for param in parameters {
            fn_compiler.symbol_table.define(param.clone(), false);
        }
        fn_compiler.compile_block(body)?;
        fn_compiler.emit(Opcode::OpReturnValue, &[]);

        let num_locals = fn_compiler.symbol_table.num_definitions;
        let free_symbols = fn_compiler.symbol_table.free_symbols.clone();
        let num_free = free_symbols.len();

        let bytecode = fn_compiler.bytecode();
        let fn_obj = Object::CompiledFunction {
            instructions: bytecode.instructions,
            constants: bytecode.constants,
            num_locals,
            num_parameters: parameters.len(),
        };

        let pos = self.add_constant(fn_obj);

        for free_sym in &free_symbols {
            match free_sym.scope {
                SymbolScope::Local => self.emit(Opcode::OpGetLocal, &[free_sym.index]),
                SymbolScope::Free => self.emit(Opcode::OpGetFree, &[free_sym.index]),
                SymbolScope::Global => self.emit(Opcode::OpGetGlobal, &[free_sym.index]),
            };
        }

        self.emit(Opcode::OpClosure, &[pos, num_free]);
        Ok(())
    }

    fn compile_call(
        &mut self,
        function: &Expression,
        arguments: &[Expression],
    ) -> Result<(), String> {
        self.compile_expression(function)?;
        for arg in arguments {
            self.compile_expression(arg)?;
        }
        self.emit(Opcode::OpCall, &[arguments.len()]);
        Ok(())
    }

    fn compile_infix(
        &mut self,
        left: &Expression,
        operator: &str,
        right: &Expression,
    ) -> Result<(), String> {
        self.compile_expression(left)?;
        self.compile_expression(right)?;
        match operator {
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
        Ok(())
    }

    fn compile_prefix(&mut self, operator: &str, right: &Expression) -> Result<(), String> {
        self.compile_expression(right)?;
        match operator {
            "-" => self.emit(Opcode::OpMinus, &[]),
            "!" => self.emit(Opcode::OpBang, &[]),
            _ => return Err(format!("Unknown prefix operator: {}", operator)),
        };
        Ok(())
    }

    fn compile_if(
        &mut self,
        condition: &Expression,
        consequence: &Expression,
        alternative: &Option<Box<Expression>>,
    ) -> Result<(), String> {
        self.compile_expression(condition)?;
        let jump_not_truthy_pos = self.emit(Opcode::OpJumpNotTruthy, &[9999]);

        self.compile_expression(consequence)?;

        if let Some(alt) = alternative {
            let jump_pos = self.emit(Opcode::OpJump, &[9999]);
            self.change_operand(jump_not_truthy_pos, self.instructions.len());

            self.compile_expression(alt)?;
            self.change_operand(jump_pos, self.instructions.len());
        } else {
            let jump_pos = self.emit(Opcode::OpJump, &[9999]);
            self.change_operand(jump_not_truthy_pos, self.instructions.len());

            self.emit(Opcode::OpFalse, &[]);
            self.change_operand(jump_pos, self.instructions.len());
        }
        Ok(())
    }

    fn compile_binding(
        &mut self,
        name: &str,
        value: &Expression,
        is_const: bool,
    ) -> Result<(), String> {
        let symbol = self.symbol_table.define(name.to_string(), is_const);
        self.compile_expression(value)?;

        if symbol.scope == SymbolScope::Local {
            self.emit(Opcode::OpSetLocal, &[symbol.index]);
        } else {
            self.emit(Opcode::OpSetGlobal, &[symbol.index]);
        }
        Ok(())
    }

    fn compile_block(&mut self, expressions: &[Expression]) -> Result<(), String> {
        if expressions.is_empty() {
            self.emit(Opcode::OpFalse, &[]);
            return Ok(());
        }
        for (i, expr) in expressions.iter().enumerate() {
            self.compile_expression(expr)?;
            if i != expressions.len() - 1 {
                self.emit(Opcode::OpPop, &[]);
            }
        }
        Ok(())
    }

    fn resolve_name(&mut self, name: &str) -> Result<Symbol, String> {
        match self.symbol_table.resolve(name) {
            Some(symbol) => {
                if symbol.scope != SymbolScope::Global
                    && self.symbol_table.outer.is_some()
                    && self.symbol_table.store.get(name).is_none()
                {
                    Ok(self.symbol_table.define_free(&symbol))
                } else {
                    Ok(symbol)
                }
            }
            None => Err(format!("Undefined variable: {}", name)),
        }
    }

    fn replace_instruction(&mut self, pos: usize, new_instruction: Vec<u8>) {
        for (i, byte) in new_instruction.iter().enumerate() {
            self.instructions[pos + i] = *byte;
        }
    }

    fn change_operand(&mut self, op_pos: usize, operand: usize) {
        let op = Opcode::from(self.instructions[op_pos]);
        self.replace_instruction(op_pos, make(op, &[operand]));
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
