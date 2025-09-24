use crate::parser::utils::merge_span;

use super::{BooleanLiteral, NamedType, NumberLiteral, StringLiteral};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Identifier(IdentifierPattern),
    Literal(LiteralPattern),
    Struct(StructPattern),
    Tuple(TuplePattern),
    Variant(VariantPattern),
}

impl Pattern {
    pub fn as_span(&self) -> pest::Span<'static> {
        match self {
            Pattern::Identifier(p) => p.span,
            Pattern::Literal(l) => l.as_span(),
            Pattern::Struct(p) => p.span,
            Pattern::Tuple(p) => p.span,
            Pattern::Variant(p) => p.span,
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
pub struct IdentifierPattern {
    pub span: pest::Span<'static>,
}

impl Into<Pattern> for IdentifierPattern {
    fn into(self) -> Pattern {
        Pattern::Identifier(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LiteralPattern {
    Boolean(BooleanLiteral),
    Number(NumberLiteral),
    String(StringLiteral),
}

impl LiteralPattern {
    pub fn as_span(&self) -> pest::Span<'static> {
        match self {
            LiteralPattern::Boolean(b) => b.span,
            LiteralPattern::Number(n) => n.span,
            LiteralPattern::String(s) => s.span,
        }
    }
}

impl From<BooleanLiteral> for LiteralPattern {
    fn from(value: BooleanLiteral) -> Self {
        Self::Boolean(value)
    }
}
impl From<NumberLiteral> for LiteralPattern {
    fn from(value: NumberLiteral) -> Self {
        Self::Number(value)
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
    pub span: pest::Span<'static>,
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
    pub span: pest::Span<'static>,
    pub identifier: String,
    pub pattern: Option<Pattern>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TuplePattern {
    pub span: pest::Span<'static>,
    pub elements: Vec<Pattern>,
}

impl Into<Pattern> for TuplePattern {
    fn into(self) -> Pattern {
        Pattern::Tuple(self)
    }
}
impl From<Vec<Pattern>> for TuplePattern {
    fn from(elements: Vec<Pattern>) -> Self {
        let span = merge_span(
            elements.first().unwrap().as_span(),
            elements.last().unwrap().as_span(),
        );
        Self { span, elements }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantPattern {
    pub span: pest::Span<'static>,
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
