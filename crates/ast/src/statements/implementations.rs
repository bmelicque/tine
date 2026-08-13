use tine_common::locations::Locatable;
use tine_macros::tree_struct;

use crate::{Docs, Expression, FunctionParams, Identifier, NamedType, Type};

/// `implemented_type` is th type being implemented.
///
/// For example:
/// - `impl MyType { ... }`
/// - `impl MyGeneric<TypeParam> { ... }`
/// - `impl MyGeneric<TypeArg> { ... }`
#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Implementation {
    pub implemented_type: Option<NamedType>,
    pub body: Option<ImplementationBody>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImplementationBody {
    pub items: Vec<MethodDefinition>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MethodDefinition {
    pub docs: Option<Docs>,
    pub public: bool,
    pub static_: bool,
    pub mut_: bool,
    pub name: Option<Identifier>,
    pub type_params: Option<Vec<Identifier>>,
    pub params: Option<FunctionParams>,
    pub return_type: Option<Type>,
    pub body: Option<Box<Expression>>,
}
