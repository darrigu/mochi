#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let { name: String, value: Expression },
    Return(Expression),
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Identifier(String),
    Number(f64),
    StringLiteral(String),
    Boolean(bool),

    Prefix {
        operator: String,
        right: Box<Expression>,
    },

    Infix {
        left: Box<Expression>,
        operator: String,
        right: Box<Expression>,
    },

    If {
        condition: Box<Expression>,
        consequence: Vec<Statement>,
        alternative: Option<Vec<Statement>>,
    },

    Function {
        parameters: Vec<String>,
        body: Vec<Statement>,
    },

    Call {
        function: Box<Expression>,
        arguments: Vec<Expression>,
    },
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}
