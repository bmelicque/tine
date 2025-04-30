use pest::Span;

use super::{expressions::Expression, types::Type};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    VariableDeclaration(VariableDeclaration),
    TypeDefinition(TypeDefinition),
    Assignment(Assignment),
    Block(BlockStatement),
    Return(ReturnStatement),
    Expression(ExpressionStatement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclaration {
    span: Span<'static>,
    name: String,
    op: DeclarationOp,
    value: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclarationOp {
    Mut,
    Const,
}

impl Into<Statement> for VariableDeclaration {
    fn into(self) -> Statement {
        Statement::VariableDeclaration(self)
    }
}

pub struct TypeAlias {
    span: Span<'static>,
    name: String,
    params: Option<Vec<String>>,
    definition: Box<TypeDefinition>,
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
    span: Span<'static>,
    fields: Vec<StructDefinitionField>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct StructMandatoryField {
    span: Span<'static>,
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
    span: Span<'static>,
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
    span: Span<'static>,
    variants: Vec<VariantDefinition>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct UnitVariant {
    span: Span<'static>,
    pub name: String,
}

impl Into<VariantDefinition> for UnitVariant {
    fn into(self) -> VariantDefinition {
        VariantDefinition::Unit(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TupleVariant {
    span: Span<'static>,
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
    span: Span<'static>,
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
    span: Span<'static>,
    name: String,
    body: Box<StructDefinition>,
}

impl Into<TypeDefinition> for TraitDefinition {
    fn into(self) -> TypeDefinition {
        TypeDefinition::Trait(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    span: Span<'static>,
    // TODO: could be a member expression or a tuple index expression
    name: String,
    value: Expression,
}

impl Into<Statement> for Assignment {
    fn into(self) -> Statement {
        Statement::Assignment(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStatement {
    span: Span<'static>,
    statements: Vec<Statement>,
}

impl Into<Statement> for BlockStatement {
    fn into(self) -> Statement {
        Statement::Block(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStatement {
    span: Span<'static>,
    value: Option<Box<Expression>>,
}

impl Into<Statement> for ReturnStatement {
    fn into(self) -> Statement {
        Statement::Return(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionStatement {
    expression: Box<Expression>,
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
