use enum_from_derive::EnumFrom;
use tine_common::{
    locations::{Locatable, Location},
    module_path::{ModuleId, ModulePath},
};
use tine_ir_macros::ir_struct;
use tine_symbols::symbols::*;

use crate::{ir_enum, Block, Expression, FunctionExpression, Typed};

ir_enum!(Statement {
    Assignment(Assignment),
    Break(BreakStatement),
    Continue(ContinueStatement),
    Enum(EnumDefinition),
    Expression(Expression),
    Function(FunctionDefinition),
    Method(MethodDefinition),
    Return(ReturnStatement),
    Struct(StructDefinition),
    Use(UseDeclaration),
    Variable(VariableDeclaration),
});

#[ir_struct(untyped)]
#[derive(Debug, Clone)]
pub struct Assignment {
    #[child]
    pub pattern: Expression,
    #[child]
    /// Should be either:
    /// - an identifier
    /// - a member expression
    /// - a indirection (`*` + identifier)
    pub value: Expression,
}

#[ir_struct(untyped)]
#[derive(Debug, Clone)]
pub struct BreakStatement {
    #[child]
    pub expression: Option<Box<Expression>>,
}

#[ir_struct(untyped)]
#[derive(Debug, Clone)]
pub struct ContinueStatement {}

#[ir_struct(untyped)]
#[derive(Debug, Clone)]
pub struct EnumDefinition {
    pub symbol: EnumSymbolId,
}

#[ir_struct]
#[derive(Debug, Clone)]
pub struct FunctionDefinition {
    pub name: (Location, FunctionName),
    pub params: Vec<(Location, VariableSymbolId)>,
    #[child]
    pub body: Block,
}
#[derive(Debug, Clone, EnumFrom)]
pub enum FunctionName {
    Function(FunctionSymbolId),
    StaticMethod(MethodSymbolId),
}

impl Into<FunctionExpression> for FunctionDefinition {
    fn into(self) -> FunctionExpression {
        FunctionExpression {
            loc: self.loc,
            name: match &self.name.1 {
                FunctionName::Function(f) => Some((self.name.0, *f)),
                FunctionName::StaticMethod(_) => panic!(),
            },
            params: self.params,
            body: self.body,
            ty: self.ty,
        }
    }
}

#[ir_struct]
#[derive(Debug, Clone)]
pub struct MethodDefinition {
    pub receiver_name: (Location, VariableSymbolId),
    pub receiver_type: (Location, TypeSymbolId),
    pub mutating: bool,
    pub name: (Location, MethodSymbolId),
    pub params: Vec<(Location, VariableSymbolId)>,
    #[child]
    pub body: Block,
}

#[ir_struct(untyped)]
#[derive(Debug, Clone)]
pub struct ReturnStatement {
    #[child]
    pub expression: Option<Box<Expression>>,
}

#[ir_struct(untyped)]
#[derive(Debug, Clone)]
pub struct StructDefinition {
    pub symbol: StructSymbolId,
}

#[ir_struct(untyped)]
#[derive(Debug, Clone)]
pub struct UseDeclaration {
    pub module: ModuleId,
    pub path: ModulePath,
    pub symbols: Vec<SymbolId>,
}

#[ir_struct(untyped)]
#[derive(Debug, Clone)]
pub struct VariableDeclaration {
    pub mutable: bool,
    pub symbol: VariableSymbolId,
    #[child]
    pub value: Expression,
}
