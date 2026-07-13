use tine_common::locations::{Locatable, Location};

use crate::{
    nodes::{ast_enum, ast_struct},
    Constructor, FloatLiteral, Identifier, IntLiteral,
};

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
    pub fn as_identifier(&self) -> Option<&Identifier> {
        match self {
            Pattern::Identifier(i) => Some(i),
            _ => None,
        }
    }

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

ast_struct!(InvalidPattern {});

ast_struct!(MutIdentifierPattern {
    identifier: Identifier,
});
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

ast_struct!(ConstructorPattern {
    qualifiers: Vec<Identifier>,
    constructor: Constructor,
    body: Option<ConstructorPatternBody>,
});

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

ast_struct!(StructPatternBody {
   fields: Vec<StructPatternField>
});

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructPattern {
    pub loc: Location,
    pub ty: Box<NamedType>,
    pub fields: Vec<StructPatternField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

ast_struct!(TuplePattern {
    elements: Vec<Pattern>,
});
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

ast_struct!(VariantPattern {
        ty: Box<NamedType>,
        name: String,
        body: Option<VariantPatternBody>,
});

ast_enum!(VariantPatternBody {
    Struct(StructPatternBody),
    Tuple(TuplePattern),
});
