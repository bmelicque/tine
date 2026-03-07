use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_function_expression(&mut self) -> ast::FunctionExpression {
        let start_range = self.eat(&[Token::Fn]);
        let start_loc = self.localize(start_range);
        let function = self.parse_function_expression_without_kw();
        match function {
            Some(mut function) => {
                function.loc = Location::merge(start_loc, function.loc);
                function
            }
            _ => ast::FunctionExpression {
                loc: start_loc,
                name: None,
                type_params: None,
                params: None,
                return_type: None,
                body: None,
            },
        }
    }

    pub fn parse_function_expression_without_kw(&mut self) -> Option<ast::FunctionExpression> {
        let name = self.parse_function_name();
        let type_params = self.parse_function_type_params();
        let params = self.parse_function_params();
        let return_type = self.parse_type();
        let body = self.parse_function_body();
        let start_loc = if let Some(name) = &name {
            name.loc
        } else if let Some(params) = &params {
            params.loc
        } else if let Some(return_type) = &return_type {
            return_type.loc()
        } else if let Some(body) = &body {
            body.loc
        } else {
            return None;
        };
        let end_loc = if let Some(body) = &body {
            body.loc
        } else if let Some(return_type) = &return_type {
            return_type.loc()
        } else if let Some(params) = &params {
            params.loc
        } else {
            name.as_ref().unwrap().loc
        };
        let loc = Location::merge(start_loc, end_loc);

        Some(ast::FunctionExpression {
            loc,
            name,
            type_params,
            params,
            return_type,
            body,
        })
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

    fn parse_function_params(&mut self) -> Option<ast::FunctionParams> {
        let Some((Ok(Token::LParen), _)) = self.tokens.peek() else {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingName, loc);
            return None;
        };
        let start_range = self.eat(&[Token::LParen]);
        let start_loc = self.localize(start_range);

        let params = self.parse_list(|p| p.parse_function_param(), Token::Comma, Token::RParen);
        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RBrace), r)) => r.clone(),
            _ => self.recover_at(&[Token::RBrace]),
        };
        let end_loc = self.localize(end_range);
        let loc = Location::merge(start_loc, end_loc);

        Some(ast::FunctionParams { loc, params })
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

    fn parse_function_body(&mut self) -> Option<ast::BlockExpression> {
        match self.tokens.peek() {
            Some((Ok(Token::LBrace), _)) => Some(self.parse_block()),
            _ => {
                let loc = self.next_loc();
                self.error(DiagnosticKind::MissingBody, loc);
                None
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
                params: Some(ast::FunctionParams {
                    loc: Location::new(0, Span::new(2, 4)),
                    params: vec![],
                }),
                return_type: None,
                body: Some(ast::BlockExpression {
                    loc: Location::new(0, Span::new(5, 7)),
                    statements: vec![],
                }),
            }),
            diagnostics: vec![],
        });
    }
}
