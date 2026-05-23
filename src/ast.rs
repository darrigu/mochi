#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,
    Identifier(String),
    Number(f64),
    StringLiteral(String),
    Atom(String),
    Tuple(Vec<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub guard: Option<Expression>,
    pub body: Expression,
}

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
    Tuple(Vec<TypeAnn>),
    Any,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Identifier(String),
    Number(f64),
    StringLiteral(String),
    Atom(String),
    Array(Vec<Expression>),
    Tuple(Vec<Expression>),

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
    Loop {
        body: Box<Expression>,
    },
    While {
        condition: Box<Expression>,
        body: Box<Expression>,
    },
    For {
        element: String,
        iterable: Box<Expression>,
        body: Box<Expression>,
    },
    ForHash {
        key: String,
        value: String,
        iterable: Box<Expression>,
        body: Box<Expression>,
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
    Block(Vec<Expression>, bool),

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

    Match {
        subject: Box<Expression>,
        cases: Vec<MatchCase>,
    },

    Question(Box<Expression>),

    Import(Box<Expression>),

    Break(Option<Box<Expression>>),
    Continue,

    Loc {
        line: usize,
        col: usize,
        expr: Box<Expression>,
    },
}

#[derive(Debug, Clone)]
pub struct Program {
    pub expressions: Vec<Expression>,
}
