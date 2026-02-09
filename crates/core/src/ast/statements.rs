use crate::{
    ast::{InvalidExpression, MemberExpression, TupleType},
    Location,
};

use super::{
    expressions::{Expression, FunctionExpression, Identifier},
    types::{NamedType, Type},
    Pattern,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    Assignment(Assignment),
    Break(BreakStatement),
    Empty,
    Enum(EnumDefinition),
    Expression(ExpressionStatement),
    Function(FunctionDefinition),
    Invalid(InvalidStatement),
    MethodDefinition(MethodDefinition),
    Return(ReturnStatement),
    StructDefinition(StructDefinition),
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
    pub loc: Location,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableDeclaration {
    pub docs: Option<Docs>,
    /// This is the span of the actual declaration, and does not include the `docs` (if any)
    pub loc: Location,
    pub keyword: DeclarationKeyword,
    pub pattern: Box<Pattern>,
    pub value: Box<Expression>,
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

impl Into<Statement> for VariableDeclaration {
    fn into(self) -> Statement {
        Statement::VariableDeclaration(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodDefinition {
    pub loc: Location,
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
    pub loc: Location,
    pub name: Identifier,
    pub ty: NamedType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeAlias {
    pub docs: Option<Docs>,
    pub loc: Location,
    pub name: Option<Identifier>,
    pub params: Option<Vec<String>>,
    pub definition: Option<Type>,
}

impl Into<Statement> for TypeAlias {
    fn into(self) -> Statement {
        Statement::TypeAlias(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDefinition {
    pub docs: Option<Docs>,
    pub loc: Location,
    pub name: Option<Identifier>,
    pub params: Option<Vec<String>>,
    pub body: Option<TypeBody>,
}

impl Into<Statement> for StructDefinition {
    fn into(self) -> Statement {
        Statement::StructDefinition(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeBody {
    Struct(StructBody),
    Tuple(TupleType),
}

impl From<StructBody> for TypeBody {
    fn from(value: StructBody) -> Self {
        Self::Struct(value)
    }
}
impl From<TupleType> for TypeBody {
    fn from(value: TupleType) -> Self {
        Self::Tuple(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructBody {
    pub loc: Location,
    pub fields: Vec<StructDefinitionField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StructDefinitionField {
    Optional(StructOptionalField),
    Mandatory(StructMandatoryField),
}

impl StructDefinitionField {
    pub fn as_name(&self) -> Option<Identifier> {
        match self {
            Self::Mandatory(m) => m.name.as_ref().map(|i| i.clone()),
            Self::Optional(o) => o.name.as_ref().map(|i| i.clone()),
        }
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

impl Into<StructDefinitionField> for StructMandatoryField {
    fn into(self) -> StructDefinitionField {
        StructDefinitionField::Mandatory(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructOptionalField {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub default: Option<Expression>,
}

impl Into<StructDefinitionField> for StructOptionalField {
    fn into(self) -> StructDefinitionField {
        StructDefinitionField::Optional(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumDefinition {
    pub docs: Option<Docs>,
    pub loc: Location,
    pub name: String,
    pub params: Option<Vec<String>>,
    pub variants: Vec<VariantDefinition>,
}

impl Into<Statement> for EnumDefinition {
    fn into(self) -> Statement {
        Statement::Enum(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantDefinition {
    pub loc: Location,
    pub name: String,
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
    pub loc: Location,
    pub identifier: Identifier,
}
impl Into<Assignee> for IndirectionAssignee {
    fn into(self) -> Assignee {
        Assignee::Indirection(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BreakStatement {
    pub loc: Location,
    pub value: Option<Box<Expression>>,
}

impl Into<Statement> for BreakStatement {
    fn into(self) -> Statement {
        Statement::Break(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReturnStatement {
    pub loc: Location,
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
pub struct FunctionDefinition {
    pub docs: Option<Docs>,
    pub definition: FunctionExpression,
}

impl Into<Statement> for FunctionDefinition {
    fn into(self) -> Statement {
        Statement::Function(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvalidStatement {
    pub loc: Location,
}
impl Into<Statement> for InvalidStatement {
    fn into(self) -> Statement {
        Statement::Invalid(self)
    }
}
impl From<InvalidExpression> for InvalidStatement {
    fn from(value: InvalidExpression) -> Self {
        Self { loc: value.loc }
    }
}
