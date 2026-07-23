mod implementations;

use tine_common::locations::{Locatable, Location};

use crate::{
    nodes::{ast_enum, ast_struct},
    InvalidExpression, MemberExpression,
};

use super::{
    expressions::{Expression, FunctionExpression, FunctionParams, Identifier},
    types::Type,
    Pattern,
};
pub use implementations::*;

ast_enum!(Statement {
    Assignment(Assignment),
    Break(BreakStatement),
    Continue(ContinueStatement),
    Enum(EnumDefinition),
    Expression(ExpressionStatement),
    Function(FunctionDefinition),
    Implementation(Implementation),
    Invalid(InvalidStatement),
    Return(ReturnStatement),
    StructDefinition(StructDefinition),
    Trait(TraitDefinition),
    TypeAlias(TypeAlias),
    VariableDeclaration(VariableDeclaration),
});

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Docs {
    pub loc: Location,
    pub text: String,
}

ast_struct!(
    #[derive(Default)]
    VariableDeclaration {
        docs: Option<Docs>,
        public: bool,
        mutable: bool,
        pattern: Option<Pattern>,
        annotation: Option<Type>,
        value: Option<Expression>,
    }
);

ast_struct!(TraitDefinition {
    docs: Option<Docs>,
    public: bool,
    name: Option<Identifier>,
    params: Option<Vec<Identifier>>,
    methods: Option<Vec<TraitMethod>>,
});
ast_struct!(TraitMethod {
    receiver: Option<Identifier>,
    name: Option<Identifier>,
    type_params: Option<Vec<Identifier>>,
    params: Option<FunctionParams>,
    return_annotation: Option<Type>,
});

ast_struct!(TypeAlias {
    docs: Option<Docs>,
    public: bool,
    name: Option<Identifier>,
    params: Option<Vec<Identifier>>,
    definition: Option<Type>,
});

ast_struct!(
    #[derive(Default)]
    StructDefinition {
        docs: Option<Docs>,
        meta: Option<Vec<MetaAttribute>>,
        public: bool,
        name: Option<Identifier>,
        params: Option<Vec<Identifier>>,
        body: Option<TypeBody>,
    }
);

ast_enum!(TypeBody {
    Struct(StructBody),
    Tuple(TupleBody),
});

ast_struct!(StructBody {
    fields: Vec<StructDefinitionField>,
});

ast_struct!(StructDefinitionField {
    public: bool,
    name: Option<Identifier>,
    definition: Option<Type>,
});

ast_struct!(
    /// (is_public, type)
    TupleBody {
        elements: Vec<(bool, Type)>,
    }
);

ast_struct!(EnumDefinition {
    docs: Option<Docs>,
    meta: Option<Vec<MetaAttribute>>,
    public: bool,
    name: Option<Identifier>,
    params: Option<Vec<Identifier>>,
    variants: Vec<VariantDefinition>,
});

ast_struct!(VariantDefinition {
    name: Option<Identifier>,
    body: Option<TypeBody>,
});
impl VariantDefinition {
    pub fn is_unit(&self) -> bool {
        self.body.is_none()
    }
}

ast_struct!(Assignment {
    pattern: Option<Assignee>,
    value: Option<Expression>,
});

ast_enum!(Assignee {
    Member(MemberExpression),
    Indirection(IndirectionAssignee),

    Pattern(Pattern),
});

ast_struct!(IndirectionAssignee {
    identifier: Identifier,
});

ast_struct!(BreakStatement {
    value: Option<Box<Expression>>,
});

ast_struct!(ContinueStatement {});

ast_struct!(ReturnStatement {
    value: Option<Box<Expression>>,
});

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExpressionStatement {
    pub expression: Box<Expression>,
}
impl From<Expression> for ExpressionStatement {
    fn from(expression: Expression) -> Self {
        ExpressionStatement {
            expression: Box::new(expression),
        }
    }
}
impl Locatable for ExpressionStatement {
    fn loc(&self) -> Location {
        self.expression.loc()
    }
}

ast_struct!(FunctionDefinition {
    docs: Option<Docs>,
    public: bool,
    definition: FunctionExpression,
});

ast_struct!(InvalidStatement {});
impl From<InvalidExpression> for InvalidStatement {
    fn from(value: InvalidExpression) -> Self {
        Self { loc: value.loc }
    }
}

ast_struct!(MetaAttribute {
    name: Option<Identifier>,
    args: Option<Vec<Identifier>>
});
