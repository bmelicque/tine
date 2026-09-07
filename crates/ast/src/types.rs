use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::{nodes::ast_enum, Identifier};

ast_enum!(Type {
    Tuple(TupleType),
    Named(NamedType),
    Array(ArrayType),
    Function(FunctionType),
    Map(MapType),
    Option(OptionType),
    Result(ResultType),
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NamedType {
    pub name: Identifier,
    pub args: Option<Vec<Type>>,
}
impl From<Identifier> for NamedType {
    fn from(name: Identifier) -> Self {
        Self {
            loc: name.loc,
            name,
            args: None,
        }
    }
}
impl From<Identifier> for Type {
    fn from(name: Identifier) -> Self {
        Self::Named(name.into())
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OptionType {
    pub base: Option<Box<Type>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ArrayType {
    pub element: Option<Box<Type>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TupleType {
    pub elements: Vec<Type>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MapType {
    pub key: Option<Box<Type>>,
    pub value: Option<Box<Type>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResultType {
    pub error: Option<Box<Type>>,
    pub ok: Option<Box<Type>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub returned: Option<Box<Type>>,
}
