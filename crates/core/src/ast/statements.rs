use crate::{
    ast::{InvalidExpression, MemberExpression},
    locations::Span,
};

use super::{
    expressions::{Expression, FunctionExpression, Identifier},
    types::{NamedType, Type},
    Pattern,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    Empty,
    Assignment(Assignment),
    Break(BreakStatement),
    Expression(ExpressionStatement),
    Invalid(InvalidStatement),
    MethodDefinition(MethodDefinition),
    Return(ReturnStatement),
    TypeAlias(TypeAlias),
    VariableDeclaration(VariableDeclaration),
}

impl Statement {
    pub fn is_empty(&self) -> bool {
        *self == Statement::Empty
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Docs {
    pub span: Span,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableDeclaration {
    pub docs: Option<Docs>,
    /// This is the span of the actual declaration, and does not include the `docs` (if any)
    pub span: Span,
    pub pattern: Box<Pattern>,
    pub op: DeclarationOp,
    pub value: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeclarationOp {
    Mut,
    Const,
}

impl From<String> for DeclarationOp {
    fn from(value: String) -> Self {
        match value.as_str() {
            ":=" => Self::Mut,
            "::" => Self::Const,
            _ => panic!(),
        }
    }
}

impl Into<Statement> for VariableDeclaration {
    fn into(self) -> Statement {
        Statement::VariableDeclaration(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodDefinition {
    pub span: Span,
    pub receiver: MethodReceiver,
    pub name: Identifier,
    pub definition: FunctionExpression,
}

impl Into<Statement> for MethodDefinition {
    fn into(self) -> Statement {
        Statement::MethodDefinition(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodReceiver {
    pub span: Span,
    pub name: Identifier,
    pub ty: NamedType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeAlias {
    pub span: Span,
    pub name: String,
    pub params: Option<Vec<String>>,
    pub op: DefinitionOp,
    pub definition: Box<TypeDefinition>,
}

impl Into<Statement> for TypeAlias {
    fn into(self) -> Statement {
        Statement::TypeAlias(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DefinitionOp {
    Strict,
    Like,
}

impl From<String> for DefinitionOp {
    fn from(value: String) -> Self {
        match value.as_str() {
            "::" => Self::Strict,
            ":~" => Self::Like,
            _ => panic!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeDefinition {
    Struct(StructDefinition),
    Enum(EnumDefinition),
    Type(Type),
}

impl TypeDefinition {
    pub fn as_span(&self) -> Span {
        match self {
            Self::Struct(s) => s.span,
            Self::Enum(e) => e.span,
            Self::Type(t) => t.as_span(),
        }
    }
}

impl From<Type> for TypeDefinition {
    fn from(value: Type) -> Self {
        TypeDefinition::Type(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDefinition {
    pub span: Span,
    pub fields: Vec<StructDefinitionField>,
}

impl Into<TypeDefinition> for StructDefinition {
    fn into(self) -> TypeDefinition {
        TypeDefinition::Struct(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StructDefinitionField {
    Optional(StructOptionalField),
    Mandatory(StructMandatoryField),
}

impl StructDefinitionField {
    pub fn as_name(&self) -> String {
        match self {
            Self::Mandatory(m) => m.name.clone(),
            Self::Optional(o) => o.name.clone(),
        }
    }

    pub fn as_span(&self) -> Span {
        match self {
            Self::Mandatory(m) => m.span.clone(),
            Self::Optional(o) => o.span.clone(),
        }
    }

    pub fn is_optional(&self) -> bool {
        matches!(self, Self::Optional(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructMandatoryField {
    pub span: Span,
    pub name: String,
    pub definition: Type,
}

impl Into<StructDefinitionField> for StructMandatoryField {
    fn into(self) -> StructDefinitionField {
        StructDefinitionField::Mandatory(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructOptionalField {
    pub span: Span,
    pub name: String,
    pub default: Expression,
}

impl Into<StructDefinitionField> for StructOptionalField {
    fn into(self) -> StructDefinitionField {
        StructDefinitionField::Optional(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumDefinition {
    pub span: Span,
    pub variants: Vec<VariantDefinition>,
}

impl Into<TypeDefinition> for EnumDefinition {
    fn into(self) -> TypeDefinition {
        TypeDefinition::Enum(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariantDefinition {
    Unit(UnitVariant),
    Tuple(TupleVariant),
    Struct(StructVariant),
}

impl VariantDefinition {
    pub fn as_name(&self) -> String {
        match self {
            Self::Unit(unit) => unit.name.clone(),
            Self::Tuple(tuple) => tuple.name.clone(),
            Self::Struct(struc) => struc.name.clone(),
        }
    }

    pub fn is_unit(&self) -> bool {
        matches!(self, Self::Unit(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnitVariant {
    pub span: Span,
    pub name: String,
}

impl Into<VariantDefinition> for UnitVariant {
    fn into(self) -> VariantDefinition {
        VariantDefinition::Unit(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TupleVariant {
    pub span: Span,
    pub name: String,
    pub elements: Vec<Type>,
}

impl Into<VariantDefinition> for TupleVariant {
    fn into(self) -> VariantDefinition {
        VariantDefinition::Tuple(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructVariant {
    pub span: Span,
    pub name: String,
    pub def: StructDefinition,
}

impl Into<VariantDefinition> for StructVariant {
    fn into(self) -> VariantDefinition {
        VariantDefinition::Struct(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Assignment {
    pub span: Span,
    pub pattern: Assignee,
    pub value: Expression,
}

impl Into<Statement> for Assignment {
    fn into(self) -> Statement {
        Statement::Assignment(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Assignee {
    Member(MemberExpression),
    Indirection(IndirectionAssignee),

    Pattern(Pattern),
}
impl From<MemberExpression> for Assignee {
    fn from(value: MemberExpression) -> Self {
        Self::Member(value)
    }
}
impl From<Pattern> for Assignee {
    fn from(value: Pattern) -> Self {
        Self::Pattern(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndirectionAssignee {
    pub span: Span,
    pub identifier: Identifier,
}
impl Into<Assignee> for IndirectionAssignee {
    fn into(self) -> Assignee {
        Assignee::Indirection(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BreakStatement {
    pub span: Span,
    pub value: Option<Box<Expression>>,
}

impl Into<Statement> for BreakStatement {
    fn into(self) -> Statement {
        Statement::Break(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReturnStatement {
    pub span: Span,
    pub value: Option<Box<Expression>>,
}

impl Into<Statement> for ReturnStatement {
    fn into(self) -> Statement {
        Statement::Return(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExpressionStatement {
    pub expression: Box<Expression>,
}

impl Into<Statement> for ExpressionStatement {
    fn into(self) -> Statement {
        match *self.expression {
            Expression::Invalid(i) => Statement::Invalid(i.into()),
            expr => Statement::Expression(expr.into()),
        }
    }
}

impl From<Expression> for ExpressionStatement {
    fn from(expression: Expression) -> Self {
        ExpressionStatement {
            expression: Box::new(expression),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvalidStatement {
    pub span: Span,
}
impl Into<Statement> for InvalidStatement {
    fn into(self) -> Statement {
        Statement::Invalid(self)
    }
}
impl From<InvalidExpression> for InvalidStatement {
    fn from(value: InvalidExpression) -> Self {
        Self { span: value.span }
    }
}
