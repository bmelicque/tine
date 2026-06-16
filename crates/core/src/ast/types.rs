use enum_from_derive::EnumFrom;

use crate::Location;

#[derive(Debug, Clone, EnumFrom, PartialEq, Eq, Hash)]
pub enum Type {
    Array(ArrayType),
    Function(FunctionType),
    Map(MapType),
    Named(NamedType),
    Option(OptionType),
    Result(ResultType),
    Tuple(TupleType),
}

impl Type {
    pub fn loc(&self) -> Location {
        match self {
            Self::Array(t) => t.loc,
            Self::Function(t) => t.loc,
            Self::Map(t) => t.loc,
            Self::Named(t) => t.loc,
            Self::Option(t) => t.loc,
            Self::Result(t) => t.loc,
            Self::Tuple(t) => t.loc,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct NamedType {
    pub loc: Location,
    pub name: String,
    pub args: Option<Vec<Type>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OptionType {
    pub loc: Location,
    pub base: Option<Box<Type>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayType {
    pub loc: Location,
    pub element: Option<Box<Type>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TupleType {
    pub loc: Location,
    pub elements: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapType {
    pub loc: Location,
    pub key: Option<Box<Type>>,
    pub value: Option<Box<Type>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResultType {
    pub loc: Location,
    pub error: Option<Box<Type>>,
    pub ok: Option<Box<Type>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType {
    pub loc: Location,
    pub params: Vec<Type>,
    pub returned: Option<Box<Type>>,
}
