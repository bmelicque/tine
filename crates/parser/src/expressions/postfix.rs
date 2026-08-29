use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, ExpressionCtx, Parser};

impl Parser<'_> {
    pub fn parse_postfix(&mut self, ctx: ExpressionCtx) -> Option<Expression> {
        let mut expression = self.parse_atom();
        use Expression::*;
        use Token::*;
        while let Some((Ok(token), _)) = self.tokens.peek() {
            expression = match token {
                Dot => Some(self.parse_dot_expression(expression).into()),
                Float(float) if *float.value >= 0. && *float.value < 1. => {
                    Some(self.parse_member_from_float(expression).into())
                }
                LParen => {
                    // Expression cannot be `None` here:
                    // if expression started with params, it would be parsed as a tuple and not going through this branch
                    Some(self.parse_call_expression(expression.unwrap()).into())
                }
                LBrace if ctx.braces() => {
                    let Some(Path(path)) = expression else {
                        break;
                    };
                    // Expression cannot be `None` here:
                    // if expression started with struct body, it would be parsed as a block
                    Some(self.parse_struct_expression(path).into())
                }
                _ => break,
            }
        }
        expression
    }

    fn parse_dot_expression(&mut self, object: Option<Expression>) -> Expression {
        let dot_range = self.eat(&[Token::Dot]);
        let loc = match &object {
            Some(object) => Location::merge(object.loc(), self.localize(dot_range)),
            None => self.localize(dot_range),
        };
        match self.tokens.peek().cloned() {
            Some((Ok(Token::Ident(_)), range)) => Expression::Member(MemberExpression {
                loc: Location::merge(loc, self.localize(range)),
                object: object.map(|o| Box::new(o)),
                prop: Some(MemberProp::FieldName(self.parse_identifier())),
            }),
            Some((Ok(Token::Int(_)), range)) => Expression::Member(MemberExpression {
                loc: Location::merge(loc, self.localize(range.clone())),
                object: object.map(|o| Box::new(o)),
                prop: Some(MemberProp::Index(self.parse_int())),
            }),
            Some((Ok(Token::Float(float)), range)) => {
                // this is actually two indices
                let (left, right) = float.src.split_once(".").unwrap();
                let left_range = range.start..range.start + left.len();
                let inner_prop = if left == "" {
                    let loc = self.localize(range.clone()).decrement().increment();
                    self.error(DiagnosticKind::InvalidMember, loc);
                    None
                } else {
                    Some(MemberProp::Index(IntLiteral {
                        loc: self.localize(left_range.clone()),
                        value: left.replace("_", "").parse().unwrap(),
                    }))
                };
                let inner = MemberExpression {
                    loc: Location::merge(loc, self.localize(left_range)),
                    object: object.map(|o| Box::new(o)),
                    prop: inner_prop,
                };

                let right_range = range.start + left.len() + 1..range.end;
                let outer_prop = if right == "" {
                    let loc = self.localize(range.clone()).increment();
                    self.error(DiagnosticKind::InvalidMember, loc);
                    None
                } else {
                    Some(MemberProp::Index(IntLiteral {
                        loc: self.localize(right_range),
                        value: right.replace("_", "").parse().unwrap(),
                    }))
                };
                Expression::Member(MemberExpression {
                    loc: Location::merge(loc, self.localize(range.clone())),
                    object: Some(Box::new(inner.into())),
                    prop: outer_prop,
                })
            }
            Some((Ok(Token::Lt), _)) => self
                .parse_call_expression_with_type_args(object, loc)
                .into(),
            _ => {
                if object.is_some() {
                    self.error(DiagnosticKind::InvalidMember, loc.increment());
                }
                Expression::Member(MemberExpression {
                    loc,
                    object: object.map(|o| Box::new(o)),
                    prop: None,
                })
            }
        }
    }

    fn parse_member_from_float(&mut self, object: Option<Expression>) -> MemberExpression {
        let Some((Ok(Token::Float(float)), float_range)) = self.tokens.next() else {
            panic!("Expected '.'");
        };
        let index_range = float_range.start + 1..float_range.end;
        let loc = match &object {
            Some(object) => Location::merge(object.loc(), self.localize(float_range)),
            None => self.localize(float_range),
        };
        let (_, right) = float.src.split_once(".").unwrap();
        let prop = if right == "" {
            let loc = self.localize(index_range);
            self.error(DiagnosticKind::InvalidMember, loc);
            None
        } else {
            Some(MemberProp::Index(IntLiteral {
                loc: self.localize(index_range.clone()),
                value: right.replace("_", "").parse::<i64>().unwrap(),
            }))
        };
        MemberExpression {
            loc,
            object: object.map(|o| Box::new(o)),
            prop,
        }
    }

    fn parse_call_expression(&mut self, callee: Expression) -> CallExpression {
        self.eat(&[Token::LParen]);
        let args = self.parse_list(
            |p| p.parse_expression_with_block(),
            Token::Comma,
            Token::RParen,
        );
        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RParen), _)) => self.eat(&[Token::RParen]),
            _ => self.recover_at(&[Token::RParen]),
        };
        let loc = Location::merge(callee.loc(), self.localize(end_range));
        CallExpression {
            loc,
            callee: Some(Box::new(callee)),
            type_args: None,
            args,
        }
    }

    fn parse_call_expression_with_type_args(
        &mut self,
        callee: Option<Expression>,
        start_loc: Location,
    ) -> CallExpression {
        self.eat(&[Token::Lt]);
        let type_args = self.parse_list(|p| p.parse_type(), Token::Comma, Token::Gt);
        let close_range = self.expect(Token::Gt);
        let Some((Ok(Token::LParen), _)) = self.tokens.peek() else {
            let error_loc = self.next_loc();
            self.error(DiagnosticKind::MissingParams, error_loc);
            let close_loc = self.localize(close_range);
            return CallExpression {
                loc: Location::merge(start_loc, close_loc),
                callee: callee.map(|c| Box::new(c)),
                type_args: Some(type_args),
                args: vec![],
            };
        };
        self.eat(&[Token::LParen]);
        let args = self.parse_list(
            |p| p.parse_expression_with_block(),
            Token::Comma,
            Token::RParen,
        );
        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RParen), _)) => self.eat(&[Token::RParen]),
            _ => self.recover_at(&[Token::RParen]),
        };
        let loc = Location::merge(start_loc, self.localize(end_range));
        CallExpression {
            loc,
            callee: callee.map(|c| Box::new(c)),
            type_args: Some(type_args),
            args,
        }
    }
}

#[cfg(test)]
mod tests {
    use tine_common::locations::Span;

    use crate::test_utils::{parse_expression, test_expression, ExpressionTest};

    use super::*;

    #[test]
    fn parse_field_access() {
        let expr = parse_expression("object.field").expect("expected no errors");
        let path = expr.as_path().expect("expected a path");
        assert_eq!(path.segments.len(), 2);
        assert!(path.segments[0].generic_args.is_none());
        assert_eq!(path.segments[0].ident.as_str(), "object");
        assert!(path.segments[1].generic_args.is_none());
        assert_eq!(path.segments[1].ident.as_str(), "field");
    }

    #[test]
    fn parse_single_dot() {
        let expr = parse_expression(".").expect("expected no errors");
        let member = expr.as_member().expect("expected a path");
        assert!(member.object.is_none());
        assert!(member.prop.is_none());
    }

    #[test]
    fn parse_tuple_index() {
        test_expression(ExpressionTest {
            input: "object.0",
            expected: Expression::Member(MemberExpression {
                loc: Location::new(0, Span::new(0, 8)),
                object: Some(Box::new(Expression::Path(PathExpression::from(
                    Identifier {
                        loc: Location::new(0, Span::new(0, 6)),
                        text: "object".to_string(),
                    },
                )))),
                prop: Some(MemberProp::Index(IntLiteral {
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
            expected: Expression::Member(MemberExpression {
                loc: Location::new(0, Span::new(0, 10)),
                object: Some(Box::new(Expression::Member(MemberExpression {
                    loc: Location::new(0, Span::new(0, 8)),
                    object: Some(Box::new(Expression::Path(PathExpression::from(
                        Identifier {
                            loc: Location::new(0, Span::new(0, 6)),
                            text: "object".to_string(),
                        },
                    )))),
                    prop: Some(MemberProp::Index(IntLiteral {
                        loc: Location::new(0, Span::new(7, 8)),
                        value: 0,
                    })),
                }))),
                prop: Some(MemberProp::Index(IntLiteral {
                    loc: Location::new(0, Span::new(9, 10)),
                    value: 1,
                })),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_call_expression_no_args() {
        test_expression(ExpressionTest {
            input: "function()",
            expected: Expression::Call(CallExpression {
                loc: Location::new(0, Span::new(0, 10)),
                callee: Some(Box::new(Expression::Path(PathExpression::from(
                    Identifier {
                        loc: Location::new(0, Span::new(0, 8)),
                        text: "function".to_string(),
                    },
                )))),
                type_args: None,
                args: vec![],
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_call_expression_one_arg() {
        test_expression(ExpressionTest {
            input: "function(1)",
            expected: Expression::Call(CallExpression {
                loc: Location::new(0, Span::new(0, 11)),
                callee: Some(Box::new(Expression::Path(PathExpression::from(
                    Identifier {
                        loc: Location::new(0, Span::new(0, 8)),
                        text: "function".to_string(),
                    },
                )))),
                type_args: None,
                args: vec![Expression::IntLiteral(IntLiteral {
                    loc: Location::new(0, Span::new(9, 10)),
                    value: 1,
                })],
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_call_expression_with_type_args() {
        test_expression(ExpressionTest {
            input: "function.<T>()",
            expected: Expression::Call(CallExpression {
                loc: Location::new(0, Span::new(0, 14)),
                callee: Some(Box::new(Expression::Path(PathExpression {
                    loc: Location::new(0, Span::new(0, 12)),
                    segments: vec![PathSegment {
                        loc: Location::new(0, Span::new(0, 12)),
                        ident: Identifier {
                            loc: Location::new(0, Span::new(0, 8)),
                            text: "function".to_string(),
                        },
                        generic_args: Some(vec![Type::Named(NamedType {
                            loc: Location::new(0, Span::new(10, 11)),
                            name: Identifier {
                                loc: Location::new(0, Span::new(10, 11)),
                                text: "T".to_string(),
                            },
                            args: None,
                        })]),
                    }],
                }))),
                type_args: None,
                args: vec![],
            }),
            diagnostics: vec![],
        });
    }
}
