use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

struct OpeningTag {
    name: String,
    attributes: Vec<Attribute>,
    self_closing: bool,
    loc: Location,
}

impl Parser<'_> {
    pub fn parse_element_expression(&mut self) -> ElementExpression {
        let tag = self.parse_opening_tag();
        if tag.self_closing {
            return ElementExpression::Void(VoidElement {
                loc: tag.loc,
                tag_name: tag.name,
                attributes: tag.attributes,
            });
        }

        let children = self.parse_children();
        let end = self.maybe_parse_end_tag();
        if end.is_none() {
            let error_loc = self.next_loc();
            self.error(DiagnosticKind::MissingCloseTag, error_loc);
        }
        match end.as_ref() {
            Some((end_name, end_loc)) if *end_name != tag.name => {
                let open = tag.name.clone();
                let close = end_name.clone();
                let kind = DiagnosticKind::MismatchedTags { open, close };
                self.error(kind, *end_loc);
            }
            _ => {}
        }
        let loc = if let Some(end) = end {
            Location::merge(tag.loc, end.1)
        } else if let Some(last) = children.last() {
            Location::merge(tag.loc, last.loc())
        } else {
            tag.loc
        };
        ElementExpression::Element(Element {
            loc,
            tag_name: tag.name,
            attributes: tag.attributes,
            children,
        })
    }

    fn parse_opening_tag(&mut self) -> OpeningTag {
        let start_range = self.eat(&[Token::Lt]);
        let mut loc = self.localize(start_range);

        let name_result = self.try_parse(
            |s| s.maybe_parse_identifier(),
            |t| matches!(*t, Token::Gt | Token::TagClose | Token::Newline),
        );

        let name = match name_result {
            Ok(Some(i)) => {
                loc = Location::merge(loc, i.loc);
                i.as_str().to_string()
            }
            _ => {
                self.error(DiagnosticKind::MissingName, loc.increment());
                "".to_string()
            }
        };

        let attributes = self.parse_attributes();
        if let Some(a) = attributes.last() {
            loc = Location::merge(loc, a.loc);
        }

        let self_closing = self.try_parse(
            |s| match s.tokens.peek().cloned() {
                Some((Ok(Token::Gt), r)) => {
                    s.tokens.next();
                    let end = s.localize(r);
                    loc = Location::merge(loc, end);
                    Some(false)
                }
                Some((Ok(Token::TagClose), r)) => {
                    s.tokens.next();
                    let end = s.localize(r);
                    loc = Location::merge(loc, end);
                    Some(true)
                }
                _ => None,
            },
            |t| *t == Token::Newline,
        );
        let self_closing = match self_closing {
            Ok(Some(s)) => s,
            _ => {
                let expected = vec![">".to_string(), "/>".to_string()];
                self.error(DiagnosticKind::ExpectedToken { expected }, loc.increment());
                true
            }
        };

        OpeningTag {
            name,
            attributes,
            self_closing,
            loc,
        }
    }

    fn parse_attributes(&mut self) -> Vec<Attribute> {
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

    fn parse_attribute(&mut self) -> Option<Attribute> {
        let result = self.better_expect(
            |t| t.text().map(|t| t.to_string()),
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
            return Some(Attribute {
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
                let value_loc = self.localize(value_range);
                loc = Location::merge(loc, value_loc);
                AttributeValue::String(StringLiteral {
                    loc: value_loc,
                    text: value,
                })
            }
            Token::LBrace => {
                let expression = self.parse_expression_with_block();
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

        Some(Attribute {
            loc,
            name,
            value: Some(attribute),
        })
    }

    fn parse_children(&mut self) -> Vec<ElementChild> {
        let mut children = Vec::new();

        while let Some((token, _)) = self.tokens.peek().cloned() {
            match token {
                Ok(Token::LtSlash) => break,

                Ok(Token::Lt) => {
                    children.push(self.parse_element_expression().into());
                }

                // Expression child: { expr }
                Ok(Token::LBrace) => {
                    self.tokens.next(); // eat '{'

                    let expression = self.parse_expression_with_block();
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
                        children.push(ElementChild::Expression(expression));
                    }
                }

                Ok(Token::Newline) => {
                    self.tokens.next();
                }

                _ => {
                    children.push(self.parse_raw_text().into());
                }
            }
        }

        children
    }

    fn parse_raw_text(&mut self) -> TextNode {
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
        TextNode { loc, text }
    }

    fn maybe_parse_end_tag(&mut self) -> Option<(String, Location)> {
        let (_, start_loc) = self.maybe_eat(|t| (*t == Token::LtSlash).then_some(()))?;

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

        Some((tag_name, Location::merge(start_loc, end_loc)))
    }
}

#[cfg(test)]
mod tests {
    use tine_common::locations::Span;

    use crate::test_utils::{test_expression, ExpressionTest};

    use super::*;

    #[test]
    fn test_parse_void_element() {
        test_expression(ExpressionTest {
            input: "<img />",
            expected: Expression::Element(ElementExpression::Void(VoidElement {
                loc: Location::new(0, Span::new(0, 7)),
                tag_name: "img".to_owned(),
                attributes: vec![],
            })),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_invalid_tag() {
        let mut parser = Parser::new(0, "<img");
        parser.parse_element_expression();
        assert!(!parser.diagnostics.is_empty())
    }

    #[test]
    fn test_parse_void_element_with_bool_attribute() {
        test_expression(ExpressionTest {
            input: "<img foo />",
            expected: Expression::Element(ElementExpression::Void(VoidElement {
                loc: Location::new(0, Span::new(0, 11)),
                tag_name: "img".to_owned(),
                attributes: vec![Attribute {
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
            expected: Expression::Element(ElementExpression::Void(VoidElement {
                loc: Location::new(0, Span::new(0, 17)),
                tag_name: "img".to_owned(),
                attributes: vec![Attribute {
                    loc: Location::new(0, Span::new(5, 14)),
                    name: "src".to_owned(),
                    value: Some(AttributeValue::String(StringLiteral {
                        loc: Location::new(0, Span::new(9, 14)),
                        text: "foo".to_string(),
                    })),
                }],
            })),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_void_element_with_expr_attribute() {
        test_expression(ExpressionTest {
            input: "<img src={foo} />",
            expected: Expression::Element(ElementExpression::Void(VoidElement {
                loc: Location::new(0, Span::new(0, 17)),
                tag_name: "img".to_owned(),
                attributes: vec![Attribute {
                    loc: Location::new(0, Span::new(5, 14)),
                    name: "src".to_owned(),
                    value: Some(AttributeValue::Expression(Expression::Path(
                        PathExpression::from(Identifier {
                            loc: Location::new(0, Span::new(10, 13)),
                            text: "foo".to_owned(),
                        }),
                    ))),
                }],
            })),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_element() {
        test_expression(ExpressionTest {
            input: "<tag></tag>",
            expected: Expression::Element(ElementExpression::Element(Element {
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
            expected: Expression::Element(ElementExpression::Element(Element {
                loc: Location::new(0, Span::new(0, 14)),
                tag_name: "tag".to_owned(),
                attributes: vec![],
                children: vec![ElementChild::Text(TextNode {
                    loc: Location::new(0, Span::new(5, 8)),
                    text: "foo".to_string(),
                })],
            })),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_element_with_non_standard_text() {
        let mut parser = Parser::new(0, "<tag>×</tag>");
        parser.parse_element_expression();
    }

    #[test]
    fn parse_kw_attribute() {
        let mut parser = Parser::new(0, "type=\"text\"");
        parser.parse_attribute().expect("expected an attribute");
    }
}
