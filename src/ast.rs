use crate::scanner::{Token, TokenLiteral};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Expr {
    Assign {
        name: Token,
        value: Box<Expr>,
        id: ExprId,
    },

    Binary {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },

    Call {
        callee: Box<Expr>,
        paren: Token,
        arguments: Vec<Expr>,
    },

    Get {
        object: Box<Expr>,
        name: Token,
    },

    Grouping {
        expression: Box<Expr>,
    },

    Literal {
        value: TokenLiteral,
    },

    Logical {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },

    Set {
        object: Box<Expr>,
        name: Token,
        value: Box<Expr>,
    },

    Super {
        keyword: Token,
        method: Token,
        id: ExprId,
    },

    This {
        keyword: Token,
        id: ExprId,
    },

    Unary {
        operator: Token,
        right: Box<Expr>,
    },

    Variable(ExprId, ExprVariable),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ExprId(pub usize);

#[derive(Clone, Debug)]
pub struct ExprVariable {
    pub name: Token,
}

pub struct StmtFunction {
    pub name: Token,
    pub params: Vec<Token>,
    pub body: Vec<Stmt>,
}

#[derive(Clone)]
pub enum Stmt {
    Block {
        statements: Vec<Stmt>,
    },

    Class {
        name: Token,
        superclass: Option<Expr>,
        methods: Vec<Rc<StmtFunction>>,
    },

    Expression(Expr),

    Function(Rc<StmtFunction>),

    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },

    Print {
        expression: Expr,
    },

    Return {
        keyword: Token,
        value: Option<Expr>,
    },

    Var {
        name: Token,
        initializer: Option<Expr>,
    },

    While {
        condition: Expr,
        body: Box<Stmt>,
    },
}
