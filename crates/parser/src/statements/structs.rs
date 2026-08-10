use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_struct_definition(
        &mut self,
        docs: Option<Docs>,
        meta: Option<Vec<MetaAttribute>>,
        pub_loc: Option<Location>,
    ) -> StructDefinition {
        let kw_range = self.eat(&[Token::Struct]);
        let kw_loc = self.localize(kw_range);
        let mut loc = pub_loc.map_or(kw_loc, |l| Location::merge(l, kw_loc));
        let public = pub_loc.is_some();

        let Ok(type_name) = self.parse_type_name(&[Token::LBrace, Token::LParen]) else {
            return StructDefinition {
                docs,
                meta,
                loc,
                public,
                ..Default::default()
            };
        };
        if let Some(type_name) = &type_name {
            loc = Location::merge(loc, type_name.loc);
        }

        let body = self.maybe_parse_struct_body();

        match &body {
            Some(body) => {
                loc = Location::merge(loc, body.loc());
            }
            None => {
                let loc = self.next_loc();
                self.error(DiagnosticKind::MissingBody, loc);
            }
        }

        StructDefinition {
            docs,
            meta,
            loc,
            public,
            name: type_name.as_ref().map(|t| t.name.clone()),
            params: type_name.and_then(|t| t.params),
            body,
        }
    }

    fn maybe_parse_struct_body(&mut self) -> Option<StructBody> {
        if !self.maybe_is(|t| *t == Token::LBrace) {
            return None;
        }
        self.try_parse(
            |self_| Some(self_.parse_struct_body()),
            |t| *t == Token::Newline,
        )
        .ok()?
    }

    pub(crate) fn parse_struct_body(&mut self) -> StructBody {
        let start_range = self.eat(&[Token::LBrace]);

        let fields = self.parse_list(
            |p| p.parse_struct_definition_field(),
            Token::Comma,
            Token::RBrace,
        );

        let end_range = self.expect(Token::RBrace);
        let loc = self.localize(start_range.start..end_range.end);
        StructBody { loc, fields }
    }

    fn parse_struct_definition_field(&mut self) -> Option<StructDefinitionField> {
        let (_, pub_loc) = self.maybe_eat(|t| t.pub_()).unzip();

        let name = self.maybe_parse_name(|t| {
            matches!(
                t,
                Token::Comma | Token::Colon | Token::Newline | Token::RBrace
            )
        })?;

        let colon = self.better_expect(
            |t| t.colon(),
            &[Token::Newline, Token::Comma, Token::RBrace],
        );
        if let Err(skipped) = colon {
            let error = DiagnosticKind::ExpectedToken {
                expected: vec![":".into()],
            };
            let loc = self.localize(skipped);
            self.error(error, loc);
            return Some(StructDefinitionField {
                loc: pub_loc.map_or(name.loc, |l| Location::merge(l, name.loc)),
                name: Some(name),
                definition: None,
                public: pub_loc.is_some(),
            });
        }

        let definition = self.parse_type();
        if definition.is_none() {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingType, loc);
        }

        let loc = match (pub_loc, &definition) {
            (Some(p), Some(def)) => Location::merge(p, def.loc()),
            (Some(p), None) => Location::merge(p, name.loc),
            (None, Some(def)) => Location::merge(name.loc, def.loc()),
            _ => return None,
        };

        Some(StructDefinitionField {
            loc,
            name: Some(name),
            definition,
            public: pub_loc.is_some(),
        })
    }
}

#[cfg(test)]
mod tests {
    use tine_common::locations::Span;

    use crate::test_utils::{test_statement, StatementTest};

    use super::*;

    #[test]
    fn test_parse_empty_struct() {
        test_statement(StatementTest {
            input: "struct Foo {}",
            expected: Statement::StructDefinition(StructDefinition {
                docs: None,
                meta: None,
                loc: Location::new(0, Span::new(0, 13)),
                public: false,
                name: Some(Identifier {
                    loc: Location::new(0, Span::new(7, 10)),
                    text: "Foo".to_string(),
                }),
                params: None,
                body: Some(StructBody {
                    loc: Location::new(0, Span::new(11, 13)),
                    fields: vec![],
                }),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_struct() {
        test_statement(StatementTest {
            input: "struct Foo {\n    bar: int\n}",
            expected: Statement::StructDefinition(StructDefinition {
                loc: Location::new(0, Span::new(0, 27)),
                name: Some(Identifier {
                    loc: Location::new(0, Span::new(7, 10)),
                    text: "Foo".to_string(),
                }),
                body: Some(StructBody {
                    loc: Location::new(0, Span::new(11, 27)),
                    fields: vec![StructDefinitionField {
                        loc: Location::new(0, Span::new(17, 25)),
                        name: Some(Identifier {
                            loc: Location::new(0, Span::new(17, 20)),
                            text: "bar".to_string(),
                        }),
                        definition: Some(Type::Named(NamedType {
                            loc: Location::new(0, Span::new(22, 25)),
                            name: Identifier {
                                loc: Location::new(0, Span::new(22, 25)),
                                text: "int".to_string(),
                            },
                            args: None,
                        })),
                        public: false,
                    }],
                }),
                ..Default::default()
            }),
            diagnostics: vec![],
        });
    }
}
