use crate::{
    ast::{FloatLiteral, Identifier, IntLiteral},
    Location,
};

use super::{BooleanLiteral, NamedType, StringLiteral};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Identifier(IdentifierPattern),
    Literal(LiteralPattern),
    Struct(StructPattern),
    Tuple(TuplePattern),
    Variant(VariantPattern),
}

impl Pattern {
    pub fn loc(&self) -> Location {
        match self {
            Pattern::Identifier(p) => p.0.loc,
            Pattern::Literal(l) => l.loc(),
            Pattern::Struct(p) => p.loc,
            Pattern::Tuple(p) => p.loc,
            Pattern::Variant(p) => p.loc,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match self {
            Pattern::Identifier(_) => true,
            _ => false,
        }
    }

    pub fn is_refutable(&self) -> bool {
        match self {
            Pattern::Identifier(_) => false,
            Pattern::Literal(_) => true,
            Pattern::Struct(s) => s
                .fields
                .iter()
                .find(|field| {
                    let Some(ref pattern) = field.pattern else {
                        return false;
                    };
                    pattern.is_refutable()
                })
                .is_some(),
            Pattern::Tuple(p) => p
                .elements
                .iter()
                .find(|pattern| pattern.is_refutable())
                .is_some(),
            Pattern::Variant(_) => true,
        }
    }

    pub fn list_identifiers(&self) -> Vec<&IdentifierPattern> {
        match self {
            Pattern::Identifier(p) => vec![p],
            Pattern::Literal(_) => vec![],
            Pattern::Struct(s) => s
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
            Pattern::Tuple(t) => t
                .elements
                .iter()
                .map(|pattern| pattern.list_identifiers())
                .flatten()
                .collect(),
            Pattern::Variant(v) => {
                let Some(body) = &v.body else { return vec![] };
                match body {
                    VariantPatternBody::Struct(fields) => fields
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
                    VariantPatternBody::Tuple(t) => t
                        .elements
                        .iter()
                        .map(|pattern| pattern.list_identifiers())
                        .flatten()
                        .collect(),
                }
            }
        }
    }
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

impl Into<Pattern> for IdentifierPattern {
    fn into(self) -> Pattern {
        Pattern::Identifier(self)
    }
}

impl Into<Identifier> for IdentifierPattern {
    fn into(self) -> Identifier {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl From<BooleanLiteral> for LiteralPattern {
    fn from(value: BooleanLiteral) -> Self {
        Self::Boolean(value)
    }
}
impl From<FloatLiteral> for LiteralPattern {
    fn from(value: FloatLiteral) -> Self {
        Self::Float(value)
    }
}
impl From<IntLiteral> for LiteralPattern {
    fn from(value: IntLiteral) -> Self {
        Self::Integer(value)
    }
}
impl From<StringLiteral> for LiteralPattern {
    fn from(value: StringLiteral) -> Self {
        Self::String(value)
    }
}
impl Into<Pattern> for LiteralPattern {
    fn into(self) -> Pattern {
        Pattern::Literal(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructPattern {
    pub loc: Location,
    pub ty: Box<NamedType>,
    pub fields: Vec<StructPatternField>,
}

impl Into<Pattern> for StructPattern {
    fn into(self) -> Pattern {
        Pattern::Struct(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructPatternField {
    pub loc: Location,
    pub identifier: Identifier,
    pub pattern: Option<Pattern>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TuplePattern {
    pub loc: Location,
    pub elements: Vec<Pattern>,
}

impl Into<Pattern> for TuplePattern {
    fn into(self) -> Pattern {
        Pattern::Tuple(self)
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

impl Into<Pattern> for VariantPattern {
    fn into(self) -> Pattern {
        Pattern::Variant(self)
    }
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
