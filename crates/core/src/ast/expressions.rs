use std::fmt;

use enum_from_derive::EnumFrom;
use ordered_float::OrderedFloat;

use crate::{ast::ElementExpression, Location};

use super::{constructor_literals::ConstructorLiteral, types::Type, Loop, Pattern, Statement};

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum Expression {
    Array(ArrayExpression),
    Binary(BinaryExpression),
    BooleanLiteral(BooleanLiteral),
    Block(BlockExpression),
    Call(CallExpression),
    ConstructorLiteral(ConstructorLiteral),
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
            Self::ConstructorLiteral(e) => e.loc,
            Self::Element(e) => e.loc(),
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayExpression {
    pub loc: Location,
    pub elements: Vec<Expression>,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IfPatExpression {
    pub loc: Location,
    pub pattern: Option<Pattern>,
    pub scrutinee: Option<Box<Expression>>,
    pub consequent: Option<BlockExpression>,
    pub alternate: Option<Box<Alternate>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IfExpression {
    pub loc: Location,
    pub condition: Option<Box<Expression>>,
    pub consequent: Option<BlockExpression>,
    pub alternate: Option<Box<Alternate>>,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvalidExpression {
    pub loc: Location,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchExpression {
    pub loc: Location,
    pub scrutinee: Option<Box<Expression>>,
    pub arms: Option<Vec<MatchArm>>,
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
        self.text.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FloatLiteral {
    pub loc: Location,
    pub value: OrderedFloat<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BooleanLiteral {
    pub loc: Location,
    pub value: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BinaryExpression {
    pub loc: Location,
    pub left: Option<Box<Expression>>,
    pub operator: BinaryOperator,
    pub right: Option<Box<Expression>>,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallExpression {
    pub loc: Location,
    pub callee: Option<Box<Expression>>,
    pub type_args: Option<Vec<Type>>,
    pub args: Vec<CallArgument>,
}

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum CallArgument {
    Expression(Expression),
    Callback(Callback),
}

impl CallArgument {
    pub fn as_expression(&self) -> Option<&Expression> {
        match self {
            CallArgument::Expression(expr) => Some(expr),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Callback {
    pub loc: Location,
    pub params: Vec<CallbackParam>,
    pub body: Option<Box<Expression>>,
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
    pub object: Option<Box<Expression>>,
    pub prop: Option<MemberProp>,
}

impl MemberExpression {
    pub fn root_expression(&self) -> Option<Expression> {
        let Some(object) = self.object.as_ref() else {
            return None;
        };

        match object.as_ref() {
            Expression::Member(expr) => expr.root_expression(),
            expr => Some(expr.clone()),
        }
    }
}

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TupleExpression {
    pub loc: Location,
    pub elements: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnaryExpression {
    pub loc: Location,
    pub operator: UnaryOperator,
    pub operand: Option<Box<Expression>>,
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
    pub type_params: Option<Vec<Identifier>>,
    pub params: Option<FunctionParams>,
    pub return_type: Option<Type>,
    pub body: Option<BlockExpression>,
}

impl Default for FunctionExpression {
    fn default() -> Self {
        Self {
            loc: Location::dummy(),
            name: None,
            type_params: None,
            params: None,
            return_type: None,
            body: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionParams {
    pub loc: Location,
    pub params: Vec<FunctionParam>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionParam {
    pub loc: Location,
    pub name: Identifier,
    pub type_annotation: Option<Type>,
}
