use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::{nodes::ast_enum, Constructor, FloatLiteral, Identifier, IntLiteral};

use super::{BooleanLiteral, NamedType, StringLiteral};

ast_enum!(Pattern {
    Invalid(InvalidPattern),

    Identifier(Identifier),
    MutIdentifier(MutIdentifierPattern),
    Constructor(ConstructorPattern),
    Literal(LiteralPattern),
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
            Pattern::Constructor(pattern) => pattern.is_refutable(),
            Pattern::Tuple(p) => p.is_refutable(),
        }
    }

    pub fn list_identifiers(&self) -> Vec<&Identifier> {
        match self {
            Pattern::Invalid { .. } => vec![],
            Pattern::Identifier(p) => vec![p],
            Pattern::MutIdentifier(p) => vec![&p.identifier],
            Pattern::Literal(_) => vec![],
            Pattern::Constructor(p) => {
                let Some(body) = &p.body else { return vec![] };
                match body {
                    ConstructorPatternBody::Struct(body) => body
                        .fields
                        .iter()
                        .filter_map(|field| {
                            if let Some(pattern) = &field.pattern {
                                Some(pattern.list_identifiers())
                            } else {
                                None
                            }
                        })
                        .flatten()
                        .collect(),
                    ConstructorPatternBody::Tuple(t) => t.list_identifiers(),
                }
            }
            Pattern::Tuple(t) => t.list_identifiers(),
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
pub struct ConstructorPattern {
    pub qualifiers: Vec<Identifier>,
    pub constructor: Constructor,
    pub body: Option<ConstructorPatternBody>,
}
impl ConstructorPattern {
    pub fn is_refutable(&self) -> bool {
        if let Constructor::Variant(_) = &self.constructor {
            return true;
        }

        match &self.body {
            Some(ConstructorPatternBody::Tuple(t)) => t.is_refutable(),
            Some(ConstructorPatternBody::Struct(s)) => s.fields.iter().any(|field| {
                let Some(ref pattern) = field.pattern else {
                    return false;
                };
                pattern.is_refutable()
            }),
            None => false,
        }
    }
}

ast_enum!(ConstructorPatternBody {
    Tuple(TuplePattern),
    Struct(StructPatternBody),
});

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructPatternBody {
    pub fields: Vec<StructPatternField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructPattern {
    pub ty: Box<NamedType>,
    pub fields: Vec<StructPatternField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructPatternField {
    pub loc: Location,
    pub identifier: Option<FieldPatternIdentifier>,
    pub pattern: Option<Pattern>,
}

ast_enum!(FieldPatternIdentifier {
    Const(Identifier),
    Mut(MutIdentifierPattern),
});

impl FieldPatternIdentifier {
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
    pub elements: Vec<Pattern>,
}
impl TuplePattern {
    pub fn is_refutable(&self) -> bool {
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
        Self { loc, elements }
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantPattern {
    pub ty: Box<NamedType>,
    pub name: String,
    pub body: Option<VariantPatternBody>,
}

ast_enum!(VariantPatternBody {
    Struct(StructPatternBody),
    Tuple(TuplePattern),
});
