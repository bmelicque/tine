use enum_from_derive::EnumFrom;

use crate::{
    ast::{Identifier, MapType, TupleExpression, Type},
    Location,
};

use super::{expressions::Expression, types::NamedType};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstructorLiteral {
    pub loc: Location,
    pub qualifiers: Vec<Identifier>,
    pub constructor: Constructor,
    pub body: Option<ConstructorBody>,
}

#[derive(Debug, Clone, EnumFrom, PartialEq, Eq, Hash)]
pub enum Constructor {
    Named(NamedType),
    Variant(VariantConstructor),
    Map(MapType),

    Invalid(Type),
}

impl Constructor {
    pub fn loc(&self) -> Location {
        match self {
            Constructor::Named(type_) => type_.loc,
            Constructor::Variant(variant) => variant.loc,
            Constructor::Map(map) => map.loc,
            Constructor::Invalid(t) => t.loc(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantConstructor {
    pub loc: Location,
    pub enum_name: Box<NamedType>,
    pub variant_name: Option<Identifier>,
}

#[derive(Debug, Clone, EnumFrom, PartialEq, Eq, Hash)]
pub enum ConstructorBody {
    Struct(StructLiteralBody),
    Tuple(TupleExpression),
}

impl ConstructorBody {
    pub fn loc(&self) -> Location {
        match self {
            ConstructorBody::Struct(body) => body.loc,
            ConstructorBody::Tuple(body) => body.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructLiteralBody {
    pub loc: Location,
    pub fields: Vec<ConstructorField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstructorField {
    pub loc: Location,
    pub key: Option<ConstructorKey>,
    pub value: Option<Expression>,
}

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum ConstructorKey {
    Name(Identifier),
    MapKey(Expression),
}

impl ConstructorKey {
    pub fn loc(&self) -> Location {
        match self {
            ConstructorKey::Name(name) => name.loc,
            ConstructorKey::MapKey(key) => key.loc(),
        }
    }
}
