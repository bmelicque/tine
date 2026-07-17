use std::fmt;

use ordered_float::OrderedFloat;
use tine_common::locations::{Locatable, Location};

use crate::{
    nodes::{ast_enum, ast_struct, operator_enum},
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

ast_struct!(ArrayExpression {
    elements: Vec<Expression>,
});

ast_struct!(
    #[derive(Default)]
    Identifier { text: String }
);
impl Identifier {
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }
}

ast_struct!(IfPatExpression {
    pattern: Option<Pattern>,
    scrutinee: Option<Box<Expression>>,
    consequent: Option<BlockExpression>,
    alternate: Option<Box<Alternate>>,
});

ast_struct!(IfExpression {
    condition: Option<Box<Expression>>,
    consequent: Option<BlockExpression>,
    alternate: Option<Box<Alternate>>,
});

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

ast_struct!(IntLiteral { value: i64 });

ast_struct!(IntrinsicCall {
    name: Identifier,
    args: Vec<Expression>
});

ast_struct!(InvalidExpression {});

ast_struct!(MatchExpression {
    scrutinee: Option<Box<Expression>>,
    arms: Option<Vec<MatchArm>>,
});

ast_struct!(MatchArm {
    pattern: Option<Box<Pattern>>,
    expression: Option<Box<Expression>>,
});

ast_struct!(StringLiteral { text: String });
impl StringLiteral {
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }
}

ast_struct!(FloatLiteral { value: OrderedFloat<f64> });

ast_struct!(BooleanLiteral { value: bool });

ast_struct!(BinaryExpression {
    left: Option<Box<Expression>>,
    operator: BinaryOperator,
    right: Option<Box<Expression>>,
});

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

ast_struct!(
    #[derive(Default)]
    BlockExpression {
        statements: Vec<Statement>,
    }
);

ast_struct!(CallExpression {
    callee: Option<Box<Expression>>,
    type_args: Option<Vec<Type>>,
    args: Vec<CallArgument>,
});

ast_enum!(CallArgument {
    Expression(Expression),
    Callback(Callback),
});
impl CallArgument {
    pub fn as_expression(&self) -> Option<&Expression> {
        match self {
            CallArgument::Expression(expr) => Some(expr),
            _ => None,
        }
    }
}

ast_struct!(Callback {
    params: Vec<CallbackParam>,
    body: Option<Box<Expression>>,
});

ast_enum!(CallbackParam {
    Identifier(Identifier),
    Param(FunctionParam),
});

ast_struct!(MemberExpression {
    object: Option<Box<Expression>>,
    prop: Option<MemberProp>,
});
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

ast_enum!(MemberProp {
    FieldName(Identifier),
    Index(IntLiteral),
});

ast_struct!(TupleExpression {
    elements: Vec<Expression>,
});

ast_struct!(
    /// Internals use only.
    /// Match a value against an enum variant.
    TypeMatch {
        expression: Option<Box<Expression>>,
        constructor: VariantConstructor,
    }
);

ast_struct!(UnaryExpression {
    operator: UnaryOperator,
    operand: Option<Box<Expression>>,
});

operator_enum!(UnaryOperator {
    Star => "*",
    Minus => "-",
    Bang => "!",
    Mut => "mut",
});

ast_struct!(
    #[derive(Default)]
    FunctionExpression {
        name: Option<Identifier>,
        type_params: Option<Vec<Identifier>>,
        params: Option<FunctionParams>,
        return_type: Option<Type>,
        body: Option<BlockExpression>,
    }
);

ast_struct!(FunctionParams {
    params: Vec<FunctionParam>,
});

ast_struct!(FunctionParam {
    name: Option<Identifier>,
    type_annotation: Option<Type>,
});
