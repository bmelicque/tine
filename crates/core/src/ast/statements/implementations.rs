use enum_from_derive::EnumFrom;

use crate::{
    ast::{
        BlockExpression, Docs, FunctionDefinition, FunctionExpression, FunctionParams, Identifier,
        NamedType, Pattern, Type,
    },
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
    pub body: Option<ImplementationBody>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImplementationBody {
    pub loc: Location,
    pub items: Vec<ImplementationItem>,
}

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum ImplementationItem {
    Method(MethodDefinition),
    StaticMethod(FunctionDefinition),
}

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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct MethodDefinition {
    pub docs: Option<Docs>,
    pub loc: Location,
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct MethodReceiver {
    pub loc: Location,
    pub mutable: bool,
    pub pattern: Option<Pattern>,
}
