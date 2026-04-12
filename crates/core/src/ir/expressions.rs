use enum_from_derive::EnumFrom;

use crate::{
    ast,
    ir::Statement,
    type_checker::{SymbolRef, TypeStore},
    types::TypeId,
    Location,
};

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
            Expression::Identifier(identifier) => identifier.ty(),
            Expression::If(if_expression) => if_expression.ty,
            Expression::IntLiteral(_) => TypeStore::INTEGER,
            Expression::Map(m) => m.ty,
            Expression::Member(m) => m.ty,
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

    /// Iterates through all the symbols captured by this expression
    pub fn dependencies<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Identifier> + 'a> {
        Box::new(
            self.walk()
                .filter_map(|child| child.as_identifier())
                .filter(|i| !i.loc.is_within(self.loc())),
        )
    }

    pub fn is_pure(&self) -> bool {
        self.dependencies().next().is_none()
    }
}

fn iterate<'a>(expressions: &'a [Expression]) -> Box<dyn Iterator<Item = &'a Expression> + 'a> {
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

#[derive(Debug, Clone)]
pub struct BooleanLiteral {
    pub loc: Location,
    pub value: bool,
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
        Block {
            loc: value.loc(),
            ty: value.ty(),
            statements: vec![Statement::Expression(value)],
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
    pub element: Identifier,
    pub iterable: Box<Expression>,
    pub body: Block,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct FunctionExpression {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub params: Vec<Identifier>,
    pub body: Block,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct FunctionParams {
    pub loc: Location,
    pub params: Vec<Identifier>,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub loc: Location,
    pub symbol: SymbolRef,
}

impl Identifier {
    pub fn as_name(&self) -> String {
        self.symbol.borrow().name.clone()
    }

    pub fn ty(&self) -> TypeId {
        self.symbol.borrow().ty
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
    pub member: Identifier,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub loc: Location,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct StructLiteral {
    pub loc: Location,
    /// A ref to the struct/enum constructor
    pub constructor: SymbolRef,
    /// If this has been constructed from an enum variant, contains a ref to the given variant.
    pub variant: Option<SymbolRef>,
    pub fields: Vec<StructLiteralField>,
    pub ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct StructLiteralField {
    pub loc: Location,
    pub name: Identifier,
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
    pub constructor: SymbolRef,
}

pub type UnaryOperator = ast::UnaryOperator;

#[derive(Debug, Clone)]
pub struct UnaryExpression {
    pub loc: Location,
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
    pub ty: TypeId,
}
