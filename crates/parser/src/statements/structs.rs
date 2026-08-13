use std::ops::Range;

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

        let (body, end) = self.parse_struct_items().unzip();

        if body.is_none() {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingBody, loc);
        }

        let loc = end.map_or(loc, |e| Location::merge(loc, e));

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

    fn parse_struct_items(&mut self) -> Option<(Vec<StructItem>, Location)> {
        self.eat_if(&[Token::LBrace])?;
        let items = self.parse_list(|p| p.parse_struct_item(), Token::Comma, Token::RBrace);
        let end = self.expect(Token::RBrace);
        let end = self.localize(end);
        Some((items, end))
    }

    fn parse_struct_item(&mut self) -> Option<StructItem> {
        let docs = self.maybe_parse_docs();
        let public_range = self.eat_if(&[Token::Pub]);

        match self.tokens.peek()?.0.as_ref().ok()? {
            Token::Static | Token::Mut | Token::Fn => self
                .parse_method_definition(docs, public_range)
                .map(Into::into),
            _ => self
                .parse_struct_definition_field(docs, public_range)
                .map(Into::into),
        }
    }

    fn parse_struct_definition_field(
        &mut self,
        docs: Option<Docs>,
        public_range: Option<Range<usize>>,
    ) -> Option<StructDefinitionField> {
        let pub_loc = public_range.map(|r| self.localize(r));
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
                docs,
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
            docs,
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
                body: Some(vec![]),
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
                body: Some(vec![StructItem::Field(StructDefinitionField {
                    docs: None,
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
                })]),
                ..Default::default()
            }),
            diagnostics: vec![],
        });
    }
}
