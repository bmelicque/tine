mod implementations;

use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::{nodes::ast_enum, walk::PushNodes, InvalidExpression, PathExpression};

use super::{
    expressions::{Expression, FunctionExpression, FunctionParams, Identifier},
    types::Type,
    Pattern,
};
pub use implementations::*;

ast_enum!(Statement {
    Assignment(Assignment),
    Break(BreakStatement),
    Comment(Comment),
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
impl<I: Into<Expression>> From<I> for Statement {
    fn from(value: I) -> Self {
        Self::Expression(ExpressionStatement {
            expression: Box::new(value.into()),
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Docs {
    pub loc: Location,
    pub text: String,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VariableDeclaration {
    pub docs: Option<Docs>,
    pub public: bool,
    pub mutable: bool,
    pub pattern: Option<Pattern>,
    pub annotation: Option<Type>,
    #[child]
    pub value: Option<Expression>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TraitDefinition {
    pub docs: Option<Docs>,
    pub public: bool,
    pub name: Option<Identifier>,
    pub params: Option<Vec<Identifier>>,
    pub methods: Option<Vec<TraitMethod>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TraitMethod {
    pub receiver: Option<Identifier>,
    pub name: Option<Identifier>,
    pub type_params: Option<Vec<Identifier>>,
    pub params: Option<FunctionParams>,
    pub return_annotation: Option<Type>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TypeAlias {
    pub docs: Option<Docs>,
    pub public: bool,
    pub name: Option<Identifier>,
    pub params: Option<Vec<Identifier>>,
    pub definition: Option<Type>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StructDefinition {
    pub docs: Option<Docs>,
    pub meta: Option<Vec<MetaAttribute>>,
    pub public: bool,
    pub name: Option<Identifier>,
    pub params: Option<Vec<Identifier>>,
    pub body: Option<Vec<StructItem>>,
}

ast_enum!(StructItem {
    Field(StructDefinitionField),
    Method(MethodDefinition),
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StructDefinitionField {
    pub docs: Option<Docs>,
    pub public: bool,
    pub name: Option<Identifier>,
    pub definition: Option<Type>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EnumDefinition {
    pub docs: Option<Docs>,
    pub meta: Option<Vec<MetaAttribute>>,
    pub public: bool,
    pub name: Option<Identifier>,
    pub params: Option<Vec<Identifier>>,
    pub items: Option<Vec<EnumItem>>,
}

ast_enum!(EnumItem {
    Variant(VariantDefinition),
    Method(MethodDefinition)
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VariantDefinition {
    pub docs: Option<Docs>,
    pub public: bool,
    pub name: Option<Identifier>,
    pub body: Option<VariantBody>,
}
impl VariantDefinition {
    pub fn is_unit(&self) -> bool {
        self.body.is_none()
    }
}

/// (is_public, type)
#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VariantBody {
    pub elements: Vec<(bool, Type)>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub pattern: Option<Assignee>,
    #[child]
    pub value: Option<Expression>,
}

ast_enum!(Assignee {
    Invalid(InvalidExpression),

    Path(PathExpression),
    Indirection(IndirectionAssignee),
    Struct(StructAssignee),
    Tuple(TupleAssignee),
});
impl From<Identifier> for Assignee {
    fn from(value: Identifier) -> Self {
        Self::Path(value.into())
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndirectionAssignee {
    pub inner: Option<Box<Assignee>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructAssignee {
    pub constructor: PathExpression,
    pub fields: Vec<StructAssigneeField>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TupleAssignee {
    pub elements: Vec<Assignee>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StructAssigneeField {
    pub key: Option<StructAssigneeFieldKey>,
    pub value: Option<Assignee>,
}
ast_enum!(StructAssigneeFieldKey {
    Identifier(Identifier),
    Invalid(InvalidExpression),
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BreakStatement {
    #[child]
    pub value: Option<Box<Expression>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Comment {}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContinueStatement {}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReturnStatement {
    #[child]
    pub value: Option<Box<Expression>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
impl ExpressionStatement {
    fn push_children<'a>(&'a self, stack: &mut Vec<crate::Node<'a>>) {
        self.expression.push_nodes(stack);
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    pub docs: Option<Docs>,
    pub public: bool,
    pub definition: FunctionExpression,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InvalidStatement {}
impl From<InvalidExpression> for InvalidStatement {
    fn from(value: InvalidExpression) -> Self {
        Self { loc: value.loc }
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MetaAttribute {
    pub name: Option<Identifier>,
    pub args: Option<Vec<Identifier>>,
}
