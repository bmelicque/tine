use crate::{
    ast,
    parser::{tokens::Token, Parser},
    Location,
};

impl Parser<'_> {
    pub fn parse_struct_definition(&mut self, docs: Option<ast::Docs>) -> ast::StructDefinition {
        let start_range = self.eat(&[Token::Struct]);
        let mut loc = self.localize(start_range);

        let Ok(type_name) = self.parse_type_name(&[Token::LBrace, Token::LParen]) else {
            return ast::StructDefinition {
                docs,
                loc,
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
            input: "struct Foo {\n    bar int\n}",
            expected: ast::Statement::StructDefinition(ast::StructDefinition {
                docs: None,
                loc: Location::new(0, Span::new(0, 26)),
                name: Some(ast::Identifier {
                    loc: Location::new(0, Span::new(7, 10)),
                    text: "Foo".to_string(),
                }),
                params: None,
                body: Some(ast::TypeBody::Struct(ast::StructBody {
                    loc: Location::new(0, Span::new(11, 26)),
                    fields: vec![ast::StructDefinitionField::Mandatory(
                        ast::StructMandatoryField {
                            loc: Location::new(0, Span::new(17, 24)),
                            name: Some(ast::Identifier {
                                loc: Location::new(0, Span::new(17, 20)),
                                text: "bar".to_string(),
                            }),
                            definition: Some(ast::Type::Named(ast::NamedType {
                                loc: Location::new(0, Span::new(21, 24)),
                                name: "int".to_string(),
                                args: None,
                            })),
                        },
                    )],
                })),
            }),
            diagnostics: vec![],
        });
    }
}
