use crate::{
    ast,
    parser::{tokens::Token, Parser},
    Location,
};

impl Parser<'_> {
    pub fn parse_struct_definition(
        &mut self,
        docs: Option<ast::Docs>,
        pub_loc: Option<Location>,
    ) -> ast::StructDefinition {
        let kw_range = self.eat(&[Token::Struct]);
        let kw_loc = self.localize(kw_range);
        let mut loc = pub_loc.map_or(kw_loc, |l| Location::merge(l, kw_loc));
        let public = pub_loc.is_some();

        let Ok(type_name) = self.parse_type_name(&[Token::LBrace, Token::LParen]) else {
            return ast::StructDefinition {
                docs,
                loc,
                public,
                name: None,
                params: None,
                body: None,
            };
        };
        if let Some(type_name) = &type_name {
            loc = Location::merge(loc, type_name.loc);
        }

        let body = self.parse_type_body();
        if let Some(body) = &body {
            loc = Location::merge(loc, body.loc());
        }

        ast::StructDefinition {
            docs,
            loc,
            public,
            name: type_name.as_ref().map(|t| t.name.clone()),
            params: type_name.and_then(|t| t.params),
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser::test_utils::{test_statement, StatementTest},
        Span,
    };

    use super::*;

    #[test]
    fn test_parse_empty_struct() {
        test_statement(StatementTest {
            input: "struct Foo {}",
            expected: ast::Statement::StructDefinition(ast::StructDefinition {
                docs: None,
                loc: Location::new(0, Span::new(0, 13)),
                public: false,
                name: Some(ast::Identifier {
                    loc: Location::new(0, Span::new(7, 10)),
                    text: "Foo".to_string(),
                }),
                params: None,
                body: Some(ast::TypeBody::Struct(ast::StructBody {
                    loc: Location::new(0, Span::new(11, 13)),
                    fields: vec![],
                })),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_struct() {
        test_statement(StatementTest {
            input: "struct Foo {\n    bar: int\n}",
            expected: ast::Statement::StructDefinition(ast::StructDefinition {
                loc: Location::new(0, Span::new(0, 27)),
                name: Some(ast::Identifier {
                    loc: Location::new(0, Span::new(7, 10)),
                    text: "Foo".to_string(),
                }),
                body: Some(ast::TypeBody::Struct(ast::StructBody {
                    loc: Location::new(0, Span::new(11, 27)),
                    fields: vec![ast::StructDefinitionField {
                        loc: Location::new(0, Span::new(17, 25)),
                        name: Some(ast::Identifier {
                            loc: Location::new(0, Span::new(17, 20)),
                            text: "bar".to_string(),
                        }),
                        definition: Some(ast::Type::Named(ast::NamedType {
                            loc: Location::new(0, Span::new(22, 25)),
                            name: ast::Identifier {
                                loc: Location::new(0, Span::new(22, 25)),
                                text: "int".to_string(),
                            },
                            args: None,
                        })),
                        public: false,
                    }],
                })),
                ..Default::default()
            }),
            diagnostics: vec![],
        });
    }
}
