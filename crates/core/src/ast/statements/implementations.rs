use enum_from_derive::EnumFrom;

use crate::{
    ast::{BlockExpression, Docs, FunctionDefinition, FunctionParam, Identifier, NamedType, Type},
    Location,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Implementation {
    pub loc: Location,
    /// The type that will be implemented.
    ///
    /// For example:
    /// - `impl MyType { ... }`
    /// - `impl MyGeneric<TypeParam> { ... }`
    /// - `impl MyGeneric<TypeArg> { ... }`
    pub implemented_type: Option<NamedType>,
    pub items: Vec<ImplementationItem>,
}

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum ImplementationItem {
    Method(MethodDefinition),
    StaticMethod(FunctionDefinition),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodDefinition {
    pub docs: Option<Docs>,
    pub loc: Location,
    pub name: Option<Identifier>,
    pub type_params: Option<Vec<Identifier>>,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<Type>,
    pub body: BlockExpression,
}
