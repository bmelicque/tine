use pest::Span;

use super::{
    expressions::Expression,
    types::{ArrayType, MapType, NamedType, OptionType},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompositeLiteral {
    Array(ArrayLiteral),
    AnonymousStruct(AnonymousStructLiteral),
    Map(MapLiteral),
    Option(OptionLiteral),
    Struct(StructLiteral),
    Variant(VariantLiteral),
}

impl CompositeLiteral {
    pub fn as_span(&self) -> Span<'static> {
        match self {
            Self::AnonymousStruct(c) => c.span,
            Self::Array(c) => c.span,
            Self::Map(c) => c.span,
            Self::Option(c) => c.span,
            Self::Struct(c) => c.span,
            Self::Variant(c) => c.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapEntry {
    pub span: Span<'static>,
    pub key: Box<Expression>,
    pub value: Box<ExpressionOrAnonymous>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayLiteral {
    pub span: Span<'static>,
    pub ty: ArrayType,
    pub elements: Vec<ExpressionOrAnonymous>,
}

impl Into<CompositeLiteral> for ArrayLiteral {
    fn into(self) -> CompositeLiteral {
        CompositeLiteral::Array(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnonymousStructLiteral {
    pub span: Span<'static>,
    pub fields: Vec<StructLiteralField>,
}

impl Into<CompositeLiteral> for AnonymousStructLiteral {
    fn into(self) -> CompositeLiteral {
        CompositeLiteral::AnonymousStruct(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructLiteralField {
    pub span: Span<'static>,
    pub prop: String,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantLiteral {
    pub span: Span<'static>,
    pub ty: NamedType,
    pub name: String,
    pub body: Option<VariantLiteralBody>,
}

impl Into<CompositeLiteral> for VariantLiteral {
    fn into(self) -> CompositeLiteral {
        CompositeLiteral::Variant(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariantLiteralBody {
    Tuple(Vec<ExpressionOrAnonymous>),
    Struct(Vec<StructLiteralField>),
}

impl From<Vec<ExpressionOrAnonymous>> for VariantLiteralBody {
    fn from(value: Vec<ExpressionOrAnonymous>) -> Self {
        VariantLiteralBody::Tuple(value)
    }
}

impl From<Vec<StructLiteralField>> for VariantLiteralBody {
    fn from(value: Vec<StructLiteralField>) -> Self {
        VariantLiteralBody::Struct(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExpressionOrAnonymous {
    Expression(Expression),
    Struct(AnonymousStructLiteral),
}

impl ExpressionOrAnonymous {
    pub fn as_span(&self) -> Span<'static> {
        match self {
            Self::Expression(expr) => expr.as_span(),
            Self::Struct(s) => s.span.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            ExpressionOrAnonymous::Expression(Expression::Empty) => true,
            _ => false,
        }
    }
}

impl From<Expression> for ExpressionOrAnonymous {
    fn from(value: Expression) -> Self {
        ExpressionOrAnonymous::Expression(value)
    }
}
impl From<AnonymousStructLiteral> for ExpressionOrAnonymous {
    fn from(value: AnonymousStructLiteral) -> Self {
        ExpressionOrAnonymous::Struct(value)
    }
}
