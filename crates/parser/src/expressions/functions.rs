use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_function_expression(&mut self) -> FunctionExpression {
        let start_range = self.eat(&[Token::Fn]);
        let start_loc = self.localize(start_range);
        let function = self.parse_function_expression_without_kw();
        match function {
            Some(mut function) => {
                function.loc = Location::merge(start_loc, function.loc);
                function
            }
            _ => FunctionExpression {
                loc: start_loc,
                ..Default::default()
            },
        }
    }

    pub fn parse_function_expression_without_kw(&mut self) -> Option<FunctionExpression> {
        let name = self.parse_function_name();
        let type_params = self.parse_function_type_params();
        let params = self.parse_function_params();
        let return_type = self.parse_function_return_type();
        let body = self.parse_function_body(return_type.is_some());
        let loc = Location::merge_list(&[
            name.as_ref().map(|x| x as &dyn Locatable),
            params.as_ref().map(|x| x as &dyn Locatable),
            return_type.as_ref().map(|x| x as &dyn Locatable),
            body.as_ref().map(|x| x as &dyn Locatable),
        ])?;

        Some(FunctionExpression {
            loc,
            name,
            type_params,
            params,
            return_type,
            body: body.map(Box::new),
        })
    }

    fn parse_function_name(&mut self) -> Option<Identifier> {
        if let Some((Ok(Token::Ident(ident)), range)) = self.tokens.peek() {
            let name = ident.to_owned();
            let range = range.clone();
            self.tokens.next();
            Some(Identifier {
                loc: self.localize(range),
                text: name,
            })
        } else {
            None
        }
    }

    fn parse_function_type_params(&mut self) -> Option<Vec<Identifier>> {
        match self.tokens.peek() {
            Some((Ok(Token::Lt), _)) => Some(self.parse_type_params()),
            _ => None,
        }
    }

    pub fn parse_function_params(&mut self) -> Option<FunctionParams> {
        let Some((Ok(Token::LParen), _)) = self.tokens.peek() else {
            let diag = DiagnosticKind::ExpectedToken {
                expected: vec!["(".to_string()],
            };
            let loc = self.next_loc();
            self.error(diag, loc);
            return None;
        };
        let start_range = self.eat(&[Token::LParen]);
        let start_loc = self.localize(start_range);

        let params = self.parse_list(|p| p.parse_function_param(), Token::Comma, Token::RParen);
        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RParen), _)) => self.eat(&[Token::RParen]),
            _ => self.recover_at(&[Token::RParen]),
        };
        let end_loc = self.localize(end_range);
        let loc = Location::merge(start_loc, end_loc);

        Some(FunctionParams { loc, params })
    }

    fn parse_function_param(&mut self) -> Option<FunctionParam> {
        let name = self.maybe_parse_name(|t| matches!(t, Token::Comma | Token::RParen))?;
        let Some((_, colon_loc)) = self.maybe_eat(|t| t.colon()) else {
            return Some(FunctionParam {
                loc: name.loc,
                name: Some(name),
                type_annotation: None,
            });
        };
        let type_annotation = self.parse_type();

        let loc = match &type_annotation {
            Some(t) => Location::merge(name.loc, t.loc()),
            None => Location::merge(name.loc, colon_loc),
        };

        Some(FunctionParam {
            loc,
            name: Some(name),
            type_annotation,
        })
    }

    fn parse_function_return_type(&mut self) -> Option<Type> {
        self.eat_if(&[Token::SlimArrow])?;
        let ty = self.parse_type();
        if ty.is_none() {
            let err_loc = self.next_loc();
            self.error(DiagnosticKind::ExpectedType, err_loc);
        }
        ty
    }

    fn parse_function_body(&mut self, expect_block: bool) -> Option<Expression> {
        let is_block = self.maybe_is(|t| *t == Token::LBrace);
        let body = match (expect_block, is_block) {
            (true, true) => return Some(self.parse_block().into()),
            (false, _) => self.parse_expression_with_block(),
            _ => None,
        };

        if body.is_none() {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingBody, loc);
        }
        body
    }
}

#[cfg(test)]
mod tests {
    use crate::{test_utils::parse_expression, Parser};

    #[test]
    fn parse_empty_function() {
        let expr = parse_expression("fn() {}").expect("expected no errors");
        let expr = expr.as_function().expect("expected a function expression");
        assert!(expr.name.is_none());
        assert!(expr.type_params.is_none());
        assert!(expr
            .params
            .as_ref()
            .expect("expected params")
            .params
            .is_empty());
        expr.body
            .as_ref()
            .expect("expected a function body")
            .as_block()
            .expect("expected a block body");
    }

    #[test]
    fn parse_function_with_param() {
        let expr = parse_expression("fn(a) {}").expect("expected no errors");
        let expr = expr.as_function().expect("expected a function expression");
        let params = &expr.params.as_ref().expect("expected params").params;
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn parse_callback() {
        let expr = parse_expression("fn(a, b) a + b").expect("expected no errors");
        let expr = expr.as_function().expect("expected a function expression");
        assert_eq!(
            expr.params.as_ref().expect("expected params").params.len(),
            2
        );
        expr.body
            .as_ref()
            .expect("expected a function body")
            .as_binary()
            .expect("expected a binary expression");
    }

    #[test]
    fn parse_function_param() {
        let mut parser = Parser::new(0, "param: Type");
        let param = parser
            .parse_function_param()
            .expect("expected a function param");
        assert_eq!(param.name.expect("expected a param name").as_str(), "param");
        assert_eq!(
            param
                .type_annotation
                .expect("expected a type annotation")
                .as_named()
                .expect("expected a named type")
                .name
                .as_str(),
            "Type"
        );
    }
}
