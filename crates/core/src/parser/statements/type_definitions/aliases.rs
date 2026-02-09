use pest::iterators::Pair;

use crate::{
    ast,
    parser::{parser::Rule, ParserEngine},
};

impl ParserEngine {
    pub fn parse_type_alias(&mut self, pair: Pair<'_, Rule>) -> ast::TypeAlias {
        debug_assert_eq!(pair.as_rule(), Rule::type_alias);
        let loc = self.localize(pair.as_span());
        let mut inner = pair.into_inner();

        let name = Some(self.parse_identifier(inner.next().unwrap()));
        let mut params = None;
        let mut definition = None;
        for pair in inner {
            match pair.as_rule() {
                Rule::type_params => {
                    params = Some(self.parse_type_params(pair));
                }
                Rule::type_annotation => {
                    definition = Some(self.parse_type(pair));
                }
                _ => unreachable!(),
            }
        }
        let definition = Some(definition.unwrap());

        ast::TypeAlias {
            docs: None,
            loc,
            name,
            params,
            definition,
        }
    }
}
