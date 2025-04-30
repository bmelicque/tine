use pest::Span;

use super::{
    expressions::Expression,
    types::{ArrayType, MapType, NamedType, OptionType},
};

#[derive(Debug, Clone, PartialEq)]
pub enum CompositeLiteral {
    Map(MapLiteral),
    Array(ArrayLiteral),
    AnonymousArray(AnonymousArrayLiteral),
    Option(OptionLiteral),
    Struct(StructLiteral),
    AnonymousStruct(AnonymousStructLiteral),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapLiteral {
    pub span: Span<'static>,
    pub ty: MapType,
    pub entries: Vec<MapEntry>,
}

impl Into<CompositeLiteral> for MapLiteral {
    fn into(self) -> CompositeLiteral {
        CompositeLiteral::Map(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapEntry {
    pub key: Box<Expression>,
    pub value: Box<ExpressionOrAnonymous>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayLiteral {
    pub span: Span<'static>,
    pub ty: ArrayType,
    pub entries: Vec<ExpressionOrAnonymous>,
}

impl Into<CompositeLiteral> for ArrayLiteral {
    fn into(self) -> CompositeLiteral {
        CompositeLiteral::Array(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnonymousArrayLiteral {
    pub span: Span<'static>,
    pub entries: Vec<ExpressionOrAnonymous>,
}

impl Into<CompositeLiteral> for AnonymousArrayLiteral {
    fn into(self) -> CompositeLiteral {
        CompositeLiteral::AnonymousArray(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionLiteral {
    pub span: Span<'static>,
    pub ty: OptionType,
    pub value: Option<Box<ExpressionOrAnonymous>>,
}

impl Into<CompositeLiteral> for OptionLiteral {
    fn into(self) -> CompositeLiteral {
        CompositeLiteral::Option(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteral {
    pub span: Span<'static>,
    pub ty: NamedType,
    pub fields: Vec<StructLiteralField>,
}

impl Into<CompositeLiteral> for StructLiteral {
    fn into(self) -> CompositeLiteral {
        CompositeLiteral::Struct(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnonymousStructLiteral {
    pub span: Span<'static>,
    pub fields: Vec<StructLiteralField>,
}

impl Into<CompositeLiteral> for AnonymousStructLiteral {
    fn into(self) -> CompositeLiteral {
        CompositeLiteral::AnonymousStruct(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteralField {
    pub span: Span<'static>,
    pub prop: String,
    pub value: ExpressionOrAnonymous,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionOrAnonymous {
    Expression(Expression),
    Array(AnonymousArrayLiteral),
    Struct(AnonymousStructLiteral),
}

impl From<Expression> for ExpressionOrAnonymous {
    fn from(value: Expression) -> Self {
        ExpressionOrAnonymous::Expression(value)
    }
}
impl From<AnonymousArrayLiteral> for ExpressionOrAnonymous {
    fn from(value: AnonymousArrayLiteral) -> Self {
        ExpressionOrAnonymous::Array(value)
    }
}
impl From<AnonymousStructLiteral> for ExpressionOrAnonymous {
    fn from(value: AnonymousStructLiteral) -> Self {
        ExpressionOrAnonymous::Struct(value)
    }
}
