use crate::{
    ast,
    parser2::{tokens::Token, Parser},
    Location,
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
        let name_result = self.try_parse_type_name();
        // TODO: parse body
        unimplemented!()
    }
}
