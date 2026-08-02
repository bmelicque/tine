use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::{
    nodes::ast_enum, BlockExpression, Docs, FunctionDefinition, FunctionExpression, FunctionParams,
    Identifier, NamedType, Pattern, Type,
};

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
    pub items: Vec<ImplementationItem>,
}

ast_enum!(ImplementationItem {
    Method(MethodDefinition),
    StaticMethod(FunctionDefinition),
});
impl ImplementationItem {
    pub fn docs(&self) -> &Option<Docs> {
        match self {
            Self::Method(m) => &m.docs,
            Self::StaticMethod(m) => &m.docs,
        }
    }

    pub fn name(&self) -> &Option<Identifier> {
        match self {
            Self::Method(m) => &m.name,
            Self::StaticMethod(m) => &m.definition.name,
        }
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MethodDefinition {
    pub docs: Option<Docs>,
    pub public: bool,
    pub receiver: MethodReceiver,
    pub name: Option<Identifier>,
    pub type_params: Option<Vec<Identifier>>,
    pub params: Option<FunctionParams>,
    pub return_type: Option<Type>,
    pub body: Option<BlockExpression>,
}

impl MethodDefinition {
    pub fn copy_function(&mut self, function: FunctionExpression) {
        self.name = function.name;
        self.type_params = function.type_params;
        self.params = function.params;
        self.return_type = function.return_type;
        self.body = function.body;
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MethodReceiver {
    pub mutable: bool,
    pub pattern: Option<Pattern>,
    pub self_type: Option<Identifier>,
}
