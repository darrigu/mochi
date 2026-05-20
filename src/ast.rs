#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnn {
    Number,
    String,
    Atom,
    Array(Box<TypeAnn>),
    Hash(Vec<(String, TypeAnn)>),
    Function {
        params: Vec<TypeAnn>,
        ret: Box<TypeAnn>,
    },
    Any,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Identifier(String),
    Number(f64),
    StringLiteral(String),
    Atom(String),
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
        parameters: Vec<(String, Option<TypeAnn>)>,
        return_type: Option<TypeAnn>,
        body: Vec<Expression>,
    },
    Call {
        function: Box<Expression>,
        arguments: Vec<Expression>,
    },
    MethodCall {
        left: Box<Expression>,
        method: String,
        arguments: Vec<Expression>,
    },
    Return(Box<Expression>),
    Block(Vec<Expression>),

    Let {
        name: String,
        type_ann: Option<TypeAnn>,
        value: Box<Expression>,
    },
    Const {
        name: String,
        type_ann: Option<TypeAnn>,
        value: Box<Expression>,
    },
    Assign {
        name: String,
        value: Box<Expression>,
    },

    Hash(Vec<(String, Expression)>),
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
