use std::fmt;

use ordered_float::OrderedFloat;

use crate::{ast::ElementExpression, Location};

use super::{composite_literals::CompositeLiteral, types::Type, Loop, Pattern, Statement};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expression {
    Empty,
    Array(ArrayExpression),
    Binary(BinaryExpression),
    BooleanLiteral(BooleanLiteral),
    Block(BlockExpression),
    Call(CallExpression),
    CompositeLiteral(CompositeLiteral),
    Element(ElementExpression),
    Function(FunctionExpression),
    Identifier(Identifier),
    If(IfExpression),
    IntLiteral(IntLiteral),
    IfDecl(IfPatExpression),
    Invalid(InvalidExpression),
    Loop(Loop),
    Match(MatchExpression),
    Member(MemberExpression),
    FloatLiteral(FloatLiteral),
    StringLiteral(StringLiteral),
    Tuple(TupleExpression),
    Unary(UnaryExpression),
}

impl Expression {
    pub fn loc(&self) -> Location {
        match self {
            Self::Array(e) => e.loc,
            Self::Binary(e) => e.loc,
            Self::BooleanLiteral(e) => e.loc,
            Self::Block(e) => e.loc,
            Self::Call(e) => e.loc,
            Self::CompositeLiteral(e) => e.loc(),
            Self::Element(e) => e.loc(),
            Self::Empty => Location::dummy(),
            Self::FloatLiteral(e) => e.loc,
            Self::Member(e) => e.loc,
            Self::Function(e) => e.loc,
            Self::Identifier(e) => e.loc,
            Self::If(e) => e.loc,
            Self::IfDecl(e) => e.loc,
            Self::IntLiteral(e) => e.loc,
            Self::Invalid(e) => e.loc,
            Self::Loop(e) => e.loc(),
            Self::Match(e) => e.loc,
            Self::StringLiteral(e) => e.loc,
            Self::Tuple(e) => e.loc,
            Self::Unary(e) => e.loc,
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Expression::Empty
    }
}

impl From<CompositeLiteral> for Expression {
    fn from(value: CompositeLiteral) -> Self {
        Expression::CompositeLiteral(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayExpression {
    pub loc: Location,
    pub elements: Vec<Expression>,
}

impl Into<Expression> for ArrayExpression {
    fn into(self) -> Expression {
        Expression::Array(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub loc: Location,
    pub text: String,
}

impl Identifier {
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }
}

impl Into<Expression> for Identifier {
    fn into(self) -> Expression {
        Expression::Identifier(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IfPatExpression {
    pub loc: Location,
    pub pattern: Box<Pattern>,
    pub scrutinee: Box<Expression>,
    pub consequent: Option<BlockExpression>,
    pub alternate: Option<Box<Alternate>>,
}

impl Into<Expression> for IfPatExpression {
    fn into(self) -> Expression {
        Expression::IfDecl(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IfExpression {
    pub loc: Location,
    pub condition: Box<Expression>,
    pub consequent: Option<BlockExpression>,
    pub alternate: Option<Box<Alternate>>,
}

impl Into<Expression> for IfExpression {
    fn into(self) -> Expression {
        Expression::If(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Alternate {
    Block(BlockExpression),
    If(IfExpression),
    IfDecl(IfPatExpression),
}
impl Alternate {
    pub fn loc(&self) -> Location {
        match self {
            Alternate::Block(b) => b.loc,
            Alternate::If(i) => i.loc,
            Alternate::IfDecl(i) => i.loc,
        }
    }
}
impl From<BlockExpression> for Alternate {
    fn from(value: BlockExpression) -> Self {
        Self::Block(value)
    }
}
impl From<IfExpression> for Alternate {
    fn from(value: IfExpression) -> Self {
        Self::If(value)
    }
}
impl From<IfPatExpression> for Alternate {
    fn from(value: IfPatExpression) -> Self {
        Self::IfDecl(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntLiteral {
    pub loc: Location,
    pub value: i64,
}

impl Into<Expression> for IntLiteral {
    fn into(self) -> Expression {
        Expression::IntLiteral(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvalidExpression {
    pub loc: Location,
}

impl Into<Expression> for InvalidExpression {
    fn into(self) -> Expression {
        Expression::Invalid(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchExpression {
    pub loc: Location,
    pub scrutinee: Option<Box<Expression>>,
    pub arms: Option<Vec<MatchArm>>,
}

impl Into<Expression> for MatchExpression {
    fn into(self) -> Expression {
        Expression::Match(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchArm {
    pub loc: Location,
    pub pattern: Option<Box<Pattern>>,
    pub expression: Option<Box<Expression>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StringLiteral {
    pub loc: Location,
    pub text: String,
}

impl StringLiteral {
    pub fn as_str(&self) -> &str {
        let str = self.text.as_str();
        &str[1..(str.len() - 1)]
    }
}

impl Into<Expression> for StringLiteral {
    fn into(self) -> Expression {
        Expression::StringLiteral(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FloatLiteral {
    pub loc: Location,
    pub value: OrderedFloat<f64>,
}

impl Into<Expression> for FloatLiteral {
    fn into(self) -> Expression {
        Expression::FloatLiteral(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BooleanLiteral {
    pub loc: Location,
    pub value: bool,
}

impl Into<Expression> for BooleanLiteral {
    fn into(self) -> Expression {
        Expression::BooleanLiteral(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BinaryExpression {
    pub loc: Location,
    pub left: Box<Expression>,
    pub operator: BinaryOperator,
    pub right: Box<Expression>,
}

impl Into<Expression> for BinaryExpression {
    fn into(self) -> Expression {
        Expression::Binary(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    LAnd,
    LOr,
    EqEq,
    Neq,
    Grt,
    Geq,
    Less,
    Leq,
}

impl BinaryOperator {
    fn as_string(&self) -> String {
        match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Sub => "-",
            BinaryOperator::Mul => "*",
            BinaryOperator::Div => "/",
            BinaryOperator::Mod => "%",
            BinaryOperator::Pow => "**",

            BinaryOperator::EqEq => "==",
            BinaryOperator::Neq => "!=",
            BinaryOperator::Less => "<",
            BinaryOperator::Leq => "<=",
            BinaryOperator::Grt => ">",
            BinaryOperator::Geq => ">=",

            BinaryOperator::LAnd => "&&",
            BinaryOperator::LOr => "||",
        }
        .into()
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

impl From<String> for BinaryOperator {
    fn from(value: String) -> Self {
        match value.as_str() {
            "+" => BinaryOperator::Add,
            "-" => BinaryOperator::Sub,
            "*" => BinaryOperator::Mul,
            "/" => BinaryOperator::Div,
            "%" => BinaryOperator::Mod,
            "**" => BinaryOperator::Pow,

            "==" => BinaryOperator::EqEq,
            "!=" => BinaryOperator::Neq,
            "<" => BinaryOperator::Less,
            "<=" => BinaryOperator::Leq,
            ">" => BinaryOperator::Grt,
            ">=" => BinaryOperator::Geq,

            "&&" => BinaryOperator::LAnd,
            "||" => BinaryOperator::LOr,

            _ => panic!("Invalid operator"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockExpression {
    pub loc: Location,
    pub statements: Vec<Statement>,
}

impl Into<Expression> for BlockExpression {
    fn into(self) -> Expression {
        Expression::Block(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallExpression {
    pub loc: Location,
    pub callee: Box<Expression>,
    pub args: Vec<CallArgument>,
}

impl Into<Expression> for CallExpression {
    fn into(self) -> Expression {
        Expression::Call(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CallArgument {
    Expression(Expression),
    Callback(Callback),
}
impl From<Expression> for CallArgument {
    fn from(value: Expression) -> Self {
        Self::Expression(value)
    }
}
impl From<Callback> for CallArgument {
    fn from(value: Callback) -> Self {
        Self::Callback(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Callback {
    pub loc: Location,
    pub params: Vec<CallbackParam>,
    pub body: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CallbackParam {
    Identifier(Identifier),
    Param(FunctionParam),
}

impl From<Identifier> for CallbackParam {
    fn from(value: Identifier) -> Self {
        Self::Identifier(value)
    }
}
impl From<FunctionParam> for CallbackParam {
    fn from(value: FunctionParam) -> Self {
        Self::Param(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemberExpression {
    pub loc: Location,
    pub object: Box<Expression>,
    pub prop: Option<MemberProp>,
}

impl MemberExpression {
    pub fn root_expression(&self) -> Expression {
        match self.object.as_ref() {
            Expression::Member(expr) => expr.root_expression(),
            expr => expr.clone(),
        }
    }
}

impl Into<Expression> for MemberExpression {
    fn into(self) -> Expression {
        Expression::Member(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemberProp {
    FieldName(Identifier),
    Index(IntLiteral),
}
impl MemberProp {
    pub fn loc(&self) -> Location {
        match self {
            Self::FieldName(i) => i.loc,
            Self::Index(n) => n.loc,
        }
    }
}
impl From<Identifier> for MemberProp {
    fn from(value: Identifier) -> Self {
        Self::FieldName(value)
    }
}
impl From<IntLiteral> for MemberProp {
    fn from(value: IntLiteral) -> Self {
        Self::Index(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TupleExpression {
    pub loc: Location,
    pub elements: Vec<Expression>,
}

impl Into<Expression> for TupleExpression {
    fn into(self) -> Expression {
        Expression::Tuple(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnaryExpression {
    pub loc: Location,
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
}

impl Into<Expression> for UnaryExpression {
    fn into(self) -> Expression {
        Expression::Unary(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    Star,      // *
    Ampersand, // &
    Dollar,    // $
    At,        // @
    Minus,     // -
    Bang,      // !
}

impl From<String> for UnaryOperator {
    fn from(value: String) -> Self {
        match value.as_str() {
            "*" => Self::Star,
            "&" => Self::Ampersand,
            "$" => Self::Dollar,
            "@" => Self::At,
            "-" => Self::Minus,
            "!" => Self::Bang,
            _ => panic!("Unknown unary operator: {}", value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionExpression {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<Type>,
    pub body: BlockExpression,
}

impl Into<Expression> for FunctionExpression {
    fn into(self) -> Expression {
        Expression::Function(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionParam {
    pub loc: Location,
    pub name: Identifier,
    pub type_annotation: Option<Type>,
}
