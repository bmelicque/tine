use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_function_type(&mut self) -> ast::FunctionType {
        let start_range = self.eat(&[Token::Fn]);
        let start_loc = self.localize(start_range);
        let params = match self.tokens.peek() {
            Some((Ok(Token::LParen), _)) => self.parse_tuple_type(),
            _ => {
                let error_loc = self.next_loc();
                self.error(DiagnosticKind::MissingParams, error_loc);
                return ast::FunctionType {
                    loc: start_loc,
                    params: vec![],
                    returned: None,
                };
            }
        };
        let Some(arrow) = self.eat_if(&[Token::SlimArrow]) else {
            return ast::FunctionType {
                loc: Location::merge(start_loc, params.loc),
                params: params.elements,
                returned: None,
            };
        };
        let returned = self.parse_type();
        let loc = match &returned {
            Some(r) => Location::merge(start_loc, r.loc()),
            None => Location::merge(start_loc, self.localize(arrow)),
        };
        ast::FunctionType {
            loc,
            params: params.elements,
            returned: returned.map(|t| Box::new(t)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Parser;

    #[test]
    fn parse_function_type() {
        let mut parser = Parser::new(0, "fn(int) -> int");
        let f = parser.parse_type().expect("expected a type");
        let f = f.as_function().expect("expected a function type");
        assert_eq!(f.params.len(), 1);
        f.returned
            .as_deref()
            .expect("expected a return type")
            .as_named()
            .expect("expected a named return type");
    }
}
