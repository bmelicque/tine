use super::{Expression, NamedType};

#[derive(Debug, Clone, PartialEq)]
pub enum PatternExpression {
    Pattern(Pattern),
    Expression(Expression),
}

impl From<Expression> for PatternExpression {
    fn from(value: Expression) -> Self {
        Self::Expression(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Identifier(IdentifierPattern),
    StructPattern(StructPattern),
    Tuple(TuplePattern),
}

impl Pattern {
    pub fn as_span(&self) -> pest::Span<'static> {
        match self {
            Pattern::Identifier(p) => p.span,
            Pattern::StructPattern(p) => p.span,
            Pattern::Tuple(p) => p.span,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match self {
            Pattern::Identifier(_) => true,
            _ => false,
        }
    }
}

impl Into<PatternExpression> for Pattern {
    fn into(self) -> PatternExpression {
        PatternExpression::Pattern(self)
    }
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
