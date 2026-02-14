use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_function_expression(&mut self) -> ast::FunctionExpression {
        let start_range = self.eat(&[Token::Fn]);
        let start_loc = self.localize(start_range);
        let name = self.parse_function_name();
        let type_params = self.parse_function_type_params();
        let params = self.parse_function_params();
        let return_type = self.parse_type();
        let body = self.parse_function_body();
        let loc = Location::merge(start_loc, body.loc);

        ast::FunctionExpression {
            loc,
            name,
            type_params,
            params,
            return_type,
            body,
        }
    }

    fn parse_function_name(&mut self) -> Option<ast::Identifier> {
        if let Some((Ok(Token::Ident(ident)), range)) = self.tokens.peek() {
            let name = ident.to_owned();
            let range = range.clone();
            self.tokens.next();
            Some(ast::Identifier {
                loc: self.localize(range),
                text: name,
            })
        } else {
            None
        }
    }

    fn parse_function_type_params(&mut self) -> Option<Vec<ast::Identifier>> {
        match self.tokens.peek() {
            Some((Ok(Token::Lt), _)) => Some(self.parse_type_params()),
            _ => None,
        }
    }

    fn parse_function_params(&mut self) -> Vec<ast::FunctionParam> {
        let result = self.better_expect(
            |t| match t {
                Token::LParen => Some(()),
                _ => None,
            },
            &[Token::Newline],
        );
        match result {
            Ok(_) => {}
            Err(range) => {
                self.error(DiagnosticKind::MissingName, self.localize(range));
                let Some((Ok(Token::LParen), _)) = self.tokens.peek() else {
                    return vec![];
                };
                self.tokens.next();
            }
        }

        let params = self.parse_list(|p| p.parse_function_param(), Token::Comma, Token::RParen);
        let result = self.better_expect(
            |t| match t {
                Token::RParen => Some(()),
                _ => None,
            },
            &[],
        );
        match result {
            Ok(_) => {}
            Err(range) => {
                let error = DiagnosticKind::ExpectedToken {
                    expected: vec![Token::RParen.to_string()],
                };
                self.error(error, self.localize(range));
                if let Some((Ok(Token::RParen), _)) = self.tokens.peek() {
                    self.tokens.next();
                };
            }
        }

        params
    }

    fn parse_function_param(&mut self) -> Option<ast::FunctionParam> {
        let result = self.better_expect(
            |t| match t {
                Token::Ident(i) => Some(i.to_owned()),
                _ => None,
            },
            &[Token::Comma, Token::RParen, Token::Newline],
        );
        let (name_text, name_loc) = match result {
            Ok(r) => r,
            Err(range) => {
                self.error(DiagnosticKind::MissingPattern, self.localize(range));
                return None;
            }
        };
        let name = ast::Identifier {
            loc: self.localize(name_loc),
            text: name_text,
        };

        let type_annotation = self.parse_type();

        let loc = match &type_annotation {
            Some(t) => Location::merge(name.loc, t.loc()),
            None => name.loc,
        };

        Some(ast::FunctionParam {
            loc,
            name,
            type_annotation,
        })
    }

    fn parse_function_body(&mut self) -> ast::BlockExpression {
        match self.tokens.peek() {
            Some((Ok(Token::LBrace), _)) => {}
            _ => {
                self.recover_before(&[Token::LBrace], &[Token::Newline]);
            }
        }

        match self.tokens.peek() {
            Some((Ok(Token::LBrace), _)) => self.parse_block(),
            _ => {
                let range = self.next_range();
                let loc = self.localize(range);
                self.error(DiagnosticKind::MissingBody, loc);
                ast::BlockExpression {
                    loc: loc.decrement(),
                    statements: vec![],
                }
            }
        }
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
    fn test_parse_empty_function() {
        test_expression(ExpressionTest {
            input: "fn() {}",
            expected: ast::Expression::Function(ast::FunctionExpression {
                loc: Location::new(0, Span::new(0, 7)),
                name: None,
                type_params: None,
                params: vec![],
                return_type: None,
                body: ast::BlockExpression {
                    loc: Location::new(0, Span::new(5, 7)),
                    statements: vec![],
                },
            }),
            diagnostics: vec![],
        });
    }
}
