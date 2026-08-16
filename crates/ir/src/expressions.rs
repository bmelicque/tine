use tine_ast as ast;
use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;
use tine_symbols::symbols::*;
use tine_types::store::TypeStore;

use crate::{ir_enum, Pattern, PushNodes, Statement, Typed};

ir_enum!(
    @typed
    Expression {
        Array(ArrayExpression),
        Binary(BinaryExpression),
        BooleanLiteral(BooleanLiteral),
        Block(Block),
        Call(CallExpression),
        Element(ElementExpression),
        FloatLiteral(FloatLiteral),
        For(ForExpression),
        ForIn(ForInExpression),
        Function(FunctionExpression),
        Identifier(Identifier),
        If(IfExpression),
        Index(IndexExpression),
        IntLiteral(IntLiteral),
        IntrinsicCall(IntrinsicCall),
        IntrinsicConstruct(IntrinsicConstruct),
        Match(MatchExpression),
        Member(MemberExpression),
        Method(MethodExpression),
        StringLiteral(StringLiteral),
        Struct(StructExpression),
        Tuple(TupleExpression),
        TypeMatch(TypeMatch),
        Unary(UnaryExpression),
    }
);
impl Expression {
    pub fn contains_this(&self) -> bool {
        self.walk()
            .filter_map(|n| n.as_expression())
            .any(|e| match e {
                Expression::Member(m) => m.object.is_none(),
                Expression::Method(m) => m.host.is_none(),
                _ => false,
            })
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct ArrayExpression {
    #[child]
    pub elements: Vec<Expression>,
}

pub type BinaryOperator = ast::BinaryOperator;

#[tree_struct]
#[derive(Debug, Clone)]
pub struct BinaryExpression {
    #[child]
    pub left: Box<Expression>,
    #[child]
    pub right: Box<Expression>,
    pub op: BinaryOperator,
}

#[tree_struct(ty = TypeStore::BOOLEAN)]
#[derive(Debug, Default, Clone)]
pub struct BooleanLiteral {
    pub value: bool,
}
impl std::fmt::Display for BooleanLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct Block {
    #[child]
    pub statements: Vec<Statement>,
}
impl From<Expression> for Block {
    fn from(value: Expression) -> Self {
        match value {
            Expression::Block(b) => b,
            _ => Block {
                loc: value.loc(),
                ty: value.ty(),
                statements: vec![Statement::Expression(value)],
            },
        }
    }
}
impl From<Statement> for Block {
    fn from(value: Statement) -> Self {
        Block {
            loc: value.loc(),
            ty: TypeStore::UNIT,
            statements: vec![value],
        }
    }
}
impl Block {
    pub fn returned_values(&self) -> Vec<Option<&Expression>> {
        self.statements
            .iter()
            .flat_map(|s| s.walk())
            .filter_map(|n| n.as_statement())
            .filter_map(|s| s.as_return())
            .map(|r| r.expression.as_deref())
            .chain(self.statements.last().map(|s| s.as_expression()))
            .collect()
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct CallExpression {
    #[child]
    pub callee: Box<Expression>,
    #[child]
    pub args: Vec<Expression>,
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct ElementExpression {
    pub tag_name: String,
    pub attributes: Vec<Attribute>,
    #[child]
    pub children: Vec<Expression>,
}
#[derive(Debug, Clone)]
pub struct Attribute {
    pub loc: Location,
    pub name: String,
    pub value: Expression,
}

#[tree_struct(ty = TypeStore::FLOAT)]
#[derive(Debug, Clone)]
pub struct FloatLiteral {
    pub value: f64,
}
impl std::fmt::Display for FloatLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct ForExpression {
    #[child]
    pub condition: Option<Box<Expression>>,
    #[child]
    pub body: Block,
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct ForInExpression {
    pub element: Pattern,
    #[child]
    pub iterable: Box<Expression>,
    #[child]
    pub body: Block,
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct FunctionExpression {
    pub name: Option<(Location, FunctionSymbolId)>,
    pub params: Vec<(Location, VariableSymbolId)>,
    #[child]
    pub body: Block,
}

#[tree_struct]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub symbol: SymbolId,
}
impl From<Identifier> for SymbolId {
    fn from(value: Identifier) -> Self {
        value.symbol
    }
}
impl From<&Identifier> for SymbolId {
    fn from(value: &Identifier) -> Self {
        value.symbol
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
/// An `if ... else` expression
///
/// `if ... else if ...` are desugared to `if ... else { if ... }`, meaning the `else` branch (if any) will be a block.
///
/// `if const ...` and `if var ...` are not handle here because they are desugared as `match` expressions.
pub struct IfExpression {
    #[child]
    pub condition: Box<Expression>,
    #[child]
    pub consequent: Block,
    #[child]
    pub alternate: Option<Block>,
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct IndexExpression {
    #[child]
    pub object: Option<Box<Expression>>,
    pub index: (Location, usize),
}

#[tree_struct(ty = TypeStore::INTEGER)]
#[derive(Debug, Clone)]
pub struct IntLiteral {
    pub value: i64,
}
impl std::fmt::Display for IntLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct IntrinsicCall {
    pub callee: FunctionSymbolId,
    pub args: Vec<Expression>,
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct IntrinsicConstruct {
    pub constructor: StructSymbolId,
    pub fields: Vec<StructLiteralField>,
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct MatchExpression {
    #[child]
    pub scrutinee: Box<Expression>,
    pub arms: Vec<(Pattern, Expression)>,
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct MemberExpression {
    #[child]
    pub object: Option<Box<Expression>>,
    pub member: (Location, MemberSymbolId),
}
impl MemberExpression {
    pub fn root_identifier(&self) -> Result<Option<&Identifier>, ()> {
        match self.object.as_deref() {
            Some(Expression::Identifier(i)) => Ok(Some(i)),
            Some(Expression::Member(m)) => m.root_identifier(),
            Some(_) => Err(()),
            None => Ok(None),
        }
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct MethodExpression {
    #[child]
    pub host: Option<Box<Expression>>,
    pub method: (Location, MethodSymbolId),
    #[child]
    pub args: Vec<Expression>,
}

#[tree_struct(ty = TypeStore::STRING)]
#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub value: String,
}
impl std::fmt::Display for StringLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct StructExpression {
    pub constructor: (Location, StructSymbolId),
    #[child]
    pub fields: Vec<StructLiteralField>,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone)]
pub struct StructLiteralField {
    pub name: (Location, MemberSymbolId),
    #[child]
    pub value: Expression,
}
impl<'a> PushNodes<'a> for StructLiteralField {
    fn push_nodes(&'a self, stack: &mut Vec<crate::Node<'a>>) {
        self.push_children(stack);
    }
}

#[tree_struct]
#[derive(Debug, Clone)]
pub struct TupleExpression {
    pub elements: Vec<Expression>,
}

#[tree_struct(ty = TypeStore::BOOLEAN)]
#[derive(Debug, Clone)]
pub struct TypeMatch {
    pub expr: Box<Expression>,
    pub variant: VariantSymbolId,
}

pub type UnaryOperator = ast::UnaryOperator;

#[tree_struct]
#[derive(Debug, Clone)]
pub struct UnaryExpression {
    pub operator: UnaryOperator,
    #[child]
    pub operand: Box<Expression>,
}
