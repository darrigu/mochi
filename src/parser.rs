use crate::ast::{Expression, Program};
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

#[derive(Debug)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub message: String,
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

    fn report_error(&mut self, msg: String) {
        self.errors.push(Diagnostic {
            line: self.cur_line,
            col: self.cur_col,
            message: msg,
        });
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Option<Expression> {
        let mut left_expr = match &self.current_token {
            Token::Ident(name) => Some(Expression::Identifier(name.clone())),
            Token::Number(val) => Some(Expression::Number(*val)),
            Token::StringLiteral(val) => Some(Expression::StringLiteral(val.clone())),
            Token::True => Some(Expression::Boolean(true)),
            Token::False => Some(Expression::Boolean(false)),
            Token::Bang | Token::Minus => self.parse_prefix_expression(),
            Token::LParen => self.parse_grouped_expression(),
            Token::If => self.parse_if_expression(),
            Token::Fn => self.parse_function_expression(),
            Token::Do => self.parse_block_expression(),
            Token::Let => self.parse_let_expression(),
            Token::Const => self.parse_const_expression(),
            Token::Return => self.parse_return_expression(),
            _ => {
                let msg = match self.current_token {
                    Token::Illegal(c) => format!("Illegal character '{}'", c),
                    Token::EOF => "Unexpected end of file".to_string(),
                    _ => format!("Unexpected token {:?}", self.current_token),
                };
                self.report_error(msg);
                None
            }
        }?;

        while self.peek_token != Token::EOF && precedence < self.peek_precedence() {
            if !self.peek_is_infix_operator() {
                return Some(left_expr);
            }

            self.next_token();
            left_expr = self.parse_infix_expression(left_expr)?;
        }

        Some(left_expr)
    }

    fn parse_let_expression(&mut self) -> Option<Expression> {
        self.next_token();
        let name = match &self.current_token {
            Token::Ident(name) => name.clone(),
            _ => {
                self.report_error(format!(
                    "Expected variable name after 'let', got {:?}",
                    self.current_token
                ));
                return None;
            }
        };

        if !self.expect_peek(Token::Assign) {
            self.report_error(format!("Expected '=' after variable name '{}'", name));
            return None;
        }
        self.next_token();

        let value = self.parse_expression(Precedence::Lowest)?;
        Some(Expression::Let {
            name,
            value: Box::new(value),
        })
    }

    fn parse_const_expression(&mut self) -> Option<Expression> {
        self.next_token();
        let name = match &self.current_token {
            Token::Ident(name) => name.clone(),
            _ => {
                self.report_error(format!(
                    "Expected constant name after 'const', got {:?}",
                    self.current_token
                ));
                return None;
            }
        };

        if !self.expect_peek(Token::Assign) {
            self.report_error(format!("Expected '=' after constant name '{}'", name));
            return None;
        }
        self.next_token();

        let value = self.parse_expression(Precedence::Lowest)?;
        Some(Expression::Const {
            name,
            value: Box::new(value),
        })
    }

    fn parse_return_expression(&mut self) -> Option<Expression> {
        self.next_token();
        let value = self.parse_expression(Precedence::Lowest)?;
        Some(Expression::Return(Box::new(value)))
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
        self.next_token();

        let exprs = self.parse_block_expressions();

        if self.current_token != Token::End {
            self.report_error(format!("Expected 'end', got {:?}", self.current_token));
            return None;
        }

        Some(Expression::Block(exprs))
    }

    fn parse_function_expression(&mut self) -> Option<Expression> {
        let mut name = None;
        if let Token::Ident(n) = &self.peek_token.clone() {
            self.next_token();
            name = Some(n.clone());
        }

        if !self.expect_peek(Token::LParen) {
            return None;
        }

        let parameters = self.parse_function_parameters()?;

        let body = if self.peek_token == Token::Do {
            self.next_token();
            self.next_token();
            let exprs = self.parse_block_expressions();

            if self.current_token != Token::End {
                self.report_error(format!("Expected 'end', got {:?}", self.current_token));
                return None;
            }

            exprs
        } else {
            self.next_token();
            let expr = self.parse_expression(Precedence::Lowest)?;
            vec![expr]
        };

        let func = Expression::Function { parameters, body };
        if let Some(n) = name {
            Some(Expression::Const {
                name: n,
                value: Box::new(func),
            })
        } else {
            Some(func)
        }
    }

    fn parse_function_parameters(&mut self) -> Option<Vec<String>> {
        let mut identifiers = vec![];

        if self.peek_token == Token::RParen {
            self.next_token();
            return Some(identifiers);
        }

        self.next_token();

        if let Token::Ident(name) = &self.current_token {
            identifiers.push(name.clone());
        }

        while self.peek_token == Token::Comma {
            self.next_token();
            self.next_token();
            if let Token::Ident(name) = &self.current_token {
                identifiers.push(name.clone());
            }
        }

        if !self.expect_peek(Token::RParen) {
            return None;
        }

        Some(identifiers)
    }

    fn parse_call_expression(&mut self, function: Expression) -> Option<Expression> {
        let arguments = self.parse_call_arguments()?;
        Some(Expression::Call {
            function: Box::new(function),
            arguments,
        })
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
                self.report_error(format!("Expected 'end', got {:?}", self.current_token));
                return None;
            }

            Some(Expression::If {
                condition: Box::new(condition),
                consequence: Box::new(consequence),
                alternative,
            })
        } else {
            self.next_token();
            let consequence = self.parse_expression(Precedence::Lowest)?;

            if self.peek_token != Token::Else {
                self.report_error("Inline 'if' expression must have an 'else' branch. Use 'if ... do ... end' for optional conditions.".to_string());
                return None;
            }

            self.next_token();
            self.next_token();
            let alternative = self.parse_expression(Precedence::Lowest)?;

            Some(Expression::If {
                condition: Box::new(condition),
                consequence: Box::new(consequence),
                alternative: Some(Box::new(alternative)),
            })
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
        let operator = match &self.current_token {
            Token::Bang => "!",
            Token::Minus => "-",
            _ => return None,
        }
        .to_string();

        self.next_token();

        let right = self.parse_expression(Precedence::Prefix)?;

        Some(Expression::Prefix {
            operator,
            right: Box::new(right),
        })
    }

    fn parse_infix_expression(&mut self, left: Expression) -> Option<Expression> {
        if self.current_token == Token::LParen {
            return self.parse_call_expression(left);
        }

        if self.current_token == Token::Assign {
            self.next_token();

            let value = self.parse_expression(Precedence::Lowest)?;

            if let Expression::Identifier(name) = left {
                return Some(Expression::Assign {
                    name,
                    value: Box::new(value),
                });
            } else {
                self.report_error("Invalid assignment target".to_string());
                return None;
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

        Some(Expression::Infix {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        })
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
        )
    }

    fn current_precedence(&self) -> Precedence {
        match self.current_token {
            Token::Eq | Token::NotEq => Precedence::Equals,
            Token::Greater | Token::Less => Precedence::LessGreater,
            Token::Plus | Token::Minus => Precedence::Sum,
            Token::Star | Token::Slash => Precedence::Product,
            Token::LParen => Precedence::Call,
            Token::Assign => Precedence::Assign,
            _ => Precedence::Lowest,
        }
    }

    fn peek_precedence(&self) -> Precedence {
        match self.peek_token {
            Token::Eq | Token::NotEq => Precedence::Equals,
            Token::Greater | Token::Less => Precedence::LessGreater,
            Token::Plus | Token::Minus => Precedence::Sum,
            Token::Star | Token::Slash => Precedence::Product,
            Token::LParen => Precedence::Call,
            Token::Assign => Precedence::Assign,
            _ => Precedence::Lowest,
        }
    }

    fn expect_peek(&mut self, expected: Token) -> bool {
        if self.peek_token == expected {
            self.next_token();
            true
        } else {
            self.report_error(format!(
                "Expected {:?}, got {:?}",
                expected, self.peek_token
            ));
            false
        }
    }
}
