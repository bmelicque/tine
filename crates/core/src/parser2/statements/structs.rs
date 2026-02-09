use crate::{
    ast,
    parser2::{tokens::Token, Parser},
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

        ast::StructDefinition {
            docs,
            loc,
            name: type_name.as_ref().map(|t| t.name.clone()),
            params: type_name.and_then(|t| t.params),
            body,
        }
    }
}
