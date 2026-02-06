use crate::{
    ast,
    parser2::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_postfix(&mut self) -> ast::Expression {
        let mut expression = self.parse_atom();
        while let Some((Ok(token), _)) = self.tokens.peek() {
            match token {
                Token::Dot => {
                    expression = self.parse_member_expression(expression).into();
                }
                Token::Float(float) if *float.value < 1. => {
                    expression = self.parse_member_from_float(expression).into();
                }
                Token::LParen => {
                    expression = self.parse_call_expression(expression).into();
                }
                _ => return expression,
            }
        }
        expression
    }

    fn parse_member_expression(&mut self, object: ast::Expression) -> ast::MemberExpression {
        let Some((Ok(Token::Dot), dot_range)) = self.tokens.next() else {
            panic!("Expected '.'");
        };
        let loc = Location::merge(object.loc(), self.localize(dot_range));
        match self.tokens.peek().cloned() {
            Some((Ok(Token::Ident(_)), range)) => ast::MemberExpression {
                loc: Location::merge(loc, self.localize(range.clone())),
                object: Box::new(object),
                prop: Some(ast::MemberProp::FieldName(self.parse_identifier())),
            },
            Some((Ok(Token::Int(_)), range)) => ast::MemberExpression {
                loc: Location::merge(loc, self.localize(range.clone())),
                object: Box::new(object),
                prop: Some(ast::MemberProp::Index(self.parse_int())),
            },
            Some((Ok(Token::Float(float)), range)) => {
                // this is actually two indices
                let (left, right) = float.src.split_once(".").unwrap();
                let left_range = range.start..range.start + left.len();
                let inner_prop = if left == "" {
                    let loc = self.localize(range.clone()).decrement().increment();
                    self.error(DiagnosticKind::InvalidMember, loc);
                    None
                } else {
                    Some(ast::MemberProp::Index(ast::IntLiteral {
                        loc: self.localize(left_range.clone()),
                        value: left.replace("_", "").parse().unwrap(),
                    }))
                };
                let inner = ast::MemberExpression {
                    loc: Location::merge(loc, self.localize(left_range)),
                    object: Box::new(object),
                    prop: inner_prop,
                };

                let right_range = range.start + left.len() + 1..range.end;
                let outer_prop = if right == "" {
                    let loc = self.localize(range.clone()).increment();
                    self.error(DiagnosticKind::InvalidMember, loc);
                    None
                } else {
                    Some(ast::MemberProp::Index(ast::IntLiteral {
                        loc: self.localize(right_range),
                        value: right.replace("_", "").parse().unwrap(),
                    }))
                };
                ast::MemberExpression {
                    loc: Location::merge(loc, self.localize(range.clone())),
                    object: Box::new(inner.into()),
                    prop: outer_prop,
                }
            }
            _ => {
                self.error(DiagnosticKind::InvalidMember, loc.increment());
                return ast::MemberExpression {
                    loc,
                    object: Box::new(object),
                    prop: None,
                };
            }
        }
    }

    fn parse_member_from_float(&mut self, object: ast::Expression) -> ast::MemberExpression {
        let Some((Ok(Token::Float(float)), float_range)) = self.tokens.next() else {
            panic!("Expected '.'");
        };
        let index_range = float_range.start + 1..float_range.end;
        let loc = Location::merge(object.loc(), self.localize(float_range));
        let (_, right) = float.src.split_once(".").unwrap();
        let prop = if right == "" {
            let loc = self.localize(index_range);
            self.error(DiagnosticKind::InvalidMember, loc);
            None
        } else {
            Some(ast::MemberProp::Index(ast::IntLiteral {
                loc: self.localize(index_range.clone()),
                value: right.replace("_", "").parse::<i64>().unwrap(),
            }))
        };
        ast::MemberExpression {
            loc,
            object: Box::new(object),
            prop,
        }
    }

    fn parse_call_expression(&mut self, callee: ast::Expression) -> ast::CallExpression {
        self.tokens.next(); // consume '('
        let args = self.parse_list(|p| p.parse_argument(), Token::Comma, Token::RParen);
        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RParen), r)) => r.clone(),
            _ => self.recover_at(&[Token::RParen]),
        };
        let loc = Location::merge(callee.loc(), self.localize(end_range));
        ast::CallExpression {
            loc,
            callee: Box::new(callee),
            args,
        }
    }

    fn parse_argument(&mut self) -> Option<ast::CallArgument> {
        let Some(argument) = self.parse_expression() else {
            return None;
        };

        let ast::Expression::Tuple(tuple) = argument else {
            return Some(argument.into());
        };

        let Some((Ok(Token::FatArrow), arrow_range)) = self.tokens.peek().cloned() else {
            return Some(ast::Expression::Tuple(tuple).into());
        };
        self.tokens.next(); // consume '=>'
        let mut loc = Location::merge(tuple.loc, self.localize(arrow_range));

        let body = match self.parse_expression() {
            Some(expr) => {
                loc = Location::merge(loc, expr.loc());
                expr
            }
            None => {
                self.error(DiagnosticKind::MissingExpression, loc.increment());
                ast::Expression::Empty
            }
        };

        let params = tuple
            .elements
            .into_iter()
            .map(|element| {
                ast::CallbackParam::Identifier(ast::Identifier {
                    loc: element.loc(),
                    text: match element {
                        ast::Expression::Identifier(identifier) => identifier.text.clone(),
                        _ => unreachable!("FIXME"),
                    },
                })
            })
            .collect();
        Some(ast::CallArgument::Callback(ast::Callback {
            loc,
            params,
            body: Box::new(body),
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser2::test_utils::{test_expression, ExpressionTest},
        Diagnostic, DiagnosticLevel, Span,
    };

    use super::*;

    #[test]
    fn parse_field_access() {
        test_expression(ExpressionTest {
            input: "object.field",
            expected: ast::Expression::Member(ast::MemberExpression {
                loc: Location::new(0, Span::new(0, 12)),
                object: Box::new(ast::Expression::Identifier(ast::Identifier {
                    loc: Location::new(0, Span::new(0, 6)),
                    text: "object".to_string(),
                })),
                prop: Some(ast::MemberProp::FieldName(ast::Identifier {
                    loc: Location::new(0, Span::new(7, 12)),
                    text: "field".to_string(),
                })),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_tuple_index() {
        test_expression(ExpressionTest {
            input: "object.0",
            expected: ast::Expression::Member(ast::MemberExpression {
                loc: Location::new(0, Span::new(0, 8)),
                object: Box::new(ast::Expression::Identifier(ast::Identifier {
                    loc: Location::new(0, Span::new(0, 6)),
                    text: "object".to_string(),
                })),
                prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                    loc: Location::new(0, Span::new(7, 8)),
                    value: 0,
                })),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_two_indices() {
        test_expression(ExpressionTest {
            input: "object.0.1",
            expected: ast::Expression::Member(ast::MemberExpression {
                loc: Location::new(0, Span::new(0, 10)),
                object: Box::new(ast::Expression::Member(ast::MemberExpression {
                    loc: Location::new(0, Span::new(0, 8)),
                    object: Box::new(ast::Expression::Identifier(ast::Identifier {
                        loc: Location::new(0, Span::new(0, 6)),
                        text: "object".to_string(),
                    })),
                    prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                        loc: Location::new(0, Span::new(7, 8)),
                        value: 0,
                    })),
                })),
                prop: Some(ast::MemberProp::Index(ast::IntLiteral {
                    loc: Location::new(0, Span::new(9, 10)),
                    value: 1,
                })),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_member_expression_with_trailing_dot() {
        test_expression(ExpressionTest {
            input: "object.",
            expected: ast::Expression::Member(ast::MemberExpression {
                loc: Location::new(0, Span::new(0, 7)),
                object: Box::new(ast::Expression::Identifier(ast::Identifier {
                    loc: Location::new(0, Span::new(0, 6)),
                    text: "object".to_string(),
                })),
                prop: None,
            }),
            diagnostics: vec![Diagnostic {
                loc: Location::new(0, Span::new(7, 8)),
                kind: DiagnosticKind::InvalidMember,
                level: DiagnosticLevel::Error,
            }],
        });
    }

    #[test]
    fn parse_call_expression_no_args() {
        test_expression(ExpressionTest {
            input: "function()",
            expected: ast::Expression::Call(ast::CallExpression {
                loc: Location::new(0, Span::new(0, 10)),
                callee: Box::new(ast::Expression::Identifier(ast::Identifier {
                    loc: Location::new(0, Span::new(0, 8)),
                    text: "function".to_string(),
                })),
                args: vec![],
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_call_expression_one_arg() {
        test_expression(ExpressionTest {
            input: "function(1)",
            expected: ast::Expression::Call(ast::CallExpression {
                loc: Location::new(0, Span::new(0, 11)),
                callee: Box::new(ast::Expression::Identifier(ast::Identifier {
                    loc: Location::new(0, Span::new(0, 8)),
                    text: "function".to_string(),
                })),
                args: vec![ast::CallArgument::Expression(ast::Expression::IntLiteral(
                    ast::IntLiteral {
                        loc: Location::new(0, Span::new(9, 10)),
                        value: 1,
                    },
                ))],
            }),
            diagnostics: vec![],
        });
    }
}
