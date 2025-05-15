use std::fmt;

use pest::Span;

use super::{composite_literals::CompositeLiteral, types::Type, Statement};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Empty,
    Array(ArrayExpression),
    Binary(BinaryExpression),
    BooleanLiteral(BooleanLiteral),
    Block(BlockExpression),
    CompositeLiteral(CompositeLiteral),
    FieldAccess(FieldAccessExpression),
    Function(FunctionExpression),
    Identifier(Identifier),
    NumberLiteral(NumberLiteral),
    StringLiteral(StringLiteral),
    Tuple(TupleExpression),
    TupleIndexing(TupleIndexingExpression),
}

impl Expression {
    pub fn as_span(&self) -> Span<'static> {
        match self {
            Self::Array(e) => e.span.clone(),
            Self::Binary(e) => e.span.clone(),
            Self::BooleanLiteral(e) => e.span.clone(),
            Self::Block(e) => e.span.clone(),
            Self::CompositeLiteral(e) => e.as_span(),
            Self::Empty => Span::new("", 0, 0).unwrap(),
            Self::FieldAccess(e) => e.span.clone(),
            Self::Function(e) => e.span.clone(),
            Self::Identifier(e) => e.span.clone(),
            Self::NumberLiteral(e) => e.span.clone(),
            Self::StringLiteral(e) => e.span.clone(),
            Self::Tuple(e) => e.span.clone(),
            Self::TupleIndexing(e) => e.span.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Expression::Empty
    }
}

impl From<CompositeLiteral> for Expression {
    fn from(value: CompositeLiteral) -> Self {
        Expression::CompositeLiteral(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayExpression {
    pub span: Span<'static>,
    pub elements: Vec<Expression>,
}

impl Into<Expression> for ArrayExpression {
    fn into(self) -> Expression {
        Expression::Array(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub span: Span<'static>,
}

impl Identifier {
    pub fn as_str(&self) -> &str {
        self.span.as_str()
    }
}

impl From<Span<'static>> for Identifier {
    fn from(span: Span<'static>) -> Self {
        Identifier { span }
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
    pub fn as_str(&self) -> &str {
        let str = self.span.as_str();
        &str[1..(str.len() - 1)]
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

impl BinaryOperator {
    fn as_string(&self) -> String {
        match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Sub => "-",
            BinaryOperator::Mul => "*",
            BinaryOperator::Div => "/",
            BinaryOperator::Mod => "%",
            BinaryOperator::Pow => "**",

            BinaryOperator::Eq => "==",
            BinaryOperator::Neq => "!=",
            BinaryOperator::Less => "<",
            BinaryOperator::Leq => "<=",
            BinaryOperator::Grt => ">",
            BinaryOperator::Geq => ">=",

            BinaryOperator::LAnd => "&&",
            BinaryOperator::LOr => "||",
        }
        .into()
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

impl From<String> for BinaryOperator {
    fn from(value: String) -> Self {
        match value.as_str() {
            "+" => BinaryOperator::Add,
            "-" => BinaryOperator::Sub,
            "*" => BinaryOperator::Mul,
            "/" => BinaryOperator::Div,
            "%" => BinaryOperator::Mod,
            "**" => BinaryOperator::Pow,

            "==" => BinaryOperator::Eq,
            "!=" => BinaryOperator::Neq,
            "<" => BinaryOperator::Less,
            "<=" => BinaryOperator::Leq,
            ">" => BinaryOperator::Grt,
            ">=" => BinaryOperator::Geq,

            "&&" => BinaryOperator::LAnd,
            "||" => BinaryOperator::LOr,

            _ => panic!("Invalid operator"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpression {
    pub span: Span<'static>,
    pub statements: Vec<Statement>,
}

impl Into<Expression> for BlockExpression {
    fn into(self) -> Expression {
        Expression::Block(self)
    }
}

pub trait PathExpression {
    fn root_expression(&self) -> Expression;
    fn base_expression(&self) -> &Expression;
    fn as_span(&self) -> pest::Span<'static>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldAccessExpression {
    pub span: Span<'static>,
    pub object: Box<Expression>,
    pub prop: Identifier,
}

impl PathExpression for FieldAccessExpression {
    fn as_span(&self) -> pest::Span<'static> {
        self.span
    }
    fn root_expression(&self) -> Expression {
        match *self.object.clone() {
            Expression::FieldAccess(expr) => expr.root_expression(),
            Expression::TupleIndexing(expr) => expr.root_expression(),
            expr => expr,
        }
    }
    fn base_expression(&self) -> &Expression {
        self.object.as_ref()
    }
}

impl Into<Expression> for FieldAccessExpression {
    fn into(self) -> Expression {
        Expression::FieldAccess(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TupleExpression {
    pub span: Span<'static>,
    pub elements: Vec<Expression>,
}

impl Into<Expression> for TupleExpression {
    fn into(self) -> Expression {
        Expression::Tuple(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TupleIndexingExpression {
    pub span: Span<'static>,
    pub tuple: Box<Expression>,
    pub index: NumberLiteral,
}

impl PathExpression for TupleIndexingExpression {
    fn as_span(&self) -> pest::Span<'static> {
        self.span
    }
    fn root_expression(&self) -> Expression {
        match *self.tuple.clone() {
            Expression::FieldAccess(expr) => expr.root_expression(),
            Expression::TupleIndexing(expr) => expr.root_expression(),
            expr => expr,
        }
    }
    fn base_expression(&self) -> &Expression {
        &self.tuple
    }
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
    pub block: BlockExpression,
}

impl Into<FunctionBody> for TypedBlock {
    fn into(self) -> FunctionBody {
        FunctionBody::TypedBlock(self)
    }
}
