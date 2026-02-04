use crate::{
    ast,
    parser2::{Parser, Token},
};

impl Parser<'_> {
    pub fn parse_atom(&mut self) -> ast::Expression {
        let Some(ranged_token) = self.tokens.peek() else {
            return ast::Expression::Empty;
        };
        let (Ok(token), _) = ranged_token else {
            // TODO: handle error with InvalidExpression
            panic!()
        };
        match token {
            Token::Bool(_) => self.parse_bool().into(),
            Token::Int(_) => self.parse_int().into(),
            Token::Float(_) => self.parse_float().into(),
            Token::String(_) => self.parse_string().into(),
            Token::Ident(_) => self.parse_identifier().into(),
            Token::LParen => self.parse_tuple().into(),
            _ => ast::Expression::Empty,
        }
    }

    pub(super) fn parse_int(&mut self) -> ast::IntLiteral {
        let Some((Ok(Token::Int(value)), span)) = self.tokens.next() else {
            panic!()
        };

        ast::IntLiteral {
            loc: self.localize(span),
            value,
        }
    }

    fn parse_float(&mut self) -> ast::FloatLiteral {
        let Some((Ok(Token::Float(value)), span)) = self.tokens.next() else {
            panic!()
        };

        ast::FloatLiteral {
            loc: self.localize(span),
            value: value.value,
        }
    }

    fn parse_bool(&mut self) -> ast::BooleanLiteral {
        let Some((Ok(Token::Bool(value)), span)) = self.tokens.next() else {
            panic!()
        };

        ast::BooleanLiteral {
            loc: self.localize(span),
            value,
        }
    }

    fn parse_string(&mut self) -> ast::StringLiteral {
        let Some((Ok(Token::String(text)), span)) = self.tokens.next() else {
            panic!()
        };

        ast::StringLiteral {
            loc: self.localize(span),
            text,
        }
    }

    pub(super) fn parse_identifier(&mut self) -> ast::Identifier {
        let Some((Ok(Token::Ident(text)), span)) = self.tokens.next() else {
            panic!()
        };

        ast::Identifier {
            loc: self.localize(span),
            text,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser2::test_utils::{run, Test},
        Location, Span,
    };

    use super::*;

    #[test]
    fn test_parse_int() {
        run(Test {
            input: "42",
            expected: ast::Expression::IntLiteral(ast::IntLiteral {
                loc: Location::new(0, Span::new(0, 2)),
                value: 42,
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn parse_int_with_underscore() {
        run(Test {
            input: "42_000",
            expected: ast::Expression::IntLiteral(ast::IntLiteral {
                loc: Location::new(0, Span::new(0, 6)),
                value: 42000,
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_float() {
        run(Test {
            input: "3.14",
            expected: ast::Expression::FloatLiteral(ast::FloatLiteral {
                loc: Location::new(0, Span::new(0, 4)),
                value: ordered_float::OrderedFloat(3.14),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_float_no_decimals() {
        run(Test {
            input: "3.",
            expected: ast::Expression::FloatLiteral(ast::FloatLiteral {
                loc: Location::new(0, Span::new(0, 2)),
                value: ordered_float::OrderedFloat(3.),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_float_with_underscore() {
        run(Test {
            input: "3.14_000",
            expected: ast::Expression::FloatLiteral(ast::FloatLiteral {
                loc: Location::new(0, Span::new(0, 8)),
                value: ordered_float::OrderedFloat(3.14),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_float_only_decimals() {
        run(Test {
            input: ".14",
            expected: ast::Expression::FloatLiteral(ast::FloatLiteral {
                loc: Location::new(0, Span::new(0, 3)),
                value: ordered_float::OrderedFloat(0.14),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn parse_bool_true() {
        run(Test {
            input: "true",
            expected: ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                loc: Location::new(0, Span::new(0, 4)),
                value: true,
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn parse_bool_false() {
        run(Test {
            input: "false",
            expected: ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                loc: Location::new(0, Span::new(0, 5)),
                value: false,
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn parse_string() {
        run(Test {
            input: "\"hello world\"",
            expected: ast::Expression::StringLiteral(ast::StringLiteral {
                loc: Location::new(0, Span::new(0, 13)),
                text: "hello world".to_string(),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn parse_string_with_escaped_quote() {
        run(Test {
            input: "\"hello \\\"world\\\"\"",
            expected: ast::Expression::StringLiteral(ast::StringLiteral {
                loc: Location::new(0, Span::new(0, 17)),
                text: "hello \"world\"".to_string(),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_identifier() {
        run(Test {
            input: "x",
            expected: ast::Expression::Identifier(ast::Identifier {
                loc: Location::new(0, Span::new(0, 1)),
                text: "x".to_string(),
            }),
            diagnostics: vec![],
        })
    }
}
