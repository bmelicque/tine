use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::{nodes::ast_enum, FloatLiteral, Identifier, IntLiteral, PathExpression};

use super::{BooleanLiteral, StringLiteral};

ast_enum!(Pattern {
    Invalid(InvalidPattern),

    Literal(LiteralPattern),
    Identifier(IdentifierPattern),
    Path(PathExpression),
    Call(CallPattern),
    Struct(StructPattern),
    Tuple(TuplePattern),
});
impl Pattern {
    pub fn is_identifier(&self) -> bool {
        match self {
            Pattern::Identifier(_) => true,
            _ => false,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Pattern::Invalid { .. } => false,
            _ => true,
        }
    }
}
impl From<Identifier> for Pattern {
    fn from(value: Identifier) -> Self {
        Self::Identifier(value.into())
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPattern {}

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierPattern {
    pub mutable: bool,
    pub identifier: Identifier,
}
impl From<Identifier> for IdentifierPattern {
    fn from(value: Identifier) -> Self {
        Self {
            loc: value.loc,
            mutable: false,
            identifier: value,
        }
    }
}

ast_enum!(LiteralPattern {
    Boolean(BooleanLiteral),
    Float(FloatLiteral),
    Integer(IntLiteral),
    String(StringLiteral),
});

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallPattern {
    pub path: PathExpression,
    pub args: Vec<Pattern>,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructPattern {
    pub path: PathExpression,
    pub fields: Vec<StructPatternField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructPatternField {
    pub loc: Location,
    pub identifier: Option<Identifier>,
    pub pattern: Option<Pattern>,
}

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuplePattern {
    pub elements: Vec<Pattern>,
}

impl From<Vec<Pattern>> for TuplePattern {
    fn from(elements: Vec<Pattern>) -> Self {
        let loc = Location::merge(
            elements.first().unwrap().loc(),
            elements.last().unwrap().loc(),
        );
        Self { loc, elements }
    }
}
