use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_type_alias(&mut self, docs: Option<ast::Docs>) -> ast::TypeAlias {
        let start_range = self.eat(&[Token::Type]);
        let mut loc = self.localize(start_range);

        let Ok(type_name) = self.parse_type_name(&[Token::Eq]) else {
            return ast::TypeAlias {
                docs,
                loc,
                name: None,
                params: None,
                definition: None,
            };
        };
        if let Some(type_name) = &type_name {
            loc = Location::merge(loc, type_name.loc);
        }
        let name = type_name.as_ref().map(|t| t.name.clone());
        let params = type_name.and_then(|t| t.params);

        match self.tokens.peek() {
            Some((Ok(Token::Eq), range)) => {
                let range = range.clone();
                loc = Location::merge(loc, self.localize(range));
                self.tokens.next();
            }
            _ => {
                let error = DiagnosticKind::ExpectedToken {
                    expected: vec![Token::Eq.to_string()],
                };
                let error_loc = self.next_loc();
                self.error(error, error_loc);
                return ast::TypeAlias {
                    docs,
                    loc,
                    name,
                    params,
                    definition: None,
                };
            }
        }

        let definition = self.parse_type();
        if let Some(definition) = &definition {
            loc = Location::merge(loc, definition.loc());
        }

        ast::TypeAlias {
            docs,
            loc,
            name,
            params,
            definition,
        }
    }
}
