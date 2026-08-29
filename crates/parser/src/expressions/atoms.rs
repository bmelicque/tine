use tine_ast::*;

use crate::{Parser, Token};

impl Parser<'_> {
    pub fn parse_atom(&mut self) -> Option<Expression> {
        let Some(ranged_token) = self.tokens.peek() else {
            return None;
        };
        let (Ok(token), _) = ranged_token else {
            // TODO: handle error with InvalidExpression
            panic!()
        };
        match token {
            Token::Bool(_) => Some(self.parse_bool().into()),
            Token::Int(_) => Some(self.parse_int().into()),
            Token::Float(_) => Some(self.parse_float().into()),
            Token::String(_) => Some(self.parse_string().into()),
            Token::Ident(_) => Some(self.parse_path_expression().into()),
            Token::LParen => Some(self.parse_tuple().into()),
            _ => None,
        }
    }

    pub(super) fn parse_int(&mut self) -> IntLiteral {
        let Some((Ok(Token::Int(value)), span)) = self.tokens.next() else {
            panic!()
        };

        IntLiteral {
            loc: self.localize(span),
            value,
        }
    }

    fn parse_float(&mut self) -> FloatLiteral {
        let Some((Ok(Token::Float(value)), span)) = self.tokens.next() else {
            panic!()
        };

        FloatLiteral {
            loc: self.localize(span),
            value: value.value,
        }
    }

    fn parse_bool(&mut self) -> BooleanLiteral {
        let Some((Ok(Token::Bool(value)), span)) = self.tokens.next() else {
            panic!()
        };

        BooleanLiteral {
            loc: self.localize(span),
            value,
        }
    }

    fn parse_string(&mut self) -> StringLiteral {
        let Some((Ok(Token::String(text)), span)) = self.tokens.next() else {
            panic!()
        };

        StringLiteral {
            loc: self.localize(span),
            text,
        }
    }

    pub fn parse_identifier(&mut self) -> Identifier {
        let Some((Ok(Token::Ident(text)), span)) = self.tokens.next() else {
            panic!()
        };

        Identifier {
            loc: self.localize(span),
            text,
        }
    }
}

#[cfg(test)]
mod tests {
    use tine_common::locations::{Location, Span};

    use crate::test_utils::{test_expression, ExpressionTest};

    use super::*;

    #[test]
    fn test_parse_int() {
        test_expression(ExpressionTest {
            input: "42",
            expected: Expression::IntLiteral(IntLiteral {
                loc: Location::new(0, Span::new(0, 2)),
                value: 42,
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn parse_int_with_underscore() {
        test_expression(ExpressionTest {
            input: "42_000",
            expected: Expression::IntLiteral(IntLiteral {
                loc: Location::new(0, Span::new(0, 6)),
                value: 42000,
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_float() {
        test_expression(ExpressionTest {
            input: "3.14",
            expected: Expression::FloatLiteral(FloatLiteral {
                loc: Location::new(0, Span::new(0, 4)),
                value: ordered_float::OrderedFloat(3.14),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_float_no_decimals() {
        test_expression(ExpressionTest {
            input: "3.",
            expected: Expression::FloatLiteral(FloatLiteral {
                loc: Location::new(0, Span::new(0, 2)),
                value: ordered_float::OrderedFloat(3.),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_float_with_underscore() {
        test_expression(ExpressionTest {
            input: "3.14_000",
            expected: Expression::FloatLiteral(FloatLiteral {
                loc: Location::new(0, Span::new(0, 8)),
                value: ordered_float::OrderedFloat(3.14),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_float_only_decimals() {
        test_expression(ExpressionTest {
            input: ".14",
            expected: Expression::FloatLiteral(FloatLiteral {
                loc: Location::new(0, Span::new(0, 3)),
                value: ordered_float::OrderedFloat(0.14),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn parse_bool_true() {
        test_expression(ExpressionTest {
            input: "true",
            expected: Expression::BooleanLiteral(BooleanLiteral {
                loc: Location::new(0, Span::new(0, 4)),
                value: true,
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn parse_bool_false() {
        test_expression(ExpressionTest {
            input: "false",
            expected: Expression::BooleanLiteral(BooleanLiteral {
                loc: Location::new(0, Span::new(0, 5)),
                value: false,
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn parse_string() {
        test_expression(ExpressionTest {
            input: "\"hello world\"",
            expected: Expression::StringLiteral(StringLiteral {
                loc: Location::new(0, Span::new(0, 13)),
                text: "hello world".to_string(),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn parse_string_with_escaped_quote() {
        test_expression(ExpressionTest {
            input: "\"hello \\\"world\\\"\"",
            expected: Expression::StringLiteral(StringLiteral {
                loc: Location::new(0, Span::new(0, 17)),
                text: "hello \"world\"".to_string(),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_identifier() {
        test_expression(ExpressionTest {
            input: "x",
            expected: Expression::Path(PathExpression::from(Identifier {
                loc: Location::new(0, Span::new(0, 1)),
                text: "x".to_string(),
            })),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_complex_identifier() {
        test_expression(ExpressionTest {
            input: "x_92$",
            expected: Expression::Path(PathExpression::from(Identifier {
                loc: Location::new(0, Span::new(0, 5)),
                text: "x_92$".to_string(),
            })),
            diagnostics: vec![],
        })
    }
}
