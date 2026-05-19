#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Identifier(String),
    Number(f64),
    StringLiteral(String),
    Boolean(bool),
    Array(Vec<Expression>),

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
        consequence: Box<Expression>,
        alternative: Option<Box<Expression>>,
    },

    Function {
        parameters: Vec<String>,
        body: Vec<Expression>,
    },
    Call {
        function: Box<Expression>,
        arguments: Vec<Expression>,
    },
    Return(Box<Expression>),
    Block(Vec<Expression>),

    Let {
        name: String,
        value: Box<Expression>,
    },
    Const {
        name: String,
        value: Box<Expression>,
    },
    Assign {
        name: String,
        value: Box<Expression>,
    },

    Hash(Vec<(Expression, Expression)>),
    Index {
        left: Box<Expression>,
        index: Box<Expression>,
    },
    IndexAssign {
        left: Box<Expression>,
        index: Box<Expression>,
        value: Box<Expression>,
    },
}

#[derive(Debug, Clone)]
pub struct Program {
    pub expressions: Vec<Expression>,
}
