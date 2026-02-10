use crate::{
    ast,
    parser2::{statements::utils::TypeName, tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_enum(&mut self, docs: Option<ast::Docs>) -> ast::EnumDefinition {
        let start_range = self.eat(&[Token::Enum]);
        let start = self.localize(start_range);

        let (name, params) = self.try_parse_type_name().unwrap_or(("".to_string(), None));
        self.expect(Token::LBrace);
        let variants = self.parse_enum_variants();
        let end_range = self.expect(Token::RBrace);
        let end = self.localize(end_range);

        ast::EnumDefinition {
            docs,
            loc: Location::merge(start, end),
            name,
            params,
            variants,
        }
    }

    fn parse_enum_variants(&mut self) -> Vec<ast::VariantDefinition> {
        let variants = self.parse_list(
            |p| p.parse_variant_definition(),
            Token::Comma,
            Token::RBrace,
        );
        variants
    }

    fn parse_variant_definition(&mut self) -> Option<ast::VariantDefinition> {
        let Ok(type_name) = self.parse_type_name(&[Token::LBrace, Token::LParen, Token::Newline])
        else {
            return None;
        };
        if let Some(TypeName {
            params: Some(_),
            loc,
            ..
        }) = type_name
        {
            self.error(DiagnosticKind::UnexpectedTypeParams, loc);
        }
        let body = self.parse_type_body();
        let loc = match (&type_name, &body) {
            (Some(type_name), Some(body)) => Location::merge(type_name.loc, body.loc()),
            (Some(type_name), None) => type_name.loc,
            (None, Some(body)) => body.loc(),
            (None, None) => return None,
        };
        Some(ast::VariantDefinition {
            loc,
            body,
            name: type_name.map(|t| t.name),
        })
    }
}
