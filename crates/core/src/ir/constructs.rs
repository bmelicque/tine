use enum_from_derive::EnumFrom;

use crate::{
    ir::{Expression, Identifier, TupleExpression},
    types::TypeId,
    Location, SymbolRef,
};

#[derive(Debug, Clone)]
pub struct ConstructorLiteral {
    pub loc: Location,
    pub constructor: SymbolRef,
    pub body: Option<ConstructorBody>,
    pub ty: TypeId,
}

#[derive(Debug, Clone, EnumFrom)]
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

#[derive(Debug, Clone)]
pub struct StructLiteralBody {
    pub loc: Location,
    pub fields: Vec<ConstructorField>,
}

#[derive(Debug, Clone)]
pub struct ConstructorField {
    pub loc: Location,
    pub key: ConstructorKey,
    pub value: Expression,
}

#[derive(Debug, EnumFrom, Clone)]
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
