use enum_from_derive::EnumFrom;

use crate::{
    ast::{Constructor, FloatLiteral, Identifier, IntLiteral},
    Location,
};

use super::{BooleanLiteral, NamedType, StringLiteral};

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Invalid(InvalidPattern),

    Identifier(IdentifierPattern),
    Constructor(ConstructorPattern),
    Literal(LiteralPattern),
    Tuple(TuplePattern),
}

impl Pattern {
    pub fn loc(&self) -> Location {
        match self {
            Pattern::Invalid(p) => p.loc,
            Pattern::Identifier(p) => p.0.loc,
            Pattern::Constructor(p) => p.loc,
            Pattern::Literal(l) => l.loc(),
            Pattern::Tuple(p) => p.loc,
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
            Pattern::Constructor(pattern) => pattern.is_refutable(),
            Pattern::Tuple(p) => p.is_refutable(),
        }
    }

    pub fn list_identifiers(&self) -> Vec<&IdentifierPattern> {
        match self {
            Pattern::Invalid { .. } => vec![],
            Pattern::Identifier(p) => vec![p],
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InvalidPattern {
    pub loc: Location,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentifierPattern(pub Identifier);

impl IdentifierPattern {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn loc(&self) -> Location {
        self.0.loc
    }
}

impl Into<Identifier> for IdentifierPattern {
    fn into(self) -> Identifier {
        self.0
    }
}

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum LiteralPattern {
    Boolean(BooleanLiteral),
    Float(FloatLiteral),
    Integer(IntLiteral),
    String(StringLiteral),
}

impl LiteralPattern {
    pub fn loc(&self) -> Location {
        match self {
            LiteralPattern::Boolean(b) => b.loc,
            LiteralPattern::Float(f) => f.loc,
            LiteralPattern::Integer(i) => i.loc,
            LiteralPattern::String(s) => s.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstructorPattern {
    pub loc: Location,
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

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum ConstructorPatternBody {
    Tuple(TuplePattern),
    Struct(StructPatternBody),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructPatternBody {
    pub loc: Location,
    pub fields: Vec<StructPatternField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructPattern {
    pub loc: Location,
    pub ty: Box<NamedType>,
    pub fields: Vec<StructPatternField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructPatternField {
    pub loc: Location,
    pub identifier: Option<Identifier>,
    pub pattern: Option<Pattern>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TuplePattern {
    pub loc: Location,
    pub elements: Vec<Pattern>,
}

impl TuplePattern {
    pub fn is_refutable(&self) -> bool {
        self.elements.iter().any(|e| e.is_refutable())
    }

    pub fn list_identifiers(&self) -> Vec<&IdentifierPattern> {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantPattern {
    pub loc: Location,
    pub ty: Box<NamedType>,
    pub name: String,
    pub body: Option<VariantPatternBody>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariantPatternBody {
    Struct(Vec<StructPatternField>),
    Tuple(TuplePattern),
}

impl From<Vec<StructPatternField>> for VariantPatternBody {
    fn from(value: Vec<StructPatternField>) -> Self {
        VariantPatternBody::Struct(value)
    }
}
impl From<TuplePattern> for VariantPatternBody {
    fn from(value: TuplePattern) -> Self {
        VariantPatternBody::Tuple(value)
    }
}
