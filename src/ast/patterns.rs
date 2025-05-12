use super::NamedType;

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Identifier(IdentifierPattern),
    StructPattern(StructPattern),
    Tuple(TuplePattern),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentifierPattern {
    pub span: pest::Span<'static>,
}

impl Into<Pattern> for IdentifierPattern {
    fn into(self) -> Pattern {
        Pattern::Identifier(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructPattern {
    pub span: pest::Span<'static>,
    pub ty: Box<NamedType>,
    pub fields: Vec<StructPatternField>,
}

impl Into<Pattern> for StructPattern {
    fn into(self) -> Pattern {
        Pattern::StructPattern(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructPatternField {
    pub span: pest::Span<'static>,
    pub identifier: String,
    pub pattern: Option<Pattern>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TuplePattern {
    pub span: pest::Span<'static>,
    pub elements: Vec<Pattern>,
}

impl Into<Pattern> for TuplePattern {
    fn into(self) -> Pattern {
        Pattern::Tuple(self)
    }
}
