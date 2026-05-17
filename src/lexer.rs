#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Fn,
    Let,
    If,
    Else,
    End,
    Return,
    Do,
    True,
    False,

    Ident(String),
    Number(f64),
    StringLiteral(String),

    Plus,
    Minus,
    Star,
    Slash,
    Assign,
    Eq,
    NotEq,
    Bang,

    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Dot,

    EOF,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    read_position: usize,
    ch: char,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer {
            input: input.chars().collect(),
            position: 0,
            read_position: 0,
            ch: '\0',
        };
        lexer.read_char();
        lexer
    }

    fn read_char(&mut self) {
        if self.read_position >= self.input.len() {
            self.ch = '\0';
        } else {
            self.ch = self.input[self.read_position];
        }
        self.position = self.read_position;
        self.read_position += 1;
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let token = match self.ch {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            ',' => Token::Comma,
            '.' => Token::Dot,
            '=' => {
                if self.peek_char() == '=' {
                    self.read_char();
                    Token::Eq
                } else {
                    Token::Assign
                }
            }
            '!' => {
                if self.peek_char() == '=' {
                    self.read_char();
                    Token::NotEq
                } else {
                    Token::Bang
                }
            }
            '\0' => Token::EOF,
            _ => {
                if self.ch.is_alphabetic() {
                    let ident = self.read_identifier();
                    return self.lookup_ident(&ident);
                } else if self.ch.is_numeric() {
                    return Token::Number(self.read_number());
                } else {
                    panic!("Unknown character: {}", self.ch);
                }
            }
        };

        self.read_char();
        token
    }

    fn skip_whitespace(&mut self) {
        while self.ch.is_whitespace() {
            self.read_char();
        }
    }

    fn read_identifier(&mut self) -> String {
        let pos = self.position;
        while self.ch.is_alphanumeric() || self.ch == '_' {
            self.read_char();
        }
        self.input[pos..self.position].iter().collect()
    }

    fn read_number(&mut self) -> f64 {
        let pos = self.position;
        while self.ch.is_numeric() || self.ch == '.' {
            self.read_char();
        }
        let num_str: String = self.input[pos..self.position].iter().collect();
        num_str.parse().unwrap_or(0.0)
    }

    fn peek_char(&self) -> char {
        if self.read_position >= self.input.len() {
            '\0'
        } else {
            self.input[self.read_position]
        }
    }

    fn lookup_ident(&self, ident: &str) -> Token {
        match ident {
            "fn" | "def" => Token::Fn,
            "let" => Token::Let,
            "if" => Token::If,
            "else" => Token::Else,
            "end" => Token::End,
            "return" => Token::Return,
            "do" => Token::Do,
            "true" => Token::True,
            "false" => Token::False,
            _ => Token::Ident(ident.to_string()),
        }
    }
}
