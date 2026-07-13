use tine_common::locations::{Locatable, Location};

use crate::{
    nodes::{ast_enum, ast_struct},
    BlockExpression, Docs, FunctionDefinition, FunctionExpression, FunctionParams, Identifier,
    NamedType, Pattern, Type,
};

ast_struct!(
    /// `implemented_type` is th type being implemented.
    ///
    /// For example:
    /// - `impl MyType { ... }`
    /// - `impl MyGeneric<TypeParam> { ... }`
    /// - `impl MyGeneric<TypeArg> { ... }`
    Implementation {
        implemented_type: Option<NamedType>,
        body: Option<ImplementationBody>,
    }
);

ast_struct!(ImplementationBody {
    items: Vec<ImplementationItem>,
});

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

ast_struct!(
    #[derive(Default)]
    MethodDefinition {
        docs: Option<Docs>,
        public: bool,
        receiver: MethodReceiver,
        name: Option<Identifier>,
        type_params: Option<Vec<Identifier>>,
        params: Option<FunctionParams>,
        return_type: Option<Type>,
        body: Option<BlockExpression>,
    }
);
impl MethodDefinition {
    pub fn copy_function(&mut self, function: FunctionExpression) {
        self.name = function.name;
        self.type_params = function.type_params;
        self.params = function.params;
        self.return_type = function.return_type;
        self.body = function.body;
    }
}

ast_struct!(
    #[derive(Default)]
    MethodReceiver {
        mutable: bool,
        pattern: Option<Pattern>,
    }
);
