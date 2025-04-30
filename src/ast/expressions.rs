use pest::Span;

use super::{composite_literals::CompositeLiteral, statements::BlockStatement, types::Type};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Identifier(Identifier),
    StringLiteral(StringLiteral),
    NumberLiteral(NumberLiteral),
    BooleanLiteral(BooleanLiteral),
    CompositeLiteral(CompositeLiteral),
    Binary(BinaryExpression),
    FieldAccess(FieldAccessExpression),
    TupleIndexing(TupleIndexingExpression),
    Function(FunctionExpression),
}

impl From<CompositeLiteral> for Expression {
    fn from(value: CompositeLiteral) -> Self {
        Expression::CompositeLiteral(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub span: Span<'static>,
}

impl Identifier {
    fn as_str(&self) -> &str {
        self.span.as_str()
    }
}

impl Into<Expression> for Identifier {
    fn into(self) -> Expression {
        Expression::Identifier(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringLiteral {
    pub span: Span<'static>,
}

impl StringLiteral {
    fn as_str(&self) -> &str {
        self.span.as_str()
    }
}

impl Into<Expression> for StringLiteral {
    fn into(self) -> Expression {
        Expression::StringLiteral(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberLiteral {
    pub span: Span<'static>,
    pub value: f64,
}

impl Into<Expression> for NumberLiteral {
    fn into(self) -> Expression {
        Expression::NumberLiteral(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BooleanLiteral {
    pub span: Span<'static>,
    pub value: bool,
}

impl Into<Expression> for BooleanLiteral {
    fn into(self) -> Expression {
        Expression::BooleanLiteral(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpression {
    pub span: Span<'static>,
    pub left: Box<Expression>,
    pub operator: BinaryOperator,
    pub right: Box<Expression>,
}

impl Into<Expression> for BinaryExpression {
    fn into(self) -> Expression {
        Expression::Binary(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    LAnd,
    LOr,
    Eq,
    Neq,
    Grt,
    Geq,
    Less,
    Leq,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldAccessExpression {
    pub span: Span<'static>,
    pub object: Box<Expression>,
    pub prop: Identifier,
}

impl Into<Expression> for FieldAccessExpression {
    fn into(self) -> Expression {
        Expression::FieldAccess(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TupleIndexingExpression {
    pub span: Span<'static>,
    pub tuple: Box<Expression>,
    pub index: NumberLiteral,
}

impl Into<Expression> for TupleIndexingExpression {
    fn into(self) -> Expression {
        Expression::TupleIndexing(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionExpression {
    pub span: Span<'static>,
    pub params: Vec<FunctionParam>,
    pub body: FunctionBody,
}

impl Into<Expression> for FunctionExpression {
    fn into(self) -> Expression {
        Expression::Function(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParam {
    pub span: Span<'static>,
    pub name: Identifier,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionBody {
    Expression(Box<Expression>),
    TypedBlock(TypedBlock),
}

impl From<Expression> for FunctionBody {
    fn from(value: Expression) -> Self {
        FunctionBody::Expression(Box::new(value))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedBlock {
    pub ty: Type,
    pub block: BlockStatement,
}

impl Into<FunctionBody> for TypedBlock {
    fn into(self) -> FunctionBody {
        FunctionBody::TypedBlock(self)
    }
}
