use pest::Span;

use super::{expressions::Expression, types::Type, Pattern, PatternExpression};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Empty,
    VariableDeclaration(VariableDeclaration),
    TypeAlias(TypeAlias),
    Assignment(Assignment),
    Block(BlockStatement),
    Return(ReturnStatement),
    Expression(ExpressionStatement),
}

impl Statement {
    pub fn is_empty(&self) -> bool {
        *self == Statement::Empty
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclaration {
    pub span: Span<'static>,
    pub pattern: Box<Pattern>,
    pub op: DeclarationOp,
    pub value: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAlias {
    pub span: Span<'static>,
    pub name: String,
    pub params: Option<Vec<String>>,
    pub definition: Box<TypeDefinition>,
}

impl Into<Statement> for TypeAlias {
    fn into(self) -> Statement {
        Statement::TypeAlias(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeDefinition {
    Struct(StructDefinition),
    Enum(EnumDefinition),
    Trait(TraitDefinition),
    Type(Type),
}

impl From<Type> for TypeDefinition {
    fn from(value: Type) -> Self {
        TypeDefinition::Type(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDefinition {
    pub span: Span<'static>,
    pub fields: Vec<StructDefinitionField>,
}

impl Into<TypeDefinition> for StructDefinition {
    fn into(self) -> TypeDefinition {
        TypeDefinition::Struct(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
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

    pub fn as_span(&self) -> Span<'static> {
        match self {
            Self::Mandatory(m) => m.span.clone(),
            Self::Optional(o) => o.span.clone(),
        }
    }

    pub fn is_optional(&self) -> bool {
        matches!(self, Self::Optional(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructMandatoryField {
    pub span: Span<'static>,
    pub name: String,
    pub definition: Type,
}

impl Into<StructDefinitionField> for StructMandatoryField {
    fn into(self) -> StructDefinitionField {
        StructDefinitionField::Mandatory(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructOptionalField {
    pub span: Span<'static>,
    pub name: String,
    pub default: Expression,
}

impl Into<StructDefinitionField> for StructOptionalField {
    fn into(self) -> StructDefinitionField {
        StructDefinitionField::Optional(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDefinition {
    pub span: Span<'static>,
    pub variants: Vec<VariantDefinition>,
}

impl Into<TypeDefinition> for EnumDefinition {
    fn into(self) -> TypeDefinition {
        TypeDefinition::Enum(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct UnitVariant {
    pub span: Span<'static>,
    pub name: String,
}

impl Into<VariantDefinition> for UnitVariant {
    fn into(self) -> VariantDefinition {
        VariantDefinition::Unit(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TupleVariant {
    pub span: Span<'static>,
    pub name: String,
    pub elements: Vec<Type>,
}

impl Into<VariantDefinition> for TupleVariant {
    fn into(self) -> VariantDefinition {
        VariantDefinition::Tuple(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructVariant {
    pub span: Span<'static>,
    pub name: String,
    pub def: StructDefinition,
}

impl Into<VariantDefinition> for StructVariant {
    fn into(self) -> VariantDefinition {
        VariantDefinition::Struct(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitDefinition {
    pub span: Span<'static>,
    pub name: String,
    pub body: Box<StructDefinition>,
}

impl Into<TypeDefinition> for TraitDefinition {
    fn into(self) -> TypeDefinition {
        TypeDefinition::Trait(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub span: Span<'static>,
    pub pattern: PatternExpression,
    pub value: Expression,
}

impl Into<Statement> for Assignment {
    fn into(self) -> Statement {
        Statement::Assignment(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStatement {
    pub span: Span<'static>,
    pub statements: Vec<Statement>,
}

impl Into<Statement> for BlockStatement {
    fn into(self) -> Statement {
        Statement::Block(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStatement {
    pub span: Span<'static>,
    pub value: Option<Box<Expression>>,
}

impl Into<Statement> for ReturnStatement {
    fn into(self) -> Statement {
        Statement::Return(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionStatement {
    pub expression: Box<Expression>,
}

impl Into<Statement> for ExpressionStatement {
    fn into(self) -> Statement {
        Statement::Expression(self)
    }
}

impl From<Expression> for ExpressionStatement {
    fn from(expression: Expression) -> Self {
        ExpressionStatement {
            expression: Box::new(expression),
        }
    }
}
