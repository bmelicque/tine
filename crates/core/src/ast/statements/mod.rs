mod implementations;

use enum_from_derive::EnumFrom;

pub use crate::ast::statements::implementations::*;
use crate::{
    ast::{InvalidExpression, MemberExpression, TupleType},
    Location,
};

use super::{
    expressions::{Expression, FunctionExpression, Identifier},
    types::Type,
    Pattern,
};

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    Assignment(Assignment),
    Break(BreakStatement),
    Enum(EnumDefinition),
    Expression(ExpressionStatement),
    Function(FunctionDefinition),
    Implementation(Implementation),
    Invalid(InvalidStatement),
    Return(ReturnStatement),
    StructDefinition(StructDefinition),
    TypeAlias(TypeAlias),
    VariableDeclaration(VariableDeclaration),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Docs {
    pub loc: Location,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableDeclaration {
    pub docs: Option<Docs>,
    /// This is the span of the actual declaration, and does not include the `docs` (if any)
    pub loc: Location,
    pub keyword: DeclarationKeyword,
    pub pattern: Option<Pattern>,
    pub value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeclarationKeyword {
    Const,
    Var,
}

impl From<&str> for DeclarationKeyword {
    fn from(value: &str) -> Self {
        match value {
            "const" => Self::Const,
            "var" => Self::Var,
            _ => panic!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeAlias {
    pub docs: Option<Docs>,
    pub loc: Location,
    pub name: Option<Identifier>,
    pub params: Option<Vec<Identifier>>,
    pub definition: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDefinition {
    pub docs: Option<Docs>,
    pub loc: Location,
    pub name: Option<Identifier>,
    pub params: Option<Vec<Identifier>>,
    pub body: Option<TypeBody>,
}

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum TypeBody {
    Struct(StructBody),
    Tuple(TupleType),
}

impl TypeBody {
    pub fn loc(&self) -> Location {
        match self {
            Self::Struct(body) => body.loc,
            Self::Tuple(body) => body.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructBody {
    pub loc: Location,
    pub fields: Vec<StructDefinitionField>,
}

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum StructDefinitionField {
    Optional(StructOptionalField),
    Mandatory(StructMandatoryField),
}

impl StructDefinitionField {
    pub fn as_name(&self) -> Option<Identifier> {
        let name = match self {
            Self::Mandatory(m) => &m.name,
            Self::Optional(o) => &o.name,
        };
        name.as_ref().map(|i| i.clone())
    }

    pub fn loc(&self) -> Location {
        match self {
            Self::Mandatory(m) => m.loc,
            Self::Optional(o) => o.loc,
        }
    }

    pub fn is_optional(&self) -> bool {
        matches!(self, Self::Optional(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructMandatoryField {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub definition: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructOptionalField {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub default: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumDefinition {
    pub docs: Option<Docs>,
    pub loc: Location,
    pub name: String,
    pub params: Option<Vec<Identifier>>,
    pub variants: Vec<VariantDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantDefinition {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub body: Option<TypeBody>,
}

impl VariantDefinition {
    pub fn is_unit(&self) -> bool {
        self.body.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Assignment {
    pub loc: Location,
    pub pattern: Option<Assignee>,
    pub value: Option<Expression>,
}

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum Assignee {
    Member(MemberExpression),
    Indirection(IndirectionAssignee),

    Pattern(Pattern),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndirectionAssignee {
    pub loc: Location,
    pub identifier: Identifier,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BreakStatement {
    pub loc: Location,
    pub value: Option<Box<Expression>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReturnStatement {
    pub loc: Location,
    pub value: Option<Box<Expression>>,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDefinition {
    pub docs: Option<Docs>,
    pub definition: FunctionExpression,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvalidStatement {
    pub loc: Location,
}
impl From<InvalidExpression> for InvalidStatement {
    fn from(value: InvalidExpression) -> Self {
        Self { loc: value.loc }
    }
}
