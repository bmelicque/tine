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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodDefinition {
    pub docs: Option<Docs>,
    pub loc: Location,
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

impl Default for MethodDefinition {
    fn default() -> Self {
        Self {
            docs: None,
            loc: Location::dummy(),
            receiver: MethodReceiver {
                loc: Location::dummy(),
                pattern: None,
            },
            name: None,
            type_params: None,
            params: None,
            return_type: None,
            body: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodReceiver {
    pub loc: Location,
    pub pattern: Option<Pattern>,
}
