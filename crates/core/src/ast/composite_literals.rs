use crate::{ast::Identifier, Location};

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
    pub fn loc(&self) -> Location {
        match self {
            Self::AnonymousStruct(c) => c.loc,
            Self::Array(c) => c.loc,
            Self::Map(c) => c.loc,
            Self::Option(c) => c.loc,
            Self::Struct(c) => c.loc,
            Self::Variant(c) => c.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapLiteral {
    pub loc: Location,
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
    pub loc: Location,
    pub key: Box<Expression>,
    pub value: Box<ExpressionOrAnonymous>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayLiteral {
    pub loc: Location,
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
    pub loc: Location,
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
    pub loc: Location,
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
    pub loc: Location,
    pub fields: Vec<StructLiteralField>,
}

impl Into<CompositeLiteral> for AnonymousStructLiteral {
    fn into(self) -> CompositeLiteral {
        CompositeLiteral::AnonymousStruct(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructLiteralField {
    pub loc: Location,
    pub prop: Identifier,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantLiteral {
    pub loc: Location,
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

impl VariantLiteralBody {
    pub fn loc(&self) -> Location {
        match self {
            Self::Tuple(tuple) => {
                let start = tuple.first().unwrap().loc();
                let end = tuple.last().unwrap().loc();
                Location::merge(start, end)
            }
            Self::Struct(st) => {
                let start = st.first().unwrap().loc;
                let end = st.last().unwrap().loc;
                Location::merge(start, end)
            }
        }
    }
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
    pub fn loc(&self) -> Location {
        match self {
            Self::Expression(expr) => expr.loc(),
            Self::Struct(s) => s.loc,
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
