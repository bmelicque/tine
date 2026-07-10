use enum_from_derive::EnumFrom;
use tine_ast as ast;
use tine_common::locations::Location;
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types::TypeId};

use crate::Statement;

#[derive(Debug, Clone, EnumFrom)]
pub enum Expression {
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
    IntLiteral(IntLiteral),
    Map(MapLiteral),
    Member(MemberExpression),
    Method(MethodExpression),
    StringLiteral(StringLiteral),
    Struct(StructLiteral),
    Tuple(TupleExpression),
    TypeMatch(TypeMatch),
    Unary(UnaryExpression),
}

impl Expression {
    pub fn loc(&self) -> Location {
        match self {
            Expression::Array(a) => a.loc,
            Expression::Binary(b) => b.loc,
            Expression::BooleanLiteral(boolean) => boolean.loc,
            Expression::Block(block) => block.loc,
            Expression::Call(call) => call.loc,
            Expression::Element(e) => e.loc,
            Expression::FloatLiteral(f) => f.loc,
            Expression::For(f) => f.loc,
            Expression::ForIn(f) => f.loc,
            Expression::Function(function) => function.loc,
            Expression::Identifier(identifier) => identifier.loc,
            Expression::If(if_expression) => if_expression.loc,
            Expression::IntLiteral(i) => i.loc,
            Expression::Map(m) => m.loc,
            Expression::Member(m) => m.loc,
            Expression::Method(m) => m.loc,
            Expression::StringLiteral(s) => s.loc,
            Expression::Struct(s) => s.loc,
            Expression::Tuple(tuple) => tuple.loc,
            Expression::TypeMatch(t) => t.loc,
            Expression::Unary(u) => u.loc,
        }
    }

    pub fn ty(&self) -> TypeId {
        match self {
            Expression::Array(array) => array.ty,
            Expression::Binary(binary) => binary.ty,
            Expression::BooleanLiteral(_) => TypeStore::BOOLEAN,
            Expression::Block(block) => block.ty,
            Expression::Call(call) => call.ty,
            Expression::Element(e) => e.ty,
            Expression::FloatLiteral(_) => TypeStore::FLOAT,
            Expression::For(f) => f.ty,
            Expression::ForIn(f) => f.ty,
            Expression::Function(function) => function.ty,
            Expression::Identifier(identifier) => identifier.ty,
            Expression::If(if_expression) => if_expression.ty,
            Expression::IntLiteral(_) => TypeStore::INTEGER,
            Expression::Map(m) => m.ty,
            Expression::Member(m) => m.ty,
            Expression::Method(m) => m.ty,
            Expression::StringLiteral(_) => TypeStore::STRING,
            Expression::Struct(s) => s.ty,
            Expression::Tuple(tuple) => tuple.ty,
            Expression::TypeMatch(_) => TypeStore::BOOLEAN,
            Expression::Unary(u) => u.ty,
        }
    }

    pub fn as_identifier<'a>(&'a self) -> Option<&'a Identifier> {
        match self {
            Self::Identifier(i) => Some(i),
            _ => None,
        }
    }

    pub fn walk<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Expression> + 'a> {
        match self {
            Self::BooleanLiteral(_)
            | Self::FloatLiteral(_)
            | Self::IntLiteral(_)
            | Self::StringLiteral(_) => Box::new(std::iter::empty()),

            Self::Array(a) => iterate(&a.elements),
            Self::Binary(b) => Box::new(b.left.walk().chain(b.right.walk())),
            Self::Block(b) => b.walk(),
            Self::Call(c) => {
                let callee = c.callee.walk();
                let args: Box<dyn Iterator<Item = &Expression>> = c
                    .args
                    .iter()
                    .fold(Box::new(std::iter::empty()), |acc, arg| {
                        Box::new(acc.chain(arg.walk()))
                    });

                Box::new(callee.chain(args))
            }
            Self::Element(e) => {
                let attributes: Box<dyn Iterator<Item = &Expression>> = e
                    .attributes
                    .iter()
                    .fold(Box::new(std::iter::empty()), |acc, arg| {
                        Box::new(acc.chain(arg.value.walk()))
                    });
                Box::new(attributes.chain(iterate(&e.children)))
            }
            Self::For(f) => match &f.condition {
                Some(c) => Box::new(c.walk().chain(f.body.walk())),
                None => f.body.walk(),
            },
            Self::ForIn(f) => Box::new(f.iterable.walk().chain(f.body.walk())),
            Self::Function(f) => f.body.walk(),
            Self::Identifier(_) => Box::new(vec![self].into_iter()),
            Self::If(i) => match &i.alternate {
                Some(alt) => Box::new(
                    i.condition
                        .walk()
                        .chain(i.consequent.walk())
                        .chain(alt.walk()),
                ),
                None => Box::new(i.condition.walk().chain(i.consequent.walk())),
            },
            Self::Map(m) => m
                .entries
                .iter()
                .fold(Box::new(std::iter::empty()), |acc, entry| {
                    Box::new(acc.chain(entry.key.walk()).chain(entry.value.walk()))
                }),
            Self::Member(m) => m.object.walk(),
            Self::Method(m) => m.host.walk(),
            Self::Struct(s) => s
                .fields
                .iter()
                .fold(Box::new(std::iter::empty()), |acc, field| {
                    Box::new(acc.chain(field.value.walk()))
                }),
            Self::Tuple(t) => iterate(&t.elements),
            Self::TypeMatch(t) => t.expr.walk(),
            Self::Unary(u) => u.operand.walk(),
        }
    }
}

fn iterate<'i: 's, 's>(
    expressions: &'i [Expression],
) -> Box<dyn Iterator<Item = &'i Expression> + 'i> {
    expressions
        .iter()
        .fold(Box::new(std::iter::empty()), |acc, arg| {
            Box::new(acc.chain(arg.walk()))
        })
}

#[derive(Debug, Clone)]
pub struct ArrayExpression {
    pub loc: Location,
    pub elements: Vec<Expression>,
    pub ty: TypeId,
}

pub type BinaryOperator = ast::BinaryOperator;

#[derive(Debug, Clone)]
pub struct BinaryExpression {
    pub loc: Location,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub op: BinaryOperator,
    pub ty: TypeId,
}

#[derive(Debug, Default, Clone)]
pub struct BooleanLiteral {
    pub loc: Location,
    pub value: bool,
}
impl std::fmt::Display for BooleanLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub loc: Location,
    pub statements: Vec<Statement>,
    pub ty: TypeId,
}

impl Block {
    pub fn walk<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Expression> + 'a> {
        self.statements
            .iter()
            .fold(Box::new(std::iter::empty()), |acc, el| {
                Box::new(acc.chain(el.walk()))
            })
    }
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

#[derive(Debug, Clone)]
pub struct CallExpression {
    pub loc: Location,
    pub callee: Box<Expression>,
    pub args: Vec<Expression>,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct ElementExpression {
    pub loc: Location,
    pub tag_name: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Expression>,
    pub ty: TypeId,
}
#[derive(Debug, Clone)]
pub struct Attribute {
    pub loc: Location,
    pub name: String,
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub struct FloatLiteral {
    pub loc: Location,
    pub value: f64,
}
impl std::fmt::Display for FloatLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone)]
pub struct ForExpression {
    pub loc: Location,
    pub condition: Option<Box<Expression>>,
    pub body: Block,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct ForInExpression {
    pub loc: Location,
    pub element: (Location, VariableSymbolId),
    pub iterable: Box<Expression>,
    pub body: Block,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct FunctionExpression {
    pub loc: Location,
    pub name: Option<(Location, FunctionSymbolId)>,
    pub params: Vec<(Location, VariableSymbolId)>,
    pub body: Block,
    pub ty: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub loc: Location,
    pub symbol: SymbolId,
    pub ty: TypeId,
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

#[derive(Debug, Clone)]
/// An `if ... else` expression
///
/// `if ... else if ...` are desugared to `if ... else { if ... }`, meaning the `else` branch (if any) will be a block.
///
/// `if const ...` and `if var ...` are not handle here because they are desugared as `match` expressions.
pub struct IfExpression {
    pub loc: Location,
    pub condition: Box<Expression>,
    pub consequent: Block,
    pub alternate: Option<Block>,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct IntLiteral {
    pub loc: Location,
    pub value: i64,
}
impl std::fmt::Display for IntLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone)]
pub struct MapLiteral {
    pub loc: Location,
    pub entries: Vec<MapEntry>,
    /// Should refer to a Map type
    pub ty: TypeId,
}
#[derive(Debug, Clone)]
pub struct MapEntry {
    pub loc: Location,
    pub key: Expression,
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub struct MemberExpression {
    pub loc: Location,
    pub object: Box<Expression>,
    pub member: (Location, MemberSymbolId),
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct MethodExpression {
    pub loc: Location,
    pub host: Box<Expression>,
    pub method: (Location, MethodSymbolId),
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub loc: Location,
    pub value: String,
}
impl std::fmt::Display for StringLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone)]
pub struct StructLiteral {
    pub loc: Location,
    pub constructor: StructConstructor,
    pub fields: Vec<StructLiteralField>,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub enum StructConstructor {
    Struct(Location, StructSymbolId),
    Enum(Location, EnumSymbolId, VariantSymbolId),
}

#[derive(Debug, Clone)]
pub struct StructLiteralField {
    pub loc: Location,
    pub name: (Location, MemberSymbolId),
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub struct TupleExpression {
    pub loc: Location,
    pub elements: Vec<Expression>,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct TypeMatch {
    pub loc: Location,
    pub expr: Box<Expression>,
    pub variant: VariantSymbolId,
}

pub type UnaryOperator = ast::UnaryOperator;

#[derive(Debug, Clone)]
pub struct UnaryExpression {
    pub loc: Location,
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
    pub ty: TypeId,
}
