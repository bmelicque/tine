use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_element_expression(&mut self) -> ast::ElementExpression {
        let start_range = self.eat(&[Token::Lt]);
        let start_loc = self.localize(start_range);

        let tag_name = self.parse_tag_name();
        let attributes = self.parse_attributes();

        match self.tokens.peek().cloned() {
            Some((Ok(Token::Gt), _)) => {
                self.tokens.next();
            }
            Some((Ok(Token::TagClose), close_range)) => {
                self.tokens.next();
                return ast::ElementExpression::Void(ast::VoidElement {
                    loc: Location::merge(start_loc, self.localize(close_range)),
                    tag_name,
                    attributes,
                });
            }
            _ => {
                todo!("handle error")
            }
        }

        let children = self.parse_children();
        let (end_tag, end_loc) = self.parse_end_tag();
        if end_tag != tag_name {
            self.error(
                DiagnosticKind::MismatchedTags {
                    open: tag_name.clone(),
                    close: end_tag.clone(),
                },
                end_loc,
            )
        }
        let loc = Location::merge(start_loc, end_loc);
        ast::ElementExpression::Element(ast::Element {
            loc,
            tag_name,
            attributes,
            children,
        })
    }

    fn parse_tag_name(&mut self) -> String {
        let result = self.better_expect(
            |t| match t {
                Token::Ident(ident) => Some(ident.to_owned()),
                _ => None,
            },
            &[Token::Newline, Token::Gt, Token::TagClose],
        );
        match result {
            Ok(r) => r.0,
            Err(range) => {
                let loc = self.localize(range);
                self.error(DiagnosticKind::MissingName, loc);
                "".to_owned()
            }
        }
    }

    fn parse_attributes(&mut self) -> Vec<ast::Attribute> {
        let mut attributes = Vec::new();

        while let Some((Ok(token), _)) = self.tokens.peek().cloned() {
            match token.clone() {
                Token::Newline => {
                    self.tokens.next();
                }
                t if t == Token::Gt || t == Token::TagClose => break,
                _ => {
                    if let Some(attribute) = self.parse_attribute() {
                        attributes.push(attribute);
                    }
                }
            }
        }

        attributes
    }

    fn parse_attribute(&mut self) -> Option<ast::Attribute> {
        let result = self.better_expect(
            |t| match t {
                Token::Ident(ident) => Some(ident.to_owned()),
                _ => None,
            },
            &[Token::Newline, Token::Gt, Token::TagClose],
        );
        let (name, name_range) = match result {
            Ok(r) => r,
            Err(range) => {
                let expected = vec![Token::Gt, Token::TagClose]
                    .into_iter()
                    .map(|t| t.to_string())
                    .collect();
                let loc = self.localize(range);
                self.error(DiagnosticKind::ExpectedToken { expected }, loc);
                return None;
            }
        };

        let Some((Ok(Token::Eq), _)) = self.tokens.peek() else {
            return Some(ast::Attribute {
                loc: self.localize(name_range),
                name,
                value: None,
            });
        };
        let eq_range = self.eat(&[Token::Eq]);

        let mut loc = self.localize(name_range.start..eq_range.end);

        let result = self.better_expect(
            |t| match t {
                Token::String(_) | Token::LBrace => Some(t.clone()),
                _ => None,
            },
            &[Token::Gt, Token::TagClose, Token::Newline],
        );
        let (token, value_range) = match result {
            Ok(r) => r,
            Err(range) => {
                let error = DiagnosticKind::ExpectedToken {
                    expected: vec!["string".to_owned(), "{".to_owned()],
                };
                let loc = self.localize(range);
                self.error(error, loc);
                return None;
            }
        };
        let attribute = match token {
            Token::String(value) => {
                loc = Location::merge(loc, self.localize(value_range));
                ast::AttributeValue::String(value)
            }
            Token::LBrace => {
                let expression = self.parse_expression();
                if expression.is_none() {
                    let loc = self.next_loc();
                    self.error(DiagnosticKind::MissingExpression, loc);
                }
                let res = self.better_expect(
                    |t| match t {
                        Token::RBrace => Some(()),
                        _ => None,
                    },
                    &[Token::Gt, Token::TagClose],
                );
                let end_loc = match res {
                    Ok((_, r)) => self.localize(r),
                    Err(r) => self.localize(r).decrement(),
                };
                loc = Location::merge(loc, end_loc);
                match expression {
                    Some(expression) => expression.into(),
                    None => return None,
                }
            }
            // unreachable thanks to `better_expect` above
            _ => unreachable!(),
        };

        Some(ast::Attribute {
            loc,
            name,
            value: Some(attribute),
        })
    }

    fn parse_children(&mut self) -> Vec<ast::ElementChild> {
        let mut children = Vec::new();

        while let Some((Ok(token), _)) = self.tokens.peek().cloned() {
            match token {
                Token::LtSlash => break,

                Token::Lt => {
                    children.push(self.parse_element_expression().into());
                }

                // Expression child: { expr }
                Token::LBrace => {
                    self.tokens.next(); // eat '{'

                    let expression = self.parse_expression();
                    if expression.is_none() {
                        let loc = self.next_loc();
                        self.error(DiagnosticKind::MissingExpression, loc);
                    }
                    let _ = self.better_expect(
                        |t| match t {
                            Token::RBrace => Some(()),
                            _ => None,
                        },
                        &[Token::Gt, Token::TagClose],
                    );

                    if let Some(expression) = expression {
                        children.push(ast::ElementChild::Expression(expression));
                    }
                }

                Token::Newline => {
                    self.tokens.next();
                }

                _ => {
                    children.push(self.parse_raw_text().into());
                }
            }
        }

        children
    }

    fn parse_raw_text(&mut self) -> ast::TextNode {
        let mut range = self.next_range();
        if range.start >= 1 && &self.src[range.start - 1..range.start] == " " {
            range.start -= 1;
        }
        while let Some((token, r)) = self.tokens.peek() {
            match token {
                Ok(Token::LBrace | Token::Lt | Token::LtSlash) => {
                    range.end = r.start;
                    break;
                }
                _ => range.end = r.end,
            }
            self.tokens.next();
        }
        let loc = self.localize(range.clone());
        let text = self.src[range].to_string();
        ast::TextNode { loc, text }
    }

    fn parse_end_tag(&mut self) -> (String, Location) {
        let start_range = self.eat(&[Token::LtSlash]);
        let start_loc = self.localize(start_range);

        let result = self.better_expect(
            |t| match t {
                Token::Ident(name) => Some(name.to_owned()),
                _ => None,
            },
            &[Token::Gt, Token::Newline],
        );
        let tag_name = match result {
            Ok(r) => r.0,
            Err(range) => {
                let loc = self.localize(range);
                self.error(DiagnosticKind::MissingName, loc);
                "".to_owned()
            }
        };

        let res = self.better_expect(
            |t| match t {
                Token::Gt => Some(()),
                _ => None,
            },
            &[Token::Newline],
        );
        let end_loc = match res {
            Ok((_, range)) => self.localize(range),
            Err(range) => self.localize(range).decrement(),
        };

        (tag_name, Location::merge(start_loc, end_loc))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser::test_utils::{test_expression, ExpressionTest},
        Span,
    };

    use super::*;

    #[test]
    fn test_parse_void_element() {
        test_expression(ExpressionTest {
            input: "<img />",
            expected: ast::Expression::Element(ast::ElementExpression::Void(ast::VoidElement {
                loc: Location::new(0, Span::new(0, 7)),
                tag_name: "img".to_owned(),
                attributes: vec![],
            })),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_void_element_with_bool_attribute() {
        test_expression(ExpressionTest {
            input: "<img foo />",
            expected: ast::Expression::Element(ast::ElementExpression::Void(ast::VoidElement {
                loc: Location::new(0, Span::new(0, 11)),
                tag_name: "img".to_owned(),
                attributes: vec![ast::Attribute {
                    loc: Location::new(0, Span::new(5, 8)),
                    name: "foo".to_owned(),
                    value: None,
                }],
            })),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_void_element_with_string_attribute() {
        test_expression(ExpressionTest {
            input: "<img src=\"foo\" />",
            expected: ast::Expression::Element(ast::ElementExpression::Void(ast::VoidElement {
                loc: Location::new(0, Span::new(0, 17)),
                tag_name: "img".to_owned(),
                attributes: vec![ast::Attribute {
                    loc: Location::new(0, Span::new(5, 14)),
                    name: "src".to_owned(),
                    value: Some(ast::AttributeValue::String("foo".to_string())),
                }],
            })),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_void_element_with_expr_attribute() {
        test_expression(ExpressionTest {
            input: "<img src={foo} />",
            expected: ast::Expression::Element(ast::ElementExpression::Void(ast::VoidElement {
                loc: Location::new(0, Span::new(0, 17)),
                tag_name: "img".to_owned(),
                attributes: vec![ast::Attribute {
                    loc: Location::new(0, Span::new(5, 14)),
                    name: "src".to_owned(),
                    value: Some(ast::AttributeValue::Expression(
                        ast::Expression::Identifier(ast::Identifier {
                            loc: Location::new(0, Span::new(10, 13)),
                            text: "foo".to_owned(),
                        }),
                    )),
                }],
            })),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_element() {
        test_expression(ExpressionTest {
            input: "<tag></tag>",
            expected: ast::Expression::Element(ast::ElementExpression::Element(ast::Element {
                loc: Location::new(0, Span::new(0, 11)),
                tag_name: "tag".to_owned(),
                attributes: vec![],
                children: vec![],
            })),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_element_with_text_child() {
        test_expression(ExpressionTest {
            input: "<tag>foo</tag>",
            expected: ast::Expression::Element(ast::ElementExpression::Element(ast::Element {
                loc: Location::new(0, Span::new(0, 14)),
                tag_name: "tag".to_owned(),
                attributes: vec![],
                children: vec![ast::ElementChild::Text(ast::TextNode {
                    loc: Location::new(0, Span::new(5, 8)),
                    text: "foo".to_string(),
                })],
            })),
            diagnostics: vec![],
        });
    }
}
