use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_type_alias(
        &mut self,
        docs: Option<ast::Docs>,
        pub_loc: Option<Location>,
    ) -> ast::TypeAlias {
        let kw_range = self.eat(&[Token::Struct]);
        let kw_loc = self.localize(kw_range);
        let mut loc = pub_loc.map_or(kw_loc, |l| Location::merge(l, kw_loc));
        let public = pub_loc.is_some();

        let Ok(type_name) = self.parse_type_name(&[Token::Eq]) else {
            return ast::TypeAlias {
                docs,
                loc,
                public,
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
                    public,
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
            public,
            name,
            params,
            definition,
        }
    }
}
