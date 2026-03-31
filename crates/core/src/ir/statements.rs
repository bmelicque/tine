use enum_from_derive::EnumFrom;

use crate::{
    ir::{Block, Expression, Identifier},
    types::TypeId,
    Location, ModuleId, SymbolRef,
};

#[derive(Debug, Clone, EnumFrom)]
pub enum Statement {
    Assignment(Assignment),
    Break(BreakStatement),
    Continue(ContinueStatement),
    Expression(Expression),
    Function(FunctionDefinition),
    Return(ReturnStatement),
    Use(UseDeclaration),
    Variable(VariableDeclaration),
}

impl Statement {
    pub fn walk<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Expression> + 'a> {
        match self {
            Self::Assignment(a) => Box::new(a.pattern.walk().chain(a.value.walk())),
            Self::Break(b) => match &b.expression {
                Some(e) => e.walk(),
                None => Box::new(std::iter::empty()),
            },
            Self::Continue(_) => Box::new(std::iter::empty()),
            Self::Expression(e) => e.walk(),
            Self::Function(f) => f.body.walk(),
            Self::Return(r) => match &r.expression {
                Some(e) => e.walk(),
                None => Box::new(std::iter::empty()),
            },
            Self::Use(_) => Box::new(std::iter::empty()),
            Self::Variable(v) => v.value.walk(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub loc: Location,
    pub pattern: Expression,
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub struct BreakStatement {
    pub loc: Location,
    pub expression: Option<Box<Expression>>,
}

#[derive(Debug, Clone)]
pub struct ContinueStatement {
    pub loc: Location,
}

#[derive(Debug, Clone)]
pub struct FunctionDefinition {
    pub loc: Location,
    pub name: Identifier,
    pub params: Vec<Identifier>,
    pub body: Block,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub loc: Location,
    pub expression: Option<Box<Expression>>,
}

#[derive(Debug, Clone)]
pub struct UseDeclaration {
    pub loc: Location,
    pub module: ModuleId,
    pub symbols: Vec<SymbolRef>,
}

#[derive(Debug, Clone)]
pub struct VariableDeclaration {
    pub loc: Location,
    pub mutable: bool,
    pub symbol: SymbolRef,
    pub value: Expression,
}
