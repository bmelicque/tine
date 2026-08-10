use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::{nodes::ast_enum, FloatLiteral, Identifier, IntLiteral, PathExpression};

use super::{BooleanLiteral, StringLiteral};

ast_enum!(Pattern {
    Invalid(InvalidPattern),

    Identifier(Identifier),
    MutIdentifier(MutIdentifierPattern),
    Literal(LiteralPattern),
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

    pub fn is_refutable(&self) -> bool {
        match self {
            Pattern::Invalid { .. } => false,
            Pattern::Identifier(_) => false,
            Pattern::Literal(_) => true,
            Pattern::MutIdentifier(_) => false,
            Pattern::Struct(s) => s.is_refutable(),
            Pattern::Tuple(p) => p.is_refutable(),
        }
    }

    pub fn list_identifiers(&self) -> Vec<&Identifier> {
        use Pattern::*;
        match self {
            Invalid { .. } => vec![],
            Identifier(p) => vec![p],
            MutIdentifier(p) => vec![&p.identifier],
            Literal(_) => vec![],
            Struct(p) => p.list_identifiers(),
            Tuple(t) => t.list_identifiers(),
        }
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPattern {}

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutIdentifierPattern {
    pub identifier: Identifier,
}
impl Into<Identifier> for MutIdentifierPattern {
    fn into(self) -> Identifier {
        self.identifier
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
pub struct StructPattern {
    pub path: PathExpression,
    pub fields: Vec<StructPatternField>,
}
impl StructPattern {
    pub fn is_refutable(&self) -> bool {
        self.fields
            .iter()
            .filter_map(|f| f.pattern.as_ref())
            .any(|pattern| pattern.is_refutable())
    }

    fn list_identifiers(&self) -> Vec<&Identifier> {
        self.fields
            .iter()
            .flat_map(|field| field.list_identifiers())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructPatternField {
    pub loc: Location,
    pub identifier: Option<FieldPatternIdentifier>,
    pub pattern: Option<Pattern>,
}
impl StructPatternField {
    fn list_identifiers(&self) -> Vec<&Identifier> {
        if let Some(pattern) = &self.pattern {
            pattern.list_identifiers()
        } else if let Some(pattern) = &self.identifier {
            vec![pattern.identifier()]
        } else {
            vec![]
        }
    }
}

ast_enum!(FieldPatternIdentifier {
    Const(Identifier),
    Mut(MutIdentifierPattern),
});

impl FieldPatternIdentifier {
    pub fn identifier(&self) -> &Identifier {
        use FieldPatternIdentifier::*;
        match self {
            Const(i) => i,
            Mut(i) => &i.identifier,
        }
    }

    pub fn is_mutable(&self) -> bool {
        match self {
            FieldPatternIdentifier::Mut(_) => true,
            _ => false,
        }
    }
}
impl Into<Identifier> for FieldPatternIdentifier {
    fn into(self) -> Identifier {
        match self {
            FieldPatternIdentifier::Const(i) => i,
            FieldPatternIdentifier::Mut(i) => i.identifier,
        }
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuplePattern {
    pub path: Option<PathExpression>,
    pub elements: Vec<Pattern>,
}
impl TuplePattern {
    pub fn is_refutable(&self) -> bool {
        if self.path.is_some() {
            return true;
        }
        self.elements.iter().any(|e| e.is_refutable())
    }

    pub fn list_identifiers(&self) -> Vec<&Identifier> {
        self.elements
            .iter()
            .map(|pattern| pattern.list_identifiers())
            .flatten()
            .collect()
    }
}
impl From<Vec<Pattern>> for TuplePattern {
    fn from(elements: Vec<Pattern>) -> Self {
        let loc = Location::merge(
            elements.first().unwrap().loc(),
            elements.last().unwrap().loc(),
        );
        Self {
            loc,
            path: None,
            elements,
        }
    }
}
