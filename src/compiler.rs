use crate::ast::{Expression, Pattern, Program, TypeAnn};
use crate::code::{make, Opcode};
use crate::error_reporter::Diagnostic;
use crate::object::Object;
use std::collections::HashMap;

macro_rules! wrap_err {
    ($self:expr, $res:expr) => {{
        let line = $self.current_line;
        let col = $self.current_col;
        $res.map_err(|e| crate::error_reporter::Diagnostic {
            line,
            col,
            message: e,
            hint: None,
        })
    }};
}

#[derive(Debug, Clone)]
pub enum BreakTarget {
    Loop {
        continue_jumps: Vec<usize>,
        break_jumps: Vec<usize>,
    },
    Block {
        break_jumps: Vec<usize>,
    },
}

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
    pub current_line: usize,
    pub current_col: usize,
    pub break_stack: Vec<BreakTarget>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            instructions: vec![],
            constants: vec![],
            symbol_table: SymbolTable::new(),
            current_line: 1,
            current_col: 1,
            break_stack: vec![],
        }
    }

    pub fn new_with_state(outer: SymbolTable) -> Self {
        Self {
            instructions: vec![],
            constants: vec![],
            symbol_table: SymbolTable::new_enclosed(outer),
            current_line: 1,
            current_col: 1,
            break_stack: vec![],
        }
    }

    fn err<T>(&self, msg: String) -> Result<T, Diagnostic> {
        Err(Diagnostic {
            line: self.current_line,
            col: self.current_col,
            message: msg,
            hint: None,
        })
    }

    pub fn compile_program(&mut self, program: &Program) -> Result<(), Diagnostic> {
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

    fn emit_get(&mut self, symbol: &Symbol) {
        match symbol.scope {
            SymbolScope::Local => {
                self.emit(Opcode::OpGetLocal, &[symbol.index]);
            }
            SymbolScope::Global => {
                self.emit(Opcode::OpGetGlobal, &[symbol.index]);
            }
            SymbolScope::Free => {
                self.emit(Opcode::OpGetFree, &[symbol.index]);
            }
        }
    }

    fn emit_set(&mut self, symbol: &Symbol) {
        match symbol.scope {
            SymbolScope::Local => {
                self.emit(Opcode::OpSetLocal, &[symbol.index]);
            }
            SymbolScope::Global => {
                self.emit(Opcode::OpSetGlobal, &[symbol.index]);
            }
            SymbolScope::Free => {
                self.emit(Opcode::OpSetFree, &[symbol.index]);
            }
        }
    }

    fn resolve_jumps(&mut self, target: BreakTarget, continue_dest: usize, end_pos: usize) {
        match target {
            BreakTarget::Loop {
                continue_jumps,
                break_jumps,
            } => {
                for jump_pos in continue_jumps {
                    self.change_operand(jump_pos, continue_dest);
                }
                for jump_pos in break_jumps {
                    self.change_operand(jump_pos, end_pos);
                }
            }
            BreakTarget::Block { break_jumps } => {
                for jump_pos in break_jumps {
                    self.change_operand(jump_pos, end_pos);
                }
            }
        }
    }

    fn compile_expression(&mut self, expr: &Expression) -> Result<(), Diagnostic> {
        if let Expression::Loc {
            line,
            col,
            expr: inner,
        } = expr
        {
            self.current_line = *line;
            self.current_col = *col;
            return self.compile_expression(inner);
        }

        match expr {
            Expression::Identifier(name) => self.compile_identifier(name),
            Expression::StringLiteral(val) => {
                self.emit_string(val);
                Ok(())
            }
            Expression::Atom(name) => {
                self.emit_atom(name);
                Ok(())
            }
            Expression::Assign { name, value } => self.compile_assign(name, value),
            Expression::Function {
                parameters,
                return_type: _,
                body,
            } => self.compile_function(parameters, body),
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
            Expression::Block(exprs, is_breakable) => self.compile_block(exprs, *is_breakable),
            Expression::If {
                condition,
                consequence,
                alternative,
            } => self.compile_if(condition, consequence, alternative),
            Expression::Let {
                name,
                type_ann: _,
                value,
            } => self.compile_binding(name, value, false),
            Expression::Const {
                name,
                type_ann: _,
                value,
            } => self.compile_binding(name, value, true),
            Expression::TypeAlias { .. } => {
                self.emit_atom("null");
                Ok(())
            }
            Expression::Return(expr) => {
                self.compile_expression(expr)?;
                self.emit(Opcode::OpReturnValue, &[]);
                Ok(())
            }
            Expression::Array(elements) => {
                for el in elements.iter() {
                    self.compile_expression(el)?;
                }
                self.emit(Opcode::OpArray, &[elements.len()]);
                Ok(())
            }
            Expression::Hash(pairs) => {
                for (key, val) in pairs {
                    self.emit_atom(key);
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
            Expression::MethodCall {
                left,
                method,
                arguments,
            } => {
                self.compile_expression(left)?;

                let atom = Object::Atom(method.clone());
                let pos = match self.constants.iter().position(|c| *c == atom) {
                    Some(idx) => idx,
                    None => self.add_constant(atom),
                };
                self.emit(Opcode::OpGetMethod, &[pos]);

                for arg in arguments {
                    self.compile_expression(arg)?;
                }

                self.emit(Opcode::OpCall, &[arguments.len() + 1]);
                Ok(())
            }
            Expression::Loop { body } => {
                let start_pos = self.instructions.len();
                self.break_stack.push(BreakTarget::Loop {
                    continue_jumps: vec![],
                    break_jumps: vec![],
                });

                self.compile_expression(body)?;
                self.emit(Opcode::OpPop, &[]);

                let continue_dest = self.instructions.len();
                self.emit(Opcode::OpJump, &[start_pos]);

                let target = self.break_stack.pop().unwrap();
                let end_pos = self.instructions.len();
                self.resolve_jumps(target, continue_dest, end_pos);

                self.emit_atom("null");
                Ok(())
            }
            Expression::While { condition, body } => {
                let start_pos = self.instructions.len();
                self.break_stack.push(BreakTarget::Loop {
                    continue_jumps: vec![],
                    break_jumps: vec![],
                });

                self.compile_expression(condition)?;
                let jump_out_pos = self.emit(Opcode::OpJumpNotTruthy, &[9999]);

                self.compile_expression(body)?;
                self.emit(Opcode::OpPop, &[]);

                let continue_dest = self.instructions.len();
                self.emit(Opcode::OpJump, &[start_pos]);

                let normal_exit = self.instructions.len();
                self.change_operand(jump_out_pos, normal_exit);
                self.emit_atom("null");

                let end_pos = self.instructions.len();
                let target = self.break_stack.pop().unwrap();
                self.resolve_jumps(target, continue_dest, end_pos);
                Ok(())
            }
            Expression::For {
                element,
                iterable,
                body,
            } => {
                let start_pos_id = self.instructions.len();
                let arr_sym = self
                    .symbol_table
                    .define(format!("_arr_{}", start_pos_id), false);
                let idx_sym = self
                    .symbol_table
                    .define(format!("_idx_{}", start_pos_id), false);
                let len_sym = self
                    .symbol_table
                    .define(format!("_len_{}", start_pos_id), false);

                self.compile_expression(iterable)?;
                self.emit_set(&arr_sym);
                self.emit(Opcode::OpPop, &[]);

                let zero_pos = self.add_constant(Object::Number(0.0));
                self.emit(Opcode::OpConstant, &[zero_pos]);
                self.emit_set(&idx_sym);
                self.emit(Opcode::OpPop, &[]);

                self.emit_get(&arr_sym);
                self.emit(Opcode::OpArrayLen, &[]);
                self.emit_set(&len_sym);
                self.emit(Opcode::OpPop, &[]);

                let loop_start = self.instructions.len();
                self.break_stack.push(BreakTarget::Loop {
                    continue_jumps: vec![],
                    break_jumps: vec![],
                });

                self.emit_get(&idx_sym);
                self.emit_get(&len_sym);
                self.emit(Opcode::OpLess, &[]);
                let jump_out_pos = self.emit(Opcode::OpJumpNotTruthy, &[9999]);

                let el_sym = self.symbol_table.define(element.clone(), false);
                self.emit_get(&arr_sym);
                self.emit_get(&idx_sym);
                self.emit(Opcode::OpIndex, &[]);
                self.emit_set(&el_sym);
                self.emit(Opcode::OpPop, &[]);

                self.compile_expression(body)?;
                self.emit(Opcode::OpPop, &[]);

                let continue_dest = self.instructions.len();
                self.emit_get(&idx_sym);
                let one_pos = self.add_constant(Object::Number(1.0));
                self.emit(Opcode::OpConstant, &[one_pos]);
                self.emit(Opcode::OpAdd, &[]);
                self.emit_set(&idx_sym);
                self.emit(Opcode::OpPop, &[]);

                self.emit(Opcode::OpJump, &[loop_start]);

                let normal_exit = self.instructions.len();
                self.change_operand(jump_out_pos, normal_exit);
                self.emit_atom("null");

                let end_pos = self.instructions.len();
                let target = self.break_stack.pop().unwrap();
                self.resolve_jumps(target, continue_dest, end_pos);
                Ok(())
            }
            Expression::ForHash {
                key,
                value,
                iterable,
                body,
            } => {
                let start_pos_id = self.instructions.len();
                let hash_sym = self
                    .symbol_table
                    .define(format!("_hash_{}", start_pos_id), false);
                let keys_sym = self
                    .symbol_table
                    .define(format!("_keys_{}", start_pos_id), false);
                let idx_sym = self
                    .symbol_table
                    .define(format!("_idx_{}", start_pos_id), false);
                let len_sym = self
                    .symbol_table
                    .define(format!("_len_{}", start_pos_id), false);

                self.compile_expression(iterable)?;
                self.emit_set(&hash_sym);
                self.emit(Opcode::OpPop, &[]);

                self.emit_get(&hash_sym);
                self.emit(Opcode::OpHashKeys, &[]);
                self.emit_set(&keys_sym);
                self.emit(Opcode::OpPop, &[]);

                let zero_pos = self.add_constant(Object::Number(0.0));
                self.emit(Opcode::OpConstant, &[zero_pos]);
                self.emit_set(&idx_sym);
                self.emit(Opcode::OpPop, &[]);

                self.emit_get(&keys_sym);
                self.emit(Opcode::OpArrayLen, &[]);
                self.emit_set(&len_sym);
                self.emit(Opcode::OpPop, &[]);

                let loop_start = self.instructions.len();
                self.break_stack.push(BreakTarget::Loop {
                    continue_jumps: vec![],
                    break_jumps: vec![],
                });

                self.emit_get(&idx_sym);
                self.emit_get(&len_sym);
                self.emit(Opcode::OpLess, &[]);
                let jump_out_pos = self.emit(Opcode::OpJumpNotTruthy, &[9999]);

                let key_sym = self.symbol_table.define(key.clone(), false);
                self.emit_get(&keys_sym);
                self.emit_get(&idx_sym);
                self.emit(Opcode::OpIndex, &[]);
                self.emit_set(&key_sym);
                self.emit(Opcode::OpPop, &[]);

                let val_sym = self.symbol_table.define(value.clone(), false);
                self.emit_get(&hash_sym);
                self.emit_get(&key_sym);
                self.emit(Opcode::OpIndex, &[]);
                self.emit_set(&val_sym);
                self.emit(Opcode::OpPop, &[]);

                self.compile_expression(body)?;
                self.emit(Opcode::OpPop, &[]);

                let continue_dest = self.instructions.len();
                self.emit_get(&idx_sym);
                let one_pos = self.add_constant(Object::Number(1.0));
                self.emit(Opcode::OpConstant, &[one_pos]);
                self.emit(Opcode::OpAdd, &[]);
                self.emit_set(&idx_sym);
                self.emit(Opcode::OpPop, &[]);

                self.emit(Opcode::OpJump, &[loop_start]);

                let normal_exit = self.instructions.len();
                self.change_operand(jump_out_pos, normal_exit);
                self.emit_atom("null");

                let end_pos = self.instructions.len();
                let target = self.break_stack.pop().unwrap();
                self.resolve_jumps(target, continue_dest, end_pos);
                Ok(())
            }
            Expression::Break(val_opt) => {
                if self.break_stack.is_empty() {
                    return self.err("break statement outside loop or block context".to_string());
                }

                if let Some(val) = val_opt {
                    self.compile_expression(val)?;
                } else {
                    self.emit_atom("null");
                }

                let offset = self.emit(Opcode::OpJump, &[9999]);
                let target = self.break_stack.last_mut().unwrap();

                match target {
                    BreakTarget::Loop { break_jumps, .. } | BreakTarget::Block { break_jumps } => {
                        break_jumps.push(offset);
                    }
                }
                Ok(())
            }
            Expression::Continue => {
                let mut loop_idx = None;
                for (i, target) in self.break_stack.iter().enumerate().rev() {
                    if let BreakTarget::Loop { .. } = target {
                        loop_idx = Some(i);
                        break;
                    }
                }

                if let Some(idx) = loop_idx {
                    let offset = self.emit(Opcode::OpJump, &[9999]);

                    if let BreakTarget::Loop { continue_jumps, .. } = &mut self.break_stack[idx] {
                        continue_jumps.push(offset);
                    }
                } else {
                    return self.err("continue statement outside loop context".to_string());
                }
                self.emit_atom("null");
                Ok(())
            }
            Expression::Tuple(elements) => {
                for el in elements {
                    self.compile_expression(el)?;
                }
                self.emit(Opcode::OpTuple, &[elements.len()]);
                Ok(())
            }
            Expression::Match { subject, cases } => {
                let start_pos_id = self.instructions.len();
                let subject_sym = self
                    .symbol_table
                    .define(format!("_subject_{}", start_pos_id), false);

                self.compile_expression(subject)?;
                self.emit_set(&subject_sym);
                self.emit(Opcode::OpPop, &[]);

                let mut jump_end_offsets = vec![];

                for case in cases {
                    let mut jump_next_offsets = vec![];

                    self.compile_pattern_elements(
                        &case.pattern,
                        &subject_sym,
                        &[],
                        &mut jump_next_offsets,
                    )?;

                    if let Some(guard_expr) = &case.guard {
                        self.compile_expression(guard_expr)?;
                        let offset = self.emit(Opcode::OpJumpNotTruthy, &[9999]);
                        jump_next_offsets.push(offset);
                    }

                    self.compile_expression(&case.body)?;

                    let end_offset = self.emit(Opcode::OpJump, &[9999]);
                    jump_end_offsets.push(end_offset);

                    let next_case_start = self.instructions.len();
                    for offset in jump_next_offsets {
                        self.change_operand(offset, next_case_start);
                    }
                }

                self.emit_atom("null");

                let end_of_match = self.instructions.len();
                for offset in jump_end_offsets {
                    self.change_operand(offset, end_of_match);
                }

                Ok(())
            }
            Expression::Question(inner) => {
                let start_pos_id = self.instructions.len();
                let res_sym = self
                    .symbol_table
                    .define(format!("_res_{}", start_pos_id), false);

                self.compile_expression(inner)?;
                self.emit_set(&res_sym);
                self.emit(Opcode::OpPop, &[]);

                self.emit_get(&res_sym);
                let zero_pos = self.add_constant(Object::Number(0.0));
                self.emit(Opcode::OpConstant, &[zero_pos]);
                self.emit(Opcode::OpIndex, &[]);

                let ok_pos = self.add_constant(Object::Atom("ok".to_string()));
                self.emit(Opcode::OpConstant, &[ok_pos]);
                self.emit(Opcode::OpEqual, &[]);

                let jump_fail_pos = self.emit(Opcode::OpJumpNotTruthy, &[9999]);

                self.emit_get(&res_sym);
                let one_pos = self.add_constant(Object::Number(1.0));
                self.emit(Opcode::OpConstant, &[one_pos]);
                self.emit(Opcode::OpIndex, &[]);
                let jump_end_pos = self.emit(Opcode::OpJump, &[9999]);

                let fail_start = self.instructions.len();
                self.change_operand(jump_fail_pos, fail_start);
                self.emit_get(&res_sym);
                self.emit(Opcode::OpReturnValue, &[]);

                let end_start = self.instructions.len();
                self.change_operand(jump_end_pos, end_start);

                Ok(())
            }
            Expression::Import(path) => {
                self.compile_expression(path)?;
                self.emit(Opcode::OpImport, &[]);
                Ok(())
            }
            Expression::Loc { .. } => unreachable!(),
        }
    }

    fn compile_identifier(&mut self, name: &str) -> Result<(), Diagnostic> {
        let symbol = wrap_err!(self, self.resolve_name(name))?;
        self.emit_get(&symbol);
        Ok(())
    }

    fn compile_assign(&mut self, name: &str, value: &Expression) -> Result<(), Diagnostic> {
        let symbol = wrap_err!(self, self.resolve_name(name))?;
        if symbol.is_const {
            return self.err(format!("Cannot reassign constant '{}'", name));
        }

        self.compile_expression(value)?;
        self.emit_set(&symbol);
        Ok(())
    }

    fn compile_function(
        &mut self,
        parameters: &[(String, Option<TypeAnn>)],
        body: &[Expression],
    ) -> Result<(), Diagnostic> {
        let mut fn_compiler = Compiler::new_with_state(self.symbol_table.clone());

        fn_compiler.constants = std::mem::take(&mut self.constants);

        for (param, _type_ann) in parameters {
            fn_compiler.symbol_table.define(param.clone(), false);
        }
        fn_compiler.compile_block(body, false)?;
        fn_compiler.emit(Opcode::OpReturnValue, &[]);

        let num_locals = fn_compiler.symbol_table.num_definitions;
        let free_symbols = fn_compiler.symbol_table.free_symbols.clone();
        let num_free = free_symbols.len();

        self.constants = fn_compiler.constants;

        let fn_obj = Object::CompiledFunction {
            instructions: fn_compiler.instructions,
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
    ) -> Result<(), Diagnostic> {
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
    ) -> Result<(), Diagnostic> {
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
            _ => return self.err(format!("Unknown operator: {}", operator)),
        };
        Ok(())
    }

    fn compile_prefix(&mut self, operator: &str, right: &Expression) -> Result<(), Diagnostic> {
        self.compile_expression(right)?;
        match operator {
            "-" => self.emit(Opcode::OpMinus, &[]),
            "!" => self.emit(Opcode::OpBang, &[]),
            _ => return self.err(format!("Unknown prefix operator: {}", operator)),
        };
        Ok(())
    }

    fn compile_if(
        &mut self,
        condition: &Expression,
        consequence: &Expression,
        alternative: &Option<Box<Expression>>,
    ) -> Result<(), Diagnostic> {
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

            self.emit_atom("null");
            self.change_operand(jump_pos, self.instructions.len());
        }
        Ok(())
    }

    fn compile_binding(
        &mut self,
        name: &str,
        value: &Expression,
        is_const: bool,
    ) -> Result<(), Diagnostic> {
        let symbol = self.symbol_table.define(name.to_string(), is_const);
        self.compile_expression(value)?;

        if symbol.scope == SymbolScope::Local {
            self.emit(Opcode::OpSetLocal, &[symbol.index]);
        } else {
            self.emit(Opcode::OpSetGlobal, &[symbol.index]);
        }
        Ok(())
    }

    fn compile_block(
        &mut self,
        expressions: &[Expression],
        is_breakable: bool,
    ) -> Result<(), Diagnostic> {
        if expressions.is_empty() {
            self.emit_atom("null");
            return Ok(());
        }
        if is_breakable {
            self.break_stack.push(BreakTarget::Block {
                break_jumps: vec![],
            });
        }

        for (i, expr) in expressions.iter().enumerate() {
            self.compile_expression(expr)?;
            if i != expressions.len() - 1 {
                self.emit(Opcode::OpPop, &[]);
            }
        }

        if is_breakable {
            let target = self.break_stack.pop().unwrap();
            let end_pos = self.instructions.len();
            self.resolve_jumps(target, 0, end_pos);
        }
        Ok(())
    }

    fn emit_access_path(&mut self, subject_sym: &Symbol, path: &[usize]) {
        self.emit_get(subject_sym);
        for idx in path {
            let idx_pos = self.add_constant(Object::Number(*idx as f64));
            self.emit(Opcode::OpConstant, &[idx_pos]);
            self.emit(Opcode::OpIndex, &[]);
        }
    }

    fn compile_pattern_elements(
        &mut self,
        pattern: &Pattern,
        subject_sym: &Symbol,
        path: &[usize],
        jump_next_offsets: &mut Vec<usize>,
    ) -> Result<(), Diagnostic> {
        match pattern {
            Pattern::Wildcard => {}
            Pattern::Identifier(name) => {
                let var_sym = self.symbol_table.define(name.clone(), false);
                self.emit_access_path(subject_sym, path);
                self.emit_set(&var_sym);
                self.emit(Opcode::OpPop, &[]);
            }
            Pattern::Number(val) => {
                self.emit_access_path(subject_sym, path);
                let const_pos = self.add_constant(Object::Number(*val));
                self.emit(Opcode::OpConstant, &[const_pos]);
                self.emit(Opcode::OpEqual, &[]);
                let offset = self.emit(Opcode::OpJumpNotTruthy, &[9999]);
                jump_next_offsets.push(offset);
            }
            Pattern::StringLiteral(val) => {
                self.emit_access_path(subject_sym, path);
                let const_pos = self.add_constant(Object::String(val.clone()));
                self.emit(Opcode::OpConstant, &[const_pos]);
                self.emit(Opcode::OpEqual, &[]);
                let offset = self.emit(Opcode::OpJumpNotTruthy, &[9999]);
                jump_next_offsets.push(offset);
            }
            Pattern::Atom(name) => {
                self.emit_access_path(subject_sym, path);
                let const_pos = self.add_constant(Object::Atom(name.clone()));
                self.emit(Opcode::OpConstant, &[const_pos]);
                self.emit(Opcode::OpEqual, &[]);
                let offset = self.emit(Opcode::OpJumpNotTruthy, &[9999]);
                jump_next_offsets.push(offset);
            }
            Pattern::Tuple(elements) => {
                self.emit_access_path(subject_sym, path);
                self.emit(Opcode::OpTupleLen, &[]);
                let size_pos = self.add_constant(Object::Number(elements.len() as f64));
                self.emit(Opcode::OpConstant, &[size_pos]);
                self.emit(Opcode::OpEqual, &[]);
                let offset = self.emit(Opcode::OpJumpNotTruthy, &[9999]);
                jump_next_offsets.push(offset);

                for (i, el) in elements.iter().enumerate() {
                    let mut new_path = path.to_vec();
                    new_path.push(i);
                    self.compile_pattern_elements(el, subject_sym, &new_path, jump_next_offsets)?;
                }
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

    fn emit_string(&mut self, value: &str) {
        let obj = Object::String(value.to_string());
        let pos = match self.constants.iter().position(|c| *c == obj) {
            Some(idx) => idx,
            None => self.add_constant(obj),
        };
        self.emit(Opcode::OpConstant, &[pos]);
    }

    fn emit_atom(&mut self, name: &str) {
        let atom = Object::Atom(name.to_string());
        let pos = match self.constants.iter().position(|c| *c == atom) {
            Some(idx) => idx,
            None => self.add_constant(atom),
        };
        self.emit(Opcode::OpConstant, &[pos]);
    }

    pub fn bytecode(self) -> Bytecode {
        Bytecode {
            instructions: self.instructions,
            constants: self.constants,
        }
    }
}
