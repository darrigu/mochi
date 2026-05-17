use crate::ast::{Expression, Program, Statement};
use crate::lexer::{Lexer, Token};

#[derive(PartialEq, PartialOrd, Debug, Clone, Copy)]
pub enum Precedence {
    Lowest,
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
    pub errors: Vec<String>,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Self {
        let current_token = lexer.next_token();
        let peek_token = lexer.next_token();

        Parser {
            lexer,
            current_token,
            peek_token,
            errors: vec![],
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.peek_token = self.lexer.next_token();
    }

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program { statements: vec![] };

        while self.current_token != Token::EOF {
            if let Some(stmt) = self.parse_statement() {
                program.statements.push(stmt);
            }
            self.next_token();
        }

        program
    }

    fn parse_statement(&mut self) -> Option<Statement> {
        match self.current_token {
            Token::Let => self.parse_let_statement(),
            Token::Return => self.parse_return_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_let_statement(&mut self) -> Option<Statement> {
        self.next_token();

        let name = match &self.current_token {
            Token::Ident(name) => name.clone(),
            _ => {
                self.errors
                    .push(format!("Expected identifier, got {:?}", self.current_token));
                return None;
            }
        };

        if !self.expect_peek(Token::Assign) {
            return None;
        }

        self.next_token();

        let value = self.parse_expression(Precedence::Lowest)?;

        Some(Statement::Let { name, value })
    }

    fn parse_return_statement(&mut self) -> Option<Statement> {
        self.next_token();

        let return_val = self.parse_expression(Precedence::Lowest)?;

        Some(Statement::Return(return_val))
    }

    fn parse_expression_statement(&mut self) -> Option<Statement> {
        let expr = self.parse_expression(Precedence::Lowest)?;
        Some(Statement::Expression(expr))
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Option<Expression> {
        let mut left_expr = match &self.current_token {
            Token::Ident(name) => Some(Expression::Identifier(name.clone())),
            Token::Number(val) => Some(Expression::Number(*val)),
            Token::True => Some(Expression::Boolean(true)),
            Token::False => Some(Expression::Boolean(false)),
            Token::Bang | Token::Minus => self.parse_prefix_expression(),
            Token::LParen => self.parse_grouped_expression(),
            Token::If => self.parse_if_expression(),
            Token::Fn => self.parse_function_expression(),
            _ => {
                self.errors.push(format!(
                    "No prefix parse function for {:?}",
                    self.current_token
                ));
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

    fn parse_function_expression(&mut self) -> Option<Expression> {
        if !self.expect_peek(Token::LParen) {
            return None;
        }

        let parameters = self.parse_function_parameters()?;

        if !self.expect_peek(Token::Do) {
            return None;
        }

        self.next_token();
        let body = self.parse_block_statements();

        if self.current_token != Token::End {
            self.errors
                .push(format!("Expected 'end', got {:?}", self.current_token));
            return None;
        }

        Some(Expression::Function { parameters, body })
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

        if !self.expect_peek(Token::Do) {
            return None;
        }

        self.next_token();

        let consequence = self.parse_block_statements();

        let mut alternative = None;

        if self.current_token == Token::Else {
            self.next_token();
            alternative = Some(self.parse_block_statements());
        }

        if self.current_token != Token::End {
            self.errors
                .push(format!("Expected 'end', got {:?}", self.current_token));
            return None;
        }

        Some(Expression::If {
            condition: Box::new(condition),
            consequence,
            alternative,
        })
    }

    fn parse_block_statements(&mut self) -> Vec<Statement> {
        let mut statements = vec![];

        while self.current_token != Token::End
            && self.current_token != Token::Else
            && self.current_token != Token::EOF
        {
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.next_token();
        }

        statements
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

        let operator = match &self.current_token {
            Token::Plus => "+",
            Token::Minus => "-",
            Token::Star => "*",
            Token::Slash => "/",
            Token::Eq => "==",
            Token::NotEq => "!=",
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
                | Token::LParen
        )
    }

    fn current_precedence(&self) -> Precedence {
        match self.current_token {
            Token::Eq | Token::NotEq => Precedence::Equals,
            Token::Plus | Token::Minus => Precedence::Sum,
            Token::Star | Token::Slash => Precedence::Product,
            Token::LParen => Precedence::Call,
            _ => Precedence::Lowest,
        }
    }

    fn peek_precedence(&self) -> Precedence {
        match self.peek_token {
            Token::Eq | Token::NotEq => Precedence::Equals,
            Token::Plus | Token::Minus => Precedence::Sum,
            Token::Star | Token::Slash => Precedence::Product,
            Token::LParen => Precedence::Call,
            _ => Precedence::Lowest,
        }
    }

    fn expect_peek(&mut self, expected: Token) -> bool {
        if self.peek_token == expected {
            self.next_token();
            true
        } else {
            self.errors.push(format!(
                "Expected {:?}, got {:?}",
                expected, self.peek_token
            ));
            false
        }
    }
}
