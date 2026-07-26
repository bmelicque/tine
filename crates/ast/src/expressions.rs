use std::fmt;

use ordered_float::OrderedFloat;
use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::{
    nodes::{ast_enum, operator_enum},
    walk::PushNodes,
    ElementExpression, VariantConstructor,
};

use super::{constructor_literals::ConstructorLiteral, types::Type, Loop, Pattern, Statement};

ast_enum!(Expression {
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
    Intrinsic(IntrinsicCall),
    IfDecl(IfPatExpression),
    Invalid(InvalidExpression),
    Loop(Loop),
    Match(MatchExpression),
    Member(MemberExpression),
    FloatLiteral(FloatLiteral),
    StringLiteral(StringLiteral),
    Tuple(TupleExpression),
    TypeMatch(TypeMatch),
    Unary(UnaryExpression),
});
impl From<Expression> for Option<Box<Expression>> {
    fn from(value: Expression) -> Self {
        Some(Box::new(value.into()))
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ArrayExpression {
    pub elements: Vec<Expression>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub text: String,
}
impl Identifier {
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    pub fn new(text: String, loc: Location) -> Self {
        Self { loc, text }
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IfPatExpression {
    pub pattern: Option<Pattern>,
    #[child]
    pub scrutinee: Option<Box<Expression>>,
    #[child]
    pub consequent: Option<BlockExpression>,
    #[child]
    pub alternate: Option<Box<Alternate>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IfExpression {
    #[child]
    pub condition: Option<Box<Expression>>,
    #[child]
    pub consequent: Option<BlockExpression>,
    #[child]
    pub alternate: Option<Box<Alternate>>,
}

ast_enum!(
    Alternate {
        Block(BlockExpression),
        If(IfExpression),
        IfDecl(IfPatExpression),
    }
);
impl Into<Expression> for Alternate {
    fn into(self) -> Expression {
        match self {
            Alternate::Block(b) => Expression::Block(b),
            Alternate::If(i) => Expression::If(i),
            Alternate::IfDecl(i) => Expression::IfDecl(i),
        }
    }
}
impl<'a> PushNodes<'a> for Alternate {
    fn push_nodes(&'a self, stack: &mut Vec<crate::Node<'a>>) {
        match self {
            Alternate::Block(b) => b.push_children(stack),
            Alternate::If(i) => i.push_children(stack),
            Alternate::IfDecl(i) => i.push_children(stack),
        };
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IntLiteral {
    pub value: i64,
}
impl IntLiteral {
    pub fn new(value: i64, loc: Location) -> Self {
        Self { loc, value }
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IntrinsicCall {
    pub name: Identifier,
    pub args: Vec<Expression>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InvalidExpression {}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MatchExpression {
    #[child]
    pub scrutinee: Option<Box<Expression>>,
    #[child]
    pub arms: Option<Vec<MatchArm>>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: Option<Box<Pattern>>,
    pub expression: Option<Box<Expression>>,
}
impl<'a> PushNodes<'a> for MatchArm {
    fn push_nodes(&'a self, stack: &mut Vec<crate::Node<'a>>) {
        self.expression.push_nodes(stack);
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StringLiteral {
    pub text: String,
}
impl StringLiteral {
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FloatLiteral {
    pub value: OrderedFloat<f64>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BooleanLiteral {
    pub value: bool,
}
impl BooleanLiteral {
    pub fn new(value: bool, loc: Location) -> Self {
        Self { value, loc }
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BinaryExpression {
    #[child]
    pub left: Option<Box<Expression>>,
    pub operator: BinaryOperator,
    #[child]
    pub right: Option<Box<Expression>>,
}
impl BinaryExpression {
    pub fn and(left: Expression, right: Expression, loc: Location) -> Self {
        Self {
            left: left.into(),
            operator: BinaryOperator::LAnd,
            right: right.into(),
            loc,
        }
    }
}

operator_enum!(BinaryOperator {
    Add => "+",
    Sub => "-",
    Mul => "*",
    Div => "/",
    Mod => "%",
    Pow => "**",

    EqEq => "==",
    Neq => "!=",
    Less => "<",
    Leq => "<=",
    Grt => ">",
    Geq => ">=",

    LAnd => "&&",
    LOr => "||",
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BlockExpression {
    #[child]
    pub statements: Vec<Statement>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CallExpression {
    #[child]
    pub callee: Option<Box<Expression>>,
    pub type_args: Option<Vec<Type>>,
    #[child]
    pub args: Vec<CallArgument>,
}

ast_enum!(CallArgument {
    Expression(Expression),
    Callback(Callback),
});
impl<'a> PushNodes<'a> for CallArgument {
    fn push_nodes(&'a self, stack: &mut Vec<crate::Node<'a>>) {
        match self {
            CallArgument::Expression(expr) => expr.push_nodes(stack),
            CallArgument::Callback(c) => c.push_children(stack),
        }
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Callback {
    pub params: Vec<CallbackParam>,
    #[child]
    pub body: Option<Box<Expression>>,
}
impl<'a> PushNodes<'a> for Callback {
    fn push_nodes(&'a self, stack: &mut Vec<crate::Node<'a>>) {
        self.body.push_nodes(stack);
    }
}

ast_enum!(CallbackParam {
    Identifier(Identifier),
    Param(FunctionParam),
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MemberExpression {
    #[child]
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

    pub fn valid<O, M>(object: O, member: M, loc: Location) -> Self
    where
        O: Into<Expression>,
        M: Into<MemberProp>,
    {
        Self {
            object: Some(Box::new(object.into())),
            prop: Some(member.into()),
            loc,
        }
    }
}

ast_enum!(MemberProp {
    FieldName(Identifier),
    Index(IntLiteral),
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TupleExpression {
    #[child]
    pub elements: Vec<Expression>,
}

/// Internals use only.
/// Match a value against an enum variant.
#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TypeMatch {
    #[child]
    pub expression: Option<Box<Expression>>,
    pub constructor: VariantConstructor,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UnaryExpression {
    pub operator: UnaryOperator,
    #[child]
    pub operand: Option<Box<Expression>>,
}

operator_enum!(UnaryOperator {
    Star => "*",
    Minus => "-",
    Bang => "!",
    Mut => "mut",
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FunctionExpression {
    pub name: Option<Identifier>,
    pub type_params: Option<Vec<Identifier>>,
    pub params: Option<FunctionParams>,
    pub return_type: Option<Type>,
    #[child]
    pub body: Option<BlockExpression>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FunctionParams {
    pub params: Vec<FunctionParam>,
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FunctionParam {
    pub name: Option<Identifier>,
    pub type_annotation: Option<Type>,
}
