#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Fn,
    Let,
    Const,
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
    Greater,
    Less,
    Bang,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Dot,
    Illegal(char),
    EOF,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    read_position: usize,
    ch: char,
    pub line: usize,
    pub col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer {
            input: input.chars().collect(),
            position: 0,
            read_position: 0,
            ch: '\0',
            line: 1,
            col: 0,
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

        if self.ch == '\n' {
            self.line += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }

        self.position = self.read_position;
        self.read_position += 1;
    }

    pub fn next_token(&mut self) -> (Token, usize, usize) {
        self.skip_ignored();

        let tok_line = self.line;
        let tok_col = self.col;

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
            '>' => Token::Greater,
            '<' => Token::Less,
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
            '"' => Token::StringLiteral(self.read_string()),
            '\0' => Token::EOF,
            _ => {
                if self.ch.is_alphabetic() {
                    let ident = self.read_identifier();
                    return (self.lookup_ident(&ident), tok_line, tok_col);
                } else if self.ch.is_numeric() {
                    return (Token::Number(self.read_number()), tok_line, tok_col);
                } else {
                    let t = Token::Illegal(self.ch);
                    self.read_char();
                    return (t, tok_line, tok_col);
                }
            }
        };

        self.read_char();
        (token, tok_line, tok_col)
    }

    fn skip_ignored(&mut self) {
        loop {
            if self.ch.is_whitespace() {
                self.read_char();
            } else if self.ch == '-' && self.peek_char() == '-' {
                while self.ch != '\n' && self.ch != '\0' {
                    self.read_char();
                }
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> String {
        let pos = self.position + 1;
        loop {
            self.read_char();
            if self.ch == '"' || self.ch == '\0' {
                break;
            }
        }
        self.input[pos..self.position].iter().collect()
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
            "const" => Token::Const,
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
