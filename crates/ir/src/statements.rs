use tine_common::{
    locations::{Locatable, Location},
    module_path::{ModuleId, ModulePath},
};
use tine_macros::{tree_struct, EnumFrom};
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

#[tree_struct(untyped)]
#[derive(Debug, Clone)]
pub struct Assignment {
    #[child]
    /// Should be either:
    /// - an identifier
    /// - a member expression
    /// - a indirection (`*` + identifier)
    pub pattern: Expression,
    #[child]
    pub value: Expression,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone)]
pub struct BreakStatement {
    #[child]
    pub expression: Option<Box<Expression>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone)]
pub struct ContinueStatement {}

#[tree_struct(untyped)]
#[derive(Debug, Clone)]
pub struct EnumDefinition {
    pub symbol: EnumSymbolId,
}

#[tree_struct]
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

#[tree_struct]
#[derive(Debug, Clone)]
pub struct MethodDefinition {
    pub mutating: bool,
    pub name: (Location, MethodSymbolId),
    pub params: Vec<(Location, VariableSymbolId)>,
    #[child]
    pub body: Block,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone)]
pub struct ReturnStatement {
    #[child]
    pub expression: Option<Box<Expression>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone)]
pub struct StructDefinition {
    pub symbol: StructSymbolId,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone)]
pub struct UseDeclaration {
    pub module: ModuleId,
    pub path: ModulePath,
    pub symbols: Vec<SymbolId>,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone)]
pub struct VariableDeclaration {
    pub mutable: bool,
    pub symbol: VariableSymbolId,
    #[child]
    pub value: Expression,
}
