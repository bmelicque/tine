use super::{BooleanLiteral, Expression, NamedType, NumberLiteral, StringLiteral};

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
    Literal(LiteralPattern),
    StructPattern(StructPattern),
    Tuple(TuplePattern),
}

impl Pattern {
    pub fn as_span(&self) -> pest::Span<'static> {
        match self {
            Pattern::Identifier(p) => p.span,
            Pattern::Literal(l) => l.as_span(),
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

    pub fn is_refutable(&self) -> bool {
        match self {
            Pattern::Identifier(_) => false,
            Pattern::Literal(_) => true,
            Pattern::StructPattern(s) => s
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
