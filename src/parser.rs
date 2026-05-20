use crate::ast::{Expression, Program, TypeAnn};
use crate::error_reporter::Diagnostic;
use crate::lexer::{Lexer, Token};

#[derive(PartialEq, PartialOrd, Debug, Clone, Copy)]
pub enum Precedence {
    Lowest,
    Assign,
    Equals,
    LessGreater,
    Sum,
    Product,
    Prefix,
    Call,
}

pub struct Parser {
    lexer: Lexer,
    current_token: Token,
    peek_token: Token,
    cur_line: usize,
    cur_col: usize,
    peek_line: usize,
    peek_col: usize,
    pub errors: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Self {
        let (current_token, cur_line, cur_col) = lexer.next_token();
        let (peek_token, peek_line, peek_col) = lexer.next_token();
        Parser {
            lexer,
            current_token,
            peek_token,
            cur_line,
            cur_col,
            peek_line,
            peek_col,
            errors: vec![],
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.cur_line = self.peek_line;
        self.cur_col = self.peek_col;

        let (next_tok, next_line, next_col) = self.lexer.next_token();
        self.peek_token = next_tok;
        self.peek_line = next_line;
        self.peek_col = next_col;
    }

    fn report_error(&mut self) {
        let msg = match self.current_token {
            Token::Illegal(c) => format!("Illegal character '{}'", c),
            Token::EOF => "Unexpected end of file".to_string(),
            _ => format!("Unexpected token {:?}", self.current_token),
        };
        self.errors.push(Diagnostic {
            line: self.cur_line,
            col: self.cur_col,
            message: msg,
            hint: Some("Check for missing operators or mismatched parentheses".to_string()),
        });
    }

    fn report_error_with_msg(&mut self, msg: String) {
        self.errors.push(Diagnostic {
            line: self.cur_line,
            col: self.cur_col,
            message: msg,
            hint: None,
        });
    }

    fn report_error_with_hint(&mut self, msg: String, hint: String) {
        self.errors.push(Diagnostic {
            line: self.cur_line,
            col: self.cur_col,
            message: msg,
            hint: Some(hint),
        });
    }

    fn wrap(&self, expr: Option<Expression>, line: usize, col: usize) -> Option<Expression> {
        expr.map(|e| Expression::Loc {
            line,
            col,
            expr: Box::new(e),
        })
    }

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program {
            expressions: vec![],
        };
        while self.current_token != Token::EOF {
            if let Some(expr) = self.parse_expression(Precedence::Lowest) {
                program.expressions.push(expr);
            }
            self.next_token();
        }
        program
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        let mut left_expr = match &self.current_token {
            Token::Ident(name) => Some(Expression::Identifier(name.clone())),
            Token::Number(val) => Some(Expression::Number(*val)),
            Token::StringLiteral(val) => Some(Expression::StringLiteral(val.clone())),
            Token::Colon => self.parse_atom_literal(),
            Token::Bang | Token::Minus => self.parse_prefix_expression(),
            Token::LParen => self.parse_grouped_expression(),
            Token::If => self.parse_if_expression(),
            Token::Fn => self.parse_function_expression(),
            Token::Do => self.parse_block_expression(),
            Token::LBracket => self.parse_array_literal(),
            Token::LBrace => self.parse_hash_literal(),
            Token::Let => self.parse_let_expression(),
            Token::Const => self.parse_const_expression(),
            Token::Return => self.parse_return_expression(),
            Token::Loop => self.parse_loop_expression(),
            Token::While => self.parse_while_expression(),
            Token::For => self.parse_for_expression(),
            _ => {
                self.report_error();
                None
            }
        }?;

        if let Some(Token::Ident(_) | Token::Number(_) | Token::StringLiteral(_)) =
            Some(&self.current_token)
        {
            left_expr = self.wrap(Some(left_expr), line, col)?;
        }

        while self.peek_token != Token::EOF && precedence < self.peek_precedence() {
            if !self.peek_is_infix_operator() {
                return Some(left_expr);
            }
            self.next_token();
            left_expr = self.parse_infix_expression(left_expr)?;
        }

        Some(left_expr)
    }

    fn parse_atom_literal(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();

        let expr = if let Token::Ident(name) = &self.current_token {
            Some(Expression::Atom(name.clone()))
        } else {
            self.report_error_with_hint(
                format!(
                    "Expected identifier after ':', got {:?}",
                    self.current_token
                ),
                "Atoms must be valid names".to_string(),
            );
            None
        };
        self.wrap(expr, line, col)
    }

    fn parse_array_literal(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        let mut elements = vec![];

        if self.peek_token == Token::RBracket {
            self.next_token();
            return self.wrap(Some(Expression::Array(elements)), line, col);
        }

        self.next_token();
        elements.push(self.parse_expression(Precedence::Lowest)?);

        while self.peek_token == Token::Comma {
            self.next_token();
            self.next_token();
            elements.push(self.parse_expression(Precedence::Lowest)?);
        }

        if !self.expect_peek(Token::RBracket) {
            return None;
        }
        self.wrap(Some(Expression::Array(elements)), line, col)
    }

    fn parse_hash_literal(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();
        let mut pairs = vec![];

        if self.current_token == Token::RBrace {
            return self.wrap(Some(Expression::Hash(pairs)), line, col);
        }

        loop {
            let key = match &self.current_token {
                Token::Ident(name) => name.clone(),
                _ => {
                    self.report_error_with_hint(
                        format!(
                            "Expected identifier for object key, got {:?}",
                            self.current_token
                        ),
                        "Object keys can only be unquoted identifiers, e.g., '{ x: 10 }'"
                            .to_string(),
                    );
                    return None;
                }
            };

            if !self.expect_peek(Token::Colon) {
                self.report_error_with_hint(
                    "Expected ':'".to_string(),
                    "Use 'key: value' inside objects".to_string(),
                );
                return None;
            }
            self.next_token();

            let value = self.parse_expression(Precedence::Lowest)?;
            pairs.push((key, value));

            if self.peek_token == Token::Comma {
                self.next_token();
                self.next_token();
                if self.current_token == Token::RBrace {
                    break;
                }
            } else if self.peek_token == Token::RBrace {
                self.next_token();
                break;
            } else {
                self.report_error_with_hint(
                    format!("Expected ',' or '}}', got {:?}", self.peek_token),
                    "Separate object properties with commas".to_string(),
                );
                return None;
            }
        }
        self.wrap(Some(Expression::Hash(pairs)), line, col)
    }

    fn parse_index_expression(&mut self, left: Expression) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();
        let index = self.parse_expression(Precedence::Lowest)?;
        if !self.expect_peek(Token::RBracket) {
            return None;
        }
        let expr = Some(Expression::Index {
            left: Box::new(left),
            index: Box::new(index),
        });
        self.wrap(expr, line, col)
    }

    fn parse_dot_expression(&mut self, left: Expression) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();
        let name = match &self.current_token {
            Token::Ident(name) => name.clone(),
            _ => {
                self.report_error_with_hint(
                    "Expected property name after '.'".to_string(),
                    "Use identifiers like 'obj.name'".to_string(),
                );
                return None;
            }
        };
        let expr = Some(Expression::Index {
            left: Box::new(left),
            index: Box::new(self.wrap(Some(Expression::Atom(name)), line, col)?),
        });
        self.wrap(expr, line, col)
    }

    fn parse_type_annotation(&mut self) -> Option<TypeAnn> {
        self.next_token();
        self.parse_type_annotation_inner()
    }

    fn parse_type_annotation_inner(&mut self) -> Option<TypeAnn> {
        match &self.current_token {
            Token::Ident(name) => match name.as_str() {
                "Number" => Some(TypeAnn::Number),
                "String" => Some(TypeAnn::String),
                "Atom" => Some(TypeAnn::Atom),
                "Any" => Some(TypeAnn::Any),
                _ => {
                    self.report_error_with_msg(format!("Unknown type annotation: '{}'", name));
                    None
                }
            },
            Token::LBracket => {
                let inner = self.parse_type_annotation()?;
                if !self.expect_peek(Token::RBracket) {
                    return None;
                }
                Some(TypeAnn::Array(Box::new(inner)))
            }
            Token::LBrace => {
                self.next_token();
                let mut fields = vec![];

                if self.current_token == Token::RBrace {
                    return Some(TypeAnn::Hash(fields));
                }

                loop {
                    let key = match &self.current_token {
                        Token::Ident(name) => name.clone(),
                        _ => {
                            self.report_error_with_msg(
                                "Expected identifier for type field".to_string(),
                            );
                            return None;
                        }
                    };

                    if !self.expect_peek(Token::Colon) {
                        return None;
                    }

                    let val_type = self.parse_type_annotation()?;
                    fields.push((key, val_type));

                    if self.peek_token == Token::Comma {
                        self.next_token();
                        self.next_token();
                        if self.current_token == Token::RBrace {
                            break;
                        }
                    } else if self.peek_token == Token::RBrace {
                        self.next_token();
                        break;
                    } else {
                        self.report_error_with_msg("Expected ',' or '}' in hash type".to_string());
                        return None;
                    }
                }
                Some(TypeAnn::Hash(fields))
            }
            Token::Fn => {
                if !self.expect_peek(Token::LParen) {
                    return None;
                }

                let mut params = vec![];
                if self.peek_token != Token::RParen {
                    loop {
                        self.next_token();

                        let is_named = match &self.current_token {
                            Token::Ident(_) => self.peek_token == Token::Colon,
                            _ => false,
                        };

                        if is_named {
                            self.next_token();
                            self.next_token();
                        }

                        let param_type = self.parse_type_annotation_inner()?;
                        params.push(param_type);

                        if self.peek_token == Token::Comma {
                            self.next_token();
                        } else {
                            break;
                        }
                    }
                }

                if !self.expect_peek(Token::RParen) {
                    return None;
                }

                if !self.expect_peek(Token::Colon) {
                    return None;
                }

                let ret = self.parse_type_annotation()?;
                Some(TypeAnn::Function {
                    params,
                    ret: Box::new(ret),
                })
            }
            _ => {
                self.report_error_with_msg(format!(
                    "Expected type annotation, got {:?}",
                    self.current_token
                ));
                None
            }
        }
    }

    fn parse_let_expression(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();
        let name = match &self.current_token {
            Token::Ident(name) => name.clone(),
            _ => {
                self.report_error_with_msg(format!(
                    "Expected variable name after 'let', got {:?}",
                    self.current_token
                ));
                return None;
            }
        };

        let mut type_ann = None;
        if self.peek_token == Token::Colon {
            self.next_token();
            type_ann = self.parse_type_annotation();
        }

        if !self.expect_peek(Token::Assign) {
            self.report_error_with_hint(
                format!("Expected '=' after variable name '{}'", name),
                "Try adding '=' followed by a value, e.g., 'let x = 5'".to_string(),
            );
            return None;
        }
        self.next_token();

        let value = self.parse_expression(Precedence::Lowest)?;
        let expr = Some(Expression::Let {
            name,
            type_ann,
            value: Box::new(value),
        });
        self.wrap(expr, line, col)
    }

    fn parse_const_expression(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();
        let name = match &self.current_token {
            Token::Ident(name) => name.clone(),
            _ => {
                self.report_error_with_msg(format!(
                    "Expected constant name after 'const', got {:?}",
                    self.current_token
                ));
                return None;
            }
        };

        let mut type_ann = None;
        if self.peek_token == Token::Colon {
            self.next_token();
            type_ann = self.parse_type_annotation();
        }

        if !self.expect_peek(Token::Assign) {
            self.report_error_with_hint(
                format!("Expected '=' after constant name '{}'", name),
                "Constants must be initialized immediately. Try adding '= <value>'".to_string(),
            );
            return None;
        }
        self.next_token();

        let value = self.parse_expression(Precedence::Lowest)?;
        let expr = Some(Expression::Const {
            name,
            type_ann,
            value: Box::new(value),
        });
        self.wrap(expr, line, col)
    }

    fn parse_return_expression(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();
        let value = self.parse_expression(Precedence::Lowest)?;
        let expr = Some(Expression::Return(Box::new(value)));
        self.wrap(expr, line, col)
    }

    fn parse_block_expressions(&mut self) -> Vec<Expression> {
        let mut expressions = vec![];
        while self.current_token != Token::End
            && self.current_token != Token::Else
            && self.current_token != Token::EOF
        {
            if let Some(expr) = self.parse_expression(Precedence::Lowest) {
                expressions.push(expr);
            }
            self.next_token();
        }
        expressions
    }

    fn parse_block_expression(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();
        let exprs = self.parse_block_expressions();

        if self.current_token != Token::End {
            self.report_error_with_hint(
                format!(
                    "Expected 'end' to close block, got {:?}",
                    self.current_token
                ),
                "Blocks started with 'do' must explicitly end with 'end'".to_string(),
            );
            return None;
        }
        let expr = Some(Expression::Block(exprs));
        self.wrap(expr, line, col)
    }

    fn parse_function_expression(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        let mut name = None;
        if let Token::Ident(n) = &self.peek_token.clone() {
            self.next_token();
            name = Some(n.clone());
        }

        if !self.expect_peek(Token::LParen) {
            return None;
        }

        let parameters = self.parse_function_parameters()?;

        let mut return_type = None;
        if self.peek_token == Token::Colon {
            self.next_token();
            return_type = self.parse_type_annotation();
        }

        let body = if self.peek_token == Token::Do {
            self.next_token();
            self.next_token();
            let exprs = self.parse_block_expressions();
            if self.current_token != Token::End {
                self.report_error_with_hint(
                    format!(
                        "Expected 'end' to close function body, got {:?}",
                        self.current_token
                    ),
                    "Function bodies starting with 'do' must explicitly end with 'end'".to_string(),
                );
                return None;
            }
            exprs
        } else {
            self.next_token();
            vec![self.parse_expression(Precedence::Lowest)?]
        };

        let func = Expression::Function {
            parameters,
            return_type,
            body,
        };

        let expr = if let Some(n) = name {
            Some(Expression::Const {
                name: n,
                type_ann: None,
                value: Box::new(func),
            })
        } else {
            Some(func)
        };
        self.wrap(expr, line, col)
    }

    fn parse_function_parameters(&mut self) -> Option<Vec<(String, Option<TypeAnn>)>> {
        let mut identifiers = vec![];

        if self.peek_token == Token::RParen {
            self.next_token();
            return Some(identifiers);
        }

        self.next_token();
        if let Token::Ident(name) = &self.current_token {
            let name_str = name.clone();
            let mut type_ann = None;
            if self.peek_token == Token::Colon {
                self.next_token();
                type_ann = self.parse_type_annotation();
            }
            identifiers.push((name_str, type_ann));
        }

        while self.peek_token == Token::Comma {
            self.next_token();
            self.next_token();
            if let Token::Ident(name) = &self.current_token {
                let name_str = name.clone();
                let mut type_ann = None;
                if self.peek_token == Token::Colon {
                    self.next_token();
                    type_ann = self.parse_type_annotation();
                }
                identifiers.push((name_str, type_ann));
            }
        }

        if !self.expect_peek(Token::RParen) {
            self.report_error_with_hint(
                "Expected ')'".to_string(),
                "Function parameters must be closed".to_string(),
            );
            return None;
        }
        Some(identifiers)
    }

    fn parse_call_expression(&mut self, function: Expression) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        let arguments = self.parse_call_arguments()?;
        let expr = Some(Expression::Call {
            function: Box::new(function),
            arguments,
        });
        self.wrap(expr, line, col)
    }

    fn parse_call_arguments(&mut self) -> Option<Vec<Expression>> {
        let mut args = vec![];

        if self.peek_token == Token::RParen {
            self.next_token();
            return Some(args);
        }

        self.next_token();
        args.push(self.parse_expression(Precedence::Lowest)?);

        while self.peek_token == Token::Comma {
            self.next_token();
            self.next_token();
            args.push(self.parse_expression(Precedence::Lowest)?);
        }

        if !self.expect_peek(Token::RParen) {
            return None;
        }
        Some(args)
    }

    fn parse_if_expression(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();
        let condition = self.parse_expression(Precedence::Lowest)?;

        if self.peek_token == Token::Do {
            self.next_token();
            self.next_token();

            let consequence_exprs = self.parse_block_expressions();
            let consequence = Expression::Block(consequence_exprs);
            let mut alternative = None;

            if self.current_token == Token::Else {
                self.next_token();
                let alt_exprs = self.parse_block_expressions();
                alternative = Some(Box::new(Expression::Block(alt_exprs)));
            }

            if self.current_token != Token::End {
                self.report_error_with_hint(
                    format!(
                        "Expected 'end' for if expression, got {:?}",
                        self.current_token
                    ),
                    "If expressions starting with 'do' must explicitly end with 'end'".to_string(),
                );
                return None;
            }

            let expr = Some(Expression::If {
                condition: Box::new(condition),
                consequence: Box::new(consequence),
                alternative,
            });
            self.wrap(expr, line, col)
        } else {
            self.next_token();
            let consequence = self.parse_expression(Precedence::Lowest)?;

            if self.peek_token != Token::Else {
                self.report_error_with_hint(
                    "Inline 'if' expression is missing an 'else' branch".to_string(),
                    "Try adding 'else <value>' or format it as a block: 'if <cond> do <val> end'"
                        .to_string(),
                );
                return None;
            }

            self.next_token();
            self.next_token();
            let alternative = self.parse_expression(Precedence::Lowest)?;

            let expr = Some(Expression::If {
                condition: Box::new(condition),
                consequence: Box::new(consequence),
                alternative: Some(Box::new(alternative)),
            });
            self.wrap(expr, line, col)
        }
    }

    fn parse_grouped_expression(&mut self) -> Option<Expression> {
        self.next_token();
        let expr = self.parse_expression(Precedence::Lowest)?;
        if !self.expect_peek(Token::RParen) {
            return None;
        }
        Some(expr)
    }

    fn parse_prefix_expression(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        let operator = match &self.current_token {
            Token::Bang => "!",
            Token::Minus => "-",
            _ => return None,
        }
        .to_string();

        self.next_token();
        let right = self.parse_expression(Precedence::Prefix)?;
        let expr = Some(Expression::Prefix {
            operator,
            right: Box::new(right),
        });
        self.wrap(expr, line, col)
    }

    fn parse_infix_expression(&mut self, left: Expression) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        if self.current_token == Token::LParen {
            return self.parse_call_expression(left);
        }
        if self.current_token == Token::LBracket {
            return self.parse_index_expression(left);
        }
        if self.current_token == Token::Dot {
            return self.parse_dot_expression(left);
        }
        if self.current_token == Token::Colon {
            self.next_token();
            let method = match &self.current_token {
                Token::Ident(name) => name.clone(),
                _ => {
                    self.report_error_with_hint(
                        "Expected method name after ':'".to_string(),
                        "Method calls use the format object:method()".to_string(),
                    );
                    return None;
                }
            };

            if !self.expect_peek(Token::LParen) {
                return None;
            }
            let arguments = self.parse_call_arguments()?;

            let expr = Some(Expression::MethodCall {
                left: Box::new(left),
                method,
                arguments,
            });
            return self.wrap(expr, line, col);
        }
        if self.current_token == Token::Assign {
            self.next_token();
            let value = self.parse_expression(Precedence::Lowest)?;

            let unwrapped_left = self.unwrap_loc(left);
            match unwrapped_left {
                Expression::Identifier(name) => {
                    let expr = Some(Expression::Assign {
                        name,
                        value: Box::new(value),
                    });
                    return self.wrap(expr, line, col);
                }
                Expression::Index { left: obj, index } => {
                    let expr = Some(Expression::IndexAssign {
                        left: obj,
                        index,
                        value: Box::new(value),
                    });
                    return self.wrap(expr, line, col);
                }
                _ => {
                    self.report_error_with_hint(
                        "Invalid assignment target".to_string(),
                        "You can only assign values to variables or object properties".to_string(),
                    );
                    return None;
                }
            }
        }

        let operator = match &self.current_token {
            Token::Plus => "+",
            Token::Minus => "-",
            Token::Star => "*",
            Token::Slash => "/",
            Token::Eq => "==",
            Token::NotEq => "!=",
            Token::Greater => ">",
            Token::Less => "<",
            _ => return None,
        }
        .to_string();

        let precedence = self.current_precedence();
        self.next_token();
        let right = self.parse_expression(precedence)?;

        let expr = Some(Expression::Infix {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        });
        self.wrap(expr, line, col)
    }

    fn parse_loop_body(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        if self.peek_token == Token::Do {
            self.next_token();
            self.next_token();
            let exprs = self.parse_block_expressions();
            if self.current_token != Token::End {
                self.report_error_with_hint(
                    format!(
                        "Expected 'end' to close loop body, got {:?}",
                        self.current_token
                    ),
                    "Loop bodies starting with 'do' must explicitly end with 'end'".to_string(),
                );
                return None;
            }
            let expr = Some(Expression::Block(exprs));
            self.wrap(expr, line, col)
        } else {
            self.next_token();
            self.parse_expression(Precedence::Lowest)
        }
    }

    fn parse_loop_expression(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        let body = self.parse_loop_body()?;
        let expr = Some(Expression::Loop {
            body: Box::new(body),
        });
        self.wrap(expr, line, col)
    }

    fn parse_while_expression(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();
        let condition = self.parse_expression(Precedence::Lowest)?;
        let body = self.parse_loop_body()?;
        let expr = Some(Expression::While {
            condition: Box::new(condition),
            body: Box::new(body),
        });
        self.wrap(expr, line, col)
    }

    fn parse_for_expression(&mut self) -> Option<Expression> {
        let line = self.cur_line;
        let col = self.cur_col;
        self.next_token();

        let first_id = match &self.current_token {
            Token::Ident(name) => name.clone(),
            _ => {
                self.report_error_with_msg("Expected identifier after 'for'".to_string());
                return None;
            }
        };

        if self.peek_token == Token::Comma {
            self.next_token();
            self.next_token();
            let second_id = match &self.current_token {
                Token::Ident(name) => name.clone(),
                _ => {
                    self.report_error_with_msg(
                        "Expected second identifier after ',' in 'for'".to_string(),
                    );
                    return None;
                }
            };

            if !self.expect_peek(Token::In) {
                self.report_error_with_msg("Expected 'in' after loop variables".to_string());
                return None;
            }
            self.next_token();
            let iterable = self.parse_expression(Precedence::Lowest)?;

            let body = self.parse_loop_body()?;
            let expr = Some(Expression::ForHash {
                key: first_id,
                value: second_id,
                iterable: Box::new(iterable),
                body: Box::new(body),
            });
            self.wrap(expr, line, col)
        } else {
            if !self.expect_peek(Token::In) {
                self.report_error_with_msg("Expected 'in' after loop variable".to_string());
                return None;
            }
            self.next_token();
            let iterable = self.parse_expression(Precedence::Lowest)?;

            let body = self.parse_loop_body()?;
            let expr = Some(Expression::For {
                element: first_id,
                iterable: Box::new(iterable),
                body: Box::new(body),
            });
            self.wrap(expr, line, col)
        }
    }

    fn token_precedence(token: &Token) -> Precedence {
        match token {
            Token::Eq | Token::NotEq => Precedence::Equals,
            Token::Greater | Token::Less => Precedence::LessGreater,
            Token::Plus | Token::Minus => Precedence::Sum,
            Token::Star | Token::Slash => Precedence::Product,
            Token::LParen | Token::LBracket | Token::Dot | Token::Colon => Precedence::Call,
            Token::Assign => Precedence::Assign,
            _ => Precedence::Lowest,
        }
    }

    fn peek_is_infix_operator(&self) -> bool {
        matches!(
            self.peek_token,
            Token::Plus
                | Token::Minus
                | Token::Star
                | Token::Slash
                | Token::Eq
                | Token::NotEq
                | Token::Less
                | Token::Greater
                | Token::LParen
                | Token::Assign
                | Token::LBracket
                | Token::Dot
                | Token::Colon
        )
    }

    fn current_precedence(&self) -> Precedence {
        Self::token_precedence(&self.current_token)
    }

    fn peek_precedence(&self) -> Precedence {
        Self::token_precedence(&self.peek_token)
    }

    fn expect_peek(&mut self, expected: Token) -> bool {
        if self.peek_token == expected {
            self.next_token();
            true
        } else {
            self.report_error_with_hint(
                format!("Expected {:?}, but got {:?}", expected, self.peek_token),
                "Check the syntax in this area for missing punctuation".to_string(),
            );
            false
        }
    }

    fn unwrap_loc(&self, expr: Expression) -> Expression {
        let mut current = expr;
        while let Expression::Loc { expr: inner, .. } = current {
            current = *inner;
        }
        current
    }
}
